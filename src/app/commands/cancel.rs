use crate::app::{error::ConvertError, response::ResponseContext, AppContext, AppError};
use rossbot::services::{database::session::SessionRepository, provider::Provider};
use tracing::error;

#[derive(Debug, poise::ChoiceParameter)]
pub enum GamemodeArg {
    Ross,
}

#[poise::command(slash_command, guild_only)]
pub async fn cancel(ctx: AppContext<'_>) -> Result<(), AppError> {
    let mut rsx = ResponseContext::new(ctx);
    if let Err(e) = process(&mut rsx, ctx).await {
        error!(error = ?e);
        rsx.respond(|b| b.content(e.for_user())).await?
    }
    rsx.finalize().await?;
    Ok(())
}

pub async fn process(rsx: &mut ResponseContext<'_>, ctx: AppContext<'_>) -> Result<(), AppError> {
    let sr: SessionRepository = ctx.data().get();
    let id = ctx.author().id;
    sr.detach_user(id).await.map_user("No prevoious session")?;
    rsx.respond(|f| f.content("Aborted previous session"))
        .await?;
    Ok(())
}
