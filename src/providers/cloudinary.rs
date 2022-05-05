use std::collections::HashMap;
use std::env;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use base64::encode;
use redis::{AsyncCommands, Client as RedisClient};
use reqwest::Client;
use serde_json::Value;

use super::Provider;

#[derive(Debug)]
pub struct CloudinaryProvider {
    redis: Arc<RedisClient>,
    reqwest: Client,
}

#[async_trait]
impl Provider for CloudinaryProvider {
    fn new() -> Self {
        let redis = Arc::new(
            RedisClient::open(std::env::var("REDIS_URL").expect("Failed to get redis url"))
                .expect("Failed to open redis client"),
        );

        Self {
            redis,
            reqwest: Client::new(),
        }
    }

    #[inline]
    fn prefix() -> String {
        "cloudinary".to_owned()
    }

    async fn get(&self, slug: String) -> Result<Vec<u8>> {
        let mut con = self.redis.get_async_connection().await?;
        let url: String = con.get(format!("{}:{slug}", CloudinaryProvider::prefix())).await?;
        let data = self.reqwest.get(url).send().await?.bytes().await?;

        Ok(data.as_ref().to_vec())
    }

    async fn set(&self, slug: String, data: Vec<u8>) -> Result<()> {
        let mut con = self.redis.get_async_connection().await?;
        let base_64_img = format!("data:image/png;base64,{}", encode(data));
        let mut params: HashMap<&'static str, String> = HashMap::new();

        params.insert("public_id", slug.clone());
        params.insert("api_key", env::var("CLOUDINARY_API_KEY")?);
        params.insert("upload_preset", env::var("CLOUDINARY_UPLOAD_PRESET")?);
        params.insert("file", base_64_img);

        let res = self
            .reqwest
            .post(format!(
                "https://api.cloudinary.com/v1_1/{}/image/upload",
                env::var("CLOUDINARY_CLOUD_NAME")?
            ))
            .form(&params)
            .send()
            .await?;

        let json = res.json::<Value>().await?;

        con.set(
            format!("{}:{slug}", CloudinaryProvider::prefix()),
            json["secure_url"].as_str().expect("Failed converting to string"),
        )
        .await?;

        Ok(())
    }

    async fn check(&self, slug: String) -> Result<bool> {
        let mut con = self.redis.get_async_connection().await?;

        match con.get::<String, String>(format!("{}:{slug}", CloudinaryProvider::prefix())).await {
            Ok(url) => {
                let req = self.reqwest.head(url).send().await?;
                let status = req.status();

                if status.is_client_error() && status.is_server_error() {
                    return Ok(false);
                } else {
                    return Ok(true);
                }
            },
            Err(_) => Ok(false),
        }
    }
}
