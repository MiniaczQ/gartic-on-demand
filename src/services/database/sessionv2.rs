use std::{ops::Add, time::Duration};

use super::{Database, DbResult, MapToNotFound};
use crate::services::{gamemodes::Mode, provider::Provider};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct User;

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
    started_at: DateTime<Utc>,
    state: SessionState,
    #[serde(rename = "in")]
    u: User,
    #[serde(rename = "out")]
    m: Match,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Match {
    mode: Mode,
}

pub struct SessionRepository {
    db: Database,
}

impl SessionRepository {
    pub async fn get(&self, uid: u64) -> DbResult<Session> {
        let query = r#"
        SELECT * FROM sessions
        WHERE u.id == $uid
        AND state.type = Active
        "#;
        let mut result = self.db.query(query).bind(("uid", uid)).await?;
        result.take::<Option<Session>>(0)?.found()
    }

    /// Active -> Expired
    pub async fn stop_expired(&self) -> DbResult<()> {
        let now = Utc::now();
        let expired = SessionState::Expired { when: now };
        let query = r#"
        UPDATE sessions
        WHERE state.type = Active
        AND state.until < $now
        SET state = $expired
        "#;
        self.db
            .query(query)
            .bind(("now", now))
            .bind(("expired", expired))
            .await?;
        Ok(())
    }

    /// Active -> Uploading
    pub async fn start_submitting(&self, uid: u64) -> DbResult<()> {
        let now: DateTime<Utc> = Utc::now();
        let pending = SessionState::Uploading { since: now };
        let query = r#"
        UPDATE sessions
        WHERE u.id = $uid
        AND state.type = Active
        SET state = $pending
        "#;
        self.db
            .query(query)
            .bind(("uid", uid))
            .bind(("pending", pending))
            .await?;
        Ok(())
    }

    /// TEMP: Uploading -> Accepted
    pub async fn finish_submitting(&self, uid: u64, aid: u64) -> DbResult<()> {
        let now: DateTime<Utc> = Utc::now();
        let accepted = SessionState::Accepted {
            when: now,
            who: uid,
            what: aid,
        };
        let query = r#"
        UPDATE sessions
        WHERE u.id = $uid
        AND state.type = Uploading
        AND state.what = NONE
        SET state = $accepted
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
        let now: DateTime<Utc> = Utc::now();
        let cancelled = SessionState::Cancelled { when: now };
        let query = r#"
        UPDATE sessions
        WHERE u.id = $uid
        AND state.type = Active
        SET state = $cancelled
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
        time: Duration,
    ) -> DbResult<Session> {
        let until = Utc::now().add(time);
        let active = SessionState::Active { until };
        let query = r#"
        LET user = type::thing(users, $uid);
        LET match = SELECT * FROM ONLY sessions
                    WHERE m.mode = $mode
                    AND array::any(->(sessions WHERE u.id IS NOT $user)) IS false
                    AND array::any(->(sessions WHERE state.type IN [Uploading, Pending])) IS false
                    AND array::len(->(sessions WHERE state.type = Accepted)) = $round
                    ORDER BY rand() LIMIT 1
        RELATE ONLY $user->sessions->$match CONTENT $active;
        "#;
        let mut result = self
            .db
            .query(query)
            .bind(("uid", uid))
            .bind(("mode", mode))
            .bind(("round", round))
            .bind(("active", active))
            .await?;
        result.take::<Option<Session>>(0)?.found()
    }

    /// -> Active
    pub async fn create_attach(&self, uid: u64, mode: Mode, time: Duration) -> DbResult<Session> {
        let until = Utc::now().add(time);
        let active = SessionState::Active { until };
        let match_ = Match { mode };
        let query = r#"
        LET user = type::thing(users, $uid);
        LET match = CREATE ONLY matches CONTENT $match;
        RELATE ONLY $user->sessions->$match CONTENT $active;
        "#;
        let mut result = self
            .db
            .query(query)
            .bind(("uid", uid))
            .bind(("match", match_))
            .bind(("active", active))
            .await?;
        result.take::<Option<Session>>(0)?.found()
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
