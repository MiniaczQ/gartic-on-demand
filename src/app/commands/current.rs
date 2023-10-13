use crate::app::{
    error::ConvertError, response::ResponseContext, util::display_already_running_round,
    AppContext, AppError,
};
use rossbot::services::{database::session::SessionRepository, provider::Provider};
use tracing::error;

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

pub async fn process(rsx: &mut ResponseContext<'_>, ctx: AppContext<'_>) -> Result<(), AppError> {
    let sr: SessionRepository = ctx.data().get();
    let user_id = ctx.author().id;
    let session = sr
        .get_current_user_game(user_id)
        .await
        .map_user("No current game")?;
    display_already_running_round(rsx, ctx, session).await?;
    Ok(())
}
