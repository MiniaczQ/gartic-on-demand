use super::{
    config::CONFIG,
    error::{AppError, ConvertError},
    rendering::ModeRenderer,
    response::ResponseContext,
    AppContext,
};
use bytes::Bytes;
use gartic_on_demand::services::{
    database::{
        assets::{AssetKind, ImageRepository},
        attempt::{Active, Approved, Attempt},
        round::RoundWithAttempts,
        Record,
    },
    gamemodes::GameLogic,
    image_processing::{concat_2_2, concat_vertical, RgbaConvert},
    provider::Provider,
};
use image::RgbaImage;
use mime::IMAGE_PNG;
use poise::serenity_prelude::{Attachment, AttachmentType, ChannelId, MessageId};
use reqwest::header::{self, HeaderValue};
use serenity::http::Http;

#[derive(Debug, thiserror::Error)]
pub enum FetchError {
    #[error("Expected discord PNG image, got {0:?}")]
    DiscordInvalidContentType(Option<String>),
    #[error("{0}")]
    Http(#[from] reqwest::Error),
    #[error("Expected HTTP PNG image, got {0:?}")]
    HttpInvalidContentType(Option<HeaderValue>),
}

pub type FetchResult<T> = Result<T, FetchError>;

pub async fn fetch_raw_image_from_attachment(attachment: &Attachment) -> FetchResult<Bytes> {
    if attachment.content_type.as_deref() != Some(IMAGE_PNG.essence_str()) {
        Err(FetchError::DiscordInvalidContentType(
            attachment.content_type.clone(),
        ))?;
    }
    let data = reqwest::get(&attachment.url).await?;
    let header = data.headers().get(header::CONTENT_TYPE);
    if header != Some(&HeaderValue::from_str(IMAGE_PNG.essence_str()).unwrap()) {
        Err(FetchError::HttpInvalidContentType(header.cloned()))?;
    }
    let bytes = data.bytes().await.unwrap();
    Ok(bytes)
}

pub async fn fetch_image_from_attachment(attachment: &Attachment) -> FetchResult<RgbaImage> {
    let bytes = fetch_raw_image_from_attachment(attachment).await?;
    let image = RgbaImage::from_png(&bytes);
    Ok(image)
}

pub async fn extract_2x2_image(
    ctx: &(impl AsRef<Http> + Send + Sync),
    ir: &ImageRepository,
    attempts: &[Record<Attempt<Approved>>],
    nsfw: bool,
) -> Result<RgbaImage, AppError> {
    let n = 4;
    let mut images = Vec::with_capacity(n);
    complement_submissions(ctx, &mut images, attempts, nsfw).await?;
    complement_draw_this(ctx, &mut images, ir, n).await?;
    complement_in_construction(ctx, &mut images, ir, n).await?;
    let image = concat_2_2(&images);
    Ok(image)
}

pub async fn extract_nx1_image(
    ctx: &(impl AsRef<Http> + Send + Sync),
    ir: &ImageRepository,
    attempts: &[Record<Attempt<Approved>>],
    nsfw: bool,
    n: usize,
) -> Result<RgbaImage, AppError> {
    let mut images = Vec::with_capacity(n);
    complement_submissions(ctx, &mut images, attempts, nsfw).await?;
    complement_draw_this(ctx, &mut images, ir, n).await?;
    complement_in_construction(ctx, &mut images, ir, n).await?;
    let image = concat_vertical(&images);
    Ok(image)
}

async fn complement_submissions(
    ctx: &(impl AsRef<Http> + Send + Sync),
    images: &mut Vec<RgbaImage>,
    attempts: &[Record<Attempt<Approved>>],
    nsfw: bool,
) -> Result<(), AppError> {
    let channel = match nsfw {
        true => CONFIG.channels.partial_nsfw,
        false => CONFIG.channels.partial,
    };
    for what in attempts.iter().map(|a| a.state.what) {
        let image = fetch_image_from_channel(ctx, channel, what).await?;
        images.push(image);
    }
    Ok(())
}

async fn complement_draw_this(
    ctx: &(impl AsRef<Http> + Send + Sync),
    images: &mut Vec<RgbaImage>,
    ir: &ImageRepository,
    n: usize,
) -> Result<(), AppError> {
    if images.len() < n {
        let required = 1;
        let assets = ir
            .random(AssetKind::DrawThis, required)
            .await
            .map_internal("Missing DrawThis assets")?;
        let placeholders = required - assets.len() as u32;
        for image in assets.into_iter().map(|a| a.id()) {
            let image = fetch_image_from_channel(ctx, CONFIG.channels.draw_this, image).await?;
            images.push(image);
        }
        for _ in 0..placeholders {
            images.push(RgbaImage::load("./assets/placeholders/draw-this.png").await);
        }
    };
    Ok(())
}

async fn complement_in_construction(
    ctx: &(impl AsRef<Http> + Send + Sync),
    images: &mut Vec<RgbaImage>,
    ir: &ImageRepository,
    n: usize,
) -> Result<(), AppError> {
    if images.len() < n {
        let required = n - images.len();
        let assets = ir
            .random(AssetKind::InConstruction, required as u32)
            .await
            .map_internal("Missing InConstruction assets")?;
        let placeholders = required - assets.len();
        for image in assets.into_iter().map(|a| a.id()) {
            let image =
                fetch_image_from_channel(ctx, CONFIG.channels.in_contruction, image).await?;
            images.push(image);
        }
        for _ in 0..placeholders {
            images.push(RgbaImage::load("./assets/placeholders/in-construction.png").await);
        }
    };
    Ok(())
}

pub fn raw_image_to_attachment<'a>(bytes: Vec<u8>) -> AttachmentType<'a> {
    AttachmentType::Bytes {
        data: std::borrow::Cow::Owned(bytes),
        filename: "image.png".to_owned(),
    }
}

pub fn image_to_attachment<'a>(image: RgbaImage) -> AttachmentType<'a> {
    raw_image_to_attachment(image.to_png())
}

pub async fn fetch_image_from_channel(
    ctx: &impl AsRef<Http>,
    channel: ChannelId,
    image_id: u64,
) -> Result<RgbaImage, AppError> {
    let msg = channel.message(ctx, MessageId(image_id)).await?;
    let image = fetch_image_from_attachment(&msg.attachments[0])
        .await
        .map_internal("Failed to fetch image")?;
    Ok(image)
}

pub fn prompt(round: &RoundWithAttempts<Active>, in_progress: bool) -> String {
    let mode = round.round.mode;
    let round_no = round.round.round_no;
    let in_progress = if in_progress {
        "Already in progress.\n"
    } else {
        ""
    };
    let sfw = if round.round.nsfw { "NSFW " } else { "" };
    format!(
        "{}{}{:?} mode round {}.\n{}\nExpiring <t:{}:R>.\nUse `/submit` or `/cancel` to continue.",
        in_progress,
        sfw,
        mode,
        round_no + 1,
        mode.prompt(round_no),
        round.attempt.state.until.timestamp()
    )
}

pub async fn respond_with_prompt(
    rsx: &mut ResponseContext<'_>,
    ctx: &AppContext<'_>,
    round: &RoundWithAttempts<Active>,
    in_progress: bool,
) -> Result<(), AppError> {
    let attachment = round
        .round
        .mode
        .render_prompt_image(ctx, round, &ctx.data().get())
        .await?;
    rsx.purge().await?;
    rsx.respond(|f| f.attachment(attachment).content(prompt(round, in_progress)))
        .await?;
    Ok(())
}

pub fn session_destination<S>(round: &RoundWithAttempts<S>) -> ChannelId {
    match (
        round.round.round_no == round.round.mode.last_round(),
        round.round.nsfw,
    ) {
        (false, false) => CONFIG.channels.partial,
        (true, false) => CONFIG.channels.complete,
        (false, true) => CONFIG.channels.partial_nsfw,
        (true, true) => CONFIG.channels.complete_nsfw,
    }
}
