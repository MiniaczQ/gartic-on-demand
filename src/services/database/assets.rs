use crate::services::provider::Provider;

use super::{Database, DbResult, IdConvert, MapToNotFound, RawRecord, Record};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum AssetKind {
    InConstruction,
    DrawThis,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Asset {
    pub kind: AssetKind,
    pub author: u64,
    pub created_at: DateTime<Utc>,
}

impl Asset {
    pub fn new(kind: AssetKind, uid: u64) -> Self {
        Self {
            kind,
            author: uid,
            created_at: Utc::now(),
        }
    }
}

pub struct ImageRepository {
    db: Database,
}

impl ImageRepository {
    pub const TABLE: &str = "assets";

    pub async fn create(&self, id: u64, entry: Asset) -> DbResult<()> {
        self.db
            .create::<Option<RawRecord>>((Self::TABLE, id))
            .content(entry)
            .await?
            .ok_or(super::DbError::NotFound)?;

        Ok(())
    }

    pub async fn delete(&self, id: u64) -> DbResult<()> {
        self.db
            .delete::<Option<RawRecord>>((Self::TABLE, id))
            .await?
            .found()?;

        Ok(())
    }

    pub async fn random(&self, kind: AssetKind, n: u32) -> DbResult<Vec<Record<Asset>>> {
        let query = r#"SELECT * FROM assets WHERE kind = $kind ORDER BY rand() LIMIT $limit"#;
        let mut result = self
            .db
            .query(query)
            .bind(("kind", kind))
            .bind(("limit", n))
            .await?;
        let images = result.take::<Vec<RawRecord<Asset>>>(0)?.convert_id()?;
        Ok(images)
    }
}

impl<T> Provider<ImageRepository> for T
where
    T: Provider<Database>,
{
    fn get(&self) -> ImageRepository {
        ImageRepository { db: self.get() }
    }
}
