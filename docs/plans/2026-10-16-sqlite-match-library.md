# SQLite Match Library Implementation Plan

**Goal:** Manage local teams and players, create a named and dated Team A versus Team B match with a venue, and reopen its saved roster and statistics after restarting the native app.

**Architecture:** Extend the approved Rust/Iced application with a bundled SQLite database. Normalize the reusable team/player catalog, snapshot names and rosters when a match is created, and append record/undo events transactionally. Commit before replacing visible state. Keep clocks and hosted syncing outside this slice.

**Tech stack:** Rust, Iced, rusqlite with bundled SQLite, directories for the OS data location, chrono for calendar dates, and UUID command IDs for safe retries.

## Design

Use normalized local tables rather than a serialized library blob. This gives membership constraints and atomic event writes without rewriting unrelated matches. A serialized document would be simpler initially but would make cross-window updates and independent catalog edits harder to protect.

Start on Matches, with Teams and Players tabs. Teams support create/rename/archive/restore and adding/removing existing players with jersey labels. Players support create/rename/archive/restore. Archives hide identities from new setup, preserve saved matches, and are reversible. Names and jersey labels are never identity keys.

The match form requires two distinct active teams, a name, and a real YYYY-MM-DD date. Venue is optional. Empty rosters are allowed; missing rules and identity claims never gate local match creation. The match copies active memberships and display names into immutable match rosters, with the first five players on court initially. Lineup editing is a later slice.

Store each stat and undo as a new event. Undo references the original event rather than deleting history. Use an expected match revision to reject stale-window writes and a unique command ID to make exact retries safe. A failed save keeps the visible match and player selection unchanged and shows a persistent error. Opening an unsupported or corrupt database must fail visibly without replacing it with an empty one.

Use the OS application data directory and a documented `HUSTLX_OSS_DATA_DIR` override for isolated development. Bundle SQLite so users do not install a database server. This is local storage, not cloud backup or encryption.

## Tasks and evidence

- [x] Write failing integration cases for catalog CRUD, roster membership, date/team validation, match snapshots, disk reopen, durable record/undo, duplicate retries, stale revisions, and write failure rollback.
- [x] Implement transactional schema version 1 and the repository in `src/storage.rs`; adapt the renderer-independent model to loaded rosters.
- [x] Add native Matches, Teams, Players, and Game Day navigation/forms in `src/ui.rs` and `src/library_ui.rs`. Remove the sample-only startup and unsaved notice for stored matches.
- [x] Exercise the real native create/reopen journey using an isolated database and capture the library and a reopened match.
- [x] Update architecture, development/storage guidance, roadmap, and verification evidence.
- [x] Run formatting, Cargo tests, Clippy, build, shared verifier boundary, and merge flagger. Push/update PR #1 targeting develop and assess Windows/macOS CI.

No existing real database or private HustlX runtime is migrated. Future schema versions must be rejected until a corresponding migration is implemented. Game/shot clocks remain clearly marked as a preview, and exports, backups/restore, lineup editing, and paid-league sync remain planned.
