use std::time::Duration;

#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    pub bind_addr: String,
    pub access_ttl: Duration,
    pub refresh_ttl: Duration,
    pub static_dir: String,
}

pub fn database_url_from_env() -> String {
    std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://data/app.db".to_string())
}

fn env_secs(key: &str, default: u64) -> Result<Duration, String> {
    match std::env::var(key) {
        Ok(v) => v
            .parse::<u64>()
            .map(Duration::from_secs)
            .map_err(|_| format!("{key} must be a number of seconds")),
        Err(_) => Ok(Duration::from_secs(default)),
    }
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        let jwt_secret =
            std::env::var("JWT_SECRET").map_err(|_| "JWT_SECRET is required".to_string())?;
        if jwt_secret.len() < 32 {
            return Err("JWT_SECRET must be at least 32 characters".into());
        }
        Ok(Self {
            database_url: database_url_from_env(),
            jwt_secret,
            bind_addr: std::env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:3000".into()),
            access_ttl: env_secs("ACCESS_TOKEN_TTL_SECS", 900)?,
            refresh_ttl: env_secs("REFRESH_TOKEN_TTL_SECS", 30 * 24 * 3600)?,
            static_dir: std::env::var("STATIC_DIR").unwrap_or_else(|_| "static".into()),
        })
    }
}
