use crate::archive::{append_vec_iter, AppendVecMeta, ArchiveAccount};
use std::rc::Rc;

// pub type ArchiveCallback = Box<dyn Fn(ArchiveAccount)>;
pub type ArchiveCallback =
    Box<dyn Fn(ArchiveAccount) -> anyhow::Result<()> + Send + Sync + 'static>;

/// Archiver handles everything related to extracting accounts from a snapshot
/// 1. Load snapshot from file or HTTP source
/// 2. Decode AppendVec
/// 3. Iterate raw accounts in AppendVec
/// 4. Deserialize archived accounts in AppendVec
/// 5. Emit each ArchiveAccount to callback
pub struct Archiver {
    accounts_count: u64,
    callback: ArchiveCallback,
}

impl Archiver {
    pub fn new(callback: ArchiveCallback) -> anyhow::Result<Self> {
        Ok(Self {
            accounts_count: 0,
            callback,
        })
    }

    // todo: par iter if possible
    pub fn extract_accounts(&mut self, append_vec: AppendVecMeta) -> anyhow::Result<()> {
        for account in append_vec_iter(Rc::new(append_vec)) {
            let account = account.snapshot_account().unwrap();
            self.archive_account_callback(account)?
        }
        Ok(())
    }

    pub fn archive_account_callback(&mut self, account: ArchiveAccount) -> anyhow::Result<()> {
        self.accounts_count += 1;
        (self.callback)(account)
    }
}
