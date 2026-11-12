-- Hub economy: one earned-only currency (Seeds), cosmetic boats, daily rewards,
-- River Road milestones and an audit of every reward and purchase.
-- No real money anywhere; every row is earned through play.

CREATE TABLE player_wallets (
    player_id   UUID PRIMARY KEY REFERENCES players (id) ON DELETE CASCADE,
    seeds       INTEGER NOT NULL DEFAULT 0 CHECK (seeds >= 0),
    runs        INTEGER NOT NULL DEFAULT 0 CHECK (runs >= 0),
    wins        INTEGER NOT NULL DEFAULT 0 CHECK (wins >= 0),
    best_score  INTEGER NOT NULL DEFAULT 0,
    equipped    TEXT NOT NULL DEFAULT 'boat_sun',
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Owned cosmetics. `price` > 0 only for shop purchases (used for undo refunds).
CREATE TABLE player_items (
    player_id    UUID NOT NULL REFERENCES players (id) ON DELETE CASCADE,
    item_key     TEXT NOT NULL,
    source       TEXT NOT NULL CHECK (source IN ('default', 'shop', 'road')),
    price        INTEGER NOT NULL DEFAULT 0 CHECK (price >= 0),
    acquired_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (player_id, item_key)
);

-- One claim per UTC day; the reward track advances by total claims, so a
-- missed day pauses the track instead of resetting it.
CREATE TABLE daily_claims (
    player_id  UUID NOT NULL REFERENCES players (id) ON DELETE CASCADE,
    day        DATE NOT NULL,
    reward     INTEGER NOT NULL CHECK (reward > 0),
    claimed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (player_id, day)
);

CREATE TABLE road_claims (
    player_id   UUID NOT NULL REFERENCES players (id) ON DELETE CASCADE,
    milestone   INTEGER NOT NULL CHECK (milestone > 0),
    claimed_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (player_id, milestone)
);

-- Every finished run's reward per player (also used for the daily win-bonus cap).
CREATE TABLE run_rewards (
    id          BIGSERIAL PRIMARY KEY,
    player_id   UUID NOT NULL REFERENCES players (id) ON DELETE CASCADE,
    room_id     UUID REFERENCES rooms (id) ON DELETE SET NULL,
    score       INTEGER NOT NULL,
    seeds       INTEGER NOT NULL CHECK (seeds >= 0),
    won         BOOLEAN NOT NULL,
    practice    BOOLEAN NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX run_rewards_player_day_idx ON run_rewards (player_id, created_at);
