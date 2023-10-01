pub mod config;
pub mod database;
pub mod handlers;
pub mod image_processing;
pub mod log;
pub mod util;

use config::CONFIG;
use handlers::{AppContext, AppHandler};
use serenity::prelude::*;
use tracing::error;

#[tokio::main]
async fn main() {
    log::setup();

    let intents = GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_MESSAGE_REACTIONS
        | GatewayIntents::GUILD_MESSAGES;

    let acx = AppContext::setup().await;
    let handler = AppHandler::new(acx);

    let mut client = Client::builder(&CONFIG.discord_token, intents)
        .event_handler(handler)
        .await
        .expect("Err creating client");

    if let Err(error) = client.start().await {
        error!("Client error: {:?}", error);
    }
}
