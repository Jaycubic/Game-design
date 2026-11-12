//! Live rooms and their WebSocket connections.
//!
//! Each room sits behind its own async mutex, so commands for one room are
//! applied strictly one at a time (deterministic resolution of simultaneous
//! actions and votes: first to acquire the lock wins; the loser's stale
//! `base_version` is rejected). A command is applied to a copy of the state,
//! persisted in one transaction, and only then made live and broadcast.

use crate::persist;
use crate::room::{level_def, Command, GameError, Notice, Outcome2, Phase, RoomState};
use chrono::Utc;
use nt_core::sim::Sim;
use nt_core::solver::{hint, Limits};
use rand::Rng;
use serde_json::json;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock};
use uuid::Uuid;

pub type Tx = mpsc::UnboundedSender<String>;

pub struct Conn {
    pub player: String,
    pub tx: Tx,
}

pub struct RoomEntry {
    pub state: RoomState,
    pub conns: HashMap<Uuid, Conn>,
    /// The live river run, if one is under way (memory only).
    pub run: Option<crate::river::RunHost>,
}

#[derive(Default)]
pub struct Hub {
    rooms: RwLock<HashMap<Uuid, Arc<Mutex<RoomEntry>>>>,
    codes: RwLock<HashMap<String, Uuid>>,
}

/// Unambiguous characters for invitation codes (no 0/O, 1/I/L).
const CODE_ALPHABET: &[u8] = b"ABCDEFGHJKMNPQRSTUVWXYZ23456789";

pub fn envelope(kind: &str, room: Uuid, seq: u64, payload: serde_json::Value) -> String {
    json!({
        "v": 1,
        "id": Uuid::new_v4(),
        "type": kind,
        "room": room,
        "seq": seq,
        "ts": Utc::now().to_rfc3339(),
        "payload": payload,
    })
    .to_string()
}

impl Hub {
    pub async fn restore(&self, rooms: Vec<RoomState>) {
        let mut map = self.rooms.write().await;
        let mut codes = self.codes.write().await;
        for r in rooms {
            codes.insert(r.code.clone(), r.id);
            map.insert(r.id, Arc::new(Mutex::new(RoomEntry { state: r, conns: HashMap::new(), run: None })));
        }
    }

    pub async fn room(&self, id: Uuid) -> Option<Arc<Mutex<RoomEntry>>> {
        self.rooms.read().await.get(&id).cloned()
    }

    pub async fn room_by_code(&self, code: &str) -> Option<Arc<Mutex<RoomEntry>>> {
        let id = *self.codes.read().await.get(&code.to_ascii_uppercase())?;
        self.room(id).await
    }

    pub async fn room_count(&self) -> usize {
        self.rooms.read().await.len()
    }

    async fn new_code(&self, len: usize) -> String {
        let codes = self.codes.read().await;
        loop {
            let code: String = {
                let mut rng = rand::thread_rng();
                (0..len).map(|_| CODE_ALPHABET[rng.gen_range(0..CODE_ALPHABET.len())] as char).collect()
            };
            if !codes.contains_key(&code) {
                return code;
            }
        }
    }

    pub async fn create(
        &self,
        db: &PgPool,
        player: Uuid,
        name: &str,
        min: usize,
        max: usize,
        code_len: usize,
    ) -> Result<RoomState, GameError> {
        for _ in 0..5 {
            let code = self.new_code(code_len).await;
            let room =
                RoomState::new(Uuid::new_v4(), code.clone(), &player.to_string(), name, min, max, Uuid::new_v4())?;
            match persist::create_room(db, &room, player).await {
                Ok(()) => {
                    self.codes.write().await.insert(code, room.id);
                    self.rooms.write().await.insert(
                        room.id,
                        Arc::new(Mutex::new(RoomEntry { state: room.clone(), conns: HashMap::new(), run: None })),
                    );
                    tracing::info!(room = %room.id, "room created");
                    return Ok(room);
                }
                // unique violation on the join code: try another code
                Err(sqlx::Error::Database(e)) if e.code().as_deref() == Some("23505") => continue,
                Err(e) => {
                    tracing::error!(error = %e, "create room failed");
                    return Err(db_error());
                }
            }
        }
        Err(GameError::new("code_exhausted", "Could not allocate an invitation code — try again."))
    }

