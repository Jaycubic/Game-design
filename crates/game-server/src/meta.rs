//! Hub economy: earned-only Seeds, boats, daily rewards, River Road.
//!
//! Fair-play rules: no real money, no paid randomness; boats are modest,
//! capped upgrades (see river_core::boat_stats); a missed day pauses the daily track
//! instead of resetting it; the win bonus counts only the first 3 wins a day;
//! every purchase can be undone for 10 minutes. All changes are transactional.

use crate::auth::AuthPlayer;
use crate::error::{ApiError, ApiResult};
use crate::AppState;
use actix_web::{web, HttpResponse};
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;
use std::collections::HashMap;
use uuid::Uuid;

pub struct Boat {
    pub key: &'static str,
    pub name: &'static str,
    pub colour: &'static str,
    pub price: i32,
    pub source: &'static str,
    /// 3D model file in client/public/models/boats (Kenney Watercraft Kit, CC0).
    pub model: &'static str,
    pub blurb: &'static str,
}

pub const DEFAULT_BOAT: &str = "boat_sun";

pub const BOATS: &[Boat] = &[
    Boat { key: "boat_sun", name: "Driftwood", colour: "#f2c14e", price: 0, source: "default", model: "boat-row-small", blurb: "A trusty rowboat. Every crew starts here." },
    Boat { key: "boat_sky", name: "Kingfisher", colour: "#4fb3ff", price: 0, source: "road", model: "boat-fishing-small", blurb: "A fishing boat with a wider net." },
    Boat { key: "boat_coral", name: "Coral Skiff", colour: "#ff7a59", price: 120, source: "shop", model: "boat-speed-c", blurb: "Quick, and charges boost fast." },
    Boat { key: "boat_lilac", name: "Bluefin", colour: "#9b6bff", price: 120, source: "shop", model: "boat-speed-a", blurb: "Quick, with a wider net." },
    Boat { key: "boat_leaf", name: "Mangrove", colour: "#3fbf7f", price: 200, source: "shop", model: "boat-speed-f", blurb: "A well-rounded cleanup cruiser." },
    Boat { key: "boat_night", name: "Night Heron", colour: "#3a4f8a", price: 300, source: "shop", model: "boat-speed-g", blurb: "A sleek jet boat: the fastest on the river." },
    Boat { key: "boat_lotus", name: "River Keeper", colour: "#ff8fc8", price: 0, source: "road", model: "boat-tug-c", blurb: "A cleanup tug whose net sweeps two lanes." },
];

/// River Road: (runs completed, Seeds, optional boat).
pub const ROAD: &[(i32, i32, Option<&str>)] = &[
    (1, 50, Some("boat_sky")),
    (3, 75, None),
    (5, 100, None),
    (8, 150, None),
    (10, 100, Some("boat_lotus")),
    (15, 300, None),
];

/// Daily reward track (advances by total claims, so missed days only pause it).
pub const DAILY: [i32; 7] = [20, 25, 30, 40, 50, 60, 100];

pub const WIN_BONUS_PER_DAY: i64 = 3;
pub const REFUND_MINUTES: i64 = 10;

pub fn boat(key: &str) -> Option<&'static Boat> {
    BOATS.iter().find(|b| b.key == key)
}

/// Seeds for one player's run. Pure, so the rules are easy to test.
pub fn run_reward(score: i32, won: bool, revived: bool, practice: bool, wins_today: i64) -> i32 {
    let mut s = (score.max(0) / 5).min(60);
    if won && wins_today < WIN_BONUS_PER_DAY {
        s += 20;
    }
    if revived {
        s += 10;
    }
    if practice {
        s /= 2;
    }
    s
}

