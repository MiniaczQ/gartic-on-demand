use config::{Config, Environment, File, FileFormat};
use dotenv::dotenv;
use lazy_static::lazy_static;
use serde::{de::DeserializeOwned, Deserialize};
use serenity::model::prelude::{ChannelId, GuildId, RoleId};

use crate::{database::DatabaseConfig, log::LogConfig, storage::StorageConfig};

lazy_static! {
    pub static ref CONFIG: AppConfig = init();
}

fn init<T: DeserializeOwned>() -> T {
    dotenv().ok();
    Config::builder()
        .add_source(File::with_name("config.json").format(FileFormat::Json))
        .add_source(Environment::default().separator("__"))
        .build()
        .expect("Failed to load configuration")
        .try_deserialize::<T>()
        .expect("Failed to deserialize configuration")
}

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub discord_token: String,
    pub guild: GuildId,
    pub channels: Channels,
    pub roles: Roles,
    pub reactions: Reactions,
    pub image: Image,
    pub log: LogConfig,
    pub database: DatabaseConfig,
    pub storage: StorageConfig,
}

#[derive(Debug, Deserialize)]
pub struct Channels {
    pub draw_this: ChannelId,
    pub in_contruction: ChannelId,
    pub moderation: ChannelId,
    pub attributes: ChannelId,
    pub complete: ChannelId,
}

#[derive(Debug, Deserialize)]
pub struct Roles {
    pub admin: RoleId,
    pub moderator: RoleId,
}

#[derive(Debug, Deserialize)]
pub struct Reactions {
    pub accept: char,
    pub reject: char,
    pub reroll: char,
    pub delete: char,
}

#[derive(Debug, Deserialize)]
pub struct Image {
    pub width: u32,
    pub height: u32,
}
