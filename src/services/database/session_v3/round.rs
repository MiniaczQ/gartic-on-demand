use super::{attempt::Attempt, Record};
use crate::services::{
    database::{session::Accepted, Database, DbResult, MapToNotFound},
    gamemodes::{GameLogic, Mode},
    provider::Provider,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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
    async fn begin_round(&self, mode: Mode, nsfw: bool, multiplex: u64) -> DbResult<Record<Round>> {
        let round = 0;
        let created_at = Utc::now();
        let round = Round {
            mode,
            nsfw,
            round_no: round,
            multiplex,
            created_at,
        };
        let mut result = self
            .db
            .query("create round content $round")
            .bind(("round", round))
            .await?;
        let user = result.take::<Option<Record<Round>>>(0)?.found()?;
        Ok(user)
    }

    async fn forward_round(
        &self,
        previous_round: &Round,
        accepted_attempt: &Record<Attempt<Accepted>>,
    ) -> DbResult<Record<Round>> {
        let round = previous_round.forward();
        let mut result = self
            .db
            .query("begin")
            .query("create round content $round")
            .bind(("round", round))
            .query("")
            .query("")
            .query("commit")
            .await?;
        todo!();
        let user = result.take::<Option<Record<Round>>>(1)?.found()?;
        Ok(user)
    }

    async fn find_round(
        &self,
        uid: u64,
        mode: Mode,
        nsfw: bool,
        round_no: u64,
    ) -> DbResult<Record<Round>> {
        let mut result = self
            .db
            .query(
                r#"
                select value * from round
                    where 
            "#,
            )
            .bind(("uid", uid))
            .bind(("mode", mode))
            .bind(("nsfw", nsfw))
            .bind(("round_no", round_no))
            .await?;
        let user = result.take::<Option<Record<Round>>>(0)?.found()?;
        Ok(user)
    }
}

#[cfg(test)]
mod tests {
    use super::RoundRepository;
    use crate::services::{database::session_v3::tests::db, provider::Provider};

    async fn sut() -> RoundRepository {
        let db = db().await;
        db.get()
    }
}
