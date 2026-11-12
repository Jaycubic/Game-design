//! Database persistence for rooms: snapshots, ordered events, votes, clues,
//! results and progression. Every accepted change is written in one transaction.

use crate::room::{Effect, Notice, RoomState};
use nt_core::campaign::CampaignPhase;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

/// Keep this many recent snapshots per campaign (older ones are pruned).
const SNAPSHOTS_KEPT: i64 = 20;

pub async fn create_room(db: &PgPool, room: &RoomState, host: Uuid) -> sqlx::Result<()> {
    let mut tx = db.begin().await?;
    sqlx::query("INSERT INTO rooms (id, join_code, host_player_id, min_players, max_players, run_index) VALUES ($1,$2,$3,$4,$5,$6)")
        .bind(room.id)
        .bind(&room.code)
        .bind(host)
        .bind(room.roster.min as i16)
        .bind(room.roster.max as i16)
        .bind(room.campaign.run_index as i32)
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO room_members (room_id, player_id, seat) VALUES ($1,$2,0)")
        .bind(room.id)
        .bind(host)
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO regional_campaigns (id, room_id, region_id, run_index) VALUES ($1,$2,$3,$4)")
        .bind(room.campaign_id)
        .bind(room.id)
        .bind(&room.campaign.region)
        .bind(room.campaign.run_index as i32)
        .execute(&mut *tx)
        .await?;
    snapshot(&mut tx, room).await?;
    tx.commit().await
}

