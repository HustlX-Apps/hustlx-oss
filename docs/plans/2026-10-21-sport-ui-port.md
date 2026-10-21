# Native sport console port

Approved scope: the seven existing OSS sports plus Football and Futsal. Swimming and Taekwondo desks are excluded. The current Nuxt develop UI is the rendering reference; OSS remains a Rust/Iced application with local SQLite authority and no account, network or subscription requirement.

## Design

Recreate the web operator layout using a compact scoreboard, sport-specific unit or clock strip, home and away participant panels, central action pad, and saved event feed. Volleyball uses the six-position court, per-player actions, rotation, substitutions and libero exchanges. Racquet sports retain their own point labels, service and game/set semantics. Football and Futsal gain separate local clock and event behavior, including draws. Basketball keeps its existing v2 console and receives a regression/parity check.

Reusable local teams are optional for non-basketball setup. When selected, copy players into immutable match-time snapshots. Named-side matches without rosters remain usable. Sport controls are durable commands with revision fencing, UUID retry and append-only undo. Rule/count deviations remain advisory. Unsupported actions must never appear as decorative working controls.

The alternatives were a cosmetic-only port over the existing point buttons, and a full server/client transplant. The selected native port includes the local behavior needed by its controls, without importing hosted APIs or JavaScript.

## Execution checklist

- [x] Confirm nine-sport scope and map source owners against fresh develop.
- [x] Establish clean domain/persistence test baseline on `codex/oss-sport-ui-port`.
- [x] Add tested Football/Futsal pure clock/event engine in `src/field_sport.rs` and `tests/field_sports.rs`.
- [x] Add tested sport participant actions, lineups, substitutions, volleyball rotation/libero bookkeeping in `src/operator.rs`, `src/sport.rs`, and `tests/operators.rs`.
- [x] Preserve old libraries while extending the sport catalog in schema 6; test migration, roster snapshots, retries, rollback and reopen in `tests/operator_storage.rs`.
- [x] Integrate football clocks into the same durable session lane, paused restart and interruption recovery; test deterministic time and failure paths.
- [x] Port native UI in `src/sport_ui.rs` and `src/operator_ui.rs`, wire setup and message handling, and exercise every sport in both themes.
- [x] Capture running-app before/after evidence at the 1100x760 minimum window.
- [x] Update support boundaries and operator/development/storage docs. Run `bash scripts/check.sh` locally and the local merge flagger, commit/push and open a develop PR. The runtime handoff URL is recorded in the evidence document.

## Source and verification

OSS base: `89b3d49`. Nuxt reference: `f2bf2b5aa`. API references remain read-only. The delegated sports track maps Football/Futsal and implements its isolated pure engine; the primary track owns persistence, operator actions, UI and integration. No hosted builds or releases are authorized.

Use failing behavior tests before domain/persistence changes. Run targeted `cargo +stable test --locked --no-default-features --test <suite>` during implementation. The final OSS local checker owns formatting, full workspace tests, Clippy, release-helper tests, native build and packaging preview. Native screenshots and manual score/undo/restart journeys supplement tests. macOS evidence does not certify Windows distribution.
