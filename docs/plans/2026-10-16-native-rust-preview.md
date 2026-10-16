# Native Rust Preview Implementation Plan

**Goal:** Build the first native, Rust-rendered basketball console matching the captured v2 layout.

**Architecture:** The user selected a Rust-rendered UI throughout on October 16, 2026. Use Iced 0.14 for the desktop interface and a separate, deterministic Rust model for sample match events and statistics. SQLite remains planned for the next persistence slice; the preview must plainly say that changes are not saved.

**Tech stack:** Rust stable, Iced 0.14. No webview, Vue, JavaScript runtime, or server dependency. Iced is an implementation choice within the approved Rust UI direction. Windows and macOS are target platforms; only actual local/CI evidence establishes build support.

## Task 1: Record the Rust UI decision

Update `README.md`, `docs/product-brief.md`, `docs/architecture-proposal.md`, and `docs/roadmap.md`. Record Rust rendering as decided, Iced as the chosen initial toolkit, and SQLite, exports, clocks, and production match management as planned. Keep licensing pending.

## Task 2: Test the preview domain behavior

Create `Cargo.toml`, `src/lib.rs`, and `tests/preview.rs`. Start with failing behavioral assertions for player-specific scoring, made/attempted shots, rebounds, undo, and rejecting unknown player IDs without changing the match. Verify the failures with `cargo test --lib --test preview` before implementing the model. Keep data synthetic and identity based on stable IDs.

## Task 3: Implement the native console

Create `src/main.rs` and `src/ui.rs`. Render the full-width scoreboard, home and away roster panels, centered action pad and play-by-play, purple/yellow team accents, and light/dark themes. Allow player selection, stat recording, and undo against the preview model. Clear selection after a successful stat. Keep the game clock visibly a fixed preview; do not expose inactive save, export, or match-completion controls.

Use a minimum window size that preserves the three-column console, with independent roster/feed scrolling. Add a persistent notice that sample changes are not saved. Do not mix rendering ticks with domain clock logic.

## Task 4: Establish development and verification

Add `rustfmt.toml`, `.github/workflows/rust.yml`, and `docs/development.md`. Configure formatting, tests, Clippy, and builds on Windows and macOS. Document `cargo run`, test commands, the snapshot-only preview scope, and the next SQLite slice.

Run `cargo fmt --all -- --check`, `cargo test --all-targets`, `cargo clippy --all-targets -- -D warnings`, and `cargo build --locked`. Open the native app, exercise selection/stat/undo/theme controls, and capture its actual window. The shared HustlX verification script currently has no Rust lane; run it for its reported scope and retain the Rust commands as the owning verification evidence.

## Task 5: Handoff

Commit and push the feature branch, open a PR targeting `develop`, attach it to this chat, and include actual verification results and preview limitations. Do not merge as part of this slice. The existing documentation-only foundation is the initial `develop` baseline.

## Open checklist

- [x] Rust decision and architecture docs updated.
- [x] Behavioral tests fail before domain implementation and pass after it.
- [x] Native preview compiled and exercised locally.
- [x] Format, tests, Clippy, build, and shared verification results recorded.
- [x] Native screenshots and docs prepared for PR handoff.

## Next slice

SQLite migrations, team/player editing, named and dated match creation with venue, durable match event storage, reopen/recovery, and CSV/JSON export. Add an actual clock only with a separately tested monotonic/recovery design. Paid-league syncing remains deferred.