async fn snapshot(tx: &mut Transaction<'_, Postgres>, room: &RoomState) -> sqlx::Result<()> {
    let state = serde_json::to_value(room).expect("room state serialises");
    sqlx::query(
        "INSERT INTO room_state_snapshots (campaign_id, version, state) VALUES ($1,$2,$3)
         ON CONFLICT (campaign_id, version) DO UPDATE SET state = EXCLUDED.state, saved_at = now()",
    )
    .bind(room.campaign_id)
    .bind(room.version as i64)
    .bind(state)
    .execute(&mut **tx)
    .await?;
    sqlx::query("DELETE FROM room_state_snapshots WHERE campaign_id = $1 AND version < $2")
        .bind(room.campaign_id)
        .bind(room.version as i64 - SNAPSHOTS_KEPT)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

/// Persist one accepted change: membership rows, snapshot, events and effects.
pub async fn commit(
    db: &PgPool,
    room: &RoomState,
    actor: Option<Uuid>,
    notices: &[Notice],
    effects: &[Effect],
) -> sqlx::Result<()> {
    let mut tx = db.begin().await?;
    // Effects first: a room status change to in_progress is checked against
    // the member rows by a trigger, so membership is synced before it.
    sync_members(&mut tx, room).await?;
    for e in effects {
        apply_effect(&mut tx, room, e).await?;
    }
    for (i, n) in notices.iter().enumerate() {
        let payload = serde_json::to_value(n).unwrap_or_default();
        let event_type = payload.get("event").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
        // seq = version * 16 + index keeps events ordered and unique per room.
        sqlx::query(
            "INSERT INTO room_events (room_id, campaign_id, seq, actor_player_id, event_type, payload)
             VALUES ($1,$2,$3,$4,$5,$6) ON CONFLICT (room_id, seq) DO NOTHING",
        )
        .bind(room.id)
        .bind(room.campaign_id)
        .bind(room.version as i64 * 16 + i as i64)
        .bind(actor)
        .bind(event_type)
        .bind(payload)
        .execute(&mut *tx)
        .await?;
    }
    snapshot(&mut tx, room).await?;
    sqlx::query("UPDATE rooms SET last_activity_at = now(), updated_at = now(), host_player_id = $2 WHERE id = $1")
        .bind(room.id)
        .bind(Uuid::parse_str(&room.roster.host).unwrap_or(Uuid::nil()))
        .execute(&mut *tx)
        .await?;
    let level = room.campaign.current_level().map(|l| l as i16).unwrap_or(10);
    sqlx::query("UPDATE regional_campaigns SET current_level = $2 WHERE id = $1")
        .bind(room.campaign_id)
        .bind(level)
        .execute(&mut *tx)
        .await?;
    tx.commit().await
}

async fn sync_members(tx: &mut Transaction<'_, Postgres>, room: &RoomState) -> sqlx::Result<()> {
    let ids: Vec<Uuid> = room.roster.members.iter().filter_map(|m| Uuid::parse_str(&m.player_id).ok()).collect();
    // Members no longer on the roster have left.
    sqlx::query(
        "UPDATE room_members SET left_at = now() WHERE room_id = $1 AND left_at IS NULL AND NOT (player_id = ANY($2))",
    )
    .bind(room.id)
    .bind(&ids)
    .execute(&mut **tx)
    .await?;
    for m in &room.roster.members {
        let Ok(pid) = Uuid::parse_str(&m.player_id) else { continue };
        sqlx::query(
            "INSERT INTO room_members (room_id, player_id, seat, ready, connected) VALUES ($1,$2,$3,$4,$5)
             ON CONFLICT (room_id, player_id) DO UPDATE
               SET seat = EXCLUDED.seat, ready = EXCLUDED.ready, connected = EXCLUDED.connected, left_at = NULL",
        )
        .bind(room.id)
        .bind(pid)
        .bind(m.seat as i16)
        .bind(m.ready)
        .bind(m.connected)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

async fn apply_effect(tx: &mut Transaction<'_, Postgres>, room: &RoomState, e: &Effect) -> sqlx::Result<()> {
    match e {
        Effect::RoomStatus(status) => {
            sqlx::query("UPDATE rooms SET status = $2 WHERE id = $1 AND status <> $2")
                .bind(room.id)
                .bind(*status)
                .execute(&mut **tx)
                .await?;
            if *status == "region_complete" {
                sqlx::query("UPDATE regional_campaigns SET status = 'complete', completed_at = now() WHERE id = $1")
                    .bind(room.campaign_id)
                    .execute(&mut **tx)
                    .await?;
            }
        }
        Effect::Clues { level, hands } => {
            sqlx::query("DELETE FROM player_clues WHERE campaign_id = $1 AND level_number = $2")
                .bind(room.campaign_id)
                .bind(*level as i16)
                .execute(&mut **tx)
                .await?;
            for (player, idx, text) in hands {
                let Ok(pid) = Uuid::parse_str(player) else { continue };
                sqlx::query(
                    "INSERT INTO player_clues (campaign_id, level_number, player_id, clue_index, payload) VALUES ($1,$2,$3,$4,$5)",
                )
                .bind(room.campaign_id)
                .bind(*level as i16)
                .bind(pid)
                .bind(*idx as i16)
                .bind(serde_json::json!({ "text": text }))
                .execute(&mut **tx)
                .await?;
            }
        }
        Effect::Amendment { number, choice, rule } => {
            let (ballots, round, tb) = match &room.campaign.phase {
                CampaignPhase::CharterVote { vote } => (
                    serde_json::to_value(&vote.ballots).unwrap_or_default(),
                    vote.ballot_round as i16,
                    vote.tie_breaker.map(|s| s as i16),
                ),
                _ => (serde_json::json!({}), 1, None),
            };
            sqlx::query(
                "INSERT INTO charter_amendments (campaign_id, amendment_number, ballot_round, ballots, tie_breaker_seat)
                 VALUES ($1,$2,$3,$4,$5) ON CONFLICT (campaign_id, amendment_number) DO NOTHING",
            )
            .bind(room.campaign_id)
            .bind(*number as i16)
            .bind(round)
            .bind(&ballots)
            .bind(tb)
            .execute(&mut **tx)
            .await?;
            if let (Some(choice), Some(rule)) = (choice, rule) {
                let letter = match choice {
                    nt_core::campaign::AmendmentChoice::A => "A",
                    nt_core::campaign::AmendmentChoice::B => "B",
                };
                // Only an unselected slot can be decided; a second decision is a no-op.
                sqlx::query(
                    "UPDATE charter_amendments SET selected = $3, selected_rule = $4, applied_at = now()
                     WHERE campaign_id = $1 AND amendment_number = $2 AND selected IS NULL",
                )
                .bind(room.campaign_id)
                .bind(*number as i16)
                .bind(letter)
                .bind(rule)
                .execute(&mut **tx)
                .await?;
                let column = if *number == 1 { "connectivity" } else { "recovery" };
                sqlx::query(&format!("UPDATE regional_campaigns SET {column} = $2 WHERE id = $1 AND {column} IS NULL"))
                    .bind(room.campaign_id)
                    .bind(rule)
                    .execute(&mut **tx)
                    .await?;
            }
        }
        Effect::VoteProgress { number } => {
            if let CampaignPhase::CharterVote { vote } = &room.campaign.phase {
                sqlx::query(
                    "UPDATE charter_amendments SET ballots = $3, ballot_round = $4, tie_breaker_seat = $5
                     WHERE campaign_id = $1 AND amendment_number = $2 AND selected IS NULL",
                )
                .bind(room.campaign_id)
                .bind(*number as i16)
                .bind(serde_json::to_value(&vote.ballots).unwrap_or_default())
                .bind(vote.ballot_round as i16)
                .bind(vote.tie_breaker.map(|s| s as i16))
                .execute(&mut **tx)
                .await?;
            }
        }
        Effect::LevelResult { result, player_ids } => {
            sqlx::query(
                "INSERT INTO level_results (campaign_id, level_number, outcome, leaf_marks, spent, ruleset, payload)
                 VALUES ($1,$2,$3,$4,$5,$6,$7)",
            )
            .bind(room.campaign_id)
            .bind(result.level as i16)
            .bind(if result.won { "won" } else { "lost" })
            .bind(result.marks.count() as i16)
            .bind(result.spent as i16)
            .bind(&result.rules)
            .bind(serde_json::json!({ "marks": result.marks, "reason": result.reason, "restored_cells": result.restored_cells }))
            .execute(&mut **tx)
            .await?;
            if !result.won {
                return Ok(());
            }
            let key = format!("{}:{}", room.campaign.region, result.level);
            for p in player_ids {
                let Ok(pid) = Uuid::parse_str(p) else { continue };
                // Best result only: totals grow by the improvement, never by repeats.
                sqlx::query("INSERT INTO player_progression (player_id) VALUES ($1) ON CONFLICT DO NOTHING")
                    .bind(pid)
                    .execute(&mut **tx)
                    .await?;
                sqlx::query(
                    "UPDATE player_progression SET
                        leaf_marks_total = leaf_marks_total + GREATEST(0, $3 - COALESCE((best_marks->>$2)::int, 0)),
                        levels_completed = levels_completed + CASE WHEN best_marks ? $2 THEN 0 ELSE 1 END,
                        regions_completed = regions_completed + CASE WHEN $4 AND NOT best_marks ? $2 THEN 1 ELSE 0 END,
                        best_marks = jsonb_set(best_marks, ARRAY[$2], to_jsonb(GREATEST($3, COALESCE((best_marks->>$2)::int, 0)))),
                        updated_at = now()
                     WHERE player_id = $1",
                )
                .bind(pid)
                .bind(&key)
                .bind(result.marks.count() as i32)
                .bind(result.level == nt_core::campaign::LEVELS_PER_REGION)
                .execute(&mut **tx)
                .await?;
                for entry in &result.journal {
                    sqlx::query(
                        "INSERT INTO field_journal_entries (player_id, entry_key) VALUES ($1,$2) ON CONFLICT DO NOTHING",
                    )
                    .bind(pid)
                    .bind(entry)
                    .execute(&mut **tx)
                    .await?;
                }
            }
        }
    }
    Ok(())
}

/// Latest snapshot of every room that is still open (for restart recovery).
pub async fn load_open_rooms(db: &PgPool) -> sqlx::Result<Vec<RoomState>> {
    let rows: Vec<(serde_json::Value,)> = sqlx::query_as(
        "SELECT DISTINCT ON (s.campaign_id) s.state
         FROM room_state_snapshots s
         JOIN regional_campaigns c ON c.id = s.campaign_id
         JOIN rooms r ON r.id = c.room_id
         WHERE r.status NOT IN ('closed', 'expired') AND c.status = 'active'
         ORDER BY s.campaign_id, s.version DESC",
    )
    .fetch_all(db)
    .await?;
    let mut rooms = Vec::new();
    for (v,) in rows {
        match serde_json::from_value::<RoomState>(v) {
            Ok(mut r) => {
                for m in &mut r.roster.members {
                    m.connected = false; // nobody is connected right after a restart
                }
                rooms.push(r);
            }
            Err(e) => tracing::warn!(error = %e, "skipping unreadable room snapshot"),
        }
    }
    Ok(rooms)
}

pub async fn expire_idle_rooms(db: &PgPool, hours: i64) -> sqlx::Result<Vec<Uuid>> {
    sqlx::query_scalar(
        "UPDATE rooms SET status = 'expired', updated_at = now()
         WHERE status NOT IN ('closed', 'expired') AND last_activity_at < now() - make_interval(hours => $1)
         RETURNING id",
    )
    .bind(hours as i32)
    .fetch_all(db)
    .await
}
