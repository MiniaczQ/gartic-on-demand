use crate::app::log::LogConfig;
use config::{Config, Environment, File, FileFormat};
use dotenv::dotenv;
use gartic_on_demand::services::database::DatabaseConfig;
use lazy_static::lazy_static;
use poise::serenity_prelude::MessageId;
use serde::{de::DeserializeOwned, Deserialize};
use serenity::model::prelude::{ChannelId, GuildId, RoleId};

use super::{expiry_notifier::ExpiryNotifierConfig, stats_printer::StatsPrinterConfig};

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
    pub messages: Messages,
    pub image: Image,
    pub log: LogConfig,
    pub database: DatabaseConfig,
    pub stats_printer: StatsPrinterConfig,
    pub expiry_notifier: ExpiryNotifierConfig,
}

#[derive(Debug, Deserialize)]
pub struct Channels {
    pub draw_this: ChannelId,
    pub in_contruction: ChannelId,
    pub moderation: ChannelId,
    pub partial: ChannelId,
    pub complete: ChannelId,
    pub partial_nsfw: ChannelId,
    pub complete_nsfw: ChannelId,
    pub rejects: ChannelId,
    pub stats: ChannelId,
}

#[derive(Debug, Deserialize)]
pub struct Roles {
    pub admin: RoleId,
    pub moderator: RoleId,
    pub trusted: RoleId,
    pub adult: RoleId,
    pub notify_always: RoleId,
}

#[derive(Debug, Deserialize)]
pub struct Reactions {
    pub accept: String,
    pub reject: String,
    pub reroll: String,
    pub delete: String,
}

#[derive(Debug, Deserialize)]
pub struct Image {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Deserialize)]
pub struct Messages {
    pub notify: MessageId,
}
