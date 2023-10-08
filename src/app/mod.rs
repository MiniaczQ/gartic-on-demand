use rossbot::services::{
    database::{migrations::Migrator, Database},
    provider::Provider,
    storage::Storage,
};

use self::config::CONFIG;

pub mod config;
pub mod log;

pub struct AppData {
    pub db: Database,
    pub sg: Storage,
}

impl AppData {
    pub async fn setup() -> Self {
        let db = Database::setup(&CONFIG.database).await;
        let sg = Storage::setup(&CONFIG.storage).await;
        Migrator::new(&CONFIG.database.migrator)
            .migrate(&db)
            .await
            .unwrap();
        Self { db, sg }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    Serenity(#[from] serenity::Error),
    #[error("{0}")]
    UserError(&'static str),
    #[error("{0}")]
    ApplicationError(&'static str),
}

pub use AppError::{ApplicationError, UserError};

impl<'a> Provider<'a, &'a Database> for AppData {
    fn get(&'a self) -> &'a Database {
        &self.db
    }
}

impl<'a> Provider<'a, &'a Storage> for AppData {
    fn get(&'a self) -> &'a Storage {
        &self.sg
    }
}
