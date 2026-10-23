# Rotation and service evidence

October 23, 2026. OSS base `0399137`; only the OSS repository changes. API `2ac921e13` and Nuxt `a2be0a96b` were read-only references. This is local macOS arm64 evidence, not a release or Windows certification.

## Corrections

Five scoring-engine regression cases failed before their respective fixes and passed afterward:

- Singles Pickleball returned receiving slot 2 after a side-out at an odd score. Both participant slots now remain 1.
- Doubles Pickleball's Serve home/away correction reset the participant to slot 1 even when slot 2 occupied the right court. The correction now selects from the saved courts and the next winning rally keeps that server.
- Doubles Badminton's serving correction discarded the parity-selected server. It now preserves court assignments and resolves both participants from the serving score.
- A custom 15-point Badminton decider used the regular 21-point target for its end-change cue. It now cues once at the deciding target's midpoint.
- A volleyball mid-set service correction changed the following set's opening side. Mid-set corrections now retain the original alternating sequence; a 0-0 correction can still choose the opening side.

SERVING/RECEIVING markers resolve saved player identities through explicit live slots. Volleyball marks only its serving P1 player. Named sides and unassigned slots retain side/slot summaries. Decided/completed matches have no live markers. Pickleball turn number remains separate from participant slot. Storage schema and command formats are unchanged; historical rows remain intact while replay uses the corrected service/cue projection.

## Automated checks

- `cargo +stable test --locked --no-default-features --test sports --test operators --test operator_storage`: focused scoring, full six-side-out court cycle, open exchanges and persistence coverage.
- `cargo +stable test --locked --bin hustlx-oss markers`: both marker regressions pass, including all five racquet sports, volleyball rotation/exchange, missing lineups, terminal status and field-sport exclusion.
- `PATH=/Users/jaylupos/.cargo/bin:$PATH bash scripts/check.sh`: exit 0; 155 Rust tests, zero failures/ignored, 17 Python tests, rustfmt, Clippy with warnings denied, native build and packaging dry-run passed. The final Archive path belongs to this OSS checkout.
- `PATH=/Users/jaylupos/.cargo/bin:$PATH python3 scripts/run_sport_harness.py --report /tmp/hustlx-oss-rotation-service-harness/report.json`: exit 0; 62 regressions, 28 App/controller tests including all nine fresh-process E2E journeys, three eval tests, 27 evaluated scenarios and 161 assertions.

The new `rotation_service_exchange_undo_survive_process_restart` integration test writes all six affected sport matches, starts a separate process, reopens each match and undoes every command to its exact prior snapshot. Volleyball exchanges use a rotated live slot while explicit starters retain their original frame. The journey includes ordinary substitutions, libero exchange, manual rotation, service correction and exact duplicate UUID acceptance. Existing negative tests continue to cover wrong-side actors, stale revisions, terminal writes and injected failed-save retry.

The initial packaging check reused an xtask artifact that embedded another worktree's path. `cargo +stable clean --package xtask` followed by the local checker rebuilt it in this checkout. The stale-path run is excluded from the final packaging evidence. See the [development caveat](../../development.md#rotation-and-service-checks).

The local merge flagger exited 0 with no P0 blockers and expected P1 score/status keyword flags. Verification and negative-path evidence are recorded here and in the PR. Maintainer acceptance of those flags remains part of PR review; no merge is performed.

## Native checks

The app ran at its 1100x760 content minimum with a separate synthetic North/South library. No operator library was opened. Both themes were inspected. The compact marker sits on the existing statistics row so it does not force the player name to wrap. Volleyball retains its existing independent panel scrolling.

| Journey | Observed result |
| --- | --- |
| Volleyball receiving side wins, then the other side wins | Both courts move clockwise once; North 2 becomes the serving P1 player. [Dark](volleyball-rotated-dark.png). |
| Replace rotated serving North 2 with North 7, then Undo | Incoming player inherits P1 and SERVING; Undo restores North 2 and keeps the original event marked undone. |
| Libero exchange North 2 to North 8 | Incoming player inherits P1, L and SERVING with no house-rule block. Undo restores North 2. [Light](volleyball-libero-light.png). |
| Quit/relaunch and reopen volleyball | 1-1, rotated court, North 2 serving and append-only exchange corrections survive. [Reopened light](volleyball-reopened-light.png). |
| Singles Pickleball, score 1-1 through side-outs | Only slot 1 serves/receives, including after full restart. [Dark](pickleball-singles-dark.png). |
| Doubles Pickleball, home point then Serve home | North 2 serves from its saved right court; South 1 receives. [Correction](pickleball-doubles-correction-dark.png). |
| Doubles Badminton, home/away rallies then Serve away | South 2 keeps service and North 1 receives. [Light](badminton-correction-light.png), [dark](badminton-correction-dark.png). |
| Doubles Table Tennis, two home points | South 1 serves; North 2 receives. [Light](table-tennis-doubles-light.png). |
| Doubles Tennis/Padel, four home points | Game score becomes 1-0; South 1 serves and North 2 receives. [Tennis](tennis-game-service-dark.png), [Padel](padel-game-service-dark.png). |
| Quit/relaunch Tennis, then Undo | Saved 1-0 game and service identities recover; Undo returns 40-0, North 1 serving and South 1 receiving. [Reopened](tennis-reopened-dark.png), [Undo](tennis-undo-after-restart-dark.png). |

Screenshots show actual native rendering and input. Cargo App view tests separately cover both themes for all nine sports. Physical power-loss behavior, Windows execution and federation certification remain outside this evidence. The existing `block` dependency future-compatibility warning remains. No hosted workflow, release build or publication was dispatched.
