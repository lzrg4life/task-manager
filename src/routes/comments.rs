use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::{
    AppState,
    auth::AuthUser,
    error::{AppError, AppResult},
};

const MAX_COMMENT_LENGTH: usize = 5000;

#[derive(Debug, Serialize, FromRow)]
pub struct CommentOut {
    id: i64,
    task_id: i64,
    user_id: i64,
    author_name: String,
    author_email: String,
    body: Option<String>,
    created_at: String,
    updated_at: String,
    deleted_at: Option<String>,
}

#[derive(Deserialize)]
pub struct CommentRequest {
    body: String,
}

#[derive(FromRow)]
struct CommentOwner {
    user_id: i64,
    deleted_at: Option<String>,
}

fn validate_body(body: &str) -> AppResult<String> {
    let body = body.trim();
    let length = body.chars().count();
    if length == 0 || length > MAX_COMMENT_LENGTH {
        return Err(AppError::BadRequest(
            "comment must be 1-5000 characters".into(),
        ));
    }
    Ok(body.to_string())
}

async fn fetch_comment(
    pool: &sqlx::SqlitePool,
    task_id: i64,
    comment_id: i64,
) -> AppResult<CommentOwner> {
    sqlx::query_as::<_, CommentOwner>(
        "SELECT user_id, deleted_at FROM task_comments WHERE id = ? AND task_id = ?",
    )
    .bind(comment_id)
    .bind(task_id)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)
}

fn require_comment_permission(user: &AuthUser, owner_id: i64) -> AppResult<()> {
    if user.is_admin || user.id == owner_id {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}

pub async fn list(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(task_id): Path<i64>,
) -> AppResult<Json<Vec<CommentOut>>> {
    let task_exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM tasks WHERE id = ?)")
        .bind(task_id)
        .fetch_one(&state.pool)
        .await?;
    if !task_exists {
        return Err(AppError::NotFound);
    }

    let comments = sqlx::query_as::<_, CommentOut>(
        "SELECT c.id, c.task_id, c.user_id, \
         CASE WHEN u.display_name = '' THEN u.email ELSE u.display_name END AS author_name, \
         u.email AS author_email, \
         CASE WHEN c.deleted_at IS NULL THEN c.body ELSE NULL END AS body, \
         c.created_at, c.updated_at, c.deleted_at \
         FROM task_comments c JOIN users u ON u.id = c.user_id \
         WHERE c.task_id = ? ORDER BY c.id",
    )
    .bind(task_id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(comments))
}

pub async fn create(
    State(state): State<AppState>,
    user: AuthUser,
    Path(task_id): Path<i64>,
    Json(req): Json<CommentRequest>,
) -> AppResult<(StatusCode, Json<CommentOut>)> {
    let body = validate_body(&req.body)?;
    let task_exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM tasks WHERE id = ?)")
        .bind(task_id)
        .fetch_one(&state.pool)
        .await?;
    if !task_exists {
        return Err(AppError::NotFound);
    }

    let id = sqlx::query("INSERT INTO task_comments (task_id, user_id, body) VALUES (?, ?, ?)")
        .bind(task_id)
        .bind(user.id)
        .bind(body)
        .execute(&state.pool)
        .await?
        .last_insert_rowid();

    let comment = sqlx::query_as::<_, CommentOut>(
        "SELECT c.id, c.task_id, c.user_id, \
         CASE WHEN u.display_name = '' THEN u.email ELSE u.display_name END AS author_name, \
         u.email AS author_email, c.body, c.created_at, c.updated_at, c.deleted_at \
         FROM task_comments c JOIN users u ON u.id = c.user_id WHERE c.id = ?",
    )
    .bind(id)
    .fetch_one(&state.pool)
    .await?;
    Ok((StatusCode::CREATED, Json(comment)))
}

pub async fn update(
    State(state): State<AppState>,
    user: AuthUser,
    Path((task_id, comment_id)): Path<(i64, i64)>,
    Json(req): Json<CommentRequest>,
) -> AppResult<Json<CommentOut>> {
    let body = validate_body(&req.body)?;
    let current = fetch_comment(&state.pool, task_id, comment_id).await?;
    require_comment_permission(&user, current.user_id)?;
    if current.deleted_at.is_some() {
        return Err(AppError::Conflict(
            "deleted comments cannot be edited".into(),
        ));
    }

    let result = sqlx::query(
        "UPDATE task_comments SET body = ?, \
         updated_at = strftime('%Y-%m-%dT%H:%M:%SZ','now') \
         WHERE id = ? AND task_id = ? AND deleted_at IS NULL",
    )
    .bind(body)
    .bind(comment_id)
    .bind(task_id)
    .execute(&state.pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::Conflict(
            "deleted comments cannot be edited".into(),
        ));
    }

    let comment = sqlx::query_as::<_, CommentOut>(
        "SELECT c.id, c.task_id, c.user_id, \
         CASE WHEN u.display_name = '' THEN u.email ELSE u.display_name END AS author_name, \
         u.email AS author_email, c.body, c.created_at, c.updated_at, c.deleted_at \
         FROM task_comments c JOIN users u ON u.id = c.user_id \
         WHERE c.id = ? AND c.task_id = ?",
    )
    .bind(comment_id)
    .bind(task_id)
    .fetch_one(&state.pool)
    .await?;
    Ok(Json(comment))
}

pub async fn delete(
    State(state): State<AppState>,
    user: AuthUser,
    Path((task_id, comment_id)): Path<(i64, i64)>,
) -> AppResult<StatusCode> {
    let current = fetch_comment(&state.pool, task_id, comment_id).await?;
    require_comment_permission(&user, current.user_id)?;
    if current.deleted_at.is_some() {
        return Err(AppError::Conflict("comment is already deleted".into()));
    }

    let result = sqlx::query(
        "UPDATE task_comments SET \
         deleted_at = strftime('%Y-%m-%dT%H:%M:%SZ','now'), \
         updated_at = strftime('%Y-%m-%dT%H:%M:%SZ','now') \
         WHERE id = ? AND task_id = ? AND deleted_at IS NULL",
    )
    .bind(comment_id)
    .bind(task_id)
    .execute(&state.pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::Conflict("comment is already deleted".into()));
    }
    Ok(StatusCode::NO_CONTENT)
}
