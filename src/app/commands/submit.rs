use std::borrow::Cow;

use crate::{
    app::{config::CONFIG, AppError, InternalError, UserError},
    AppData,
};
use poise::{
    serenity_prelude::{Attachment, AttachmentType},
    Context,
};
use rossbot::services::{
    database::{
        images::{Image, ImageKind, ImageRepository},
        session::SessionRepository,
    },
    image_processing::{normalize_image, RgbaConvert},
    provider::Provider,
    util::fetch_image_from_attachment,
};
use tracing::error;

#[derive(Debug, poise::ChoiceParameter)]
pub enum GamemodeArg {
    Ross,
}

#[poise::command(slash_command, dm_only)]
pub async fn submit(
    ctx: Context<'_, AppData, AppError>,
    attachment: Attachment,
) -> Result<(), AppError> {
    let sr: SessionRepository = ctx.data().get();
    let ir: ImageRepository = ctx.data().get();
    let id = ctx.author().id;

    sr.remove_expiry(id).await.map_err(|e| {
        error!(error = ?e);
        InternalError("Failed to find existing session")
    })?;

    let image = fetch_image_from_attachment(attachment)
        .await
        .ok_or(UserError("Attachment is not an image"))?;
    let image = normalize_image(&image, CONFIG.image.width, CONFIG.image.height);
    let image = AttachmentType::Bytes {
        data: Cow::Owned(image.to_png().to_vec()),
        filename: ctx.id().to_string() + ".png",
    };

    let message = CONFIG
        .channels
        .attributes
        .send_message(ctx, |m| m.add_file(image).content(format!("<@{}>", id)))
        .await?;

    ir.create(message.id.0, Image::new(ImageKind::Submission, id))
        .await
        .map_err(|e| {
            error!(error = ?e);
            InternalError("Failed to add image to database")
        })?;

    sr.attach_image(id, message.id.0).await.map_err(|e| {
        error!(error = ?e);
        InternalError("Failed to attach image to game session")
    })?;

    sr.detach_user(id).await.map_err(|e| {
        error!(error = ?e);
        InternalError("Failed to remove user session")
    })?;

    ctx.send(|f| f.content("Submited!")).await?;

    Ok(())
}
