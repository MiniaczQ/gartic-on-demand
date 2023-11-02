use std::ops::{Deref, DerefMut};

use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;

pub mod attempt;
pub mod round;
pub mod user;

#[derive(Debug, Serialize, Deserialize)]
pub struct Record<T = ()> {
    pub id: Thing,
    #[serde(flatten)]
    pub entry: T,
}

impl<T> Deref for Record<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.entry
    }
}

impl<T> DerefMut for Record<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.entry
    }
}

#[cfg(test)]
mod tests {
    use crate::services::database::Database;
    use surrealdb::engine::any::connect;

    pub async fn db() -> Database {
        let db = Database {
            inner: connect("mem://").await.unwrap(),
        };
        db.use_ns("test").use_db("test").await.unwrap();
        db
    }
}
