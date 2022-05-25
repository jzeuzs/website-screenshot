use anyhow::Result;
use async_trait::async_trait;
use cfg_if::cfg_if;

#[async_trait]
pub trait Provider {
    fn new() -> Self;
    fn prefix() -> String;

    async fn get(&self, slug: String) -> Result<Vec<u8>>;
    async fn set(&self, slug: String, data: Vec<u8>) -> Result<()>;
    async fn check(&self, slug: String) -> Result<bool>;
}

cfg_if! {
    if #[cfg(feature = "cloudinary_storage")] {
        mod cloudinary;

        pub use cloudinary::CloudinaryProvider as Storage;
    } else if #[cfg(feature = "fs_storage")] {
        mod fs;

        pub use fs::FsProvider as Storage;
    } else if #[cfg(feature = "s3_storage")] {
        mod s3;

        pub use self::s3::S3Provider as Storage;
    } else if #[cfg(feature = "tixte_storage")] {
        mod tixte;

        pub use tixte::TixteProvider as Storage;
    } else if #[cfg(feature = "sled_storage")] {
        mod sled;

        pub use self::sled::SledProvider as Storage;
    }
}
