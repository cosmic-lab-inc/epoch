use crate::archive::{parse_append_vec_name, AppendVecIterator, AppendVecMeta, ArchiveIterator};
use crate::decode_accounts::{
    deserialize_from, AccountsDbFields, AppendVec, DeserializableVersionedBank,
    SerializableAccountStorageEntry,
};
use crate::SnapshotError;
use log::info;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Component, Path};
use std::pin::Pin;
use std::sync::Arc;
use std::time::Instant;
use tar::{Archive, Entries, Entry};
use tokio::sync::{Mutex, RwLock};
use zstd::Decoder;

/// Extracts account data from a .tar.zst HTTP stream or local file.
pub struct ArchiveSnapshotExtractor<Source>
where
    Source: Read + Send + Sync + Unpin + 'static,
{
    accounts_db_fields: AccountsDbFields<SerializableAccountStorageEntry>,
    entries: Option<RwLock<Entries<'static, Decoder<'static, BufReader<Source>>>>>,
}

/// Extracts account data from a .tar.zst HTTP stream
impl<Source> ArchiveIterator for ArchiveSnapshotExtractor<Source>
where
    Source: Read + Send + Sync + Unpin + 'static,
{
    fn iter(&mut self) -> AppendVecIterator {
        Box::new(self.unboxed_iter())
    }
}

impl<Source> ArchiveSnapshotExtractor<Source>
where
    Source: Read + Send + Sync + Unpin + 'static,
{
    pub async fn from_reader(source: Source) -> anyhow::Result<Self> {
        let tar_stream =
            tokio::task::spawn_blocking(|| zstd::stream::read::Decoder::new(source)).await??;
        info!("tar stream");

        let archive = RwLock::new(Archive::new(tar_stream));
        let mut archive_guard = archive.write().await;
        info!("archive lock");

        let snapshot_file: RwLock<Option<Entry<_>>> = RwLock::new(None);
        let mut snapshot_guard = snapshot_file.write().await;
        info!("snapshot lock");

        while let Some(entry) = archive_guard.entries()?.next() {
            info!("while");
            let entry = entry?;
            let path = entry.path()?.into_owned();
            if Self::is_snapshot_manifest_file(&path) {
                *snapshot_guard = Some(entry);
                break;
            } else if Self::is_appendvec_file(&path) {
                return Err(anyhow::Error::from(SnapshotError::UnexpectedAppendVec));
            }
        }
        drop(archive_guard);
        // let entries = archive_guard.entries()?;
        // let entries = RwLock::new(entries);
        info!("end while");

        let snapshot_file_path = match snapshot_file.read().await.as_ref() {
            Some(snapshot) => Ok(snapshot.path()?.as_ref().to_path_buf()),
            None => Err(anyhow::Error::from(SnapshotError::NoSnapshotManifest)),
        }?;
        info!("Opening snapshot manifest: {:?}", &snapshot_file_path);

        // drop RwLock since it was only needed for spawn blocking
        let mut write_lock = snapshot_file.write().await;
        let mut snapshot_file = match std::mem::replace(&mut *write_lock, None) {
            Some(snapshot) => snapshot,
            None => return Err(anyhow::Error::from(SnapshotError::NoSnapshotManifest)),
        };

        info!("Deserializing versioned bank");
        let pre_unpack = Instant::now();
        let versioned_bank: DeserializableVersionedBank = deserialize_from(&mut snapshot_file)?;
        drop(versioned_bank);
        let versioned_bank_post_time = Instant::now();

        info!("Deserializing accounts DB fields");
        let accounts_db_fields: AccountsDbFields<SerializableAccountStorageEntry> =
            deserialize_from(&mut snapshot_file)?;
        let accounts_db_fields_post_time = Instant::now();
        drop(snapshot_file);

        info!(
            "Read bank fields in {:?}",
            versioned_bank_post_time - pre_unpack
        );
        info!(
            "Read accounts DB fields in {:?}",
            accounts_db_fields_post_time - versioned_bank_post_time
        );

        Ok(ArchiveSnapshotExtractor {
            // _archive: archive,
            accounts_db_fields,
            entries: Some(entries),
        })
    }

    fn unboxed_iter(&mut self) -> impl Iterator<Item = anyhow::Result<AppendVecMeta>> + '_ {
        self.entries
            .take()
            .into_iter()
            .flatten()
            .filter_map(|entry| {
                let mut entry = match entry {
                    Ok(x) => x,
                    Err(e) => return Some(Err(e.into())),
                };
                let path = match entry.path() {
                    Ok(x) => x,
                    Err(e) => return Some(Err(e.into())),
                };
                let (slot, id) = path.file_name().and_then(parse_append_vec_name)?;
                Some(self.process_entry(&mut entry, slot, id))
            })
    }

    fn process_entry(
        &self,
        entry: &mut Entry<'static, Decoder<'static, BufReader<Source>>>,
        slot: u64,
        id: u64,
    ) -> anyhow::Result<AppendVecMeta> {
        let known_vecs = self
            .accounts_db_fields
            .0
            .get(&slot)
            .map(|v| &v[..])
            .unwrap_or(&[]);
        let known_vec = known_vecs.iter().find(|entry| entry.id == (id as usize));
        let known_vec = match known_vec {
            None => return Err(anyhow::Error::from(SnapshotError::UnexpectedAppendVec)),
            Some(v) => v,
        };
        let append_vec = AppendVec::new_from_reader(entry, known_vec.accounts_current_len)?;
        let meta = AppendVecMeta { slot, append_vec };
        Ok(meta)
    }

    fn is_snapshot_manifest_file(path: &Path) -> bool {
        let mut components = path.components();
        if components.next() != Some(Component::Normal("snapshots".as_ref())) {
            return false;
        }
        let slot_number_str_1 = match components.next() {
            Some(Component::Normal(slot)) => slot,
            _ => return false,
        };
        // Check if slot number file is valid u64.
        if slot_number_str_1
            .to_str()
            .and_then(|s| s.parse::<u64>().ok())
            .is_none()
        {
            return false;
        }
        let slot_number_str_2 = match components.next() {
            Some(Component::Normal(slot)) => slot,
            _ => return false,
        };
        components.next().is_none() && slot_number_str_1 == slot_number_str_2
    }

    fn is_appendvec_file(path: &Path) -> bool {
        let mut components = path.components();
        if components.next() != Some(Component::Normal("accounts".as_ref())) {
            return false;
        }
        let name = match components.next() {
            Some(Component::Normal(c)) => c,
            _ => return false,
        };
        components.next().is_none() && parse_append_vec_name(name).is_some()
    }
}

/// Extracts account data from a .tar.zst local file.
impl ArchiveSnapshotExtractor<File> {
    pub async fn open(path: &Path) -> anyhow::Result<Self> {
        Self::from_reader(File::open(path)?).await
    }
}
