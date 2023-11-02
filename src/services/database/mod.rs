pub mod assets;
pub mod migrations;
pub mod session;
pub mod session_v3;

use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use serde::{Deserialize, Serialize};
use surrealdb::{
    engine::any::{connect, Any},
    sql::{Id, Thing},
    Surreal,
};

use self::migrations::MigratorConfig;

#[derive(Debug, Deserialize)]
pub struct DatabaseConfig {
    pub address: String,
    pub namespace: String,
    pub database: String,
    pub migrator: MigratorConfig,
}

#[derive(Clone)]
pub struct Database {
    inner: Surreal<Any>,
}

impl Database {
    pub async fn setup(config: &DatabaseConfig) -> DbResult<Self> {
        let inner = connect(&config.address).await.unwrap();
        inner
            .use_ns(&config.namespace)
            .use_db(&config.database)
            .await?;
        Ok(Database { inner })
    }
}

impl Deref for Database {
    type Target = Surreal<Any>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Deserialize)]
pub struct RawRecord<T = ()> {
    id: Thing,
    #[serde(flatten)]
    entry: T,
}

#[derive(Debug, thiserror::Error)]
#[error("Id could not be converted to the correct type")]
pub struct IdConversionError;

impl<T> TryFrom<RawRecord<T>> for Record<T> {
    type Error = IdConversionError;

    fn try_from(value: RawRecord<T>) -> Result<Self, Self::Error> {
        let Id::Number(id) = value.id.id else {
            Err(IdConversionError)?
        };
        let id: u64 = id.try_into().map_err(|_| IdConversionError)?;
        Ok(Record {
            id,
            entry: value.entry,
        })
    }
}

pub trait IdConvert {
    type Target;

    fn convert_id(self) -> Result<Self::Target, IdConversionError>;
}

impl<T> IdConvert for RawRecord<T> {
    type Target = Record<T>;

    fn convert_id(self) -> Result<Self::Target, IdConversionError> {
        self.try_into()
    }
}

impl<T> IdConvert for Vec<RawRecord<T>> {
    type Target = Vec<Record<T>>;

    fn convert_id(self) -> Result<Self::Target, IdConversionError> {
        self.into_iter().map(Record::try_from).collect()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Record<T = ()> {
    pub id: u64,
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

#[derive(Deserialize)]
pub struct Count {
    pub count: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("{0}")]
    IdConversion(#[from] IdConversionError),
    #[error("{0}")]
    Database(#[from] surrealdb::Error),
    #[error("{0:?}")]
    DatabaseCheck(HashMap<usize, surrealdb::Error>),
    #[error("Not found")]
    NotFound,
}

pub type DbResult<T> = Result<T, DbError>;

pub trait MapToNotFound<T> {
    fn found(self) -> DbResult<T>;
}

impl<T> MapToNotFound<T> for Option<T> {
    fn found(self) -> DbResult<T> {
        self.ok_or(DbError::NotFound)
    }
}

pub trait BetterCheck
where
    Self: Sized,
{
    fn better_check(self) -> DbResult<Self>;
}

impl BetterCheck for surrealdb::Response {
    fn better_check(mut self) -> DbResult<Self> {
        let errors = self.take_errors();
        if errors.is_empty() {
            Ok(self)
        } else {
            Err(DbError::DatabaseCheck(errors))
        }
    }
}
