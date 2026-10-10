use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,
    pub email: String,
    pub is_admin: bool,
    pub iat: u64,
    pub exp: u64,
    pub jti: String,
}

pub fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn issue(
    secret: &str,
    user_id: i64,
    email: &str,
    is_admin: bool,
    ttl_secs: u64,
) -> AppResult<String> {
    let now = now_secs();
    let claims = Claims {
        sub: user_id.to_string(),
        email: email.to_owned(),
        is_admin,
        iat: now,
        exp: now + ttl_secs,
        jti: uuid::Uuid::new_v4().to_string(),
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AppError::Internal(e.to_string()))
}

pub fn verify(secret: &str, token: &str) -> AppResult<Claims> {
    let mut validation = Validation::default();
    validation.leeway = 0;
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map(|d| d.claims)
    .map_err(|_| AppError::Unauthorized)
}
