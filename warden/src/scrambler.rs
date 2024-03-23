use crate::{ToRedisKey, WardenError};
use std::collections::hash_map::DefaultHasher;
use std::hash::Hash;
use std::hash::Hasher;

#[derive(Debug, Default)]
pub struct Scrambler(pub DefaultHasher);

pub trait HasherTrait {
    fn new() -> Self;
    fn finish(&mut self) -> u64;
    fn hash<T: ToRedisKey>(&mut self, key: &T) -> u64;
    fn verify<T: ToRedisKey>(&mut self, key: &T, hash: u64) -> anyhow::Result<()>;
}

impl HasherTrait for Scrambler {
    fn new() -> Self {
        Self(DefaultHasher::new())
    }

    fn finish(&mut self) -> u64 {
        self.0.finish()
    }

    fn hash<T: ToRedisKey>(&mut self, key: &T) -> u64 {
        self.0 = DefaultHasher::new();
        key.to_redis_key().hash(&mut self.0);
        self.finish()
    }

    fn verify<T: ToRedisKey>(&mut self, key: &T, hash: u64) -> anyhow::Result<()> {
        match self.hash(key) == hash {
            true => Ok(()),
            false => Err(anyhow::Error::from(WardenError::ApiKeyMismatch)),
        }
    }
}
