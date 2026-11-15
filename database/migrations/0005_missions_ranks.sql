-- Daily missions and ranks.
--
-- Missions: three per player per UTC day, drawn from a fixed pool (see
-- crates/game-server/src/missions.rs). Progress counts across runs and modes
-- (practice counts, the tutorial doesn't). A mission pays its Seeds once, the
-- moment it is completed (completed_at records when).
CREATE TABLE player_missions (
    player_id     UUID NOT NULL REFERENCES players (id) ON DELETE CASCADE,
    day           DATE NOT NULL,
    slot          SMALLINT NOT NULL CHECK (slot BETWEEN 0 AND 2),
    mission       TEXT NOT NULL,
    progress      INTEGER NOT NULL DEFAULT 0 CHECK (progress >= 0),
    goal          INTEGER NOT NULL CHECK (goal > 0),
    reward        INTEGER NOT NULL CHECK (reward > 0),
    completed_at  TIMESTAMPTZ,
    PRIMARY KEY (player_id, day, slot)
);

-- Ranks are the skill rating shown as a name. The shown rank may drop by at
-- most one step per UTC day: rank_day_start is the rank the day began with.
ALTER TABLE player_skill
    ADD COLUMN shown_rank      SMALLINT NOT NULL DEFAULT 1 CHECK (shown_rank BETWEEN 0 AND 4),
    ADD COLUMN rank_day        DATE,
    ADD COLUMN rank_day_start  SMALLINT;

-- Existing players start from the rank their rating already earns.
UPDATE player_skill SET shown_rank = CASE
    WHEN rating >= 1350 THEN 4
    WHEN rating >= 1200 THEN 3
    WHEN rating >= 1050 THEN 2
    WHEN rating >= 900 THEN 1
    ELSE 0 END;
