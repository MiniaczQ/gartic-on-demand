use super::Record;
use crate::services::{
    database::{Database, DbResult, MapToNotFound},
    provider::Provider,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::warn;

#[derive(Debug, Serialize)]
struct CreateUser<'a> {
    id: u64,
    name: &'a str,
    created_at: DateTime<Utc>,
    notify_once: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub notify_once: bool,
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
        match self.update_name(id, name).await {
            Err(e) => {
                warn!(error = ?e);
                self.create(id, name).await
            }
            Ok(user) => Ok(user),
        }
    }

    async fn create(&self, id: u64, name: &str) -> DbResult<Record<User>> {
        let created_at = Utc::now();
        let user = CreateUser {
            id,
            name,
            created_at,
            notify_once: false,
        };
        let mut result = self
            .db
            .query("create user content $user")
            .bind(("user", user))
            .await?;
        let user = result.take::<Option<Record<User>>>(0)?.found()?;
        Ok(user)
    }

    async fn update_name(&self, id: u64, name: &str) -> DbResult<Record<User>> {
        let mut result = self
            .db
            .query("update user set name = $name where meta::id(id) is $id")
            .bind(("name", name))
            .bind(("id", id))
            .await?;
        let user = result.take::<Option<Record<User>>>(0)?.found()?;
        Ok(user)
    }

    pub async fn update_notify_once(&self, id: u64, notify_once: bool) -> DbResult<Record<User>> {
        let mut result = self
            .db
            .query("update user set notify_once = $notify_once where meta::id(id) is $id")
            .bind(("notify_once", notify_once))
            .bind(("id", id))
            .await?;
        let user = result.take::<Option<Record<User>>>(0)?.found()?;
        Ok(user)
    }

    pub async fn get_user(&self, id: u64) -> DbResult<Record<User>> {
        let mut result = self
            .db
            .query("select * from only user where meta::id(id) is $id")
            .bind(("id", id))
            .await?;
        let user = result.take::<Option<Record<User>>>(0)?.found()?;
        Ok(user)
    }

    pub async fn take_users_to_notify_once(&self) -> DbResult<Vec<Record<User>>> {
        let mut result = self
            .db
            .query("update user set notify_once = false where notify_once is true")
            .await?;
        let users = result.take::<Vec<Record<User>>>(0)?;
        Ok(users)
    }
}

#[cfg(test)]
mod tests {
    use super::UserRepository;
    use crate::services::{database::tests::db, provider::Provider};

    async fn setup() -> UserRepository {
        let db = db().await;
        db.get()
    }

    #[tokio::test]
    async fn fail_second_create() {
        let sut = setup().await;

        sut.create(0, "a").await.unwrap();
        sut.create(0, "a").await.unwrap_err();
    }

    #[tokio::test]
    async fn fail_update_without_create() {
        let sut = setup().await;

        sut.update_name(0, "a").await.unwrap_err();
    }

    #[tokio::test]
    async fn succeed_update_after_create() {
        let sut = setup().await;

        sut.create(0, "a").await.unwrap();
        sut.update_name(0, "a").await.unwrap();
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

    #[tokio::test]
    async fn update_user_notify_once() {
        let sut = setup().await;
        let user = sut.create_or_update_user(0, "a").await.unwrap();

        let r1 = sut.update_notify_once(user.id(), true).await.unwrap();
        let r2 = sut.update_notify_once(user.id(), false).await.unwrap();

        assert_eq!(r1.notify_once, true);
        assert_eq!(r2.notify_once, false);
    }

    #[tokio::test]
    async fn take_users_with_notify_once() {
        let sut = setup().await;
        let u1 = sut.create_or_update_user(0, "a").await.unwrap();
        sut.create_or_update_user(1, "a").await.unwrap();
        sut.update_notify_once(u1.id(), true).await.unwrap();

        let r1 = sut.take_users_to_notify_once().await.unwrap();
        let r2 = sut.take_users_to_notify_once().await.unwrap();

        assert_eq!(r1.len(), 1);
        assert_eq!(r1[0].id(), 0);
        assert_eq!(r2.len(), 0);
    }
}
