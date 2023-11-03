use super::{
    config::CONFIG,
    error::{AppError, ConvertError},
    rendering::ModeRenderer,
    response::ResponseContext,
    AppContext,
};
use bytes::Bytes;
use image::RgbaImage;
use mime::IMAGE_PNG;
use poise::serenity_prelude::{Attachment, AttachmentType, ChannelId, MessageId};
use reqwest::header::{self, HeaderValue};
use rossbot::services::{
    database::{
        assets::{AssetKind, ImageRepository},
        attempt::Active,
        round::RoundWithAttempts,
    },
    gamemodes::GameLogic,
    image_processing::{concat_2_2, RgbaConvert},
    provider::Provider,
};
use serenity::http::Http;

pub async fn fetch_raw_image_from_attachment(attachment: &Attachment) -> Option<Bytes> {
    if attachment.content_type.as_deref() != Some(IMAGE_PNG.essence_str()) {
        return None;
    }
    let response = reqwest::get(&attachment.url).await;
    let Ok(data) = response else {
        return None;
    };
    if data.headers().get(header::CONTENT_TYPE)
        != Some(&HeaderValue::from_str(IMAGE_PNG.essence_str()).unwrap())
    {
        return None;
    }
    let bytes = data.bytes().await.unwrap();
    Some(bytes)
}

pub async fn fetch_image_from_attachment(attachment: &Attachment) -> Option<RgbaImage> {
    let bytes = fetch_raw_image_from_attachment(attachment).await?;
    let image = RgbaImage::from_png(&bytes);
    Some(image)
}

pub async fn extract_2x2_image<T>(
    ctx: &(impl AsRef<Http> + Send + Sync),
    round: &RoundWithAttempts<T>,
    ir: &ImageRepository,
) -> Result<RgbaImage, AppError> {
    let mut images = Vec::with_capacity(4);
    let channel = match round.round.nsfw {
        true => CONFIG.channels.partial_nsfw,
        false => CONFIG.channels.partial,
    };
    for what in round.previous.iter().map(|a| a.state.what) {
        let image = fetch_image_from_channel(ctx, channel, what).await?;
        images.push(image);
    }
    if images.len() < 4 {
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
    }
    if images.len() < 4 {
        let required = 4 - images.len() as u32;
        let assets = ir
            .random(AssetKind::InConstruction, required)
            .await
            .map_internal("Missing InConstruction assets")?;
        let placeholders = required - assets.len() as u32;
        for image in assets.into_iter().map(|a| a.id()) {
            let image =
                fetch_image_from_channel(ctx, CONFIG.channels.in_contruction, image).await?;
            images.push(image);
        }
        for _ in 0..placeholders {
            images.push(RgbaImage::load("./assets/placeholders/in-construction.png").await);
        }
    }
    let image = concat_2_2(&images);
    Ok(image)
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
        .unwrap();
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
