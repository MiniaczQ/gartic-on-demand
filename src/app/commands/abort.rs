use crate::{
    app::{AppError, InternalError},
    AppData,
};
use poise::Context;
use rossbot::services::{database::session::SessionRepository, provider::Provider};
use tracing::error;

#[derive(Debug, poise::ChoiceParameter)]
pub enum GamemodeArg {
    Ross,
}

#[poise::command(slash_command, dm_only)]
pub async fn abort(ctx: Context<'_, AppData, AppError>) -> Result<(), AppError> {
    let sr: SessionRepository = ctx.data().get();
    let id = ctx.author().id;

    sr.detach_user(id).await.map_err(|e| {
        error!(error = ?e);
        InternalError("No prevoious session")
    })?;

    ctx.send(|f| f.content("Aborted previous session")).await?;

    Ok(())
}
