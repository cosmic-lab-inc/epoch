use crate::{ToRedisKey, WardenError};
use log::*;
use redis::{Client, Commands};

pub struct RedisClient {
    client: Client,
}

impl RedisClient {
    pub fn new(redis_url: &str) -> anyhow::Result<Self> {
        let client = Client::open(redis_url).map_err(anyhow::Error::from)?;
        Ok(Self { client })
    }

    pub fn fmt_redis_url(user: &str, password: &str, host: &str, port: u16) -> String {
        format!("redis://{user}:{password}@{host}:{port}")
    }

    pub fn get<K: ToRedisKey>(&self, key: K) -> anyhow::Result<Option<String>> {
        let key = key.to_redis_key();
        let mut con = self
            .client
            .get_connection()
            .map_err(|_| WardenError::RedisConnectionError)?;
        let (result,): (Option<String>,) = redis::pipe().atomic().get(&key).query(&mut con)?;

        info!("get Redis key {}, value: {:?}", key, result);
        Ok(result)
    }

    pub fn upsert<K: ToRedisKey>(
        &self,
        key: K,
        value: Option<String>,
    ) -> anyhow::Result<Option<String>> {
        let key = key.to_redis_key();
        let mut con = self
            .client
            .get_connection()
            .map_err(|_| WardenError::RedisConnectionError)?;

        let old_val: Vec<Option<String>> = con.get(&key)?;
        info!("old Redis value: {:?}", old_val.first());

        let (result,): (Option<String>,) = match &value {
            Some(value) => redis::pipe()
                .atomic()
                .set(&key, value)
                .ignore()
                .get(&key)
                .query(&mut con),
            None => redis::pipe()
                .atomic()
                .del(&key)
                .ignore()
                .get(&key)
                .query(&mut con),
        }?;
        info!("key: {}, new Redis value: {:?}", key, result);

        Ok(result)
    }
}
