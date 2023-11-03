pub mod app;

use app::{
    commands,
    config::CONFIG,
    error::AppError,
    handlers::{
        accept_submission::AcceptSubmission, notify_activity::NotifyActivity,
        remove_asset::RemoveAsset, AssetHandler,
    },
    stats_printer::StatsPrinter,
    AppData,
};
use poise::{serenity_prelude::Ready, Event, Framework, FrameworkContext};
use rossbot::services::{provider::Provider, status_update::status_update_pair};
use serenity::prelude::{Context, GatewayIntents};
use std::{future::Future, pin::Pin};
use tokio::spawn;
use tracing::error;

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

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    app::log::setup();
    let options = options();
    let (waker, waiter) = status_update_pair();
    let app_data = AppData::setup(waker).await.unwrap();

    let intents = GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_MESSAGE_REACTIONS
        | GatewayIntents::GUILD_MESSAGES;

    let token = &CONFIG.discord_token;

    poise::Framework::builder()
        .token(token)
        .setup(
            move |ctx: &Context, _ready: &Ready, framework: &Framework<AppData, AppError>| {
                let stats_printer = StatsPrinter::new(app_data.get(), waiter, ctx.clone());
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

fn event_handler<'a>(
    ctx: &Context,
    event: &Event<'a>,
    fcx: FrameworkContext<'a, AppData, AppError>,
    data: &AppData,
) -> Pin<Box<dyn Future<Output = Result<(), AppError>> + Send + 'a>> {
    let ctx = ctx.clone();
    let event = event.clone();
    let data = data.clone();
    Box::pin(async move {
        let handlers: &[&dyn AssetHandler] = &[&RemoveAsset, &AcceptSubmission, &NotifyActivity];
        for handler in handlers {
            if let Err(e) = handler.handle(&ctx, &event, fcx, &data).await {
                error!(error = %e, handler = ?handler, "Error in handler");
            }
        }
        Ok(())
    })
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
            commands::random_attributes::random_attributes(),
            commands::purge::purge(),
            commands::extend::extend(),
            commands::reroll::reroll(),
        ],
        on_error: |error| Box::pin(on_error(error)),
        event_handler,
        ..Default::default()
    }
}
