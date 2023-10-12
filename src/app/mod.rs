use std::error::Error;

use rossbot::services::{
    database::{migrations::Migrator, Database},
    provider::Provider,
    storage::Storage,
};

use self::config::CONFIG;

pub mod config;
pub mod log;
pub mod commands;
pub mod handlers;

pub struct AppData {
    pub db: Database,
    pub sg: Storage,
}

impl AppData {
    pub async fn setup() -> Result<Self, Box<dyn Error>> {
        let db = Database::setup(&CONFIG.database).await?;
        let sg = Storage::setup(&CONFIG.storage).await?;
        Migrator::new(&CONFIG.database.migrator)
            .migrate(&db)
            .await?;
        Ok(Self { db, sg })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    Serenity(#[from] serenity::Error),
    #[error("{0}")]
    UserError(&'static str),
    #[error("{0}")]
    InternalError(&'static str),
}

pub use AppError::{InternalError, UserError};

impl Provider<Database> for AppData {
    fn get(&self) -> Database {
        self.db.clone()
    }
}

impl Provider<Storage> for AppData {
    fn get(&self) -> Storage {
        self.sg.clone()
    }
}
