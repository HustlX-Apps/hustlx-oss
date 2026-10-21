# Sport verification plan

Extend the approved nine-sport port with local regression, end-to-end and deterministic evaluation evidence. OSS is the only edited repository. Its Rust engines, App messages and SQLite receipts are runtime truth. Execution uses Cargo and Python's standard library; verification uses the OSS checker and the new sport harness. Documentation impact is the coverage matrix, commands and evidence. Payments and hosted launch lanes are unaffected.

The audit found that new attributed-action persistence tests focused on Volleyball and Pickleball. Other racquet sports had pure scoring tests and basic setup/render coverage, but lacked equivalent action, exchange, failed-save and process-restart journeys. No separate browser application or parallel test framework is needed for this native app.

- [x] Add racquet action/service/error and durable negative-path tests for Pickleball, Padel, Tennis, Badminton and Table Tennis.
- [x] Add nine named App-message journeys covering setup, saved actors, cancellation/confirmation, scoring, undo, closeout where supported and reopen in a fresh process.
- [x] Add versioned evaluation fixtures with independent expected scores, service, unit/result and rejection assertions. Include all nine sports and reject empty or incomplete eval runs.
- [x] Add a local harness producing JSON/log evidence, failing on failed commands, empty tests, missing sport cases or ignored tests. Test its failure reporting and timeout handling.
- [x] Run targeted suites, the complete local checker and harness, record limits, then update PR #8. GitHub Actions stays disabled.

App-message end-to-end tests cross the actual native controller, session and SQLite boundary, including a separate process on restart. They do not synthesize keyboard input or measure rendered pixels. Native GUI inspection remains a complementary evidence lane.
