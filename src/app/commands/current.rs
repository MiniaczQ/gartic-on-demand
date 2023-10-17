use crate::app::{
    error::ConvertError, response::ResponseContext, util::respond_with_prompt, AppContext, AppError,
};
use rossbot::services::{
    database::session::SessionRepository, provider::Provider, status_update::StatusUpdateWaker,
};
use tracing::error;

/// Get current game session
#[poise::command(slash_command, guild_only)]
pub async fn current(ctx: AppContext<'_>) -> Result<(), AppError> {
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
    let waker: StatusUpdateWaker = ctx.data().get();
    waker.wake();
    let lobby = sr.get(uid).await.map_user("No current game")?;
    respond_with_prompt(rsx, &ctx, &lobby, false).await?;
    Ok(())
}
