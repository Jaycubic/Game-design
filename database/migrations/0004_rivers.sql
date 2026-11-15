-- Rivers: each player's progress on each river. Stars are kept as a bitmask
-- (1 = clean river, 2 = score target, 4 = the river's own goal) and are never
-- lost once earned. best_score is the best crew score on that river.

CREATE TABLE player_rivers (
    player_id   UUID NOT NULL REFERENCES players (id) ON DELETE CASCADE,
    river       TEXT NOT NULL CHECK (river IN ('canal', 'mangrove', 'gorge')),
    stars       SMALLINT NOT NULL DEFAULT 0 CHECK (stars BETWEEN 0 AND 7),
    best_score  INTEGER NOT NULL DEFAULT 0,
    runs        INTEGER NOT NULL DEFAULT 0 CHECK (runs >= 0),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (player_id, river)
);
