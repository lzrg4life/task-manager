use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};

use crate::error::{AppError, AppResult};

pub async fn hash(password: &str) -> AppResult<String> {
    let password = password.to_owned();
    tokio::task::spawn_blocking(move || {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map(|h| h.to_string())
            .map_err(|e| AppError::Internal(e.to_string()))
    })
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?
}

/// Verifies a password; `None` hash means the user does not exist and a dummy hash is checked
/// so response timing does not reveal which emails are registered.
pub async fn verify(password: &str, hash: Option<&str>) -> bool {
    static DUMMY: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    let password = password.to_owned();
    let stored = hash.map(str::to_owned);
    tokio::task::spawn_blocking(move || {
        let real = stored.is_some();
        let text = stored.as_deref().unwrap_or_else(|| {
            DUMMY.get_or_init(|| {
                let salt = SaltString::generate(&mut OsRng);
                Argon2::default()
                    .hash_password(b"dummy-password", &salt)
                    .map(|h| h.to_string())
                    .unwrap_or_default()
            })
        });
        let Ok(parsed) = PasswordHash::new(text) else {
            return false;
        };
        let ok = Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok();
        real && ok
    })
    .await
    .unwrap_or(false)
}
