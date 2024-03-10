use crate::ArchiveAccount;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hash;
use std::hash::Hasher;

pub trait AccountTrait {
    fn key(&self) -> String;
    fn slot(&self) -> u64;
}

#[derive(Debug, Default)]
pub struct AccountHasher(pub DefaultHasher);

pub trait HashTrait {
    fn new() -> Self;
    fn finish(&mut self) -> u64;
    fn hash_account<T: AccountTrait>(&mut self, account: &T) -> u64;
}

impl HashTrait for AccountHasher {
    fn new() -> Self {
        Self(DefaultHasher::new())
    }
    /// Reset contents of hasher for reuse
    fn finish(&mut self) -> u64 {
        self.0.finish()
    }
    /// Generate a hash for this pubkey at a slot
    fn hash_account<T: AccountTrait>(&mut self, account: &T) -> u64 {
        self.0 = DefaultHasher::new();
        account.key().hash(&mut self.0);
        account.slot().hash(&mut self.0);
        self.finish()
    }
}

impl AccountTrait for ArchiveAccount {
    fn key(&self) -> String {
        self.key.to_string()
    }
    fn slot(&self) -> u64 {
        self.slot
    }
}
