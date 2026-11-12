//! Accounts and sessions: Argon2id password hashes, HS256 tokens whose `jti`
//! is a revocable row in `player_sessions`. Tokens are never stored or logged.

use crate::error::{ApiError, ApiResult};
use crate::AppState;
use actix_web::{dev::Payload, web, FromRequest, HttpRequest, HttpResponse};
use argon2::password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub jti: Uuid,
    pub exp: i64,
    pub iat: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlayerInfo {
    pub id: Uuid,
    pub handle: String,
    pub display_name: String,
}

/// An authenticated player, extracted from `Authorization: Bearer <token>`.
#[derive(Debug, Clone)]
pub struct AuthPlayer(pub PlayerInfo);

pub fn hash_password(password: &str) -> anyhow::Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| anyhow::anyhow!("hash failed: {e}"))
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    PasswordHash::new(hash).is_ok_and(|h| Argon2::default().verify_password(password.as_bytes(), &h).is_ok())
}

pub fn issue_token(
    secret: &str,
    player: Uuid,
    session: Uuid,
    hours: i64,
) -> anyhow::Result<(String, chrono::DateTime<Utc>)> {
    let now = Utc::now();
    let exp = now + Duration::hours(hours);
    let claims = Claims { sub: player, jti: session, exp: exp.timestamp(), iat: now.timestamp() };
    let token = encode(&Header::new(Algorithm::HS256), &claims, &EncodingKey::from_secret(secret.as_bytes()))?;
    Ok((token, exp))
}

pub fn decode_token(secret: &str, token: &str) -> Option<Claims> {
    let mut v = Validation::new(Algorithm::HS256);
    v.leeway = 5;
    decode::<Claims>(token, &DecodingKey::from_secret(secret.as_bytes()), &v).ok().map(|d| d.claims)
}

/// Verify a token and its session row; returns the player.
pub async fn authenticate(state: &AppState, token: &str) -> ApiResult<PlayerInfo> {
    let claims = decode_token(&state.config.jwt_secret, token).ok_or_else(ApiError::unauthorized)?;
    let row: Option<(Uuid, String, String)> = sqlx::query_as(
        "SELECT p.id, p.handle, p.display_name FROM player_sessions s JOIN players p ON p.id = s.player_id
         WHERE s.id = $1 AND s.player_id = $2 AND s.revoked_at IS NULL AND s.expires_at > now()",
    )
    .bind(claims.jti)
    .bind(claims.sub)
    .fetch_optional(&state.db)
    .await?;
    let (id, handle, display_name) = row.ok_or_else(ApiError::unauthorized)?;
    Ok(PlayerInfo { id, handle, display_name })
}

fn bearer(req: &HttpRequest) -> Option<String> {
    req.headers().get("authorization")?.to_str().ok()?.strip_prefix("Bearer ").map(|t| t.trim().to_string())
}

impl FromRequest for AuthPlayer {
    type Error = ApiError;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let state = req.app_data::<web::Data<AppState>>().cloned();
        let token = bearer(req);
        Box::pin(async move {
            let state = state.ok_or_else(|| ApiError::Internal(anyhow::anyhow!("state missing")))?;
            let token = token.ok_or_else(ApiError::unauthorized)?;
            authenticate(&state, &token).await.map(AuthPlayer)
        })
    }
}

