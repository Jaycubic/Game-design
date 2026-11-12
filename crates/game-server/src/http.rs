//! REST endpoints (all JSON). See docs/API.md.

use crate::auth::AuthPlayer;
use crate::error::{ApiError, ApiResult};
use crate::room::{region, GameError};
use crate::AppState;
use actix_web::{web, HttpRequest, HttpResponse};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

impl From<GameError> for ApiError {
    fn from(e: GameError) -> Self {
        let status = match e.code.as_str() {
            "room_not_found" | "not_found" => 404,
            "database_unavailable" => 503,
            "not_member" | "not_host" => 403,
            "room_full" | "level_in_progress" | "already_member" => 409,
            _ => 400,
        };
        ApiError::new(status, &e.code, e.message)
    }
}

/// Liveness: the process is up.
pub async fn health(state: web::Data<AppState>) -> HttpResponse {
    HttpResponse::Ok().json(json!({
        "status": "ok",
        "service": "natures-thread",
        "version": env!("CARGO_PKG_VERSION"),
        "rooms": state.hub.room_count().await,
    }))
}

/// Readiness: the database answers.
pub async fn ready(state: web::Data<AppState>) -> HttpResponse {
    match sqlx::query_scalar::<_, i32>("SELECT 1").fetch_one(&state.db).await {
        Ok(_) => HttpResponse::Ok().json(json!({ "status": "ready", "database": "ok" })),
        Err(e) => {
            tracing::warn!(error = %e, "readiness check failed");
            HttpResponse::ServiceUnavailable().json(json!({ "status": "not_ready", "database": "unavailable" }))
        }
    }
}

/// Static game content. Field Report clue texts are removed: each player only
/// ever receives their own clues over their WebSocket.
pub async fn content() -> HttpResponse {
    let r = region();
    let mut levels = serde_json::to_value(&r.levels).unwrap_or_default();
    if let Some(arr) = levels.as_array_mut() {
        for l in arr {
            if let Some(d) = l.get_mut("deduction").and_then(|d| d.as_object_mut()) {
                let n = d.get("clues").and_then(|c| c.as_array()).map(|c| c.len()).unwrap_or(0);
                d.remove("clues");
                d.insert("clue_count".into(), json!(n));
            }
        }
    }
    HttpResponse::Ok().insert_header(("Cache-Control", "public, max-age=300")).json(json!({
        "region": { "id": r.id, "name": r.name, "ecosystem": r.ecosystem, "tagline": r.tagline, "areas": r.areas, "amendments": r.amendments },
        "levels": levels,
        "journal": nt_core::content::journal(),
    }))
}

#[derive(Deserialize)]
pub struct JoinBody {
    pub code: String,
}

pub async fn create_room(state: web::Data<AppState>, player: AuthPlayer) -> ApiResult<HttpResponse> {
    let p = player.0;
    let c = &state.config;
    let room = state
        .hub
        .create(&state.db, p.id, &p.display_name, c.room_min_players, c.room_max_players, c.room_code_length)
        .await?;
    Ok(HttpResponse::Created()
        .json(json!({ "room_id": room.id, "code": room.code, "state": room.view_for(&p.id.to_string()) })))
}

pub async fn join_room(
    state: web::Data<AppState>,
    req: HttpRequest,
    player: AuthPlayer,
    body: web::Json<JoinBody>,
) -> ApiResult<HttpResponse> {
    state.limiter.check(&req, "join")?;
    let code = body.code.trim().to_ascii_uppercase();
    if code.len() < 4 || code.len() > 8 || !code.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(ApiError::bad_request("invalid_code", "Invitation codes are 4–8 letters and digits."));
    }
    let p = player.0;
    let room = state.hub.join(&state.db, &code, p.id, &p.display_name).await?;
    Ok(HttpResponse::Ok()
        .json(json!({ "room_id": room.id, "code": room.code, "state": room.view_for(&p.id.to_string()) })))
}

