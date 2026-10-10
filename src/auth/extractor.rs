use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts},
};

use super::jwt;
use crate::{AppState, error::AppError};

/// Authenticated caller, built from a valid `Authorization: Bearer <jwt>` header.
/// Admin status comes from the token, so changes take effect when the access token is refreshed.
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id: i64,
    pub email: String,
    pub is_admin: bool,
}

impl AuthUser {
    pub fn require_admin(&self) -> Result<(), AppError> {
        if self.is_admin {
            Ok(())
        } else {
            Err(AppError::Forbidden)
        }
    }
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, AppError> {
        let header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or(AppError::Unauthorized)?;
        let token = header
            .strip_prefix("Bearer ")
            .ok_or(AppError::Unauthorized)?;
        let claims = jwt::verify(&state.config.jwt_secret, token)?;
        Ok(AuthUser {
            id: claims.sub.parse().map_err(|_| AppError::Unauthorized)?,
            email: claims.email,
            is_admin: claims.is_admin,
        })
    }
}
