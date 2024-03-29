use log::*;
use redis::Client;

use common::RedisUser;

use crate::{ToRedisKey, WardenError};

// use base64::{engine::general_purpose, Engine as _};

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

    fn encode(value: RedisUser) -> anyhow::Result<String> {
        Ok(serde_json::to_string(&value)?)
    }

    fn decode(value: String) -> anyhow::Result<RedisUser> {
        Ok(serde_json::from_str(&value)?)
    }

    pub fn get<K: ToRedisKey>(&self, key: K) -> anyhow::Result<Option<String>> {
        let key = key.to_redis_key();
        let mut con = self
            .client
            .get_connection()
            .map_err(|_| WardenError::RedisConnectionError)?;
        let (result,): (Option<String>,) = redis::pipe().atomic().get(&key).query(&mut con)?;
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