#[derive(Deserialize)]
pub struct RegisterBody {
    pub handle: String,
    pub display_name: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct LoginBody {
    pub handle: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub expires_at: chrono::DateTime<Utc>,
    pub player: PlayerInfo,
}

fn normalise_handle(h: &str) -> ApiResult<String> {
    let h = h.trim().to_lowercase();
    let ok = (3..=24).contains(&h.len()) && h.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
    if ok {
        Ok(h)
    } else {
        Err(ApiError::bad_request("invalid_handle", "Handles are 3–24 letters, digits or underscores."))
    }
}

async fn start_session(state: &AppState, player: PlayerInfo) -> ApiResult<AuthResponse> {
    let session = Uuid::new_v4();
    let (token, expires_at) =
        issue_token(&state.config.jwt_secret, player.id, session, state.config.jwt_expires_hours)?;
    sqlx::query("INSERT INTO player_sessions (id, player_id, expires_at) VALUES ($1, $2, $3)")
        .bind(session)
        .bind(player.id)
        .bind(expires_at)
        .execute(&state.db)
        .await?;
    Ok(AuthResponse { token, expires_at, player })
}

pub async fn register(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<RegisterBody>,
) -> ApiResult<HttpResponse> {
    state.limiter.check(&req, "auth")?;
    let handle = normalise_handle(&body.handle)?;
    let display_name = body.display_name.trim().to_string();
    if display_name.is_empty() || display_name.chars().count() > 32 {
        return Err(ApiError::bad_request("invalid_name", "Display names are 1–32 characters."));
    }
    if body.password.chars().count() < 8 || body.password.len() > 128 {
        return Err(ApiError::bad_request("weak_password", "Passwords need at least 8 characters."));
    }
    let password = body.password.clone();
    let hash = web::block(move || hash_password(&password)).await.map_err(|e| anyhow::anyhow!("{e}"))??;
    let id: Option<Uuid> = sqlx::query_scalar(
        "INSERT INTO players (handle, display_name, password_hash) VALUES ($1, $2, $3)
         ON CONFLICT (handle) DO NOTHING RETURNING id",
    )
    .bind(&handle)
    .bind(&display_name)
    .bind(&hash)
    .fetch_optional(&state.db)
    .await?;
    let id = id.ok_or_else(|| ApiError::conflict("handle_taken", "That handle is already taken."))?;
    sqlx::query("INSERT INTO player_progression (player_id) VALUES ($1) ON CONFLICT DO NOTHING")
        .bind(id)
        .execute(&state.db)
        .await?;
    tracing::info!(player = %id, "player registered");
    Ok(HttpResponse::Created().json(start_session(&state, PlayerInfo { id, handle, display_name }).await?))
}

pub async fn login(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<LoginBody>,
) -> ApiResult<HttpResponse> {
    state.limiter.check(&req, "auth")?;
    let handle = body.handle.trim().to_lowercase();
    let row: Option<(Uuid, String, String)> =
        sqlx::query_as("SELECT id, display_name, password_hash FROM players WHERE handle = $1")
            .bind(&handle)
            .fetch_optional(&state.db)
            .await?;
    let password = body.password.clone();
    // Verify even for unknown handles so timing does not reveal which handles exist.
    static DUMMY_HASH: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    let (id, display_name, hash) = row.unwrap_or_else(|| {
        let dummy = DUMMY_HASH.get_or_init(|| hash_password("timing-equaliser").unwrap_or_default());
        (Uuid::nil(), String::new(), dummy.clone())
    });
    let ok = web::block(move || verify_password(&password, &hash)).await.map_err(|e| anyhow::anyhow!("{e}"))?;
    if !ok || id.is_nil() {
        return Err(ApiError::new(401, "invalid_credentials", "Handle or password is incorrect."));
    }
    Ok(HttpResponse::Ok().json(start_session(&state, PlayerInfo { id, handle, display_name }).await?))
}

pub async fn me(player: AuthPlayer) -> ApiResult<HttpResponse> {
    Ok(HttpResponse::Ok().json(player.0))
}

pub async fn logout(state: web::Data<AppState>, req: HttpRequest) -> ApiResult<HttpResponse> {
    let token = bearer(&req).ok_or_else(ApiError::unauthorized)?;
    let claims = decode_token(&state.config.jwt_secret, &token).ok_or_else(ApiError::unauthorized)?;
    sqlx::query("UPDATE player_sessions SET revoked_at = now() WHERE id = $1 AND revoked_at IS NULL")
        .bind(claims.jti)
        .execute(&state.db)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_round_trip() {
        let h = hash_password("correct horse").unwrap();
        assert!(h.starts_with("$argon2id$"));
        assert!(verify_password("correct horse", &h));
        assert!(!verify_password("wrong", &h));
    }

    #[test]
    fn tokens_verify_and_reject_tampering() {
        let secret = "x".repeat(40);
        let (t, _) = issue_token(&secret, Uuid::new_v4(), Uuid::new_v4(), 1).unwrap();
        assert!(decode_token(&secret, &t).is_some());
        assert!(decode_token(&"y".repeat(40), &t).is_none());
        let mut bad = t.clone();
        bad.push('x');
        assert!(decode_token(&secret, &bad).is_none());
    }
}
