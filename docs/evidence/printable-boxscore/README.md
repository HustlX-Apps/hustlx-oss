# Printable box score evidence

Recorded October 24, 2026 on macOS Apple Silicon from branch `codex/oss-printable-boxscore-basketball` in the hustlx-oss worktree, implementation commits `fce7b37` through `92ca610`. Synthetic fixture data only; no browser or hosted service was used. The PDFs were produced by the app's own renderer and converted to JPEG with `sips` for this page.

## Rendered pages

Generated with `cargo +stable xtask boxscore-fixtures`, which writes three PDFs to `target/boxscore/`. Two consecutive runs produced identical SHA-256 sums for all three files (cross-process determinism).

- [Full fixture](basketball-full.jpg): twelve players a side, four stamped periods, chained substitutions, one technical and one unsportsmanlike foul. Header, saved clock reading `P4 00:00`, scoring by period, both tables with `MIN` and `+/-`, totals, fouls notes, team fouls by period, game flow, legend and the "powered by HustlX" footer, all on one page at the first ladder step.
- [Oversized fixture](basketball-oversized.jpg): twenty players a side, a 60-character home name and 40-character player names. Names shorten with an ellipsis inside their columns, no header label or totals cell is cut, and the sheet still fits at the first ladder step.

The unstamped fixture (not pictured) prints neither the line score, the game flow nor the `MIN` and `+/-` columns, and its legend omits both.

## Automated evidence

`bash scripts/check.sh` passed on the branch head:

- `cargo +stable fmt --all -- --check`: passed.
- `cargo +stable test --locked --workspace --all-targets`: 206 passed, 0 failed across 20 test binaries, including the 41 `boxscore` library tests, the 8 Box score screen tests, the storage-driven `tests/boxscore.rs` render test and the xtask fixture command test.
- `cargo +stable clippy --locked --workspace --all-targets -- -D warnings`: passed.
- `python3 -m unittest discover -s scripts`: 17 tests, OK.
- `cargo +stable build --locked`: passed.
- `cargo +stable xtask release --dry-run`: validated the local archive path for this checkout.

The library also builds without the desktop feature (`cargo +stable build --locked --no-default-features`), which is what the xtask command compiles against.

## Manual journey

Not performed in this run. The native save dialog, Open, Show in folder and the read-only folder error path are driven by Iced tasks that an automated session cannot exercise, and screen capture was not authorized. The screen tests cover the same message flow headlessly: cancel returns to idle with no file written, a chosen path gets `.pdf` appended and is written atomically, and a failed write shows the inline error with Retry export. The owner should run the journey once from `cargo run --locked` before release: open Matches, press Box score on a basketball match, Export PDF, cancel once, save once, Open, Show in folder, then export into a read-only folder and retry into a writable one.
