use axum::{Json, extract::State, http::StatusCode};
use serde::{Deserialize, Serialize};

use crate::{
    AppState,
    auth::{AuthUser, jwt, password, refresh},
    error::{AppError, AppResult},
    models::{self, UserOut},
};

#[derive(Deserialize)]
pub struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Deserialize)]
pub struct RefreshRequest {
    refresh_token: String,
}

#[derive(Serialize)]
pub struct TokenResponse {
    access_token: String,
    refresh_token: String,
    token_type: &'static str,
    expires_in: u64,
    user: UserOut,
}

async fn token_response(
    state: &AppState,
    user_id: i64,
    refresh_token: String,
) -> AppResult<TokenResponse> {
    let user = models::fetch_user(&state.pool, user_id).await?;
    let ttl = state.config.access_ttl.as_secs();
    let access_token = jwt::issue(
        &state.config.jwt_secret,
        user.id,
        &user.email,
        user.roles.clone(),
        ttl,
    )?;
    Ok(TokenResponse {
        access_token,
        refresh_token,
        token_type: "Bearer",
        expires_in: ttl,
        user,
    })
}

pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> AppResult<Json<TokenResponse>> {
    let row: Option<(i64, String, bool)> =
        sqlx::query_as("SELECT id, password_hash, is_active FROM users WHERE email = ?")
            .bind(req.email.trim())
            .fetch_optional(&state.pool)
            .await?;
    let hash = row.as_ref().map(|r| r.1.as_str());
    let valid = password::verify(&req.password, hash).await;
    match row {
        Some((id, _, true)) if valid => {
            let token = refresh::issue(&state.pool, id, state.config.refresh_ttl.as_secs()).await?;
            Ok(Json(token_response(&state, id, token).await?))
        }
        _ => Err(AppError::Unauthorized),
    }
}

pub async fn refresh(
    State(state): State<AppState>,
    Json(req): Json<RefreshRequest>,
) -> AppResult<Json<TokenResponse>> {
    let (user_id, token) = refresh::rotate(
        &state.pool,
        &req.refresh_token,
        state.config.refresh_ttl.as_secs(),
    )
    .await?;
    Ok(Json(token_response(&state, user_id, token).await?))
}

pub async fn logout(
    State(state): State<AppState>,
    Json(req): Json<RefreshRequest>,
) -> AppResult<StatusCode> {
    refresh::revoke_token(&state.pool, &req.refresh_token).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn me(State(state): State<AppState>, user: AuthUser) -> AppResult<Json<UserOut>> {
    let out = models::fetch_user(&state.pool, user.id)
        .await
        .map_err(|e| match e {
            AppError::NotFound => AppError::Unauthorized,
            other => other,
        })?;
    Ok(Json(out))
}
