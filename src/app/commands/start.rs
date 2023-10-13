use crate::app::{
    error::{AppError, ConvertError},
    response::ResponseContext,
    util::display_started_round,
    AppContext,
};
use chrono::Utc;
use poise::serenity_prelude::UserId;
use rossbot::services::{
    database::session::{Session, SessionRepository, UserSession},
    gamemodes::{GameLogic, GameSession, Mode},
    provider::Provider,
};
use std::ops::{Add, Sub};
use tracing::error;

#[derive(Debug, poise::ChoiceParameter)]
pub enum GameArg {
    Ross,
}

#[poise::command(slash_command, guild_only)]
pub async fn start(
    ctx: AppContext<'_>,
    mode: GameArg,
    #[min = 1] round: Option<u64>,
) -> Result<(), AppError> {
    let mut rsx = ResponseContext::new(ctx);
    rsx.init().await?;
    if let Err(e) = process(&mut rsx, ctx, mode, round).await {
        error!(error = ?e);
        rsx.respond(|b| b.content(e.for_user())).await?
    }
    rsx.finalize().await?;
    Ok(())
}

async fn process(
    rsx: &mut ResponseContext<'_>,
    ctx: AppContext<'_>,
    mode: GameArg,
    round: Option<u64>,
) -> Result<(), AppError> {
    let sr: SessionRepository = ctx.data().get();
    let user_id = ctx.author().id;
    let round = round.unwrap_or(1).sub(1);

    let mby_session = sr.get_current_user_game(user_id).await;
    if let Ok(session) = mby_session {
        rsx.respond(|f| f.content(session.prompt_already_running()))
            .await?;
        return Ok(());
    }

    sr.remove_expired_sessions()
        .await
        .map_internal("Failed to unlock expired sessions")?;

    let session = find_or_create_session(sr, user_id, mode, round).await?;
    display_started_round(rsx, ctx, session).await?;
    Ok(())
}

async fn find_or_create_session(
    sr: SessionRepository,
    user_id: UserId,
    mode: GameArg,
    round: u64,
) -> Result<Session, AppError> {
    let mode = map_game(mode);

    if round > mode.last_round() {
        None.map_user("Gamemode does not support this many rounds")?;
    }

    let user = UserSession {
        user_id,
        expires_at: Utc::now().add(mode.time_limit(round)),
    };

    let mby_session = sr.find_game_for_user(mode, round, user).await;
    let session = match (mby_session, round) {
        (Ok(session), _) => session,
        (Err(_), 0) => {
            let session = Session::new(user, GameSession::new(mode));
            sr.create_game_for_user(session)
                .await
                .map_internal("Failed to create game session")?
        }
        (e, _) => e.map_user("Did not find pending sessions")?,
    };

    Ok(session)
}

fn map_game(mode: GameArg) -> Mode {
    match mode {
        GameArg::Ross => Mode::Ross,
    }
}
