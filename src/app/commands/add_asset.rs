use std::borrow::Cow;

use crate::{
    app::{config::CONFIG, error::ConvertError, util::fetch_image_from_attachment, AppError},
    AppData,
};
use poise::{
    serenity_prelude::{Attachment, AttachmentType, ChannelId},
    Context,
};
use rossbot::services::{
    database::images::{AssetKind, Image, ImageRepository},
    image_processing::{normalize_image, RgbaConvert},
    provider::Provider,
};

#[derive(Debug, poise::ChoiceParameter)]
pub enum AssetKindArg {
    InConstruction,
    DrawThis,
}

impl From<AssetKindArg> for AssetKind {
    fn from(value: AssetKindArg) -> Self {
        match value {
            AssetKindArg::InConstruction => AssetKind::InConstruction,
            AssetKindArg::DrawThis => AssetKind::DrawThis,
        }
    }
}

fn kind_to_channel(kind: AssetKind) -> ChannelId {
    match kind {
        AssetKind::InConstruction => CONFIG.channels.in_contruction,
        AssetKind::DrawThis => CONFIG.channels.draw_this,
    }
}

#[poise::command(slash_command, guild_only)]
pub async fn add_asset(
    ctx: Context<'_, AppData, AppError>,
    attachment: Attachment,
    kind: AssetKindArg,
) -> Result<(), AppError> {
    let kind: AssetKind = kind.into();

    let image = fetch_image_from_attachment(attachment)
        .await
        .map_user("Attachment is not an image")?;
    let image = normalize_image(&image, CONFIG.image.width, CONFIG.image.height);
    let image = AttachmentType::Bytes {
        data: Cow::Owned(image.to_png().to_vec()),
        filename: ctx.id().to_string() + ".png",
    };

    let author = ctx.author();

    let message = kind_to_channel(kind)
        .send_message(ctx, |m| {
            m.add_file(image)
                .content(format!("<@{}>", author.id))
                .reactions([CONFIG.reactions.delete.clone()])
        })
        .await?;

    let ir: ImageRepository = ctx.data().get();
    ir.create(message.id.0, Image::new(kind.into(), author.id))
        .await
        .map_internal("Failed to add image to database")?;

    ctx.send(|f| f.content("Added")).await?;

    Ok(())
}
