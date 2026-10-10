use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde::Deserialize;
use sqlx::SqlitePool;

use crate::{
    AppState,
    auth::{AuthUser, password, refresh},
    error::{AppError, AppResult},
    models::{self, UserOut, validate_email, validate_password},
};

#[derive(Deserialize)]
pub struct NewUser {
    email: String,
    password: String,
    #[serde(default)]
    display_name: String,
    #[serde(default)]
    is_admin: bool,
}

#[derive(Deserialize)]
pub struct UpdateUser {
    email: Option<String>,
    display_name: Option<String>,
    password: Option<String>,
    is_active: Option<bool>,
    is_admin: Option<bool>,
}

pub async fn list(State(state): State<AppState>, user: AuthUser) -> AppResult<Json<Vec<UserOut>>> {
    user.require_admin()?;
    Ok(Json(models::list_users(&state.pool).await?))
}

pub async fn get_one(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<i64>,
) -> AppResult<Json<UserOut>> {
    user.require_admin()?;
    Ok(Json(models::fetch_user(&state.pool, id).await?))
}

fn is_unique_violation(e: &sqlx::Error) -> bool {
    e.as_database_error()
        .map(|d| d.is_unique_violation())
        .unwrap_or(false)
}

pub async fn create(
    State(state): State<AppState>,
    user: AuthUser,
    Json(req): Json<NewUser>,
) -> AppResult<(StatusCode, Json<UserOut>)> {
    user.require_admin()?;
    let email = req.email.trim().to_string();
    validate_email(&email)?;
    validate_password(&req.password)?;
    let hash = password::hash(&req.password).await?;
    let mut tx = state.pool.begin().await?;
    let id = sqlx::query(
        "INSERT INTO users (email, password_hash, display_name, is_admin) VALUES (?, ?, ?, ?)",
    )
    .bind(&email)
    .bind(hash)
    .bind(req.display_name.trim())
    .bind(req.is_admin)
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        if is_unique_violation(&e) {
            AppError::Conflict("email already in use".into())
        } else {
            e.into()
        }
    })?
    .last_insert_rowid();
    tx.commit().await?;
    Ok((
        StatusCode::CREATED,
        Json(models::fetch_user(&state.pool, id).await?),
    ))
}

async fn other_active_admins(pool: &SqlitePool, excluding: i64) -> AppResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT COUNT(*) FROM users WHERE is_admin = 1 AND is_active = 1 AND id != ?",
    )
    .bind(excluding)
    .fetch_one(pool)
    .await?)
}

pub async fn update(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<UpdateUser>,
) -> AppResult<Json<UserOut>> {
    user.require_admin()?;
    let current = models::fetch_user(&state.pool, id).await?;

    let will_be_active = req.is_active.unwrap_or(current.is_active);
    let will_be_admin = req.is_admin.unwrap_or(current.is_admin);
    let is_active_admin = current.is_active && current.is_admin;
    if is_active_admin
        && !(will_be_active && will_be_admin)
        && other_active_admins(&state.pool, id).await? == 0
    {
        return Err(AppError::Conflict(
            "cannot remove the last active admin".into(),
        ));
    }

    let new_hash = match &req.password {
        Some(p) => {
            validate_password(p)?;
            Some(password::hash(p).await?)
        }
        None => None,
    };
    if let Some(email) = &req.email {
        validate_email(email.trim())?;
    }

    let mut tx = state.pool.begin().await?;
    sqlx::query(
        "UPDATE users SET email = COALESCE(?, email), display_name = COALESCE(?, display_name), \
         password_hash = COALESCE(?, password_hash), is_active = ?, is_admin = ?, \
         updated_at = strftime('%Y-%m-%dT%H:%M:%SZ','now') WHERE id = ?",
    )
    .bind(req.email.as_deref().map(str::trim))
    .bind(req.display_name.as_deref().map(str::trim))
    .bind(new_hash.as_deref())
    .bind(will_be_active)
    .bind(will_be_admin)
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        if is_unique_violation(&e) {
            AppError::Conflict("email already in use".into())
        } else {
            e.into()
        }
    })?;
    tx.commit().await?;

    if !will_be_active || req.password.is_some() {
        refresh::revoke_all_for_user(&state.pool, id).await?;
    }
    Ok(Json(models::fetch_user(&state.pool, id).await?))
}
