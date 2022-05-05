use std::env;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use cuid::cuid;
use redis::{AsyncCommands, Client};
use sled::Db;

use super::Provider;

#[derive(Debug)]
pub struct SledProvider {
    redis: Arc<Client>,
    db: Arc<Db>,
}

#[async_trait]
impl Provider for SledProvider {
    fn new() -> Self {
        let redis = Arc::new(
            Client::open(env::var("REDIS_URL").expect("Failed to load redis url"))
                .expect("Failed to open redis client"),
        );

        let db = Arc::new(
            sled::open(env::var("SLED_PATH").unwrap_or_else(|_| ".website-screenshot".to_owned()))
                .expect("Failed to open sled database"),
        );

        Self {
            redis,
            db,
        }
    }

    #[inline]
    fn prefix() -> String {
        "sled".to_owned()
    }

    async fn get(&self, slug: String) -> Result<Vec<u8>> {
        let mut con = self.redis.get_async_connection().await?;
        let key: String = con.get(format!("{}:{slug}", SledProvider::prefix())).await?;
        let data = self.db.get(key)?.expect("Failed to get data").as_ref().to_vec();

        Ok(data)
    }

    async fn set(&self, slug: String, data: Vec<u8>) -> Result<()> {
        let mut con = self.redis.get_async_connection().await?;
        let key = cuid()?;

        con.set(format!("{}:{slug}", SledProvider::prefix()), &key).await?;
        self.db.insert(key, data)?;

        Ok(())
    }

    async fn check(&self, slug: String) -> Result<bool> {
        let mut con = self.redis.get_async_connection().await?;

        match con.get::<String, String>(format!("{}:{slug}", SledProvider::prefix())).await {
            Ok(key) => Ok(self.db.contains_key(key)?),
            Err(_) => Ok(false),
        }
    }
}
