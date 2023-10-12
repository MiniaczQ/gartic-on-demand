use super::{AppData, AppError, AssetHandler};
use poise::{Event, FrameworkContext};
use rossbot::services::{database::images::ImageRepository, provider::Provider};
use serenity::prelude::Context;
use std::{future::Future, pin::Pin};
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
        match event {
            Event::ReactionAdd { add_reaction } => Some(Box::pin(async move {
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
