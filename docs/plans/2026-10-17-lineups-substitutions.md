# Lineups and Substitutions Implementation Plan

**Goal:** Choose starting lineups and confirm v2-style substitutions, retaining match-specific lineups and history across close/reopen.

**Architecture:** Rust owns ordered side-scoped court selections and editable drafts. SQLite schema 3 adds append-only lineup command receipts/history; existing match_players on_court/sort_order store the live projection. Starting-lineup receipts preserve the selected starters separately from current participation. Commands commit roster changes, clock snapshot and shared match revision atomically using the existing pending/retry lane. New matches start with empty courts; migration preserves existing court flags and never infers unknown starters.

**Tech stack:** Rust, Iced 0.14, SQLite/rusqlite. No new dependencies.

Repo set: hustlx-oss only; Nuxt v2 substitution board/hotkey source is read-only reference. Source of truth: local Rust/SQLite. Execution: terminal and native UI. Verification: targeted Rust tests, formatter, Clippy/build, native saved-lineup/substitution/restart/failure journey, shared verifier boundary and merge flagger, Windows/macOS CI. Documentation impact: local storage/architecture/shortcut guides, roadmap and native evidence. League game-day workflow; payment boundary unaffected, no Xendit or hosted integration. The API/Nuxt multi-sport, clock and claim harnesses are outside this Rust-only diff.

## Behavior

- Per-side Starters and Subs controls; editor uses court/bench columns, clear/add/remove, select outgoing then incoming to swap at the same slot, Save/Confirm and Cancel. Drafts do not mutate committed lineups; reopening/navigation/close discards unconfirmed drafts. Pending saves remain blocked/retryable.
- Typical court size is five; other counts warn without blocking, including empty/short-handed rosters. No automatic rule/claim/ejection gate. Side plus player ID remains identity, including a player present on both rosters.
- Starting saves set both starting selection and current court; later substitutions update only current court. Clock continues during the editor and is snapshotted without start/pause/reset side effects on commit. Cancel never writes. Stat Undo stays stat-only; reverse a substitution with another confirmed swap.
- Every lineup command has UUID, kind, side, exact ordered court target, before lineup and clock context. Exact retries return committed state; payload reuse, cross-kind collisions, duplicate IDs, foreign players and stale revisions fail. Any failure rolls back all writes and preserves visible court/stats until Retry or explicit reopen/discard.
- X opens substitutions for typed/clicked player's side or explicit comma/period side. The existing jersey guards remain; the native board handles confirmation. O/timeouts remains unbound. Scoring shortcuts cannot leak through editor/history.

## Checklist

- [ ] Write failing persistence/draft/session tests for scope, snapshots, migration, restart, idempotency, concurrency and atomic failure.
- [ ] Implement lineup model/schema 3/repository/session commands; pass targeted tests.
- [ ] Add native editor, live starter markers, saved history and guarded X shortcut; build and render.
- [ ] Verify native multi-player swaps/restart/cancel/save failure, both themes/minimum size; document evidence and limits.
- [ ] Run required checks, push branch/update PR to develop, assess Windows/macOS CI.
