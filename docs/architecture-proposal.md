# Architecture and proposals

Status: Rust-rendered UI throughout was selected by the user on October 16, 2026. Iced is the initial toolkit. SQLite persistence, the local library, native clocks, supported shortcuts, lineups and substitutions are implemented. Complete official game operation, export/backup tooling, and syncing remain planned.

## UI decision

The UI must be written in Rust and rendered by the desktop application. Use Iced 0.14 with custom widgets/styles to follow the existing v2 console. Keep the domain model independent of Iced. There is no webview or JavaScript runtime.

| Rust UI option | Fit | Tradeoff |
| --- | --- | --- |
| Iced, selected for the first build | Rust widgets, custom styling, and explicit message/update/view boundaries | The v2 presentation must be recreated; the framework is still evolving |
| egui with eframe | Straightforward immediate-mode Rust UI and custom painting | Would require more deliberate styling to move away from a tooling interface |

References checked October 16, 2026: [Iced 0.14 documentation](https://docs.rs/iced/0.14.0/iced/) and [egui repository](https://github.com/emilk/egui). Framework support is not evidence that our Windows/macOS builds have passed.

## Implemented boundaries

- `src/lib.rs` owns stable player identity, side-scoped event-derived player/team statistics, selection, and undo projection. It has no renderer dependency.
- `src/storage.rs`, `src/schema.sql` and `src/migration_v2.sql` through `src/migration_v6.sql` own SQLite schema version 6, catalog operations, match creation, snapshots, and record/undo transactions. Form values use bound parameters; the UI does not issue SQL.
- `src/session.rs` owns the active committed match and one pending command. A write failure leaves visible stats and selection unchanged. Retry uses the same UUID.
- `src/ui.rs` and `src/library_ui.rs` own native Iced navigation, forms, and the v2 console. SQLite calls run synchronously with a bounded 250 ms busy timeout. Larger libraries may require background loading later.

SQLite is the durable source of truth. Read transactions load metadata, rosters, and history consistently. Write transactions append a stat or undo event and increment the match revision atomically before returning a new projection. Revision checks reject stale writes; exact UUID retries do not add another event. Undo references the original event and preserves the audit trail.

Catalog players and teams have local integer IDs; names and jersey labels are not keys. Match creation copies active team names and memberships into per-side snapshots. Renaming, archiving, or changing catalog memberships cannot rewrite saved match attribution. A player may belong to multiple teams, including both sides; the match side and player ID identify the actor together. Empty rosters are valid.

The database uses foreign keys, WAL, and synchronous FULL. First open creates schema 1 and migrates through schema 6 transactionally. Newer, foreign, and corrupt databases fail visibly without a reset. The additive migrations preserve existing events, snapshots, court flags and clock truth. Schema 3 adds lineup receipts/history without inferring unknown starters. Schema 4 adds the standalone multi-sport tables. Schema 5 adds pre-action clock context to stat events; legacy timing stays unknown. No downgrade path exists; preserve a pre-upgrade backup. See [storage](storage.md) for paths, schema, and preservation boundaries.

## Future boundaries

- **Results/export:** consistent box scores, filesystem destination selection, portable backups, and tested restores.
- **Sync adapter:** optional remote mapping and commands. No network call belongs in the local commit path, and local identity must survive linking. No hosted contract is assumed or implemented.

## Clock and recovery

`src/clock.rs` derives elapsed milliseconds from monotonic `Instant` anchors. UI ticks redraw at 100 ms; they do not subtract fixed increments. `src/session.rs` routes clocks and stats through the same expected revision and commit lane. Stat side effects and clock snapshots commit atomically. Every command and five-second running checkpoint stores the baseline. Graceful close pauses before closing; AppKit Quit is cancelled and forwarded to this same path through `src/native_quit.rs`. That bridge adds the public optional applicationShouldTerminate delegate method, and refuses to overwrite a future toolkit handler. Failure leaves the window open for retry. Abrupt recovery reopens the last checkpoint paused. Gaps over two seconds or wall/monotonic divergence freeze at the last observed state with a notice. See [clock policy](clocks-hotkeys.md) for supported controls and limits.

For stat entry, the pending command holds the pre-action clock projection. Storage appends that immutable snapshot to the event and applies stat clock effects to the current baseline in the same transaction. `Play::clock` is optional because legacy and clock-free synthetic events have no trustworthy timing. Play-by-play shows the saved period/game/shot context while retaining append order. No quarter totals, minutes or plus-minus are derived yet.

## Failures and exports

Failed stat/undo saves remain visibly pending with an explicit retry. Pending commands are currently memory-only and do not survive an abrupt exit before they commit. Reopening a match explicitly discards its pending command and reloads committed truth. Export a consistent snapshot, let the operator select the destination, and surface filesystem failures. Backups must include schema version and sufficient match history to restore the record.

## Verification boundaries

Rust tests cover scoring, durable undo, duplicate commands, stale revisions, atomic persistence, and failed-save display behavior. Clock tests cover first expiry, independent shot pause, exact checkpoints, close/reopen, uncertain intervals, stale/duplicate commands, and atomic stat/clock failure. UI tests should cover Team A versus Team B setup, player attribution, local results, and export errors. Windows and macOS builds and native smoke tests are separate release gates. Verify launch, game operation, close/reopen, and export with the network disconnected.

## Lineups and substitutions

`src/lineup.rs` owns ordered court selections, structural validation and local editor drafts. `src/lineup_storage.rs` writes the current court projection and an append-only before/after receipt with clock context in one transaction and one shared revision. Lineup UUIDs cannot collide with stat or clock commands. `SavedMatch::starting_lineup` reads the latest explicit starter selection separately from current on-court flags; substitutions never rewrite it. Side plus player ID remains identity. `src/lineup_ui.rs` provides the v2-style confirmation board and saved history. Count deviations are advisory; no rule/claim gates are added. See [operator guide](lineups-substitutions.md).

## Standalone sport scoring

`src/sport.rs` owns the typed local sport catalog, immutable rules and deterministic scoring/service transitions. `src/sport_storage.rs` persists versioned sport commands separately from basketball player statistics. `Session` routes sport commands and undo through its existing pending-save lane. `src/sport_ui.rs` owns setup and a native score/result console selected from the saved sport. Basketball clocks, hotkeys and lineups are guarded from these matches at the UI, session and store boundaries.

The implementation reads Nuxt/API behavior as a reference and implements it natively; it imports no web runtime. Eight additional sports operate on named sides with optional immutable catalog roster snapshots. `src/operator.rs` owns participant actions, explicit court/service order, volleyball rotation, exchanges and local counts. `src/field_sport.rs` owns football count-up, futsal countdown, field events, extra periods and draw-aware completion. `src/operator_ui.rs` and `src/field_ui.rs` render the ported native operator layout. Full hosted workflow parity, statistical reports and sync remain deferred. See [the sport guide](multi-sport.md).

## Schema 6 and field timing

Schema 6 widens the saved sport constraint to Football and Futsal using SQLite's [documented table rebuild](https://www.sqlite.org/lang_altertable.html#making_other_kinds_of_table_schema_changes). Foreign keys are disabled before the migration transaction; the parent table is copied/rebuilt without renaming the old parent first, child IDs and receipts are preserved, indexes/triggers and the autoincrement high-water mark are restored, and foreign keys are checked before commit and enabled afterward. Preserve a backup before upgrades; there is no downgrade path.

`Command::FieldAt` contains the pre-action clock baseline and field command in one sport receipt. Session projection uses monotonic elapsed time, checkpoints every five seconds, and saves paused on close or uncertain intervals. Reopening a running baseline pauses it without deducting time while closed. Undo excludes clock-only receipts and retracts the domain action while retaining its captured time paused. Do not omit the whole timed receipt on replay: that would rewind the clock or resume it after undoing completion.
