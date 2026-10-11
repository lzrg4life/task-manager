use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde::Deserialize;

use crate::{
    AppState,
    auth::AuthUser,
    error::{AppError, AppResult},
    models::{self, TagOut},
};

const MAX_NAME: usize = 40;
const MAX_DESCRIPTION: usize = 500;

#[derive(Deserialize)]
pub struct NewTag {
    name: String,
    #[serde(default)]
    description: String,
}

#[derive(Deserialize)]
pub struct TagQuery {
    include_archived: Option<bool>,
}

pub async fn list(
    State(state): State<AppState>,
    _user: AuthUser,
    Query(query): Query<TagQuery>,
) -> AppResult<Json<Vec<TagOut>>> {
    let include_archived = query.include_archived.unwrap_or(false);
    Ok(Json(
        models::list_tags(&state.pool, include_archived).await?,
    ))
}

pub async fn create(
    State(state): State<AppState>,
    user: AuthUser,
    Json(req): Json<NewTag>,
) -> AppResult<(StatusCode, Json<TagOut>)> {
    user.require_admin()?;
    let name = req.name.trim();
    let description = req.description.trim();
    if name.is_empty() || name.chars().count() > MAX_NAME {
        return Err(AppError::BadRequest(format!(
            "tag name must be 1-{MAX_NAME} characters"
        )));
    }
    if description.chars().count() > MAX_DESCRIPTION {
        return Err(AppError::BadRequest(format!(
            "tag description must be at most {MAX_DESCRIPTION} characters"
        )));
    }

    let id = sqlx::query("INSERT INTO tags (name, description) VALUES (?, ?)")
        .bind(name)
        .bind(description)
        .execute(&state.pool)
        .await
        .map_err(|e| {
            if e.as_database_error()
                .map(|db| db.is_unique_violation())
                .unwrap_or(false)
            {
                AppError::Conflict("tag name already exists".into())
            } else {
                e.into()
            }
        })?
        .last_insert_rowid();

    Ok((
        StatusCode::CREATED,
        Json(TagOut {
            id,
            name: name.to_string(),
            description: description.to_string(),
            archived: false,
        }),
    ))
}

pub async fn archive(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<i64>,
) -> AppResult<Json<TagOut>> {
    user.require_admin()?;
    let tag = sqlx::query_as::<_, TagOut>(
        "UPDATE tags SET archived = 1 WHERE id = ? \
         RETURNING id, name, description, archived",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AppError::NotFound)?;
    Ok(Json(tag))
}
