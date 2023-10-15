use crate::app::{config::CONFIG, permission::has_admin};

use super::{AppData, AppError, AssetHandler};
use poise::{Event, FrameworkContext};
use rossbot::services::{database::assets::ImageRepository, provider::Provider};
use serenity::prelude::Context;
use std::{cmp::Ordering, future::Future, pin::Pin};
use tracing::error;

pub struct RemoveAsset;

impl AssetHandler for RemoveAsset {
    fn handle(
        &self,
        ctx: &Context,
        event: &Event<'_>,
        _fcx: FrameworkContext<'_, AppData, AppError>,
        data: &AppData,
    ) -> Option<Pin<Box<dyn Future<Output = Result<(), AppError>> + Send>>> {
        let ctx = ctx.clone();
        let ir: ImageRepository = data.get();
        let event = event.clone();
        let channels = [CONFIG.channels.draw_this, CONFIG.channels.in_contruction];
        match event {
            Event::ReactionAdd { add_reaction } => Some(Box::pin(async move {
                let user = add_reaction.user(&ctx).await?;
                if !channels.contains(&add_reaction.channel_id) {
                    return Ok(());
                }
                if user.bot {
                    return Ok(());
                }
                if add_reaction
                    .emoji
                    .unicode_partial_cmp(&CONFIG.reactions.delete)
                    == Some(Ordering::Equal)
                {
                    return Ok(());
                }
                has_admin(&ctx, &user).await?;
                let result = ir.delete(add_reaction.message_id.0).await;
                match result {
                    Ok(_) => {
                        add_reaction
                            .channel_id
                            .edit_message(ctx, add_reaction.message_id, |f| f.content("Deleted"))
                            .await
                            .ok();
                    }
                    Err(e) => error!(error = ?e),
                }

                Ok(())
            })),
            _ => None,
        }
    }
}
