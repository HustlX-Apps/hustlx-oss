# Sport forms verification

Verified October 22, 2026 on macOS 27.0.1 (26A434), Apple Silicon. OSS base: `8bd5c0d`; Nuxt reference: `b3a9a2a8d`. The base already supports all nine sports. This change brings applicable console presentation improvements into the native app and refines match, team and player forms using the approved grouped design.

## Automated checks

- `bash scripts/check.sh`: passed formatting, Clippy with warnings denied, 146 Rust tests, 17 Python tests, native build and packaging dry-run. No tests failed or were ignored. The existing `block` dependency reports a future Rust compatibility warning.
- `python3 scripts/run_sport_harness.py --report /tmp/oss-forms-harness/report.json`: passed 55 regression tests, 26 App/controller tests and 3 evaluation tests covering 27 cases and 161 assertions. The controller suite includes create, operate, undo, close and fresh-process reopen journeys for all nine sports. See [the report](harness.json).
- New regressions cover invalid dates and missing names, corrected submissions, optional roster removal, retained draft names across all nine sport defaults, invalid team/player inputs, sport-specific numeric bounds and disabled deciding-tiebreak fields.

The report records a dirty tree on the base SHA because verification preceded the implementation commit. These results apply to the implementation in this change; subsequent edits were documentation only. A shared Cargo target directory was used for compilation caching. Tests and native review used disposable databases.

## Native review

The native app was inspected in both themes at its minimum window size. Its initial match form clipped the create action; the new footer stays visible while the form scrolls. Labels and related fields are grouped, optional rosters can be removed without erasing names, and errors remain next to the relevant field until corrected. Tab navigation and Enter submission were exercised in the tested text fields.

| Journey | Evidence |
| --- | --- |
| Previous narrow setup | [Before](before.png) |
| Grouped setup and fixed footer | [Light](setup-light.png), [dark](setup-dark.png) |
| Required-field and numeric feedback | [Empty setup](validation-dark.png), [invalid best-of](rules-error-dark.png), [corrected rules](rules-light.png) |
| Sport-specific configuration | [Football](football-light.png), [tennis](tennis-light.png) |
| Optional roster removed, name retained | [Roster removal](roster-removed-light.png) |
| Team and player correction flows | [Team feedback](team-feedback-dark.png), [player feedback](player-feedback-dark.png), [saved player](player-light.png) |
| Pickleball completed and live game chips | [Game Day](game-strip-dark.png) |
| Quit and relaunch with saved score/history | [Reopened match](reopened.png) |

The manually created pickleball doubles match retained its completed first game (11-0), live second game and 11 history actions after quitting and relaunching. Basketball, pickleball, football and tennis setup surfaces were visually inspected; all nine sport journeys are covered by automation, not nine separate manual visual sessions.

## Scope and remaining checks

The local merge flagger returned `FLAG` for status/permission/payment and stats keywords in form styles, sport configuration, score chips and documentation. Inspection found no payment/permission changes, score recalculation or new status transitions: the relevant changes are input status styling, existing validation and score presentation. No blockers were found. Michael Jay Lupos is the requested merge-review owner; acceptance is pending PR review.

Scoring engines, SQLite schema and local command authority are unchanged. Hosted league statistics and network command queues are outside the standalone app's scope. Setup validation reflects existing engine/store requirements; it introduces no rule-provenance or claim-state gates.

Windows native appearance and interaction were not tested. Full keyboard and screen-reader accessibility remain subject to the existing Iced limitations; the exercised Tab/Enter flows are not a complete accessibility audit. This is development evidence, not a signed release or installer smoke test. Hosted builds and releases remain disabled.
