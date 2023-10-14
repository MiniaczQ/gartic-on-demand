use crate::app::{
    error::ConvertError,
    response::ResponseContext,
    util::{extract_2x2_image, image_to_attachment},
    AppContext, AppError,
};
use rossbot::services::{database::sessionv2::SessionRepository2, provider::Provider};
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
    let sr: SessionRepository2 = ctx.data().get();
    let uid = ctx.author().id.0;
    let match_ = sr.get(uid).await.map_user("No current game")?;
    let image = extract_2x2_image(ctx, &match_).await?;
    let attachment = image_to_attachment(image);
    rsx.purge().await?;
    rsx.respond(|f| {
        f.attachment(attachment)
            .content(match_.prompt_already_running())
    })
    .await?;
    Ok(())
}
