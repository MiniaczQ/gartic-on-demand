use super::{
    assets::{self},
    Count, Record,
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
            created_at: value.created_at,
        }
    }
}

#[derive(Debug)]
pub struct AssetEntryQuery {
    pub id: u64,
    pub kind: AssetKind,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct AssetEntry {
    kind: AssetKind,
    created_at: DateTime<Utc>,
}

impl AssetEntry {
    pub fn new(kind: AssetKind) -> Self {
        Self {
            kind,
            created_at: Utc::now(),
        }
    }
}

#[async_trait]
pub trait AssetRepository {
    async fn create_asset(&self, id: u64, entry: AssetEntry) -> Option<()>;
    async fn delete_asset(&self, id: u64) -> Option<()>;
    async fn get_random_assets(&self, kind: AssetKind, limit: u32) -> Option<Vec<AssetEntryQuery>>;
    async fn get_assets(
        &self,
        kind: AssetKind,
        limit: u64,
        start: u64,
    ) -> Option<Vec<AssetEntryQuery>>;
    async fn get_asset_count(&self, kind: AssetKind) -> Option<u64>;
}

#[async_trait]
impl<T: Connection> AssetRepository for Surreal<T> {
    async fn create_asset(&self, id: u64, entry: AssetEntry) -> Option<()> {
        self.create::<Option<Record>>((assets::TABLE, id))
            .content(entry)
            .await
            .map_err(|e| error!(error = ?e))
            .ok()
            .flatten()
            .is_some()
            .then_some(())
    }

    async fn delete_asset(&self, id: u64) -> Option<()> {
        self.delete::<Option<Record>>((assets::TABLE, id))
            .await
            .map_err(|e| error!(error = ?e))
            .ok()
            .flatten()
            .is_some()
            .then_some(())
    }

    async fn get_random_assets(&self, kind: AssetKind, n: u32) -> Option<Vec<AssetEntryQuery>> {
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
        let images: Vec<AssetEntryQueryRaw> =
            result.take(0).map_err(|e| error!(error = ?e)).ok()?;
        let images: Vec<AssetEntryQuery> = images
            .into_iter()
            .map(|e| AssetEntryQuery::try_from(e))
            .collect::<Result<_, _>>()
            .ok()?;
        Some(images)
    }

    async fn get_assets(
        &self,
        kind: AssetKind,
        limit: u64,
        start: u64,
    ) -> Option<Vec<AssetEntryQuery>> {
        let query =
            r#"SELECT * FROM type::table($table) WHERE kind = $kind LIMIT $limit START $start"#;
        let mut result = self
            .query(query)
            .bind(("table", TABLE))
            .bind(("kind", kind))
            .bind(("limit", limit))
            .bind(("start", start))
            .await
            .map_err(|e| error!(error = ?e))
            .ok()?;
        let images: Vec<AssetEntryQueryRaw> =
            result.take(0).map_err(|e| error!(error = ?e)).ok()?;
        let images: Vec<AssetEntryQuery> = images
            .into_iter()
            .map(|e| AssetEntryQuery::try_from(e))
            .collect::<Result<_, _>>()
            .ok()?;
        Some(images)
    }

    async fn get_asset_count(&self, kind: AssetKind) -> Option<u64> {
        let query = r#"SELECT count() FROM type::table($table) WHERE kind = $kind GROUP ALL"#;
        let mut result = self
            .query(query)
            .bind(("table", TABLE))
            .bind(("kind", kind))
            .await
            .map_err(|e| error!(error = ?e))
            .ok()?;
        let count: Option<Count> = result.take(0).map_err(|e| error!(error = ?e)).ok()?;
        count.map(|c| c.count)
    }
}
