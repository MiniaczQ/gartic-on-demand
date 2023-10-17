use std::borrow::Cow;

use async_trait::async_trait;
use poise::serenity_prelude::{Attachment, AttachmentType};
use rossbot::services::{
    database::{
        assets::ImageRepository,
        session::{Active, LobbyWithSessions},
    },
    gamemodes::Mode,
    image_processing::{concat_vertical, normalize_image_aoi, RgbaConvert},
};
use serenity::http::Http;

use super::{
    config::CONFIG,
    error::{AppError, ConvertError},
    util::{extract_2x2_image, fetch_image_from_attachment},
};

#[async_trait]
pub trait Renderer {
    async fn render_prompt(
        &self,
        ctx: &(impl AsRef<Http> + Send + Sync),
        lobby: &LobbyWithSessions<Active>,
        ir: &ImageRepository,
    ) -> Result<AttachmentType<'static>, AppError>;

    async fn render_partial(
        &self,
        attachment: &Attachment,
    ) -> Result<AttachmentType<'static>, AppError>;

    async fn render_complete<T: Send + Sync>(
        &self,
        ctx: &(impl AsRef<Http> + Send + Sync),
        lobby: &LobbyWithSessions<T>,
        ir: &ImageRepository,
        attachment: &Attachment,
    ) -> Result<AttachmentType<'static>, AppError>;
}

#[async_trait]
impl Renderer for Mode {
    async fn render_prompt(
        &self,
        ctx: &(impl AsRef<Http> + Send + Sync),
        lobby: &LobbyWithSessions<Active>,
        ir: &ImageRepository,
    ) -> Result<AttachmentType<'static>, AppError> {
        let image = extract_2x2_image(ctx, lobby, ir).await?;
        let attachment = AttachmentType::Bytes {
            data: std::borrow::Cow::Owned(image.to_png()),
            filename: "prompt.png".to_owned(),
        };
        Ok(attachment)
    }

    async fn render_partial(
        &self,
        attachment: &Attachment,
    ) -> Result<AttachmentType<'static>, AppError> {
        let image = fetch_image_from_attachment(attachment)
            .await
            .map_user("Attachment is not a valid image")?;
        let image = normalize_image_aoi(&image, CONFIG.image.width, CONFIG.image.height);
        let attachment = AttachmentType::Bytes {
            data: Cow::Owned(image.to_png().to_vec()),
            filename: "partial.png".to_owned(),
        };
        Ok(attachment)
    }

    async fn render_complete<T: Send + Sync>(
        &self,
        ctx: &(impl AsRef<Http> + Send + Sync),
        lobby: &LobbyWithSessions<T>,
        ir: &ImageRepository,
        attachment: &Attachment,
    ) -> Result<AttachmentType<'static>, AppError> {
        let image = fetch_image_from_attachment(attachment)
            .await
            .map_user("Attachment is not a valid image")?;
        let attributes = extract_2x2_image(&ctx, lobby, ir).await?;
        let image = normalize_image_aoi(&image, 2 * CONFIG.image.width, 2 * CONFIG.image.height);
        let image = concat_vertical(&[attributes, image]);
        let attachment = AttachmentType::Bytes {
            data: Cow::Owned(image.to_png().to_vec()),
            filename: "complete.png".to_owned(),
        };
        Ok(attachment)
    }
}
