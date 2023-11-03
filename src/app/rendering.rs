use std::borrow::Cow;

use async_trait::async_trait;
use poise::serenity_prelude::{Attachment, AttachmentType};
use rossbot::services::{
    database::{assets::ImageRepository, attempt::Active, round::RoundWithAttempts, ThingToU64},
    gamemodes::{evolution::Evolution, ross::Ross, Mode},
    image_processing::{concat_vertical, normalize_image_aoi, RgbaConvert},
};
use serenity::http::Http;

use super::{
    config::CONFIG,
    error::{AppError, ConvertError},
    util::{extract_2x2_image, extract_nx1_image, fetch_image_from_attachment},
};

#[async_trait]
pub trait ModeRenderer {
    async fn render_prompt_image(
        &self,
        ctx: &(impl AsRef<Http> + Send + Sync),
        round: &RoundWithAttempts<Active>,
        ir: &ImageRepository,
    ) -> Result<AttachmentType<'static>, AppError>;

    async fn render_partial_image(
        &self,
        attachment: &Attachment,
    ) -> Result<AttachmentType<'static>, AppError>;

    async fn render_complete_image<T: Send + Sync>(
        &self,
        ctx: &(impl AsRef<Http> + Send + Sync),
        round: &RoundWithAttempts<T>,
        ir: &ImageRepository,
        attachment: &Attachment,
    ) -> Result<AttachmentType<'static>, AppError>;

    fn render_partial_author(&self, author: u64) -> String;
    fn render_partial_authors(&self, authors: &[u64]) -> String;
    fn render_complete_authors(&self, author: u64, others: &[u64]) -> String;
}

#[async_trait]
impl ModeRenderer for Mode {
    async fn render_prompt_image(
        &self,
        ctx: &(impl AsRef<Http> + Send + Sync),
        round: &RoundWithAttempts<Active>,
        ir: &ImageRepository,
    ) -> Result<AttachmentType<'static>, AppError> {
        match &self {
            Mode::Ross => Ross.render_prompt_image(ctx, round, ir).await,
            Mode::Evolution => Evolution.render_prompt_image(ctx, round, ir).await,
        }
    }

    async fn render_partial_image(
        &self,
        attachment: &Attachment,
    ) -> Result<AttachmentType<'static>, AppError> {
        match &self {
            Mode::Ross => Ross.render_partial_image(attachment).await,
            Mode::Evolution => Evolution.render_partial_image(attachment).await,
        }
    }

    async fn render_complete_image<T: Send + Sync>(
        &self,
        ctx: &(impl AsRef<Http> + Send + Sync),
        round: &RoundWithAttempts<T>,
        ir: &ImageRepository,
        attachment: &Attachment,
    ) -> Result<AttachmentType<'static>, AppError> {
        match &self {
            Mode::Ross => Ross.render_complete_image(ctx, round, ir, attachment).await,
            Mode::Evolution => {
                Evolution
                    .render_complete_image(ctx, round, ir, attachment)
                    .await
            }
        }
    }

    fn render_partial_author(&self, author: u64) -> String {
        match &self {
            Mode::Ross => Ross.render_partial_author(author),
            Mode::Evolution => Evolution.render_partial_author(author),
        }
    }

    fn render_partial_authors(&self, authors: &[u64]) -> String {
        match &self {
            Mode::Ross => Ross.render_partial_authors(authors),
            Mode::Evolution => Evolution.render_partial_authors(authors),
        }
    }

    fn render_complete_authors(&self, author: u64, others: &[u64]) -> String {
        match &self {
            Mode::Ross => Ross.render_complete_authors(author, others),
            Mode::Evolution => Evolution.render_complete_authors(author, others),
        }
    }
}

#[async_trait]
impl ModeRenderer for Ross {
    async fn render_prompt_image(
        &self,
        ctx: &(impl AsRef<Http> + Send + Sync),
        round: &RoundWithAttempts<Active>,
        ir: &ImageRepository,
    ) -> Result<AttachmentType<'static>, AppError> {
        let image = extract_2x2_image(ctx, ir, &round.previous, round.round.nsfw).await?;
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
        round: &RoundWithAttempts<T>,
        ir: &ImageRepository,
        attachment: &Attachment,
    ) -> Result<AttachmentType<'static>, AppError> {
        let image = fetch_image_from_attachment(attachment)
            .await
            .map_user("Attachment is not a valid image")?;
        let attributes = extract_2x2_image(&ctx, ir, &round.previous, round.round.nsfw).await?;
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

    fn render_partial_authors(&self, authors: &[u64]) -> String {
        let authors = authors
            .iter()
            .map(|author| format!("<@{}>", author))
            .collect::<Vec<_>>();
        authors.join(", ")
    }

    fn render_complete_authors(&self, author: u64, others: &[u64]) -> String {
        let others = self.render_partial_authors(others);
        format!("<@{}>, attributes by {}", author, others)
    }
}

#[async_trait]
impl ModeRenderer for Evolution {
    async fn render_prompt_image(
        &self,
        ctx: &(impl AsRef<Http> + Send + Sync),
        round: &RoundWithAttempts<Active>,
        ir: &ImageRepository,
    ) -> Result<AttachmentType<'static>, AppError> {
        let image = extract_nx1_image(ctx, ir, &round.previous, round.round.nsfw, 3).await?;
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
        round: &RoundWithAttempts<T>,
        ir: &ImageRepository,
        attachment: &Attachment,
    ) -> Result<AttachmentType<'static>, AppError> {
        let image = fetch_image_from_attachment(attachment)
            .await
            .map_user("Attachment is not a valid image")?;
        let previous = extract_nx1_image(ctx, ir, &round.previous, round.round.nsfw, 2).await?;
        let image = normalize_image_aoi(&image, CONFIG.image.width, CONFIG.image.height);
        let image = concat_vertical(&[previous, image]);
        let attachment = AttachmentType::Bytes {
            data: Cow::Owned(image.to_png().to_vec()),
            filename: "complete.png".to_owned(),
        };
        Ok(attachment)
    }

    fn render_partial_author(&self, author: u64) -> String {
        format!("<@{}>", author)
    }

    fn render_partial_authors(&self, authors: &[u64]) -> String {
        let authors = authors
            .iter()
            .map(|author| format!("<@{}>", author))
            .collect::<Vec<_>>();
        authors.join(", ")
    }

    fn render_complete_authors(&self, author: u64, others: &[u64]) -> String {
        let others = self.render_partial_authors(others);
        format!("By {}, <@{}>", others, author)
    }
}

pub trait RoundRenderer {
    fn render_partial_text(&self) -> String;
    fn render_complete_text(&self) -> String;
}

impl<T> RoundRenderer for RoundWithAttempts<T> {
    fn render_partial_text(&self) -> String {
        let sfw: &str = if self.round.nsfw { "NSFW " } else { "" };
        let content = format!(
            "{}{:?} mode round {} by {}",
            sfw,
            self.round.mode,
            self.round.round_no + 1,
            self.round
                .mode
                .render_partial_author(self.attempt.who.to_u64())
        );
        content
    }

    fn render_complete_text(&self) -> String {
        let sfw: &str = if self.round.nsfw { "NSFW " } else { "" };
        let others = self
            .previous
            .iter()
            .map(|a| a.who.to_u64())
            .collect::<Vec<_>>();
        let content = format!(
            "{}{:?} mode round {} by {}",
            sfw,
            self.round.mode,
            self.round.round_no + 1,
            self.round
                .mode
                .render_complete_authors(self.attempt.who.to_u64(), &others)
        );
        content
    }
}
