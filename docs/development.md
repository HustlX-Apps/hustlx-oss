# Development

## Current slice

The native Rust preview recreates the web v2 console using Iced 0.14. All interface and domain code is Rust. It opens a synthetic match and lets you select a player, record a shot/stat, undo the last action, and switch themes.

This is an in-memory preview. Changes are not saved when the window closes. The clock is a fixed preview, not a running game/shot clock. Team/player editing, local match creation, SQLite storage, recovery, export, substitutions, timeouts, and completion remain planned. Do not use the preview to keep a real game's official record.

## Prerequisites

Install the stable Rust toolchain using [rustup](https://rustup.rs/). On macOS install Xcode Command Line Tools. On Windows use the MSVC Rust target and the Visual Studio C++ build tools and Windows SDK. See [Rust's installation guide](https://www.rust-lang.org/tools/install).

The first local setup installed Rust without modifying the shell PATH. If `cargo` is not found on macOS, open a configured shell or run `. "$HOME/.cargo/env"`. The toolchain is outside this repository.

## Run

```sh
cargo run --locked
```

No Node.js, browser, server, account, or network connection is needed at runtime. The first build downloads Cargo dependencies; after they are cached, use `cargo run --locked --offline` to build/run without fetching packages.

## Verify

```sh
cargo fmt --all -- --check
cargo test --locked --all-targets
cargo clippy --locked --all-targets -- -D warnings
cargo build --locked
```

The domain can also be tested without compiling the renderer:

```sh
cargo test --locked --no-default-features --lib --test preview
```

The shared umbrella `verify-changed.sh` does not yet contain an owning Rust lane. Its results do not substitute for these Cargo checks. `.github/workflows/rust.yml` applies the full checks to Windows and macOS; a configured workflow is not a passed run.

## Manual preview check

1. Open the native window. Confirm the sample/unsaved notice and three-column v2 layout.
2. Select a home player and record a made three. That player's points and the home score increase by three; the away side does not change.
3. Confirm selection clears. Another stat requires selecting a player again.
4. Undo. Confirm the score and player row return to their previous values.
5. Repeat on the away side with a missed shot, rebound, and foul. Check the feed and undo.
6. Switch themes. Check contrast and layout at the minimum window size, and scroll the roster/feed independently.
7. Close and reopen. Confirm the preview starts fresh. Persistence is not implemented.

## Structure

- `src/lib.rs`: renderer-independent preview model and event-derived statistics.
- `src/main.rs`: native application entry point.
- `src/ui.rs`: Iced layout, controls, and colors.
- `tests/preview.rs`: scoring, player identity, shooting, stat attribution, and undo behavior.
- `docs/references/`: original web screenshots, separate from native preview evidence.

The next slice adds SQLite and local teams/players/matches before implementing real game clocks or hosted integrations.
