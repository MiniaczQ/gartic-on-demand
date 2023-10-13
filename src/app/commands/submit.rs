use std::{borrow::Cow, ops::Add};

use crate::app::{
    config::CONFIG,
    error::ConvertError,
    response::ResponseContext,
    util::{display_started_round, extract_2x2_image, fetch_image_from_attachment},
    AppContext, AppError,
};
use chrono::Utc;
use poise::serenity_prelude::{Attachment, AttachmentType};
use rossbot::services::{
    database::{
        images::{Image, ImageKind, ImageRepository},
        session::{SessionRepository, UserSession},
    },
    gamemodes::GameLogic,
    image_processing::{concat_vertical, normalize_image, RgbaConvert},
    provider::Provider,
};
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
    let sr: SessionRepository = ctx.data().get();
    let ir: ImageRepository = ctx.data().get();
    let user_id = ctx.author().id;

    let session = sr
        .remove_expiry(user_id)
        .await
        .map_internal("Failed to find existing session")?;

    let round = session.game.round();
    let is_last = session.game.mode.last_round() == round;

    let image = fetch_image_from_attachment(&attachment)
        .await
        .map_user("Attachment is not a valid image")?;

    let (image, channel) = if is_last {
        let channel = CONFIG.channels.complete;
        let attributes = extract_2x2_image(ctx, &session).await?;
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
        .send_message(ctx, |m| {
            m.add_file(image).content(format!("<@{}>", user_id))
        })
        .await?;

    ir.create(message.id.0, Image::new(ImageKind::Submission, user_id))
        .await
        .map_internal("Failed to add image to database")?;

    sr.attach_image(user_id, message.id.0)
        .await
        .map_internal("Failed to attach image to game session")?;

    let game = sr
        .detach_user(user_id)
        .await
        .map_internal("Failed to remove user session")?;

    rsx.respond(|f| f.content("Submited!")).await?;
    rsx.reset();

    if is_last {
        rsx.respond(|b| b.content("This was the final round.\nUse `/start` to play again."))
            .await?;
    } else {
        let next_round = game.round();
        let user = UserSession {
            user_id,
            expires_at: Utc::now().add(game.mode.time_limit(next_round)),
        };

        let session = sr
            .find_game_for_user(game.mode, next_round, user)
            .await
            .map_user("No further rounds available currently.\nUse `/start` to play again.")?;

        display_started_round(rsx, ctx, session).await?;
    }
    Ok(())
}
