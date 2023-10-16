use crate::app::{
    config::CONFIG,
    error::ConvertError,
    permission::is_trusted,
    response::ResponseContext,
    util::{extract_2x2_image, fetch_image_from_attachment, session_destination, show_round},
    AppContext, AppError,
};
use poise::serenity_prelude::{Attachment, AttachmentType, ReactionType};
use rossbot::services::{
    database::session::SessionRepository,
    gamemodes::GameLogic,
    image_processing::{concat_vertical, normalize_image_aoi, RgbaConvert},
    provider::Provider,
    status_update::StatusUpdateWaker,
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

async fn process(
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

    let channel = if !trusted {
        CONFIG.channels.moderation
    } else {
        session_destination(&lobby)
    };

    let image = if is_last {
        let attributes = extract_2x2_image(&ctx, &lobby).await?;
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

    let nsfw = if lobby.lobby.nsfw { "NSFW" } else { "SFW" };
    let content = format!(
        "<@{}> - {:?} mode - round {} - {}",
        uid, lobby.lobby.mode, lobby.active.round, nsfw
    );
    if trusted {
        let message = channel
            .send_message(ctx, |m| m.add_file(image).embed(|e| e.description(content)))
            .await?;
        sr.finish_submitting_trusted(uid, message.id.0).await
    } else {
        let message = channel
            .send_message(ctx, |m| {
                m.add_file(image)
                    .embed(|e| e.description(content))
                    .reactions([
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

        show_round(rsx, &ctx, &lobby, false).await?;
    }
    let waker: StatusUpdateWaker = ctx.data().get();
    waker.wake();
    Ok(())
}
