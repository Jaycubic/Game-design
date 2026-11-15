//! River Director, server part: learns each player's skill and sets the
//! starting difficulty of every room. (The in-run part lives in river-core.)
//!
//! Internal design feature, documented in docs/GAME_DESIGN.md and the design
//! spec only; it is deliberately not described in the rule book or the UI.
//!
//! - Skill: one rating per player (starts at 1000). After each finished run
//!   (not the tutorial) the player's performance is scored on the same scale
//!   and the rating moves part of the way towards it: a lot while there are
//!   few runs, less as confidence grows (online Elo/Glicko-style learning).
//! - Room difficulty: the average rating of everyone in the run, mapped to
//!   `d` in [D_MIN, D_MAX]. One value for the whole river, so crews compete on
//!   equal terms.
//! - Every scored run is logged for tuning and for a future trained model.

use river_core::{Run, D_MAX, D_MIN};
use serde_json::Value;
use sqlx::PgPool;
use std::collections::HashMap;
use uuid::Uuid;

pub const START_RATING: f32 = 1000.0;

/// Rating → starting difficulty: 800 or less → 0.8, 1000 → 1.0, 1200 or more → 1.2.
pub fn difficulty_for(avg_rating: f32) -> f32 {
    (1.0 + (avg_rating - START_RATING) / 1000.0).clamp(D_MIN, D_MAX)
}

/// Starting difficulty for these players (unknown players count as new).
pub async fn room_difficulty(db: &PgPool, players: &[Uuid]) -> f32 {
    if players.is_empty() {
        return 1.0;
    }
    let rows: Vec<(Uuid, f32)> = sqlx::query_as("SELECT player_id, rating FROM player_skill WHERE player_id = ANY($1)")
        .bind(players)
        .fetch_all(db)
        .await
        .unwrap_or_else(|e| {
            tracing::warn!(error = %e, "reading skill ratings failed; using the default difficulty");
            Vec::new()
        });
    let sum: f32 = players
        .iter()
        .map(|p| rows.iter().find(|(q, _)| q == p).map(|(_, r)| *r).unwrap_or(START_RATING))
        .sum();
    difficulty_for(sum / players.len() as f32)
}

/// A player's performance in a finished run, on the rating scale (500–1600).
///
/// Combines their share of the plastic that passed (compared with a fair share),
/// capsizes per minute, pipes they plugged and the river's final health.
pub fn performance(run: &Run, seat: u8) -> Option<f32> {
    let b = run.boats.iter().find(|b| b.seat == seat)?;
    let minutes = (run.t / 60.0).max(0.5);
    let resolved = run.boats.iter().map(|b| b.collected).sum::<u32>() + run.missed;
    let fair = resolved as f32 / run.boats.len().max(1) as f32;
    let share = if fair > 0.0 { b.collected as f32 / fair } else { 0.65 };
    let capsizes = b.capsizes as f32 / minutes;
    let health = (run.health - 60.0) / 40.0;
    let p = START_RATING + 500.0 * (share - 0.65) - 80.0 * capsizes + 30.0 * b.plugged as f32 + 150.0 * health;
    Some(p.clamp(500.0, 1600.0))
}

/// Move a rating towards a performance. Returns (new rating, new confidence).
pub fn learn(rating: f32, runs: i32, perf: f32) -> (f32, f32) {
    let k = (1.0 / (runs as f32 + 2.0)).max(0.1);
    let runs = runs + 1;
    (rating + k * (perf - rating), runs as f32 / (runs as f32 + 4.0))
}

