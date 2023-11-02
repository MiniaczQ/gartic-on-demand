use super::Record;
use crate::services::{
    database::{Database, DbResult, MapToNotFound},
    provider::Provider,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
struct CreateUser<'a> {
    id: u64,
    name: &'a str,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub name: String,
    pub created_at: DateTime<Utc>,
}

pub struct UserRepository {
    db: Database,
}

impl<T> Provider<UserRepository> for T
where
    T: Provider<Database>,
{
    fn get(&self) -> UserRepository {
        UserRepository { db: self.get() }
    }
}

impl UserRepository {
    pub async fn create_or_update_user(&self, id: u64, name: &str) -> DbResult<Record<User>> {
        match self.update_user(id, name).await {
            Err(_) => self.create_user(id, name).await,
            Ok(user) => Ok(user),
        }
    }

    async fn create_user(&self, id: u64, name: &str) -> DbResult<Record<User>> {
        let created_at = Utc::now();
        let user = CreateUser {
            id,
            name,
            created_at,
        };
        let mut result = self
            .db
            .query("create user content $user")
            .bind(("user", user))
            .await?;
        let user = result.take::<Option<Record<User>>>(0)?.found()?;
        Ok(user)
    }

    async fn update_user(&self, id: u64, name: &str) -> DbResult<Record<User>> {
        let mut result = self
            .db
            .query("update user set name = $name where meta::id(id) is $id")
            .bind(("name", name))
            .bind(("id", id))
            .await?;
        let user = result.take::<Option<Record<User>>>(0)?.found()?;
        Ok(user)
    }

    async fn get_user(&self, id: u64) -> DbResult<Record<User>> {
        let mut result = self
            .db
            .query("select * from only user where meta::id(id) is $id")
            .bind(("id", id))
            .await?;
        let user = result.take::<Option<Record<User>>>(0)?.found()?;
        Ok(user)
    }
}

#[cfg(test)]
mod tests {
    use super::UserRepository;
    use crate::services::{database::session_v3::tests::db, provider::Provider};

    async fn setup() -> UserRepository {
        let db = db().await;
        db.get()
    }

    #[tokio::test]
    async fn fail_second_create() {
        let sut = setup().await;

        sut.create_user(0, "a").await.unwrap();
        sut.create_user(0, "a").await.unwrap_err();
    }

    #[tokio::test]
    async fn fail_update_without_create() {
        let sut = setup().await;

        sut.update_user(0, "a").await.unwrap_err();
    }

    #[tokio::test]
    async fn succeed_update_after_create() {
        let sut = setup().await;

        sut.create_user(0, "a").await.unwrap();
        sut.update_user(0, "a").await.unwrap();
    }

    #[tokio::test]
    async fn succeed_create_then_update() {
        let sut = setup().await;

        sut.create_or_update_user(0, "a").await.unwrap();
        sut.create_or_update_user(0, "a").await.unwrap();
    }

    #[tokio::test]
    async fn get_existing_user() {
        let sut = setup().await;

        sut.create_or_update_user(0, "a").await.unwrap();
        sut.get_user(0).await.unwrap();
    }

    #[tokio::test]
    async fn fail_get_non_existing_user() {
        let sut = setup().await;

        sut.get_user(0).await.unwrap_err();
    }
}
