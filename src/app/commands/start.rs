use std::ops::{Add, Sub};

use crate::{
    app::{config::CONFIG, AppError, InternalError},
    AppData,
};
use chrono::Utc;
use poise::{serenity_prelude::MessageId, Context};
use reqwest::Url;
use rossbot::services::{
    database::session::{Session, SessionRepository, UserSession},
    gamemodes::{GameSession, Gamemode},
    provider::Provider,
    util::get_image_attachment_link,
};
use tracing::{error, info};

#[derive(Debug, poise::ChoiceParameter)]
pub enum GamemodeArg {
    Ross,
}

#[poise::command(slash_command, dm_only)]
pub async fn start(
    ctx: Context<'_, AppData, AppError>,
    mode: GamemodeArg,
    #[min = 1] round: Option<u64>,
) -> Result<(), AppError> {
    let sr: SessionRepository = ctx.data().get();
    let id = ctx.author().id;
    let round = round.unwrap_or(1).sub(1);

    if let Ok(session) = sr.get_current_user_game(id).await {
        ctx.send(|f| {
            f.content(format!(
                "{:?} mode already running.\nExpiring <t:{}:R>.",
                session.game.mode,
                session.user.unwrap().expires_at.timestamp()
            ))
        })
        .await?;

        return Ok(());
    }

    sr.remove_expired_sessions().await.map_err(|e| {
        error!(error = ?e);
        InternalError("Failed to unlock expired sessions")
    })?;

    let mode = match mode {
        GamemodeArg::Ross => Gamemode::Ross,
    };
    let created_at = Utc::now();
    let expires_at = created_at.add(mode.round_time(round));

    let reply = ctx.send(|f| f.content("Starting session...")).await?;
    let user = UserSession {
        user: ctx.author().id,
        expires_at,
    };

    let result = sr.find_game_for_user(mode, round, user).await;
    let session = match (result, round) {
        (Ok(session), _) => session,
        (Err(e), 0) => {
            error!(error = ?e);
            let session = Session::new(user, GameSession::new(mode));
            sr.create_game_for_user(session).await.map_err(|e| {
                error!(error = ?e);
                InternalError("Failed to create game session")
            })?
        }
        (Err(e), _) => {
            error!(error = ?e);
            Err(InternalError("Failed to find existing game session"))?
        }
    };

    info!("Session {:?}", session);

    let mut attachments = Vec::with_capacity(session.game.images.len());
    for image in session.game.images.iter() {
        let msg = CONFIG
            .channels
            .attributes
            .message(ctx, MessageId(*image))
            .await?;
        let url = get_image_attachment_link(&msg).unwrap();
        let attachment: Url = url.try_into().unwrap();
        attachments.push(attachment);
    }

    info!("Attachments {:?}", attachments);

    reply
        .edit(ctx, |f| {
            for attachment in attachments {
                f.embed(|e| e.image(attachment));
            }
            f.content(format!(
                "Started {:?} mode round {}.\nExpiring <t:{}:R>.",
                mode,
                session.game.images.len() + 1,
                expires_at.timestamp()
            ))
        })
        .await?;

    Ok(())
}
