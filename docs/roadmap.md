# Roadmap

Each slice should produce a reviewable change, matching documentation, and evidence appropriate to its behavior. Everything below is planned unless checked.

## Foundation

- [x] Add automatic SemVer bump/preview, native three-target package CI, and explicit GitHub Release publication automation. Hosted CI/CD was subsequently paused on 2026-10-20 to save cost; use local checks and packages only. See [release guide](releases.md) and [decision](notes.md). Actual publication and platform launch certification are separate release actions.

- [x] Create an independent local repository named hustlx-oss.
- [x] Record the initial scope and proposed architecture.
- [x] Capture dark and light references from the real web v2 components with sample data.
- [x] Select Rust-rendered UI throughout. Use Iced for the first native build.
- [x] Select the open-source license: MIT for project code and documentation; see [LICENSE](../LICENSE) and [asset exceptions](../THIRD_PARTY_NOTICES.md).
- [x] Scaffold the application and establish formatting, tests, and Windows/macOS build jobs. Hosted jobs were subsequently removed under the 2026-10-20 local-only policy; native checks run locally.

## Native preview

- [x] Render the scoreboard, roster panels, stat pad, and play-by-play natively in Rust.
- [x] Exercise player selection, event-derived statistics, undo, and light/dark themes.
- [x] Capture the actual native window and record [local verification](evidence/native-preview/README.md).

## Local library

- [x] Persist teams, players, and roster memberships, including rename/archive/restore.
- [x] Create a named, dated match with an optional venue by choosing Team A and Team B.
- [x] Save match roster snapshots and reopen matches from the local library. See [evidence](evidence/sqlite-library/README.md).

## Basketball Game Day

- [x] Reproduce the v2 scoreboard and three-column console in both themes.
- [x] Persist scoring, player attribution, shots, rebounds, assists, steals, blocks, turnovers, and fouls.
- [x] Implement explicit starting lineups, confirmed substitutions and saved lineup history.
- [x] Save pre-action period/game/shot context with stat events and show it in play-by-play; preserve unknown legacy timing.
- [ ] Implement timeouts and configurable periods.
- [x] Implement durable undo with an append-only correction trail.
- [x] Implement game/shot clocks, explicit time correction, and paused checkpoint recovery.
- [x] Add default player jerseys/positions and supported v2 stat/clock hotkeys.
- [ ] Implement broader stat correction editing.
- [x] Prove committed stats survive close/reopen and failed writes roll back.
- [x] Test running-checkpoint recovery and sleep/time discontinuity policies deterministically.
- [ ] Exercise physical sleep, Force Quit and power interruption on both operating systems.

## Results and release

- [x] View the saved basketball box score locally and export it as a one-page PDF.
- [ ] Export the box score PDF for the other eight sports.
- [ ] Export statistics and portable backups and test restore behavior.
- [ ] Verify the complete journey without network access on Windows and macOS.
- [x] Add version preparation and native local ZIP/checksum commands, without publishing.
- [ ] Verify optimized real-app packages and document supported OS/CPU targets, installers, signing, and upgrade behavior.
- [ ] Publish the first licensed software release with verification evidence.

## Later

- [ ] Design optional paid-league syncing against the actual HustlX server contract.
- [ ] Add entitlement validation, explicit linking, retries, deduplication, and conflict handling without blocking offline games.

## Local multi-sport flows

- [x] Configure and create Volleyball, Pickleball, Table Tennis, Tennis, Badminton and Padel matches with named sides.
- [x] Persist sport-specific scoring, serving, unit results, explicit completion and undo across restart.
- [x] Preserve basketball history through schema 4 and isolate its controls from set/rally sports.
- [x] Capture native setup/scoring/result evidence and complete Cargo verification.
- [x] Port the current native operator layout with optional roster snapshots, participant actions, volleyball rotation/substitutions/libero and local Football/Futsal clocks/events/results.
- [ ] Add full statistical reports, detailed field event correction, staff cards, reduction timers and remaining hosted-console workflows.
