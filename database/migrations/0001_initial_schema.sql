-- Nature's Thread — initial schema.
-- Applied by the server's embedded migrator (sqlx); the migrator records each
-- version in _sqlx_migrations, so re-running is safe and a failed migration
-- stops startup with an error.
--
-- Static content (levels, species, events, Charter options) lives in content/
-- and is compiled into the server; the database stores players, rooms,
-- campaign state, votes, results and progression. All timestamps are UTC.

CREATE EXTENSION IF NOT EXISTS pgcrypto;  -- gen_random_uuid()

-- ── Identity ────────────────────────────────────────────────────────────────

CREATE TABLE players (
    id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    handle         TEXT NOT NULL CHECK (handle ~ '^[a-z0-9_]{3,24}$'),
    display_name   TEXT NOT NULL CHECK (char_length(display_name) BETWEEN 1 AND 32),
    password_hash  TEXT NOT NULL,           -- Argon2id PHC string; never plaintext
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX players_handle_key ON players (handle);

-- One row per issued token (its jti). Tokens themselves are never stored.
CREATE TABLE player_sessions (
    id          UUID PRIMARY KEY,
    player_id   UUID NOT NULL REFERENCES players (id) ON DELETE CASCADE,
    expires_at  TIMESTAMPTZ NOT NULL,
    revoked_at  TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX player_sessions_player_idx ON player_sessions (player_id);

-- ── Rooms ───────────────────────────────────────────────────────────────────

CREATE TABLE rooms (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    join_code         TEXT NOT NULL CHECK (join_code ~ '^[A-Z2-9]{4,8}$'),
    host_player_id    UUID NOT NULL REFERENCES players (id),
    status            TEXT NOT NULL DEFAULT 'waiting'
                      CHECK (status IN ('waiting', 'in_progress', 'level_complete', 'level_failed',
                                        'charter_vote', 'region_complete', 'closed', 'expired')),
    min_players       SMALLINT NOT NULL DEFAULT 2 CHECK (min_players BETWEEN 2 AND 4),
    max_players       SMALLINT NOT NULL DEFAULT 4 CHECK (max_players BETWEEN 2 AND 4),
    run_index         INTEGER NOT NULL DEFAULT 0 CHECK (run_index >= 0),
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_activity_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (min_players <= max_players)
);
-- A join code is unique among rooms that are still open.
CREATE UNIQUE INDEX rooms_active_join_code_key ON rooms (join_code)
    WHERE status NOT IN ('closed', 'expired');
CREATE INDEX rooms_active_status_idx ON rooms (status, last_activity_at)
    WHERE status NOT IN ('closed', 'expired');

CREATE TABLE room_members (
    room_id    UUID NOT NULL REFERENCES rooms (id) ON DELETE CASCADE,
    player_id  UUID NOT NULL REFERENCES players (id) ON DELETE CASCADE,
    seat       SMALLINT NOT NULL CHECK (seat BETWEEN 0 AND 3),
    ready      BOOLEAN NOT NULL DEFAULT false,
    connected  BOOLEAN NOT NULL DEFAULT true,
    joined_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    left_at    TIMESTAMPTZ,
    PRIMARY KEY (room_id, player_id)
);
CREATE UNIQUE INDEX room_members_active_seat_key ON room_members (room_id, seat) WHERE left_at IS NULL;
CREATE INDEX room_members_player_idx ON room_members (player_id) WHERE left_at IS NULL;

-- Never more active members than the room allows.
CREATE FUNCTION enforce_room_capacity() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    active_count INTEGER;
    room_max     SMALLINT;
BEGIN
    IF NEW.left_at IS NOT NULL THEN
        RETURN NEW;
    END IF;
    SELECT max_players INTO room_max FROM rooms WHERE id = NEW.room_id FOR UPDATE;
    SELECT count(*) INTO active_count FROM room_members
        WHERE room_id = NEW.room_id AND left_at IS NULL AND player_id <> NEW.player_id;
    IF active_count + 1 > room_max THEN
        RAISE EXCEPTION 'room % is full (% players)', NEW.room_id, room_max USING ERRCODE = 'check_violation';
    END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER room_members_capacity BEFORE INSERT OR UPDATE ON room_members
    FOR EACH ROW EXECUTE FUNCTION enforce_room_capacity();

-- A room can only leave the waiting state with at least min_players active members.
CREATE FUNCTION enforce_room_start() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    active_count INTEGER;
BEGIN
    IF OLD.status = 'waiting' AND NEW.status = 'in_progress' THEN
        SELECT count(*) INTO active_count FROM room_members WHERE room_id = NEW.id AND left_at IS NULL;
        IF active_count < NEW.min_players OR active_count > NEW.max_players THEN
            RAISE EXCEPTION 'room % cannot start with % players', NEW.id, active_count USING ERRCODE = 'check_violation';
        END IF;
    END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER rooms_start_check BEFORE UPDATE OF status ON rooms
    FOR EACH ROW EXECUTE FUNCTION enforce_room_start();

-- ── Campaigns, state and events ─────────────────────────────────────────────

CREATE TABLE regional_campaigns (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    room_id       UUID NOT NULL REFERENCES rooms (id) ON DELETE CASCADE,
    region_id     TEXT NOT NULL,
    run_index     INTEGER NOT NULL CHECK (run_index >= 0),
    current_level SMALLINT NOT NULL DEFAULT 1 CHECK (current_level BETWEEN 1 AND 10),
    connectivity  TEXT CHECK (connectivity IN ('continuous', 'stepping_stone')),
    recovery      TEXT CHECK (recovery IN ('succession', 'active')),
    status        TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'complete', 'abandoned')),
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    completed_at  TIMESTAMPTZ,
    UNIQUE (room_id, run_index),
    -- recovery can only be chosen after connectivity
    CHECK (recovery IS NULL OR connectivity IS NOT NULL)
);
CREATE INDEX regional_campaigns_room_idx ON regional_campaigns (room_id, status);
CREATE UNIQUE INDEX regional_campaigns_one_active_key ON regional_campaigns (room_id) WHERE status = 'active';

