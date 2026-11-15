//! Daily missions and ranks.
//!
//! Missions: three a day per player (UTC), drawn from a fixed pool with a
//! per-player, per-day seed, so everyone gets a varied but stable set. Progress
//! counts across runs and modes (practice too, never the tutorial), and each
//! mission pays its Seeds once, automatically, when it is completed.
//!
//! Ranks: the River Director's skill rating shown as a name. The rating itself
//! is never shown. A shown rank can rise at any time but falls by at most one
//! step per UTC day, so one bad evening can't wipe out a week of progress.

use river_core::Run;
use serde_json::{json, Value};
use sqlx::{PgPool, Postgres, Transaction};
use std::collections::HashMap;
use uuid::Uuid;

// ── missions ─────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Metric {
    Plastic,
    Pipes,
    Weed,
    Runs,
    Revived,
    CrewWin,
    Star,
    NoCapsize,
    Score100,
}

pub struct Mission {
    pub key: &'static str,
    pub text: &'static str,
    pub metric: Metric,
    pub goal: i32,
    pub reward: i32,
}

pub const POOL: &[Mission] = &[
    Mission { key: "plastic_60", text: "Collect 60 plastic", metric: Metric::Plastic, goal: 60, reward: 20 },
    Mission { key: "plastic_150", text: "Collect 150 plastic", metric: Metric::Plastic, goal: 150, reward: 35 },
    Mission { key: "pipes_2", text: "Plug 2 sewage pipes", metric: Metric::Pipes, goal: 2, reward: 25 },
    Mission { key: "weed_6", text: "Cut 6 weed mats", metric: Metric::Weed, goal: 6, reward: 20 },
    Mission { key: "runs_3", text: "Finish 3 runs", metric: Metric::Runs, goal: 3, reward: 20 },
    Mission { key: "revive_1", text: "Finish a run with the river revived", metric: Metric::Revived, goal: 1, reward: 25 },
    Mission { key: "win_1", text: "Win a crew race", metric: Metric::CrewWin, goal: 1, reward: 40 },
    Mission { key: "star_1", text: "Earn a star on any river", metric: Metric::Star, goal: 1, reward: 30 },
    Mission { key: "clean_1", text: "Finish a run without capsizing", metric: Metric::NoCapsize, goal: 1, reward: 30 },
    Mission { key: "score_100", text: "Reach a crew score of 100 in one run", metric: Metric::Score100, goal: 1, reward: 25 },
];

pub fn mission(key: &str) -> Option<&'static Mission> {
    POOL.iter().find(|m| m.key == key)
}

