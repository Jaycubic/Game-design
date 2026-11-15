//! River run simulation — deterministic, fixed-step, no I/O.
//!
//! Boats move forward on their own along a river of 3, 4 or 6 lanes (wider
//! with more boats, lanes numbered from 0 on the left). Players steer,
//! hop and press one context action. Plastic is collected on contact, weed mats
//! are cut, logs and debris waves must be hopped, and sewage pipes on the banks
//! drain the shared River Health until a crew plugs them with a gate code that
//! only a crewmate can read clearly. Exactly two rule changes happen in every
//! run, both chosen by players: a Twist vote and a Finale pick.

use serde::{Deserialize, Serialize};

pub const TICK_HZ: u32 = 30;
pub const DT: f32 = 1.0 / TICK_HZ as f32;
pub const RUN_SECONDS: f32 = 180.0;
pub const TWIST_AT: f32 = 63.0; // 35 %
pub const FINALE_AT: f32 = 140.0; // 78 %

// ── Solo Endless ──
// One capsize ends the run; the boat speeds up 2 % every 15 s; the two
// player-chosen rule changes come at 60 s and 150 s; score = metres + plastic x 5.
pub const ENDLESS_TWIST_AT: f32 = 60.0;
pub const ENDLESS_FINALE_AT: f32 = 150.0;
pub const ENDLESS_SPEEDUP: f32 = 1.02;
pub const ENDLESS_STEP: f32 = 15.0;
pub const ENDLESS_PLASTIC_POINTS: i32 = 5;
/// Team Endless: the crew shares this many lives (solo Endless has one).
pub const ENDLESS_CREW_LIVES: u8 = 3;

/// Catch-up current: a boat this far behind the front boat sails a little faster,
/// so stopping to fix a pipe never strands anyone.
pub const CATCH_UP_GAP: f32 = 25.0;
pub const CATCH_UP_MULT: f32 = 1.15;
pub const COUNTDOWN: f32 = 3.0;
pub const WARNING: f32 = 5.0;
pub const TWIST_VOTE: f32 = 15.0;
pub const FINALE_PICK: f32 = 10.0;
pub const REVEAL: f32 = 3.0;

pub const BASE_SPEED: f32 = 14.0;
pub const BOOST_MULT: f32 = 1.5;
pub const SLOW_MULT: f32 = 0.55;
pub const HOP_TIME: f32 = 0.75;
pub const CAPSIZE_TIME: f32 = 1.2;
/// After a capsize, obstacles pass through the boat for this long.
pub const IMMUNE_TIME: f32 = 1.0;
/// Plastic a capsized boat spills back into the river.
pub const CAPSIZE_DROP: u32 = 3;
pub const SLOW_TIME: f32 = 1.0;
pub const BOOST_TIME: f32 = 2.5;
pub const HIT_WINDOW: f32 = 0.9;
pub const CUT_REACH: f32 = 5.0;
pub const PLUG_TIME: f32 = 8.0;
/// After a correct gate code the boat stays put this long while its crew fixes the pipe.
pub const FIX_TIME: f32 = 1.0;
/// A boat with no crewmate sees its gate code for this long, then types it from memory.
pub const MEMORY_TIME: f32 = 2.0;

// ── Crew key locks (2+ players) ──
// When the front boat nears an open pipe, every player is dealt a different
// secret key and sees only their own, briefly. A pipe's lock has one slot per
// player (seat order); teammates answer by pressing their key, and a fixer
// types the full set. Several fixers may open locks at the same pipe: answers go
// to all of them and the first correct entry fixes it. A wrong entry or running
// out of time costs River Health and deals new keys.
pub const KEY_DEAL_AT: f32 = 70.0;
pub const KEY_SHOW: f32 = 3.0;
pub const LOCK_TIME: f32 = 12.0;
pub const LOCK_FAIL_COST: f32 = 5.0;

/// Keys dealt for one pipe.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Deal {
    pub pipe: u32,
    /// (seat, key glyph), in seat order.
    pub keys: Vec<(u8, u8)>,
    /// Seconds each player still sees their own key.
    pub shown: f32,
    /// Keys teammates have sent for this pipe: (seat, glyph).
    pub answers: Vec<(u8, u8)>,
}
pub const COURSE_LEN: f32 = 3300.0;

pub const HEALTH_START: f32 = 70.0;
pub const HEALTH_PLASTIC: f32 = 0.5;
pub const HEALTH_MISSED: f32 = -0.8;
pub const HEALTH_CUT: f32 = 1.5;
pub const HEALTH_PLUG: f32 = 10.0;
pub const PIPE_DRAIN: f32 = 0.12; // per active pipe per second
pub const REVIVED_AT: f32 = 60.0;
pub const REVIVED_BONUS: i32 = 20;

pub const GLYPHS: u8 = 6; // leaf, drop, fish, bridge, arrow, star

/// Lanes for a number of boats: more boats get a wider river.
pub fn lane_count_for(boats: usize) -> u8 {
    match boats {
        0 | 1 => 3,
        2 => 4,
        _ => 6,
    }
}

/// (plastic, obstacle) spawn multipliers for a river of `lanes` lanes.
pub fn spawn_mults(lanes: u8) -> (f32, f32) {
    match lanes {
        0..=3 => (1.0, 1.0),
        4 | 5 => (1.3, 1.15),
        _ => (1.8, 1.4),
    }
}

fn three() -> u8 {
    3
}

fn one() -> f32 {
    1.0
}

// ── Rivers ──
// Each river adds one mechanic on top of everything before it.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum River {
    /// Town Canal: the original river.
    #[default]
    Canal,
    /// Mangrove Maze: ghost nets span two lanes and hold trapped plastic.
    Mangrove,
    /// Rapids Gorge: currents push boats sideways; rocks can't be hopped.
    Gorge,
}

pub const RIVERS: [River; 3] = [River::Canal, River::Mangrove, River::Gorge];

impl River {
    pub fn key(self) -> &'static str {
        match self {
            River::Canal => "canal",
            River::Mangrove => "mangrove",
            River::Gorge => "gorge",
        }
    }
    pub fn from_key(key: &str) -> Option<River> {
        RIVERS.into_iter().find(|r| r.key() == key)
    }
    /// The crew score needed for the second star.
    pub fn score_target(self) -> i32 {
        match self {
            River::Canal => 120,
            River::Mangrove => 150,
            River::Gorge => 180,
        }
    }
}

/// Presses (by anyone) needed to cut a ghost net free.
pub const NET_HP: u8 = 2;
/// Plastic a freed ghost net releases.
pub const NET_PLASTIC: u32 = 4;

fn is_zero_u8(v: &u8) -> bool {
    *v == 0
}
fn is_zero_i8(v: &i8) -> bool {
    *v == 0
}

// ── River Director (adaptive difficulty) ──
// One factor `d` scales the river: obstacles and pipes x d, debris waves come
// d times as often, plastic x (2 - d). It never touches scoring, River Health
// rules, the rule-change options or the speeds players react to.
pub const D_MIN: f32 = 0.8;
pub const D_MAX: f32 = 1.2;
/// In-run nudge size and how far it may drift from the room's starting value.
pub const D_STEP: f32 = 0.03;
pub const D_SWING: f32 = 0.1;
/// Seconds of run clock between in-run checks.
pub const DIRECTOR_EVERY: f32 = 10.0;
/// The "flow band": share of passing plastic a crew should be netting.
pub const FLOW_LOW: f32 = 0.55;
pub const FLOW_HIGH: f32 = 0.80;
/// The course is generated this far ahead of the leading boat.
pub const GEN_AHEAD: f32 = 200.0;

// ── deterministic RNG ────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15 | 1)
    }
    pub fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        (x >> 32) as u32
    }
    pub fn range(&mut self, n: u32) -> u32 {
        self.next_u32() % n.max(1)
    }
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() % 1_000_000) as f32 / 1_000_000.0
    }
    pub fn lane(&mut self, lanes: &[i8]) -> i8 {
        lanes[self.range(lanes.len() as u32) as usize]
    }
}

