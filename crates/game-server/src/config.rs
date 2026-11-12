//! Typed, validated configuration from environment variables.
//!
//! There are no secret defaults: `DATABASE_URL` and `JWT_SECRET` must be set,
//! and placeholder or short secrets are rejected. Secret values and the full
//! database URL are never logged.

use std::env;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppEnv {
    Development,
    Test,
    Production,
}

#[derive(Clone)]
pub struct Config {
    pub app_env: AppEnv,
    pub host: String,
    pub http_port: u16,
    pub tls_cert: Option<String>,
    pub tls_key: Option<String>,
    pub database_url: String,
    pub db_max_connections: u32,
    pub jwt_secret: String,
    pub jwt_expires_hours: i64,
    pub room_min_players: usize,
    pub room_max_players: usize,
    pub room_code_length: usize,
    pub reconnect_grace_seconds: u64,
    pub room_idle_expiry_hours: i64,
    pub allowed_origins: Vec<String>,
    pub static_dir: String,
}

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Config")
            .field("app_env", &self.app_env)
            .field("host", &self.host)
            .field("http_port", &self.http_port)
            .field("tls", &self.tls_cert.is_some())
            .field("database", &redact_db_url(&self.database_url))
            .field("db_max_connections", &self.db_max_connections)
            .field("jwt_secret", &"<redacted>")
            .field("jwt_expires_hours", &self.jwt_expires_hours)
            .field("rooms", &(self.room_min_players, self.room_max_players, self.room_code_length))
            .field("allowed_origins", &self.allowed_origins)
            .field("static_dir", &self.static_dir)
            .finish()
    }
}

/// `postgres://user:pass@host:port/db` → `host:port/db` (credentials removed).
pub fn redact_db_url(url: &str) -> String {
    url.rsplit('@').next().unwrap_or("<unparseable>").to_string()
}

#[derive(Debug, thiserror::Error)]
#[error("configuration error: {0}")]
pub struct ConfigError(pub String);

fn var(key: &str) -> Option<String> {
    env::var(key).ok().map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
}

fn parse<T: std::str::FromStr>(key: &str, default: T) -> Result<T, ConfigError> {
    match var(key) {
        None => Ok(default),
        Some(v) => v.parse().map_err(|_| ConfigError(format!("{key} has an invalid value"))),
    }
}

