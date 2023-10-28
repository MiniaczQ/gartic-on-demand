use crate::app::{
    error::ConvertError, response::ResponseContext, util::respond_with_prompt, AppContext, AppError,
};
use chrono::Utc;
use rossbot::services::{
    database::session::{Active, SessionRepository},
    gamemodes::GameLogic,
    provider::Provider,
};
use tracing::error;

/// Reset the expiry timer on current game session
#[poise::command(slash_command, guild_only)]
pub async fn extend(ctx: AppContext<'_>) -> Result<(), AppError> {
    let mut rsx = ResponseContext::new(ctx);
    rsx.init().await?;
    if let Err(e) = process(&mut rsx, ctx).await {
        error!(error = ?e);
        rsx.respond(|b| b.content(e.for_user())).await?
    }
    rsx.finalize().await?;
    Ok(())
}

async fn process(rsx: &mut ResponseContext<'_>, ctx: AppContext<'_>) -> Result<(), AppError> {
    let sr: SessionRepository = ctx.data().get();
    let uid = ctx.author().id.0;
    let mut lobby = sr.get(uid).await.map_user("No active game session")?;
    let new_until = Utc::now() + lobby.lobby.mode.time_limit(lobby.active.round);
    lobby.active.state.until = new_until;
    sr.extend(uid, new_until)
        .await
        .map_internal("Failed to extend timer")?;
    respond_with_prompt(rsx, &ctx, &lobby, true).await?;
    Ok(())
}
