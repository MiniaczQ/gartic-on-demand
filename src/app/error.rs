use std::error::Error;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    Serenity(#[from] serenity::Error),
    #[error("{1}: {0}")]
    UserError(Box<dyn Error + Send + Sync>, &'static str),
    #[error("{1}: {0}")]
    InternalError(Box<dyn Error + Send + Sync>, &'static str),
}

const INTERNAL: &'static str = "Internal server error";

impl AppError {
    pub fn user(e: impl Error + Send + Sync + 'static, msg: &'static str) -> Self {
        Self::UserError(e.into(), msg)
    }

    pub fn internal(e: impl Error + Send + Sync + 'static, msg: &'static str) -> Self {
        Self::InternalError(e.into(), msg)
    }

    pub fn for_user(self) -> &'static str {
        match self {
            AppError::UserError(_, s) => s,
            _ => INTERNAL,
        }
    }
}

pub trait ConvertError<T> {
    fn map_user(self, msg: &'static str) -> Result<T, AppError>;
    fn map_internal(self, msg: &'static str) -> Result<T, AppError>;
}

impl<T, E: Error + Send + Sync + 'static> ConvertError<T> for Result<T, E> {
    fn map_user(self, msg: &'static str) -> Result<T, AppError> {
        self.map_err(|e| AppError::user(e, msg))
    }

    fn map_internal(self, msg: &'static str) -> Result<T, AppError> {
        self.map_err(|e| AppError::user(e, msg))
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Option empty")]
pub struct OptionEmptyError;

impl<T> ConvertError<T> for Option<T> {
    fn map_user(self, msg: &'static str) -> Result<T, AppError> {
        self.ok_or(AppError::user(OptionEmptyError, msg))
    }

    fn map_internal(self, msg: &'static str) -> Result<T, AppError> {
        self.ok_or(AppError::user(OptionEmptyError, msg))
    }
}
