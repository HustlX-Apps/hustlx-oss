# Clock-stamped event evidence

Recorded October 20, 2026 on macOS Apple Silicon from branch `codex/oss-clock-stamped-events`, implementation commit `80fc40f`, [PR #4](https://github.com/HustlX-Apps/hustlx-oss/pull/4). Synthetic teams/players only, in an isolated temporary library. Screenshots show the running Iced application at its 1100-point minimum width, 2200-pixel Retina captures; no browser or hosted service was used.

## Native journey

1. Opened a disposable schema 3 library containing one old made three and a paused period 2 clock at exactly 310125 game milliseconds and 4750 shot milliseconds. The app upgraded it to schema 4 without changing the old event or current clock.
2. The old event displayed **Time not recorded**. Recorded a home made two from the native pad. The score became 5, the live shot clock reset to 24, and the new row retained `P2 05:11 / SHOT 4.8`. SQLite retained exact 310125/4750 readings.
3. Inspected the new timing label and independently scrolling legacy row in dark and light themes at the minimum width. Labels fit without clipping.
4. Advanced to period 3, quit using Command-Q, restarted the entire app and reopened the match. The current clock was period 3 at 10:00/24 paused, the score remained 5, and the old made-two row still showed period 2 at 05:11/4.8.

- [Dark timing label](dark.png)
- [Light timing label](light.png)
- [Unknown legacy timing](legacy.png)
- [Reopened after advancing the period](reopened.png)

## Automated evidence

The six tests in `tests/clock_events.rs` failed on missing event context before implementation and passed afterward. They cover live pre-action milliseconds before shot resets/foul pauses, clock corrections/period changes, paused and zero clocks, disk reopen, undo audit retention, changed-clock UUID rejection, invalid/stale commands, schema 3 preservation with no inferred times, partial-stamp constraints, and atomic rollback with original-time retry. Existing schema 1/2 migration suites also remain enabled.

- `cargo fmt --all -- --check`: passed.
- `cargo test --locked --offline --workspace --all-targets`: 61 passed, none failed (50 application integration tests and 11 packaging tests).
- `cargo clippy --locked --offline --workspace --all-targets -- -D warnings`: passed.
- `cargo build --locked --offline`: passed.
- `git diff --check`: passed.
- `bash .agents/scripts/verify/verify-changed.sh --strict develop`: interrupted with exit 130 after it selected unrelated API/Nuxt branch changes. Its repository map excludes hustlx-oss, so it cannot verify this Rust slice. The Cargo checks above are the owning evidence; the shared command is not reported as passing.
- `python3 .agents/scripts/merge-flagger/flag-diff.py --repo hustlx-oss --base origin/develop --pr-body /tmp/hustlx-oss-clock-stamped-events-pr.md`: exit 0, FLAG for expected stat/history keywords and plan wording, with no P0 blockers. Event migration, clock effects, retry and history evidence are recorded above for PR review; no merge was performed.

Cargo reports a future Rust compatibility notice for the existing `block 0.1.6` dependency. There is no dependency change in this slice. Native Windows/macOS Intel execution, physical power/sleep interruption, signed packages and release qualification were not performed here. This evidence establishes event timing on the tested macOS build, not full game-day release readiness. Schema 4 is forward-only; preserve a closed-library backup before upgrading.
