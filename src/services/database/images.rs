use crate::services::provider::Provider;

use super::{Count, Database, DbError, DbResult, IdConvert, RawRecord, Record};
use chrono::{DateTime, Utc};
use poise::serenity_prelude::UserId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum ImageKind {
    Asset(AssetKind),
    Submission,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum AssetKind {
    InConstruction,
    DrawThis,
}

impl From<AssetKind> for ImageKind {
    fn from(value: AssetKind) -> Self {
        ImageKind::Asset(value)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Image {
    pub kind: ImageKind,
    pub author: UserId,
    pub created_at: DateTime<Utc>,
}

impl Image {
    pub fn new(kind: ImageKind, author: UserId) -> Self {
        Self {
            kind,
            author,
            created_at: Utc::now(),
        }
    }
}

pub struct ImageRepository {
    db: Database,
}

impl ImageRepository {
    pub const TABLE: &str = "images";

    pub async fn create(&self, id: u64, entry: Image) -> DbResult<()> {
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
            .ok_or(super::DbError::NotFound)?;

        Ok(())
    }

    pub async fn random(&self, kind: ImageKind, n: u32) -> DbResult<Vec<Record<Image>>> {
        let query =
            r#"SELECT * FROM type::table($table) WHERE kind = $kind ORDER BY rand() LIMIT $limit"#;
        let mut result = self
            .db
            .query(query)
            .bind(("table", Self::TABLE))
            .bind(("kind", kind))
            .bind(("limit", n))
            .await?;
        let images: Vec<RawRecord<Image>> = result.take(0)?;
        let images = images.convert_id()?;
        Ok(images)
    }

    pub async fn get(
        &self,
        kind: impl Into<ImageKind>,
        limit: u64,
        start: u64,
    ) -> DbResult<Vec<Record<Image>>> {
        let kind: ImageKind = kind.into();
        let query =
            r#"SELECT * FROM type::table($table) WHERE kind = $kind LIMIT $limit START $start"#;
        let mut result = self
            .db
            .query(query)
            .bind(("table", Self::TABLE))
            .bind(("kind", kind))
            .bind(("limit", limit))
            .bind(("start", start))
            .await?;
        let images: Vec<RawRecord<Image>> = result.take(0)?;
        let images = images.convert_id()?;
        Ok(images)
    }

    pub async fn count(&self, kind: ImageKind) -> DbResult<u64> {
        let query = r#"SELECT count() FROM type::table($table) WHERE kind = $kind GROUP ALL"#;
        let mut result = self
            .db
            .query(query)
            .bind(("table", Self::TABLE))
            .bind(("kind", kind))
            .await?;
        let count: Option<Count> = result.take(0)?;
        let count = count.ok_or(DbError::NotFound)?;
        Ok(count.count)
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
