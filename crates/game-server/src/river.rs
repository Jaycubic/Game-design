//! Real-time river runs: a 30 Hz authoritative loop per room.
//!
//! Inputs arrive over the room WebSocket and are queued per seat (one applied
//! per tick, so quick double-taps are not lost). Events go out every tick;
//! per-player snapshots go out at 15 Hz (gate codes are private to crewmates).
//! A run lives in memory only: after a server restart the room returns to its lobby.
//! When a run ends, every player is paid Seeds (see `meta::award`).

use crate::hub::{broadcast, envelope, RoomEntry};
use crate::meta;
use crate::room::GameError;
use river_core::{Input, Phase as RunPhase, River, Run, TICK_HZ};
use serde_json::json;
use sqlx::PgPool;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use uuid::Uuid;

const MAX_QUEUED: usize = 8;

pub struct RunHost {
    pub run: Run,
    pub practice: bool,
    pub tutorial: bool,
    /// seat → equipped boat key (sets the 3D model and its stats).
    pub skins: HashMap<u8, String>,
    pending: HashMap<u8, VecDeque<Input>>,
}

impl RunHost {
    pub fn queue(&mut self, seat: u8, input: Input) {
        let q = self.pending.entry(seat).or_default();
        if q.len() < MAX_QUEUED {
            q.push_back(input);
        }
    }

    fn take_inputs(&mut self) -> Vec<(u8, Input)> {
        self.run
            .boats
            .iter()
            .map(|b| (b.seat, self.pending.get_mut(&b.seat).and_then(|q| q.pop_front()).unwrap_or_default()))
            .collect()
    }
}

/// Player ids of the seats that would race now (for looking up equipped boats).
pub fn racers(g: &RoomEntry) -> Vec<(u8, Uuid)> {
    g.state
        .roster
        .members
        .iter()
        .filter(|m| m.connected)
        .filter_map(|m| Uuid::parse_str(&m.player_id).ok().map(|p| (m.seat, p)))
        .collect()
}

/// Start a run in this room. `practice` lets the host play alone (half Seeds).
/// `difficulty` is the River Director's starting value for these players.
#[allow(clippy::too_many_arguments)]
pub fn start(
    entry_arc: Arc<Mutex<RoomEntry>>,
    g: &mut RoomEntry,
    player: &str,
    practice: bool,
    tutorial: bool,
    skins: HashMap<u8, String>,
    difficulty: f32,
    river: River,
    db: PgPool,
) -> Result<(), GameError> {
    if g.state.roster.host != player {
        return Err(GameError::new("not_host", "Only the host can start the run."));
    }
    if g.run.is_some() {
        return Err(GameError::new("run_in_progress", "A run is already under way."));
    }
    if !matches!(g.state.phase, crate::room::Phase::Lobby) {
        return Err(GameError::new("not_in_lobby", "Return to the lobby first."));
    }
    if !practice {
        g.state.roster.can_start(player)?;
    }
    let seats = g.state.roster.connected_seats();
    if seats.is_empty() {
        return Err(GameError::new("no_players", "Nobody is connected."));
    }
    let seed = rand::random::<u64>();
    let mut run = if tutorial { Run::tutorial(seed, &seats) } else { Run::create(seed, &seats, difficulty, river) };
    // Each boat's upgrade levels come from the boat its player has equipped.
    for (&seat, key) in &skins {
        run.equip(seat, river_core::boat_stats(key));
    }
    let boats: Vec<_> = run.boats.iter().map(|b| json!({ "seat": b.seat, "crew": b.crew })).collect();
    let names: HashMap<u8, String> = g
        .state
        .roster
        .members
        .iter()
        .map(|m| (m.seat, g.state.names.get(&m.player_id).cloned().unwrap_or_default()))
        .collect();
    let msg = envelope(
        "run.started",
        g.state.id,
        g.state.version,
        json!({ "seed": seed, "boats": boats, "names": names, "skins": skins, "practice": practice, "tutorial": tutorial, "river": run.river }),
    );
    for c in g.conns.values() {
        let _ = c.tx.send(msg.clone());
    }
    g.run = Some(RunHost { run, practice, tutorial, skins, pending: HashMap::new() });
    tokio::spawn(run_loop(entry_arc, db));
    Ok(())
}

