use crate::config::CONFIG;
use rossbot::{
    database::assets::{AssetEntry, AssetKind},
    image_processing::{normalize_image, RgbaConvert},
    util::{fetch_image, get_image_attachment_link},
};
use serenity::{
    model::prelude::{AttachmentType, ChannelId, Message, Reaction},
    prelude::Context,
};
use tracing::{error, warn};

use super::AppContext;

pub struct AssetsHandler;

fn channel_to_asset_kind(channel_id: &ChannelId) -> Option<AssetKind> {
    if channel_id == CONFIG.channels.in_contruction.as_ref() {
        Some(AssetKind::InConstruction)
    } else if channel_id == CONFIG.channels.draw_this.as_ref() {
        Some(AssetKind::DrawThis)
    } else {
        None
    }
}

impl AssetsHandler {
    async fn create_asset(acx: &AppContext, scx: &Context, msg: &Message, kind: AssetKind) {
        // Filter author
        if msg.author.bot {
            return;
        }
        let Ok(true) = msg
            .author
            .has_role(scx, CONFIG.guild, CONFIG.roles.admin)
            .await
            .map_err(|e| error!(error = ?e))
        else {
            return;
        };
        // Extract image
        let Some(url) = get_image_attachment_link(msg) else {
            warn!("No attachment");
            return;
        };
        let Some(image) = fetch_image(url).await else {
            return;
        };
        let image = normalize_image(&image, CONFIG.image.width, CONFIG.image.height);
        let bytes = image.to_png();
        // Save normalized image
        acx.sg.upload(format!("{}.png", msg.id), &bytes).await;
        let file = AttachmentType::Bytes {
            data: std::borrow::Cow::Owned(bytes.to_vec()),
            filename: "asset.png".to_owned(),
        };
        let reactions = [CONFIG.reactions.delete];
        let result = msg
            .channel_id
            .send_message(scx, |m| m.add_file(file).reactions(reactions))
            .await
            .map_err(|e| error!(error = ?e))
            .ok();
        let Some(new_msg) = result else {
            return;
        };
        let new_id = new_msg.id;
        let Some(new_url) = get_image_attachment_link(&new_msg) else {
            warn!("No attachment");
            return;
        };
        // Save reference in db
        if !acx
            .db
            .create_asset(new_id.0, AssetEntry::new(kind, new_url))
            .await
        {
            warn!("Failed to add to db");
            return;
        }
        // Remove old image
        msg.delete(scx).await.map_err(|e| error!(error = ?e)).ok();
    }

    pub async fn message(acx: &AppContext, scx: &Context, msg: &Message) {
        let Some(kind) = channel_to_asset_kind(&msg.channel_id) else {
            return;
        };
        Self::create_asset(acx, scx, msg, kind).await;
    }

    pub async fn reaction_add(acx: &AppContext, scx: &Context, add_reaction: &Reaction) {
        // Filter
        let Some(author_id) = add_reaction.user_id else {
            return;
        };
        let Ok(author) = author_id.to_user(scx).await.map_err(|e| error!(error = ?e)) else {
            return;
        };
        if author.bot {
            return;
        }
        let Ok(true) = author
            .has_role(scx, CONFIG.guild, CONFIG.roles.admin)
            .await
            .map_err(|e| error!(error = ?e))
        else {
            return;
        };
        // Delete
        let Some(_) = channel_to_asset_kind(&add_reaction.channel_id) else {
            return;
        };
        if add_reaction.emoji == CONFIG.reactions.delete.into() {
            acx.db.delete_asset(add_reaction.message_id.0).await;
            add_reaction
                .channel_id
                .delete_message(scx, add_reaction.message_id)
                .await
                .map_err(|e| error!(error = ?e))
                .ok();
        }
    }
}
