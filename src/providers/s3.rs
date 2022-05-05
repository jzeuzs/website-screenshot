use std::env;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use redis::{AsyncCommands, Client};
use s3::creds::Credentials;
use s3::{Bucket, Region};

use super::Provider;

#[derive(Debug)]
pub struct S3Provider {
    redis: Arc<Client>,
    bucket: Arc<Bucket>,
}

#[async_trait]
impl Provider for S3Provider {
    fn new() -> Self {
        let redis = Arc::new(
            Client::open(env::var("REDIS_URL").expect("Failed to load redis url"))
                .expect("Failed to open redis client"),
        );

        let creds = Credentials {
            access_key: Some(env::var("S3_ACCESS_KEY").expect("Failed to load s3 access key")),
            secret_key: Some(env::var("S3_SECRET_KEY").expect("Failed to load s3 secret key")),
            security_token: match env::var("S3_SECURITY_TOKEN") {
                Ok(t) => Some(t),
                Err(_) => None,
            },
            session_token: match env::var("S3_SESSION_TOKEN") {
                Ok(t) => Some(t),
                Err(_) => None,
            },
        };

        let region: Region = match env::var("S3_REGION") {
            Ok(region) => region.parse().expect("Failed to parse s3 region"),
            Err(_) => {
                let endpoint = env::var("S3_ENDPOINT_URL").expect("Failed to load s3 endpoint url");

                Region::Custom {
                    region: "".to_owned(),
                    endpoint,
                }
            },
        };

        let bucket = Arc::new(
            Bucket::new(
                &env::var("S3_BUCKET_NAME").expect("Failed to load s3 bucket name"),
                region,
                creds,
            )
            .expect("Failed to initialize s3 bucket"),
        );

        Self {
            redis,
            bucket,
        }
    }

    #[inline]
    fn prefix() -> String {
        "s3".to_owned()
    }

    async fn get(&self, slug: String) -> Result<Vec<u8>> {
        let mut con = self.redis.get_async_connection().await?;
        let path: String = con.get(format!("{}:{slug}", S3Provider::prefix())).await?;
        let (data, _) = self.bucket.get_object(path).await?;

        Ok(data)
    }

    async fn set(&self, slug: String, data: Vec<u8>) -> Result<()> {
        let mut con = self.redis.get_async_connection().await?;
        let path = format!("{}.png", slug.clone());

        self.bucket.put_object(&path, &data).await?;
        con.set(format!("{}:{slug}", S3Provider::prefix()), path).await?;

        Ok(())
    }

    async fn check(&self, slug: String) -> Result<bool> {
        let mut con = self.redis.get_async_connection().await?;

        match con.get::<String, String>(format!("{}:{slug}", S3Provider::prefix())).await {
            Ok(path) => match self.bucket.head_object(path).await {
                Ok((res, code)) => {
                    if code >= 400 || code <= 599 {
                        return Ok(false);
                    }

                    if res.content_type.is_none() || res.content_length.is_none() {
                        return Ok(false);
                    }

                    Ok(true)
                },
                Err(_) => Ok(false),
            },
            Err(_) => Ok(false),
        }
    }
}