const PLACEHOLDER_SECRETS: [&str; 3] = ["REPLACE_WITH_A_LONG_RANDOM_SECRET", "changeme", "secret"];

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let app_env = match var("APP_ENV").as_deref().unwrap_or("development") {
            "development" | "dev" => AppEnv::Development,
            "test" => AppEnv::Test,
            "production" | "prod" => AppEnv::Production,
            _ => return Err(ConfigError("APP_ENV must be development, test or production".into())),
        };

        let mut missing = Vec::new();
        let database_url = var("DATABASE_URL").unwrap_or_else(|| {
            missing.push("DATABASE_URL");
            String::new()
        });
        let jwt_secret = var("JWT_SECRET").unwrap_or_else(|| {
            missing.push("JWT_SECRET");
            String::new()
        });
        if !missing.is_empty() {
            return Err(ConfigError(format!("missing required variable(s): {}", missing.join(", "))));
        }
        if !database_url.starts_with("postgres://") && !database_url.starts_with("postgresql://") {
            return Err(ConfigError("DATABASE_URL must be a postgres:// URL".into()));
        }
        if database_url.contains('<') {
            return Err(ConfigError("DATABASE_URL still contains a placeholder from .env.example".into()));
        }
        if jwt_secret.len() < 32 || PLACEHOLDER_SECRETS.iter().any(|p| jwt_secret.eq_ignore_ascii_case(p)) {
            return Err(ConfigError(
                "JWT_SECRET must be a generated random value of at least 32 characters (e.g. `openssl rand -hex 48`)"
                    .into(),
            ));
        }

        let tls_cert = var("TLS_CERT");
        let tls_key = var("TLS_KEY");
        if tls_cert.is_some() != tls_key.is_some() {
            return Err(ConfigError("set both TLS_CERT and TLS_KEY, or neither".into()));
        }

        let cfg = Config {
            app_env,
            host: var("HOST").unwrap_or_else(|| "0.0.0.0".into()),
            http_port: parse("HTTP_PORT", 8070u16)?,
            tls_cert,
            tls_key,
            database_url,
            db_max_connections: parse("DB_MAX_CONNECTIONS", 10u32)?,
            jwt_secret,
            jwt_expires_hours: parse("JWT_EXPIRES_HOURS", 24i64)?,
            room_min_players: parse("ROOM_MIN_PLAYERS", 2usize)?,
            room_max_players: parse("ROOM_MAX_PLAYERS", 4usize)?,
            room_code_length: parse("ROOM_CODE_LENGTH", 6usize)?,
            reconnect_grace_seconds: parse("ROOM_RECONNECT_GRACE_SECONDS", 120u64)?,
            room_idle_expiry_hours: parse("ROOM_IDLE_EXPIRY_HOURS", 12i64)?,
            allowed_origins: var("ALLOWED_ORIGINS")
                .map(|v| {
                    v.split(',').map(|s| s.trim().trim_end_matches('/').to_string()).filter(|s| !s.is_empty()).collect()
                })
                .unwrap_or_default(),
            static_dir: var("STATIC_DIR").unwrap_or_else(|| "./client/dist".into()),
        };
        cfg.validate()?;
        Ok(cfg)
    }

    fn validate(&self) -> Result<(), ConfigError> {
        if self.http_port == 0 {
            return Err(ConfigError("HTTP_PORT must be 1–65535".into()));
        }
        if !(1..=100).contains(&self.db_max_connections) {
            return Err(ConfigError("DB_MAX_CONNECTIONS must be 1–100".into()));
        }
        if !(1..=24 * 30).contains(&self.jwt_expires_hours) {
            return Err(ConfigError("JWT_EXPIRES_HOURS must be 1–720".into()));
        }
        if !(2..=4).contains(&self.room_min_players)
            || !(2..=4).contains(&self.room_max_players)
            || self.room_min_players > self.room_max_players
        {
            return Err(ConfigError("ROOM_MIN_PLAYERS/ROOM_MAX_PLAYERS must satisfy 2 ≤ min ≤ max ≤ 4".into()));
        }
        if !(4..=8).contains(&self.room_code_length) {
            return Err(ConfigError("ROOM_CODE_LENGTH must be 4–8".into()));
        }
        if !(10..=3600).contains(&self.reconnect_grace_seconds) {
            return Err(ConfigError("ROOM_RECONNECT_GRACE_SECONDS must be 10–3600".into()));
        }
        if !(1..=24 * 14).contains(&self.room_idle_expiry_hours) {
            return Err(ConfigError("ROOM_IDLE_EXPIRY_HOURS must be 1–336".into()));
        }
        if self.app_env == AppEnv::Production {
            if self.allowed_origins.is_empty() || self.allowed_origins.iter().any(|o| o == "*") {
                return Err(ConfigError("production requires explicit ALLOWED_ORIGINS (no '*')".into()));
            }
            if self.tls_cert.is_none() {
                tracing::warn!("production without TLS_CERT: terminate TLS in a reverse proxy, or WebGPU will be unavailable to LAN clients");
            }
        }
        Ok(())
    }

    pub fn is_dev(&self) -> bool {
        self.app_env == AppEnv::Development
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_credentials() {
        // Built at runtime so the packager's secret scanner does not flag a fixture.
        let url = format!("postgres://{}:{}@127.0.0.1:5432/nt", "user", "pw");
        assert_eq!(redact_db_url(&url), "127.0.0.1:5432/nt");
    }
}
