# Rotation and service follow-up

Approved October 23, 2026: repair service behavior, show server/receiver markers, test rotation, substitutions, Undo and restart, then open a develop PR.

## Design

Keep the existing Rust scoring engine, Iced console and append-only SQLite command log. Volleyball P1 identifies the server on the serving side. Racquet service slots identify participants only through their explicit saved lineup order. Do not infer a player from a roster, name or jersey. Volleyball has no single designated receiver. Completed matches show no live service markers. Missing lineups retain named-side scoring and numbered service labels.

Correct singles Pickleball's receiving slot after a side-out, doubles Pickleball/Badminton server corrections after court changes, the Badminton deciding-game end-change cue for a custom target, and volleyball opening-side alternation after a mid-set correction. Keep score rules, open substitutions and roster identity boundaries intact. Existing commands replay with the corrected service/cue projection; no stored event, statistic or score is rewritten.

Alternatives were behavior-only fixes and a new service-order editor. The approved choice adds visible identity markers to the existing lineup workflow without another editor or persistence format.

## Implementation plan

Goal: make service identities visible and keep service/court state consistent through corrections and recovery.

Architecture: fix the existing pure State transitions; derive presentation from committed State and saved roster snapshots. Retain Session/Store UUID retry, revision fencing and replay Undo.

Tech stack: Rust, Iced, SQLite, Cargo tests and the local Python sport harness.

- [x] Add failing service regressions in `tests/sports.rs`; run `cargo +stable test --locked --no-default-features --test sports` and capture the failures.
- [x] Repair `src/sport.rs`; rerun the focused cases and existing sports suite.
- [x] Add server/receiver marker regressions in `src/operator_ui.rs`; implement labels and the service summary there.
- [x] Extend `tests/operators.rs` and `tests/operator_storage.rs` for a full rotation cycle, live-slot substitution/libero exchanges, exact Undo, UUID replay and disk reopen.
- [x] Inspect the native app in both themes at 1100x760, including rotation, exchange, Undo and full-process restart; save evidence.
- [x] Update `docs/multi-sport.md` and evidence; run `bash scripts/check.sh`, `python3 scripts/run_sport_harness.py`, and the local merge flagger. Commit/push this non-protected branch and open a PR to `develop`.

Repo set: edit OSS only; API/Nuxt are read-only references. Base OSS `0399137`, API `2ac921e13`, Nuxt `a2be0a96b`. Execution and verification are local. Payments and hosted launch status are unaffected. Windows execution remains a separate evidence gap.
