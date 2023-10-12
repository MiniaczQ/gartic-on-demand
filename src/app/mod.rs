use std::error::Error;

use poise::Context;
use rossbot::services::{
    database::{migrations::Migrator, Database},
    provider::Provider,
    storage::Storage,
};

use self::{config::CONFIG, error::AppError};

pub mod commands;
pub mod config;
pub mod error;
pub mod handlers;
pub mod log;
pub mod response;
pub mod util;

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

pub type AppContext<'a> = Context<'a, AppData, AppError>;
