use rand::RngCore;
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;

use super::jwt::now_secs;
use crate::error::{AppError, AppResult};

fn new_token() -> String {
    let mut bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    hex::encode(bytes)
}

fn hash_token(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

/// Starts a new token family (a fresh login) and returns the opaque refresh token.
pub async fn issue(pool: &SqlitePool, user_id: i64, ttl_secs: u64) -> AppResult<String> {
    let family = uuid::Uuid::new_v4().to_string();
    insert(pool, user_id, &family, ttl_secs)
        .await
        .map(|(token, _)| token)
}

async fn insert(
    pool: &SqlitePool,
    user_id: i64,
    family: &str,
    ttl_secs: u64,
) -> AppResult<(String, i64)> {
    let token = new_token();
    let now = now_secs() as i64;
    let id = sqlx::query(
        "INSERT INTO refresh_tokens (user_id, token_hash, family_id, expires_at, created_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(user_id)
    .bind(hash_token(&token))
    .bind(family)
    .bind(now + ttl_secs as i64)
    .bind(now)
    .execute(pool)
    .await?
    .last_insert_rowid();
    Ok((token, id))
}

#[derive(sqlx::FromRow)]
struct TokenRow {
    id: i64,
    user_id: i64,
    family_id: String,
    expires_at: i64,
    revoked_at: Option<i64>,
}

/// Exchanges a refresh token for a new one. Returns (user_id, new_refresh_token).
/// Presenting an already-rotated token revokes the whole family (suspected theft).
pub async fn rotate(pool: &SqlitePool, token: &str, ttl_secs: u64) -> AppResult<(i64, String)> {
    let row = sqlx::query_as::<_, TokenRow>(
        "SELECT id, user_id, family_id, expires_at, revoked_at FROM refresh_tokens WHERE token_hash = ?",
    )
    .bind(hash_token(token))
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::Unauthorized)?;

    let now = now_secs() as i64;
    if row.revoked_at.is_some() {
        revoke_family(pool, &row.family_id).await?;
        return Err(AppError::Unauthorized);
    }
    if row.expires_at <= now {
        return Err(AppError::Unauthorized);
    }
    let active: Option<bool> = sqlx::query_scalar("SELECT is_active FROM users WHERE id = ?")
        .bind(row.user_id)
        .fetch_optional(pool)
        .await?;
    if active != Some(true) {
        return Err(AppError::Unauthorized);
    }

    // Atomically claim the old token so concurrent use of the same token only succeeds once.
    let claimed =
        sqlx::query("UPDATE refresh_tokens SET revoked_at = ? WHERE id = ? AND revoked_at IS NULL")
            .bind(now)
            .bind(row.id)
            .execute(pool)
            .await?
            .rows_affected();
    if claimed != 1 {
        revoke_family(pool, &row.family_id).await?;
        return Err(AppError::Unauthorized);
    }
    let (new, new_id) = insert(pool, row.user_id, &row.family_id, ttl_secs).await?;
    sqlx::query("UPDATE refresh_tokens SET replaced_by = ? WHERE id = ?")
        .bind(new_id)
        .bind(row.id)
        .execute(pool)
        .await?;
    Ok((row.user_id, new))
}

async fn revoke_family(pool: &SqlitePool, family: &str) -> AppResult<()> {
    sqlx::query(
        "UPDATE refresh_tokens SET revoked_at = ? WHERE family_id = ? AND revoked_at IS NULL",
    )
    .bind(now_secs() as i64)
    .bind(family)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn revoke_token(pool: &SqlitePool, token: &str) -> AppResult<()> {
    let family: Option<String> =
        sqlx::query_scalar("SELECT family_id FROM refresh_tokens WHERE token_hash = ?")
            .bind(hash_token(token))
            .fetch_optional(pool)
            .await?;
    if let Some(family) = family {
        revoke_family(pool, &family).await?;
    }
    Ok(())
}

pub async fn revoke_all_for_user(pool: &SqlitePool, user_id: i64) -> AppResult<()> {
    sqlx::query(
        "UPDATE refresh_tokens SET revoked_at = ? WHERE user_id = ? AND revoked_at IS NULL",
    )
    .bind(now_secs() as i64)
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(())
}