// ── world ─────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Plastic,
    Weed,
    Log,
    Wave,
    /// A sewage pipe on a bank: `lane` is the side, -1 (left) or +1 (right).
    /// It is plugged from the outermost open lane on that side.
    Pipe,
    /// Mangrove Maze: an abandoned fishing net across `lane` and `lane + 1`.
    /// It slows boats like weed; cutting it (NET_HP presses, by anyone) frees trapped plastic.
    Net,
    /// Rapids Gorge: a current that pushes a boat one lane towards `dir` unless it hops.
    Current,
    /// Rapids Gorge: a rock. Capsizes like a log, but can't be hopped: steer round it.
    Rock,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Obj {
    pub id: u32,
    pub kind: Kind,
    pub lane: i8,
    pub z: f32,
    pub alive: bool,
    /// Pipes: plugged.
    #[serde(default)]
    pub plugged: bool,
    /// Debris waves on wide rivers leave a second gap lane.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gap2: Option<i8>,
    /// Ghost nets: presses still needed to cut it free.
    #[serde(default, skip_serializing_if = "is_zero_u8")]
    pub hp: u8,
    /// Currents: which way it pushes (-1 left, +1 right).
    #[serde(default, skip_serializing_if = "is_zero_i8")]
    pub dir: i8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Twist {
    /// Debris waves every 15 s, but extra plastic appears.
    MonsoonSurge,
    /// The river loses a third of its lanes and pipes pour twice as fast.
    DrySeason,
    /// Plastic counts double, but weed mats spread.
    VolunteerDay,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Finale {
    /// The picking crew's boost fills and extra plastic flows into their lanes for 20 s.
    SecondWind,
    /// Every unplugged pipe pours at half rate for the rest of the run.
    CleanSweep,
}

pub const TWISTS: [Twist; 3] = [Twist::MonsoonSurge, Twist::DrySeason, Twist::VolunteerDay];
pub const FINALES: [Finale; 2] = [Finale::SecondWind, Finale::CleanSweep];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Plug {
    pub pipe: u32,
    /// Solo: a 3-symbol code. Crew lock: every player's key, in seat order.
    pub code: Vec<u8>,
    pub entered: Vec<u8>,
    pub left: f32,
    /// A crew key lock (rather than the solo memory code).
    #[serde(default)]
    pub team: bool,
}

/// A boat's upgrade levels, 1–5 each. Level 2 is the starter boat and
/// matches the base tuning; each level above it is a modest step.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stats {
    /// Top speed: +4 % per level.
    pub speed: u8,
    /// Plastic net: +15 % pickup reach per level; level 5 also nets the next lane.
    pub net: u8,
    /// Weed cutter: +15 % cutting reach per level.
    pub cutter: u8,
    /// Boost charge: +15 % charge per plastic per level.
    pub boost: u8,
}

impl Default for Stats {
    fn default() -> Self {
        Stats { speed: 2, net: 2, cutter: 2, boost: 2 }
    }
}

impl Stats {
    fn step(level: u8, per: f32) -> f32 {
        1.0 + per * (level.clamp(1, 5) as f32 - 2.0)
    }
    pub fn speed_mult(&self) -> f32 {
        Self::step(self.speed, 0.04)
    }
    pub fn net_window(&self) -> f32 {
        HIT_WINDOW * Self::step(self.net, 0.15)
    }
    pub fn wide_net(&self) -> bool {
        self.net >= 5
    }
    pub fn cut_reach(&self) -> f32 {
        CUT_REACH * Self::step(self.cutter, 0.15)
    }
    pub fn boost_per_plastic(&self) -> f32 {
        0.12 * Self::step(self.boost, 0.15)
    }
}

/// Stats for each boat in the catalog (unknown keys get the starter stats).
pub fn boat_stats(key: &str) -> Stats {
    let (speed, net, cutter, boost) = match key {
        "boat_sky" => (2, 3, 2, 2),
        "boat_coral" => (3, 2, 2, 3),
        "boat_lilac" => (3, 3, 2, 2),
        "boat_leaf" => (3, 3, 3, 3),
        "boat_night" => (4, 3, 3, 4),
        "boat_lotus" => (3, 5, 4, 3),
        _ => (2, 2, 2, 2),
    };
    Stats { speed, net, cutter, boost }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Boat {
    pub seat: u8,
    pub crew: u8,
    pub lane: i8,
    pub z: f32,
    pub hop: f32,
    pub capsize: f32,
    /// Seconds of protection left (counts the capsize itself plus IMMUNE_TIME).
    #[serde(default)]
    pub immune: f32,
    pub slow: f32,
    pub boost: f32,
    pub boosting: f32,
    pub plug: Option<Plug>,
    pub plastic: u32,
    #[serde(default)]
    pub stats: Stats,
    /// Lifetime counters for this run (performance scoring).
    #[serde(default)]
    pub collected: u32,
    #[serde(default)]
    pub capsizes: u32,
    #[serde(default)]
    pub plugged: u32,
    /// Weed mats this boat cut.
    #[serde(default)]
    pub cuts: u32,
    /// Seconds left fixing a just-plugged pipe (the boat is stopped).
    #[serde(default)]
    pub fixing: f32,
    /// The last current that pushed this boat (each current pushes once).
    #[serde(default)]
    pub last_current: u32,
}

impl Boat {
    pub fn airborne(&self) -> bool {
        self.hop > 0.0
    }
    pub fn speed(&self) -> f32 {
        if self.capsize > 0.0 || self.plug.is_some() || self.fixing > 0.0 {
            return 0.0;
        }
        let mut s = BASE_SPEED * self.stats.speed_mult();
        if self.boosting > 0.0 {
            s *= BOOST_MULT;
        }
        if self.slow > 0.0 {
            s *= SLOW_MULT;
        }
        s
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Crew {
    pub id: u8,
    pub plastic: u32,
    pub pipes: u32,
    pub banks: u32,
    /// Ghost nets cut free (Mangrove Maze).
    #[serde(default)]
    pub nets: u32,
}

impl Crew {
    pub fn score(&self) -> i32 {
        (self.plastic as f32 * (1.0 + 0.5 * self.pipes as f32)).round() as i32 + 5 * self.banks as i32
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeKind {
    Twist,
    Finale,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "phase", rename_all = "snake_case")]
pub enum Phase {
    Countdown {
        left: f32,
    },
    Running,
    /// "Rule change in 5…" — the run keeps going.
    Warning {
        change: ChangeKind,
        left: f32,
    },
    /// Paused: the crew votes (Twist, then Finale). `pickers` is kept for older clients and is always None.
    Choosing {
        change: ChangeKind,
        left: f32,
        pickers: Option<u8>,
    },
    /// Paused: the card shows what changed.
    Reveal {
        change: ChangeKind,
        left: f32,
        spin: bool,
    },
    Finished,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    Collected { seat: u8, id: u32 },
    Missed { id: u32 },
    Cut { seat: u8, id: u32 },
    Slowed { seat: u8 },
    Capsized {
        seat: u8,
        /// Plastic spilled back into the river.
        #[serde(default)]
        dropped: u32,
    },
    Bumped { seat: u8, by: u8 },
    Boost { seat: u8 },
    PlugStarted { seat: u8, pipe: u32 },
    Plugged { seat: u8, pipe: u32 },
    PlugFailed { seat: u8, pipe: u32 },
    Ping { seat: u8, glyph: u8 },
    WaveIncoming { id: u32 },
    RuleChanged { change: ChangeKind, choice: String },
    Choked,
    /// A ghost net took a cut; `left` presses remain.
    NetHit { seat: u8, id: u32, left: u8 },
    /// A ghost net was cut free and released its plastic.
    NetFreed { seat: u8, id: u32 },
    /// A current pushed a boat one lane.
    Pushed { seat: u8, dir: i8 },
    /// New keys for the pipe ahead (each player sees only their own).
    KeysDealt { pipe: u32 },
    /// A fixer pressed hop to leave a lock and sail on.
    PlugSkipped { seat: u8, pipe: u32 },
    /// A teammate sent their key to an open lock.
    Answered { seat: u8, pipe: u32 },
}

/// One player's input since the last tick.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Input {
    /// -1 = steer left, +1 = steer right (one lane per press).
    #[serde(default)]
    pub steer: i8,
    #[serde(default)]
    pub hop: bool,
    #[serde(default)]
    pub action: bool,
    /// Glyph 0..5: a ping, or a gate-code entry while plugging.
    #[serde(default)]
    pub glyph: Option<u8>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Results {
    pub scores: Vec<(u8, i32)>,
    pub winner: Option<u8>,
    pub health: f32,
    pub choked: bool,
    pub revived: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Run {
    pub seed: u64,
    pub rng: Rng,
    /// Run clock (only advances while the river is moving).
    pub t: f32,
    pub tick: u64,
    pub phase: Phase,
    pub boats: Vec<Boat>,
    pub crews: Vec<Crew>,
    pub objs: Vec<Obj>,
    pub next_id: u32,
    pub health: f32,
    pub choked: bool,
    pub twist: Option<Twist>,
    pub finale: Option<Finale>,
    pub finale_crew: Option<u8>,
    pub changes: u8,
    pub votes: Vec<(u8, u8)>,
    pub next_wave: f32,
    pub second_wind_left: f32,
    pub spawn_timer: f32,
    pub events: Vec<Event>,
    pub results: Option<Results>,
    /// Scripted 60-second tutorial course: no rule changes, no debris waves.
    #[serde(default)]
    pub tutorial: bool,
    /// Lanes in the full-width river (3, 4 or 6).
    #[serde(default = "three")]
    pub lane_count: u8,
    /// River Director: current and starting difficulty, and whether it may adapt in-run
    /// (only with a single crew, so it can never favour one crew over another).
    #[serde(default = "one")]
    pub difficulty: f32,
    #[serde(default = "one")]
    pub difficulty_start: f32,
    #[serde(default)]
    pub adaptive: bool,
    #[serde(default)]
    pub director: DirectorWindow,
    /// Plastic that escaped downstream, for performance scoring.
    #[serde(default)]
    pub missed: u32,
    /// Course generation cursor (the next row's z) and the next pipe's z.
    #[serde(default)]
    pub gen_z: f32,
    #[serde(default)]
    pub gen_pipe: f32,
    #[serde(default)]
    pub river: River,
    /// Crew key deals, one per pipe being approached.
    #[serde(default)]
    pub deals: Vec<Deal>,
    /// Seats whose player is disconnected: their key is filled in automatically.
    #[serde(default)]
    pub away: Vec<u8>,
    /// Endless mode (see the ENDLESS_ constants).
    #[serde(default)]
    pub endless: bool,
    /// Endless: lives the crew has left (each capsize uses one).
    #[serde(default)]
    pub lives: u8,
}

/// What the Director saw since its last check.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DirectorWindow {
    pub timer: f32,
    pub collected: u32,
    pub missed: u32,
    pub capsizes: u32,
}

pub const TUTORIAL_SECONDS: f32 = 60.0;

impl Run {
    /// Seats are assigned to crews alternately (0,1,0,1). Two players = one crew.
    pub fn new(seed: u64, seats: &[u8]) -> Run {
        Run::with_difficulty(seed, seats, 1.0)
    }

    /// A run whose river starts at difficulty `d` (see the River Director constants).
    pub fn with_difficulty(seed: u64, seats: &[u8], d: f32) -> Run {
        Run::create(seed, seats, d, River::Canal)
    }

    /// A run on a chosen river at difficulty `d`.
    pub fn create(seed: u64, seats: &[u8], d: f32, river: River) -> Run {
        // Round 3: every player is in one crew, against the pollution.
        let lane_count = lane_count_for(seats.len());
        let k = seats.len().max(1) as f32;
        // Spread the starting boats across the river.
        let start =
            |i: usize| (((i as f32 + 0.5) * lane_count as f32 / k - 0.5).round() as i8).clamp(0, lane_count as i8 - 1);
        let boats: Vec<Boat> = seats
            .iter()
            .enumerate()
            .map(|(i, &seat)| Boat {
                seat,
                crew: 0,
                lane: start(i),
                z: -(i as f32) * 2.5,
                hop: 0.0,
                capsize: 0.0,
                immune: 0.0,
                slow: 0.0,
                boost: 0.0,
                boosting: 0.0,
                plug: None,
                plastic: 0,
                stats: Stats::default(),
                collected: 0,
                capsizes: 0,
                plugged: 0,
                cuts: 0,
                fixing: 0.0,
                last_current: 0,
            })
            .collect();
        let crews = vec![Crew { id: 0, ..Default::default() }];
        let mut run = Run {
            seed,
            rng: Rng::new(seed),
            t: 0.0,
            tick: 0,
            phase: Phase::Countdown { left: COUNTDOWN },
            boats,
            crews,
            objs: Vec::new(),
            next_id: 1,
            health: HEALTH_START,
            choked: false,
            twist: None,
            finale: None,
            finale_crew: None,
            changes: 0,
            votes: Vec::new(),
            next_wave: 30.0,
            second_wind_left: 0.0,
            spawn_timer: 0.0,
            events: Vec::new(),
            results: None,
            tutorial: false,
            lane_count,
            difficulty: d.clamp(D_MIN, D_MAX),
            difficulty_start: d.clamp(D_MIN, D_MAX),
            adaptive: true,
            director: DirectorWindow::default(),
            missed: 0,
            gen_z: 45.0,
            gen_pipe: 120.0,
            river,
            deals: Vec::new(),
            away: Vec::new(),
            endless: false,
            lives: 0,
        };
        run.next_wave = 30.0 / run.difficulty;
        run.generate_to(GEN_AHEAD);
        run
    }

    /// A short hand-built course that teaches one control at a time:
    /// plastic lines → logs to hop → weed to cut → a pipe to plug → open water.
    pub fn tutorial(seed: u64, seats: &[u8]) -> Run {
        let mut run = Run::new(seed, seats);
        run.tutorial = true;
        run.adaptive = false;
        run.objs.clear();
        run.gen_z = f32::MAX; // hand-built course only
        run.next_id = 1;
        run.next_wave = f32::MAX;
        // 1–2: steer and collect (plastic on both side lanes)
        for k in 0..6 {
            run.spawn(Kind::Plastic, if k < 3 { 0 } else { 2 }, 40.0 + k as f32 * 4.0);
        }
        for k in 0..5 {
            run.spawn(Kind::Plastic, 1, 110.0 + k as f32 * 3.0);
        }
        // 3: hop — logs across every lane, so hopping is the only way through
        for (i, z) in [190.0, 250.0, 310.0].into_iter().enumerate() {
            for lane in 0..3 {
                run.spawn(Kind::Log, lane, z);
            }
            run.spawn(Kind::Plastic, i as i8, z + 8.0);
        }
        // 4: cut — weed mats in every lane
        for (i, z) in [380.0, 440.0].into_iter().enumerate() {
            for lane in 0..3 {
                run.spawn(Kind::Weed, lane, z);
                run.spawn(Kind::Plastic, lane, z + 5.0 + i as f32);
            }
        }
        // 5: plug — a pipe on each bank, well spaced
        run.spawn(Kind::Pipe, -1, 520.0);
        run.spawn(Kind::Pipe, 1, 610.0);
        // then a relaxed run-out with plastic to finish
        for k in 0..20 {
            run.spawn(Kind::Plastic, (k % 3) as i8, 660.0 + k as f32 * 7.0);
        }
        run
    }

    /// A solo Endless run on a chosen river at difficulty `d`.
    /// An Endless run for 1–4 players. Solo has one life; a crew shares ENDLESS_CREW_LIVES.
    pub fn endless(seed: u64, seats: &[u8], d: f32, river: River) -> Run {
        let mut run = Run::create(seed, seats, d, river);
        run.endless = true;
        run.lives = if seats.len() > 1 { ENDLESS_CREW_LIVES } else { 1 };
        run.adaptive = false; // the run speeds up on its own schedule instead
        run
    }

    pub fn duration(&self) -> f32 {
        if self.tutorial {
            TUTORIAL_SECONDS
        } else if self.endless {
            f32::INFINITY
        } else {
            RUN_SECONDS
        }
    }

    fn twist_at(&self) -> f32 {
        if self.endless { ENDLESS_TWIST_AT } else { TWIST_AT }
    }

    fn finale_at(&self) -> f32 {
        if self.endless { ENDLESS_FINALE_AT } else { FINALE_AT }
    }

    /// Endless speed-up: x1.02 for every 15 s survived (1.0 otherwise).
    pub fn pace(&self) -> f32 {
        if self.endless {
            ENDLESS_SPEEDUP.powi((self.t / ENDLESS_STEP) as i32)
        } else {
            1.0
        }
    }

    /// Metres travelled by the leading boat.
    pub fn distance(&self) -> f32 {
        self.leader_z().max(0.0)
    }

    /// The open lanes, 0 = leftmost. Dry Season closes a third of them on the right.
    pub fn lanes(&self) -> Vec<i8> {
        let n = self.lane_count;
        let open = if self.twist == Some(Twist::DrySeason) { n - n / 3 } else { n };
        (0..open as i8).collect()
    }

    /// The lane a boat must be in to plug a pipe on `side` (-1 left, +1 right).
    pub fn pipe_lane(&self, side: i8) -> i8 {
        let lanes = self.lanes();
        if side < 0 {
            lanes[0]
        } else {
            *lanes.last().unwrap()
        }
    }

    fn spawn(&mut self, kind: Kind, lane: i8, z: f32) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        let hp = if kind == Kind::Net { NET_HP } else { 0 };
        self.objs.push(Obj { id, kind, lane, z, alive: true, plugged: false, gap2: None, hp, dir: 0 });
        id
    }

    /// Hand-shaped random patterns along the whole course.
    /// Generate course rows up to `until` (and never past the course end).
    /// Wider rivers and a higher difficulty get denser rows (more obstacles);
    /// plastic per row is then thinned or topped up to reach its own target,
    /// plastic_mult x (2 - d). The course is built ahead of the boats as they go,
    /// so the Director's in-run changes shape the river still to come.
    pub fn generate_to(&mut self, until: f32) {
        let n = self.lane_count as i8;
        let all: Vec<i8> = (0..n).collect();
        let (plastic_mult, obstacle_mult) = spawn_mults(self.lane_count);
        let d = self.difficulty;
        let rows = obstacle_mult * d;
        // A row holds about 2.6 plastic; `ratio` is how much of that each row should carry.
        let ratio = plastic_mult * (2.0 - d) / rows;
        let keep = ratio.min(1.0);
        let extra_plastic = (ratio - 1.0).max(0.0) * 2.6;
        let mut z = self.gen_z;
        let mut next_pipe = self.gen_pipe;
        let until = if self.endless { until } else { until.min(COURSE_LEN) };
        while z < until {
            match self.rng.range(10) {
                0..=3 => {
                    // a line of plastic to follow
                    let lane = self.rng.lane(&all);
                    for k in 0..(3 + self.rng.range(3)) {
                        self.gen_plastic(keep, lane, z + k as f32 * 3.0);
                    }
                }
                4..=5 if self.river == River::Mangrove && n >= 2 && self.rng.unit() < 0.5 => {
                    // a ghost net across two lanes; its plastic is trapped until it's cut
                    let lane = self.rng.range((n - 1) as u32) as i8;
                    self.spawn(Kind::Net, lane, z);
                }
                4..=5 => {
                    // weed mat guarding plastic in the same lane
                    let lane = self.rng.lane(&all);
                    self.spawn(Kind::Weed, lane, z);
                    for k in 0..3 {
                        self.gen_plastic(keep, lane, z + 4.0 + k as f32 * 3.0);
                    }
                }
                6..=7 => {
                    // a run of logs across 1-2 lanes (up to 3 on a wide river), plastic beside it
                    let extra = if n >= 6 { self.rng.range(2) as i8 } else { 0 };
                    let c = (1 + self.rng.range(2) as i8 + extra).min(n - 1);
                    let start = self.rng.range((n - c + 1) as u32) as i8;
                    let kind = if self.river == River::Gorge && self.rng.unit() < 0.5 { Kind::Rock } else { Kind::Log };
                    for l in start..start + c {
                        self.spawn(kind, l, z);
                    }
                    let left = start > 0 && (start + c >= n || self.rng.range(2) == 0);
                    let reward = if left { start - 1 } else { start + c };
                    self.gen_plastic(keep, reward, z);
                }
                _ if self.river == River::Gorge && self.rng.unit() < 0.5 => {
                    // a current, with plastic waiting where it carries you
                    let lane = self.rng.lane(&all);
                    let dir = if lane == 0 {
                        1
                    } else if lane == n - 1 || self.rng.range(2) == 0 {
                        -1
                    } else {
                        1
                    };
                    let id = self.spawn(Kind::Current, lane, z);
                    if let Some(c) = self.objs.iter_mut().find(|o| o.id == id) {
                        c.dir = dir;
                    }
                    for k in 0..2 {
                        self.gen_plastic(keep, lane + dir, z + 6.0 + k as f32 * 3.0);
                    }
                }
                _ => {
                    // scattered plastic
                    for _ in 0..2 {
                        let lane = self.rng.lane(&all);
                        let dz = self.rng.unit() * 8.0;
                        self.gen_plastic(keep, lane, z + dz);
                    }
                }
            }
            if extra_plastic > 0.0 {
                let count = extra_plastic.floor() as u32 + u32::from(self.rng.unit() < extra_plastic.fract());
                for _ in 0..count {
                    let lane = self.rng.lane(&all);
                    let dz = 4.0 + self.rng.unit() * 8.0;
                    self.spawn(Kind::Plastic, lane, z + dz);
                }
            }
            if z >= next_pipe {
                let side = if self.rng.range(2) == 0 { -1 } else { 1 };
                self.spawn(Kind::Pipe, side, z + 6.0);
                next_pipe += (190.0 + self.rng.unit() * 60.0) / d;
            }
            z += (16.0 + self.rng.unit() * 10.0) / rows;
        }
        self.gen_z = z;
        self.gen_pipe = next_pipe;
    }

    /// Course plastic: kept with probability `keep` (always drawn, so the river
    /// layout stays deterministic for a given seed and difficulty).
    fn gen_plastic(&mut self, keep: f32, lane: i8, z: f32) {
        if self.rng.unit() < keep {
            self.spawn(Kind::Plastic, lane, z);
        }
    }

    pub fn leader_z(&self) -> f32 {
        self.boats.iter().map(|b| b.z).fold(f32::MIN, f32::max)
    }

    pub fn last_z(&self) -> f32 {
        self.boats.iter().map(|b| b.z).fold(f32::MAX, f32::min)
    }

    fn crew_mut(&mut self, id: u8) -> &mut Crew {
        self.crews.iter_mut().find(|c| c.id == id).expect("crew exists")
    }

    pub fn crew_size(&self, crew: u8) -> usize {
        self.boats.iter().filter(|b| b.crew == crew).count()
    }

    fn heal(&mut self, d: f32) {
        self.health = (self.health + d).clamp(0.0, 100.0);
        if self.health <= 0.0 && !self.choked {
            self.choked = true;
            self.events.push(Event::Choked);
        }
    }

    pub fn is_paused(&self) -> bool {
        matches!(self.phase, Phase::Countdown { .. } | Phase::Choosing { .. } | Phase::Reveal { .. } | Phase::Finished)
    }

    /// A Twist vote or Finale pick from a seat. Ignored outside the matching phase.
    pub fn choose(&mut self, seat: u8, option: u8) -> Result<(), &'static str> {
        let Phase::Choosing { change, pickers, .. } = self.phase.clone() else { return Err("no choice is open") };
        let max = match change {
            ChangeKind::Twist => TWISTS.len(),
            ChangeKind::Finale => FINALES.len(),
        } as u8;
        if option >= max {
            return Err("unknown option");
        }
        if !self.boats.iter().any(|b| b.seat == seat) {
            return Err("not in this run");
        }
        let _ = pickers;
        self.votes.retain(|(s, _)| *s != seat);
        self.votes.push((seat, option));
        if self.votes.len() == self.boats.len() {
            match change {
                ChangeKind::Twist => self.resolve_twist(),
                ChangeKind::Finale => self.resolve_finale_vote(),
            }
        }
        Ok(())
    }

    /// The most-voted option (of `n`), and whether a tie (or no votes) was settled by a spin.
    fn tally(&mut self, n: usize) -> (usize, bool) {
        let mut counts = vec![0u32; n];
        for (_, o) in &self.votes {
            counts[*o as usize] += 1;
        }
        let best = *counts.iter().max().unwrap();
        let tied: Vec<usize> = (0..n).filter(|i| counts[*i] == best).collect();
        (tied[self.rng.range(tied.len() as u32) as usize], tied.len() > 1)
    }

    fn resolve_finale_vote(&mut self) {
        let (pick, spin) = self.tally(FINALES.len());
        self.resolve_finale(Some(pick), spin);
    }

    fn resolve_twist(&mut self) {
        let (pick, spin) = self.tally(TWISTS.len());
        let twist = TWISTS[pick];
        self.twist = Some(twist);
        self.changes += 1;
        if twist == Twist::MonsoonSurge {
            self.next_wave = self.next_wave.min(self.t + 15.0 / self.difficulty);
        }
        if twist == Twist::DrySeason {
            let max = *self.lanes().last().unwrap();
            for b in &mut self.boats {
                b.lane = b.lane.min(max);
            }
        }
        self.events.push(Event::RuleChanged { change: ChangeKind::Twist, choice: name_of(&twist) });
        self.phase = Phase::Reveal { change: ChangeKind::Twist, left: REVEAL, spin };
    }

    fn resolve_finale(&mut self, choice: Option<usize>, spin: bool) {
        let idx = choice.unwrap_or_else(|| self.rng.range(FINALES.len() as u32) as usize);
        let f = FINALES[idx];
        self.finale = Some(f);
        self.changes += 1;
        if f == Finale::SecondWind {
            self.second_wind_left = 20.0;
            if let Some(c) = self.finale_crew {
                for b in self.boats.iter_mut().filter(|b| b.crew == c) {
                    b.boost = 1.0;
                }
            }
        }
        self.events.push(Event::RuleChanged { change: ChangeKind::Finale, choice: name_of(&f) });
        self.phase = Phase::Reveal { change: ChangeKind::Finale, left: REVEAL, spin: spin || choice.is_none() };
    }

    /// Advance one fixed step with each seat's input.
    pub fn step(&mut self, inputs: &[(u8, Input)]) {
        self.tick += 1;
        // Choices and phase timers.
        match self.phase.clone() {
            Phase::Finished => return,
            Phase::Countdown { left } => {
                self.phase = if left - DT <= 0.0 { Phase::Running } else { Phase::Countdown { left: left - DT } };
                return;
            }
            Phase::Choosing { change, left, .. } => {
                if left - DT <= 0.0 {
                    match change {
                        ChangeKind::Twist => self.resolve_twist(),
                        ChangeKind::Finale => self.resolve_finale_vote(),
                    }
                } else if let Phase::Choosing { left: l, .. } = &mut self.phase {
                    *l -= DT;
                }
                return;
            }
            Phase::Reveal { left, .. } => {
                if left - DT <= 0.0 {
                    self.phase = Phase::Running;
                } else if let Phase::Reveal { left: l, .. } = &mut self.phase {
                    *l -= DT;
                }
                return;
            }
            Phase::Warning { change, left } => {
                if left - DT <= 0.0 {
                    if change == ChangeKind::Finale {
                        self.finale_crew = Some(0); // Second Wind helps the whole crew
                    }
                    self.votes.clear();
                    let left = if change == ChangeKind::Twist { TWIST_VOTE } else { FINALE_PICK };
                    self.phase = Phase::Choosing { change, left, pickers: None };
                    return;
                }
                self.phase = Phase::Warning { change, left: left - DT };
            }
            Phase::Running => {
                if self.tutorial {
                    // no rule changes in the tutorial
                } else if self.changes == 0 && self.t + WARNING >= self.twist_at() {
                    self.phase = Phase::Warning { change: ChangeKind::Twist, left: self.twist_at() - self.t };
                } else if self.changes == 1 && self.twist.is_some() && self.t + WARNING >= self.finale_at() {
                    self.phase = Phase::Warning { change: ChangeKind::Finale, left: self.finale_at() - self.t };
                }
            }
        }

        self.t += DT;
        for (seat, input) in inputs {
            self.apply_input(*seat, input);
        }
        self.move_boats();
        self.collide();
        self.pressure();
        self.deal_keys();
        self.direct();
        let ahead = self.leader_z() + GEN_AHEAD;
        if self.gen_z < ahead {
            self.generate_to(ahead);
        }
        let wrecked = self.endless && self.lives == 0;
        if self.t >= self.duration() || wrecked {
            self.finish();
        }
    }

    fn apply_input(&mut self, seat: u8, input: &Input) {
        let lanes = self.lanes();
        let (min, max) = (*lanes.first().unwrap(), *lanes.last().unwrap());
        let Some(i) = self.boats.iter().position(|b| b.seat == seat) else { return };
        if let Some(g) = input.glyph.filter(|g| *g < GLYPHS) {
            if let Some(plug) = &mut self.boats[i].plug {
                plug.entered.push(g);
                let (pipe, team, typed, done) = (plug.pipe, plug.team, plug.entered.len(), plug.entered.len() == plug.code.len());
                // A fixer's own key, typed into its slot, also counts as their answer for other fixers.
                if team && self.slot_of(seat) == Some(typed - 1) {
                    self.answer(pipe, seat, g);
                }
                if done {
                    let plug = self.boats[i].plug.take().expect("plugging");
                    self.finish_plug(i, pipe, plug.entered == plug.code);
                }
            } else {
                // Not fixing: the key answers any teammate's open lock (and shows over the boat).
                let mut open: Vec<u32> = self.boats.iter().filter_map(|b| b.plug.as_ref().filter(|p| p.team).map(|p| p.pipe)).collect();
                open.dedup();
                for pipe in open {
                    self.answer(pipe, seat, g);
                    self.events.push(Event::Answered { seat, pipe });
                }
                self.events.push(Event::Ping { seat, glyph: g });
            }
        }
        // ▲ while fixing leaves the lock and sails on.
        if input.hop && self.boats[i].plug.is_some() {
            let pipe = self.boats[i].plug.take().map(|p| p.pipe).unwrap_or_default();
            self.events.push(Event::PlugSkipped { seat, pipe });
            return;
        }
        let b = &mut self.boats[i];
        if b.capsize > 0.0 || b.plug.is_some() || b.fixing > 0.0 {
            return;
        }
        if input.steer != 0 {
            b.lane = (b.lane + input.steer.signum()).clamp(min, max);
        }
        if input.hop && !b.airborne() {
            b.hop = HOP_TIME;
        }
        if input.action {
            self.do_action(i);
        }
    }

    /// Give a seat's boat its stats (called before the run starts).
    pub fn equip(&mut self, seat: u8, stats: Stats) {
        if let Some(b) = self.boats.iter_mut().find(|b| b.seat == seat) {
            b.stats = stats;
        }
    }

    fn do_action(&mut self, i: usize) {
        let (seat, lane, z) = (self.boats[i].seat, self.boats[i].lane, self.boats[i].z);
        let reach = self.boats[i].stats.cut_reach();
        // 1) cut the nearest weed mat ahead in this lane
        if let Some(w) = self
            .objs
            .iter_mut()
            .filter(|o| o.alive && o.kind == Kind::Weed && o.lane == lane && o.z >= z - 1.0 && o.z <= z + reach)
            .min_by(|a, b| a.z.total_cmp(&b.z))
        {
            w.alive = false;
            let id = w.id;
            let crew = self.boats[i].crew;
            self.boats[i].cuts += 1;
            self.crew_mut(crew).banks += 1;
            self.heal(HEALTH_CUT);
            self.events.push(Event::Cut { seat, id });
            return;
        }
        // 2) cut at the nearest ghost net ahead that covers this lane
        if let Some(oi) = self
            .objs
            .iter()
            .enumerate()
            .filter(|(_, o)| o.alive && o.kind == Kind::Net && covers(o, lane) && o.z >= z - 1.0 && o.z <= z + reach)
            .min_by(|a, b| a.1.z.total_cmp(&b.1.z))
            .map(|(i, _)| i)
        {
            let net = &mut self.objs[oi];
            net.hp = net.hp.saturating_sub(1);
            let (id, left, nl, nz) = (net.id, net.hp, net.lane, net.z);
            self.events.push(Event::NetHit { seat, id, left });
            if left == 0 {
                self.objs[oi].alive = false;
                let crew = self.boats[i].crew;
                self.crew_mut(crew).nets += 1;
                self.heal(HEALTH_CUT * 2.0);
                for k in 0..NET_PLASTIC {
                    self.spawn(Kind::Plastic, nl + (k % 2) as i8, nz + 4.0 + (k / 2) as f32 * 3.0);
                }
                self.events.push(Event::NetFreed { seat, id });
            }
            return;
        }
        // 3) start plugging a pipe beside this lane (only from an edge lane)
        let side = if lane == self.pipe_lane(-1) {
            -1
        } else if lane == self.pipe_lane(1) {
            1
        } else {
            return;
        };
        let pipe = self
            .objs
            .iter()
            .find(|o| o.kind == Kind::Pipe && !o.plugged && o.lane == side && z >= o.z - 7.0 && z <= o.z + 3.0)
            .map(|o| o.id);
        if let Some(pipe) = pipe {
            let plug = if self.boats.len() > 1 {
                // Crew lock: everyone's key, in seat order.
                if !self.deals.iter().any(|d| d.pipe == pipe) {
                    self.new_deal(pipe);
                }
                let deal = self.deals.iter().find(|d| d.pipe == pipe).expect("deal");
                let code = deal.keys.iter().map(|(_, k)| *k).collect();
                Plug { pipe, code, entered: Vec::new(), left: LOCK_TIME, team: true }
            } else {
                let code = (0..3).map(|_| self.rng.range(GLYPHS as u32) as u8).collect();
                Plug { pipe, code, entered: Vec::new(), left: PLUG_TIME, team: false }
            };
            self.boats[i].plug = Some(plug);
            self.events.push(Event::PlugStarted { seat, pipe });
        }
    }

    /// A seat's slot in a crew lock (seat order).
    pub fn slot_of(&self, seat: u8) -> Option<usize> {
        let mut seats: Vec<u8> = self.boats.iter().map(|b| b.seat).collect();
        seats.sort();
        seats.iter().position(|s| *s == seat)
    }

    /// Deal every player a different key for `pipe` (replacing any earlier deal).
    fn new_deal(&mut self, pipe: u32) {
        let mut pool: Vec<u8> = (0..GLYPHS).collect();
        for k in (1..pool.len()).rev() {
            let j = self.rng.range(k as u32 + 1) as usize;
            pool.swap(k, j);
        }
        let mut seats: Vec<u8> = self.boats.iter().map(|b| b.seat).collect();
        seats.sort();
        let keys: Vec<(u8, u8)> = seats.iter().zip(pool).map(|(s, k)| (*s, k)).collect();
        let answers = keys.iter().filter(|(s, _)| self.away.contains(s)).copied().collect();
        self.deals.retain(|d| d.pipe != pipe);
        self.deals.push(Deal { pipe, keys, shown: KEY_SHOW, answers });
        self.events.push(Event::KeysDealt { pipe });
    }

    /// Record a teammate's key for a pipe's lock (a later press replaces an earlier one).
    fn answer(&mut self, pipe: u32, seat: u8, glyph: u8) {
        if let Some(d) = self.deals.iter_mut().find(|d| d.pipe == pipe) {
            d.answers.retain(|(s, _)| *s != seat);
            d.answers.push((seat, glyph));
        }
    }

    /// Mark players as away (disconnected): their keys fill in by themselves.
    pub fn set_away(&mut self, seats: Vec<u8>) {
        self.away = seats;
    }

    /// Deal keys for pipes coming up, count down how long keys show, fill in away players.
    fn deal_keys(&mut self) {
        for d in &mut self.deals {
            d.shown = (d.shown - DT).max(0.0);
        }
        if self.boats.len() < 2 || self.tutorial {
            return;
        }
        let (front, last) = (self.leader_z(), self.last_z());
        let objs = &self.objs;
        self.deals.retain(|d| objs.iter().any(|o| o.id == d.pipe && !o.plugged && o.z > last - 40.0));
        // One pipe's keys at a time, so "your key" is never ambiguous: the next pipe's
        // keys come once the current pipe is fixed or everyone has passed it.
        if self.deals.is_empty() {
            let next = self
                .objs
                .iter()
                .filter(|o| o.kind == Kind::Pipe && !o.plugged && o.z > front - 3.0 && o.z <= front + KEY_DEAL_AT)
                .min_by(|a, b| a.z.total_cmp(&b.z))
                .map(|o| o.id);
            if let Some(pipe) = next {
                self.new_deal(pipe);
            }
        }
        let away = self.away.clone();
        for d in &mut self.deals {
            for (s, k) in d.keys.clone() {
                if away.contains(&s) && !d.answers.iter().any(|(a, _)| *a == s) {
                    d.answers.push((s, k));
                }
            }
        }
    }

    fn finish_plug(&mut self, i: usize, pipe: u32, ok: bool) {
        let seat = self.boats[i].seat;
        // Any other lock on this pipe closes: it was fixed, or the keys are about to change.
        for b in &mut self.boats {
            if b.plug.as_ref().is_some_and(|p| p.pipe == pipe) {
                b.plug = None;
            }
        }
        if ok {
            self.deals.retain(|d| d.pipe != pipe);
            if let Some(p) = self.objs.iter_mut().find(|o| o.id == pipe) {
                p.plugged = true;
            }
            let crew = self.boats[i].crew;
            self.boats[i].plugged += 1;
            self.boats[i].fixing = FIX_TIME;
            self.crew_mut(crew).pipes += 1;
            self.heal(HEALTH_PLUG);
            self.events.push(Event::Plugged { seat, pipe });
        } else {
            // The river takes a hit, and a crew gets new keys for its next try.
            self.heal(-LOCK_FAIL_COST);
            self.events.push(Event::PlugFailed { seat, pipe });
            if self.boats.len() > 1 {
                self.new_deal(pipe);
            }
        }
    }

    fn move_boats(&mut self) {
        let mut failed = Vec::new();
        let pace = self.pace();
        let front = self.leader_z();
        for (i, b) in self.boats.iter_mut().enumerate() {
            b.hop = (b.hop - DT).max(0.0);
            b.capsize = (b.capsize - DT).max(0.0);
            b.fixing = (b.fixing - DT).max(0.0);
            b.immune = (b.immune - DT).max(0.0);
            b.slow = (b.slow - DT).max(0.0);
            b.boosting = (b.boosting - DT).max(0.0);
            if let Some(p) = &mut b.plug {
                p.left -= DT;
                if p.left <= 0.0 {
                    failed.push((i, p.pipe));
                }
            }
            if b.boost >= 1.0 && b.boosting <= 0.0 && b.capsize <= 0.0 {
                b.boost = 0.0;
                b.boosting = BOOST_TIME;
                self.events.push(Event::Boost { seat: b.seat });
            }
            let catch_up = if b.z < front - CATCH_UP_GAP { CATCH_UP_MULT } else { 1.0 };
            b.z += b.speed() * pace * catch_up * DT;
        }
        for (i, pipe) in failed {
            self.boats[i].plug = None;
            self.finish_plug(i, pipe, false);
        }
        // Bumps: a faster boat running into the back of another in the same lane slows down.
        for a in 0..self.boats.len() {
            for b in 0..self.boats.len() {
                if a == b {
                    continue;
                }
                let (ba, bb) = (&self.boats[a], &self.boats[b]);
                let dz = bb.z - ba.z;
                if ba.lane == bb.lane && dz > 0.0 && dz < 1.1 && !ba.airborne() && !bb.airborne() && ba.slow <= 0.0 {
                    let by = bb.seat;
                    self.boats[a].slow = 0.4;
                    self.events.push(Event::Bumped { seat: self.boats[a].seat, by });
                }
            }
        }
    }

    fn collide(&mut self) {
        let open = self.lanes();
        let plastic_value = if self.twist == Some(Twist::VolunteerDay) { 2 } else { 1 };
        let last = self.last_z();
        for oi in 0..self.objs.len() {
            let o = self.objs[oi].clone();
            if !o.alive || o.kind == Kind::Pipe || (o.kind != Kind::Wave && !open.iter().any(|l| covers(&o, *l))) {
                continue;
            }
            for bi in 0..self.boats.len() {
                let b = &self.boats[bi];
                if o.kind == Kind::Plastic {
                    let lane_ok = o.lane == b.lane || (b.stats.wide_net() && (o.lane - b.lane).abs() == 1);
                    if (o.z - b.z).abs() > b.stats.net_window() || !lane_ok {
                        continue;
                    }
                } else if (o.z - b.z).abs() > HIT_WINDOW || !covers(&o, b.lane) {
                    continue;
                }
                let seat = b.seat;
                match o.kind {
                    Kind::Plastic => {
                        self.objs[oi].alive = false;
                        let crew = self.boats[bi].crew;
                        self.boats[bi].plastic += plastic_value;
                        self.boats[bi].collected += 1;
                        self.director.collected += 1;
                        self.boats[bi].boost = (self.boats[bi].boost + self.boats[bi].stats.boost_per_plastic()).min(1.0);
                        self.crew_mut(crew).plastic += plastic_value;
                        self.heal(HEALTH_PLASTIC);
                        self.events.push(Event::Collected { seat, id: o.id });
                        break;
                    }
                    Kind::Weed | Kind::Net => {
                        if !self.boats[bi].airborne() && self.boats[bi].slow <= 0.0 && self.boats[bi].boosting <= 0.0 {
                            self.boats[bi].slow = SLOW_TIME;
                            self.events.push(Event::Slowed { seat });
                        }
                    }
                    Kind::Log | Kind::Wave => {
                        if !self.boats[bi].airborne() && self.boats[bi].immune <= 0.0 {
                            self.capsize(bi);
                        }
                    }
                    Kind::Rock => {
                        if self.boats[bi].immune <= 0.0 {
                            self.capsize(bi);
                        }
                    }
                    Kind::Current => {
                        let b = &mut self.boats[bi];
                        if !b.airborne() && b.last_current != o.id && b.capsize <= 0.0 && b.plug.is_none() {
                            b.last_current = o.id;
                            let (min, max) = (*open.first().unwrap(), *open.last().unwrap());
                            let to = (b.lane + o.dir).clamp(min, max);
                            if to != b.lane {
                                b.lane = to;
                                self.events.push(Event::Pushed { seat, dir: o.dir });
                            }
                        }
                    }
                    Kind::Pipe => {}
                }
            }
            // Plastic that every boat has passed escapes downstream.
            let (alive, kind, z, id) = (self.objs[oi].alive, self.objs[oi].kind, self.objs[oi].z, self.objs[oi].id);
            if alive && kind == Kind::Plastic && z < last - 2.0 {
                self.objs[oi].alive = false;
                self.missed += 1;
                self.director.missed += 1;
                self.events.push(Event::Missed { id });
                self.heal(HEALTH_MISSED);
            }
        }
        // Old objects far behind are dropped to keep snapshots small.
        let cutoff = last - 40.0;
        self.objs.retain(|o| o.z > cutoff || (o.kind == Kind::Pipe && !o.plugged));
    }

    /// A short stop, then a moment of protection; the boat spills some of its
    /// plastic just ahead, spread over nearby lanes, where anyone can net it again.
    fn capsize(&mut self, bi: usize) {
        self.director.capsizes += 1;
        if self.endless {
            self.lives = self.lives.saturating_sub(1);
        }
        let (seat, crew, lane, z) = (self.boats[bi].seat, self.boats[bi].crew, self.boats[bi].lane, self.boats[bi].z);
        let b = &mut self.boats[bi];
        b.capsize = CAPSIZE_TIME;
        b.immune = CAPSIZE_TIME + IMMUNE_TIME;
        b.capsizes += 1;
        b.boosting = 0.0;
        b.plug = None;
        let dropped = CAPSIZE_DROP.min(b.plastic);
        b.plastic -= dropped;
        let c = self.crew_mut(crew);
        c.plastic = c.plastic.saturating_sub(dropped);
        let lanes = self.lanes();
        let (min, max) = (*lanes.first().unwrap(), *lanes.last().unwrap());
        for k in 0..dropped {
            let l = (lane + [0i8, -1, 1][k as usize % 3]).clamp(min, max);
            self.spawn(Kind::Plastic, l, z + 7.0 + k as f32 * 3.0);
        }
        self.events.push(Event::Capsized { seat, dropped });
    }

    /// River Director, in-run part: every 10 s compare the crew with the flow
    /// band and nudge the difficulty by a small step, never more than
    /// D_SWING away from where the room started. Single-crew runs only.
    fn direct(&mut self) {
        if !self.adaptive || self.tutorial {
            return;
        }
        let w = &mut self.director;
        w.timer += DT;
        if w.timer < DIRECTOR_EVERY {
            return;
        }
        let seen = w.collected + w.missed;
        let share = if seen > 0 { w.collected as f32 / seen as f32 } else { (FLOW_LOW + FLOW_HIGH) / 2.0 };
        let step = if w.capsizes >= 2 || share < FLOW_LOW {
            -D_STEP
        } else if w.capsizes == 0 && share > FLOW_HIGH {
            D_STEP
        } else {
            0.0
        };
        *w = DirectorWindow::default();
        let lo = (self.difficulty_start - D_SWING).max(D_MIN);
        let hi = (self.difficulty_start + D_SWING).min(D_MAX);
        self.difficulty = (self.difficulty + step).clamp(lo, hi);
    }

    fn pressure(&mut self) {
        let leader = self.leader_z();
        // Pipes pour murk once boats have reached them, until plugged.
        let mut rate = PIPE_DRAIN;
        if self.twist == Some(Twist::DrySeason) {
            rate *= 2.0;
        }
        if self.finale == Some(Finale::CleanSweep) {
            rate *= 0.5;
        }
        let active = self.objs.iter().filter(|o| o.kind == Kind::Pipe && !o.plugged && o.z <= leader + 30.0).count();
        self.heal(-(active as f32) * rate * DT);

        // Debris waves.
        if self.t >= self.next_wave {
            let lanes = self.lanes();
            let gap = self.rng.lane(&lanes);
            let id = self.spawn(Kind::Wave, gap, leader + 70.0);
            // One gap per 3 lanes: wide rivers get a second one, away from the first.
            if lanes.len() >= 6 {
                let others: Vec<i8> = lanes.iter().copied().filter(|l| (l - gap).abs() > 1).collect();
                let g2 = self.rng.lane(&others);
                if let Some(w) = self.objs.iter_mut().find(|o| o.id == id) {
                    w.gap2 = Some(g2);
                }
            }
            self.events.push(Event::WaveIncoming { id });
            let every = if self.twist == Some(Twist::MonsoonSurge) { 15.0 } else { 30.0 };
            self.next_wave = self.t + every / self.difficulty;
        }

        // Extra spawns from rule changes.
        self.spawn_timer -= DT;
        if self.spawn_timer <= 0.0 {
            self.spawn_timer = 0.75;
            let lanes = self.lanes();
            if self.twist == Some(Twist::MonsoonSurge) {
                let (pm, _) = spawn_mults(self.lane_count);
                let count = 1 + u32::from(self.rng.unit() < pm - 1.0);
                for _ in 0..count {
                    let l = self.rng.lane(&lanes);
                    self.spawn(Kind::Plastic, l, leader + 55.0);
                }
            }
            if self.twist == Some(Twist::VolunteerDay) && self.rng.range(4) == 0 {
                let l = self.rng.lane(&lanes);
                self.spawn(Kind::Weed, l, leader + 60.0);
            }
            if self.second_wind_left > 0.0 {
                if let Some(c) = self.finale_crew {
                    let targets: Vec<(i8, f32)> =
                        self.boats.iter().filter(|b| b.crew == c).map(|b| (b.lane, b.z)).collect();
                    for (lane, z) in targets {
                        self.spawn(Kind::Plastic, lane, z + 30.0);
                    }
                }
            }
        }
        self.second_wind_left = (self.second_wind_left - DT).max(0.0);
    }

    /// Endless score: metres travelled plus 5 per plastic collected.
    pub fn endless_score(&self) -> i32 {
        let plastic: u32 = self.boats.iter().map(|b| b.collected).sum();
        self.distance() as i32 + plastic as i32 * ENDLESS_PLASTIC_POINTS
    }

    fn finish(&mut self) {
        let revived = self.health >= REVIVED_AT;
        if self.endless {
            let scores = self.crews.iter().map(|c| (c.id, self.endless_score())).collect();
            self.results = Some(Results { scores, winner: None, health: self.health, choked: self.choked, revived });
            self.phase = Phase::Finished;
            return;
        }
        let mut scores: Vec<(u8, i32)> = self
            .crews
            .iter()
            .map(|c| {
                let mut s = c.score();
                if self.choked {
                    s /= 2;
                }
                if revived {
                    s += REVIVED_BONUS;
                }
                (c.id, s)
            })
            .collect();
        scores.sort_by_key(|(id, _)| *id);
        let winner = if scores.len() == 2 && scores[0].1 != scores[1].1 {
            Some(if scores[0].1 > scores[1].1 { scores[0].0 } else { scores[1].0 })
        } else {
            None
        };
        self.results = Some(Results { scores, winner, health: self.health, choked: self.choked, revived });
        self.phase = Phase::Finished;
    }

    /// The three stars this seat earned in a finished run: clean river, score
    /// target, and the river's own goal. Tutorials earn none.
    pub fn stars(&self, seat: u8) -> [bool; 3] {
        let (Some(res), Some(b)) = (&self.results, self.boats.iter().find(|b| b.seat == seat)) else {
            return [false; 3];
        };
        if self.tutorial || self.endless {
            return [false; 3];
        }
        let score = res.scores.iter().find(|(c, _)| *c == b.crew).map(|(_, s)| *s).unwrap_or(0);
        let crew = self.crews.iter().find(|c| c.id == b.crew);
        let goal = match self.river {
            // every pipe the boats reached is plugged (and there was at least one)
            River::Canal => {
                let leader = self.leader_z();
                let open = self.objs.iter().any(|o| o.kind == Kind::Pipe && !o.plugged && o.z <= leader);
                let plugged: u32 = self.crews.iter().map(|c| c.pipes).sum();
                !open && plugged > 0
            }
            River::Mangrove => crew.map(|c| c.nets).unwrap_or(0) >= 3,
            River::Gorge => b.capsizes == 0,
        };
        [res.revived, score >= self.river.score_target(), goal]
    }

    pub fn drain_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }
}

/// Whether an object occupies `lane` (nets span two lanes, waves all but their gaps).
pub fn covers(o: &Obj, lane: i8) -> bool {
    match o.kind {
        Kind::Wave => wave_covers(o, lane),
        Kind::Net => lane == o.lane || lane == o.lane + 1,
        _ => lane == o.lane,
    }
}

/// A wave blocks every lane except its gap lane(s).
pub fn wave_covers(wave: &Obj, lane: i8) -> bool {
    lane != wave.lane && Some(lane) != wave.gap2
}

pub fn name_of<T: Serialize>(v: &T) -> String {
    serde_json::to_value(v).ok().and_then(|x| x.as_str().map(String::from)).unwrap_or_default()
}

// ── snapshots for clients ────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize)]
pub struct BoatView {
    pub seat: u8,
    pub crew: u8,
    pub lane: i8,
    pub z: f32,
    pub speed: f32,
    pub hop: f32,
    pub capsize: f32,
    pub immune: f32,
    pub slow: f32,
    pub boost: f32,
    pub boosting: f32,
    pub plastic: u32,
    pub plugging: Option<u32>,
    /// Seconds left fixing a just-plugged pipe.
    pub fixing: f32,
    /// Pipes this boat has fixed this run (for the suggested fixer).
    pub pipes: u32,
}

#[derive(Clone, Debug, Serialize)]
pub struct PlugView {
    pub pipe: u32,
    /// Solo: the symbols as this viewer may see them. Crew locks: empty (see `slots`).
    pub code: Vec<u8>,
    /// A crew key lock.
    pub team: bool,
    /// Crew locks: one slot per player in seat order: (seat, answered, key). The key is
    /// shown to the fixer for teammates' slots, and to each player for their own answer.
    pub slots: Vec<(u8, bool, Option<u8>)>,
    /// Crew locks: slots the fixer has typed so far.
    pub typed: usize,
    pub clear: bool,
    /// No crewmate: a memory puzzle (shown clearly, then hidden).
    pub memory: bool,
    /// Memory puzzle, after the code has been hidden: `code` carries nothing.
    pub hidden: bool,
    pub entered: usize,
    pub left: f32,
}

#[derive(Clone, Debug, Serialize)]
pub struct Snapshot {
    pub tick: u64,
    pub river: River,
    pub endless: bool,
    /// Endless: lives the crew has left.
    pub lives: u8,
    /// Metres travelled by the leading boat (shown in Endless).
    pub distance: f32,
    pub duration: f32,
    pub tutorial: bool,
    pub t: f32,
    pub phase: Phase,
    pub health: f32,
    pub choked: bool,
    pub twist: Option<Twist>,
    pub finale: Option<Finale>,
    pub finale_crew: Option<u8>,
    pub changes: u8,
    pub votes: Vec<(u8, u8)>,
    pub lanes: Vec<i8>,
    /// Lanes in the full-width river (the open ones are `lanes`).
    pub lane_count: u8,
    pub boats: Vec<BoatView>,
    /// (crew, score, plastic, pipes, banks, ghost nets freed)
    pub crews: Vec<(u8, i32, u32, u32, u32, u32)>,
    pub objs: Vec<Obj>,
    /// Gate codes this viewer is involved in (own plug, or a crewmate's).
    pub plugs: Vec<(u8, PlugView)>,
    /// This viewer's own key for the pipe ahead, while it is being shown.
    pub my_key: Option<u8>,
    pub results: Option<Results>,
}

impl Run {
    /// What `viewer` may see. Gate codes are clear only to the plugging boat's
    /// crewmates (or briefly to a lone boat); objects are limited to the stretch near the boats.
    pub fn snapshot_for(&self, viewer: u8) -> Snapshot {
        let lo = self.last_z() - 20.0;
        let hi = self.leader_z() + 140.0;
        let me = self.boats.iter().find(|b| b.seat == viewer);
        let plugs = self
            .boats
            .iter()
            .filter_map(|b| {
                let p = b.plug.as_ref()?;
                let me = me?;
                if me.crew != b.crew {
                    return None;
                }
                let own = b.seat == viewer;
                if p.team {
                    let deal = self.deals.iter().find(|d| d.pipe == p.pipe);
                    let slots = deal
                        .map(|d| {
                            d.keys
                                .iter()
                                .map(|(s, _)| {
                                    let ans = d.answers.iter().find(|(a, _)| a == s).map(|(_, g)| *g);
                                    let visible = own || *s == viewer;
                                    (*s, ans.is_some(), if visible { ans } else { None })
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    return Some((
                        b.seat,
                        PlugView {
                            pipe: p.pipe,
                            code: Vec::new(),
                            team: true,
                            slots,
                            typed: p.entered.len(),
                            clear: false,
                            memory: false,
                            hidden: false,
                            entered: p.entered.len(),
                            left: p.left,
                        },
                    ));
                }
                let memory = own;
                let clear = !own || p.left > PLUG_TIME - MEMORY_TIME;
                let hidden = memory && !clear;
                Some((
                    b.seat,
                    PlugView {
                        pipe: p.pipe,
                        code: if hidden { vec![0; 3] } else { p.code.clone() },
                        team: false,
                        slots: Vec::new(),
                        typed: 0,
                        clear,
                        memory,
                        hidden,
                        entered: p.entered.len(),
                        left: p.left,
                    },
                ))
            })
            .collect();
        Snapshot {
            tick: self.tick,
            t: self.t,
            phase: self.phase.clone(),
            health: self.health,
            choked: self.choked,
            twist: self.twist,
            finale: self.finale,
            finale_crew: self.finale_crew,
            changes: self.changes,
            votes: self.votes.clone(),
            lanes: self.lanes(),
            lane_count: self.lane_count,
            boats: self
                .boats
                .iter()
                .map(|b| BoatView {
                    seat: b.seat,
                    crew: b.crew,
                    lane: b.lane,
                    z: b.z,
                    speed: b.speed() * self.pace(),
                    hop: b.hop,
                    capsize: b.capsize,
                    immune: b.immune,
                    slow: b.slow,
                    boost: b.boost,
                    boosting: b.boosting,
                    plastic: b.plastic,
                    plugging: b.plug.as_ref().map(|p| p.pipe),
                    fixing: b.fixing,
                    pipes: b.plugged,
                })
                .collect(),
            crews: self.crews.iter().map(|c| (c.id, c.score(), c.plastic, c.pipes, c.banks, c.nets)).collect(),
            objs: self
                .objs
                .iter()
                .filter(|o| (o.alive || o.kind == Kind::Pipe) && o.z >= lo && o.z <= hi)
                .cloned()
                .collect(),
            plugs,
            my_key: self
                .deals
                .iter()
                .filter(|d| d.shown > 0.0)
                .find_map(|d| d.keys.iter().find(|(s, _)| *s == viewer).map(|(_, k)| *k)),
            // Endless has no time limit (0 here, since JSON can't carry infinity).
            duration: if self.endless { 0.0 } else { self.duration() },
            tutorial: self.tutorial,
            river: self.river,
            endless: self.endless,
            lives: self.lives,
            distance: self.distance(),
            results: self.results.clone(),
        }
    }
}

#[cfg(test)]
mod tests;
