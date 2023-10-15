use crate::app::{
    config::CONFIG,
    error::ConvertError,
    permission::is_trusted,
    response::ResponseContext,
    util::{extract_2x2_image, fetch_image_from_attachment, image_to_attachment},
    AppContext, AppError,
};
use poise::serenity_prelude::{Attachment, AttachmentType, ReactionType};
use rossbot::services::{
    database::session::{SessionRepository, SubmissionKind},
    gamemodes::GameLogic,
    image_processing::{concat_vertical, normalize_image_aoi, RgbaConvert},
    provider::Provider,
};
use std::borrow::Cow;
use tracing::error;

/// Submit an image to the current game session
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
    let sr: SessionRepository = ctx.data().get();
    let user = ctx.author();
    let uid = user.id.0;

    let lobby = sr
        .start_submitting(uid)
        .await
        .map_internal("Failed to find existing session")?;

    let round = lobby.round();
    let is_last = lobby.lobby.mode.last_round() == round;

    let image = fetch_image_from_attachment(&attachment)
        .await
        .map_user("Attachment is not a valid image")?;

    let trusted = is_trusted(&ctx, user).await?;

    let channel = match (trusted, &lobby.active.kind) {
        (false, _) => CONFIG.channels.moderation,
        (true, SubmissionKind::RossAttribute) => CONFIG.channels.partial,
        (true, SubmissionKind::RossComplete) => CONFIG.channels.complete,
    };

    let image = if is_last {
        let attributes = extract_2x2_image(ctx, &lobby).await?;
        let image = normalize_image_aoi(&image, 2 * CONFIG.image.width, 2 * CONFIG.image.height);
        let image = concat_vertical(&[attributes, image]);
        AttachmentType::Bytes {
            data: Cow::Owned(image.to_png().to_vec()),
            filename: ctx.id().to_string() + ".png",
        }
    } else {
        let image = normalize_image_aoi(&image, CONFIG.image.width, CONFIG.image.height);
        AttachmentType::Bytes {
            data: Cow::Owned(image.to_png().to_vec()),
            filename: ctx.id().to_string() + ".png",
        }
    };

    if trusted {
        let message = channel
            .send_message(ctx, |m| m.add_file(image).content(format!("<@{}>", uid)))
            .await?;
        sr.finish_submitting_trusted(uid, message.id.0).await
    } else {
        let message = channel
            .send_message(ctx, |m| {
                m.add_file(image).content(format!("<@{}>", uid)).reactions([
                    ReactionType::Unicode(CONFIG.reactions.accept.clone()),
                    ReactionType::Unicode(CONFIG.reactions.reject.clone()),
                ])
            })
            .await?;
        sr.finish_submitting_untrusted(uid, message.id.0).await
    }
    .map_internal("Failed to attach image")?;

    rsx.respond(|f| f.content("Submited!")).await?;
    rsx.reset();

    if is_last {
        rsx.respond(|b| b.content("This was the final round.\nUse `/start` to play again."))
            .await?;
    } else {
        let next_round = lobby.round() + 1;
        let lobby = sr
            .find_attach(uid, lobby.lobby.mode, next_round)
            .await
            .map_user("No further rounds available currently.\nUse `/start` to play again.")?;

        let image = extract_2x2_image(ctx, &lobby).await?;
        let attachment = image_to_attachment(image);
        rsx.purge().await?;
        rsx.respond(|f| f.attachment(attachment).content(lobby.prompt_started()))
            .await?;
    }
    Ok(())
}
