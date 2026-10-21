# Sport coverage audit and local harness

October 21, 2026. Scope: all nine approved OSS sports. This follow-up adds tests and local tooling to PR #8; it changes no production scoring rules or storage schema. API, Nuxt, payments and hosted infrastructure remain unchanged.

## Audit outcome

The coverage gap was concentrated in the new participant controls. Volleyball had attributed scoring, exchanges and persistence tests; Pickleball had a side-out attribution case. Tennis, Padel, Badminton and Table Tennis had pure scoring/service tests and basic App setup coverage, but lacked equivalent actor, failed-save, exchange and restart journeys. Those gaps now have automated coverage. The added cases found no further runtime defect.

| Sport | New regression/eval focus | App E2E |
| --- | --- | --- |
| Basketball | Attributed made shots, wrong-side rejection, saved score and undo | Catalog forms, match creation, scoring, restart, undo |
| Volleyball | Attributed points, invalid actions, set closeout and undo | Lineup cancel/confirm, exchange/undo, scoring, result confirmation, restart |
| Pickleball | Side-out without a point, two-server state, attribution, durable negative paths | Same participant/result/restart journey |
| Tennis | Double faults through deuce, game boundary, attributed errors and durable negative paths | Same participant/result/restart journey |
| Padel | Double faults through deuce, game boundary, attributed errors and durable negative paths | Same participant/result/restart journey |
| Badminton | Attributed cap point at 30-29, unit rollback, errors and durable negative paths | Same participant/result/restart journey |
| Table Tennis | Deuce service, unit rollback, attributed errors and durable negative paths | Same participant/result/restart journey |
| Football | Own goal, period clock rollback, separate shootout, closeout | Participant setup/exchange, goal/undo, confirmed result, restart |
| Futsal | Own goal, period rollback, separate shootout, extra-time foul window | Participant setup/exchange, goal/undo, confirmed result, restart |

For all five racquet sports, durable negative paths cover wrong-side actors with identical names/jerseys, stale revisions, changed-command UUID reuse, exact retry, immutable roster snapshots, confirmed exchange/undo and injected write failure. A failed save blocks another action and close, then retries exactly once; undo remains saved after reopen.

## Repeatable commands

```sh
python3 scripts/run_sport_harness.py
python3 scripts/run_sport_harness.py --lane regression
python3 scripts/run_sport_harness.py --lane e2e
python3 scripts/run_sport_harness.py --lane eval
bash scripts/check.sh
```

Use `python` and `./scripts/check.ps1` on Windows. Rust stable must be on PATH. The harness writes `artifacts/sport-harness/report.json` and per-lane logs; these generated files are ignored. `--report PATH` chooses another output location. Reports include host, source revision/dirty status, fixture SHA-256, exact commands, test counts and evaluated case IDs. Each lane has a configurable 180-second timeout. A failed lane stops the run, produces a failed report and returns nonzero.

The harness rejects missing/empty Rust summaries, ignored or failed tests, nonzero command exits, omitted App sport journeys and missing/duplicate/unasserted eval cases. It replaces a previous green report with a running report before starting so an interrupted attempt cannot inherit success.

The E2E lane drives the actual App form and game-day messages through Session and SQLite. Each of nine named tests closes its App, starts the test executable in a separate process, reconstructs the saved match and verifies another durable undo/reopen. It constructs both native theme views at key stages. This is functional App E2E, not automated native keyboard input or pixel-layout validation. Basketball completion remains outside runtime support and is not invented for a test.

The eval lane reads `tests/fixtures/sport-evals.json`: 27 versioned scenarios with independent expected outcomes, rejection assertions and immutable-state checks on rejected writes. All nine sports and unique case IDs are required. Unknown fixture fields, missing assertions and invalid command definitions fail rather than becoming expected rejections. `--eval-fixture PATH` supports a disposable candidate fixture while retaining the all-sport completeness requirement. These are deterministic sports evaluations, not model-quality benchmarks.

## Recorded results

On macOS arm64:

- `PATH=/Users/jaylupos/.cargo/bin:$PATH bash scripts/check.sh`: exit 0; 137 Rust tests, zero failures/ignored, 16 Python tests, formatting, Clippy with warnings denied, native build and packaging preview passed.
- `PATH=/Users/jaylupos/.cargo/bin:$PATH python3 scripts/run_sport_harness.py --report /tmp/hustlx-oss-sport-harness/report.json`: exit 0; 55 regression tests, nine App E2E tests, three eval-runner tests, 27 evaluated cases and 161 assertions.
- Disposable negative fixture changed the first Basketball expected score to 999. The eval command exited 101, the harness exited 1 and its JSON report said failed. The committed fixture was unchanged.
- Seven harness contract tests prove fail-closed reporting, including timeout output retention and replacement of a stale green report.

Existing native captures and manual journeys remain in [the port evidence](README.md) and [the previous audit](audit.md). The new work is test-only and changes no visible production layout. Windows execution, physical power interruption, native input automation and a complete official-record certification for every sport remain outside this evidence. The existing `block` dependency future-compatibility notice is unchanged. No hosted check, merge or release was dispatched.
