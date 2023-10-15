use poise::serenity_prelude::User;
use serenity::http::CacheHttp;

use super::{
    config::CONFIG,
    error::{AppError, OptionEmptyError},
};

pub async fn is_trusted(cache_http: impl CacheHttp, user: &User) -> serenity::Result<bool> {
    let is_trusted: bool = user
        .has_role(&cache_http, CONFIG.guild, CONFIG.roles.trusted)
        .await?;
    Ok(is_trusted)
}

async fn is_mod(cache_http: impl CacheHttp, user: &User) -> serenity::Result<bool> {
    let mut is_mod: bool = user
        .has_role(&cache_http, CONFIG.guild, CONFIG.roles.moderator)
        .await?;
    if !is_mod {
        is_mod = is_admin(&cache_http, user).await?;
    }
    Ok(is_mod)
}

async fn is_admin(cache_http: impl CacheHttp, user: &User) -> serenity::Result<bool> {
    user.has_role(&cache_http, CONFIG.guild, CONFIG.roles.admin)
        .await
}

pub async fn has_mod(cache_http: impl CacheHttp, user: &User) -> Result<(), AppError> {
    is_mod(cache_http, user)
        .await?
        .then_some(())
        .ok_or(AppError::internal(
            OptionEmptyError,
            "Missing required permission",
        ))
}

pub async fn has_admin(cache_http: impl CacheHttp, user: &User) -> Result<(), AppError> {
    is_admin(cache_http, user)
        .await?
        .then_some(())
        .ok_or(AppError::internal(
            OptionEmptyError,
            "Missing required permission",
        ))
}
