//! Real-time river runs: a 30 Hz authoritative loop per room.
//!
//! Inputs arrive over the room WebSocket and are queued per seat (one applied
//! per tick, so quick double-taps are not lost). Events go out every tick;
//! per-player snapshots go out at 15 Hz (gate codes are private to crewmates).
//! A run lives in memory only: after a server restart the room returns to its lobby.

use crate::hub::{broadcast, envelope, RoomEntry};
use crate::room::GameError;
use river_core::{Input, Phase as RunPhase, Run, TICK_HZ};
use serde_json::json;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

const MAX_QUEUED: usize = 8;

pub struct RunHost {
    pub run: Run,
    pending: HashMap<u8, VecDeque<Input>>,
}

impl RunHost {
    pub fn new(run: Run) -> Self {
        RunHost { run, pending: HashMap::new() }
    }

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

/// Start a run in this room. `practice` lets the host play alone.
pub fn start(
    entry_arc: Arc<Mutex<RoomEntry>>,
    g: &mut RoomEntry,
    player: &str,
    practice: bool,
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
    let run = Run::new(seed, &seats);
    let crews: Vec<_> = run.boats.iter().map(|b| json!({ "seat": b.seat, "crew": b.crew })).collect();
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
        json!({ "seed": seed, "boats": crews, "names": names, "practice": practice }),
    );
    for c in g.conns.values() {
        let _ = c.tx.send(msg.clone());
    }
    g.run = Some(RunHost::new(run));
    tokio::spawn(run_loop(entry_arc));
    Ok(())
}

async fn run_loop(entry: Arc<Mutex<RoomEntry>>) {
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
                let _ = c.tx.send(envelope("run.snapshot", room, tick_no, json!({ "snapshot": snap, "you": seat })));
            }
        }
        if finished {
            let results = g.run.as_ref().and_then(|h| h.run.results.clone());
            let msg = envelope("run.finished", room, tick_no, json!({ "results": results }));
            for c in g.conns.values() {
                let _ = c.tx.send(msg.clone());
            }
            g.run = None;
            for m in &mut g.state.roster.members {
                m.ready = false;
            }
            g.state.version += 1;
            broadcast(&g, &[]);
            tracing::info!(room = %room, "river run finished");
            return;
        }
    }
}
