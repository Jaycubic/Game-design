-- River Director: a per-player skill estimate (an online Elo/Glicko-style
-- rating) that sets each room's starting difficulty, plus a log of every
-- scored run. The log is kept to tune the Director and, once there is enough
-- of it, to train a model of which settings keep players playing.
-- Internal only: nothing here is shown in the rule book or the game's UI
-- (ranks built on the rating come later and show only the rank name).

CREATE TABLE player_skill (
    player_id   UUID PRIMARY KEY REFERENCES players (id) ON DELETE CASCADE,
    rating      REAL NOT NULL DEFAULT 1000,
    confidence  REAL NOT NULL DEFAULT 0 CHECK (confidence >= 0 AND confidence <= 1),
    runs        INTEGER NOT NULL DEFAULT 0 CHECK (runs >= 0),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- One row per finished run (tutorials excluded).
CREATE TABLE director_runs (
    id            BIGSERIAL PRIMARY KEY,
    room_id       UUID NOT NULL,
    practice      BOOLEAN NOT NULL,
    boats         SMALLINT NOT NULL,
    crews         SMALLINT NOT NULL,
    lanes         SMALLINT NOT NULL,
    adaptive      BOOLEAN NOT NULL,
    d_start       REAL NOT NULL,
    d_end         REAL NOT NULL,
    final_health  REAL NOT NULL,
    finished_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Each player's part in a logged run. "Played again within 24 hours" is
-- derived from the next row for the same player (see docs/DATABASE.md).
CREATE TABLE director_players (
    run_id         BIGINT NOT NULL REFERENCES director_runs (id) ON DELETE CASCADE,
    player_id      UUID NOT NULL REFERENCES players (id) ON DELETE CASCADE,
    performance    REAL NOT NULL,
    rating_before  REAL NOT NULL,
    rating_after   REAL NOT NULL,
    collected      INTEGER NOT NULL,
    capsizes       INTEGER NOT NULL,
    pipes          INTEGER NOT NULL,
    PRIMARY KEY (run_id, player_id)
);

CREATE INDEX director_players_player_idx ON director_players (player_id, run_id);
