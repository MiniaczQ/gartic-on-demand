use serde::Deserialize;

use super::{round::Round, user::User, Record};
use crate::services::{
    database::{BetterCheck, Database, DbResult},
    gamemodes::Mode,
    provider::Provider,
};

pub struct StatsRepository {
    db: Database,
}

impl<T> Provider<StatsRepository> for T
where
    T: Provider<Database>,
{
    fn get(&self) -> StatsRepository {
        StatsRepository { db: self.get() }
    }
}

#[derive(Debug, Deserialize)]
pub struct ActiveUser {
    user: Record<User>,
    round: Record<Round>,
}

#[derive(Debug, Deserialize)]
pub struct UnallocatedRound {
    pub mode: Mode,
    pub nsfw: bool,
    pub round_no: u64,
    pub unallocated: u64,
}

impl StatsRepository {
    pub async fn get_active_users(&self) -> DbResult<Vec<ActiveUser>> {
        let mut result = self
            .db
            .query(
                r"
                select
                    in as user,
                    out as round
                    from attempt
                    where state.type in $active_state_types
                    fetch user, round
                ",
            )
            .await?
            .better_check()?;
        let users = result.take::<Vec<ActiveUser>>(0)?;
        Ok(users)
    }

    pub async fn get_unallocated_rounds(&self) -> DbResult<Vec<UnallocatedRound>> {
        let mut result = self
            .db
            .query(
                r"
                select
                    nsfw,
                    mode,
                    round_no,
                    math::sum(multiplex - array::len(<-(attempt where state.type in $allocating_state_types))) as unallocated
                    from round
                    group by nsfw, mode, round_no
                    order by nsfw, mode, round_no
                ",
            )
            .await?
            .better_check()?;
        let rounds = result.take::<Vec<UnallocatedRound>>(0)?;
        Ok(rounds)
    }
}

#[cfg(test)]
mod tests {
    use super::StatsRepository;
    use crate::services::{
        database::session_v3::{
            attempt::AttemptRepository, round::RoundRepository, tests::db, user::UserRepository,
        },
        gamemodes::Mode,
        provider::Provider,
    };
    use chrono::Duration;

    async fn setup() -> (
        UserRepository,
        RoundRepository,
        AttemptRepository,
        StatsRepository,
    ) {
        let db = db().await;
        (db.get(), db.get(), db.get(), db.get())
    }

    #[tokio::test]
    async fn list_active_users() {
        let (users, rounds, attempts, sut) = setup().await;
        let user0 = users.create_or_update_user(0, "").await.unwrap();
        let user1 = users.create_or_update_user(1, "").await.unwrap();
        let user2 = users.create_or_update_user(2, "").await.unwrap();
        let user3 = users.create_or_update_user(3, "").await.unwrap();
        let _ = users.create_or_update_user(4, "").await.unwrap();
        rounds
            .attempt_new_round(&user0, Mode::Ross, false, 1, Duration::zero())
            .await
            .unwrap();
        rounds
            .attempt_new_round(&user1, Mode::Ross, false, 1, Duration::zero())
            .await
            .unwrap();
        rounds
            .attempt_new_round(&user2, Mode::Ross, false, 1, Duration::zero())
            .await
            .unwrap();
        rounds
            .attempt_new_round(&user3, Mode::Ross, false, 1, Duration::zero())
            .await
            .unwrap();
        attempts.upload_active_attempt(&user0).await.unwrap();
        attempts.upload_active_attempt(&user1).await.unwrap();
        attempts.moderate_uploaded_attempt(&user1, 1).await.unwrap();
        attempts.upload_active_attempt(&user2).await.unwrap();
        attempts
            .approve_uploaded_attempt(&user2, &user2, 2)
            .await
            .unwrap();

        let active_users = sut.get_active_users().await.unwrap();

        assert_eq!(active_users.len(), 3);
    }

    #[tokio::test]
    async fn list_unallocated_rounds() {
        let (users, rounds, attempts, sut) = setup().await;
        let user = users.create_or_update_user(0, "").await.unwrap();
        rounds
            .attempt_new_round(&user, Mode::Ross, true, 4, Duration::zero())
            .await
            .unwrap();
        attempts.cancel_active_attempt(&user).await.unwrap();
        rounds
            .attempt_new_round(&user, Mode::Ross, false, 2, Duration::zero())
            .await
            .unwrap();
        attempts.upload_active_attempt(&user).await.unwrap();
        let round = attempts
            .approve_uploaded_attempt(&user, &user, 0)
            .await
            .unwrap();
        rounds
            .forward_complete_round(&round.round, &round.attempt, round.round.forward())
            .await
            .unwrap();

        let unallocated_rounds = sut.get_unallocated_rounds().await.unwrap();

        assert_eq!(
            unallocated_rounds
                .into_iter()
                .map(|x| x.unallocated)
                .collect::<Vec<_>>(),
            &[1, 2, 4]
        );
    }
}