/// Three different missions for this player and day.
pub fn draw(player: Uuid, day: i32) -> [&'static Mission; 3] {
    let mut x = u64::from_le_bytes(player.as_bytes()[..8].try_into().expect("8 bytes")) ^ (day as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
    let mut next = || {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        (x >> 33) as usize
    };
    let mut picks: Vec<usize> = Vec::new();
    while picks.len() < 3 {
        let i = next() % POOL.len();
        // at most one plastic mission a day
        let plastic_twice = POOL[i].metric == Metric::Plastic && picks.iter().any(|p| POOL[*p].metric == Metric::Plastic);
        if !picks.contains(&i) && !plastic_twice {
            picks.push(i);
        }
    }
    [&POOL[picks[0]], &POOL[picks[1]], &POOL[picks[2]]]
}

/// What a finished run adds to a mission for this seat.
pub fn amount(metric: Metric, run: &Run, seat: u8) -> i32 {
    let (Some(res), Some(b)) = (&run.results, run.boats.iter().find(|b| b.seat == seat)) else { return 0 };
    let score = res.scores.iter().find(|(c, _)| *c == b.crew).map(|(_, s)| *s).unwrap_or(0);
    match metric {
        Metric::Plastic => b.collected as i32,
        Metric::Pipes => b.plugged as i32,
        Metric::Weed => b.cuts as i32,
        Metric::Runs => 1,
        Metric::Revived => i32::from(res.revived),
        Metric::CrewWin => i32::from(res.scores.len() > 1 && res.winner == Some(b.crew)),
        Metric::Star => i32::from(run.stars(seat).iter().any(|s| *s)),
        Metric::NoCapsize => i32::from(b.capsizes == 0),
        Metric::Score100 => i32::from(score >= 100),
    }
}

type Row = (i16, String, i32, i32, i32, bool);

/// Today's three missions for a player, creating them on first use.
async fn today(tx: &mut Transaction<'_, Postgres>, pid: Uuid) -> sqlx::Result<Vec<Row>> {
    let day: i32 = sqlx::query_scalar("SELECT ((now() AT TIME ZONE 'utc')::date - DATE '2026-01-01')::int")
        .fetch_one(&mut **tx)
        .await?;
    for (slot, m) in draw(pid, day).iter().enumerate() {
        sqlx::query(
            "INSERT INTO player_missions (player_id, day, slot, mission, goal, reward)
             VALUES ($1, (now() AT TIME ZONE 'utc')::date, $2, $3, $4, $5) ON CONFLICT DO NOTHING",
        )
        .bind(pid)
        .bind(slot as i16)
        .bind(m.key)
        .bind(m.goal)
        .bind(m.reward)
        .execute(&mut **tx)
        .await?;
    }
    sqlx::query_as(
        "SELECT slot, mission, progress, goal, reward, completed_at IS NOT NULL FROM player_missions
         WHERE player_id = $1 AND day = (now() AT TIME ZONE 'utc')::date ORDER BY slot FOR UPDATE",
    )
    .bind(pid)
    .fetch_all(&mut **tx)
    .await
}

fn view(r: &Row, just_done: bool) -> Value {
    let text = mission(&r.1).map(|m| m.text).unwrap_or("Mission");
    json!({ "key": r.1, "text": text, "progress": r.2, "goal": r.3, "reward": r.4, "done": r.5, "just_done": just_done })
}

/// The hub's mission list.
pub async fn list(db: &PgPool, pid: Uuid) -> sqlx::Result<Value> {
    let mut tx = db.begin().await?;
    let rows = today(&mut tx, pid).await?;
    tx.commit().await?;
    Ok(Value::Array(rows.iter().map(|r| view(r, false)).collect()))
}

/// Add a finished run to each player's missions and pay any just completed.
/// Returns, per seat, the missions as the results screen shows them.
pub async fn record(db: &PgPool, run: &Run, players: &[(u8, Uuid)]) -> sqlx::Result<HashMap<u8, Value>> {
    let mut out = HashMap::new();
    for &(seat, pid) in players {
        let mut tx = db.begin().await?;
        let mut rows = today(&mut tx, pid).await?;
        let mut shown = Vec::new();
        for r in &mut rows {
            let mut just = false;
            if !r.5 {
                if let Some(m) = mission(&r.1) {
                    let add = amount(m.metric, run, seat);
                    r.2 = (r.2 + add).min(r.3);
                    just = r.2 >= r.3;
                    sqlx::query(
                        "UPDATE player_missions SET progress = $4, completed_at = CASE WHEN $5 THEN now() ELSE NULL END
                         WHERE player_id = $1 AND day = (now() AT TIME ZONE 'utc')::date AND slot = $2 AND mission = $3",
                    )
                    .bind(pid)
                    .bind(r.0)
                    .bind(&r.1)
                    .bind(r.2)
                    .bind(just)
                    .execute(&mut *tx)
                    .await?;
                    if just {
                        r.5 = true;
                        sqlx::query("UPDATE player_wallets SET seeds = seeds + $2, updated_at = now() WHERE player_id = $1")
                            .bind(pid)
                            .bind(r.4)
                            .execute(&mut *tx)
                            .await?;
                    }
                }
            }
            shown.push(view(r, just));
        }
        tx.commit().await?;
        out.insert(seat, Value::Array(shown));
    }
    Ok(out)
}

// ── ranks ────────────────────────────────────────────────────────────────────

pub const RANKS: [&str; 5] = ["Rookie", "Deckhand", "Skipper", "Captain", "River Guardian"];
/// Rating at which each rank above Rookie starts.
pub const RANK_AT: [f32; 4] = [900.0, 1050.0, 1200.0, 1350.0];

pub fn rank_for(rating: f32) -> i16 {
    RANK_AT.iter().filter(|t| rating >= **t).count() as i16
}

/// The rank to show after a rating change: up at once, down at most one step
/// below the rank the day started with.
pub fn shown_rank(computed: i16, day_start: i16) -> i16 {
    computed.max(day_start - 1).clamp(0, 4)
}

/// Progress (0–1) through the shown rank towards the next one.
pub fn rank_progress(rating: f32, shown: i16) -> f32 {
    if shown >= 4 {
        return 1.0;
    }
    let lo = if shown == 0 { 750.0 } else { RANK_AT[shown as usize - 1] };
    let hi = RANK_AT[shown as usize];
    ((rating - lo) / (hi - lo)).clamp(0.0, 1.0)
}

pub fn rank_view(rating: f32, shown: i16, before: Option<i16>) -> Value {
    json!({
        "name": RANKS[shown as usize], "index": shown,
        "next": RANKS.get(shown as usize + 1),
        "progress": (rank_progress(rating, shown) * 100.0).round() / 100.0,
        "change": before.map(|b| (shown - b).signum()).unwrap_or(0),
    })
}

/// The hub's rank badge.
pub async fn rank(db: &PgPool, pid: Uuid) -> sqlx::Result<Value> {
    let row: Option<(f32, i16)> = sqlx::query_as("SELECT rating, shown_rank FROM player_skill WHERE player_id = $1")
        .bind(pid)
        .fetch_optional(db)
        .await?;
    let (rating, shown) = row.unwrap_or((crate::director::START_RATING, rank_for(crate::director::START_RATING)));
    Ok(rank_view(rating, shown, None))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_different_missions_a_day_stable_for_the_day() {
        let p = Uuid::from_u128(0x1234_5678_9abc_def0_1111_2222_3333_4444);
        for day in 0..200 {
            let d = draw(p, day);
            assert!(d[0].key != d[1].key && d[1].key != d[2].key && d[0].key != d[2].key);
            assert!(d.iter().filter(|m| m.metric == Metric::Plastic).count() <= 1);
            assert_eq!(d.map(|m| m.key), draw(p, day).map(|m| m.key));
        }
        assert_ne!(
            (0..10).map(|d| draw(p, d)[0].key).collect::<Vec<_>>(),
            (0..10).map(|d| draw(p, d + 1)[0].key).collect::<Vec<_>>()
        );
        assert!(POOL.iter().all(|m| (20..=40).contains(&m.reward)));
    }

    #[test]
    fn ranks_rise_freely_but_fall_one_step_a_day() {
        assert_eq!(rank_for(899.0), 0);
        assert_eq!(rank_for(1000.0), 1);
        assert_eq!(rank_for(1400.0), 4);
        assert_eq!(shown_rank(3, 1), 3, "up at once");
        assert_eq!(shown_rank(0, 3), 2, "down at most one step");
        assert_eq!(shown_rank(1, 2), 1);
        assert!((rank_progress(975.0, 1) - 0.5).abs() < 1e-6);
        assert_eq!(rank_progress(2000.0, 4), 1.0);
    }

    #[test]
    fn mission_amounts_come_from_the_run() {
        let mut r = Run::new(1, &[0]);
        r.phase = river_core::Phase::Running;
        r.t = r.duration();
        r.boats[0].collected = 12;
        r.boats[0].cuts = 2;
        r.boats[0].capsizes = 0;
        r.crews[0].plastic = 120;
        r.health = 80.0;
        r.step(&[(0, Default::default())]);
        assert_eq!(amount(Metric::Plastic, &r, 0), 12);
        assert_eq!(amount(Metric::Weed, &r, 0), 2);
        assert_eq!(amount(Metric::Revived, &r, 0), 1);
        assert_eq!(amount(Metric::Score100, &r, 0), 1);
        assert_eq!(amount(Metric::CrewWin, &r, 0), 0, "no crew race when solo");
        assert_eq!(amount(Metric::Runs, &r, 7), 0, "unknown seat");
    }
}
