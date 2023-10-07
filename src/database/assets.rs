use super::{
    assets::{self},
    Record,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use surrealdb::sql::{Id, Thing};
use surrealdb::{Connection, Surreal};
use tracing::error;

const TABLE: &str = "assets";

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum AssetKind {
    InConstruction,
    DrawThis,
}

#[derive(Debug, Deserialize)]
struct AssetEntryQueryRaw {
    id: Thing,
    kind: AssetKind,
    url: String,
    created_at: DateTime<Utc>,
}

impl From<AssetEntryQueryRaw> for AssetEntryQuery {
    fn from(value: AssetEntryQueryRaw) -> Self {
        let Id::Number(id) = value.id.id else {
            panic!()
        };
        AssetEntryQuery {
            id: id as u64,
            kind: value.kind,
            url: value.url,
            created_at: value.created_at,
        }
    }
}

#[derive(Debug)]
pub struct AssetEntryQuery {
    pub id: u64,
    pub kind: AssetKind,
    pub url: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct AssetEntry<'a> {
    kind: AssetKind,
    url: &'a str,
    created_at: DateTime<Utc>,
}

impl<'a> AssetEntry<'a> {
    pub fn new(kind: AssetKind, url: &'a str) -> Self {
        Self {
            kind,
            url,
            created_at: Utc::now(),
        }
    }
}

#[async_trait]
pub trait AssetRepository {
    async fn create_asset<'a>(&self, id: u64, entry: AssetEntry<'a>) -> bool;
    async fn delete_asset(&self, id: u64) -> bool;
    async fn random_assets(&self, kind: AssetKind, n: u32) -> Option<AssetEntryQuery>;
}

#[async_trait]
impl<T: Connection> AssetRepository for Surreal<T> {
    async fn create_asset<'a>(&self, id: u64, entry: AssetEntry<'a>) -> bool {
        self.create::<Option<Record>>((assets::TABLE, id))
            .content(entry)
            .await
            .map_err(|e| error!(error = ?e))
            .ok()
            .flatten()
            .is_some()
    }

    async fn delete_asset(&self, id: u64) -> bool {
        self.delete::<Option<Record>>((assets::TABLE, id))
            .await
            .map_err(|e| error!(error = ?e))
            .ok()
            .flatten()
            .is_some()
    }

    async fn random_assets(&self, kind: AssetKind, n: u32) -> Option<AssetEntryQuery> {
        let query =
            r#"SELECT * FROM type::table($table) WHERE kind = $kind ORDER BY rand() LIMIT $limit"#;
        let mut result = self
            .query(query)
            .bind(("table", TABLE))
            .bind(("kind", kind))
            .bind(("limit", n))
            .await
            .map_err(|e| error!(error = ?e))
            .ok()?;
        let image: Option<AssetEntryQueryRaw> =
            result.take(0).map_err(|e| error!(error = ?e)).ok()?;
        let image = image?.into();
        Some(image)
    }
}
