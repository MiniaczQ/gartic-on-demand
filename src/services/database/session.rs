use super::{Database, DbResult, MapToNotFound};
use crate::services::{
    gamemodes::{GameLogic, Mode},
    provider::Provider,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::ops::Add;
use tracing::info;

#[derive(Debug, Serialize, Deserialize)]
pub struct User;

#[derive(Debug, Serialize, Deserialize)]
pub struct Active {
    pub until: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Accepted {
    pub when: DateTime<Utc>,
    pub who: u64,
    pub what: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Uploading {
    since: DateTime<Utc>,
}

/// Active -> Cancelled
/// Active -> Expired
/// Active -> Uploading
/// TEMP: Uploading -> Accepted
/// SOON: Uploading -> Pending
/// SOON: Pending -> Accepted
/// SOON: Pending -> Rejected
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SessionState {
    Active {
        until: DateTime<Utc>,
    },
    Cancelled {
        when: DateTime<Utc>,
    },
    Expired {
        when: DateTime<Utc>,
    },
    Uploading {
        since: DateTime<Utc>,
    },
    Pending {
        since: DateTime<Utc>,
        what: u64,
    },
    Accepted {
        when: DateTime<Utc>,
        who: u64,
        what: u64,
    },
    Rejected {
        when: DateTime<Utc>,
        who: u64,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Session {
    pub started_at: DateTime<Utc>,
    pub state: SessionState,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TypedSession<T> {
    pub started_at: DateTime<Utc>,
    pub state: T,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Lobby {
    pub mode: Mode,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LobbyWithSessions<S> {
    pub lobby: Lobby,
    pub accepted: Vec<TypedSession<Accepted>>,
    pub active: TypedSession<S>,
}

impl<S> LobbyWithSessions<S> {
    pub fn round(&self) -> u64 {
        self.accepted.len() as u64
    }
}

impl LobbyWithSessions<Active> {
    pub fn prompt_started(&self) -> String {
        let mode = self.lobby.mode;
        let round = self.round();
        format!(
            "Started {:?} mode round {}.\n{}\nExpiring <t:{}:R>.\nUse `/submit` or `/cancel` to continue.",
            mode,
            round + 1,
            mode.prompt(round),
            self.active.state.until.timestamp()
        )
    }

    pub fn prompt_already_running(&self) -> String {
        let mode = self.lobby.mode;
        let round = self.round();
        format!(
            "{:?} mode round {} already running.\n{}\nExpiring <t:{}:R>.\nUse `/submit` or `/cancel` to continue.",
            mode,
            round + 1,
            mode.prompt(round),
            self.active.state.until.timestamp()
        )
    }
}

pub struct SessionRepository {
    db: Database,
}

impl SessionRepository {
    pub async fn ensure_user(&self, uid: u64) -> DbResult<()> {
        let query = r#"INSERT INTO users {id: $uid}"#;
        self.db.query(query).bind(("uid", uid)).await?;
        Ok(())
    }

    pub async fn get(&self, uid: u64) -> DbResult<LobbyWithSessions<Active>> {
        info!(uid = uid, "Get");
        let query = r#"
        LET $user_session = SELECT * FROM ONLY sessions
            WHERE meta::id(in) Is $uid
            AND state.type IS "Active";
        RETURN IF $user_session IS NONE {
            RETURN []
        } ELSE {
            RETURN (
                SELECT
                    out AS lobby,
                    $user_session AS active,
                    out<-(sessions WHERE state.type IS "Accepted") AS accepted
                FROM $user_session
                FETCH lobby, accepted
            )
        }
        "#;
        let mut result = self.db.query(query).bind(("uid", uid)).await?;
        result.take::<Option<_>>(1)?.found()
    }

    /// Active -> Expired
    pub async fn stop_expired(&self) -> DbResult<()> {
        info!("Stop expired");
        let now = Utc::now();
        let expired = SessionState::Expired { when: now };
        let query = r#"
        UPDATE sessions
            SET state = $expired
            WHERE state.type IS "Active"
            AND state.until < $now
        "#;
        self.db
            .query(query)
            .bind(("now", now))
            .bind(("expired", expired))
            .await?;
        Ok(())
    }

    /// Active -> Uploading
    pub async fn start_submitting(&self, uid: u64) -> DbResult<LobbyWithSessions<Uploading>> {
        info!(uid = uid, "Start submitting");
        let now: DateTime<Utc> = Utc::now();
        let uploading = SessionState::Uploading { since: now };
        let query = r#"
        LET $user_session = UPDATE ONLY sessions
            SET state = $uploading
            WHERE meta::id(in) IS $uid
            AND state.type IS "Active";
        RETURN IF $user_session IS NONE {
            RETURN []
        } ELSE {
            RETURN (
                SELECT
                    out AS lobby,
                    $user_session AS active,
                    out<-(sessions WHERE state.type IS "Accepted") AS accepted
                FROM $user_session
                FETCH lobby, accepted
            )
        }
        "#;
        let mut result = self
            .db
            .query(query)
            .bind(("uid", uid))
            .bind(("uploading", uploading))
            .await?;
        result.take::<Option<_>>(1)?.found()
    }

    /// TEMP: Uploading -> Accepted
    pub async fn finish_submitting(&self, uid: u64, aid: u64) -> DbResult<()> {
        info!(uid = uid, aid = aid, "Finish submitting");
        let now: DateTime<Utc> = Utc::now();
        let accepted = SessionState::Accepted {
            when: now,
            who: uid,
            what: aid,
        };
        let query = r#"
        UPDATE sessions
            SET state = $accepted
            WHERE meta::id(in) = $uid
            AND state.type IS "Uploading"
        "#;
        self.db
            .query(query)
            .bind(("uid", uid))
            .bind(("accepted", accepted))
            .await?;
        Ok(())
    }

    /// Active -> Cancelled
    pub async fn cancel(&self, uid: u64) -> DbResult<()> {
        info!(uid = uid, "Cancel");
        let now: DateTime<Utc> = Utc::now();
        let cancelled = SessionState::Cancelled { when: now };
        let query = r#"
        UPDATE sessions
        SET state = $cancelled
        WHERE meta::id(in) IS $uid
        AND state.type IS "Active"
        "#;
        self.db
            .query(query)
            .bind(("uid", uid))
            .bind(("cancelled", cancelled))
            .await?;
        Ok(())
    }

    /// -> Active
    pub async fn find_attach(
        &self,
        uid: u64,
        mode: Mode,
        round: u64,
    ) -> DbResult<LobbyWithSessions<Active>> {
        info!(uid = uid, mode = ?mode, round = round, "Find attach");
        let now = Utc::now();
        let until = now.add(mode.time_limit(round));
        let active = SessionState::Active { until };
        let session = Session {
            started_at: now,
            state: active,
        };
        let query = r#"
        LET $user = type::thing("users", $uid);
        LET $lobby = SELECT * FROM ONLY lobbies
            WHERE mode = $mode
            AND array::any(id<-(sessions WHERE in IS $user)) IS false
            AND array::any(id<-(sessions WHERE state.type IN ["Uploading", "Pending"])) IS false
            AND array::len(id<-(sessions WHERE state.type IS "Accepted")) = $round
            ORDER BY rand() LIMIT 1;
        LET $user_session = RELATE ONLY $user->sessions->($lobby.id) CONTENT $session_content;
        RETURN IF $user_session IS NONE {
            RETURN []
        } ELSE {
            RETURN (
                SELECT
                    out AS lobby,
                    $user_session AS active,
                    out<-(sessions WHERE state.type IS "Accepted") AS accepted
                FROM ONLY $user_session
                FETCH lobby, accepted
            )
        }
        "#;
        let mut result = self
            .db
            .query(query)
            .bind(("uid", uid))
            .bind(("mode", mode))
            .bind(("round", round))
            .bind(("session_content", session))
            .await?;
        result.take::<Option<_>>(3)?.found()
    }

    /// -> Active
    pub async fn create_attach(&self, uid: u64, mode: Mode) -> DbResult<LobbyWithSessions<Active>> {
        info!(uid = uid, mode = ?mode, "Create attach");
        let now = Utc::now();
        let until = now.add(mode.time_limit(0));
        let active = SessionState::Active { until };
        let session = Session {
            started_at: now,
            state: active,
        };
        let lobby = Lobby { mode };
        let query = r#"
        LET $user = type::thing("users", $uid);
        LET $lobby = CREATE ONLY lobbies CONTENT $lobby_content;
        LET $user_session = RELATE ONLY $user->sessions->($lobby.id) CONTENT $session_content;
        RETURN IF $user_session IS NONE {
            RETURN []
        } ELSE {
            RETURN (
                SELECT
                    out AS lobby,
                    $user_session AS active,
                    out<-(sessions WHERE state.type IS "Accepted") AS accepted
                FROM $user_session
                FETCH lobby, accepted
            )
        }
        "#;
        let mut result = self
            .db
            .query(query)
            .bind(("uid", uid))
            .bind(("lobby_content", lobby))
            .bind(("session_content", session))
            .await?;
        result.take::<Option<_>>(3)?.found()
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
