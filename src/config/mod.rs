//! Application configuration loaded from environment.

use std::net::SocketAddr;

/// Application configuration loaded from `.env` and environment variables.
#[derive(Debug, Clone)]
pub struct Config {
    /// Server bind address (e.g. `0.0.0.0:3000`).
    pub server_addr: SocketAddr,
    /// Redis connection URL (e.g. `redis://127.0.0.1/`).
    pub redis_url: String,
    /// PostgreSQL connection URL.
    pub database_url: String,
    /// Secret for signing private/presence channel auth (e.g. `app_secret`).
    pub app_secret: String,
    /// Application key identifier (e.g. `app_key`) — legacy single key for broadcast.
    pub app_key: String,
    /// JWT signing secret (min 32 chars).
    pub jwt_secret: String,
    /// Log level: `error`, `warn`, `info`, `debug`, `trace`.
    pub log_level: String,
}

impl Config {
    /// Load configuration from environment. Call `dotenvy::dotenv().ok()` before this.
    pub fn from_env() -> Result<Self, ConfigLoadError> {
        let server_addr = std::env::var("SERVER_ADDR")
            .unwrap_or_else(|_| "0.0.0.0:3000".to_string());
        let server_addr: SocketAddr = server_addr
            .parse()
            .map_err(|_| ConfigLoadError::InvalidServerAddr)?;

        let redis_url =
            std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1/".to_string());
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://notif:notif@localhost:5432/notif".to_string());
        let app_secret =
            std::env::var("APP_SECRET").unwrap_or_else(|_| "notif_secret".to_string());
        let app_key = std::env::var("APP_KEY").unwrap_or_else(|_| "notif_key".to_string());
        let jwt_secret = std::env::var("JWT_SECRET")
            .unwrap_or_else(|_| "notif_jwt_secret_change_in_production_32chars".to_string());
        let log_level = std::env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string());

        Ok(Self {
            server_addr,
            redis_url,
            database_url,
            app_secret,
            app_key,
            jwt_secret,
            log_level,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigLoadError {
    #[error("Invalid SERVER_ADDR")]
    InvalidServerAddr,
}
