use crate::app::{error::ConvertError, response::ResponseContext, AppContext, AppError};
use gartic_bot::services::{
    database::{attempt::AttemptRepository, user::UserRepository},
    provider::Provider,
    status_update::StatusUpdateWaker,
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
    let ar: AttemptRepository = ctx.data().get();
    let ur: UserRepository = ctx.data().get();
    let user = ctx.author();
    let user = ur
        .create_or_update_user(user.id.0, &user.name)
        .await
        .map_internal("Failed to update user")?;
    ar.cancel_active_attempt(&user)
        .await
        .map_user("No previous session")?;
    rsx.respond(|f| f.content("Cancelled previous session"))
        .await?;
    let waker: StatusUpdateWaker = ctx.data().get();
    waker.wake();
    Ok(())
}
