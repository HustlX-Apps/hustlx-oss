-- Rebuild only the parent table to widen the sport CHECK. Child IDs and history stay intact.
CREATE TABLE matches_v6 (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    date TEXT NOT NULL,
    venue TEXT NOT NULL DEFAULT '',
    home_team_id INTEGER NOT NULL REFERENCES teams(id),
    away_team_id INTEGER NOT NULL REFERENCES teams(id),
    home_name TEXT NOT NULL,
    away_name TEXT NOT NULL,
    revision INTEGER NOT NULL DEFAULT 0 CHECK(revision >= 0),
    sport TEXT NOT NULL DEFAULT 'basketball' CHECK(sport IN ('basketball','volleyball','pickleball','table-tennis','tennis','badminton','padel','football','futsal')),
    CHECK(home_team_id != away_team_id)
);
INSERT INTO matches_v6 SELECT id,name,date,venue,home_team_id,away_team_id,home_name,away_name,revision,sport FROM matches;
DROP TABLE matches;
ALTER TABLE matches_v6 RENAME TO matches;
