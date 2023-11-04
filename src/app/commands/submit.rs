use crate::app::{
    config::CONFIG,
    error::ConvertError,
    permission::is_trusted,
    rendering::{ModeRenderer, RoundRenderer},
    response::ResponseContext,
    util::respond_with_prompt,
    AppContext, AppError,
};
use gartic_bot::services::{
    database::{attempt::AttemptRepository, round::RoundRepository, user::UserRepository},
    gamemodes::GameLogic,
    provider::Provider,
    status_update::StatusUpdateWaker,
};
use poise::serenity_prelude::{Attachment, ReactionType};
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
    let ar: AttemptRepository = ctx.data().get();
    let ur: UserRepository = ctx.data().get();
    let rr: RoundRepository = ctx.data().get();
    let discord_user = ctx.author();
    let user = ur
        .create_or_update_user(discord_user.id.0, &discord_user.name)
        .await
        .map_internal("Failed to update user")?;

    let round = ar
        .upload_active_attempt(&user)
        .await
        .map_internal("Failed to find existing session")?;

    let trusted = is_trusted(&ctx, discord_user).await?;

    if trusted {
        let (channel, attachment, content) =
            if round.round.round_no == round.round.mode.last_round() {
                let channel = match round.round.nsfw {
                    true => CONFIG.channels.complete_nsfw,
                    false => CONFIG.channels.complete,
                };
                let attachment = round
                    .round
                    .mode
                    .render_complete_image(&ctx, &round, &ctx.data().get(), &attachment)
                    .await?;
                let content = round.render_complete_text();
                (channel, attachment, content)
            } else {
                let channel = match round.round.nsfw {
                    true => CONFIG.channels.partial_nsfw,
                    false => CONFIG.channels.partial,
                };
                let attachment = round.round.mode.render_partial_image(&attachment).await?;
                let content = round.render_partial_text();
                (channel, attachment, content)
            };

        let message = channel
            .send_message(ctx, |m| m.add_file(attachment).content(content))
            .await?;
        let round = ar
            .approve_uploaded_attempt(&user, message.id.0)
            .await
            .map_internal("Failed to attach image")?;
        rr.forward_complete_round(&round.round, &round.attempt, round.round.forward())
            .await
            .map_internal("Failed to forward round")?;
    } else {
        let channel = CONFIG.channels.moderation;
        let attachment = round.round.mode.render_partial_image(&attachment).await?;
        let content = round.render_partial_text();
        let message = channel
            .send_message(ctx, |m| {
                m.add_file(attachment).content(content).reactions([
                    ReactionType::Unicode(CONFIG.reactions.accept.clone()),
                    ReactionType::Unicode(CONFIG.reactions.reject.clone()),
                ])
            })
            .await?;
        ar.moderate_uploaded_attempt(&user, message.id.0)
            .await
            .map_internal("Failed to attach image")?;
    }

    rsx.respond(|f| f.content("Submited!")).await?;
    rsx.reset();

    if round.round.round_no == round.round.mode.last_round() {
        rsx.respond(|b| b.content("This was the final round.\nUse `/start` to play again."))
            .await?;
    } else {
        let round_no = round.round.round_no + 1;
        let mode = round.round.mode;
        let nsfw = round.round.nsfw;
        if let Ok(round) = rr
            .attempt_existing_round(&user, mode, nsfw, round_no, mode.time_limit(round_no))
            .await
        {
            respond_with_prompt(rsx, &ctx, &round, false).await?;
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
