use crate::app::{
    config::CONFIG,
    error::ConvertError,
    permission::is_trusted,
    rendering::{LobbyRenderer, ModeRenderer},
    response::ResponseContext,
    util::respond_with_prompt,
    AppContext, AppError,
};
use poise::serenity_prelude::{Attachment, ReactionType};
use rossbot::services::{
    database::session::SessionRepository, provider::Provider, status_update::StatusUpdateWaker,
};
use tracing::error;

/// Submit an image to the current game session
#[poise::command(slash_command, guild_only)]
pub async fn submit(
    ctx: AppContext<'_>,
    #[description = "Your submission"] attachment: Attachment,
) -> Result<(), AppError> {
    let mut rsx = ResponseContext::new(ctx);
    rsx.init().await?;
    if let Err(e) = process(&mut rsx, ctx, attachment).await {
        error!(error = ?e);
        rsx.respond(|b| b.content(e.for_user())).await?
    }
    rsx.finalize().await?;
    Ok(())
}

async fn process(
    rsx: &mut ResponseContext<'_>,
    ctx: AppContext<'_>,
    attachment: Attachment,
) -> Result<(), AppError> {
    let sr: SessionRepository = ctx.data().get();
    let user = ctx.author();
    let uid = user.id.0;

    let lobby = sr
        .start_submitting(uid)
        .await
        .map_internal("Failed to find existing session")?;

    let trusted = is_trusted(&ctx, user).await?;

    if trusted {
        let (channel, attachment, content) = if lobby.active.last {
            let channel = match lobby.lobby.nsfw {
                true => CONFIG.channels.complete_nsfw,
                false => CONFIG.channels.complete,
            };
            let attachment = lobby
                .active
                .mode
                .render_complete_image(&ctx, &lobby, &ctx.data().get(), &attachment)
                .await?;
            let content = lobby.render_complete_text();
            (channel, attachment, content)
        } else {
            let channel = match lobby.lobby.nsfw {
                true => CONFIG.channels.partial_nsfw,
                false => CONFIG.channels.partial,
            };
            let attachment = lobby.active.mode.render_partial_image(&attachment).await?;
            let content = lobby.render_partial_text();
            (channel, attachment, content)
        };

        let message = channel
            .send_message(ctx, |m| m.add_file(attachment).content(content))
            .await?;
        sr.finish_submitting_trusted(uid, message.id.0)
            .await
            .map_internal("Failed to attach image")?;
    } else {
        let channel = CONFIG.channels.moderation;
        let attachment = lobby.active.mode.render_partial_image(&attachment).await?;
        let content = lobby.render_partial_text();
        let message = channel
            .send_message(ctx, |m| {
                m.add_file(attachment).content(content).reactions([
                    ReactionType::Unicode(CONFIG.reactions.accept.clone()),
                    ReactionType::Unicode(CONFIG.reactions.reject.clone()),
                ])
            })
            .await?;
        sr.finish_submitting_untrusted(uid, message.id.0)
            .await
            .map_internal("Failed to attach image")?;
    }

    rsx.respond(|f| f.content("Submited!")).await?;
    rsx.reset();

    if lobby.active.last {
        rsx.respond(|b| b.content("This was the final round.\nUse `/start` to play again."))
            .await?;
    } else {
        let next_round = lobby.active.round + 1;
        let mode = lobby.lobby.mode;
        let nsfw = lobby.lobby.nsfw;
        if let Ok(lobby) = sr.find_attach(uid, mode, next_round, nsfw, false).await {
            respond_with_prompt(rsx, &ctx, &lobby, false).await?;
        } else if let Ok(lobby) = sr.find_attach(uid, mode, next_round, nsfw, true).await {
            respond_with_prompt(rsx, &ctx, &lobby, false).await?;
        } else {
            rsx.respond(|b| {
                b.content("No further rounds available currently.\nUse `/start` to play again.")
            })
            .await?
        }
    }
    let waker: StatusUpdateWaker = ctx.data().get();
    waker.wake();
    Ok(())
}
