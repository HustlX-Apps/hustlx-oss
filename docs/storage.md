# Local storage

## Using the library

Create reusable players on Players, then teams and memberships on Teams. Teams and players can be renamed, archived, and restored. Enable Show archived to restore an identity. Removing a membership leaves the player and historical matches intact. Archiving a player hides their memberships from new matches; restoring them makes those memberships available again.

Choose two different active teams on Matches. Enter a match name and real calendar date as YYYY-MM-DD. Venue is optional. Duplicate display names and jersey labels are allowed; selectors show the local identity number. A team may have an empty roster. No account, league, claim, subscription, or network check gates this flow.

Create and open match commits its metadata and roster snapshots together. Reopen through Matches after restarting the app. Names, jerseys, and active memberships are copied at creation, so library edits affect future matches only. The first five players are initially on court. Editing a saved roster or its match metadata is not implemented yet.

## Data location

SQLite is bundled with the Rust executable. Normal data locations are:

| Platform | Database |
| --- | --- |
| macOS | `~/Library/Application Support/com.HustlX.hustlx-oss/hustlx.sqlite3` |
| Windows | `%LOCALAPPDATA%\HustlX\hustlx-oss\data\hustlx.sqlite3` |

`HUSTLX_OSS_DATA_DIR` overrides the directory, not the filename. It must be an absolute path. Use separate directories for manual testing. The application's bundle identifier does not select the database; `Store::default_path` does.

The application creates the directory on first launch and starts with an empty catalog. The database and its journal sidecars belong to the operator and must never be committed to Git. No data is sent to HustlX. There is no cloud backup or database encryption in this slice.

## Schema version 1

| Table | Responsibility |
| --- | --- |
| `teams`, `players` | Reusable identities, names, archive flags |
| `team_players` | Membership and jersey label per team/player pair |
| `matches` | Name, date, venue, source team IDs, snapshot team names, revision |
| `match_players` | Names, jerseys, on-court flags, and order copied per match side/player |
| `match_events` | Append-only record/undo commands, UUIDs, actors, stats, and undo targets |

Foreign keys scope events to a match roster and undo targets to their match. SQLite application ID `0x48584f53` identifies this app; `user_version=1` identifies the schema. First schema creation is transactional. This version refuses newer/foreign schemas and existing unrecognized tables. It does not rewrite, downgrade, or replace them. Future schema changes need explicit migrations and compatibility tests. Rollback of this first development slice means using the prior binary with its own data directory; it cannot read this library.

## Save and retry behavior

A stat/undo command carries a UUID and expected match revision. An immediate transaction checks identity and revision, appends the event, updates the revision, loads the projection, and commits before the UI changes. Exact command retries return saved state without applying the stat twice. Reusing a UUID for a different command fails. Undo adds an event pointing at the last remaining recorded play; it does not delete history.

WAL and synchronous FULL are enabled. Another writer can make the database busy; calls wait at most 250 ms before surfacing a storage error. If a stat save fails, the old score and player selection remain, the footer shows SAVE PENDING, and further stat entry is disabled. Retry save reuses the same command. A stale-window error requires reopening from Matches to load current committed state.

Pending commands are held in memory only. Quitting before a failed save is retried loses that uncommitted intent. Opening a saved match explicitly discards the pending command and restores committed state. There is no automatic pending-command recovery yet. Catalog/match form validation errors preserve the draft; successful writes clear it.

An unavailable or incompatible library shows an error and retry control. It never falls back to an empty in-memory match. The app does not automatically delete or repair a damaged database.

## Preservation and planned backups

There is no in-app backup/restore or statistics export yet. For a manual safety copy, close every app instance using the library, then copy the entire data directory to a safe location. Preserve any `-wal` and `-shm` sidecars along with the database; copying only the main file while it is open can omit committed data. Do not overwrite your only copy while troubleshooting. Automated backup integrity and restore verification remain future work.
