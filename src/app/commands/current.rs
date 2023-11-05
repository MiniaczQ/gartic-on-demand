use crate::app::{
    error::ConvertError, response::ResponseContext, util::respond_with_prompt, AppContext, AppError,
};
use gartic_on_demand::services::{
    database::{attempt::AttemptRepository, round::RoundRepository, user::UserRepository},
    provider::Provider,
    status_update::StatusUpdateWaker,
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
    let ar: AttemptRepository = ctx.data().get();
    let ur: UserRepository = ctx.data().get();
    let rr: RoundRepository = ctx.data().get();
    let user = ctx.author();
    let user = ur
        .create_or_update_user(user.id.0, &user.name)
        .await
        .map_internal("Failed to update user")?;
    ar.expire_active_attempts()
        .await
        .map_internal("Failed to unlock expired sessions")?;
    let waker: StatusUpdateWaker = ctx.data().get();
    waker.wake();
    let lobby = rr
        .get_active_round(&user)
        .await
        .map_user("No current game")?;
    respond_with_prompt(rsx, &ctx, &lobby, false).await?;
    Ok(())
}