async fn ensure(db: &PgPool, pid: Uuid) -> sqlx::Result<()> {
    sqlx::query("INSERT INTO player_wallets (player_id) VALUES ($1) ON CONFLICT DO NOTHING")
        .bind(pid)
        .execute(db)
        .await?;
    sqlx::query(
        "INSERT INTO player_items (player_id, item_key, source) VALUES ($1, $2, 'default') ON CONFLICT DO NOTHING",
    )
    .bind(pid)
    .bind(DEFAULT_BOAT)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn profile(state: web::Data<AppState>, player: AuthPlayer) -> ApiResult<HttpResponse> {
    let pid = player.0.id;
    ensure(&state.db, pid).await?;
    // Independent reads run side by side (each on its own pooled connection).
    let db = &state.db;
    let wallet = async {
        sqlx::query_as::<_, (i32, i32, i32, i32, String)>(
            "SELECT seeds, runs, wins, best_score, equipped FROM player_wallets WHERE player_id = $1",
        )
        .bind(pid)
        .fetch_one(db)
        .await
    };
    let items = async {
        sqlx::query_as::<_, (String, String, bool)>(
            "SELECT item_key, source, (source = 'shop' AND acquired_at > now() - make_interval(mins => $2)) FROM player_items WHERE player_id = $1",
        )
        .bind(pid)
        .bind(REFUND_MINUTES as i32)
        .fetch_all(db)
        .await
    };
    // Daily claims: how many, and whether today's is taken, in one query.
    let daily = async {
        sqlx::query_as::<_, (i64, bool)>(
            "SELECT count(*), COALESCE(bool_or(day = (now() AT TIME ZONE 'utc')::date), false) FROM daily_claims WHERE player_id = $1",
        )
        .bind(pid)
        .fetch_one(db)
        .await
    };
    let road = async {
        sqlx::query_scalar::<_, i32>("SELECT milestone FROM road_claims WHERE player_id = $1").bind(pid).fetch_all(db).await
    };
    let ((seeds, runs, wins, best, equipped), items, (claims, claimed_today), road_claimed, rivers, missions, rank) = tokio::try_join!(
        wallet,
        items,
        daily,
        road,
        crate::rivers::profile(db, pid),
        crate::missions::list(db, pid),
        crate::missions::rank(db, pid),
    )?;
    let owned: Vec<&String> = items.iter().map(|(k, _, _)| k).collect();
    Ok(HttpResponse::Ok().json(json!({
        "missions": missions, "rank": rank,
        "rivers": rivers["rivers"], "stars": rivers["stars"],
        "seeds": seeds, "runs": runs, "wins": wins, "best_score": best, "equipped": equipped,
        "owned": owned,
        "refundable": items.iter().filter(|(_, _, r)| *r).map(|(k, _, _)| k).collect::<Vec<_>>(),
        "catalog": BOATS.iter().map(|b| {
            let st = river_core::boat_stats(b.key);
            json!({ "key": b.key, "name": b.name, "colour": b.colour, "price": b.price, "source": b.source, "model": b.model, "blurb": b.blurb,
                    "stats": { "speed": st.speed, "net": st.net, "cutter": st.cutter, "boost": st.boost } })
        }).collect::<Vec<_>>(),
        "daily": { "claims": claims, "claimed_today": claimed_today, "track": DAILY, "next": DAILY[(claims % 7) as usize] },
        "road": ROAD.iter().map(|(r, s, item)| json!({ "runs": r, "seeds": s, "item": item, "reached": runs >= *r, "claimed": road_claimed.contains(r) })).collect::<Vec<_>>(),
        "rules": { "win_bonus_per_day": WIN_BONUS_PER_DAY, "refund_minutes": REFUND_MINUTES },
    })))
}

pub async fn claim_daily(state: web::Data<AppState>, player: AuthPlayer) -> ApiResult<HttpResponse> {
    let pid = player.0.id;
    ensure(&state.db, pid).await?;
    let mut tx = state.db.begin().await?;
    let claims: i64 = sqlx::query_scalar("SELECT count(*) FROM daily_claims WHERE player_id = $1")
        .bind(pid)
        .fetch_one(&mut *tx)
        .await?;
    let reward = DAILY[(claims % 7) as usize];
    let ok = sqlx::query(
        "INSERT INTO daily_claims (player_id, day, reward) VALUES ($1, (now() AT TIME ZONE 'utc')::date, $2) ON CONFLICT DO NOTHING",
    )
    .bind(pid)
    .bind(reward)
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if ok == 0 {
        return Err(ApiError::conflict("already_claimed", "Today's reward is already collected — come back tomorrow."));
    }
    sqlx::query("UPDATE player_wallets SET seeds = seeds + $2, updated_at = now() WHERE player_id = $1")
        .bind(pid)
        .bind(reward)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(json!({ "reward": reward })))
}

#[derive(Deserialize)]
pub struct ItemBody {
    pub item: String,
}

