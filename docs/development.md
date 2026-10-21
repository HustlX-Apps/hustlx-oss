# Development

## Current slice

Rust and Iced render the interface. SQLite stores reusable teams/players, memberships, named and dated matches, roster snapshots, and durable stat/undo events. Startup opens Matches with an empty library on first use. No sample records are inserted.

Game and shot clocks use monotonic countdowns, transactional commands and paused restart recovery. Player profiles include optional jerseys and positions; supported v2 shortcuts work on Game Day. Starting lineups, confirmed substitutions and saved lineup history are available. Basketball timeouts and completion, full box scores, exports, and installers remain planned. The six set/rally sports have local scoring, participant actions, lineup/exchange, completion and reopened-result workflows. Football and Futsal add native field events, clocks, shootouts and draw-aware closeout. Volleyball adds six-position rotation and libero exchanges. New matches start with empty courts and require explicit starter selection to put players on court; this is advisory and does not block other game-day actions. This is not ready to keep a real game's complete official record.

Stats now retain pre-action period/game/shot context across retry and reopen. The native feed shows saved timing; legacy rows explicitly show Time not recorded. See [event timing evidence](evidence/clock-stamped-events/README.md).

## Prerequisites

Install stable Rust using [rustup](https://rustup.rs/). On macOS install Xcode Command Line Tools. On Windows use the MSVC Rust target and Visual Studio C++ build tools and Windows SDK. SQLite is built from bundled source with the application.

If `cargo` is not found on macOS, open a configured shell or run `. "$HOME/.cargo/env"`.

## Run

```sh
cargo run --locked
```

No Node.js, browser, server, account, or network connection is needed at runtime. The first build downloads Cargo dependencies; after they are cached, use `cargo run --locked --offline`.

For an isolated test library on macOS, set an absolute directory before launch:

```sh
HUSTLX_OSS_DATA_DIR="/tmp/hustlx-oss-development" cargo run --locked
```

PowerShell:

```powershell
$env:HUSTLX_OSS_DATA_DIR = Join-Path $env:TEMP 'hustlx-oss-development'
cargo run --locked
```

Do not use a real operator library for destructive tests. [Storage guidance](storage.md) describes paths and preservation requirements.

## Verify

CI/CD is paused to save cost. GitHub Actions is disabled and hosted workflows are removed. Use the local checker from the repository root:

```sh
bash scripts/check.sh
```

Windows PowerShell:

```powershell
./scripts/check.ps1
```

Install stable Rust with rustfmt and Clippy, plus Python 3.9+ (`python3` on macOS, `python` on Windows). The checker stops at the first failure and runs formatting, workspace/all-target tests, Clippy with warnings denied, Python release safety tests, a native debug build and a packaging dry-run. It never starts hosted jobs or publishes. Include the host OS and actual results in the PR.

The domain and persistence tests can also run without the renderer:

```sh
cargo test --locked --no-default-features
```

The shared umbrella verifier has no owning Rust lane; its summary does not substitute for this local checker. Manual graphical and platform release evidence remains required. Do not restore CI/CD without explicit user direction. See [the decision](notes.md) and [current native evidence](evidence/native-clocks/README.md).

## Native smoke check

1. Start with an isolated empty library. Create two teams and players, assign memberships and jerseys, and edit a name/jersey.
2. Archive and restore an identity. Confirm archives hide it from new setup while saved matches remain usable.
3. Create a match with two different teams, a name, a real YYYY-MM-DD date, and an optional venue. Empty rosters are allowed.
4. Select a home player and record a made three; select an away player and record a made two. Check individual scores and play-by-play.
5. Close the entire app and reopen it using the same data directory. Open the saved match and confirm its metadata, rosters, score, and history.
6. Undo the latest action and reopen again. Confirm the correction persists.
7. Rename a catalog player/team and change membership. Confirm the saved match keeps its original snapshot.
8. Inspect both themes at the 1100x760 minimum size; scroll rosters and feed independently.

Integration tests use temporary databases for unknown identities, invalid dates, stale revisions, duplicate commands, unsupported/corrupt files, and write rollback. The session failure test injects a failing SQLite trigger, checks that visible score/selection are preserved, removes the trigger, and retries the same pending command.

## Structure

- `src/lib.rs`: renderer-independent match/stat projections. Synthetic defaults support tests only.
- `src/schema.sql`, `src/storage.rs`: schema version 6 with `src/migration_v2.sql` through `src/migration_v6.sql`, catalog operations, roster snapshots, transactional event commands and nullable pre-action timing.
- `src/session.rs`: commit-before-display state and pending retries.
- `src/brand.rs`: embedded wordmarks, Inter/Exo font roles, and light/dark palettes. See [branding](branding.md).
- `src/main.rs`, `src/ui.rs`, `src/library_ui.rs`, `src/clock_ui.rs`, `src/lineup_ui.rs`: native entry point, Game Day console, and library forms.
- `src/lineup.rs`, `src/lineup_storage.rs`: side-scoped lineup drafts, current court and starter/substitution receipts.
- `src/clock.rs`, `src/profile.rs`, `src/hotkeys.rs`: renderer-independent clocks, player details, and shortcut routing.
- `src/native_quit.rs`: macOS AppKit Quit delegates to the durable close path; Windows uses Iced close requests. Recheck the native Quit journey after Iced/winit upgrades.
- `tests/preview.rs`, `tests/storage.rs`, `tests/clocks.rs`, `tests/desktop.rs`, `tests/hotkeys.rs`, `tests/lineups.rs`: domain, persistence, recovery and shortcut tests.
- `tests/clock_events.rs`: pre-action timing, exact milliseconds, clock effects, legacy migration, corrections/period changes, zero clocks, undo, idempotency and failed-save retry.
- `docs/references/`: original web screenshots, separate from native evidence.

## Clock smoke check

Use [clocks and shortcuts](clocks-hotkeys.md). Check Space, 24/14 resets, independent shot pause, time corrections, both expiry paths, period advance, jersey/side selection and undo. Close while running with both the window close button and macOS Command-Q, reopen paused, and confirm no closed interval was deducted. Inject a failing clock write only into an isolated test database; confirm Quit stays open and Retry saves once before closing. Sleep/discontinuity and abrupt recovery have deterministic session tests; physical sleep/power-loss tests remain a release gate.

For event timing, record a made field goal with a shot reading other than 24. Its feed row must keep the pre-reset reading while the current shot clock resets to 24. Advance the period, quit and restart; the old row must retain its original period/time. Upgrade a disposable schema 3 library and check that its old events show Time not recorded. Inspect both themes at the minimum window size; do not test upgrades on an operator's only library.

## Lineup smoke check

Create an isolated match with seven players per side. Save five starters per side, record attributed stats, stage two swaps, cancel and verify the court is unchanged. Confirm the swaps and restart the complete app; court order, starter markers, bench membership, scores and lineup history must persist. Confirm X chooses the hinted side and stat shortcuts do not leak into the editor/history. Inject an abort trigger on lineup receipts in the isolated database only; Confirm must leave a retryable pending command, preserve the committed court/clock, and block Quit. Remove the trigger, retry, and confirm exactly one receipt. Inspect both themes at the minimum window size. See [native evidence](evidence/native-lineups/README.md).

## Versioned packaging commands

See [desktop releases](releases.md). `cargo xtask version --bump patch --dry-run` previews the next version; omit `--dry-run` to update the app manifest/lock. Exact versions and minor/major bumps are also supported. `cargo xtask release --version VERSION --dry-run` previews local packaging; omit `--dry-run` to build a native ZIP/checksum. Cargo commands do not publish. Hosted build dispatch and automated publication are paused. Run the local checker after release-tooling changes; graphical launch and upgrade tests remain manual. Historical GitHub automation helpers do not authorize remote runs.

## Multi-sport verification

`tests/sports.rs` covers sport scoring and service boundaries; `tests/sport_storage.rs` covers durable journeys, replay corrections, revision/UUID integrity, migration and failed-write rollback. Native UI routing tests cover creation and basketball control isolation. Use the complete Cargo checks above before handoff. Exercise setup, scoring, undo, completion and restart in an isolated library and inspect both themes at 1100x760. The hosted API/Nuxt harness is not a substitute for these Rust tests and is not required when those repositories remain unchanged.

## Sport UI port checks

`tests/operators.rs` covers attributed outcomes, rotation, open exchanges and per-set reset; `tests/operator_storage.rs` covers roster snapshots and schema 5 preservation. `tests/field_sports.rs` covers field clock/event semantics, while `tests/field_session.rs` covers monotonic checkpoints, exact retry, interruption, completion undo and restart. Native App tests exercise all nine sport setup/render paths, confirmed volleyball exchanges and field closeout. Run the local checker after changes.

Native smoke: use an isolated library, create roster-backed and named-side matches, verify volleyball Kill/Attack/Error, side-out rotation, lineup cancellation, confirmed substitution/libero, Undo and restart. For racquet sports inspect singles/doubles service, point labels and game/set strip; for Football/Futsal inspect clock direction, own goals, team/player attribution, paused next period, shootout and confirmed draw/decisive closeout. Keep the six volleyball positions visible at 1100x760, and inspect both themes. See [port evidence](evidence/sport-ui-port/README.md).
