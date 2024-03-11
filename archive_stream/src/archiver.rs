use crate::account::ArchiveAccount;
use crate::archive::{append_vec_iter, AppendVecMeta};
use crate::SnapshotError;
use rayon::iter::ParallelIterator;
use rayon::prelude::IntoParallelRefIterator;
use std::sync::Arc;

/// Archiver handles everything related to extracting accounts from a snapshot
/// 1. Load snapshot from file or HTTP source
/// 2. Decode AppendVec
/// 3. Iterate raw accounts in AppendVec
/// 4. Deserialize archived accounts in AppendVec
/// 5. Emit each ArchiveAccount to callback
pub struct Archiver;

impl Archiver {
    // todo: par iter if possible
    pub fn extract_accounts(
        append_vec: Arc<AppendVecMeta>,
        sender: crossbeam_channel::Sender<ArchiveAccount>,
    ) -> anyhow::Result<()> {
        append_vec_iter(append_vec)
            .par_iter()
            .try_for_each(|handle| {
                let account = match handle.snapshot_account() {
                    Some(account) => Ok(account),
                    None => Err(anyhow::Error::from(SnapshotError::AccountAccessFailed)),
                }?;
                sender.send(account)?;
                Result::<_, anyhow::Error>::Ok(())
            })
    }
}
