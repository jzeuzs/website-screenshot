use std::env;
use std::fs::create_dir;
use std::ops::Not;
use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use redis::{AsyncCommands, Client};
use tokio::fs::{read, File};
use tokio::io::AsyncWriteExt;

use super::Provider;

#[derive(Debug)]
pub struct FsProvider(Arc<Client>);

#[async_trait]
impl Provider for FsProvider {
    fn new() -> Self {
        let path = Path::new("screenshots");

        if path.exists().not() && path.is_dir().not() {
            create_dir("screenshots").expect("Failed creating directory");
        }

        let redis = Arc::new(
            Client::open(env::var("REDIS_URL").expect("Failed to load redis url"))
                .expect("Failed to open redis client"),
        );

        Self(redis)
    }

    #[inline]
    fn prefix() -> String {
        "fs".to_owned()
    }

    async fn get(&self, slug: String) -> Result<Vec<u8>> {
        let mut con = self.0.get_async_connection().await?;
        let path: String = con.get(format!("{}:{slug}", FsProvider::prefix())).await?;
        let contents = read(path).await?;

        Ok(contents)
    }

    async fn set(&self, slug: String, data: Vec<u8>) -> Result<()> {
        let file_name = format!("{}.png", slug);
        let file_path = format!("screenshots/{}", file_name);
        let mut file = File::create(&file_path).await?;
        let mut con = self.0.get_async_connection().await?;

        file.write_all(&data).await?;
        con.set(format!("{}:{slug}", FsProvider::prefix()), file_path).await?;

        Ok(())
    }
}
