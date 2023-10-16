use crate::app::{
    error::ConvertError, response::ResponseContext, util::show_round, AppContext, AppError,
};
use chrono::Utc;
use rossbot::services::{
    database::session::SessionRepository, gamemodes::GameLogic, provider::Provider,
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
    let lobby = sr.get(uid).await.map_user("No active game session")?;
    let until = Utc::now() + lobby.lobby.mode.time_limit(lobby.active.round);
    sr.extend(uid, until)
        .await
        .map_internal("Failed to extend timer")?;
    show_round(rsx, &ctx, &lobby, true).await?;
    Ok(())
}
