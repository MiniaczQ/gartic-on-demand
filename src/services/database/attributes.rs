use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serenity::model::prelude::UserId;
use surrealdb::sql::{Id, Thing};
use surrealdb::{Connection, Surreal};

const TABLE: &str = "attributes";

#[derive(Debug, Deserialize)]
struct AttributeQueryRaw {
    id: Thing,
    author_id: UserId,
    url: String,
    created_at: DateTime<Utc>,
}

impl From<AttributeQueryRaw> for AttributeQuery {
    fn from(value: AttributeQueryRaw) -> Self {
        let Id::Number(id) = value.id.id else {
            panic!()
        };
        AttributeQuery {
            id: id as u64,
            url: value.url,
            author_id: value.author_id,
            created_at: value.created_at,
        }
    }
}

#[derive(Debug)]
pub struct AttributeQuery {
    pub id: u64,
    pub author_id: UserId,
    pub url: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct Attribute<'a> {
    author_id: UserId,
    url: &'a str,
    created_at: DateTime<Utc>,
}

impl<'a> Attribute<'a> {
    pub fn new(author_id: UserId, url: &'a str) -> Self {
        Self {
            author_id,
            url,
            created_at: Utc::now(),
        }
    }
}

#[async_trait]
pub trait AttributeRepository {
}

#[async_trait]
impl<T: Connection> AttributeRepository for Surreal<T> {
}
