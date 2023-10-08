use poise::Context;

use crate::{app::AppError, AppData};

/// Show this help menu
#[poise::command(slash_command)]
pub async fn help(
    ctx: Context<'_, AppData, AppError>,
    #[description = "Specific command to show help about"]
    #[autocomplete = "poise::builtins::autocomplete_command"]
    command: Option<String>,
) -> Result<(), AppError> {
    poise::builtins::help(
        ctx,
        command.as_deref(),
        poise::builtins::HelpConfiguration::default(),
    )
    .await
    .unwrap();
    Ok(())
}
