//! Authoritative room state machine. Pure logic (no I/O): the hub feeds it
//! commands, persists the returned effects and broadcasts the new state.

use nt_core::campaign::{AmendmentChoice, CampaignPhase, CampaignState, LevelCompletion, VoteOutcome};
use nt_core::content::{forest_realm, RegionDef};
use nt_core::level::{LevelDef, LevelKind, Mode};
use nt_core::room::{RoomError, Roster};
use nt_core::rules::{Connectivity, Recovery, Ruleset};
use nt_core::sim::{Action, LeafMarks, Outcome, RoundReport, Sim, State, Status};
use nt_core::Terrain;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::OnceLock;
use uuid::Uuid;

pub fn region() -> &'static RegionDef {
    static REGION: OnceLock<RegionDef> = OnceLock::new();
    REGION.get_or_init(forest_realm)
}

pub fn level_def(n: u8) -> Option<&'static LevelDef> {
    region().levels.iter().find(|l| l.number == n)
}

/// Rules for a level: amendments only apply to levels after their checkpoint,
/// so replaying an early level uses the rules it was designed for.
pub fn rules_for_level(c: &CampaignState, level: u8) -> Ruleset {
    let connectivity = if level > nt_core::campaign::AMENDMENT_1_AFTER {
        c.connectivity.unwrap_or(Connectivity::Base)
    } else {
        Connectivity::Base
    };
    let recovery = if level > nt_core::campaign::AMENDMENT_2_AFTER {
        c.recovery.unwrap_or(Recovery::Base)
    } else {
        Recovery::Base
    };
    Ruleset::new(connectivity, recovery)
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Play {
    pub level: u8,
    pub rules: Ruleset,
    pub state: State,
    /// Expedition only: whose turn it is.
    pub turn_seat: Option<u8>,
    /// Seats that passed since the last action this round.
    pub passed: Vec<u8>,
    /// Deduction: clue indices dealt to each seat.
    pub clue_hands: BTreeMap<u8, Vec<usize>>,
    pub last_report: Option<RoundReport>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LevelResult {
    pub level: u8,
    pub won: bool,
    pub marks: LeafMarks,
    pub spent: u16,
    pub par: Option<u16>,
    pub reason: Option<String>,
    pub rules: String,
    pub journal: Vec<String>,
    pub insight: Option<String>,
    pub restored_cells: u16,
    pub final_board: nt_core::Board,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "phase", rename_all = "snake_case")]
pub enum Phase {
    Lobby,
    Playing { play: Box<Play> },
    Result { result: Box<LevelResult> },
    Vote,
    Complete,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RoomState {
    pub id: Uuid,
    pub code: String,
    pub roster: Roster,
    /// player id → display name
    pub names: BTreeMap<String, String>,
    pub campaign_id: Uuid,
    pub campaign: CampaignState,
    pub phase: Phase,
    /// Increases with every accepted change; clients send it back with actions.
    pub version: u64,
}

/// Client commands (the `type` field of the WebSocket envelope).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum Command {
    #[serde(rename = "room.ready")]
    Ready { ready: bool },
    #[serde(rename = "level.start")]
    StartLevel,
    #[serde(rename = "level.replay")]
    Replay { level: u8 },
    #[serde(rename = "player.action")]
    Act { action: Action, base_version: u64 },
    #[serde(rename = "turn.pass")]
    Pass { base_version: u64 },
    #[serde(rename = "charter.vote")]
    Vote { choice: AmendmentChoice },
    #[serde(rename = "level.continue")]
    Continue,
    #[serde(rename = "hint.request")]
    Hint,
    #[serde(rename = "room.leave")]
    Leave,
    #[serde(rename = "ping")]
    Ping,
    /// River run: host starts a run (practice = allowed alone).
    #[serde(rename = "run.start")]
    RunStart {
        #[serde(default)]
        practice: bool,
        #[serde(default)]
        tutorial: bool,
        /// River key ("canal", "mangrove", "gorge"); the host must have it unlocked.
        #[serde(default)]
        river: Option<String>,
        /// Solo Endless: one capsize ends the run (pays like practice).
        #[serde(default)]
        endless: bool,
    },
    /// River run: one player's input (applied on the next tick).
    #[serde(rename = "run.input")]
    RunInput {
        #[serde(default)]
        steer: i8,
        #[serde(default)]
        hop: bool,
        #[serde(default)]
        action: bool,
        #[serde(default)]
        glyph: Option<u8>,
    },
    /// River run: Twist vote or Finale pick.
    #[serde(rename = "run.choose")]
    RunChoose { option: u8 },
    /// River run: pause or resume. Only a solo run really stops; in a crew run the river keeps flowing.
    #[serde(rename = "run.pause")]
    RunPause {
        #[serde(default)]
        paused: bool,
    },
}

/// What happened, for clients to animate (the envelope `type` uses these names).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "event")]
pub enum Notice {
    #[serde(rename = "room.member_joined")]
    MemberJoined { seat: u8, name: String },
    #[serde(rename = "room.member_left")]
    MemberLeft { seat: u8 },
    #[serde(rename = "room.member_disconnected")]
    MemberDisconnected { seat: u8 },
    #[serde(rename = "room.member_reconnected")]
    MemberReconnected { seat: u8 },
    #[serde(rename = "room.ready_changed")]
    ReadyChanged { seat: u8, ready: bool },
    #[serde(rename = "level.started")]
    LevelStarted { level: u8, rules: String },
    #[serde(rename = "player.action_submitted")]
    ActionApplied { seat: u8, action: Action, cost: u8 },
    #[serde(rename = "turn.passed")]
    Passed { seat: u8 },
    #[serde(rename = "environment.event_revealed")]
    EventRevealed { round: u8, titles: Vec<String> },
    #[serde(rename = "game.round_resolved")]
    RoundResolved { report: RoundReport },
    #[serde(rename = "game.check_failed")]
    CheckFailed { report: RoundReport, seat: u8 },
    #[serde(rename = "game.answered")]
    Answered { seat: u8, correct: bool },
    #[serde(rename = "level.completed")]
    LevelCompleted { level: u8, marks: u8 },
    #[serde(rename = "level.failed")]
    LevelFailed { level: u8, reason: String },
    #[serde(rename = "charter.vote_opened")]
    VoteOpened { amendment: u8 },
    #[serde(rename = "charter.vote_submitted")]
    VoteSubmitted { seat: u8 },
    #[serde(rename = "charter.revote")]
    Revote,
    #[serde(rename = "charter.tie_break")]
    TieBreak { seat: u8 },
    #[serde(rename = "charter.amendment_applied")]
    AmendmentApplied { amendment: u8, choice: AmendmentChoice, rule: String },
    #[serde(rename = "campaign.progress_updated")]
    CampaignProgress { level: Option<u8> },
    #[serde(rename = "campaign.region_complete")]
    RegionComplete,
}

/// Side effects the hub must persist (inside one transaction).
#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    LevelResult { result: LevelResult, player_ids: Vec<String> },
    Amendment { number: u8, choice: Option<AmendmentChoice>, rule: Option<String> },
    VoteProgress { number: u8 },
    Clues { level: u8, hands: Vec<(String, usize, String)> },
    RoomStatus(&'static str),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct GameError {
    pub code: String,
    pub message: String,
}

impl GameError {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        GameError { code: code.into(), message: message.into() }
    }
}

