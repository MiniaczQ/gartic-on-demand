use super::{
    attempt::{Active, Approved, Attempt, AttemptState, CreateAttempt},
    user::User,
    Record,
};
use crate::services::{
    database::{BetterCheck, Database, DbResult, MapToNotFound},
    gamemodes::{GameLogic, Mode},
    provider::Provider,
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::ops::Deref;

#[derive(Debug, Serialize, Deserialize)]
pub struct Round {
    pub mode: Mode,
    pub nsfw: bool,
    pub round_no: u64,
    pub multiplex: u64,
    pub created_at: DateTime<Utc>,
}

impl Round {
    pub fn forward(&self) -> Self {
        let round_no = self.round_no + 1;
        let created_at = Utc::now();
        Self {
            mode: self.mode,
            nsfw: self.nsfw,
            round_no: self.round_no + 1,
            multiplex: self.mode.multiplex(round_no),
            created_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct RoundWithAttempts<T> {
    pub attempt: Record<Attempt<T>>,
    #[serde(flatten)]
    pub inner: RoundWithPreviousAttempts,
}

impl<T> Deref for RoundWithAttempts<T> {
    type Target = RoundWithPreviousAttempts;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Debug, Deserialize)]
pub struct RoundWithPreviousAttempts {
    pub round: Record<Round>,
    pub previous: Vec<Record<Attempt<Approved>>>,
}

pub struct RoundRepository {
    db: Database,
}

impl<T> Provider<RoundRepository> for T
where
    T: Provider<Database>,
{
    fn get(&self) -> RoundRepository {
        RoundRepository { db: self.get() }
    }
}

impl RoundRepository {
    pub async fn attempt_new_round(
        &self,
        user: &Record<User>,
        mode: Mode,
        nsfw: bool,
        multiplex: u64,
        time_limit: Duration,
    ) -> DbResult<RoundWithAttempts<Active>> {
        let round_no = 0;
        let now = Utc::now();
        let round = Round {
            mode,
            nsfw,
            round_no,
            multiplex,
            created_at: now,
        };
        let attempt = CreateAttempt {
            state: AttemptState::Active {
                inner: Active {
                    until: now + time_limit,
                },
            },
            created_at: now,
        };
        let mut result = self
            .db
            .query("begin")
            .query("let $round = create round content $round")
            .bind(("round", round))
            .query("let $attempt = relate only $user -> attempt -> $round content $attempt")
            .bind(("user", user))
            .bind(("attempt", attempt))
            .query("commit")
            .query("fn::get_round_with_attempt($attempt)")
            .await?
            .better_check()?;
        let round = result
            .take::<Option<RoundWithAttempts<Active>>>(2)?
            .found()?;
        Ok(round)
    }

    pub async fn forward_complete_round(
        &self,
        previous_round: &Record<Round>,
        attempt: &Record<Attempt<Approved>>,
        round: Round,
    ) -> DbResult<RoundWithPreviousAttempts> {
        let mut result = self
            .db
            .query("begin")
            .query("let $round = create only round content $round")
            .bind(("round", round))
            .query("relate ($previous_round<-previous.in) -> previous -> $round")
            .bind(("previous_round", &previous_round.id))
            .query("relate only $previous_attempt -> previous -> $round")
            .bind(("previous_attempt", &attempt.id))
            .query("commit")
            .query("fn::get_round($round)")
            .await?
            .better_check()?;
        let round = result
            .take::<Option<RoundWithPreviousAttempts>>(3)?
            .found()?;
        Ok(round)
    }

    pub async fn attempt_existing_round(
        &self,
        user: &Record<User>,
        mode: Mode,
        nsfw: bool,
        round_no: u64,
        time_limit: Duration,
    ) -> DbResult<RoundWithAttempts<Active>> {
        let now = Utc::now();
        let attempt = CreateAttempt {
            state: AttemptState::Active {
                inner: Active {
                    until: now + time_limit,
                },
            },
            created_at: now,
        };
        let mut result = self
            .db
            .query("begin")
            .query(
                r"
                let $random = select
                    *,
                    array::len(<-previous<-(attempt where in is $user)) as previously_participated
                    from round
                    where mode = $mode
                    and nsfw = $nsfw
                    and round_no = $round_no
                    and array::len(<-(attempt where state.type in $allocating_state_types)) < multiplex
                    and array::any(<-(attempt where state.type in $allocating_state_types and $user is in)) is false
                    order by rand()
                ",
            )
            .bind(("user", &user.id))
            .bind(("mode", mode))
            .bind(("nsfw", nsfw))
            .bind(("round_no", round_no))
            .query(
                r"
                let $attempt_result = if array::any($random) {
                    let $round = select * from only $random order by previously_participated limit 1;
                    return relate only $user -> attempt -> $round content $attempt;
                }
                "
            )
            .bind(("attempt", attempt))
            .query("commit")
            .query("fn::try_get_round_with_attempt($attempt_result)")
            .await?
        .better_check()?;
        let round = result
            .take::<Option<RoundWithAttempts<Active>>>(2)?
            .found()?;
        Ok(round)
    }

    pub async fn get_active_round(
        &self,
        user: &Record<User>,
    ) -> DbResult<RoundWithAttempts<Active>> {
        let mut result = self
            .db
            .query("let $attempt = select * from only attempt where in is $user and state.type is $state_type")
            .bind(("user", &user.id))
            .bind(("state_type", "Active"))
            .query("fn::try_get_round_with_attempt($attempt)")
            .await?;
        let round = result
            .take::<Option<RoundWithAttempts<Active>>>(1)?
            .found()?;
        Ok(round)
    }
}

#[cfg(test)]
mod tests {
    use super::RoundRepository;
    use crate::services::{
        database::{attempt::AttemptRepository, tests::db, user::UserRepository, DbError},
        gamemodes::Mode,
        provider::Provider,
    };
    use chrono::Duration;

    async fn setup() -> (UserRepository, AttemptRepository, RoundRepository) {
        let db = db().await;
        (db.get(), db.get(), db.get())
    }

    #[tokio::test]
    async fn create_then_find_round() {
        let (users, _, sut) = setup().await;
        let user = users.create_or_update_user(0, "").await.unwrap();

        sut.attempt_new_round(&user, Mode::Ross, false, 1, Duration::seconds(0))
            .await
            .unwrap();
        sut.get_active_round(&user).await.unwrap();
    }

    #[tokio::test]
    async fn create_then_cancel_and_attempt_same_round() {
        let (users, attempts, sut) = setup().await;
        let user = users.create_or_update_user(0, "").await.unwrap();
        let mode = Mode::Ross;
        let nsfw = false;
        let time_limit = Duration::seconds(0);

        sut.attempt_new_round(&user, mode, nsfw, 1, time_limit)
            .await
            .unwrap();
        attempts.cancel_active_attempt(&user).await.unwrap();
        sut.attempt_existing_round(&user, mode, nsfw, 0, time_limit)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn create_then_fail_attempting_same_round() {
        let (users, _, sut) = setup().await;
        let user = users.create_or_update_user(0, "").await.unwrap();
        let mode = Mode::Ross;
        let nsfw = false;
        let time_limit = Duration::seconds(0);

        sut.attempt_new_round(&user, mode, nsfw, 1, time_limit)
            .await
            .unwrap();
        let error = sut
            .attempt_existing_round(&user, mode, nsfw, 0, time_limit)
            .await
            .unwrap_err();
        assert!(matches!(error, DbError::NotFound))
    }

    #[tokio::test]
    async fn fail_attempting_allocated_round() {
        let (users, _, sut) = setup().await;
        let user0 = users.create_or_update_user(0, "").await.unwrap();
        let user1 = users.create_or_update_user(1, "").await.unwrap();
        let mode = Mode::Ross;
        let nsfw = false;
        let time_limit = Duration::seconds(0);

        sut.attempt_new_round(&user0, mode, nsfw, 1, time_limit)
            .await
            .unwrap();
        sut.attempt_existing_round(&user1, mode, nsfw, 0, time_limit)
            .await
            .unwrap_err();
    }

    #[tokio::test]
    async fn succeed_attempting_not_fully_allocated_round() {
        let (users, _, sut) = setup().await;
        let user0 = users.create_or_update_user(0, "").await.unwrap();
        let user1 = users.create_or_update_user(1, "").await.unwrap();
        let mode = Mode::Ross;
        let nsfw = false;
        let time_limit = Duration::seconds(0);

        sut.attempt_new_round(&user0, mode, nsfw, 2, time_limit)
            .await
            .unwrap();
        sut.attempt_existing_round(&user1, mode, nsfw, 0, time_limit)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn complete_round() {
        let (users, attempts, sut) = setup().await;
        let user = users.create_or_update_user(0, "").await.unwrap();
        let mode = Mode::Ross;
        let nsfw = false;
        let time_limit = Duration::seconds(0);
        sut.attempt_new_round(&user, mode, nsfw, 2, time_limit)
            .await
            .unwrap();
        attempts.upload_active_attempt(&user).await.unwrap();
        let round = attempts.approve_uploaded_attempt(&user, 0).await.unwrap();

        sut.forward_complete_round(&round.round, &round.attempt, round.round.forward())
            .await
            .unwrap();
    }

    #[tracing_test::traced_test]
    #[tokio::test]
    #[ignore = "Only use with docker, in memory overflows stack"]
    async fn second_round() {
        let (users, attempts, sut) = setup().await;
        let user = users.create_or_update_user(0, "").await.unwrap();
        let mode = Mode::Ross;
        let nsfw = false;
        let time_limit = Duration::seconds(0);
        sut.attempt_new_round(&user, mode, nsfw, 2, time_limit)
            .await
            .unwrap();
        attempts.upload_active_attempt(&user).await.unwrap();
        let round = attempts.approve_uploaded_attempt(&user, 0).await.unwrap();
        sut.forward_complete_round(&round.round, &round.attempt, round.round.forward())
            .await
            .unwrap();

        let result = sut
            .attempt_existing_round(&user, mode, nsfw, 1, time_limit)
            .await
            .unwrap();

        assert_eq!(result.previous.len(), 1);
    }
}
