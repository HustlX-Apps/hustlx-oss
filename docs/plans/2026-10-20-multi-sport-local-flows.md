# Multi-sport local match flows

## Approved design

Build complete local match flows for Volleyball, Pickleball, Table Tennis, Tennis (including the Long Tennis name), Badminton and Padel in the existing Rust/Iced application. The approved scope includes configurable matches, scoring and service state, undo, explicit completion, saved results and restart recovery. Advanced player statistics, full web-console parity and cloud sync are later work.

SQLite remains the authority for accepted actions. A typed sport catalog and pure replayable scoring engine supply sport semantics. Add schema v4 metadata and a separate append-only sport command log while preserving existing basketball records. Every command shares the existing revision, UUID retry and commit-before-display guarantees. Reopening reconstructs scores, service, unit results and completion from immutable rules and events. Undo appends a correction; it never deletes history.

The native console follows the existing Nuxt scoreboard, unit strip, serve strip, action pad and feed patterns using Rust widgets. Each sport exposes only relevant settings and controls. Named sides can be created locally without league, account or network access. Basketball clocks, stat hotkeys and player positions must not leak into these consoles. Stable local identities and versioned commands preserve a future sync boundary, but no remote contract is invented.

Execution uses an isolated `codex/oss-multi-sport` branch from `origin/develop`. Runtime changes belong only to OSS; Nuxt and API are read-only references. Payments are unaffected. Cargo tests/builds and native UI evidence prove this slice; they do not certify Windows/macOS distribution or the hosted sports launch.

## Implementation checklist

- [x] Pure engine: `src/sport.rs`, `tests/sports.rs`, minimal library exports and serialization dependencies. Write boundary tests first, then implement sport-specific scoring/service transitions, valid configurable defaults, completion and serializable rules/commands. Cover deuce, cap, side-out, tiebreak, unit transitions and replay.
- [x] Persistence: `src/migration_v4.sql`, `src/sport_storage.rs`, `src/storage.rs`, `src/session.rs`, UUID checks in `src/lineup_storage.rs`, `tests/sport_storage.rs`. Add immutable settings, named side snapshots, transactional event append/undo, expected revisions, exact retries, v3 migration, failed-write rollback and reopen tests. Keep basketball creation compatible.
- [x] Native UI: `src/sport_ui.rs`, `src/library_ui.rs`, `src/ui.rs`, `src/clock_ui.rs`, focused desktop tests. Add sport/rules/side setup and sport console routing. Expose score, server, unit results, correction history, finish and undo. Disable basketball-only actions at both session and UI boundaries.
- [x] Integration: run all six create/score/undo/finish/reopen journeys, legacy basketball regression, retry/collision/rollback cases and smallest-window rendering in both themes.
- [x] Documentation and handoff: update product brief, README, architecture, storage, development, roadmap and sport guide. Record supported settings and deferred features precisely. Commit, push and open PR to develop with required evidence headings.

## Verification

Run `/Users/jaylupos/.cargo/bin/cargo fmt --all -- --check`, `cargo test --locked --workspace --all-targets`, `cargo clippy --locked --workspace --all-targets -- -D warnings` and `cargo build --locked` from this worktree (use the absolute Cargo path where shell PATH omits it). Use `cargo test --locked --no-default-features` for fast domain/persistence iterations. Native smoke uses an isolated `HUSTLX_OSS_DATA_DIR`, never the operator's library. Preserve before/after screenshots. Run the umbrella merge flagger with the actual PR body; its generic verifier has no Rust lane and cannot replace Cargo checks.

Runtime handoff: [PR #5](https://github.com/HustlX-Apps/hustlx-oss/pull/5). Local evidence is in [the native verification record](../evidence/multi-sport/README.md). Publishing and cross-platform native certification are separate release actions.