    /// Apply a state change under the room lock: run `f` on a copy, persist, then commit.
    async fn mutate<F>(
        &self,
        db: &PgPool,
        entry: &Arc<Mutex<RoomEntry>>,
        actor: Option<Uuid>,
        f: F,
    ) -> Result<Outcome2, GameError>
    where
        F: FnOnce(&mut RoomState) -> Result<Outcome2, GameError>,
    {
        let mut guard = entry.lock().await;
        let mut next = guard.state.clone();
        let out = f(&mut next)?;
        if !out.changed {
            return Ok(out);
        }
        if let Err(e) = persist::commit(db, &next, actor, &out.notices, &out.effects).await {
            tracing::error!(error = %e, room = %next.id, "persist failed; change rejected");
            return Err(db_error());
        }
        guard.state = next;
        broadcast(&guard, &out.notices);
        Ok(out)
    }

    pub async fn join(&self, db: &PgPool, code: &str, player: Uuid, name: &str) -> Result<RoomState, GameError> {
        let entry = self
            .room_by_code(code)
            .await
            .ok_or_else(|| GameError::new("room_not_found", "No open room has that invitation code."))?;
        let pid = player.to_string();
        self.mutate(db, &entry, Some(player), |s| {
            let (_, out) = s.join(&pid, name)?;
            s.version += 1;
            Ok(out)
        })
        .await?;
        let state = entry.lock().await.state.clone();
        Ok(state)
    }

    /// Register a live connection; marks the player connected and sends the state.
    pub async fn attach(&self, db: &PgPool, room: Uuid, player: Uuid, conn: Uuid, tx: Tx) -> Result<(), GameError> {
        let entry =
            self.room(room).await.ok_or_else(|| GameError::new("room_not_found", "That room no longer exists."))?;
        let pid = player.to_string();
        {
            let mut g = entry.lock().await;
            if g.state.roster.member(&pid).is_none() {
                return Err(GameError::new("not_member", "Join the room with its invitation code first."));
            }
            g.conns.insert(conn, Conn { player: pid.clone(), tx: tx.clone() });
        }
        let was_connected = entry.lock().await.state.roster.member(&pid).is_some_and(|m| m.connected);
        if !was_connected {
            self.mutate(db, &entry, Some(player), |s| {
                let out = s.reconnect(&pid);
                s.version += 1;
                Ok(out)
            })
            .await?;
        }
        let g = entry.lock().await;
        let _ = tx.send(envelope("room.joined", room, g.state.version, json!({ "state": g.state.view_for(&pid) })));
        send_private_clues(&g, &pid, &tx);
        Ok(())
    }

