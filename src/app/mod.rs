use self::{config::CONFIG, error::AppError};
use gartic_on_demand::services::{
    database::{migrations::Migrator, Database},
    provider::Provider,
    status_update::StatusUpdateWaker,
};
use poise::Context;
use std::error::Error;

pub mod commands;
pub mod config;
pub mod error;
pub mod handlers;
pub mod log;
pub mod permission;
pub mod rendering;
pub mod response;
pub mod stats_printer;
pub mod util;

#[derive(Clone)]
pub struct AppData {
    db: Database,
    sw: StatusUpdateWaker,
}

impl AppData {
    pub async fn setup(sw: StatusUpdateWaker) -> Result<Self, Box<dyn Error>> {
        let db = Database::setup(&CONFIG.database).await?;
        Migrator::new(&CONFIG.database.migrator)
            .migrate(&db)
            .await?;
        Ok(Self { db, sw })
    }
}

impl Provider<Database> for AppData {
    fn get(&self) -> Database {
        self.db.clone()
    }
}

impl Provider<StatusUpdateWaker> for AppData {
    fn get(&self) -> StatusUpdateWaker {
        self.sw.clone()
    }
}

pub type AppContext<'a> = Context<'a, AppData, AppError>;
