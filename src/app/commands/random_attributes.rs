use crate::app::{
    config::CONFIG,
    error::ConvertError,
    response::ResponseContext,
    util::{fetch_image_from_channel, image_to_attachment},
    AppContext, AppError,
};
use rossbot::services::{
    database::session::SessionRepository, image_processing::concat_2_2, provider::Provider,
};
use tracing::error;

/// Get 4 random attributes
#[poise::command(slash_command, guild_only)]
pub async fn random_attributes(ctx: AppContext<'_>) -> Result<(), AppError> {
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
    let count = 4;
    let ids = sr
        .random_attributes(count)
        .await
        .map_internal("Failed to fetch random attributes")?;
    if ids.len() as u64 != count {
        return None.map_user("Not enough attributes");
    }
    let mut images = Vec::with_capacity(4);
    for id in ids {
        let image = fetch_image_from_channel(ctx, CONFIG.channels.partial, id).await?;
        images.push(image);
    }
    let image = concat_2_2(&images);
    let attachment = image_to_attachment(image);
    rsx.purge().await?;
    rsx.respond(|f| f.attachment(attachment)).await?;
    Ok(())
}
