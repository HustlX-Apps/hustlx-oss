# Native lineup verification

Captured October 17, 2026 on macOS arm64 using the Rust/Iced app at its 1100x760 content minimum (2200x1584 Retina capture including title bar). Synthetic test records use the existing isolated test library. No database is committed.

## Native journey

1. Upgraded the existing schema 2 test library without resetting its prior matches. Added synthetic reusable memberships so each team had seven players; created Saturday Lineup Test at Community Gym, 2026-10-17 through the native form. Both courts started empty.
2. Chose home starters #4, #6, #7, #8, #9 and away starters #3, #6, #7, #8, #9. Saved each separately. Both consoles showed five on court and starters saved. Scoring key W inside the starter editor did not record a stat.
3. Used existing jersey hotkeys to record a home three for #4 and an away two for #3. Comma then X opened home substitutions. Staged #4 out / #10 in, pressed Escape, and confirmed #4 remained on court with the 3-2 score unchanged.
4. Reopened home substitutions, staged #4 to #10 in slot 1 and #6 to #11 in slot 2, and confirmed. Quit the complete app with Command-Q, relaunched it, and reopened the match. Both new court slots, five-player count, original starter markers, score and plays persisted. Lineup history retained both starting selections and one substitution with correct incoming/outgoing names.
5. Inspected the away board in light mode. Added an abort trigger on lineup receipt INSERT in the isolated database. Staged away #3 to #10 and confirmed. SAVE PENDING appeared; Command-Q kept the app open with Retry. SQLite inspection confirmed three receipts and the unchanged committed away court #3/#6/#7/#8/#9. Removed the trigger and retried. The blocked close completed; SQLite contained exactly four receipts, with away #10 in slot 1 and no duplicate substitution.

## Screenshots

- [Five-player starting lineup draft](starting-lineup.jpg)
- [Two staged slot-preserving substitutions](substitution-board.jpg)
- [Saved match reopened after full restart](reopened-match.jpg)
- [Starting selections and substitution history after restart](history.jpg)
- [Away board in light theme](substitution-light.jpg)
- [Failed save blocking normal Quit](pending-save.jpg)

## Automated evidence

- `cargo test --locked --no-default-features --test lineups`: six passed after the initial missing-feature failure.
- `cargo test --locked --all-targets`: 44 passed, zero failed (7 clock, 8 desktop, 5 hotkey, 6 lineup, 6 domain, 12 storage).
- `cargo clippy --locked --all-targets -- -D warnings`: passed after using an inclusive range for supported schema versions.
- `cargo build --locked`: passed on macOS arm64.
- Formatting and whitespace checked before handoff; CI runs formatting, all tests, Clippy and build on Windows/macOS.
- Shared strict verifier: exit 0, no changed mapped repos. There is no shared Rust lane; Cargo owns the runtime evidence.

Lineup tests exercise disk reopen, original starters versus current court, stat preservation, slot order, cancel isolation, migration preservation without inferred starters, short/empty/over-five choices, same player on two sides, unknown/wrong-side/duplicate identity rejection, UUID payload reuse/cross-kind collision rejection, stale revisions, and atomic rollback/retry/close with clock state. The hotkey test verifies X and input guards. GUI evidence covers editor confirmation/cancel, restart, both themes and pending-save close/retry.

The hosted API/Nuxt clock, claim and multi-sport harnesses do not apply to this Rust-only implementation; those repositories and their contracts were unchanged. Payment and subscription behavior is unaffected.

Windows GUI operation, physical sleep/power interruption, disk-full hardware, packaging/signing and full official-game operation remain release checks. CI is cross-platform build/test evidence only; the PR records the final head's result. Minutes played, plus-minus, automatic foul-out substitutions, timeout controls and syncing are outside this slice.
