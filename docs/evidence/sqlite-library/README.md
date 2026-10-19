# SQLite library evidence

Recorded October 16, 2026 on macOS arm64. All names and match data are synthetic. These screenshots show the Rust/Iced application, not the web reference.

## Native journey

Using an isolated temporary data directory, the native forms created Batangas Blazers and Lipa City Hawks, Miguel Santos and Rafael Mendoza, and memberships with jerseys 4 and 3. The match form created Friday Open Run for 2026-10-16 at Community Gym. Player selection and stat buttons recorded a home made three and an away made two.

The entire test application was quit, then reopened with the same data directory. The match appeared in the local library. Open match restored its name, date, venue, team names, jerseys, 3–2 score, and two plays. Both themes were inspected at the 1100x760 minimum content size. Roster and feed areas scroll; the feed shows one full play at this height.

- [Team and roster management](teams.jpg)
- [Completed match form](create-match.jpg)
- [Library after restart](matches.jpg)
- [Reopened match, dark](reopened-match.jpg)
- [Reopened match, light](reopened-match-light.jpg)

Test records stayed in the isolated directory and are not bundled or inserted into the normal library. No database file is committed.

## Automated evidence

Commands run from `hustlx-oss`:

```sh
cargo fmt --all -- --check
cargo test --locked --all-targets
cargo clippy --locked --all-targets -- -D warnings
cargo build --locked
```

Local result: 18 tests passed (6 domain, 12 storage/session), Clippy and build passed. The initial 10 persistence tests failed against the unimplemented repository, then passed after implementation. Coverage includes disk reopen, catalog rename/archive/restore, membership edits, duplicate names, empty rosters, valid dates, immutable match snapshots, all stat variants, durable undo, exact retries, stale windows, foreign actors, incompatible/corrupt files, transaction rollback, and pending-save display state.

The macOS Metal dependency chain still emits the existing future-compatibility warning for `block 0.1.6`. Clippy reports no project warnings.

The required umbrella command `bash .agents/scripts/verify/verify-changed.sh --strict origin/develop` exited 0 with “no changes detected in mapped repos.” `hustlx-oss` has no mapped Rust lane, so this is not runtime verification. The Cargo commands above provide local evidence. The PR records the deterministic merge-flagger outcome and actual [Windows/macOS CI results](https://github.com/HustlX-Apps/hustlx-oss/pull/1/checks).

## Boundaries

Windows native UI interaction has not been exercised here. CI builds and tests do not prove graphical operation. Abrupt termination, disk-full hardware behavior, clock recovery, export/restore, installers, signing, and sync remain outside this evidence. Failed-save behavior is tested by an injected SQLite failure; uncommitted pending intent does not survive quitting.
