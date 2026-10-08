use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde::Deserialize;
use sqlx::{Sqlite, SqlitePool, Transaction};

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
    roles: Vec<String>,
}

#[derive(Deserialize)]
pub struct UpdateUser {
    email: Option<String>,
    display_name: Option<String>,
    password: Option<String>,
    is_active: Option<bool>,
    roles: Option<Vec<String>>,
}

pub async fn list(State(state): State<AppState>, user: AuthUser) -> AppResult<Json<Vec<UserOut>>> {
    user.require_role("admin")?;
    Ok(Json(models::list_users(&state.pool).await?))
}

pub async fn get_one(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<i64>,
) -> AppResult<Json<UserOut>> {
    user.require_role("admin")?;
    Ok(Json(models::fetch_user(&state.pool, id).await?))
}

async fn set_roles(
    tx: &mut Transaction<'_, Sqlite>,
    user_id: i64,
    names: &[String],
) -> AppResult<()> {
    if names.is_empty() {
        return Err(AppError::BadRequest(
            "a user needs at least one role".into(),
        ));
    }
    sqlx::query("DELETE FROM user_roles WHERE user_id = ?")
        .bind(user_id)
        .execute(&mut **tx)
        .await?;
    for name in names {
        let role_id: Option<i64> = sqlx::query_scalar("SELECT id FROM roles WHERE name = ?")
            .bind(name.trim())
            .fetch_optional(&mut **tx)
            .await?;
        let role_id =
            role_id.ok_or_else(|| AppError::BadRequest(format!("unknown role: {name}")))?;
        sqlx::query("INSERT OR IGNORE INTO user_roles (user_id, role_id) VALUES (?, ?)")
            .bind(user_id)
            .bind(role_id)
            .execute(&mut **tx)
            .await?;
    }
    Ok(())
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
    user.require_role("admin")?;
    let email = req.email.trim().to_string();
    validate_email(&email)?;
    validate_password(&req.password)?;
    let hash = password::hash(&req.password).await?;
    let mut tx = state.pool.begin().await?;
    let id = sqlx::query("INSERT INTO users (email, password_hash, display_name) VALUES (?, ?, ?)")
        .bind(&email)
        .bind(hash)
        .bind(req.display_name.trim())
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
    set_roles(&mut tx, id, &req.roles).await?;
    tx.commit().await?;
    Ok((
        StatusCode::CREATED,
        Json(models::fetch_user(&state.pool, id).await?),
    ))
}

async fn other_active_admins(pool: &SqlitePool, excluding: i64) -> AppResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT COUNT(*) FROM users u JOIN user_roles ur ON ur.user_id = u.id JOIN roles r ON r.id = ur.role_id \
         WHERE r.name = 'admin' AND u.is_active = 1 AND u.id != ?",
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
    user.require_role("admin")?;
    let current = models::fetch_user(&state.pool, id).await?;

    let will_be_active = req.is_active.unwrap_or(current.is_active);
    let will_be_admin = match &req.roles {
        Some(r) => r.iter().any(|n| n.trim().eq_ignore_ascii_case("admin")),
        None => current.roles.iter().any(|n| n == "admin"),
    };
    let is_active_admin = current.is_active && current.roles.iter().any(|n| n == "admin");
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
         password_hash = COALESCE(?, password_hash), is_active = ?, \
         updated_at = strftime('%Y-%m-%dT%H:%M:%SZ','now') WHERE id = ?",
    )
    .bind(req.email.as_deref().map(str::trim))
    .bind(req.display_name.as_deref().map(str::trim))
    .bind(new_hash.as_deref())
    .bind(will_be_active)
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
    if let Some(roles) = &req.roles {
        set_roles(&mut tx, id, roles).await?;
    }
    tx.commit().await?;

    if !will_be_active || req.password.is_some() {
        refresh::revoke_all_for_user(&state.pool, id).await?;
    }
    Ok(Json(models::fetch_user(&state.pool, id).await?))
}
