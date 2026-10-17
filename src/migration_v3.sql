-- Keep existing match_players flags/order exactly as saved. Earlier versions
-- did not record operator-confirmed starters, so no starter history is inferred.
CREATE TABLE lineup_commands (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    command_id TEXT NOT NULL UNIQUE,
    match_id INTEGER NOT NULL REFERENCES matches(id),
    side TEXT NOT NULL CHECK(side IN ('home','away')),
    kind TEXT NOT NULL CHECK(kind IN ('starters','substitution')),
    before_players TEXT NOT NULL,
    court_players TEXT NOT NULL,
    game_ms INTEGER NOT NULL,
    shot_ms INTEGER NOT NULL,
    period INTEGER NOT NULL,
    running INTEGER NOT NULL,
    shot_running INTEGER NOT NULL
);
CREATE INDEX lineup_commands_match_order ON lineup_commands(match_id,id);
