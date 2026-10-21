# Native keyboard and UI E2E audit

October 21, 2026. All nine approved sports passed the recorded macOS native journeys after five fixes. This extends [the sport coverage audit](../sport-ui-port/verification.md) with actual native keyboard and pointer input, screenshot inspection, full application Quit/relaunch and saved-result reopen. [Machine-readable results](results.json) records the case IDs, fixture configuration, binary SHA-256 and observed outcomes.

OSS is the only edited repository. Its native App, Session and SQLite command history are the source of truth. Execution used native computer controls and Cargo; verification used screenshots, failing-then-passing controller regressions, the local checker and sport harness. Documentation impact is this audit, the keyboard/sport guides and development commands. Hosted launch and payment paths are unaffected.

## Resolved findings

| Severity | Confidence | Finding | Correction and evidence |
| --- | --- | --- | --- |
| P1 | High | Clicking a Basketball player then pressing a stat key toggled that player off before saving. The app rejected the shot with “Select a player first.” | Keyboard routing retains the existing target selection. Both sides, undo, repeat/modifier/capture guards have a controller regression. Native [before](basketball-key-before.jpg) and [after](basketball-key-after.jpg) show the failure and saved scoring. |
| P2 | High | Tab did not advance between form inputs, so subsequent typing overwrote the previous field. | The native key adapter maps Tab and Shift-Tab to widget focus operations while preserving capture/modifier/repeat guards. [Before](tab-before.jpg), [after](tab-after.jpg). |
| P2 | High | Escape did not cancel the non-Basketball lineup board, outcome chooser or result review. | Escape dismisses these drafts without committing. Captured text-field Escape still belongs to the field. All eight sport controllers have regression coverage. [Original stuck board](escape-before.jpg); the native journeys exercised cancellation before scoring. |
| P2 | High | Enter in Football/Futsal time correction did not submit the typed reading. | Both clock fields now submit through the same validated command as Set time, enabled only for editable matches. Native [before](enter-before.jpg) and [after](futsal-enter.jpg); Football also saved 13:00 through Enter. |
| P2 | High | Tennis/Padel large final-score labels showed the last game's 40-0 rather than the completed set's games. | Result-ready/completed views show saved set games with LAST SET GAMES. A deciding match tiebreak keeps its points with MATCH TIEBREAK POINTS. Live 15/30/40/AD labels stay unchanged. [Before](tennis-final-before.jpg), [Tennis after restart](tennis-reopened.jpg), [Padel after restart](padel-reopened.jpg). |

The Tab, Escape, clicked-player scoring and result-label tests failed before their respective fixes. The Enter failure was reproduced and resolved through real focused text input. The result-label regression also covers an 11-9 match tiebreak so the presentation correction does not replace those points with 0-0 games.

## Recorded sport journeys

| Sport | Native input and UI checks | Saved state after full restart |
| --- | --- | --- |
| Basketball | Home/away jersey keys, clicked-player keys, Backspace undo, held W, Ctrl key guard, help/substitution isolation, Space, shot reset, P, editor Tab/Enter and Quit while running. | [6-0, Q2 paused](basketball-reopened.jpg). [Time editor](basketball-editor.jpg). |
| Volleyball | Six-position court, lineup Escape, outcome Escape, rotation/undo, confirmed libero exchange/undo, attributed kill, result-review Escape and confirmed finish. Basketball scoring keys do not act. | [3-1, one set won](volleyball-reopened.jpg). [Libero exchange](volleyball-libero.jpg). |
| Pickleball | Lineup Escape, receiving rally changes service without a point, undo restores service, attributed winner rallies and confirmed finish. Basketball keys do not act. | [2-0, one game won](pickleball-reopened.jpg). [Side-out](pickleball-sideout.jpg). |
| Tennis | Lineup Escape, live 30/15 labels, attributed point/undo, game/set progression, result confirmation and corrected final display. Basketball keys do not act. | [2-0 set games, one set won](tennis-reopened.jpg). |
| Padel | Lineup Escape, double fault credits opponent, undo restores points, attributed game/set progression, result confirmation and corrected final display. Basketball keys do not act. | [2-0 set games, one set won](padel-reopened.jpg). [Double fault](padel-double-fault.jpg). |
| Badminton | Confirmed substitution and undo, lineup Escape, attributed final point, result confirmation. Basketball keys do not act. | [3-1, one game won](badminton-reopened.jpg). [Substitution](badminton-substitution.jpg). |
| Table Tennis | Lineup/substitution Escape, attributed final point/undo and result confirmation. Basketball keys do not act. | [3-1, one game won](table-tennis-reopened.jpg). |
| Football | Enter correction, focused typing, Escape cancellation, count-up start/pause, goal/undo, own goal and confirmed draw. Basketball keys do not act. | [2-2 draw, paused 13:00](football-reopened.jpg). [Own goal](football-own-goal.jpg). |
| Futsal | Enter correction, focused typing, Escape cancellation, countdown start/pause, goal/undo, next-period rollback and result confirmation. Basketball keys do not act. | [2-1, paused 04:40](futsal-reopened.jpg). [Countdown](futsal-running.jpg), [period rollback](futsal-period-undo.jpg). |