async fn run_loop(entry: Arc<Mutex<RoomEntry>>, db: PgPool) {
    let mut tick = tokio::time::interval(Duration::from_micros(1_000_000 / TICK_HZ as u64));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        tick.tick().await;
        let mut g = entry.lock().await;
        let room = g.state.id;
        let Some(host) = g.run.as_mut() else { return };
        let inputs = host.take_inputs();
        host.run.step(&inputs);
        let events = host.run.drain_events();
        let tick_no = host.run.tick;
        let finished = host.run.phase == RunPhase::Finished;

        if !events.is_empty() {
            let msg = envelope("run.events", room, tick_no, json!({ "events": events }));
            for c in g.conns.values() {
                let _ = c.tx.send(msg.clone());
            }
        }
        if tick_no % 2 == 0 || finished {
            let host = g.run.as_ref().expect("run present");
            for c in g.conns.values() {
                let seat = g.state.roster.member(&c.player).map(|m| m.seat).unwrap_or(u8::MAX);
                let snap = host.run.snapshot_for(seat);
                let _ = c.tx.send(envelope(
                    "run.snapshot",
                    room,
                    tick_no,
                    json!({ "snapshot": snap, "you": seat, "skins": host.skins }),
                ));
            }
        }
        if finished {
            let host = g.run.take().expect("run present");
            let outcomes = outcomes(&g, &host.run);
            for m in &mut g.state.roster.members {
                m.ready = false;
            }
            g.state.version += 1;
            broadcast(&g, &[]);
            drop(g);
            if host.tutorial {
                tracing::info!(room = %room, "tutorial finished");
                return;
            }
            // Learn from the run (River Director), then pay out, after releasing the room lock.
            let seats: Vec<(u8, Uuid)> = outcomes.iter().map(|o| (o.seat, o.player)).collect();
            if let Err(e) = crate::director::record(&db, room, host.practice, &host.run, &seats).await {
                tracing::error!(error = %e, room = %room, "recording the run for the River Director failed");
            }
            let river_results = crate::rivers::record(&db, &host.run, &seats).await.unwrap_or_else(|e| {
                tracing::error!(error = %e, room = %room, "recording river stars failed");
                HashMap::new()
            });
            match meta::award(&db, room, host.practice, outcomes).await {
                Ok(paid) => {
                    let g = entry.lock().await;
                    for c in g.conns.values() {
                        let seat = g.state.roster.member(&c.player).map(|m| m.seat).unwrap_or(u8::MAX);
                        let seeds = paid.get(&seat).copied().unwrap_or(0);
                        let _ = c.tx.send(envelope(
                            "run.rewards",
                            room,
                            tick_no,
                            json!({ "seeds": seeds, "practice": host.practice, "river": river_results.get(&seat) }),
                        ));
                    }
                }
                Err(e) => tracing::error!(error = %e, room = %room, "awarding run rewards failed"),
            }
            tracing::info!(room = %room, "river run finished");
            return;
        }
    }
}

fn outcomes(g: &RoomEntry, run: &Run) -> Vec<meta::RunOutcome> {
    let Some(res) = &run.results else { return Vec::new() };
    run.boats
        .iter()
        .filter_map(|b| {
            let m = g.state.roster.members.iter().find(|m| m.seat == b.seat)?;
            let player = Uuid::parse_str(&m.player_id).ok()?;
            let score = res.scores.iter().find(|(c, _)| *c == b.crew).map(|(_, s)| *s).unwrap_or(0);
            // One crew (co-op): "winning" means keeping the river alive.
            let won = if res.scores.len() > 1 { res.winner == Some(b.crew) } else { res.revived };
            Some(meta::RunOutcome { player, seat: b.seat, score, won, revived: res.revived })
        })
        .collect()
}
