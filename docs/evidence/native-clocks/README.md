# Native clocks and shortcuts evidence

Captured October 16, 2026 on macOS arm64 in the Rust/Iced application at its 1100x760 content minimum. Records are synthetic and use an isolated temporary library. No database or operator data is committed.

## Native journey

- Edited Miguel Santos to default jersey 4, PG/SG, and Rafael Mendoza to jersey 3, C. Created Clock & Hotkey Test with Team A/B, date and Community Gym. Match snapshots retained the details.
- Used comma, 4, W to score a home two; period, 3, E to score an away three. Backspace undid the three, then repeating the away input restored 2-3 with two active plays. Scoring reset the shot clock to 24.
- Space ran and paused both clocks; `]` reset 14 while paused. Corrected times to 01:30 and 2 seconds. Shot expiry paused both at exactly 01:28/0 in SQLite, demonstrating first-expiry clamping.
- Started 01:28/24 and used Command-Q. The saved clock was game 84988 ms / shot 20988 ms, both paused. Reopening showed 01:25/21 paused without a recovery warning. The closed interval was excluded.
- Injected an abort trigger on clock writes in the isolated database. Command-Q left the native window open with SAVE PENDING and Retry save. Removed the trigger and clicked Retry; the command saved once and the requested close completed. Reopened paused at about 01:20/16. A subsequent running-clock window-close-button check stored game 79519 ms / shot 15519 ms, both paused.
- Inspected the shortcut guide and both themes. Corrected a crowded minimum-size layout by moving help into navigation and reducing stat-button vertical spacing. The play feed scrolls independently.
- Refreshed and launched the separate operator Desktop preview against its normal empty library. Synthetic records remain confined to the test application.

## Screenshots

- [Paused match after Command-Q and restart](reopened-match.jpg)
- [Light theme after retried close and restart](reopened-match-light.jpg)
- [Player jersey and positions](player-details.jpg)
- [Explicit time correction](set-times.jpg)
- [Shot expiry notice](shot-expiry.jpg)
- [Shortcut guide](hotkeys.jpg)
- [Failed save blocking Quit](failed-save-blocks-quit.jpg)

The player form capture predates its Rename button label changing to Edit; the underlying detail controls are unchanged.

## Automated evidence

- `cargo fmt --all -- --check`: passed.
- `cargo test --locked --all-targets`: 37 passed, zero failed (7 clock, 8 migration/profile/session, 4 shortcuts, 6 domain, 12 storage).
- `cargo clippy --locked --all-targets -- -D warnings`: passed.
- `cargo build --locked`: passed, macOS arm64.
- `bash .agents/scripts/verify/verify-changed.sh --strict origin/develop`: exit 0, no changed mapped repos. The shared script has no Rust lane; Cargo owns this evidence.

Tests cover monotonic millisecond projection, pause/resume, first expiry, independent shot pause, resets, corrections/bounds, exact duplicate command receipts, stale revisions, transactional stat/clock rollback and retry, forward migration with history intact, optional profiles and immutable snapshots, graceful/recovered paused opening, scheduling gaps, wall-time changes, shortcut guards and ambiguous jerseys. New feature tests failed before their implementation. A first-expiry test exposed an excessive elapsed subtraction and passed after clamping to the earliest countdown expiry.

The web/API clock harness is outside this Rust-only diff: no hosted clock command, lease, payload, permission or subscription changed. CI builds/tests both Windows and macOS but does not launch their GUI. See the PR checks for the final head's CI result.

## Remaining release checks

Windows graphical operation, physical sleep, Force Quit/power loss, hardware disk-full behavior, signed packaging and a complete real-game journey remain unverified. Sleep and running-checkpoint recovery policies have deterministic tests. The small macOS AppKit Quit adapter has native journey evidence rather than an automated GUI harness. The existing upstream `block 0.1.6` future-compatibility warning remains. See [supported behavior](../../clocks-hotkeys.md) for feature limits.
