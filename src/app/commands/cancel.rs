use crate::app::{error::ConvertError, response::ResponseContext, AppContext, AppError};
use rossbot::services::{
    database::session::SessionRepository, provider::Provider, status_update::StatusUpdateWaker,
};
use tracing::error;

/// Cancel the current game session
#[poise::command(slash_command, guild_only)]
pub async fn cancel(ctx: AppContext<'_>) -> Result<(), AppError> {
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
    sr.cancel(uid).await.map_user("No previous session")?;
    rsx.respond(|f| f.content("Aborted previous session"))
        .await?;
    let waker: StatusUpdateWaker = ctx.data().get();
    waker.wake();
    Ok(())
}
