pub mod app;
pub mod commands;

use app::{config::CONFIG, AppData, AppError};
use poise::{serenity_prelude::Ready, Framework};
use serenity::prelude::{Context, GatewayIntents};

async fn on_error(error: poise::FrameworkError<'_, AppData, AppError>) {
    match error {
        poise::FrameworkError::Setup { error, .. } => panic!("Failed to start bot: {:?}", error),
        poise::FrameworkError::Command { error, ctx } => {
            println!("Error in command `{}`: {:?}", ctx.command().name, error,);
        }
        error => {
            if let Err(e) = poise::builtins::on_error(error).await {
                println!("Error while handling error: {}", e)
            }
        }
    }
}

#[tokio::main]
async fn main() {
    app::log::setup();
    let options = options();
    let app_data = AppData::setup().await;

    let intents = GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_MESSAGE_REACTIONS
        | GatewayIntents::GUILD_MESSAGES;

    let token = &CONFIG.discord_token;

    poise::Framework::builder()
        .token(token)
        .setup(
            move |ctx: &Context, _ready: &Ready, framework: &Framework<AppData, AppError>| {
                Box::pin(async move {
                    poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                    Ok(app_data)
                })
            },
        )
        .options(options)
        .intents(intents)
        .run()
        .await
        .unwrap();
}

fn options() -> poise::FrameworkOptions<AppData, AppError> {
    poise::FrameworkOptions {
        commands: vec![
            commands::help::help(),
            commands::assets::add_asset(),
            commands::assets::show_assets(),
            commands::assets::remove_asset(),
        ],
        on_error: |error| Box::pin(on_error(error)),
        ..Default::default()
    }
}
