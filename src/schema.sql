CREATE TABLE teams (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL CHECK(length(trim(name)) > 0),
    archived INTEGER NOT NULL DEFAULT 0 CHECK(archived IN (0,1))
);
CREATE TABLE players (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL CHECK(length(trim(name)) > 0),
    archived INTEGER NOT NULL DEFAULT 0 CHECK(archived IN (0,1))
);
CREATE TABLE team_players (
    team_id INTEGER NOT NULL REFERENCES teams(id),
    player_id INTEGER NOT NULL REFERENCES players(id),
    jersey TEXT NOT NULL DEFAULT '',
    PRIMARY KEY(team_id, player_id)
);
CREATE TABLE matches (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    date TEXT NOT NULL,
    venue TEXT NOT NULL DEFAULT '',
    home_team_id INTEGER NOT NULL REFERENCES teams(id),
    away_team_id INTEGER NOT NULL REFERENCES teams(id),
    home_name TEXT NOT NULL,
    away_name TEXT NOT NULL,
    revision INTEGER NOT NULL DEFAULT 0 CHECK(revision >= 0),
    CHECK(home_team_id != away_team_id)
);
CREATE TABLE match_players (
    match_id INTEGER NOT NULL REFERENCES matches(id),
    side TEXT NOT NULL CHECK(side IN ('home','away')),
    player_id INTEGER NOT NULL REFERENCES players(id),
    name TEXT NOT NULL,
    jersey TEXT NOT NULL,
    on_court INTEGER NOT NULL CHECK(on_court IN (0,1)),
    sort_order INTEGER NOT NULL,
    PRIMARY KEY(match_id,side,player_id)
);
CREATE TABLE match_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    match_id INTEGER NOT NULL REFERENCES matches(id),
    command_id TEXT NOT NULL UNIQUE,
    kind TEXT NOT NULL CHECK(kind IN ('record','undo')),
    side TEXT,
    player_id INTEGER,
    stat TEXT,
    target_id INTEGER UNIQUE,
    UNIQUE(match_id,id),
    FOREIGN KEY(match_id,side,player_id) REFERENCES match_players(match_id,side,player_id),
    FOREIGN KEY(match_id,target_id) REFERENCES match_events(match_id,id),
    CHECK((kind='record' AND side IS NOT NULL AND player_id IS NOT NULL AND stat IS NOT NULL AND target_id IS NULL)
       OR (kind='undo' AND target_id IS NOT NULL AND side IS NULL AND player_id IS NULL AND stat IS NULL))
);
CREATE INDEX match_events_match_order ON match_events(match_id,id);
CREATE INDEX team_players_player ON team_players(player_id);