    /// A connection closed. When the player has no other connection they are
    /// marked disconnected; during a vote a grace timer may remove them.
    pub async fn detach(self: &Arc<Self>, db: PgPool, room: Uuid, player: Uuid, conn: Uuid, grace_secs: u64) {
        let Some(entry) = self.room(room).await else { return };
        let pid = player.to_string();
        let still_connected = {
            let mut g = entry.lock().await;
            g.conns.remove(&conn);
            g.conns.values().any(|c| c.player == pid)
        };
        if still_connected {
            return;
        }
        let _ = self
            .mutate(&db, &entry, Some(player), |s| {
                let out = s.disconnect(&pid);
                if out.changed {
                    s.version += 1;
                }
                Ok(out)
            })
            .await;
        let hub = Arc::clone(self);
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(grace_secs)).await;
            if let Some(entry) = hub.room(room).await {
                let _ = hub
                    .mutate(&db, &entry, None, |s| {
                        let out = s.absent_voter_timeout(&pid)?;
                        if out.changed {
                            s.version += 1;
                        }
                        Ok(out)
                    })
                    .await;
            }
        });
    }

    pub async fn command(&self, db: &PgPool, room: Uuid, player: Uuid, cmd: Command) -> Result<Outcome2, GameError> {
        let entry =
            self.room(room).await.ok_or_else(|| GameError::new("room_not_found", "That room no longer exists."))?;
        let pid = player.to_string();
        if cmd == Command::Hint {
            return self.hint(&entry, &pid).await;
        }
        match cmd {
            Command::RunStart { practice } => {
                let mut g = entry.lock().await;
                crate::river::start(entry.clone(), &mut g, &pid, practice)?;
                return Ok(Outcome2::default());
            }
            Command::RunInput { steer, hop, action, glyph } => {
                let mut g = entry.lock().await;
                let seat = g.state.seat_of(&pid)?;
                let host = g.run.as_mut().ok_or_else(|| GameError::new("no_run", "No run is under way."))?;
                host.queue(seat, river_core::Input { steer: steer.clamp(-1, 1), hop, action, glyph });
                return Ok(Outcome2::default());
            }
            Command::RunChoose { option } => {
                let mut g = entry.lock().await;
                let seat = g.state.seat_of(&pid)?;
                let host = g.run.as_mut().ok_or_else(|| GameError::new("no_run", "No run is under way."))?;
                host.run.choose(seat, option).map_err(|m| GameError::new("choice_rejected", m))?;
                return Ok(Outcome2::default());
            }
            _ => {}
        }
        let leaving = cmd == Command::Leave;
        let out = self.mutate(db, &entry, Some(player), |s| s.handle(&pid, cmd)).await?;
        if leaving {
            let mut g = entry.lock().await;
            g.conns.retain(|_, c| c.player != pid);
            if g.state.roster.members.is_empty() {
                let code = g.state.code.clone();
                drop(g);
                self.rooms.write().await.remove(&room);
                self.codes.write().await.remove(&code);
            }
        }
        // Newly dealt clues go privately to their holders.
        if out.notices.iter().any(|n| matches!(n, Notice::LevelStarted { .. })) {
            let g = entry.lock().await;
            for c in g.conns.values() {
                send_private_clues(&g, &c.player, &c.tx);
            }
        }
        Ok(out)
    }

    /// Solver hint for the current level, computed off the async runtime.
    async fn hint(&self, entry: &Arc<Mutex<RoomEntry>>, pid: &str) -> Result<Outcome2, GameError> {
        let (play, room_id, version, seat) = {
            let g = entry.lock().await;
            let seat = g.state.seat_of(pid)?;
            match &g.state.phase {
                Phase::Playing { play } => (play.clone(), g.state.id, g.state.version, seat),
                _ => return Err(GameError::new("no_level", "Hints are available during a level.")),
            }
        };
        if level_def(play.level).is_some_and(|l| l.deduction.is_some()) {
            return Err(GameError::new("no_hint", "Field Report puzzles have no hints — share your clues!"));
        }
        let suggestion = tokio::task::spawn_blocking(move || {
            let def = level_def(play.level)?;
            let sim = Sim::new(def, play.rules).ok()?;
            hint(&sim, &play.state, &Limits { max_states: 400_000 })
        })
        .await
        .map_err(|_| GameError::new("internal", "The hint could not be computed."))?;
        let g = entry.lock().await;
        let payload = match suggestion {
            Some(action) => json!({ "seat": seat, "action": action }),
            None => {
                json!({ "seat": seat, "action": null, "message": "From here the objective can no longer be reached — try again or replay the level." })
            }
        };
        let msg = envelope("game.hint", room_id, version, payload);
        for c in g.conns.values() {
            let _ = c.tx.send(msg.clone());
        }
        Ok(Outcome2::default())
    }

    pub async fn expire(&self, ids: &[Uuid]) {
        let mut rooms = self.rooms.write().await;
        let mut codes = self.codes.write().await;
        for id in ids {
            if let Some(entry) = rooms.remove(id) {
                let g = entry.lock().await;
                codes.remove(&g.state.code);
                let msg = envelope(
                    "room.expired",
                    *id,
                    g.state.version,
                    json!({ "message": "This room expired after being idle." }),
                );
                for c in g.conns.values() {
                    let _ = c.tx.send(msg.clone());
                }
            }
        }
    }
}

fn db_error() -> GameError {
    GameError::new(
        "database_unavailable",
        "The game database is unavailable, so that change was not saved. Check PostgreSQL and try again.",
    )
}

fn send_private_clues(g: &RoomEntry, pid: &str, tx: &Tx) {
    let view = g.state.view_for(pid);
    if let Some(clues) = view["play"]["my_clues"].as_array() {
        if !clues.is_empty() {
            let _ = tx.send(envelope(
                "player.clue_assigned",
                g.state.id,
                g.state.version,
                json!({ "level": view["play"]["level"], "clues": clues }),
            ));
        }
    }
}

/// Send each connection its own view (private clues filtered per player).
pub(crate) fn broadcast(g: &RoomEntry, notices: &[Notice]) {
    let kind = notices
        .last()
        .and_then(|n| serde_json::to_value(n).ok())
        .and_then(|v| v.get("event").and_then(|e| e.as_str()).map(String::from))
        .unwrap_or_else(|| "game.state_updated".into());
    let mut cache: HashMap<&str, String> = HashMap::new();
    for c in g.conns.values() {
        let msg = cache.entry(c.player.as_str()).or_insert_with(|| {
            envelope(
                &kind,
                g.state.id,
                g.state.version,
                json!({ "events": notices, "state": g.state.view_for(&c.player) }),
            )
        });
        let _ = c.tx.send(msg.clone());
    }
}
