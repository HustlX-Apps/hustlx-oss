# Multi-sport local flow verification

Captured October 20, 2026 on macOS arm64 at the native 1100x760 content minimum (2200x1584 Retina images including the title bar). The isolated library was `/tmp/hustlx-oss-multi-sport-ui-baseline`; all displayed names are synthetic. No database is committed.

## Native journey

1. Captured the prior basketball-only setup, then launched the new binary against the same empty test library. The picker exposed basketball and all six new sports.
2. Created Volleyball local final, North Spikers versus South Spikers, at Community Court. Changed the house rules to best of one, target three, deciding target three and win by two. The native form remained scrollable and every action reachable.
3. Scored to 2-2, then 3-2. The match remained live. At 4-2 it became decided and required explicit completion. Completion displayed the saved winner and 1-0 set result.
4. Undid completion and then the deciding point. The match returned to 3-2 with no completed set. Re-scored and completed again. The history retained both undone actions and their correction records.
5. Quit the entire app, relaunched it and reopened the saved match. The completed 4-2 result, 1-0 set count, settings and correction history were preserved. Inspected the result and history in light and dark themes.
6. Created a Tennis singles match through the native setup, scored both sides to deuce, then scored the advantage point. The console rendered AD versus 40 and separate current-set game counts. Basketball controls and the Hotkeys guide were absent.

The first visual pass found a basketball-only Hotkeys button still shown on the sport console. It was removed and the new executable was used for the final result, history and tennis captures.

## Screenshots

- [Before: basketball-only setup](before.png)
- [Sport selection](sport-picker.png)
- [Completed volleyball result after restart](result-reopened.png)
- [Saved result in light theme](result-light.png)
- [Correction history after restart](correction-history.png)
- [Tennis advantage in light theme](tennis-advantage.png)

## Automated evidence

- `cargo fmt --all -- --check`: passed.
- `cargo test --locked --workspace --all-targets`: 83 tests passed, zero failed or ignored (5 native App, 44 existing basketball/domain/storage, 13 sport engine, 10 sport persistence, 11 packaging-tool tests).
- `cargo clippy --locked --workspace --all-targets -- -D warnings`: passed.
- `cargo build --locked`: passed on macOS arm64.
- `git diff --check`: passed.
- Umbrella `verify-changed.sh --strict origin/develop`: exit 0, no mapped Rust lane. Cargo supplies the runtime proof.

The engine suite covers volleyball deuce/decider, pickleball side-out and rally scoring, opening doubles service and odd-score service recovery, table-tennis service cadence/deuce/doubles decider/manual corrections, badminton cap and doubles positions, tennis advantage/no-ad/tiebreak, padel Star Point/match tiebreak and deterministic replay before every sport's final point.

Native App tests run the real message/session path for all six create/score/undo/complete/reopen journeys, invalid rules, failure/retry and basketball isolation. SQLite integration tests add exact retries, stale revisions, cross-family UUID collisions in both directions, same-name identity independence, invalid versions, rollback and schema-v3 preservation.

## Scope and remaining release evidence

The manual native journey covered volleyball and tennis. The other four sports have automated engine, persistence and App-message evidence, not separate manual match certifications. Runtime code uses local SQLite without a hosted service; the physical network adapter was not disabled during capture. Windows GUI, macOS Intel GUI, physical power loss and signed installer/upgrade certification remain platform release work. CI packages do not replace those checks.

The existing `block` 0.1.6 dependency emits Cargo's future-incompatibility notice; current tests, Clippy and build pass. Advanced player statistics, volleyball rotation/libero/substitution parity, real participant service-order editing, exports and cloud sync remain outside this approved slice.