/// Update ratings and ranks, and log the run. `players` maps seats to player ids.
/// Returns each seat's rank as the results screen shows it.
pub async fn record(
    db: &PgPool,
    room: Uuid,
    practice: bool,
    run: &Run,
    players: &[(u8, Uuid)],
) -> sqlx::Result<HashMap<u8, Value>> {
    let mut ranks = HashMap::new();
    let mut tx = db.begin().await?;
    let run_id: i64 = sqlx::query_scalar(
        "INSERT INTO director_runs (room_id, practice, boats, crews, lanes, adaptive, d_start, d_end, final_health)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING id",
    )
    .bind(room)
    .bind(practice)
    .bind(run.boats.len() as i16)
    .bind(run.crews.len() as i16)
    .bind(run.lane_count as i16)
    .bind(run.adaptive)
    .bind(run.difficulty_start)
    .bind(run.difficulty)
    .bind(run.health)
    .fetch_one(&mut *tx)
    .await?;
    for &(seat, player) in players {
        let Some(perf) = performance(run, seat) else { continue };
        let b = run.boats.iter().find(|b| b.seat == seat).expect("boat for seat");
        let start_rank = crate::missions::rank_for(START_RATING);
        // shown rank, and the rank today started with (a new day starts from the shown rank)
        let (rating, runs, shown, day_start): (f32, i32, i16, i16) = sqlx::query_as(
            "SELECT rating, runs, shown_rank,
                    CASE WHEN rank_day = (now() AT TIME ZONE 'utc')::date THEN rank_day_start ELSE shown_rank END
             FROM player_skill WHERE player_id = $1 FOR UPDATE",
        )
        .bind(player)
        .fetch_optional(&mut *tx)
        .await?
        .unwrap_or((START_RATING, 0, start_rank, start_rank));
        let (after, confidence) = learn(rating, runs, perf);
        let new_rank = crate::missions::shown_rank(crate::missions::rank_for(after), day_start);
        sqlx::query(
            "INSERT INTO player_skill (player_id, rating, confidence, runs, updated_at, shown_rank, rank_day, rank_day_start)
             VALUES ($1, $2, $3, 1, now(), $4, (now() AT TIME ZONE 'utc')::date, $5)
             ON CONFLICT (player_id) DO UPDATE SET rating = $2, confidence = $3, runs = player_skill.runs + 1,
               updated_at = now(), shown_rank = $4, rank_day = (now() AT TIME ZONE 'utc')::date, rank_day_start = $5",
        )
        .bind(player)
        .bind(after)
        .bind(confidence)
        .bind(new_rank)
        .bind(day_start)
        .execute(&mut *tx)
        .await?;
        ranks.insert(seat, crate::missions::rank_view(after, new_rank, Some(shown)));
        sqlx::query(
            "INSERT INTO director_players (run_id, player_id, performance, rating_before, rating_after, collected, capsizes, pipes)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(run_id)
        .bind(player)
        .bind(perf)
        .bind(rating)
        .bind(after)
        .bind(b.collected as i32)
        .bind(b.capsizes as i32)
        .bind(b.plugged as i32)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(ranks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn difficulty_follows_the_room_average_within_limits() {
        assert_eq!(difficulty_for(1000.0), 1.0);
        assert!((difficulty_for(1100.0) - 1.1).abs() < 1e-6);
        assert_eq!(difficulty_for(2000.0), D_MAX);
        assert_eq!(difficulty_for(100.0), D_MIN);
    }

    #[test]
    fn ratings_learn_fast_at_first_then_settle() {
        let (r1, c1) = learn(1000.0, 0, 1200.0);
        assert!((r1 - 1100.0).abs() < 1e-3, "first run moves halfway: {r1}");
        let (r2, c2) = learn(1000.0, 20, 1200.0);
        assert!(r2 < 1025.0, "an established rating moves a little: {r2}");
        assert!(c2 > c1);
        let (r3, _) = learn(1000.0, 200, 1200.0);
        assert!((r3 - 1020.0).abs() < 1e-3, "never slower than 10 %: {r3}");
    }

    #[test]
    fn better_play_scores_higher() {
        let mut good = Run::new(1, &[0]);
        let mut poor = Run::new(1, &[0]);
        for r in [&mut good, &mut poor] {
            r.t = 180.0;
            r.health = 70.0;
        }
        good.boats[0].collected = 80;
        good.missed = 20;
        good.boats[0].plugged = 2;
        poor.boats[0].collected = 30;
        poor.missed = 70;
        poor.boats[0].capsizes = 9;
        let (g, p) = (performance(&good, 0).unwrap(), performance(&poor, 0).unwrap());
        assert!(g > 1050.0 && p < 950.0, "good {g}, poor {p}");
        assert!(performance(&good, 7).is_none());
    }
}
