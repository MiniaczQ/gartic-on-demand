use crate::app::{
    error::{AppError, ConvertError},
    response::ResponseContext,
    util::{extract_2x2_image, image_to_attachment},
    AppContext,
};
use rossbot::services::{
    database::sessionv2::{Active, MatchWithSessions, SessionRepository2},
    gamemodes::{GameLogic, Mode},
    provider::Provider,
};
use std::ops::Sub;
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
    let sr: SessionRepository2 = ctx.data().get();
    let uid = ctx.author().id.0;
    let round = round.unwrap_or(1).sub(1);

    let mby_session = sr.get(uid).await;
    if let Ok(session) = mby_session {
        rsx.respond(|f| f.content(session.prompt_already_running()))
            .await?;
        return Ok(());
    }

    sr.stop_expired()
        .await
        .map_internal("Failed to unlock expired sessions")?;

    let match_ = find_or_create_session(sr, uid, mode, round).await?;
    let image = extract_2x2_image(ctx, &match_).await?;
    let attachment = image_to_attachment(image);
    rsx.purge().await?;
    rsx.respond(|f| f.attachment(attachment).content(match_.prompt_started()))
        .await?;
    Ok(())
}

async fn find_or_create_session(
    sr: SessionRepository2,
    uid: u64,
    mode: GameArg,
    round: u64,
) -> Result<MatchWithSessions<Active>, AppError> {
    let mode = map_game(mode);

    if round > mode.last_round() {
        None.map_user("Gamemode does not support this many rounds")?;
    }

    let mby_match = sr.find_attach(uid, mode, round).await;
    let match_ = match (mby_match, round) {
        (Ok(match_), _) => match_,
        (Err(_), 0) => sr
            .create_attach(uid, mode)
            .await
            .map_internal("Failed to create game session")?,
        (e, _) => e.map_user("Did not find pending sessions")?,
    };

    Ok(match_)
}

fn map_game(mode: GameArg) -> Mode {
    match mode {
        GameArg::Ross => Mode::Ross,
    }
}
