use aws_credential_types::{provider::SharedCredentialsProvider, Credentials};
use aws_sdk_s3::Client;
use aws_types::{app_name::AppName, region::Region, SdkConfig};
use bytes::Bytes;
use serde::Deserialize;
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

#[derive(Clone)]
pub struct Storage {
    client: Client,
    bucket: String,
}

impl Storage {
    pub async fn setup(config: &StorageConfig) -> SgResult<Self> {
        let client = Client::new(&config.into());
        let bucket = config.bucket.clone();
        let this = Self { client, bucket };
        this.client
            .list_buckets()
            .send()
            .await
            .map_err(aws_sdk_s3::Error::from)?;
        Ok(this)
    }

    pub async fn upload(&self, key: impl Into<String>, obj: &Bytes) -> SgResult<()> {
        let _ = self
            .client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(obj.clone().into())
            .send()
            .await
            .map_err(aws_sdk_s3::Error::from)?;
        Ok(())
    }

    pub async fn download(&self, key: impl Into<String>) -> SgResult<Bytes> {
        let obj = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(aws_sdk_s3::Error::from)?;
        let obj = obj.body.collect().await?.into_bytes();
        Ok(obj)
    }

    pub async fn download_many(
        &self,
        keys: impl Iterator<Item = impl Into<String>>,
    ) -> SgResult<Vec<Bytes>> {
        let mut objs = Vec::with_capacity(keys.size_hint().0);
        for key in keys {
            let obj = self.download(key).await?;
            objs.push(obj);
        }
        Ok(objs)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SgError {
    #[error("{0}")]
    S3(#[from] aws_sdk_s3::Error),
    #[error("{0}")]
    ByteStream(#[from] aws_smithy_http::byte_stream::error::Error),
}

pub type SgResult<T> = Result<T, SgError>;