pub async fn buy(state: web::Data<AppState>, player: AuthPlayer, body: web::Json<ItemBody>) -> ApiResult<HttpResponse> {
    let pid = player.0.id;
    let b = boat(&body.item).filter(|b| b.source == "shop").ok_or_else(|| ApiError::not_found("That item"))?;
    ensure(&state.db, pid).await?;
    let mut tx = state.db.begin().await?;
    let seeds: i32 = sqlx::query_scalar("SELECT seeds FROM player_wallets WHERE player_id = $1 FOR UPDATE")
        .bind(pid)
        .fetch_one(&mut *tx)
        .await?;
    if seeds < b.price {
        return Err(ApiError::bad_request(
            "not_enough_seeds",
            format!("You need {} Seeds — you have {seeds}. Play runs to earn more.", b.price),
        ));
    }
    let added = sqlx::query("INSERT INTO player_items (player_id, item_key, source, price) VALUES ($1, $2, 'shop', $3) ON CONFLICT DO NOTHING")
        .bind(pid)
        .bind(b.key)
        .bind(b.price)
        .execute(&mut *tx)
        .await?
        .rows_affected();
    if added == 0 {
        return Err(ApiError::conflict("already_owned", "You already own that boat."));
    }
    sqlx::query("UPDATE player_wallets SET seeds = seeds - $2, updated_at = now() WHERE player_id = $1")
        .bind(pid)
        .bind(b.price)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(json!({ "item": b.key, "spent": b.price, "seeds": seeds - b.price })))
}

pub async fn refund(
    state: web::Data<AppState>,
    player: AuthPlayer,
    body: web::Json<ItemBody>,
) -> ApiResult<HttpResponse> {
    let pid = player.0.id;
    let mut tx = state.db.begin().await?;
    let price: Option<i32> = sqlx::query_scalar(
        "DELETE FROM player_items WHERE player_id = $1 AND item_key = $2 AND source = 'shop'
         AND acquired_at > now() - make_interval(mins => $3) RETURNING price",
    )
    .bind(pid)
    .bind(&body.item)
    .bind(REFUND_MINUTES as i32)
    .fetch_optional(&mut *tx)
    .await?;
    let price =
        price.ok_or_else(|| ApiError::bad_request("not_refundable", "Purchases can be undone for 10 minutes only."))?;
    sqlx::query(
        "UPDATE player_wallets SET seeds = seeds + $2, equipped = CASE WHEN equipped = $3 THEN $4 ELSE equipped END, updated_at = now()
         WHERE player_id = $1",
    )
    .bind(pid)
    .bind(price)
    .bind(&body.item)
    .bind(DEFAULT_BOAT)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(json!({ "refunded": price })))
}

pub async fn equip(
    state: web::Data<AppState>,
    player: AuthPlayer,
    body: web::Json<ItemBody>,
) -> ApiResult<HttpResponse> {
    let pid = player.0.id;
    ensure(&state.db, pid).await?;
    let n = sqlx::query(
        "UPDATE player_wallets SET equipped = $2, updated_at = now() WHERE player_id = $1
         AND EXISTS (SELECT 1 FROM player_items WHERE player_id = $1 AND item_key = $2)",
    )
    .bind(pid)
    .bind(&body.item)
    .execute(&state.db)
    .await?
    .rows_affected();
    if n == 0 {
        return Err(ApiError::bad_request("not_owned", "You don't own that boat yet."));
    }
    Ok(HttpResponse::Ok().json(json!({ "equipped": body.item })))
}

#[derive(Deserialize)]
pub struct MilestoneBody {
    pub runs: i32,
}

