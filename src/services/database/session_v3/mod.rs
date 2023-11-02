use std::ops::{Deref, DerefMut};

use serde::{Deserialize, Serialize};
use surrealdb::sql::{Id, Thing};

pub mod attempt;
pub mod byproducts;
pub mod round;
pub mod stats;
pub mod user;

#[derive(Debug, Serialize, Deserialize)]
pub struct Record<T = ()> {
    pub id: Thing,
    #[serde(flatten)]
    pub entry: T,
}

impl<T> Record<T> {
    pub fn id(&self) -> u64 {
        let Id::Number(id) = self.id.id else {
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
