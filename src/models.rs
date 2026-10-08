use serde::Serialize;
use sqlx::{FromRow, SqlitePool};

use crate::error::{AppError, AppResult};

#[derive(Debug, Serialize, Clone)]
pub struct UserOut {
    pub id: i64,
    pub email: String,
    pub display_name: String,
    pub is_active: bool,
    pub roles: Vec<String>,
    pub created_at: String,
}

#[derive(FromRow)]
struct UserRow {
    id: i64,
    email: String,
    display_name: String,
    is_active: bool,
    created_at: String,
}

#[derive(Debug, Serialize, FromRow)]
pub struct Role {
    pub id: i64,
    pub name: String,
    pub description: String,
}

pub async fn roles_for_user(pool: &SqlitePool, user_id: i64) -> AppResult<Vec<String>> {
    let names = sqlx::query_scalar::<_, String>(
        "SELECT r.name FROM roles r JOIN user_roles ur ON ur.role_id = r.id \
         WHERE ur.user_id = ? ORDER BY r.name",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(names)
}

pub async fn fetch_user(pool: &SqlitePool, id: i64) -> AppResult<UserOut> {
    let row = sqlx::query_as::<_, UserRow>(
        "SELECT id, email, display_name, is_active, created_at FROM users WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)?;
    let roles = roles_for_user(pool, id).await?;
    Ok(UserOut {
        id: row.id,
        email: row.email,
        display_name: row.display_name,
        is_active: row.is_active,
        roles,
        created_at: row.created_at,
    })
}

pub async fn list_users(pool: &SqlitePool) -> AppResult<Vec<UserOut>> {
    let ids = sqlx::query_scalar::<_, i64>("SELECT id FROM users ORDER BY email")
        .fetch_all(pool)
        .await?;
    let mut out = Vec::with_capacity(ids.len());
    for id in ids {
        out.push(fetch_user(pool, id).await?);
    }
    Ok(out)
}

pub fn validate_email(email: &str) -> AppResult<()> {
    let ok = email.len() <= 254
        && !email.chars().any(char::is_whitespace)
        && matches!(email.split_once('@'), Some((l, d)) if !l.is_empty() && d.contains('.') && !d.starts_with('.') && !d.ends_with('.') && !d.contains('@'));
    if ok {
        Ok(())
    } else {
        Err(AppError::BadRequest("invalid email address".into()))
    }
}

pub fn validate_password(password: &str) -> AppResult<()> {
    if password.len() < 10 || password.len() > 128 {
        return Err(AppError::BadRequest(
            "password must be 10-128 characters".into(),
        ));
    }
    Ok(())
}
