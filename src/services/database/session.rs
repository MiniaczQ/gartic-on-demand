use super::{Database, DbResult, IdConversionError, IdConvert, MapToNotFound, RawRecord, Record};
use crate::services::{
    gamemodes::{ross::Ross, GameLogic, Mode},
    provider::Provider,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{fmt::Display, ops::Add};
use surrealdb::sql::{Id, Thing};
use tracing::info;

#[derive(Debug, Serialize, Deserialize)]
pub struct User<'a> {
    pub name: &'a str,
}

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
    pub since: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Pending {
    pub since: DateTime<Utc>,
    pub what: u64,
}

/// Active -> Cancelled
/// Active -> Expired
/// Active -> Uploading
/// Uploading -> Accepted
/// Uploading -> Pending
/// Pending -> Accepted
/// Pending -> Rejected
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
        what: u64,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Session {
    pub started_at: DateTime<Utc>,
    pub round: u64,
    pub last: bool,
    pub mode: Mode,
    pub nsfw: bool,
    pub state: SessionState,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RawTypedSession<T> {
    pub started_at: DateTime<Utc>,
    #[serde(rename = "in")]
    pub who: Thing,
    pub round: u64,
    pub last: bool,
    pub mode: Mode,
    pub nsfw: bool,
    pub state: T,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TypedSession<T> {
    pub started_at: DateTime<Utc>,
    pub who: u64,
    pub round: u64,
    pub last: bool,
    pub mode: Mode,
    pub nsfw: bool,
    pub state: T,
}

impl<T> TryFrom<RawTypedSession<T>> for TypedSession<T> {
    type Error = IdConversionError;

    fn try_from(value: RawTypedSession<T>) -> Result<Self, Self::Error> {
        let Id::Number(id) = value.who.id else {
            Err(IdConversionError)?
        };
        let id: u64 = id.try_into().map_err(|_| IdConversionError)?;
        Ok(TypedSession {
            started_at: value.started_at,
            who: id,
            round: value.round,
            last: value.last,
            mode: value.mode,
            nsfw: value.nsfw,
            state: value.state,
        })
    }
}

impl<T> IdConvert for RawTypedSession<T> {
    type Target = TypedSession<T>;

    fn convert_id(self) -> Result<Self::Target, IdConversionError> {
        self.try_into()
    }
}

impl<T> IdConvert for Vec<RawTypedSession<T>> {
    type Target = Vec<TypedSession<T>>;

    fn convert_id(self) -> Result<Self::Target, IdConversionError> {
        self.into_iter().map(TypedSession::try_from).collect()
    }
}

impl<T> IdConvert for RawLobbyWithSessions<T> {
    type Target = LobbyWithSessions<T>;

    fn convert_id(self) -> Result<Self::Target, IdConversionError> {
        Ok(LobbyWithSessions {
            lobby: self.lobby,
            accepted: self.accepted.convert_id()?,
            active: self.active.convert_id()?,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Lobby {
    pub created_at: DateTime<Utc>,
    pub mode: Mode,
    pub nsfw: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RawLobbyWithSessions<S> {
    pub lobby: Lobby,
    pub accepted: Vec<RawTypedSession<Accepted>>,
    pub active: RawTypedSession<S>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LobbyWithSessions<S> {
    pub lobby: Lobby,
    pub accepted: Vec<TypedSession<Accepted>>,
    pub active: TypedSession<S>,
}

impl LobbyWithSessions<Active> {
    pub fn prompt(&self, in_progress: bool) -> String {
        let mode = self.lobby.mode;
        let round = self.active.round;
        let in_progress = if in_progress {
            "Already in progress.\n"
        } else {
            ""
        };
        let sfw = if self.lobby.nsfw { "NSFW " } else { "" };
        format!(
            "{}{}{:?} mode round {}.\n{}\nExpiring <t:{}:R>.\nUse `/submit` or `/cancel` to continue.",
            in_progress,
            sfw,
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
    pub async fn ensure_user(&self, uid: u64, name: &str) -> DbResult<()> {
        let user = Record {
            id: uid,
            entry: User { name },
        };
        let query = r#"INSERT INTO users $user"#;
        self.db.query(query).bind(("user", user)).await?;
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
        let lobby = result
            .take::<Option<RawLobbyWithSessions<_>>>(1)?
            .found()?
            .convert_id()?;
        Ok(lobby)
    }

    pub async fn extend(&self, uid: u64, until: DateTime<Utc>) -> DbResult<()> {
        info!(uid = uid, "Extend");
        let query = r#"
        UPDATE ONLY sessions
            SET state.until = $until
            WHERE meta::id(in) Is $uid
            AND state.type IS "Active";
        "#;
        self.db
            .query(query)
            .bind(("uid", uid))
            .bind(("until", until))
            .await?;
        Ok(())
    }

    pub async fn get_pending(&self, aid: u64) -> DbResult<LobbyWithSessions<Pending>> {
        info!(aid = aid, "Get submission");
        let query = r#"
        LET $user_session = SELECT * FROM ONLY sessions
            WHERE state.what = $aid
            AND state.type = "Pending";
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
        let mut result = self.db.query(query).bind(("aid", aid)).await?;
        let lobby = result
            .take::<Option<RawLobbyWithSessions<_>>>(1)?
            .found()?
            .convert_id()?;
        Ok(lobby)
    }

    /// Active -> Expired
    pub async fn stop_expired(&self) -> DbResult<()> {
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
        let lobby = result
            .take::<Option<RawLobbyWithSessions<_>>>(1)?
            .found()?
            .convert_id()?;
        Ok(lobby)
    }

    /// Uploading -> Accepted
    pub async fn finish_submitting_trusted(&self, uid: u64, aid: u64) -> DbResult<()> {
        info!(uid = uid, aid = aid, "Finish submitting trusted");
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

    /// Uploading -> Pending
    pub async fn finish_submitting_untrusted(&self, uid: u64, aid: u64) -> DbResult<()> {
        info!(uid = uid, aid = aid, "Finish submitting untrusted");
        let now: DateTime<Utc> = Utc::now();
        let pending = SessionState::Pending {
            since: now,
            what: aid,
        };
        let query = r#"
        UPDATE sessions
            SET state = $pending
            WHERE meta::id(in) = $uid
            AND state.type IS "Uploading"
        "#;
        self.db
            .query(query)
            .bind(("uid", uid))
            .bind(("pending", pending))
            .await?;
        Ok(())
    }

    /// Pending -> Accepted
    pub async fn accept_pending(&self, uid: u64, old_aid: u64, new_aid: u64) -> DbResult<()> {
        info!(
            uid = uid,
            old_aid = old_aid,
            new_aid = new_aid,
            "Accept pending"
        );
        let now: DateTime<Utc> = Utc::now();
        let accepted = SessionState::Accepted {
            when: now,
            who: uid,
            what: new_aid,
        };
        let query = r#"
        UPDATE sessions
            SET state = $accepted
            WHERE state.what = $old_aid
            AND state.type IS "Pending"
        "#;
        self.db
            .query(query)
            .bind(("old_aid", old_aid))
            .bind(("accepted", accepted))
            .await?;
        Ok(())
    }

    /// Pending -> Rejected
    pub async fn reject_pending(&self, uid: u64, old_aid: u64, new_aid: u64) -> DbResult<()> {
        info!(uid = uid, old_aid = old_aid, "Reject pending");
        let now: DateTime<Utc> = Utc::now();
        let accepted = SessionState::Rejected {
            when: now,
            who: uid,
            what: new_aid,
        };
        let query = r#"
        UPDATE sessions
            SET state = $accepted
            WHERE state.what = $old_aid
            AND state.type IS "Pending"
        "#;
        self.db
            .query(query)
            .bind(("old_aid", old_aid))
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
        nsfw: bool,
    ) -> DbResult<LobbyWithSessions<Active>> {
        info!(uid = uid, mode = ?mode, round = round, "Find attach");
        let now = Utc::now();
        let until = now.add(mode.time_limit(round));
        let active = SessionState::Active { until };
        let last = round == mode.last_round();
        let session = Session {
            started_at: now,
            round,
            mode,
            last,
            nsfw,
            state: active,
        };
        // AND array::any(id<-(sessions WHERE in IS $user AND state.type NOT IN ["Cancelled", "Expired"])) IS false
        let query = r#"
        LET $user = type::thing("users", $uid);
        LET $lobby = SELECT * FROM ONLY lobbies
            WHERE mode = $mode
            
            AND array::any(id<-(sessions WHERE state.type IN ["Active", "Uploading", "Pending"])) IS false
            AND array::len(id<-(sessions WHERE state.type IS "Accepted")) = $round
            AND nsfw IS $nsfw
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
            .bind(("nsfw", nsfw))
            .bind(("session_content", session))
            .await?;
        let lobby = result
            .take::<Option<RawLobbyWithSessions<_>>>(3)?
            .found()?
            .convert_id()?;
        Ok(lobby)
    }

    /// -> Active
    pub async fn create_attach(
        &self,
        uid: u64,
        mode: Mode,
        nsfw: bool,
    ) -> DbResult<LobbyWithSessions<Active>> {
        info!(uid = uid, mode = ?mode, "Create attach");
        let now = Utc::now();
        let round = 0;
        let until = now.add(mode.time_limit(round));
        let active = SessionState::Active { until };
        let last = round == mode.last_round();
        let session = Session {
            started_at: now,
            round: 0,
            mode,
            last,
            nsfw,
            state: active,
        };
        let lobby = Lobby {
            mode,
            created_at: now,
            nsfw,
        };
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
        let lobby = result
            .take::<Option<RawLobbyWithSessions<_>>>(3)?
            .found()?
            .convert_id()?;
        Ok(lobby)
    }

    pub async fn active_users(&self) -> DbResult<Vec<u64>> {
        let query = r#"
        SELECT in AS id
        FROM sessions
        WHERE state.type IS "Active"
        "#;
        let mut result = self.db.query(query).await?;
        let users = result
            .take::<Vec<RawRecord<()>>>(0)?
            .convert_id()?
            .into_iter()
            .map(|r| r.id)
            .collect();
        Ok(users)
    }

    pub async fn incomplete_games(&self) -> DbResult<Vec<IncompleteGames>> {
        let query = r#"
        SELECT
            mode,
            count() AS count,
            array::len(<-(sessions WHERE state.type IS "Accepted")) AS round,
            nsfw
            FROM lobbies
            WHERE <-(sessions WHERE state.type IN ["Active", "Uploading", "Pending"]) IS []
            GROUP BY mode, round, nsfw
        "#;
        let mut result = self.db.query(query).await?;
        let stats = result.take::<Vec<_>>(0)?.filter();
        Ok(stats)
    }

    pub async fn incomplete_games_for_user(&self, uid: u64) -> DbResult<Vec<IncompleteGames>> {
        let query = r#"
        SELECT
            mode,
            count() AS count,
            array::len(<-(sessions WHERE state.type IS "Accepted")) AS round,
            nsfw
            FROM lobbies
            WHERE <-(sessions WHERE state.type IN ["Active", "Uploading", "Pending"]) IS []
            AND array::any(id<-(sessions WHERE meta::id(in) IS $uid AND state.type NOT IN ["Cancelled", "Expired"])) IS false
            GROUP BY mode, round, nsfw
        "#;
        let mut result = self.db.query(query).bind(("uid", uid)).await?;
        let stats = result.take::<Vec<_>>(0)?.filter();
        Ok(stats)
    }

    pub async fn random_attributes(&self, n: u64) -> DbResult<Vec<u64>> {
        let query = r#"
        LET $attr = SELECT *
            FROM sessions
            WHERE mode IS $mode
            WHERE nsfw IS $nsfw
            WHERE last IS $last
            WHERE state.type IS "Accepted"
            ORDER BY rand() LIMIT $limit;
        SELECT state.what AS id FROM $attr
        "#;
        let mut result = self
            .db
            .query(query)
            .bind(("limit", n))
            .bind(("mode", Ross))
            .bind(("nsfw", false))
            .bind(("last", false))
            .await?;
        let users = result
            .take::<Vec<Record<()>>>(1)?
            .into_iter()
            .map(|r| r.id)
            .collect();
        Ok(users)
    }
}

#[derive(Debug, Deserialize)]
pub struct IncompleteGames {
    pub mode: Mode,
    pub count: u64,
    pub round: u64,
    pub nsfw: bool,
}

impl Display for IncompleteGames {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let sfw = if self.nsfw { "NSFW " } else { "" };
        f.write_fmt(format_args!(
            "{:?} mode {}- round {} - available {}",
            self.mode,
            sfw,
            self.round + 1,
            self.count
        ))
    }
}

pub trait PlayableFilter {
    fn filter(self) -> Self;
}

impl PlayableFilter for Vec<IncompleteGames> {
    fn filter(self) -> Self {
        self.into_iter()
            .filter(|p| p.round <= p.mode.last_round() && p.round != 0)
            .collect()
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
