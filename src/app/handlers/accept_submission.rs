use super::{AppData, AppError, AssetHandler};
use crate::app::{
    config::CONFIG,
    error::{ConvertError, OptionEmptyError},
    permission::has_mod,
    util::{fetch_raw_image_from_attachment, raw_image_to_attachment},
};
use poise::{Event, FrameworkContext};
use rossbot::services::{
    database::session::{SessionRepository, SubmissionKind},
    provider::Provider,
};
use serenity::prelude::Context;
use std::{cmp::Ordering, future::Future, pin::Pin};

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
        let sr: SessionRepository = data.get();
        let event = event.clone();
        match event {
            Event::ReactionAdd { add_reaction } => Some(Box::pin(async move {
                let user = add_reaction.user(&ctx).await?;
                if add_reaction.channel_id != CONFIG.channels.moderation {
                    return Ok(());
                }
                if user.bot {
                    return Ok(());
                }
                has_mod(&ctx, &user).await?;

                let accept = add_reaction
                    .emoji
                    .unicode_partial_cmp(&CONFIG.reactions.accept)
                    == Some(Ordering::Equal);
                let reject = add_reaction
                    .emoji
                    .unicode_partial_cmp(&CONFIG.reactions.reject)
                    == Some(Ordering::Equal);

                let accepted = match (accept, reject) {
                    (true, false) => true,
                    (false, true) => false,
                    (false, false) => return Ok(()),
                    (true, true) => unreachable!(),
                };

                let old_aid = add_reaction.message_id.0;
                let session = sr
                    .get_pending(old_aid)
                    .await
                    .map_internal("Failed to get pending session")?;

                let channel = match (accepted, session.kind) {
                    (true, SubmissionKind::RossAttribute) => CONFIG.channels.partial,
                    (true, SubmissionKind::RossComplete) => CONFIG.channels.complete,
                    (false, _) => CONFIG.channels.rejects,
                };

                let message = add_reaction.message(&ctx).await?;
                let raw_image = fetch_raw_image_from_attachment(&message.attachments[0])
                    .await
                    .ok_or(AppError::internal(
                        OptionEmptyError,
                        "Failed to fetch image",
                    ))?;

                let attachment = raw_image_to_attachment(raw_image.into());
                let message = channel
                    .send_message(ctx, |m| m.add_file(attachment).content(message.content))
                    .await?;

                let uid = user.id.0;
                let new_aid = message.id.0;
                match accepted {
                    true => sr.accept_pending(uid, old_aid, new_aid).await,
                    false => sr.reject_pending(uid, old_aid, new_aid).await,
                }
                .map_internal("Failed to accept/reject session")?;

                Ok(())
            })),
            _ => None,
        }
    }
}
