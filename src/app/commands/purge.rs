use crate::app::{
    config::CONFIG, permission::has_admin, response::ResponseContext, AppContext, AppError,
};
use chrono::Utc;
use tracing::error;

/// Prints the purge message
#[poise::command(slash_command, guild_only)]
pub async fn purge(ctx: AppContext<'_>) -> Result<(), AppError> {
    let mut rsx = ResponseContext::new(ctx);
    rsx.init().await?;
    if let Err(e) = process(&mut rsx, ctx).await {
        error!(error = ?e);
        rsx.respond(|b| b.content(e.for_user())).await?
    }
    rsx.finalize().await?;
    Ok(())
}

async fn process(rsx: &mut ResponseContext<'_>, ctx: AppContext<'_>) -> Result<(), AppError> {
    let user = ctx.author();
    has_admin(&ctx, user).await?;
    let channels = &[
        CONFIG.channels.complete,
        CONFIG.channels.complete_nsfw,
        CONFIG.channels.partial,
        CONFIG.channels.partial_nsfw,
        CONFIG.channels.moderation,
        CONFIG.channels.rejects,
    ];
    let now = Utc::now();
    let message = format!(
        "Database was purged <t:{}>.\nAll images above are now unregistered.",
        now.timestamp()
    );
    for channel in channels {
        channel
            .send_message(&ctx, |b| b.embed(|b| b.description(&message)))
            .await?;
    }
    rsx.respond(|b| b.content("Done")).await?;
    Ok(())
}
