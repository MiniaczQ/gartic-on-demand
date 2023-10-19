use std::borrow::Cow;

use async_trait::async_trait;
use poise::serenity_prelude::{Attachment, AttachmentType};
use rossbot::services::{
    database::{
        assets::ImageRepository,
        session::{Active, LobbyWithSessions},
    },
    gamemodes::{ross::Ross, Mode},
    image_processing::{concat_vertical, normalize_image_aoi, RgbaConvert},
};
use serenity::http::Http;

use super::{
    config::CONFIG,
    error::{AppError, ConvertError},
    util::{extract_2x2_image, fetch_image_from_attachment},
};

#[async_trait]
pub trait ModeRenderer {
    async fn render_prompt_image(
        &self,
        ctx: &(impl AsRef<Http> + Send + Sync),
        lobby: &LobbyWithSessions<Active>,
        ir: &ImageRepository,
    ) -> Result<AttachmentType<'static>, AppError>;

    async fn render_partial_image(
        &self,
        attachment: &Attachment,
    ) -> Result<AttachmentType<'static>, AppError>;

    async fn render_complete_image<T: Send + Sync>(
        &self,
        ctx: &(impl AsRef<Http> + Send + Sync),
        lobby: &LobbyWithSessions<T>,
        ir: &ImageRepository,
        attachment: &Attachment,
    ) -> Result<AttachmentType<'static>, AppError>;

    fn render_partial_author(&self, author: u64) -> String;
    fn render_complete_authors(&self, author: u64, others: &[u64]) -> String;
}

#[async_trait]
impl ModeRenderer for Mode {
    async fn render_prompt_image(
        &self,
        ctx: &(impl AsRef<Http> + Send + Sync),
        lobby: &LobbyWithSessions<Active>,
        ir: &ImageRepository,
    ) -> Result<AttachmentType<'static>, AppError> {
        match &self {
            Mode::Ross => Ross.render_prompt_image(ctx, lobby, ir).await,
        }
    }

    async fn render_partial_image(
        &self,
        attachment: &Attachment,
    ) -> Result<AttachmentType<'static>, AppError> {
        match &self {
            Mode::Ross => Ross.render_partial_image(attachment).await,
        }
    }

    async fn render_complete_image<T: Send + Sync>(
        &self,
        ctx: &(impl AsRef<Http> + Send + Sync),
        lobby: &LobbyWithSessions<T>,
        ir: &ImageRepository,
        attachment: &Attachment,
    ) -> Result<AttachmentType<'static>, AppError> {
        match &self {
            Mode::Ross => Ross.render_complete_image(ctx, lobby, ir, attachment).await,
        }
    }

    fn render_partial_author(&self, author: u64) -> String {
        match &self {
            Mode::Ross => Ross.render_partial_author(author),
        }
    }

    fn render_complete_authors(&self, author: u64, others: &[u64]) -> String {
        match &self {
            Mode::Ross => Ross.render_complete_authors(author, others),
        }
    }
}

#[async_trait]
impl ModeRenderer for Ross {
    async fn render_prompt_image(
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

    async fn render_partial_image(
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

    async fn render_complete_image<T: Send + Sync>(
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

    fn render_partial_author(&self, author: u64) -> String {
        format!("<@{}>", author)
    }

    fn render_complete_authors(&self, author: u64, others: &[u64]) -> String {
        let others = others
            .iter()
            .map(|author| format!("<@{}>", author))
            .collect::<Vec<_>>();
        let others = others.join(", ");
        format!("<@{}>, attributes by {}", author, others)
    }
}

pub trait LobbyRenderer {
    fn render_partial_text(&self) -> String;
    fn render_complete_text(&self) -> String;
}

impl<T> LobbyRenderer for LobbyWithSessions<T> {
    fn render_partial_text(&self) -> String {
        let sfw: &str = if self.lobby.nsfw { "NSFW " } else { "" };
        let content = format!(
            "{}{:?} mode round {} by {}",
            sfw,
            self.active.mode,
            self.active.round + 1,
            self.active.mode.render_partial_author(self.active.who)
        );
        content
    }

    fn render_complete_text(&self) -> String {
        let sfw: &str = if self.lobby.nsfw { "NSFW " } else { "" };
        let others = self.accepted.iter().map(|a| a.who).collect::<Vec<_>>();
        let content = format!(
            "{}{:?} mode round {} by {}",
            sfw,
            self.active.mode,
            self.active.round + 1,
            self.active
                .mode
                .render_complete_authors(self.active.who, &others)
        );
        content
    }
}
