# Roadmap

Each slice should produce a reviewable change, matching documentation, and evidence appropriate to its behavior. Everything below is planned unless checked.

## Foundation

- [x] Create an independent local repository named hustlx-oss.
- [x] Record the initial scope and proposed architecture.
- [x] Capture dark and light references from the real web v2 components with sample data.
- [x] Select Rust-rendered UI throughout. Use Iced for the first native build.
- [ ] Select the open-source license.
- [x] Scaffold the application and establish formatting, tests, and Windows/macOS build jobs. CI results must be checked separately from configuration.

## Native preview

- [x] Render the scoreboard, roster panels, stat pad, and play-by-play natively in Rust.
- [x] Exercise player selection, event-derived statistics, undo, and light/dark themes.
- [x] Capture the actual native window and record [local verification](evidence/native-preview/README.md).

## Local library

- [ ] Persist teams, players, and roster memberships.
- [ ] Create a named, dated match at a venue by choosing Team A and Team B.
- [ ] Save match roster snapshots and reopen matches from the local library.

## Basketball Game Day

- [ ] Reproduce the v2 scoreboard and three-column console in both themes.
- [ ] Implement scoring, player attribution, shots, rebounds, assists, steals, blocks, turnovers, and fouls.
- [ ] Implement lineups, substitutions, timeouts, and configurable periods.
- [ ] Implement durable undo/corrections and game/shot clock recovery.
- [ ] Prove that closing or interrupting the app does not lose committed game actions.

## Results and release

- [ ] View saved box scores and play-by-play locally.
- [ ] Export statistics and portable backups and test restore behavior.
- [ ] Verify the complete journey without network access on Windows and macOS.
- [ ] Document supported OS/CPU targets, installers, signing, and upgrade behavior.
- [ ] Publish the first licensed software release with verification evidence.

## Later

- [ ] Design optional paid-league syncing against the actual HustlX server contract.
- [ ] Add entitlement validation, explicit linking, retries, deduplication, and conflict handling without blocking offline games.
