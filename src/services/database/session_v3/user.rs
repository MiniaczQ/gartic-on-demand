use super::{Database, DbResult};
use crate::services::provider::Provider;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct User<'a> {
    pub name: &'a str,
    pub created_at: DateTime<Utc>,
}

pub struct UsersRepository {
    db: Database,
}

impl<T> Provider<UsersRepository> for T
where
    T: Provider<Database>,
{
    fn get(&self) -> UsersRepository {
        UsersRepository { db: self.get() }
    }
}

impl UsersRepository {
    pub async fn create_user(&self, uid: u64, name: &str) -> DbResult<()> {
        todo!()
    }

    pub async fn update_user(&self, uid: u64, name: &str) -> DbResult<()> {
        todo!()
    }
}
