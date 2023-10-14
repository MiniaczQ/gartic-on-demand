use std::error::Error;

use poise::Context;
use rossbot::services::{
    database::{migrations::Migrator, Database},
    provider::Provider,
};

use self::{config::CONFIG, error::AppError};

pub mod commands;
pub mod config;
pub mod error;
pub mod handlers;
pub mod log;
pub mod response;
pub mod stats_printer;
pub mod util;

pub struct AppData {
    pub db: Database,
}

impl AppData {
    pub async fn setup() -> Result<Self, Box<dyn Error>> {
        let db = Database::setup(&CONFIG.database).await?;
        Migrator::new(&CONFIG.database.migrator)
            .migrate(&db)
            .await?;
        Ok(Self { db })
    }
}

impl Provider<Database> for AppData {
    fn get(&self) -> Database {
        self.db.clone()
    }
}

pub type AppContext<'a> = Context<'a, AppData, AppError>;
