pub mod assets;
pub mod attributes;
pub mod migrations;

use serde::Deserialize;
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

impl DatabaseConfig {
    pub async fn connect(&self) -> Database {
        let db = connect(&self.address).await.unwrap();
        db.use_ns(&self.namespace)
            .use_db(&self.database)
            .await
            .unwrap();
        db
    }
}

#[derive(Deserialize)]
pub struct Record {
    #[allow(dead_code)]
    id: Thing,
}

pub type Database = Surreal<Any>;
