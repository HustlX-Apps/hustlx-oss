-- Existing match rows and basketball history retain their identities and meaning.
ALTER TABLE matches ADD COLUMN sport TEXT NOT NULL DEFAULT 'basketball'
    CHECK(sport IN ('basketball','volleyball','pickleball','table-tennis','tennis','badminton','padel'));
CREATE TABLE sport_matches (
    match_id INTEGER PRIMARY KEY REFERENCES matches(id),
    local_id TEXT NOT NULL UNIQUE,
    rules_version INTEGER NOT NULL CHECK(rules_version=1),
    rules_json TEXT NOT NULL
);
CREATE TABLE sport_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    match_id INTEGER NOT NULL REFERENCES sport_matches(match_id),
    command_id TEXT NOT NULL UNIQUE,
    payload_version INTEGER NOT NULL CHECK(payload_version=1),
    kind TEXT NOT NULL CHECK(kind IN ('record','undo')),
    payload TEXT,
    target_id INTEGER UNIQUE,
    UNIQUE(match_id,id),
    FOREIGN KEY(match_id,target_id) REFERENCES sport_events(match_id,id),
    CHECK((kind='record' AND payload IS NOT NULL AND target_id IS NULL)
       OR (kind='undo' AND payload IS NULL AND target_id IS NOT NULL))
);
CREATE INDEX sport_events_match_order ON sport_events(match_id,id);
