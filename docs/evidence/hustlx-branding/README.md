# Native HustlX branding evidence

Recorded October 16, 2026 on macOS arm64. Actual Rust/Iced application captures at the 1100x760 minimum content size (2200x1584 captures include the native title bar at 2x). Data is synthetic and stays in the isolated test library used by the SQLite smoke check.

Official wordmarks, Inter body/control text, Exo headings, Exo Black scores/roster names, navy/gold accents, and graphite dark surfaces were inspected across all four views in both themes. Filled buttons use white labels with sufficient contrast. Existing metadata, jerseys, scores, and history remain visible after opening the same stored match. No new sample data was inserted into the normal operator library.

| Surface | Dark | Light |
| --- | --- | --- |
| Matches | [Capture](matches-dark.jpg) | [Capture](matches-light.jpg) |
| Teams/roster | [Capture](teams-dark.jpg) | [Capture](teams-light.jpg) |
| Players | [Capture](players-dark.jpg) | [Capture](players-light.jpg) |
| Game Day | [Capture](game-day-dark.jpg) | [Capture](game-day-light.jpg) |

Local commands passed:

```sh
cargo fmt --all -- --check
cargo test --locked --all-targets
cargo clippy --locked --all-targets -- -D warnings
cargo build --locked
```

Tests: 18 passed, 0 failed. This presentation-only slice adds no implementation-mirroring UI assertions; actual native renders supply visual evidence. The existing upstream macOS `block 0.1.6` future-compatibility warning remains. CI and deterministic merge-flagger outcomes are recorded in [PR #1](https://github.com/HustlX-Apps/hustlx-oss/pull/1). Windows graphical interaction remains untested; CI verifies compilation and automated tests.
