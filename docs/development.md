# Development

## Current slice

Rust and Iced render the interface. SQLite stores reusable teams/players, memberships, named and dated matches, roster snapshots, and durable stat/undo events. Startup opens Matches with an empty library on first use. No sample records are inserted.

Game and shot clocks remain fixed previews. Substitutions, timeouts, completion, full box scores, exports, and installers remain planned. The first five roster members are initially on court, with no lineup editor yet. This is not ready to keep a real game's complete official record.

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
cargo test --locked --all-targets
cargo clippy --locked --all-targets -- -D warnings
cargo build --locked
```

The domain and persistence tests can run without the renderer:

```sh
cargo test --locked --no-default-features --lib --test preview --test storage
```

The shared umbrella verifier has no owning Rust lane. Its summary does not substitute for Cargo checks. `.github/workflows/rust.yml` runs formatting, tests, Clippy, and build on Windows and macOS. CI does not launch the graphical app. See [current evidence](evidence/sqlite-library/README.md).

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
- `src/schema.sql`, `src/storage.rs`: schema version 1, catalog operations, roster snapshots, transactional event commands.
- `src/session.rs`: commit-before-display state and pending retries.
- `src/brand.rs`: embedded wordmarks, Inter/Exo font roles, and light/dark palettes. See [branding](branding.md).
- `src/main.rs`, `src/ui.rs`, `src/library_ui.rs`: native entry point, Game Day console, and library forms.
- `tests/preview.rs`, `tests/storage.rs`: domain and persistence/session tests.
- `docs/references/`: original web screenshots, separate from native evidence.