## Fixture and replay procedure

The isolated library is `/tmp/hustlx-oss-native-e2e-20261021`. No operator library was opened or changed. It contains synthetic North/South rosters with 12 players per side. Match IDs and house-rule settings are in `results.json`. Short best-of-one matches and two-point/game targets make completion reproducible; these runs do not certify complete regulation matches. Field period lengths retain the 45/20-minute defaults. No database is committed.

For another native run, create equivalent disposable fixtures using the existing Store setup or native forms. Launch the current debug binary with an absolute `HUSTLX_OSS_DATA_DIR`, then:

1. In Matches, type a draft name, Tab to date, Tab to venue, Shift-Tab back to date. Verify each edit stays in its intended field.
2. For each non-Basketball match, stage a lineup change and press Escape. Verify the saved lineup/revision is unchanged. Press W, Space and Backspace outside fields; they must not invoke Basketball commands.
3. Exercise the sport-specific actions in the matrix, undo a saved action, then continue. Confirm result review only saves after its confirmation control.
4. For field sports, type a corrected MM:SS and press Enter. Exercise typing/backspace while focused; score and period must not change. Check the sport's clock direction and paused rollback.
5. For Basketball, click a player then use W/E, undo with Backspace, hold a stat key and try Ctrl+E. Check guide/editor/substitution guards, supported clock keys and focused field submission.
6. Quit the entire app, including once with a running clock. Relaunch, reopen all nine fixtures, compare saved scores/counters/history and confirm clocks are paused.
7. Inspect both themes and independent roster/history scrolling at the 1100x760 content minimum. Captures here are JPEG, at 1100x792 or 2200x1584 including the title bar, depending on desktop scale.

This run used native keyboard/mouse automation with visual assertions. It is a recorded macOS evidence lane, not a new unattended GUI runner in the local Python harness. Iced exposes only window/menu accessibility nodes here, so coordinate actions must be based on the current screenshot and recalibrated after a relaunch or display-scale change. The functional App E2E lane remains repeatable through Cargo and covers a separate process on restart.

## Automated verification and limits

`PATH=/Users/jaylupos/.cargo/bin:$PATH bash scripts/check.sh` exited 0: 141 Rust tests, zero failed/ignored, 17 Python tests, formatting, Clippy with warnings denied, native debug build and packaging preview passed.

`PATH=/Users/jaylupos/.cargo/bin:$PATH python3 scripts/run_sport_harness.py --report /tmp/hustlx-oss-native-sport-harness/report.json` exited 0: 55 sport regression tests, 21 App/controller tests including all nine E2E restart journeys, and three eval-runner tests covering 27 cases with 161 assertions. The E2E lane now includes the keyboard/controller regressions while continuing to require all nine named journeys. A harness contract test proves that replacing a missing sport journey with an extra controller test still fails.

No further reproducible scoring or persistence defect remained in these journeys. The local merge flagger's scoring/status keyword flags still require owner review. Windows execution, native screen-reader/full keyboard-only operation, physical power interruption, signed distribution and full regulation-match certification remain outside this evidence. Non-Basketball scoring buttons use pointer input; Basketball scoring shortcuts are deliberately isolated to its console. The existing `block` dependency future-compatibility notice remains unchanged. No hosted build, merge or release was dispatched.
