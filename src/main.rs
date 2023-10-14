pub mod app;

use app::{
    commands,
    config::CONFIG,
    error::AppError,
    handlers::{remove_asset::RemoveAsset, AssetHandler},
    stats_printer::StatsPrinter,
    AppData,
};
use poise::{serenity_prelude::Ready, Event, Framework, FrameworkContext};
use serenity::prelude::{Context, GatewayIntents};
use std::{future::Future, pin::Pin};
use tokio::spawn;

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
    let app_data = AppData::setup().await.unwrap();

    let intents = GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_MESSAGE_REACTIONS
        | GatewayIntents::GUILD_MESSAGES;

    let token = &CONFIG.discord_token;

    poise::Framework::builder()
        .token(token)
        .setup(
            move |ctx: &Context, _ready: &Ready, framework: &Framework<AppData, AppError>| {
                let stats_printer = StatsPrinter::new(app_data.db.clone(), ctx.clone());
                Box::pin(async move {
                    poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                    spawn(stats_printer.run());
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

fn event_handler(
    ctx: &Context,
    event: &Event<'_>,
    fcx: FrameworkContext<'_, AppData, AppError>,
    data: &AppData,
) -> Pin<Box<dyn Future<Output = Result<(), AppError>> + Send>> {
    for handler in [RemoveAsset] {
        if let Some(handled) = handler.handle(ctx, event, fcx, data) {
            return handled;
        }
    }
    Box::pin(async { Ok(()) })
}

fn options() -> poise::FrameworkOptions<AppData, AppError> {
    poise::FrameworkOptions {
        commands: vec![
            commands::help::help(),
            commands::add_asset::add_asset(),
            commands::start::start(),
            commands::submit::submit(),
            commands::cancel::cancel(),
            commands::current::current(),
            commands::incomplete_games::incomplete_games(),
            commands::random_attributes::random_attributes(),
        ],
        on_error: |error| Box::pin(on_error(error)),
        event_handler,
        ..Default::default()
    }
}