pub async fn claim_road(
    state: web::Data<AppState>,
    player: AuthPlayer,
    body: web::Json<MilestoneBody>,
) -> ApiResult<HttpResponse> {
    let pid = player.0.id;
    let (runs_needed, seeds, item) =
        *ROAD.iter().find(|(r, _, _)| *r == body.runs).ok_or_else(|| ApiError::not_found("That milestone"))?;
    ensure(&state.db, pid).await?;
    let mut tx = state.db.begin().await?;
    let runs: i32 = sqlx::query_scalar("SELECT runs FROM player_wallets WHERE player_id = $1 FOR UPDATE")
        .bind(pid)
        .fetch_one(&mut *tx)
        .await?;
    if runs < runs_needed {
        return Err(ApiError::bad_request(
            "not_reached",
            format!("Complete {runs_needed} runs to reach this milestone ({runs} so far)."),
        ));
    }
    let n = sqlx::query("INSERT INTO road_claims (player_id, milestone) VALUES ($1, $2) ON CONFLICT DO NOTHING")
        .bind(pid)
        .bind(runs_needed)
        .execute(&mut *tx)
        .await?
        .rows_affected();
    if n == 0 {
        return Err(ApiError::conflict("already_claimed", "You already collected this milestone."));
    }
    sqlx::query("UPDATE player_wallets SET seeds = seeds + $2, updated_at = now() WHERE player_id = $1")
        .bind(pid)
        .bind(seeds)
        .execute(&mut *tx)
        .await?;
    if let Some(item) = item {
        sqlx::query(
            "INSERT INTO player_items (player_id, item_key, source) VALUES ($1, $2, 'road') ON CONFLICT DO NOTHING",
        )
        .bind(pid)
        .bind(item)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(json!({ "seeds": seeds, "item": item })))
}

/// Equipped boat key for each player (players without a wallet get the default).
pub async fn skins(db: &PgPool, players: &[Uuid]) -> HashMap<Uuid, String> {
    let rows: Vec<(Uuid, String)> =
        sqlx::query_as("SELECT player_id, equipped FROM player_wallets WHERE player_id = ANY($1)")
            .bind(players)
            .fetch_all(db)
            .await
            .unwrap_or_default();
    rows.into_iter().filter_map(|(p, k)| boat(&k).map(|b| (p, b.key.to_string()))).collect()
}

/// Record a finished run for each player and pay out Seeds. Returns seat → Seeds.
pub struct RunOutcome {
    pub player: Uuid,
    pub seat: u8,
    pub score: i32,
    pub won: bool,
    pub revived: bool,
}

pub async fn award(
    db: &PgPool,
    room: Uuid,
    practice: bool,
    outcomes: Vec<RunOutcome>,
) -> sqlx::Result<HashMap<u8, i32>> {
    let mut paid = HashMap::new();
    for o in outcomes {
        ensure(db, o.player).await?;
        let mut tx = db.begin().await?;
        let wins_today: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM run_rewards WHERE player_id = $1 AND won AND NOT practice
             AND created_at >= date_trunc('day', now() AT TIME ZONE 'utc') AT TIME ZONE 'utc'",
        )
        .bind(o.player)
        .fetch_one(&mut *tx)
        .await?;
        let seeds = run_reward(o.score, o.won, o.revived, practice, wins_today);
        sqlx::query(
            "INSERT INTO run_rewards (player_id, room_id, score, seeds, won, practice) VALUES ($1,$2,$3,$4,$5,$6)",
        )
        .bind(o.player)
        .bind(room)
        .bind(o.score)
        .bind(seeds)
        .bind(o.won)
        .bind(practice)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "UPDATE player_wallets SET seeds = seeds + $2, runs = runs + 1, wins = wins + $3::int,
             best_score = GREATEST(best_score, $4), updated_at = now() WHERE player_id = $1",
        )
        .bind(o.player)
        .bind(seeds)
        .bind(i32::from(o.won && !practice))
        .bind(o.score)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        paid.insert(o.seat, seeds);
    }
    Ok(paid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewards_are_capped_and_fair() {
        assert_eq!(run_reward(100, false, false, false, 0), 20);
        assert_eq!(run_reward(10_000, false, false, false, 0), 60, "score part capped");
        assert_eq!(run_reward(100, true, true, false, 0), 50);
        assert_eq!(run_reward(100, true, false, false, 3), 20, "win bonus only for the first 3 wins a day");
        assert_eq!(run_reward(100, true, true, true, 0), 25, "practice pays half");
        assert_eq!(run_reward(-40, false, false, false, 0), 0);
    }

    #[test]
    fn catalog_is_consistent() {
        assert!(boat(DEFAULT_BOAT).is_some());
        for (_, _, item) in ROAD {
            if let Some(i) = item {
                assert_eq!(boat(i).map(|b| b.source), Some("road"));
            }
        }
        assert!(BOATS.iter().filter(|b| b.source == "shop").all(|b| b.price > 0));
    }
}
