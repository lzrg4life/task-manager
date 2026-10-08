use axum::{Json, extract::State, http::StatusCode};
use serde::Deserialize;

use crate::{
    AppState,
    auth::AuthUser,
    error::{AppError, AppResult},
    models::Role,
};

#[derive(Deserialize)]
pub struct NewRole {
    name: String,
    #[serde(default)]
    description: String,
}

pub async fn list(State(state): State<AppState>, user: AuthUser) -> AppResult<Json<Vec<Role>>> {
    user.require_role("admin")?;
    let roles = sqlx::query_as::<_, Role>("SELECT id, name, description FROM roles ORDER BY name")
        .fetch_all(&state.pool)
        .await?;
    Ok(Json(roles))
}

pub async fn create(
    State(state): State<AppState>,
    user: AuthUser,
    Json(req): Json<NewRole>,
) -> AppResult<(StatusCode, Json<Role>)> {
    user.require_role("admin")?;
    let name = req.name.trim().to_lowercase();
    let valid = !name.is_empty()
        && name.len() <= 32
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-');
    if !valid {
        return Err(AppError::BadRequest(
            "role name must be 1-32 chars of a-z, 0-9, _ or -".into(),
        ));
    }
    let exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM roles WHERE name = ?")
        .bind(&name)
        .fetch_one(&state.pool)
        .await?;
    if exists > 0 {
        return Err(AppError::Conflict("role already exists".into()));
    }
    let id = sqlx::query("INSERT INTO roles (name, description) VALUES (?, ?)")
        .bind(&name)
        .bind(req.description.trim())
        .execute(&state.pool)
        .await?
        .last_insert_rowid();
    Ok((
        StatusCode::CREATED,
        Json(Role {
            id,
            name,
            description: req.description.trim().to_string(),
        }),
    ))
}
