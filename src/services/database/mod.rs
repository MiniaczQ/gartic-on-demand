pub mod assets;
pub mod attributes;
pub mod migrations;

use std::ops::Deref;

use serde::Deserialize;
use serenity::prelude::TypeMapKey;
use surrealdb::{
    engine::any::{connect, Any},
    sql::Thing,
    Surreal,
};

use self::migrations::MigratorConfig;

#[derive(Debug, Deserialize)]
pub struct DatabaseConfig {
    pub address: String,
    pub namespace: String,
    pub database: String,
    pub migrator: MigratorConfig,
}

impl Database {
    pub async fn setup(config: &DatabaseConfig) -> Self {
        let inner = connect(&config.address).await.unwrap();
        inner
            .use_ns(&config.namespace)
            .use_db(&config.database)
            .await
            .unwrap();
        Database { inner }
    }
}

#[derive(Deserialize)]
pub struct Record {
    #[allow(dead_code)]
    id: Thing,
}

#[derive(Deserialize)]
pub struct Count {
    pub count: u64,
}

pub struct Database {
    inner: Surreal<Any>,
}

impl Deref for Database {
    type Target = Surreal<Any>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl TypeMapKey for Database {
    type Value = Database;
}