impl From<RoomError> for GameError {
    fn from(e: RoomError) -> Self {
        let code = serde_json::to_value(&e)
            .ok()
            .and_then(|v| v.get("code").and_then(|c| c.as_str()).map(String::from))
            .unwrap_or_else(|| "room_error".into());
        GameError::new(&code, e.message())
    }
}

impl From<nt_core::sim::RuleError> for GameError {
    fn from(e: nt_core::sim::RuleError) -> Self {
        let code = serde_json::to_value(&e)
            .ok()
            .and_then(|v| v.get("code").and_then(|c| c.as_str()).map(String::from))
            .unwrap_or_else(|| "rule_error".into());
        GameError::new(&code, e.message())
    }
}

impl From<nt_core::campaign::CampaignError> for GameError {
    fn from(e: nt_core::campaign::CampaignError) -> Self {
        let code = serde_json::to_value(&e)
            .ok()
            .and_then(|v| v.get("code").and_then(|c| c.as_str()).map(String::from))
            .unwrap_or_else(|| "campaign_error".into());
        GameError::new(&code, e.message())
    }
}

#[derive(Debug, Default)]
pub struct Outcome2 {
    pub notices: Vec<Notice>,
    pub effects: Vec<Effect>,
    /// False for no-op commands (ping): no version bump, no broadcast.
    pub changed: bool,
}

impl Outcome2 {
    fn push(&mut self, n: Notice) {
        self.notices.push(n);
        self.changed = true;
    }
}

impl RoomState {
    pub fn new(
        id: Uuid,
        code: String,
        host: &str,
        host_name: &str,
        min: usize,
        max: usize,
        campaign_id: Uuid,
    ) -> Result<Self, GameError> {
        let roster = Roster::new(host, min, max)?;
        Ok(RoomState {
            id,
            code,
            roster,
            names: BTreeMap::from([(host.to_string(), host_name.to_string())]),
            campaign_id,
            campaign: CampaignState::new(&region().id, 0),
            phase: Phase::Lobby,
            version: 1,
        })
    }

