use super::attempt::{Approved, Attempt};
use crate::services::{
    database::{Database, DbResult},
    provider::Provider,
};

pub struct ByproductsRepository {
    db: Database,
}

impl<T> Provider<ByproductsRepository> for T
where
    T: Provider<Database>,
{
    fn get(&self) -> ByproductsRepository {
        ByproductsRepository { db: self.get() }
    }
}

impl ByproductsRepository {
    pub async fn get_random_ross_attributes(&self) -> DbResult<Vec<Attempt<Approved>>> {
        let mut result = self
            .db
            .query(
                r"
                select * from attempt
                    where out.mode is $mode
                    and out.round_no in $round_nos
                    and out.nsfw is false
                    order by rand()
                    limit 4
                ",
            )
            .bind(("mode", "Ross"))
            .bind(("round_nos", &[0, 1, 2, 3]))
            .await?;
        let attempts = result.take::<Vec<Attempt<Approved>>>(0)?;
        Ok(attempts)
    }
}

#[cfg(test)]
mod tests {
    use super::ByproductsRepository;
    use crate::services::{
        database::{
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
        ByproductsRepository,
    ) {
        let db = db().await;
        (db.get(), db.get(), db.get(), db.get())
    }

    #[tokio::test]
    async fn no_attributes_available() {
        let (_, _, _, sut) = setup().await;

        let attributes = sut.get_random_ross_attributes().await.unwrap();
        assert_eq!(attributes.len(), 0);
    }

    #[tokio::test]
    async fn enough_attributes_available() {
        let (users, rounds, attempts, sut) = setup().await;
        let user = users.create_or_update_user(0, "").await.unwrap();

        for i in 0..4 {
            rounds
                .attempt_new_round(&user, Mode::Ross, false, 0, Duration::zero())
                .await
                .unwrap();
            attempts.upload_active_attempt(&user).await.unwrap();
            let round = attempts.approve_uploaded_attempt(&user, i).await.unwrap();
            rounds
                .forward_complete_round(&round.round, &round.attempt, round.round.forward())
                .await
                .unwrap();
        }

        let attributes = sut.get_random_ross_attributes().await.unwrap();
        assert_eq!(attributes.len(), 4);
    }
}
