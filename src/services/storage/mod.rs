use aws_credential_types::{provider::SharedCredentialsProvider, Credentials};
use aws_sdk_s3::Client;
use aws_types::{app_name::AppName, region::Region, SdkConfig};
use bytes::Bytes;
use serde::Deserialize;
use serenity::prelude::TypeMapKey;
use tracing::error;

#[derive(Debug, Deserialize)]
pub struct StorageConfig {
    address: String,
    access_key_id: String,
    secret_access_key: String,
    bucket: String,
}

impl From<&StorageConfig> for SdkConfig {
    fn from(value: &StorageConfig) -> Self {
        let name = AppName::new("ross-bot").unwrap();
        let credentials = SharedCredentialsProvider::new(Credentials::new(
            &value.access_key_id,
            &value.secret_access_key,
            None,
            None,
            "Config",
        ));
        let region = Region::new("eu-east-1");
        Self::builder()
            .app_name(name)
            .endpoint_url(&value.address)
            .credentials_provider(credentials)
            .region(region)
            .build()
    }
}

pub struct Storage {
    client: Client,
    bucket: String,
}

impl Storage {
    pub async fn setup(config: &StorageConfig) -> Self {
        let client = Client::new(&config.into());
        let bucket = config.bucket.clone();
        let this = Self { client, bucket };
        this.client.list_buckets().send().await.unwrap();
        this
    }

    pub async fn upload(&self, key: impl Into<String>, obj: &Bytes) -> Option<()> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(obj.clone().into())
            .send()
            .await
            .map_err(|e| error!(error = ?e))
            .is_ok()
            .then_some(())
    }

    pub async fn download(&self, key: impl Into<String>) -> Option<Bytes> {
        let obj = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| error!(error = ?e))
            .ok()?;
        let obj = obj
            .body
            .collect()
            .await
            .map_err(|e| error!(error = ?e))
            .ok()?
            .into_bytes();
        Some(obj)
    }
}

impl TypeMapKey for Storage {
    type Value = Storage;
}
