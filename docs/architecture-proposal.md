# Architecture proposal

Status: proposed, pending maintainer approval. No runtime code has been scaffolded.

## Options

| Approach | Benefit | Tradeoff |
| --- | --- | --- |
| Tauri 2 + Rust + SQLite + Vue | Keeps Rust responsible for local game operations while using familiar web styling to closely match Game Day v2 | UI is Vue/TypeScript; Windows and macOS webviews need separate validation |
| Rust-rendered UI + SQLite | Uses Rust for the interface as well as domain logic | Requires rebuilding the Vue/CSS presentation and more work to match the existing console |

Recommendation: Tauri 2 with Vue, TypeScript, and a Rust domain core. Tauri supports Rust logic with HTML/CSS/JavaScript frontends and desktop distribution. This is an architectural recommendation, not a claim of built or tested platform support.

References checked October 16, 2026: [Tauri overview](https://v2.tauri.app/start/) and [distribution](https://v2.tauri.app/distribute/).

## Proposed boundaries

- **Desktop UI:** teams, players, match library, setup, Game Day console, and local results. Package assets with the app and use no hosted page as the application shell.
- **Rust domain core:** match commands, score/stat projections, lineups, undo/corrections, and clock state. Keep domain behavior testable without a webview.
- **Rust persistence:** SQLite transactions, versioned migrations, match-time roster snapshots, and export/backup operations. The UI should not write arbitrary SQL or arbitrary filesystem paths.
- **Future sync adapter:** a separate optional integration. No network call belongs in the local command commit path.

## Local data and commands

Proposed records are players, teams, roster memberships, matches, match roster snapshots, match events, and clock checkpoints. Use stable local identifiers. Keep future remote mappings separate rather than replacing local identity.

A UI action should call a typed Rust command. Validate the command against local state, atomically store its event and resulting state, then return the committed projection to the UI. An action identifier should prevent accidental duplicate application during retries. Corrections must preserve the original audit trail.

SQLite is the proposed durable source of truth. Browser storage should not be the only match record. Define schema migrations and backup/restore compatibility before users accumulate real data.

## Clock and recovery

Use a Rust monotonic time source while the process runs. Rendering ticks must not determine elapsed game time. Persist running/paused state, remaining time, period, and a recovery checkpoint. Explicitly define what happens after sleep, restart, or an uncertain recovery interval before implementing the clock. Prefer operator confirmation when elapsed time cannot be established safely.

## Failures and exports

On a failed save, keep the command visibly pending or show a retryable error; do not silently lose it. Export a consistent snapshot, let the operator select the destination, and surface filesystem failures. Backups must include schema version and sufficient match history to restore the record.

## Verification to establish during implementation

Rust tests should cover scoring, undo, duplicate commands, atomic persistence, recovery, and clock calculations. UI tests should cover Team A versus Team B setup, player attribution, local results, and export errors. Windows and macOS builds and native smoke tests are separate release gates. Verify launch, game operation, close/reopen, and export with the network disconnected.
