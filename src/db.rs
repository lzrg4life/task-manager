use std::str::FromStr;

use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
};

use crate::{
    auth::password,
    error::{AppError, AppResult},
    models::{validate_email, validate_password},
};

pub async fn connect(url: &str) -> Result<SqlitePool, sqlx::Error> {
    let mut opts = SqliteConnectOptions::from_str(url)?
        .create_if_missing(true)
        .foreign_keys(true);
    let in_memory = url.contains(":memory:") || url.contains("mode=memory");
    if !in_memory {
        opts = opts.journal_mode(SqliteJournalMode::Wal);
        if let Some(parent) = opts.get_filename().parent()
            && !parent.as_os_str().is_empty()
        {
            let _ = std::fs::create_dir_all(parent);
        }
    }
    SqlitePoolOptions::new()
        .max_connections(if in_memory { 1 } else { 5 })
        .connect_with(opts)
        .await
}

pub async fn migrate(pool: &SqlitePool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("./migrations").run(pool).await
}

/// Creates the first admin user. Fails if any user already exists.
pub async fn create_first_admin(
    pool: &SqlitePool,
    email: &str,
    password_plain: &str,
) -> AppResult<i64> {
    let email = email.trim();
    validate_email(email)?;
    validate_password(password_plain)?;
    let existing: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await?;
    if existing > 0 {
        return Err(AppError::Conflict(
            "database already has users; init-db refused".into(),
        ));
    }
    let hash = password::hash(password_plain).await?;
    let mut tx = pool.begin().await?;
    let id = sqlx::query(
        "INSERT INTO users (email, password_hash, display_name) VALUES (?, ?, 'Administrator')",
    )
    .bind(email)
    .bind(hash)
    .execute(&mut *tx)
    .await?
    .last_insert_rowid();
    sqlx::query(
        "INSERT INTO user_roles (user_id, role_id) SELECT ?, id FROM roles WHERE name = 'admin'",
    )
    .bind(id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(id)
}
