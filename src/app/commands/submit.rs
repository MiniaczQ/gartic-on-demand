use crate::app::{
    config::CONFIG,
    error::ConvertError,
    response::ResponseContext,
    util::{extract_2x2_image, fetch_image_from_attachment, image_to_attachment},
    AppContext, AppError,
};
use poise::serenity_prelude::{Attachment, AttachmentType};
use rossbot::services::{
    database::sessionv2::SessionRepository2,
    gamemodes::GameLogic,
    image_processing::{concat_vertical, normalize_image, RgbaConvert},
    provider::Provider,
};
use std::borrow::Cow;
use tracing::error;

#[poise::command(slash_command, guild_only)]
pub async fn submit(ctx: AppContext<'_>, attachment: Attachment) -> Result<(), AppError> {
    let mut rsx = ResponseContext::new(ctx);
    rsx.init().await?;
    if let Err(e) = process(&mut rsx, ctx, attachment).await {
        error!(error = ?e);
        rsx.respond(|b| b.content(e.for_user())).await?
    }
    rsx.finalize().await?;
    Ok(())
}

pub async fn process(
    rsx: &mut ResponseContext<'_>,
    ctx: AppContext<'_>,
    attachment: Attachment,
) -> Result<(), AppError> {
    let sr: SessionRepository2 = ctx.data().get();
    let uid = ctx.author().id.0;

    let match_ = sr
        .start_submitting(uid)
        .await
        .map_internal("Failed to find existing session")?;

    let round = match_.round();
    let is_last = match_.m.mode.last_round() == round;

    let image = fetch_image_from_attachment(&attachment)
        .await
        .map_user("Attachment is not a valid image")?;

    let (image, channel) = if is_last {
        let channel = CONFIG.channels.complete;
        let attributes = extract_2x2_image(ctx, &match_).await?;
        let image = normalize_image(&image, 2 * CONFIG.image.width, 2 * CONFIG.image.height);
        let image = concat_vertical(&[attributes, image]);
        let image = AttachmentType::Bytes {
            data: Cow::Owned(image.to_png().to_vec()),
            filename: ctx.id().to_string() + ".png",
        };
        (image, channel)
    } else {
        let channel = CONFIG.channels.partial;
        let image = normalize_image(&image, CONFIG.image.width, CONFIG.image.height);
        let image = AttachmentType::Bytes {
            data: Cow::Owned(image.to_png().to_vec()),
            filename: ctx.id().to_string() + ".png",
        };
        (image, channel)
    };
    let message = channel
        .send_message(ctx, |m| m.add_file(image).content(format!("<@{}>", uid)))
        .await?;

    sr.finish_submitting(uid, message.id.0)
        .await
        .map_internal("Failed to attach image")?;

    rsx.respond(|f| f.content("Submited!")).await?;
    rsx.reset();

    if is_last {
        rsx.respond(|b| b.content("This was the final round.\nUse `/start` to play again."))
            .await?;
    } else {
        let next_round = match_.round() + 1;
        let match_ = sr
            .find_attach(uid, match_.m.mode, next_round)
            .await
            .map_user("No further rounds available currently.\nUse `/start` to play again.")?;

        let image = extract_2x2_image(ctx, &match_).await?;
        let attachment = image_to_attachment(image);
        rsx.purge().await?;
        rsx.respond(|f| f.attachment(attachment).content(match_.prompt_started()))
            .await?;
    }
    Ok(())
}
