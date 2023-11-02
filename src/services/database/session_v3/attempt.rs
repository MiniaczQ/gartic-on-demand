use super::{round::RoundWithAttempts, user::User, Record};
use crate::services::{
    database::{Database, DbResult, MapToNotFound},
    provider::Provider,
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;

#[derive(Debug, Serialize, Deserialize)]
pub struct Active {
    pub until: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Cancelled {
    pub when: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Expired {
    pub when: DateTime<Utc>,
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

#[derive(Debug, Serialize, Deserialize)]
pub struct Approved {
    pub when: DateTime<Utc>,
    pub who: Thing,
    pub what: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Rejected {
    pub when: DateTime<Utc>,
    pub who: Thing,
    pub what: u64,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum AttemptState {
    Active {
        #[serde(flatten)]
        inner: Active,
    },
    Cancelled {
        #[serde(flatten)]
        inner: Cancelled,
    },
    Expired {
        #[serde(flatten)]
        inner: Expired,
    },
    Uploading {
        #[serde(flatten)]
        inner: Uploading,
    },
    Pending {
        #[serde(flatten)]
        inner: Pending,
    },
    Approved {
        #[serde(flatten)]
        inner: Approved,
    },
    Rejected {
        #[serde(flatten)]
        inner: Rejected,
    },
}

#[derive(Debug, Deserialize)]
pub struct Attempt<T> {
    pub state: T,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct CreateAttempt {
    pub state: AttemptState,
    pub created_at: DateTime<Utc>,
}

pub struct AttemptRepository {
    db: Database,
}

impl<T> Provider<AttemptRepository> for T
where
    T: Provider<Database>,
{
    fn get(&self) -> AttemptRepository {
        AttemptRepository { db: self.get() }
    }
}

impl AttemptRepository {
    pub async fn extend_active_attempt(
        &self,
        user: &Record<User>,
        time_limit: Duration,
    ) -> DbResult<Record<Attempt<Active>>> {
        let now = Utc::now();
        let state = AttemptState::Active {
            inner: Active {
                until: now + time_limit,
            },
        };
        let mut result = self
            .db
            .query("update only attempt set state = $state where in is $user and state.type = $state_type")
            .bind(("state_type", "Active"))
            .bind(("user", &user.id))
            .bind(("state", state))
            .await?;
        let attempt = result.take::<Option<Record<Attempt<Active>>>>(0)?.found()?;
        Ok(attempt)
    }

    pub async fn cancel_active_attempt(
        &self,
        user: &Record<User>,
    ) -> DbResult<Record<Attempt<Cancelled>>> {
        let now = Utc::now();
        let state = AttemptState::Cancelled {
            inner: Cancelled { when: now },
        };
        let mut result = self
            .db
            .query("update only attempt set state = $state where in is $user and state.type = $state_type")
            .bind(("state_type", "Active"))
            .bind(("user", &user.id))
            .bind(("state", state))
            .await?;
        let attempt = result
            .take::<Option<Record<Attempt<Cancelled>>>>(0)?
            .found()?;
        Ok(attempt)
    }

    pub async fn expire_active_attempts(&self) -> DbResult<Vec<Record<()>>> {
        let now = Utc::now();
        let state = AttemptState::Expired {
            inner: Expired { when: now },
        };
        let mut result = self
            .db
            .query("update attempt set state = $state where state.type = $state_type and state.until < $now")
            .bind(("state_type", "Active"))
            .bind(("state", state))
            .bind(("now", now))
            .await?;
        let attempt = result.take::<Vec<Record<()>>>(0)?;
        Ok(attempt)
    }

    pub async fn upload_active_attempt(
        &self,
        user: &Record<User>,
    ) -> DbResult<Record<Attempt<Uploading>>> {
        let now = Utc::now();
        let state = AttemptState::Uploading {
            inner: Uploading { since: now },
        };
        let mut result = self
            .db
            .query("update only attempt set state = $state where in is $user and state.type = $state_type")
            .bind(("state_type", "Active"))
            .bind(("user", &user.id))
            .bind(("state", state))
            .await?;
        let attempt = result
            .take::<Option<Record<Attempt<Uploading>>>>(0)?
            .found()?;
        Ok(attempt)
    }

    pub async fn approve_uploaded_attempt(
        &self,
        user: &Record<User>,
        reviewer: &Record<User>,
        image_id: u64,
    ) -> DbResult<RoundWithAttempts<Approved>> {
        let now = Utc::now();
        let state = AttemptState::Approved {
            inner: Approved {
                when: now,
                who: reviewer.id.clone(),
                what: image_id,
            },
        };
        let mut result = self
            .db
            .query("let $attempt = update only attempt set state = $state where in is $user and state.type = $state_type")
            .bind(("state_type", "Uploading"))
            .bind(("user", &user.id))
            .bind(("state", state))
            .query("fn::get_round_with_attempt($attempt)")
            .await?;
        let attempt = result
            .take::<Option<RoundWithAttempts<Approved>>>(1)?
            .found()?;
        Ok(attempt)
    }

    pub async fn moderate_uploaded_attempt(
        &self,
        user: &Record<User>,
        image_id: u64,
    ) -> DbResult<Record<Attempt<Pending>>> {
        let now = Utc::now();
        let state = AttemptState::Pending {
            inner: Pending {
                since: now,
                what: image_id,
            },
        };
        let mut result = self
            .db
            .query("update only attempt set state = $state where in is $user and state.type = $state_type")
            .bind(("state_type", "Uploading"))
            .bind(("user", &user.id))
            .bind(("state", state))
            .await?;
        let attempt = result
            .take::<Option<Record<Attempt<Pending>>>>(0)?
            .found()?;
        Ok(attempt)
    }

    pub async fn approve_pending_attempt(
        &self,
        user: &Record<User>,
        reviewer: &Record<User>,
        image_id: u64,
    ) -> DbResult<RoundWithAttempts<Approved>> {
        let now = Utc::now();
        let state = AttemptState::Approved {
            inner: Approved {
                when: now,
                who: reviewer.id.clone(),
                what: image_id,
            },
        };
        let mut result = self
            .db
            .query("let $attempt = update only attempt set state = $state where in is $user and state.type = $state_type")
            .bind(("state_type", "Pending"))
            .bind(("user", &user.id))
            .bind(("state", state))
            .query("fn::get_round_with_attempt($attempt)")
            .await?;
        let attempt = result
            .take::<Option<RoundWithAttempts<Approved>>>(1)?
            .found()?;
        Ok(attempt)
    }

    pub async fn reject_pending_attempt(
        &self,
        user: &Record<User>,
        reviewer: &Record<User>,
        image_id: u64,
    ) -> DbResult<Record<Attempt<Rejected>>> {
        let now = Utc::now();
        let state = AttemptState::Rejected {
            inner: Rejected {
                when: now,
                who: reviewer.id.clone(),
                what: image_id,
            },
        };
        let mut result = self
            .db
            .query("update only attempt set state = $state where in is $user and state.type = $state_type")
            .bind(("state_type", "Pending"))
            .bind(("user", &user.id))
            .bind(("state", state))
            .await?;
        let attempt = result
            .take::<Option<Record<Attempt<Rejected>>>>(0)?
            .found()?;
        Ok(attempt)
    }
}

#[cfg(test)]
mod tests {
    use chrono::Duration;

    use super::AttemptRepository;
    use crate::services::{
        database::session_v3::{round::RoundRepository, tests::db, user::UserRepository},
        gamemodes::Mode,
        provider::Provider,
    };

    async fn setup() -> (UserRepository, RoundRepository, AttemptRepository) {
        let db = db().await;
        (db.get(), db.get(), db.get())
    }

    #[tokio::test]
    async fn extend_attempt_to_avoid_expiration() {
        let (users, rounds, sut) = setup().await;
        let user = users.create_or_update_user(0, "").await.unwrap();
        rounds
            .attempt_new_round(&user, Mode::Ross, false, 1, Duration::seconds(-60 * 60))
            .await
            .unwrap();

        sut.extend_active_attempt(&user, Duration::seconds(60 * 60))
            .await
            .unwrap();

        let expired = sut.expire_active_attempts().await.unwrap();
        assert!(expired.is_empty());
    }

    #[tokio::test]
    async fn expire_one_out_of_two() {
        let (users, rounds, sut) = setup().await;
        let user = users.create_or_update_user(0, "").await.unwrap();
        rounds
            .attempt_new_round(&user, Mode::Ross, false, 1, Duration::seconds(-60 * 60))
            .await
            .unwrap();
        rounds
            .attempt_new_round(&user, Mode::Ross, false, 1, Duration::seconds(60 * 60))
            .await
            .unwrap();

        let expired = sut.expire_active_attempts().await.unwrap();

        assert_eq!(expired.len(), 1);
    }

    #[tokio::test]
    async fn cancel_attempt() {
        let (users, rounds, sut) = setup().await;
        let user = users.create_or_update_user(0, "").await.unwrap();
        rounds
            .attempt_new_round(&user, Mode::Ross, false, 1, Duration::seconds(-60 * 60))
            .await
            .unwrap();

        sut.cancel_active_attempt(&user).await.unwrap();
    }

    #[tokio::test]
    async fn accept_unmoderated_attempt() {
        let (users, rounds, sut) = setup().await;
        let user = users.create_or_update_user(0, "").await.unwrap();
        rounds
            .attempt_new_round(&user, Mode::Ross, false, 1, Duration::seconds(-60 * 60))
            .await
            .unwrap();

        sut.upload_active_attempt(&user).await.unwrap();
        sut.approve_uploaded_attempt(&user, &user, 0).await.unwrap();
    }

    #[tokio::test]
    async fn accept_moderated_attempt() {
        let (users, rounds, sut) = setup().await;
        let user = users.create_or_update_user(0, "").await.unwrap();
        rounds
            .attempt_new_round(&user, Mode::Ross, false, 1, Duration::seconds(-60 * 60))
            .await
            .unwrap();

        sut.upload_active_attempt(&user).await.unwrap();
        sut.moderate_uploaded_attempt(&user, 0).await.unwrap();
        sut.approve_pending_attempt(&user, &user, 0).await.unwrap();
    }

    #[tokio::test]
    async fn reject_moderated_attempt() {
        let (users, rounds, sut) = setup().await;
        let user = users.create_or_update_user(0, "").await.unwrap();
        rounds
            .attempt_new_round(&user, Mode::Ross, false, 1, Duration::seconds(-60 * 60))
            .await
            .unwrap();

        sut.upload_active_attempt(&user).await.unwrap();
        sut.moderate_uploaded_attempt(&user, 0).await.unwrap();
        sut.reject_pending_attempt(&user, &user, 0).await.unwrap();
    }
}