pub async fn get_room(
    state: web::Data<AppState>,
    player: AuthPlayer,
    path: web::Path<Uuid>,
) -> ApiResult<HttpResponse> {
    let entry = state.hub.room(*path).await.ok_or_else(|| ApiError::not_found("Room"))?;
    let g = entry.lock().await;
    let pid = player.0.id.to_string();
    if g.state.roster.member(&pid).is_none() {
        return Err(ApiError::forbidden("You are not a member of this room."));
    }
    Ok(HttpResponse::Ok().json(g.state.view_for(&pid)))
}

/// Rooms this player belongs to that are still open (for reconnecting).
pub async fn my_rooms(state: web::Data<AppState>, player: AuthPlayer) -> ApiResult<HttpResponse> {
    let rows: Vec<(Uuid, String, String)> = sqlx::query_as(
        "SELECT r.id, r.join_code, r.status FROM room_members m JOIN rooms r ON r.id = m.room_id
         WHERE m.player_id = $1 AND m.left_at IS NULL AND r.status NOT IN ('closed','expired')
         ORDER BY r.last_activity_at DESC LIMIT 10",
    )
    .bind(player.0.id)
    .fetch_all(&state.db)
    .await?;
    let rooms: Vec<_> =
        rows.into_iter().map(|(id, code, status)| json!({ "room_id": id, "code": code, "status": status })).collect();
    Ok(HttpResponse::Ok().json(json!({ "rooms": rooms })))
}

pub async fn progress(state: web::Data<AppState>, player: AuthPlayer) -> ApiResult<HttpResponse> {
    let row: Option<(i32, i32, i32, serde_json::Value)> = sqlx::query_as(
        "SELECT leaf_marks_total, levels_completed, regions_completed, best_marks FROM player_progression WHERE player_id = $1",
    )
    .bind(player.0.id)
    .fetch_optional(&state.db)
    .await?;
    let journal: Vec<(String, chrono::DateTime<chrono::Utc>)> = sqlx::query_as(
        "SELECT entry_key, discovered_at FROM field_journal_entries WHERE player_id = $1 ORDER BY discovered_at",
    )
    .bind(player.0.id)
    .fetch_all(&state.db)
    .await?;
    let (marks, levels, regions, best) = row.unwrap_or((0, 0, 0, json!({})));
    Ok(HttpResponse::Ok().json(json!({
        "player": player.0,
        "leaf_marks_total": marks,
        "levels_completed": levels,
        "regions_completed": regions,
        "best_marks": best,
        "journal": journal.into_iter().map(|(k, t)| json!({ "key": k, "discovered_at": t })).collect::<Vec<_>>(),
    })))
}

pub async fn room_results(
    state: web::Data<AppState>,
    player: AuthPlayer,
    path: web::Path<Uuid>,
) -> ApiResult<HttpResponse> {
    let room = *path;
    let member: Option<i16> =
        sqlx::query_scalar("SELECT seat FROM room_members WHERE room_id = $1 AND player_id = $2 AND left_at IS NULL")
            .bind(room)
            .bind(player.0.id)
            .fetch_optional(&state.db)
            .await?;
    if member.is_none() {
        return Err(ApiError::forbidden("You are not a member of this room."));
    }
    let rows: Vec<(i16, String, i16, i16, String, chrono::DateTime<chrono::Utc>)> = sqlx::query_as(
        "SELECT lr.level_number, lr.outcome, lr.leaf_marks, lr.spent, lr.ruleset, lr.created_at
         FROM level_results lr JOIN regional_campaigns c ON c.id = lr.campaign_id
         WHERE c.room_id = $1 ORDER BY lr.created_at",
    )
    .bind(room)
    .fetch_all(&state.db)
    .await?;
    let results: Vec<_> = rows
        .into_iter()
        .map(|(level, outcome, marks, spent, rules, at)| json!({ "level": level, "outcome": outcome, "leaf_marks": marks, "spent": spent, "rules": rules, "at": at }))
        .collect();
    Ok(HttpResponse::Ok().json(json!({ "results": results })))
}
