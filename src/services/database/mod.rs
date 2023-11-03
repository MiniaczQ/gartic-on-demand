pub mod assets;
pub mod attempt;
pub mod byproducts;
pub mod migrations;
pub mod round;
pub mod stats;
pub mod user;

use self::migrations::MigratorConfig;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};
use surrealdb::{
    engine::any::{connect, Any},
    sql::{Id, Thing},
    Surreal,
};

#[derive(Debug, Deserialize)]
pub struct DatabaseConfig {
    pub address: String,
    pub namespace: String,
    pub database: String,
    pub migrator: MigratorConfig,
}

#[derive(Clone)]
pub struct Database {
    inner: Surreal<Any>,
}

impl Database {
    pub async fn setup(config: &DatabaseConfig) -> DbResult<Self> {
        let inner = connect(&config.address).await.unwrap();
        inner
            .use_ns(&config.namespace)
            .use_db(&config.database)
            .await?;
        Ok(Database { inner })
    }
}

impl Deref for Database {
    type Target = Surreal<Any>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("{0}")]
    Database(#[from] surrealdb::Error),
    #[error("{0:?}")]
    DatabaseCheck(HashMap<usize, surrealdb::Error>),
    #[error("Not found")]
    NotFound,
}

pub type DbResult<T> = Result<T, DbError>;

pub trait MapToNotFound<T> {
    fn found(self) -> DbResult<T>;
}

impl<T> MapToNotFound<T> for Option<T> {
    fn found(self) -> DbResult<T> {
        self.ok_or(DbError::NotFound)
    }
}

pub trait BetterCheck
where
    Self: Sized,
{
    fn better_check(self) -> DbResult<Self>;
}

impl BetterCheck for surrealdb::Response {
    fn better_check(mut self) -> DbResult<Self> {
        let errors = self.take_errors();
        if errors.is_empty() {
            Ok(self)
        } else {
            Err(DbError::DatabaseCheck(errors))
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Record<T = ()> {
    pub id: Thing,
    #[serde(flatten)]
    pub entry: T,
}

impl<T> Record<T> {
    pub fn id(&self) -> u64 {
        self.id.to_u64()
    }
}

pub trait ThingToU64 {
    fn to_u64(&self) -> u64;
}

impl ThingToU64 for Thing {
    fn to_u64(&self) -> u64 {
        let Id::Number(id) = self.id else {
            panic!("Expected numeric id")
        };
        id.try_into().expect("Failed cast")
    }
}

impl<T> Deref for Record<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.entry
    }
}

impl<T> DerefMut for Record<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.entry
    }
}

#[cfg(test)]
mod tests {
    use crate::services::database::{
        migrations::{Migrator, MigratorConfig},
        Database,
    };
    use surrealdb::engine::any::connect;

    fn memory() -> &'static str {
        "mem://"
    }

    #[allow(dead_code)]
    fn docker() -> &'static str {
        "ws://127.0.0.1:8000"
    }

    async fn clear_db(db: &Database) {
        db.query(
            r"
            remove table migrations;
            remove table user;
            remove table round;
            remove table attempt;
            remove table previous;
            ",
        )
        .await
        .unwrap();
    }

    async fn migrate_db(db: &Database) {
        Migrator::new(&MigratorConfig {
            migrations_dir: "./migrations".into(),
        })
        .migrate(db)
        .await
        .unwrap();
    }

    pub async fn db() -> Database {
        let addr = memory();
        let db = Database {
            inner: connect(addr).await.unwrap(),
        };
        db.use_ns("test").use_db("test").await.unwrap();
        clear_db(&db).await;
        migrate_db(&db).await;
        db
    }
}
