# Development

## Current slice

Rust and Iced render the interface. SQLite stores reusable teams/players, memberships, named and dated matches, roster snapshots, and durable stat/undo events. Startup opens Matches with an empty library on first use. No sample records are inserted.

Game and shot clocks use monotonic countdowns, transactional commands and paused restart recovery. Player profiles include optional jerseys and positions; supported v2 shortcuts work on Game Day. Starting lineups, confirmed substitutions and saved lineup history are available. Timeouts, completion, full box scores, exports, and installers remain planned. New matches start with empty courts and require explicit starter selection to put players on court; this is advisory and does not block other game-day actions. This is not ready to keep a real game's complete official record.

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

```sh
cargo fmt --all -- --check
cargo test --locked --workspace --all-targets
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo build --locked
```

The domain and persistence tests can run without the renderer:

```sh
cargo test --locked --no-default-features
```

The shared umbrella verifier has no owning Rust lane. Its summary does not substitute for Cargo checks. `.github/workflows/rust.yml` runs formatting, tests, Clippy, and build on Windows and macOS. CI does not launch the graphical app. See [current evidence](evidence/native-clocks/README.md).

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
- `src/schema.sql`, `src/storage.rs`: schema version 4 with `src/migration_v2.sql` through `src/migration_v4.sql`, catalog operations, roster snapshots, transactional event commands and nullable pre-action timing.
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

See [desktop releases](releases.md). `cargo xtask version --bump patch --dry-run` previews the next version; omit `--dry-run` to update the app manifest/lock. Exact versions and minor/major bumps are also supported. `cargo xtask release --version VERSION --dry-run` previews local packaging; omit `--dry-run` to build a native ZIP/checksum. Cargo commands do not publish. The separate Python command dispatches GitHub native builds and requires explicit `--publish --ref main` for GitHub Releases. Run `python3 -m unittest discover -s scripts -p test_github_release.py -v` and actionlint when changing the release tooling. Relevant PRs build all three optimized packages without publishing; graphical launch and upgrade tests remain manual.
