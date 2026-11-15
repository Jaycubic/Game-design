//! Rivers: unlocks, stars and best scores per player.
//!
//! A river unlocks with runs completed or stars earned, whichever comes first.
//! Stars (clean river, score target, the river's goal) are scored by
//! river-core and kept forever once earned; best_score is the best crew score.

use river_core::{River, Run, RIVERS};
use serde_json::{json, Value};
use sqlx::PgPool;
use std::collections::HashMap;
use uuid::Uuid;

pub struct RiverInfo {
    pub river: River,
    pub name: &'static str,
    pub line: &'static str,
    /// The third star's goal, in words.
    pub goal: &'static str,
    /// Unlocks after this many runs, or this many stars in total.
    pub runs: i32,
    pub stars: i32,
}

pub const INFO: [RiverInfo; 3] = [
    RiverInfo {
        river: River::Canal,
        name: "Town Canal",
        line: "Where it all starts: plastic, weed, logs and sewage pipes.",
        goal: "Plug every pipe you pass",
        runs: 0,
        stars: 0,
    },
    RiverInfo {
        river: River::Mangrove,
        name: "Mangrove Maze",
        line: "Ghost nets span two lanes. Cut them twice (or with a crewmate) to free the plastic inside.",
        goal: "Free 3 ghost nets",
        runs: 3,
        stars: 3,
    },
    RiverInfo {
        river: River::Gorge,
        name: "Rapids Gorge",
        line: "Currents push you sideways unless you hop, and rocks can't be hopped.",
        goal: "Finish without capsizing",
        runs: 8,
        stars: 7,
    },
];

pub fn info(river: River) -> &'static RiverInfo {
    INFO.iter().find(|i| i.river == river).expect("every river has info")
}

fn bits(stars: [bool; 3]) -> i16 {
    stars.iter().enumerate().filter(|(_, s)| **s).map(|(i, _)| 1i16 << i).sum()
}

fn unpack(mask: i16) -> [bool; 3] {
    [mask & 1 != 0, mask & 2 != 0, mask & 4 != 0]
}

/// (runs completed, total stars, per river: (stars mask, best score)).
async fn progress(db: &PgPool, pid: Uuid) -> sqlx::Result<(i32, i32, HashMap<String, (i16, i32)>)> {
    let runs: Option<i32> = sqlx::query_scalar("SELECT runs FROM player_wallets WHERE player_id = $1")
        .bind(pid)
        .fetch_optional(db)
        .await?;
    let rows: Vec<(String, i16, i32)> =
        sqlx::query_as("SELECT river, stars, best_score FROM player_rivers WHERE player_id = $1")
            .bind(pid)
            .fetch_all(db)
            .await?;
    let total = rows.iter().map(|(_, m, _)| m.count_ones() as i32).sum();
    Ok((runs.unwrap_or(0), total, rows.into_iter().map(|(r, m, b)| (r, (m, b))).collect()))
}

fn is_unlocked(i: &RiverInfo, runs: i32, stars: i32) -> bool {
    runs >= i.runs || stars >= i.stars
}

/// Whether this player may choose this river.
pub async fn unlocked(db: &PgPool, pid: Uuid, river: River) -> sqlx::Result<bool> {
    let (runs, stars, _) = progress(db, pid).await?;
    Ok(is_unlocked(info(river), runs, stars))
}

/// The river cards for the hub.
pub async fn profile(db: &PgPool, pid: Uuid) -> sqlx::Result<Value> {
    let (runs, total, rows) = progress(db, pid).await?;
    let cards: Vec<Value> = RIVERS
        .iter()
        .map(|&r| {
            let i = info(r);
            let (mask, best) = rows.get(r.key()).copied().unwrap_or((0, 0));
            json!({
                "key": r.key(), "name": i.name, "line": i.line,
                "goals": ["Keep River Health at 60 % or more", format!("Crew score {} or more", r.score_target()), i.goal],
                "stars": unpack(mask), "best": best,
                "unlocked": is_unlocked(i, runs, total), "need_runs": i.runs, "need_stars": i.stars,
            })
        })
        .collect();
    Ok(json!({ "stars": total, "rivers": cards }))
}

/// Store stars and best scores for a finished run. Returns, per seat, what the
/// results screen shows: stars this run, which are new, and the best score.
pub async fn record(db: &PgPool, run: &Run, players: &[(u8, Uuid)]) -> sqlx::Result<HashMap<u8, Value>> {
    let mut out = HashMap::new();
    let Some(res) = &run.results else { return Ok(out) };
    let key = run.river.key();
    for &(seat, player) in players {
        let Some(b) = run.boats.iter().find(|b| b.seat == seat) else { continue };
        let score = res.scores.iter().find(|(c, _)| *c == b.crew).map(|(_, s)| *s).unwrap_or(0);
        let stars = run.stars(seat);
        let mut tx = db.begin().await?;
        let before: Option<(i16, i32)> =
            sqlx::query_as("SELECT stars, best_score FROM player_rivers WHERE player_id = $1 AND river = $2 FOR UPDATE")
                .bind(player)
                .bind(key)
                .fetch_optional(&mut *tx)
                .await?;
        let (old_mask, old_best) = before.unwrap_or((0, 0));
        let mask = old_mask | bits(stars);
        let best = old_best.max(score);
        sqlx::query(
            "INSERT INTO player_rivers (player_id, river, stars, best_score, runs, updated_at) VALUES ($1, $2, $3, $4, 1, now())
             ON CONFLICT (player_id, river) DO UPDATE SET stars = $3, best_score = $4, runs = player_rivers.runs + 1, updated_at = now()",
        )
        .bind(player)
        .bind(key)
        .bind(mask)
        .bind(best)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        out.insert(
            seat,
            json!({
                "river": key, "name": info(run.river).name, "goals": [
                    "River Health 60 % or more", format!("Crew score {}+", run.river.score_target()), info(run.river).goal],
                "stars": stars, "new_stars": unpack(mask & !old_mask), "kept": unpack(mask),
                "score": score, "best": best, "new_best": score > old_best && score > 0,
            }),
        );
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rivers_unlock_by_runs_or_stars() {
        let gorge = info(River::Gorge);
        assert!(!is_unlocked(gorge, 7, 6));
        assert!(is_unlocked(gorge, 8, 0));
        assert!(is_unlocked(gorge, 0, 7));
        assert!(is_unlocked(info(River::Canal), 0, 0));
    }

    #[test]
    fn star_masks_round_trip() {
        assert_eq!(bits([true, false, true]), 5);
        assert_eq!(unpack(5), [true, false, true]);
        assert_eq!(unpack(bits([false, true, false])), [false, true, false]);
    }
}
