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
    pub code: [u8; 3],
    pub entered: Vec<u8>,
    pub left: f32,
}

impl Plug {
    /// What the plugging boat sees: the right symbols in the wrong order.
    pub fn scrambled(&self) -> [u8; 3] {
        [self.code[1], self.code[2], self.code[0]]
    }
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
}

impl Boat {
    pub fn airborne(&self) -> bool {
        self.hop > 0.0
    }
    pub fn speed(&self) -> f32 {
        if self.capsize > 0.0 || self.plug.is_some() {
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
    /// Paused: players choose. Twist = all vote; Finale = `pickers` crew chooses.
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
}

pub const TUTORIAL_SECONDS: f32 = 60.0;

impl Run {
    /// Seats are assigned to crews alternately (0,1,0,1). Two players = one crew.
    pub fn new(seed: u64, seats: &[u8]) -> Run {
        let two_crews = seats.len() >= 3;
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
                crew: if two_crews { (i % 2) as u8 } else { 0 },
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
            })
            .collect();
        let crews = (0..if two_crews { 2 } else { 1 }).map(|id| Crew { id, ..Default::default() }).collect();
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
        };
        run.generate_course();
        run
    }

    /// A short hand-built course that teaches one control at a time:
    /// plastic lines → logs to hop → weed to cut → a pipe to plug → open water.
    pub fn tutorial(seed: u64, seats: &[u8]) -> Run {
        let mut run = Run::new(seed, seats);
        run.tutorial = true;
        run.objs.clear();
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

    pub fn duration(&self) -> f32 {
        if self.tutorial {
            TUTORIAL_SECONDS
        } else {
            RUN_SECONDS
        }
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
        self.objs.push(Obj { id, kind, lane, z, alive: true, plugged: false, gap2: None });
        id
    }

    /// Hand-shaped random patterns along the whole course.
    /// Wider rivers get denser rows (the obstacle multiplier) plus extra loose
    /// plastic (to reach the plastic multiplier).
    fn generate_course(&mut self) {
        let n = self.lane_count as i8;
        let all: Vec<i8> = (0..n).collect();
        let (plastic_mult, obstacle_mult) = spawn_mults(self.lane_count);
        // A row holds about 2.6 plastic on average; top it up so plastic grows faster than obstacles.
        let extra_plastic = (plastic_mult / obstacle_mult - 1.0) * 2.6;
        let mut z = 45.0;
        let mut next_pipe = 120.0;
        while z < COURSE_LEN {
            match self.rng.range(10) {
                0..=3 => {
                    // a line of plastic to follow
                    let lane = self.rng.lane(&all);
                    for k in 0..(3 + self.rng.range(3)) {
                        self.spawn(Kind::Plastic, lane, z + k as f32 * 3.0);
                    }
                }
                4..=5 => {
                    // weed mat guarding plastic in the same lane
                    let lane = self.rng.lane(&all);
                    self.spawn(Kind::Weed, lane, z);
                    for k in 0..3 {
                        self.spawn(Kind::Plastic, lane, z + 4.0 + k as f32 * 3.0);
                    }
                }
                6..=7 => {
                    // a run of logs across 1-2 lanes (up to 3 on a wide river), plastic beside it
                    let extra = if n >= 6 { self.rng.range(2) as i8 } else { 0 };
                    let c = (1 + self.rng.range(2) as i8 + extra).min(n - 1);
                    let start = self.rng.range((n - c + 1) as u32) as i8;
                    for l in start..start + c {
                        self.spawn(Kind::Log, l, z);
                    }
                    let left = start > 0 && (start + c >= n || self.rng.range(2) == 0);
                    let reward = if left { start - 1 } else { start + c };
                    self.spawn(Kind::Plastic, reward, z);
                }
                _ => {
                    // scattered plastic
                    for _ in 0..2 {
                        let lane = self.rng.lane(&all);
                        let dz = self.rng.unit() * 8.0;
                        self.spawn(Kind::Plastic, lane, z + dz);
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
                next_pipe += 190.0 + self.rng.unit() * 60.0;
            }
            z += (16.0 + self.rng.unit() * 10.0) / obstacle_mult;
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
        let Some(boat) = self.boats.iter().find(|b| b.seat == seat) else { return Err("not in this run") };
        if let Some(c) = pickers {
            if boat.crew != c {
                return Err("only the trailing crew picks the Finale");
            }
            // First pick from the picking crew decides.
            self.resolve_finale(Some(option as usize), false);
            return Ok(());
        }
        self.votes.retain(|(s, _)| *s != seat);
        self.votes.push((seat, option));
        if self.votes.len() == self.boats.len() {
            self.resolve_twist();
        }
        Ok(())
    }

    fn resolve_twist(&mut self) {
        let mut counts = [0u32; 3];
        for (_, o) in &self.votes {
            counts[*o as usize] += 1;
        }
        let best = *counts.iter().max().unwrap();
        let tied: Vec<usize> = (0..3).filter(|i| counts[*i] == best).collect();
        let spin = tied.len() > 1;
        let pick = tied[self.rng.range(tied.len() as u32) as usize];
        let twist = TWISTS[pick];
        self.twist = Some(twist);
        self.changes += 1;
        if twist == Twist::MonsoonSurge {
            self.next_wave = self.next_wave.min(self.t + 15.0);
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

    /// The crew that picks the Finale: the trailing one (a tie is a visible spin).
    fn trailing_crew(&mut self) -> u8 {
        if self.crews.len() == 1 {
            return self.crews[0].id;
        }
        let (a, b) = (self.crews[0].score(), self.crews[1].score());
        if a < b {
            self.crews[0].id
        } else if b < a {
            self.crews[1].id
        } else {
            self.crews[self.rng.range(2) as usize].id
        }
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
                        ChangeKind::Finale => self.resolve_finale(None, true),
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
                    let pickers = if change == ChangeKind::Finale {
                        let c = self.trailing_crew();
                        self.finale_crew = Some(c);
                        Some(c)
                    } else {
                        None
                    };
                    self.votes.clear();
                    let left = if change == ChangeKind::Twist { TWIST_VOTE } else { FINALE_PICK };
                    self.phase = Phase::Choosing { change, left, pickers };
                    return;
                }
                self.phase = Phase::Warning { change, left: left - DT };
            }
            Phase::Running => {
                if self.tutorial {
                    // no rule changes in the tutorial
                } else if self.changes == 0 && self.t + WARNING >= TWIST_AT {
                    self.phase = Phase::Warning { change: ChangeKind::Twist, left: TWIST_AT - self.t };
                } else if self.changes == 1 && self.twist.is_some() && self.t + WARNING >= FINALE_AT {
                    self.phase = Phase::Warning { change: ChangeKind::Finale, left: FINALE_AT - self.t };
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
        if self.t >= self.duration() {
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
                if plug.entered.len() == 3 {
                    let ok = plug.entered == plug.code;
                    let pipe = plug.pipe;
                    self.boats[i].plug = None;
                    self.finish_plug(i, pipe, ok);
                }
            } else {
                self.events.push(Event::Ping { seat, glyph: g });
            }
        }
        let b = &mut self.boats[i];
        if b.capsize > 0.0 || b.plug.is_some() {
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
            self.crew_mut(crew).banks += 1;
            self.heal(HEALTH_CUT);
            self.events.push(Event::Cut { seat, id });
            return;
        }
        // 2) start plugging a pipe beside this lane (only from an edge lane)
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
            let code = [
                self.rng.range(GLYPHS as u32) as u8,
                self.rng.range(GLYPHS as u32) as u8,
                self.rng.range(GLYPHS as u32) as u8,
            ];
            self.boats[i].plug = Some(Plug { pipe, code, entered: Vec::new(), left: PLUG_TIME });
            self.events.push(Event::PlugStarted { seat, pipe });
        }
    }

    fn finish_plug(&mut self, i: usize, pipe: u32, ok: bool) {
        let seat = self.boats[i].seat;
        if ok {
            if let Some(p) = self.objs.iter_mut().find(|o| o.id == pipe) {
                p.plugged = true;
            }
            let crew = self.boats[i].crew;
            self.crew_mut(crew).pipes += 1;
            self.heal(HEALTH_PLUG);
            self.events.push(Event::Plugged { seat, pipe });
        } else {
            self.events.push(Event::PlugFailed { seat, pipe });
        }
    }

    fn move_boats(&mut self) {
        let mut failed = Vec::new();
        for (i, b) in self.boats.iter_mut().enumerate() {
            b.hop = (b.hop - DT).max(0.0);
            b.capsize = (b.capsize - DT).max(0.0);
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
            b.z += b.speed() * DT;
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
            if !o.alive || o.kind == Kind::Pipe || (o.kind != Kind::Wave && !open.contains(&o.lane)) {
                continue;
            }
            for bi in 0..self.boats.len() {
                let b = &self.boats[bi];
                if o.kind == Kind::Plastic {
                    let lane_ok = o.lane == b.lane || (b.stats.wide_net() && (o.lane - b.lane).abs() == 1);
                    if (o.z - b.z).abs() > b.stats.net_window() || !lane_ok {
                        continue;
                    }
                } else if (o.z - b.z).abs() > HIT_WINDOW || (o.kind != Kind::Wave && o.lane != b.lane) {
                    continue;
                }
                if o.kind == Kind::Wave && !wave_covers(&o, b.lane) {
                    continue;
                }
                let seat = b.seat;
                match o.kind {
                    Kind::Plastic => {
                        self.objs[oi].alive = false;
                        let crew = self.boats[bi].crew;
                        self.boats[bi].plastic += plastic_value;
                        self.boats[bi].boost = (self.boats[bi].boost + self.boats[bi].stats.boost_per_plastic()).min(1.0);
                        self.crew_mut(crew).plastic += plastic_value;
                        self.heal(HEALTH_PLASTIC);
                        self.events.push(Event::Collected { seat, id: o.id });
                        break;
                    }
                    Kind::Weed => {
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
                    Kind::Pipe => {}
                }
            }
            // Plastic that every boat has passed escapes downstream.
            let (alive, kind, z, id) = (self.objs[oi].alive, self.objs[oi].kind, self.objs[oi].z, self.objs[oi].id);
            if alive && kind == Kind::Plastic && z < last - 2.0 {
                self.objs[oi].alive = false;
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
        let (seat, crew, lane, z) = (self.boats[bi].seat, self.boats[bi].crew, self.boats[bi].lane, self.boats[bi].z);
        let b = &mut self.boats[bi];
        b.capsize = CAPSIZE_TIME;
        b.immune = CAPSIZE_TIME + IMMUNE_TIME;
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
            self.next_wave = self.t + if self.twist == Some(Twist::MonsoonSurge) { 15.0 } else { 30.0 };
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

    fn finish(&mut self) {
        let revived = self.health >= REVIVED_AT;
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

    pub fn drain_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
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
}

#[derive(Clone, Debug, Serialize)]
pub struct PlugView {
    pub pipe: u32,
    /// The symbols as this viewer may see them.
    pub code: [u8; 3],
    pub clear: bool,
    pub entered: usize,
    pub left: f32,
}

#[derive(Clone, Debug, Serialize)]
pub struct Snapshot {
    pub tick: u64,
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
    pub crews: Vec<(u8, i32, u32, u32, u32)>,
    pub objs: Vec<Obj>,
    /// Gate codes this viewer is involved in (own plug, or a crewmate's).
    pub plugs: Vec<(u8, PlugView)>,
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
                let alone = self.crew_size(b.crew) == 1;
                let own = b.seat == viewer;
                let clear = !own || (alone && p.left > PLUG_TIME - 1.5);
                Some((
                    b.seat,
                    PlugView {
                        pipe: p.pipe,
                        code: if clear { p.code } else { p.scrambled() },
                        clear,
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
                    speed: b.speed(),
                    hop: b.hop,
                    capsize: b.capsize,
                    immune: b.immune,
                    slow: b.slow,
                    boost: b.boost,
                    boosting: b.boosting,
                    plastic: b.plastic,
                    plugging: b.plug.as_ref().map(|p| p.pipe),
                })
                .collect(),
            crews: self.crews.iter().map(|c| (c.id, c.score(), c.plastic, c.pipes, c.banks)).collect(),
            objs: self
                .objs
                .iter()
                .filter(|o| (o.alive || o.kind == Kind::Pipe) && o.z >= lo && o.z <= hi)
                .cloned()
                .collect(),
            plugs,
            duration: self.duration(),
            tutorial: self.tutorial,
            results: self.results.clone(),
        }
    }
}

#[cfg(test)]
mod tests;
