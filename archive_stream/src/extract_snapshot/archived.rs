use crate::archive::{parse_append_vec_name, AppendVecIterator, AppendVecMeta, ArchiveIterator};
use crate::decode_accounts::{
    deserialize_from, AccountsDbFields, AppendVec, DeserializableVersionedBank,
    SerializableAccountStorageEntry,
};
use crate::SnapshotError;
use async_compression::tokio::bufread::ZstdDecoder;
use log::info;
use std::path::{Component, Path};
use std::time::Instant;
use tokio::fs::File;
use tokio::io::{AsyncBufRead, AsyncReadExt, BufReader};
use tokio_stream::*;
use tokio_tar::{Archive, Entries, Entry};

/// Extracts account data from a .tar.zst HTTP stream or local file.
pub struct ArchiveSnapshotExtractor<Source>
where
    Source: AsyncBufRead + Send + Sync + Unpin + 'static,
{
    accounts_db_fields: AccountsDbFields<SerializableAccountStorageEntry>,
    archive: Archive<ZstdDecoder<Source>>,
    entries: Option<Entries<ZstdDecoder<Source>>>,
}

/// Extracts account data from a .tar.zst HTTP stream
impl<Source> ArchiveIterator for ArchiveSnapshotExtractor<Source>
where
    Source: AsyncBufRead + Send + Sync + Unpin + 'static,
{
    fn iter(&mut self) -> AppendVecIterator {
        Box::new(self.unboxed_iter())
    }
}

impl<Source> ArchiveSnapshotExtractor<Source>
where
    Source: AsyncBufRead + Send + Sync + Unpin + 'static,
{
    pub async fn from_reader(source: Source) -> anyhow::Result<Self> {
        let tar_stream = ZstdDecoder::new(source);
        info!("tar stream");
        let mut archive = Archive::new(tar_stream);
        info!("archive");
        let mut entries = archive.entries()?;
        info!("entries");

        let mut snapshot_file: Option<Entry<_>> = None;
        while let Some(entry) = entries.next().await {
            info!("iter");
            let entry = entry?;
            let path = entry.path()?;
            if Self::is_snapshot_manifest_file(&path) {
                snapshot_file = Some(entry);
                break;
            } else if Self::is_appendvec_file(&path) {
                // TODO Support archives where AppendVecs precede snapshot manifests
                return Err(anyhow::Error::from(SnapshotError::UnexpectedAppendVec));
            }
        }

        let snapshot_file = snapshot_file.ok_or(SnapshotError::NoSnapshotManifest)?;
        info!("snapshot file ok");
        let snapshot_file_path = snapshot_file.path()?.as_ref().to_path_buf();

        info!("Opening snapshot manifest: {:?}", &snapshot_file_path);
        let mut snapshot_file = BufReader::new(snapshot_file);

        info!("Deserializing versioned bank");
        let pre_unpack = Instant::now();
        let versioned_bank: DeserializableVersionedBank = deserialize_from(&mut snapshot_file)?;
        drop(versioned_bank);
        let versioned_bank_post_time = Instant::now();
        info!(
            "Read bank fields in {:?}",
            versioned_bank_post_time - pre_unpack
        );

        info!("Deserializing accounts DB fields");
        let accounts_db_fields: AccountsDbFields<SerializableAccountStorageEntry> =
            deserialize_from(&mut snapshot_file)?;
        let accounts_db_fields_post_time = Instant::now();
        drop(snapshot_file);

        info!(
            "Read accounts DB fields in {:?}",
            accounts_db_fields_post_time - versioned_bank_post_time
        );

        Ok(ArchiveSnapshotExtractor {
            archive,
            accounts_db_fields,
            entries: Some(entries),
        })
    }

    async fn unboxed_iter(&mut self) -> impl Iterator<Item = anyhow::Result<AppendVecMeta>> + '_ {
        let mut collector = Vec::<anyhow::Result<AppendVecMeta>>::new();
        match self.entries.take() {
            None => collector.into_iter(),
            Some(mut entries) => {
                while let Some(entry) = entries.next().await {
                    if let Ok(mut entry) = entry {
                        if let Ok(path) = entry.path() {
                            let res = path.file_name().and_then(parse_append_vec_name);
                            if let Some((slot, id)) = res {
                                collector.push(self.process_entry(&mut entry, slot, id));
                            }
                        };
                    }
                }
                collector.into_iter()
            }
        }
    }

    fn process_entry(
        &self,
        entry: &mut Entry<Archive<ZstdDecoder<Source>>>,
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
impl ArchiveSnapshotExtractor<BufReader<File>> {
    pub async fn open(path: &Path) -> anyhow::Result<Self> {
        let f = File::open(path).await?;
        let t = BufReader::new(f);
        Self::from_reader(t).await
    }
}
