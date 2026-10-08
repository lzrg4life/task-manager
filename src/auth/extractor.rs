use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts},
};

use super::jwt;
use crate::{AppState, error::AppError};

/// Authenticated caller, built from a valid `Authorization: Bearer <jwt>` header.
/// Roles come from the token, so role changes take effect when the access token is refreshed.
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id: i64,
    pub email: String,
    pub roles: Vec<String>,
}

impl AuthUser {
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r.eq_ignore_ascii_case(role))
    }

    pub fn require_role(&self, role: &str) -> Result<(), AppError> {
        if self.has_role(role) {
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
            roles: claims.roles,
        })
    }
}
