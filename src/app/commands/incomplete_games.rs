use crate::app::{error::ConvertError, response::ResponseContext, AppContext, AppError};
use rossbot::services::{database::session::SessionRepository, provider::Provider};
use tracing::error;

/// Show incomplete games in which you can participate through `round` parameter in `/start` command
#[poise::command(slash_command, guild_only)]
pub async fn incomplete_games(ctx: AppContext<'_>) -> Result<(), AppError> {
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
    sr.stop_expired()
        .await
        .map_internal("Failed to unlock expired sessions")?;
    let mut response = sr
        .incomplete_games_for_user(uid)
        .await
        .map_user("No current game")?
        .iter()
        .map(|p| p.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    if response.is_empty() {
        response.push_str("No incomplete games to join");
    }
    rsx.respond(|b| b.content(response)).await?;
    Ok(())
}
