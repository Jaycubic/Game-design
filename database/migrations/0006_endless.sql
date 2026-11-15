-- Solo Endless: each player's best Endless score (metres + plastic x 5) per river.
ALTER TABLE player_rivers ADD COLUMN best_endless INTEGER NOT NULL DEFAULT 0;
