use crate::app::{
    config::CONFIG,
    error::ConvertError,
    rendering::ModeRenderer,
    response::ResponseContext,
    util::{fetch_image_from_channel, image_to_attachment},
    AppContext, AppError,
};
use rossbot::services::{
    database::{byproducts::ByproductsRepository, ThingToU64},
    gamemodes::Mode,
    image_processing::concat_2_2,
    provider::Provider,
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

async fn process(rsx: &mut ResponseContext<'_>, ctx: AppContext<'_>) -> Result<(), AppError> {
    let br: ByproductsRepository = ctx.data().get();
    let count = 4;
    let attempts = br
        .get_random_ross_attributes()
        .await
        .map_internal("Failed to fetch random attributes")?;
    if attempts.len() as u64 != count {
        return None.map_user("Not enough attributes");
    }
    let mut images = Vec::with_capacity(4);
    let mut authors = Vec::with_capacity(4);
    for attempt in attempts {
        let image =
            fetch_image_from_channel(&ctx, CONFIG.channels.partial, attempt.state.what).await?;
        images.push(image);
        authors.push(attempt.who.to_u64());
    }
    let image = concat_2_2(&images);
    let attachment = image_to_attachment(image);
    rsx.purge().await?;
    let authors = Mode::Ross.render_partial_authors(&authors);
    rsx.respond(|f| f.content(authors).attachment(attachment))
        .await?;
    Ok(())
}
