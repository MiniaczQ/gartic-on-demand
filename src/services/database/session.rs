use super::{Database, DbError, DbResult, RawRecord};
use crate::services::{
    gamemodes::{GameLogic, GameSession, Mode},
    provider::Provider,
};
use chrono::{DateTime, Utc};
use poise::serenity_prelude::UserId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct UserSession {
    pub user_id: UserId,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Session {
    pub user: Option<UserSession>,
    pub game: GameSession,
}

impl Session {
    pub fn new(user: UserSession, game: GameSession) -> Self {
        Self {
            user: Some(user),
            game,
        }
    }

    pub fn prompt_started(&self) -> String {
        let mode = self.game.mode;
        let round = self.game.round();
        format!(
            "Started {:?} mode round {}.\n{}\nExpiring <t:{}:R>.\nUse `/submit` or `/cancel` to continue.",
            mode,
            round + 1,
            mode.prompt(round),
            self.user.unwrap().expires_at.timestamp()
        )
    }

    pub fn prompt_already_running(&self) -> String {
        let mode = self.game.mode;
        let round = self.game.round();
        format!(
            "{:?} mode round {} already running.\n{}\nExpiring <t:{}:R>.\nnUse `/submit` or `/cancel` to continue.",
            mode,
            round + 1,
            mode.prompt(round),
            self.user.unwrap().expires_at.timestamp()
        )
    }
}

pub struct SessionRepository {
    db: Database,
}

impl SessionRepository {
    const TABLE: &str = "sessions";

    pub async fn get_current_user_game(&self, user_id: UserId) -> DbResult<Session> {
        let query = r#"
        SELECT * FROM type::table($table) WHERE user.user_id = $user_id
        "#;
        let mut result = self
            .db
            .query(query)
            .bind(("table", Self::TABLE))
            .bind(("user_id", user_id))
            .await?;
        let session: Option<RawRecord<Session>> = result.take(0)?;
        let session = session.ok_or(DbError::NotFound)?;
        Ok(session.entry)
    }

    pub async fn find_game_for_user(
        &self,
        mode: Mode,
        round: u64,
        user: UserSession,
    ) -> DbResult<Session> {
        let query = r#"
        UPDATE (
            SELECT * FROM type::table($table)
                WHERE user = NONE
                AND array::len(game.images) = $round
                AND game.mode = $mode
            ORDER BY rand() LIMIT 1
        )
        SET user = $user
        RETURN AFTER
        "#;
        let mut result = self
            .db
            .query(query)
            .bind(("table", Self::TABLE))
            .bind(("round", round))
            .bind(("mode", mode))
            .bind(("user", user))
            .await?;
        let session: Option<RawRecord<Session>> = result.take(0)?;
        let session = session.ok_or(DbError::NotFound)?;
        Ok(session.entry)
    }

    pub async fn create_game_for_user(&self, session: Session) -> DbResult<Session> {
        let session: Vec<RawRecord<Session>> = self.db.create(Self::TABLE).content(session).await?;
        let session = session.into_iter().next().ok_or(DbError::NotFound)?;
        Ok(session.entry)
    }

    pub async fn remove_expired_sessions(&self) -> DbResult<()> {
        let query = r#"
        UPDATE type::table($table) SET user = NONE WHERE user.expires_at < $now
        "#;
        self.db
            .query(query)
            .bind(("table", Self::TABLE))
            .bind(("round", Utc::now()))
            .await?;
        Ok(())
    }

    pub async fn remove_expiry(&self, user_id: UserId) -> DbResult<Session> {
        let query = r#"
        UPDATE type::table($table) SET
            user.expires_at = NONE
        WHERE user.user_id = $user_id
        RETURN BEFORE
        "#;
        let mut result = self
            .db
            .query(query)
            .bind(("table", Self::TABLE))
            .bind(("user_id", user_id))
            .await?;
        let session: Option<RawRecord<Session>> = result.take(0)?;
        let session = session.ok_or(DbError::NotFound)?;
        Ok(session.entry)
    }

    pub async fn attach_image(&self, user_id: UserId, image: u64) -> DbResult<()> {
        let query =
            r#"UPDATE type::table($table) SET game.images += $image WHERE user.user_id = $user_id"#;
        self.db
            .query(query)
            .bind(("table", Self::TABLE))
            .bind(("user_id", user_id))
            .bind(("image", image))
            .await?;
        Ok(())
    }

    pub async fn detach_user(&self, user_id: UserId) -> DbResult<GameSession> {
        let query = r#"UPDATE type::table($table) SET user = NONE WHERE user.user_id = $user_id"#;
        let mut result = self
            .db
            .query(query)
            .bind(("table", Self::TABLE))
            .bind(("user_id", user_id))
            .await?;
        let session: Option<RawRecord<Session>> = result.take(0)?;
        let session = session.ok_or(DbError::NotFound)?;
        Ok(session.entry.game)
    }
}

impl<T> Provider<SessionRepository> for T
where
    T: Provider<Database>,
{
    fn get(&self) -> SessionRepository {
        SessionRepository { db: self.get() }
    }
}