    pub fn seat_of(&self, player: &str) -> Result<u8, GameError> {
        self.roster
            .member(player)
            .map(|m| m.seat)
            .ok_or_else(|| GameError::new("not_member", "You are not in this room."))
    }

    fn require_host(&self, player: &str) -> Result<(), GameError> {
        if self.roster.host == player {
            Ok(())
        } else {
            Err(GameError::new("not_host", "Only the host can do that."))
        }
    }

    pub fn status_label(&self) -> &'static str {
        match &self.phase {
            Phase::Lobby => "waiting",
            Phase::Playing { .. } => "in_progress",
            Phase::Result { result } => {
                if result.won {
                    "level_complete"
                } else {
                    "level_failed"
                }
            }
            Phase::Vote => "charter_vote",
            Phase::Complete => "region_complete",
        }
    }

    // ── Membership ──────────────────────────────────────────────────────────

    pub fn join(&mut self, player: &str, name: &str) -> Result<(u8, Outcome2), GameError> {
        let existed = self.roster.member(player).is_some();
        if !existed && !matches!(self.phase, Phase::Lobby) {
            // Late joiners may enter between levels; never mid-level.
            if matches!(self.phase, Phase::Playing { .. }) {
                return Err(GameError::new(
                    "level_in_progress",
                    "This room is in the middle of a level — join after it ends.",
                ));
            }
        }
        let seat = self.roster.join(player)?;
        self.names.insert(player.to_string(), name.to_string());
        let mut out = Outcome2::default();
        if existed {
            out.push(Notice::MemberReconnected { seat });
        } else {
            out.push(Notice::MemberJoined { seat, name: name.to_string() });
        }
        Ok((seat, out))
    }

    pub fn disconnect(&mut self, player: &str) -> Outcome2 {
        let mut out = Outcome2::default();
        let Ok(seat) = self.roster.set_connected(player, false) else { return out };
        out.push(Notice::MemberDisconnected { seat });
        self.after_presence_change(&mut out);
        out
    }

    pub fn reconnect(&mut self, player: &str) -> Outcome2 {
        let mut out = Outcome2::default();
        if let Ok(seat) = self.roster.set_connected(player, true) {
            out.push(Notice::MemberReconnected { seat });
            if let Phase::Playing { play } = &mut self.phase {
                if play.turn_seat.is_none() && level_def(play.level).is_some_and(|l| l.mode == Mode::Expedition) {
                    play.turn_seat = Some(seat);
                }
            }
        }
        out
    }

    /// Vote grace expired for a still-absent player: drop them from the vote.
    pub fn absent_voter_timeout(&mut self, player: &str) -> Result<Outcome2, GameError> {
        let mut out = Outcome2::default();
        let Some(m) = self.roster.member(player) else { return Ok(out) };
        if m.connected || !matches!(self.phase, Phase::Vote) {
            return Ok(out);
        }
        let seat = m.seat;
        let o = self.campaign.voter_left(seat)?;
        self.handle_vote_outcome(o, &mut out)?;
        Ok(out)
    }

    fn after_presence_change(&mut self, out: &mut Outcome2) {
        let connected = self.roster.connected_seats();
        if let Phase::Playing { play } = &mut self.phase {
            if let Some(t) = play.turn_seat {
                if !connected.contains(&t) {
                    play.turn_seat = self.roster.next_turn(t).filter(|s| connected.contains(s));
                }
            }
            play.passed.retain(|s| connected.contains(s));
        }
        let _ = out;
    }

    pub fn leave(&mut self, player: &str) -> Result<Outcome2, GameError> {
        let seat = self.seat_of(player)?;
        self.roster.leave(player)?;
        let mut out = Outcome2::default();
        out.push(Notice::MemberLeft { seat });
        self.after_presence_change(&mut out);
        if matches!(self.phase, Phase::Vote) {
            let o = self.campaign.voter_left(seat)?;
            self.handle_vote_outcome(o, &mut out)?;
        }
        if self.roster.members.is_empty() {
            out.effects.push(Effect::RoomStatus("closed"));
        }
        Ok(out)
    }

    // ── Commands ───────────────────────────────────────────────────────────

    pub fn handle(&mut self, player: &str, cmd: Command) -> Result<Outcome2, GameError> {
        let seat = self.seat_of(player)?;
        let mut out = Outcome2::default();
        match cmd {
            // River-run commands are handled by the hub's real-time loop.
            Command::Ping | Command::RunStart { .. } | Command::RunInput { .. } | Command::RunChoose { .. } | Command::RunPause { .. } => {}
            Command::Hint => {} // computed by the hub off the async runtime
            Command::Leave => return self.leave(player),
            Command::Ready { ready } => {
                if !matches!(self.phase, Phase::Lobby) {
                    return Err(GameError::new("not_in_lobby", "Ready only applies in the lobby."));
                }
                self.roster.set_ready(player, ready)?;
                out.push(Notice::ReadyChanged { seat, ready });
            }
            Command::StartLevel => {
                self.require_host(player)?;
                if !matches!(self.phase, Phase::Lobby) {
                    return Err(GameError::new("not_in_lobby", "The level has already started."));
                }
                self.roster.can_start(player)?;
                let level = self.campaign.current_level().ok_or_else(|| match self.campaign.phase {
                    CampaignPhase::Complete => GameError::new("campaign_complete", "This region is complete."),
                    _ => GameError::new("vote_pending", "Finish the Living Charter vote first."),
                })?;
                self.start_level(level, &mut out)?;
            }
            Command::Replay { level } => {
                self.require_host(player)?;
                if matches!(self.phase, Phase::Playing { .. } | Phase::Vote) {
                    return Err(GameError::new("busy", "Finish the current level or vote first."));
                }
                let current = self.campaign.current_level().unwrap_or(nt_core::campaign::LEVELS_PER_REGION);
                if level == 0 || level > current {
                    return Err(GameError::new("level_locked", "That level is not unlocked yet."));
                }
                self.start_level(level, &mut out)?;
            }
            Command::Act { action, base_version } => {
                self.check_version(base_version)?;
                self.act(seat, action, &mut out)?;
            }
            Command::Pass { base_version } => {
                self.check_version(base_version)?;
                self.pass(seat, &mut out)?;
            }
            Command::Vote { choice } => {
                if !matches!(self.phase, Phase::Vote) {
                    return Err(GameError::new("no_vote_open", "There is no Living Charter vote open."));
                }
                let o = self.campaign.vote(seat, choice)?;
                out.push(Notice::VoteSubmitted { seat });
                self.handle_vote_outcome(o, &mut out)?;
            }
            Command::Continue => {
                self.require_host(player)?;
                let Phase::Result { result } = &self.phase else {
                    return Err(GameError::new("nothing_to_continue", "There is no finished level to continue from."));
                };
                let (won, level) = (result.won, result.level);
                if !won {
                    self.start_level(level, &mut out)?;
                } else {
                    match &self.campaign.phase {
                        CampaignPhase::CharterVote { vote } => {
                            out.push(Notice::VoteOpened { amendment: vote.amendment });
                            out.effects.push(Effect::Amendment { number: vote.amendment, choice: None, rule: None });
                            self.phase = Phase::Vote;
                        }
                        CampaignPhase::Complete => {
                            self.phase = Phase::Complete;
                            out.push(Notice::RegionComplete);
                            out.effects.push(Effect::RoomStatus("region_complete"));
                        }
                        CampaignPhase::Level { number } => {
                            let n = *number;
                            self.start_level(n, &mut out)?;
                        }
                    }
                }
            }
        }
        if out.changed {
            self.version += 1;
        }
        Ok(out)
    }

    fn check_version(&self, base: u64) -> Result<(), GameError> {
        if base != self.version {
            return Err(GameError::new(
                "stale_state",
                format!(
                    "Your view was out of date (version {base}, now {}). The latest state has been sent.",
                    self.version
                ),
            ));
        }
        Ok(())
    }

    fn start_level(&mut self, level: u8, out: &mut Outcome2) -> Result<(), GameError> {
        let def = level_def(level).ok_or_else(|| GameError::new("unknown_level", "That level does not exist."))?;
        let rules = rules_for_level(&self.campaign, level);
        let sim = Sim::new(def, rules).map_err(|e| GameError::new("bad_level", e))?;
        let state = sim.initial_state();
        let connected = self.roster.connected_seats();
        let mut clue_hands = BTreeMap::new();
        if let Some(d) = &def.deduction {
            let hands = d.deal(connected.len().max(1));
            let mut persisted = Vec::new();
            for (seat, hand) in connected.iter().zip(hands) {
                if let Some(m) = self.roster.members.iter().find(|m| m.seat == *seat) {
                    for &k in &hand {
                        persisted.push((m.player_id.clone(), k, d.clues[k].text.clone()));
                    }
                }
                clue_hands.insert(*seat, hand);
            }
            out.effects.push(Effect::Clues { level, hands: persisted });
        }
        let turn_seat = (def.mode == Mode::Expedition).then(|| connected.first().copied()).flatten();
        out.push(Notice::LevelStarted { level, rules: rules.key() });
        let titles: Vec<String> = sim.events_in(1).map(|e| e.title.clone()).collect();
        if !titles.is_empty() {
            out.push(Notice::EventRevealed { round: 1, titles });
        }
        self.phase = Phase::Playing {
            play: Box::new(Play { level, rules, state, turn_seat, passed: Vec::new(), clue_hands, last_report: None }),
        };
        out.effects.push(Effect::RoomStatus("in_progress"));
        Ok(())
    }

    fn act(&mut self, seat: u8, action: Action, out: &mut Outcome2) -> Result<(), GameError> {
        let Phase::Playing { play } = &mut self.phase else {
            return Err(GameError::new("no_level", "No level is in progress."));
        };
        let def = level_def(play.level).expect("level exists");
        let sim = Sim::new(def, play.rules).map_err(|e| GameError::new("bad_level", e))?;
        let expedition = def.mode == Mode::Expedition;
        if expedition {
            if play.turn_seat != Some(seat) {
                return Err(GameError::new(
                    "not_your_turn",
                    "Wait for your turn — the highlighted Guardian is acting.",
                ));
            }
            if matches!(action, Action::EndRound) {
                return Err(GameError::new(
                    "use_pass",
                    "Pass your turn instead; the round ends when everyone has passed.",
                ));
            }
        }
        let outcome = sim.apply(&mut play.state, &action)?;
        let round_before = play.state.round;
        match outcome {
            Outcome::Applied { cost } => {
                out.push(Notice::ActionApplied { seat, action, cost });
                if expedition {
                    play.passed.clear();
                    let next = self.roster.next_turn(seat);
                    play.turn_seat = next;
                    if play.state.capacity_left == 0 {
                        Self::end_round(&sim, play, out);
                    }
                }
            }
            Outcome::Reverted { .. } | Outcome::Placed => out.push(Notice::ActionApplied { seat, action, cost: 0 }),
            Outcome::CheckFailed { report } => out.push(Notice::CheckFailed { report, seat }),
            Outcome::RoundResolved { report } => {
                play.last_report = Some(report.clone());
                out.push(Notice::RoundResolved { report });
            }
            Outcome::Answered { correct } => out.push(Notice::Answered { seat, correct }),
        }
        let _ = round_before;
        self.settle(out)
    }

    fn pass(&mut self, seat: u8, out: &mut Outcome2) -> Result<(), GameError> {
        let Phase::Playing { play } = &mut self.phase else {
            return Err(GameError::new("no_level", "No level is in progress."));
        };
        let def = level_def(play.level).expect("level exists");
        if def.mode != Mode::Expedition {
            return Err(GameError::new("not_expedition", "Passing only applies to Expedition turns."));
        }
        if play.turn_seat != Some(seat) {
            return Err(GameError::new("not_your_turn", "It is not your turn."));
        }
        let sim = Sim::new(def, play.rules).map_err(|e| GameError::new("bad_level", e))?;
        if !play.passed.contains(&seat) {
            play.passed.push(seat);
        }
        out.push(Notice::Passed { seat });
        let connected = self.roster.connected_seats();
        if connected.iter().all(|s| play.passed.contains(s)) {
            Self::end_round(&sim, play, out);
        } else {
            play.turn_seat = self.roster.next_turn(seat);
        }
        self.settle(out)
    }

    fn end_round(sim: &Sim, play: &mut Play, out: &mut Outcome2) {
        let Ok(Outcome::RoundResolved { report }) = sim.apply(&mut play.state, &Action::EndRound) else { return };
        play.last_report = Some(report.clone());
        play.passed.clear();
        out.push(Notice::RoundResolved { report });
        if play.state.status == Status::InProgress {
            let titles: Vec<String> = sim.events_in(play.state.round).map(|e| e.title.clone()).collect();
            if !titles.is_empty() {
                out.push(Notice::EventRevealed { round: play.state.round, titles });
            }
        }
    }

    /// If the level ended, record the result and move to the Result phase.
    fn settle(&mut self, out: &mut Outcome2) -> Result<(), GameError> {
        let Phase::Playing { play } = &self.phase else { return Ok(()) };
        let status = play.state.status.clone();
        if status == Status::InProgress {
            return Ok(());
        }
        let def = level_def(play.level).expect("level exists");
        let sim = Sim::new(def, play.rules).map_err(|e| GameError::new("bad_level", e))?;
        let marks = sim.leaf_marks(&play.state);
        let initial = sim.initial_board();
        let restored_cells = initial
            .tiles
            .iter()
            .zip(&play.state.board.tiles)
            .filter(|(a, b)| a.terrain != Terrain::Forest && b.terrain == Terrain::Forest)
            .count() as u16;
        let won = matches!(status, Status::Won { .. });
        let reason = match &status {
            Status::Lost { reason } => Some(reason.clone()),
            _ => None,
        };
        let result = LevelResult {
            level: play.level,
            won,
            marks,
            spent: play.state.spent,
            par: def.par_for(&play.rules),
            reason: reason.clone(),
            rules: play.rules.key(),
            journal: if won { def.reward.journal.clone() } else { vec![] },
            insight: if won { def.reward.insight.clone() } else { None },
            restored_cells,
            final_board: play.state.board.clone(),
        };
        if won {
            let voters = self.roster.connected_seats();
            self.campaign.complete_level(
                LevelCompletion {
                    number: play.level,
                    leaf_marks: marks.count(),
                    spent: play.state.spent,
                    restored_cells,
                    insight: result.insight.clone(),
                },
                voters,
            )?;
            out.push(Notice::LevelCompleted { level: play.level, marks: marks.count() });
            out.push(Notice::CampaignProgress { level: self.campaign.current_level() });
        } else {
            out.push(Notice::LevelFailed { level: play.level, reason: reason.unwrap_or_default() });
        }
        let player_ids = self.roster.members.iter().map(|m| m.player_id.clone()).collect();
        out.effects.push(Effect::LevelResult { result: result.clone(), player_ids });
        out.effects.push(Effect::RoomStatus(if won { "level_complete" } else { "level_failed" }));
        self.phase = Phase::Result { result: Box::new(result) };
        Ok(())
    }

    fn handle_vote_outcome(&mut self, o: VoteOutcome, out: &mut Outcome2) -> Result<(), GameError> {
        let number = match &self.campaign.phase {
            CampaignPhase::CharterVote { vote } => vote.amendment,
            _ => {
                if self.campaign.recovery.is_some() {
                    2
                } else {
                    1
                }
            }
        };
        match o {
            VoteOutcome::Pending { .. } => out.effects.push(Effect::VoteProgress { number }),
            VoteOutcome::Revote => {
                out.push(Notice::Revote);
                out.effects.push(Effect::VoteProgress { number });
            }
            VoteOutcome::AwaitTieBreak { seat } => {
                out.push(Notice::TieBreak { seat });
                out.effects.push(Effect::VoteProgress { number });
            }
            VoteOutcome::Decided { choice } => {
                let rule = match number {
                    1 => self.campaign.connectivity.map(|c| Ruleset::new(c, Recovery::Base).key()),
                    _ => self.campaign.recovery.map(|r| Ruleset::new(Connectivity::Base, r).key()),
                }
                .unwrap_or_default();
                let rule_name = rule.split('+').nth(if number == 1 { 0 } else { 1 }).unwrap_or("").to_string();
                out.push(Notice::AmendmentApplied { amendment: number, choice, rule: rule_name.clone() });
                out.effects.push(Effect::Amendment { number, choice: Some(choice), rule: Some(rule_name) });
                let next = self.campaign.current_level().expect("a decided vote resumes the campaign");
                self.start_level(next, out)?;
            }
        }
        Ok(())
    }

    // ── Views ──────────────────────────────────────────────────────────────

    /// The state as one player may see it: private clues only for that player.
    pub fn view_for(&self, player: &str) -> serde_json::Value {
        let my_seat = self.roster.member(player).map(|m| m.seat);
        let members: Vec<serde_json::Value> = self
            .roster
            .members
            .iter()
            .map(|m| {
                serde_json::json!({
                    "seat": m.seat,
                    "name": self.names.get(&m.player_id).cloned().unwrap_or_default(),
                    "ready": m.ready,
                    "connected": m.connected,
                    "host": m.player_id == self.roster.host,
                    "you": m.player_id == player,
                })
            })
            .collect();
        let vote = match &self.campaign.phase {
            CampaignPhase::CharterVote { vote } => Some(serde_json::json!({
                "amendment": vote.amendment,
                "ballot_round": vote.ballot_round,
                "eligible": vote.eligible,
                "voted": vote.ballots.keys().collect::<Vec<_>>(),
                "my_vote": my_seat.and_then(|s| vote.ballots.get(&s)),
                "tie_breaker": vote.tie_breaker,
            })),
            _ => None,
        };
        let play = match &self.phase {
            Phase::Playing { play } => {
                let def = level_def(play.level).expect("level exists");
                let sim = Sim::new(def, play.rules).expect("valid level");
                let preview = (def.kind != LevelKind::Deduction).then(|| sim.resolve(&play.state).1);
                let my_clues: Vec<String> = my_seat
                    .and_then(|s| play.clue_hands.get(&s))
                    .map(|hand| {
                        let d = def.deduction.as_ref().expect("deduction level");
                        hand.iter().map(|k| d.clues[*k].text.clone()).collect()
                    })
                    .unwrap_or_default();
                let events: Vec<_> = sim.events_in(play.state.round).cloned().collect();
                Some(serde_json::json!({
                    "level": play.level,
                    "rules": play.rules,
                    "rules_key": play.rules.key(),
                    "state": play.state,
                    "turn_seat": play.turn_seat,
                    "passed": play.passed,
                    "threatened": sim.threatened(play.state.round),
                    "round_capacity": sim.round_capacity(play.state.round),
                    "events": events,
                    "evaluation": sim.evaluate(&play.state.board, &play.state.placements),
                    "preview": preview,
                    "last_report": play.last_report,
                    "par": def.par_for(&play.rules),
                    "my_clues": my_clues,
                    "clue_counts": play.clue_hands.iter().map(|(s, h)| (s.to_string(), h.len())).collect::<BTreeMap<_, _>>(),
                }))
            }
            _ => None,
        };
        let result = match &self.phase {
            Phase::Result { result } => serde_json::to_value(result).ok(),
            _ => None,
        };
        serde_json::json!({
            "id": self.id,
            "code": self.code,
            "version": self.version,
            "status": self.status_label(),
            "phase": match &self.phase {
                Phase::Lobby => "lobby",
                Phase::Playing { .. } => "playing",
                Phase::Result { .. } => "result",
                Phase::Vote => "vote",
                Phase::Complete => "complete",
            },
            "my_seat": my_seat,
            "members": members,
            "min_players": self.roster.min,
            "max_players": self.roster.max,
            "campaign": {
                "region": self.campaign.region,
                "current_level": self.campaign.current_level(),
                "connectivity": self.campaign.connectivity,
                "recovery": self.campaign.recovery,
                "best": self.campaign.best,
                "insights": self.campaign.insights,
                "restored": self.campaign.restored,
                "vote": vote,
            },
            "play": play,
            "result": result,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nt_core::solver::{solve, Limits};

    fn room() -> RoomState {
        let mut r = RoomState::new(Uuid::new_v4(), "ABCDEF".into(), "a", "Asha", 2, 4, Uuid::new_v4()).unwrap();
        r.join("b", "Bilal").unwrap();
        r
    }

    fn ready_all(r: &mut RoomState) {
        for p in ["a", "b"] {
            r.handle(p, Command::Ready { ready: true }).unwrap();
        }
    }

    /// Play the current level optimally through room commands.
    fn win_current(r: &mut RoomState) {
        let Phase::Playing { play } = &r.phase else { panic!("not playing") };
        let def = level_def(play.level).unwrap();
        let sim = Sim::new(def, play.rules).unwrap();
        let sol = solve(&sim, &play.state, &Limits::default()).solution().cloned().unwrap();
        let expedition = def.mode == Mode::Expedition;
        for a in sol.plan {
            if expedition && a == Action::EndRound {
                // everyone passes until the round resolves
                let round = match &r.phase {
                    Phase::Playing { play } => play.state.round,
                    _ => return,
                };
                while matches!(&r.phase, Phase::Playing { play } if play.state.round == round && play.state.status == Status::InProgress)
                {
                    let turn = match &r.phase {
                        Phase::Playing { play } => play.turn_seat.unwrap(),
                        _ => unreachable!(),
                    };
                    let p = if turn == 0 { "a" } else { "b" };
                    let v = r.version;
                    r.handle(p, Command::Pass { base_version: v }).unwrap();
                }
                continue;
            }
            let turn = match &r.phase {
                Phase::Playing { play } => play.turn_seat.unwrap_or(0),
                _ => return,
            };
            let p = if turn == 0 { "a" } else { "b" };
            let v = r.version;
            r.handle(p, Command::Act { action: a, base_version: v }).unwrap();
        }
    }

    #[test]
    fn lobby_requires_two_ready_players_and_host() {
        let mut r = RoomState::new(Uuid::new_v4(), "ABCDEF".into(), "a", "Asha", 2, 4, Uuid::new_v4()).unwrap();
        r.handle("a", Command::Ready { ready: true }).unwrap();
        assert_eq!(r.handle("a", Command::StartLevel).unwrap_err().code, "not_enough_players");
        r.join("b", "Bilal").unwrap();
        assert_eq!(r.handle("b", Command::StartLevel).unwrap_err().code, "not_host");
        assert_eq!(r.handle("a", Command::StartLevel).unwrap_err().code, "not_all_ready");
        r.handle("b", Command::Ready { ready: true }).unwrap();
        r.handle("a", Command::StartLevel).unwrap();
        assert!(matches!(r.phase, Phase::Playing { .. }));
    }

    #[test]
    fn stale_and_rejected_actions_do_not_change_state() {
        let mut r = room();
        ready_all(&mut r);
        r.handle("a", Command::StartLevel).unwrap();
        let before = r.clone();
        let stale =
            r.handle("a", Command::Act { action: Action::Plant { at: nt_core::Pos::new(2, 0) }, base_version: 1 });
        assert_eq!(stale.unwrap_err().code, "stale_state");
        let bad = r.handle(
            "a",
            Command::Act { action: Action::Plant { at: nt_core::Pos::new(0, 0) }, base_version: r.version },
        );
        assert_eq!(bad.unwrap_err().code, "wrong_terrain");
        assert_eq!(r, before);
    }

    #[test]
    fn expedition_turns_and_full_campaign_with_votes() {
        let mut r = room();
        ready_all(&mut r);
        r.handle("a", Command::StartLevel).unwrap();
        win_current(&mut r); // level 1
        assert!(matches!(&r.phase, Phase::Result { result } if result.won));
        r.handle("a", Command::Continue).unwrap();
        // level 2 is an expedition: seat 1 cannot act on seat 0's turn
        let v = r.version;
        let e = r.handle("b", Command::Act { action: Action::Plant { at: nt_core::Pos::new(2, 1) }, base_version: v });
        assert_eq!(e.unwrap_err().code, "not_your_turn");
        win_current(&mut r);
        r.handle("a", Command::Continue).unwrap();
        assert!(matches!(r.phase, Phase::Vote));
        r.handle("a", Command::Vote { choice: AmendmentChoice::B }).unwrap();
        let o = r.handle("b", Command::Vote { choice: AmendmentChoice::B }).unwrap();
        assert!(o.notices.iter().any(|n| matches!(n, Notice::AmendmentApplied { amendment: 1, .. })));
        assert_eq!(r.campaign.connectivity, Some(Connectivity::SteppingStone));
        // level 3 starts automatically
        assert!(matches!(&r.phase, Phase::Playing { play } if play.level == 3));
        for _ in 3..=4 {
            win_current(&mut r);
            r.handle("a", Command::Continue).unwrap();
        }
        assert!(matches!(r.phase, Phase::Vote));
        // tie → revote → tie → tie-breaker (run 0 → seat 0)
        r.handle("a", Command::Vote { choice: AmendmentChoice::A }).unwrap();
        assert!(r.handle("b", Command::Vote { choice: AmendmentChoice::B }).unwrap().notices.contains(&Notice::Revote));
        r.handle("a", Command::Vote { choice: AmendmentChoice::A }).unwrap();
        let o = r.handle("b", Command::Vote { choice: AmendmentChoice::B }).unwrap();
        assert!(o.notices.contains(&Notice::TieBreak { seat: 0 }));
        assert_eq!(r.handle("b", Command::Vote { choice: AmendmentChoice::B }).unwrap_err().code, "not_tie_breaker");
        r.handle("a", Command::Vote { choice: AmendmentChoice::B }).unwrap();
        assert_eq!(r.campaign.recovery, Some(Recovery::Active));
        assert_eq!(r.campaign.amendments_applied(), 2);
        // the deduction level deals private clues to each seat
        let Phase::Playing { play } = &r.phase else { panic!() };
        assert_eq!(play.level, 5);
        assert_eq!(play.clue_hands.len(), 2);
        let va = r.view_for("a");
        let vb = r.view_for("b");
        assert_ne!(va["play"]["my_clues"], vb["play"]["my_clues"]);
        assert!(va["play"]["my_clues"].as_array().unwrap().len() >= 2);
    }

    #[test]
    fn disconnect_skips_turn_and_reconnect_restores_view() {
        let mut r = room();
        ready_all(&mut r);
        r.handle("a", Command::StartLevel).unwrap();
        win_current(&mut r);
        r.handle("a", Command::Continue).unwrap(); // level 2 expedition, seat 0's turn
        r.disconnect("a");
        let Phase::Playing { play } = &r.phase else { panic!() };
        assert_eq!(play.turn_seat, Some(1));
        let before = r.view_for("a");
        r.reconnect("a");
        let after = r.view_for("a");
        assert_eq!(before["play"]["state"], after["play"]["state"]);
        assert_eq!(after["my_seat"], 0);
    }

    #[test]
    fn state_round_trips_for_restart_recovery() {
        let mut r = room();
        ready_all(&mut r);
        r.handle("a", Command::StartLevel).unwrap();
        let json = serde_json::to_value(&r).unwrap();
        let back: RoomState = serde_json::from_value(json).unwrap();
        assert_eq!(back, r);
    }
}
