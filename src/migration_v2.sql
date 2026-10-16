ALTER TABLE players ADD COLUMN jersey TEXT NOT NULL DEFAULT '';
ALTER TABLE players ADD COLUMN positions TEXT NOT NULL DEFAULT '';
ALTER TABLE match_players ADD COLUMN positions TEXT NOT NULL DEFAULT '';
CREATE TABLE match_clocks (
    match_id INTEGER PRIMARY KEY REFERENCES matches(id),
    game_ms INTEGER NOT NULL DEFAULT 600000 CHECK(game_ms BETWEEN 0 AND 86400000),
    shot_ms INTEGER NOT NULL DEFAULT 24000 CHECK(shot_ms BETWEEN 0 AND 120000),
    period INTEGER NOT NULL DEFAULT 1 CHECK(period BETWEEN 1 AND 999),
    running INTEGER NOT NULL DEFAULT 0 CHECK(running IN (0,1)),
    shot_running INTEGER NOT NULL DEFAULT 0 CHECK(shot_running IN (0,1)),
    CHECK(shot_running=0 OR (running=1 AND shot_ms>0)),
    CHECK(running=0 OR game_ms>0)
);
INSERT INTO match_clocks(match_id) SELECT id FROM matches;
CREATE TABLE clock_commands (
    command_id TEXT PRIMARY KEY,
    match_id INTEGER NOT NULL REFERENCES matches(id),
    game_ms INTEGER NOT NULL,
    shot_ms INTEGER NOT NULL,
    period INTEGER NOT NULL,
    running INTEGER NOT NULL,
    shot_running INTEGER NOT NULL
);