-- Recoverable authoritative state; `version` increases with every accepted change.
CREATE TABLE room_state_snapshots (
    campaign_id  UUID NOT NULL REFERENCES regional_campaigns (id) ON DELETE CASCADE,
    version      BIGINT NOT NULL CHECK (version >= 0),
    state        JSONB NOT NULL,
    saved_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (campaign_id, version)
);

CREATE TABLE room_events (
    id               BIGSERIAL PRIMARY KEY,
    room_id          UUID NOT NULL REFERENCES rooms (id) ON DELETE CASCADE,
    campaign_id      UUID REFERENCES regional_campaigns (id) ON DELETE CASCADE,
    seq              BIGINT NOT NULL CHECK (seq > 0),
    actor_player_id  UUID REFERENCES players (id),
    event_type       TEXT NOT NULL,
    payload          JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (room_id, seq)
);
CREATE INDEX room_events_campaign_idx ON room_events (campaign_id, seq);

-- ── Living Charter ──────────────────────────────────────────────────────────
-- Exactly two amendment slots per campaign: the CHECK plus the primary key make
-- a third amendment, or a second result for the same slot, impossible.
CREATE TABLE charter_amendments (
    campaign_id       UUID NOT NULL REFERENCES regional_campaigns (id) ON DELETE CASCADE,
    amendment_number  SMALLINT NOT NULL CHECK (amendment_number IN (1, 2)),
    ballot_round      SMALLINT NOT NULL DEFAULT 1 CHECK (ballot_round IN (1, 2)),
    ballots           JSONB NOT NULL DEFAULT '{}'::jsonb,  -- seat -> "A" | "B"
    tie_breaker_seat  SMALLINT CHECK (tie_breaker_seat BETWEEN 0 AND 3),
    selected          TEXT CHECK (selected IN ('A', 'B')),
    selected_rule     TEXT CHECK (selected_rule IN ('continuous', 'stepping_stone', 'succession', 'active')),
    opened_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    applied_at        TIMESTAMPTZ,
    PRIMARY KEY (campaign_id, amendment_number),
    CHECK ((selected IS NULL) = (applied_at IS NULL)),
    CHECK (selected_rule IS NULL
           OR (amendment_number = 1 AND selected_rule IN ('continuous', 'stepping_stone'))
           OR (amendment_number = 2 AND selected_rule IN ('succession', 'active')))
);

-- Private Field Report fragments: only ever sent to `player_id`.
CREATE TABLE player_clues (
    campaign_id   UUID NOT NULL REFERENCES regional_campaigns (id) ON DELETE CASCADE,
    level_number  SMALLINT NOT NULL CHECK (level_number BETWEEN 1 AND 10),
    player_id     UUID NOT NULL REFERENCES players (id) ON DELETE CASCADE,
    clue_index    SMALLINT NOT NULL CHECK (clue_index >= 0),
    payload       JSONB NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (campaign_id, level_number, clue_index)
);
CREATE INDEX player_clues_player_idx ON player_clues (player_id, campaign_id);

-- ── Results and progression ─────────────────────────────────────────────────

CREATE TABLE level_results (
    id            BIGSERIAL PRIMARY KEY,
    campaign_id   UUID NOT NULL REFERENCES regional_campaigns (id) ON DELETE CASCADE,
    level_number  SMALLINT NOT NULL CHECK (level_number BETWEEN 1 AND 10),
    outcome       TEXT NOT NULL CHECK (outcome IN ('won', 'lost')),
    leaf_marks    SMALLINT NOT NULL CHECK (leaf_marks BETWEEN 0 AND 3),
    spent         SMALLINT NOT NULL CHECK (spent >= 0),
    ruleset       TEXT NOT NULL,
    payload       JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (outcome = 'won' OR leaf_marks = 0)
);
CREATE INDEX level_results_campaign_idx ON level_results (campaign_id, level_number);

CREATE TABLE player_progression (
    player_id          UUID PRIMARY KEY REFERENCES players (id) ON DELETE CASCADE,
    leaf_marks_total   INTEGER NOT NULL DEFAULT 0 CHECK (leaf_marks_total >= 0),
    levels_completed   INTEGER NOT NULL DEFAULT 0 CHECK (levels_completed >= 0),
    regions_completed  INTEGER NOT NULL DEFAULT 0 CHECK (regions_completed >= 0),
    best_marks         JSONB NOT NULL DEFAULT '{}'::jsonb,  -- "region:level" -> best marks
    updated_at         TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE field_journal_entries (
    player_id      UUID NOT NULL REFERENCES players (id) ON DELETE CASCADE,
    entry_key      TEXT NOT NULL,
    discovered_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (player_id, entry_key)
);
