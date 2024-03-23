use crate::{errors::WardenError, hasher::Hasher, redis::to_redis_key::*, warden::Warden};
use log::*;
use redis::{Client, Commands, ErrorKind, RedisResult};
use solana_sdk::pubkey::Pubkey;
use std::{fmt::Debug, marker::PhantomData, time::Duration};

pub struct RedisClient {
    client: Client,
}

impl RedisClient {
    pub fn new(redis_url: &str) -> anyhow::Result<Self> {
        let client = Client::open(redis_url).map_err(|e| anyhow::Error::from(e))?;
        Ok(Self { client })
    }

    pub fn get<K: ToRedisKey>(&self, key: K) -> anyhow::Result<Option<String>> {
        let key = key.to_redis_key();
        let mut con = self
            .client
            .get_connection()
            .map_err(|_| WardenError::RedisConnectionError)?;
        let (result,): (Option<String>,) = redis::transaction(&mut con, &[&key], |con, pipe| {
            pipe.atomic().get(&key).query(con)
        })?;
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
        // Transaction watches all keys and enters pipeline in atomic mode
        let (result,): (Option<String>,) = redis::transaction(&mut con, &[&key], |con, pipe| {
            let old_val: Vec<Option<String>> = con.get(&key)?;
            debug!("old Redis value: {:?}", old_val);
            match &value {
                Some(value) => pipe.atomic().set(&key, value).ignore().get(&key).query(con),
                None => pipe.atomic().del(&key).ignore().get(&key).query(con),
            }
        })?;
        Ok(result)
    }
}

/// Format redis url
#[must_use]
pub fn fmt_url(user: &str, password: &str, host: &str, port: &str) -> String {
    format!("redis://{user}:{password}@{host}:{port}")
}

#[tokio::test]
async fn test_redis_client() -> anyhow::Result<()> {
    let redis_username = "default";
    let redis_password = "IJD4LqEHEk3mjoMxvcXDvDIKSUyNUSDD";
    let redis_host = "redis-17359.c284.us-east1-2.gce.cloud.redislabs.com";
    let redis_port = 17359;
    let url = fmt_url(
        redis_username,
        redis_password,
        redis_host,
        &redis_port.to_string(),
    );
    let client = RedisClient::new(&url)?;

    let value = Pubkey::new_unique().to_string();
    let key = uuid::Uuid::new_v4().to_string();
    let hashed_key = Hasher::hash(key.as_bytes())?;
    let upsert_res = match client.upsert(hashed_key.clone(), Some(value.clone())) {
        Ok(upsert_res) => upsert_res,
        Err(e) => {
            panic!("Error upserting value: {:?}", e);
        }
    };
    println!("Upsert result: {:?}", upsert_res);

    let cached_value = client.get(hashed_key.clone())?;
    match cached_value {
        None => {
            panic!("Cached value is None");
        }
        Some(cached_value) => match Hasher::verify(key.as_bytes(), &hashed_key) {
            Ok(_) => {
                println!("API key verified successfully for value: {}", cached_value);
            }
            Err(e) => {
                panic!("API key verification failed: {:?}", e);
            }
        },
    }
    Ok(())
}
