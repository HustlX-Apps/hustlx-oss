# Architecture proposal

Status: Rust-rendered UI throughout was selected by the user on October 16, 2026. Iced is the initial toolkit. The persistence and production clock design below remains proposed. The first implementation is a deliberately in-memory sample console.

## UI decision

The UI must be written in Rust and rendered by the desktop application. Use Iced 0.14 with custom widgets/styles to follow the existing v2 console. Keep the domain model independent of Iced. There is no webview or JavaScript runtime.

| Rust UI option | Fit | Tradeoff |
| --- | --- | --- |
| Iced, selected for the first build | Rust widgets, custom styling, and explicit message/update/view boundaries | The v2 presentation must be recreated; the framework is still evolving |
| egui with eframe | Straightforward immediate-mode Rust UI and custom painting | Would require more deliberate styling to move away from a tooling interface |

References checked October 16, 2026: [Iced 0.14 documentation](https://docs.rs/iced/0.14.0/iced/) and [egui repository](https://github.com/emilk/egui). Framework support is not evidence that our Windows/macOS builds have passed.

## First implementation boundary

`src/lib.rs` owns a synthetic match, stable player IDs, event-derived player/team statistics, and reversible preview actions. `src/ui.rs` owns the Iced interface. The preview uses memory only, displays its unsaved state persistently, and shows a fixed clock marked as a preview. It does not claim production game-day operation, clock reliability, or durable storage. License selection remains pending.

## Proposed boundaries

- **Desktop UI:** teams, players, match library, setup, Game Day console, and local results. Package assets with the app and use no hosted page as the application shell.
- **Rust domain core:** match commands, score/stat projections, lineups, undo/corrections, and clock state. Keep domain behavior testable without the renderer.
- **Rust persistence:** SQLite transactions, versioned migrations, match-time roster snapshots, and export/backup operations. The UI should not write arbitrary SQL or arbitrary filesystem paths.
- **Future sync adapter:** a separate optional integration. No network call belongs in the local command commit path.

## Local data and commands

Proposed records are players, teams, roster memberships, matches, match roster snapshots, match events, and clock checkpoints. Use stable local identifiers. Keep future remote mappings separate rather than replacing local identity.

A UI action should call a typed Rust command. Validate the command against local state, atomically store its event and resulting state, then return the committed projection to the UI. An action identifier should prevent accidental duplicate application during retries. Corrections must preserve the original audit trail.

SQLite is the proposed durable source of truth. The in-memory preview is not a match record and must be replaced by durable storage before real game use. Define schema migrations and backup/restore compatibility before users accumulate real data.

## Clock and recovery

Use a Rust monotonic time source while the process runs. Rendering ticks must not determine elapsed game time. Persist running/paused state, remaining time, period, and a recovery checkpoint. Explicitly define what happens after sleep, restart, or an uncertain recovery interval before implementing the clock. Prefer operator confirmation when elapsed time cannot be established safely.

## Failures and exports

On a failed save, keep the command visibly pending or show a retryable error; do not silently lose it. Export a consistent snapshot, let the operator select the destination, and surface filesystem failures. Backups must include schema version and sufficient match history to restore the record.

## Verification to establish during implementation

Rust tests should cover scoring, undo, duplicate commands, atomic persistence, recovery, and clock calculations. UI tests should cover Team A versus Team B setup, player attribution, local results, and export errors. Windows and macOS builds and native smoke tests are separate release gates. Verify launch, game operation, close/reopen, and export with the network disconnected.
