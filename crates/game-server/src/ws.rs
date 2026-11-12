//! WebSocket endpoint: `GET /ws?room=<uuid>`.
//!
//! Protocol (JSON text frames, envelope `{ v, id, type, payload }`):
//! 1. The client's first message must be `{"type":"auth","payload":{"token":"…"}}`
//!    within 10 seconds (tokens never appear in URLs or logs).
//! 2. The server replies `room.joined` with the player's view of the room.
//! 3. The client sends commands (`room.ready`, `level.start`, `player.action`,
//!    `turn.pass`, `charter.vote`, `level.continue`, `level.replay`,
//!    `hint.request`, `room.leave`, `ping`); the server broadcasts state updates.
//!
//! Errors are `{"type":"error","payload":{"code","message","ref"}}` and never
//! close the connection unless authentication fails.

use crate::auth::authenticate;
use crate::hub::envelope;
use crate::room::Command;
use crate::AppState;
use actix_web::{web, HttpRequest, HttpResponse};
use actix_ws::{AggregatedMessage, CloseCode, CloseReason};
use futures_util::StreamExt;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::VecDeque;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use uuid::Uuid;

const AUTH_TIMEOUT: Duration = Duration::from_secs(10);
const HEARTBEAT: Duration = Duration::from_secs(15);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(45);
const MAX_MESSAGE_BYTES: usize = 32 * 1024;
const DEDUP_WINDOW: usize = 128;

#[derive(Deserialize)]
pub struct WsQuery {
    room: Uuid,
}

#[derive(Deserialize)]
struct Incoming {
    #[serde(default)]
    id: Option<String>,
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    payload: Value,
}

fn error_msg(room: Uuid, code: &str, message: &str, reference: Option<&str>) -> String {
    envelope("error", room, 0, json!({ "code": code, "message": message, "ref": reference }))
}

pub async fn ws_handler(
    req: HttpRequest,
    body: web::Payload,
    query: web::Query<WsQuery>,
    state: web::Data<AppState>,
) -> actix_web::Result<HttpResponse> {
    let (response, mut session, stream) = actix_ws::handle(&req, body)?;
    let mut stream =
        stream.max_frame_size(MAX_MESSAGE_BYTES).aggregate_continuations().max_continuation_size(MAX_MESSAGE_BYTES);
    let room = query.room;

    actix_web::rt::spawn(async move {
        // ── 1. authenticate ────────────────────────────────────────────────
        let first = tokio::time::timeout(AUTH_TIMEOUT, stream.next()).await;
        let token = match first {
            Ok(Some(Ok(AggregatedMessage::Text(t)))) => serde_json::from_str::<Incoming>(&t)
                .ok()
                .filter(|m| m.kind == "auth")
                .and_then(|m| m.payload.get("token").and_then(|v| v.as_str()).map(String::from)),
            _ => None,
        };
        let player = match token {
            Some(t) => authenticate(&state, &t).await.ok(),
            None => None,
        };
        let Some(player) = player else {
            let _ = session.text(error_msg(room, "unauthorized", "Please sign in again.", None)).await;
            let _ = session
                .close(Some(CloseReason { code: CloseCode::Policy, description: Some("unauthorized".into()) }))
                .await;
            return;
        };

        // ── 2. attach to the room ──────────────────────────────────────────
        let conn_id = Uuid::new_v4();
        let (tx, mut rx) = mpsc::unbounded_channel::<String>();
        if let Err(e) = state.hub.attach(&state.db, room, player.id, conn_id, tx.clone()).await {
            let _ = session.text(error_msg(room, &e.code, &e.message, None)).await;
            let _ = session.close(Some(CloseReason { code: CloseCode::Normal, description: Some(e.code) })).await;
            return;
        }
        tracing::info!(room = %room, player = %player.id, "socket attached");

        // ── 3. message loop ────────────────────────────────────────────────
        let mut last_seen = Instant::now();
        let mut heartbeat = tokio::time::interval(HEARTBEAT);
        let mut recent: VecDeque<String> = VecDeque::with_capacity(DEDUP_WINDOW);
        loop {
            tokio::select! {
                out = rx.recv() => {
                    let Some(text) = out else { break };
                    if session.text(text).await.is_err() { break; }
                }
                _ = heartbeat.tick() => {
                    if last_seen.elapsed() > CLIENT_TIMEOUT { break; }
                    if session.ping(b"").await.is_err() { break; }
                }
                msg = stream.next() => {
                    let Some(Ok(msg)) = msg else { break };
                    last_seen = Instant::now();
                    match msg {
                        AggregatedMessage::Text(text) => {
                            let reply = handle_text(&state, room, player.id, &text, &mut recent).await;
                            if let Some(r) = reply {
                                if session.text(r).await.is_err() { break; }
                            }
                        }
                        AggregatedMessage::Ping(b) => { if session.pong(&b).await.is_err() { break; } }
                        AggregatedMessage::Pong(_) => {}
                        AggregatedMessage::Binary(_) => {
                            let _ = session.text(error_msg(room, "bad_message", "Send JSON text messages.", None)).await;
                        }
                        AggregatedMessage::Close(_) => break,
                    }
                }
            }
        }
        let _ = session.close(None).await;
        state.hub.detach(state.db.clone(), room, player.id, conn_id, state.config.reconnect_grace_seconds).await;
        tracing::info!(room = %room, player = %player.id, "socket closed");
    });

    Ok(response)
}

async fn handle_text(
    state: &AppState,
    room: Uuid,
    player: Uuid,
    text: &str,
    recent: &mut VecDeque<String>,
) -> Option<String> {
    let Ok(msg) = serde_json::from_str::<Incoming>(text) else {
        return Some(error_msg(room, "bad_message", "That message was not valid JSON.", None));
    };
    if let Some(id) = &msg.id {
        if recent.contains(id) {
            return Some(envelope("ack", room, 0, json!({ "ref": id, "duplicate": true })));
        }
        if recent.len() == DEDUP_WINDOW {
            recent.pop_front();
        }
        recent.push_back(id.clone());
    }
    // Commands are the payload object plus its type tag.
    let mut obj = match msg.payload {
        Value::Object(m) => m,
        Value::Null => serde_json::Map::new(),
        _ => return Some(error_msg(room, "bad_message", "The payload must be an object.", msg.id.as_deref())),
    };
    obj.insert("type".into(), Value::String(msg.kind.clone()));
    let cmd: Command = match serde_json::from_value(Value::Object(obj)) {
        Ok(c) => c,
        Err(_) => {
            return Some(error_msg(
                room,
                "unknown_command",
                &format!("'{}' is not a command this server understands, or its fields are wrong.", msg.kind),
                msg.id.as_deref(),
            ))
        }
    };
    let is_ping = cmd == Command::Ping;
    match state.hub.command(&state.db, room, player, cmd).await {
        Ok(_) if is_ping => Some(envelope("pong", room, 0, json!({ "ref": msg.id }))),
        Ok(_) => msg.id.map(|id| envelope("ack", room, 0, json!({ "ref": id }))),
        Err(e) => {
            // A stale client gets the latest state straight away.
            if e.code == "stale_state" {
                if let Some(entry) = state.hub.room(room).await {
                    let g = entry.lock().await;
                    let view = g.state.view_for(&player.to_string());
                    let latest =
                        envelope("game.state_updated", room, g.state.version, json!({ "events": [], "state": view }));
                    if let Some(c) = g.conns.values().find(|c| c.player == player.to_string()) {
                        let _ = c.tx.send(latest);
                    }
                }
            }
            Some(error_msg(room, &e.code, &e.message, msg.id.as_deref()))
        }
    }
}
