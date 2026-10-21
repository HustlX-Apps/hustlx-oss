# Sport port audit

October 21, 2026. Audited PR #8 on `codex/oss-sport-ui-port` against `origin/develop` (`89b3d49`). Runtime edits remain confined to OSS. Nuxt and API are read-only references. Local Cargo checks and native app inspection provide verification; this report, storage guide and sport guide record the documentation impact.

## Findings resolved

| Severity | Confidence | Finding and correction |
| --- | --- | --- |
| P1 | High | Undo of a field period advance replayed subsequent clock-only receipts into the preceding period. This restored the wrong clock; a Futsal extra-time reading exceeding a configured short regulation period made Undo fail and left a pending save. Replay now follows the period at receipt time, including intervening undo receipts. Clock observations from retracted periods remain in history without overwriting the restored period. |
| P2 | High | Futsal accumulated fouls incorrectly reset on entry to each extra-time period, hiding the sixth-foul advisory. The API's `LeagueMatch::futsalAccumulatedFouls` carries period 2 through periods 3 and 4. OSS now matches that behavior, resets at halftime, clears for shootout and restores counts through undo. Native help text was corrected, and the Futsal scoreboard uses PERIOD rather than HALF. |
| P3 | High | The storage guide's schema heading said 6 while its version and migration paragraph still said 5. The paragraph now describes schema 6 and its migration/rollback boundary. |

## Scope inspected

Reviewed the nine-sport catalog and setup, preserved rally scoring/service paths, player action attribution, volleyball rotation/exchanges and set reset, actor identity validation, UUID/revision handling, field clock recovery and completion, append-only undo, and schema 6 preservation. Existing all-sport App-message tests and the original native captures cover setup and rendering. No additional blocker was found in those paths. This audit does not certify every complete match or Windows distribution.

## Regression evidence

The two new clock regression tests failed before the replay correction: one restored 26 seconds instead of 12 seconds, and the other rejected undo with “Clock time exceeds the supported period range.” The foul carryover test failed before the engine correction, showing zero instead of five home fouls on entering extra time.

Four added tests cover both field sports' period rollback, reopen, continued play after undo, short regulation versus longer extra time, Futsal carryover through both extra-time periods, shootout clearing and undo restoring the prior foul window. Existing completion undo, timed retry and legacy library preservation tests remain active.

Native inspection used the existing isolated synthetic library. In Futsal, advanced from period 1 at 12:33, corrected period 2 to 05:00, then used Undo. Period 1 returned paused at 12:33. Continued into period 2, recorded five home fouls, entered extra time with five, recorded the sixth, then entered the second extra-time period. Six fouls and the advisory remained after reopening. Captures are 1100x792 including the title bar, with the same 1100x760 content minimum.

- [Period undo restores 12:33](futsal-period-undo.png)
- [Second extra-time period retains six fouls](futsal-extra-time-fouls.png)

The full local checker passed on macOS arm64: 120 Rust tests, zero failures/ignored, 9 Python release-helper tests, formatting, Clippy with warnings denied, native debug build and packaging dry-run. GitHub Actions remains disabled. The existing `block` dependency's future-compatibility notice remains the same release caveat as in the original evidence.

The follow-up local merge flagger exited 0 with Decision FLAG for scoring/statistics/status keywords and no P0 blockers. The regression and native evidence above addresses those categories; merge still requires owner review.

Windows GUI/signing, physical power interruption, reports/exports and the hosted workflows already excluded from this port remain separate release work. No merge or release is performed by this audit.
