use super::Record;
use crate::services::{
    database::{Database, DbResult, MapToNotFound},
    provider::Provider,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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
pub struct Accepted {
    pub when: DateTime<Utc>,
    pub who: u64,
    pub what: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Rejected {
    pub when: DateTime<Utc>,
    pub who: u64,
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
    Accepted {
        #[serde(flatten)]
        inner: Accepted,
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
    pub async fn cancel_attempt(&self, user_id: u64) -> DbResult<Record<Attempt<Cancelled>>> {
        let now = Utc::now();
        let state = AttemptState::Cancelled {
            inner: Cancelled { when: now },
        };
        let mut result = self
            .db
            .query("update only attempt content state = $state where meta::id(in) is $user_id and state.type = state_type")
            .bind(("state_type", "Active"))
            .bind(("user_id", user_id))
            .bind(("state", state))
            .await?;
        let attempt = result
            .take::<Option<Record<Attempt<Cancelled>>>>(0)?
            .found()?;
        Ok(attempt)
    }

    pub async fn expire_attempt(&self) -> DbResult<Vec<Record<Attempt<Expired>>>> {
        let now = Utc::now();
        let state = AttemptState::Expired {
            inner: Expired { when: now },
        };
        let mut result = self
            .db
            .query("update only attempt content state = $state where state.type = state_type and state.until < $now")
            .bind(("state_type", "Active"))
            .bind(("state", state))
            .bind(("now", now))
            .await?;
        let attempt = result.take::<Vec<Record<Attempt<Expired>>>>(0)?;
        Ok(attempt)
    }

    pub async fn start_uploading_attempt(
        &self,
        user_id: u64,
    ) -> DbResult<Record<Attempt<Uploading>>> {
        let now = Utc::now();
        let state = AttemptState::Uploading {
            inner: Uploading { since: now },
        };
        let mut result = self
            .db
            .query("update only attempt content state = $state where meta::id(in) is $user_id and state.type = state_type")
            .bind(("state_type", "Active"))
            .bind(("user_id", user_id))
            .bind(("state", state))
            .await?;
        let attempt = result
            .take::<Option<Record<Attempt<Uploading>>>>(0)?
            .found()?;
        Ok(attempt)
    }

    pub async fn approve_upload_attempt(
        &self,
        user_id: u64,
        image_id: u64,
        mod_id: u64,
    ) -> DbResult<Record<Attempt<Accepted>>> {
        let now = Utc::now();
        let state = AttemptState::Accepted {
            inner: Accepted {
                when: now,
                who: mod_id,
                what: image_id,
            },
        };
        let mut result = self
            .db
            .query("update only attempt content state = $state where meta::id(in) is $user_id and state.type = state_type")
            .bind(("state_type", "Uploading"))
            .bind(("user_id", user_id))
            .bind(("state", state))
            .await?;
        let attempt = result
            .take::<Option<Record<Attempt<Accepted>>>>(0)?
            .found()?;
        Ok(attempt)
    }

    pub async fn moderate_upload_attempt(
        &self,
        user_id: u64,
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
            .query("update only attempt content state = $state where meta::id(in) is $user_id and state.type = state_type")
            .bind(("state_type", "Uploading"))
            .bind(("user_id", user_id))
            .bind(("state", state))
            .await?;
        let attempt = result
            .take::<Option<Record<Attempt<Pending>>>>(0)?
            .found()?;
        Ok(attempt)
    }

    pub async fn approve_attempt(
        &self,
        user_id: u64,
        image_id: u64,
        mod_id: u64,
    ) -> DbResult<Record<Attempt<Accepted>>> {
        let now = Utc::now();
        let state = AttemptState::Accepted {
            inner: Accepted {
                when: now,
                who: mod_id,
                what: image_id,
            },
        };
        let mut result = self
            .db
            .query("update only attempt content state = $state where meta::id(in) is $user_id and state.type = state_type")
            .bind(("state_type", "Pending"))
            .bind(("user_id", user_id))
            .bind(("state", state))
            .await?;
        let attempt = result
            .take::<Option<Record<Attempt<Accepted>>>>(0)?
            .found()?;
        Ok(attempt)
    }

    pub async fn reject_attempt(
        &self,
        user_id: u64,
        image_id: u64,
        mod_id: u64,
    ) -> DbResult<Record<Attempt<Rejected>>> {
        let now = Utc::now();
        let state = AttemptState::Rejected {
            inner: Rejected {
                when: now,
                who: mod_id,
                what: image_id,
            },
        };
        let mut result = self
            .db
            .query("update only attempt content state = $state where meta::id(in) is $user_id and state.type = state_type")
            .bind(("state_type", "Pending"))
            .bind(("user_id", user_id))
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
    use super::AttemptRepository;
    use crate::services::{database::session_v3::tests::db, provider::Provider};

    async fn sut() -> AttemptRepository {
        let db = db().await;
        db.get()
    }
}
