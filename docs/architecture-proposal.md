# Architecture and proposals

Status: Rust-rendered UI throughout was selected by the user on October 16, 2026. Iced is the initial toolkit. SQLite persistence and the local library are implemented. Production clocks, export/backup tooling, and syncing remain proposed.

## UI decision

The UI must be written in Rust and rendered by the desktop application. Use Iced 0.14 with custom widgets/styles to follow the existing v2 console. Keep the domain model independent of Iced. There is no webview or JavaScript runtime.

| Rust UI option | Fit | Tradeoff |
| --- | --- | --- |
| Iced, selected for the first build | Rust widgets, custom styling, and explicit message/update/view boundaries | The v2 presentation must be recreated; the framework is still evolving |
| egui with eframe | Straightforward immediate-mode Rust UI and custom painting | Would require more deliberate styling to move away from a tooling interface |

References checked October 16, 2026: [Iced 0.14 documentation](https://docs.rs/iced/0.14.0/iced/) and [egui repository](https://github.com/emilk/egui). Framework support is not evidence that our Windows/macOS builds have passed.

## Implemented boundaries

- `src/lib.rs` owns stable player identity, side-scoped event-derived player/team statistics, selection, and undo projection. It has no renderer dependency.
- `src/storage.rs` and `src/schema.sql` own SQLite schema version 1, catalog operations, match creation, snapshots, and record/undo transactions. Form values use bound parameters; the UI does not issue SQL.
- `src/session.rs` owns the active committed match and one pending command. A write failure leaves visible stats and selection unchanged. Retry uses the same UUID.
- `src/ui.rs` and `src/library_ui.rs` own native Iced navigation, forms, and the v2 console. SQLite calls run synchronously with a bounded 250 ms busy timeout. Larger libraries may require background loading later.

SQLite is the durable source of truth. Read transactions load metadata, rosters, and history consistently. Write transactions append a stat or undo event and increment the match revision atomically before returning a new projection. Revision checks reject stale writes; exact UUID retries do not add another event. Undo references the original event and preserves the audit trail.

Catalog players and teams have local integer IDs; names and jersey labels are not keys. Match creation copies active team names and memberships into per-side snapshots. Renaming, archiving, or changing catalog memberships cannot rewrite saved match attribution. A player may belong to multiple teams, including both sides; the match side and player ID identify the actor together. Empty rosters are valid.

The database uses foreign keys, WAL, and synchronous FULL. First open creates schema version 1 transactionally. Newer, foreign, and corrupt databases fail visibly without a reset. There is no legacy real-data migration or downgrade path in this first schema. See [storage](storage.md) for paths, schema, and preservation boundaries.

## Future boundaries

- **Clock core:** monotonic clock commands, running/paused checkpoints, sleep/restart policy, and operator recovery.
- **Results/export:** consistent box scores, filesystem destination selection, portable backups, and tested restores.
- **Sync adapter:** optional remote mapping and commands. No network call belongs in the local commit path, and local identity must survive linking. No hosted contract is assumed or implemented.

## Clock and recovery

Use a Rust monotonic time source while the process runs. Rendering ticks must not determine elapsed game time. Persist running/paused state, remaining time, period, and a recovery checkpoint. Explicitly define what happens after sleep, restart, or an uncertain recovery interval before implementing the clock. Prefer operator confirmation when elapsed time cannot be established safely.

## Failures and exports

Failed stat/undo saves remain visibly pending with an explicit retry. Pending commands are currently memory-only and do not survive quitting before they commit. Reopening a match explicitly discards its pending command and reloads committed truth. Export a consistent snapshot, let the operator select the destination, and surface filesystem failures. Backups must include schema version and sufficient match history to restore the record.

## Verification boundaries

Rust tests cover scoring, durable undo, duplicate commands, stale revisions, atomic persistence, and failed-save display behavior. Clock recovery/calculations remain unimplemented and unverified. UI tests should cover Team A versus Team B setup, player attribution, local results, and export errors. Windows and macOS builds and native smoke tests are separate release gates. Verify launch, game operation, close/reopen, and export with the network disconnected.
