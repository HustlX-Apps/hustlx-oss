# Native sport UI port evidence

October 21, 2026. Host: macOS 27 arm64. Scope: Volleyball, Pickleball, Padel, Tennis, Badminton, Table Tennis, Football and Futsal, with the existing Basketball v2 console preserved. OSS base is `89b3d49`; read-only web reference is Nuxt `f2bf2b5aa`, and Football/Futsal behavior was mapped against API `3f9e95d54`.

## Local verification

`PATH=/Users/jaylupos/.cargo/bin:$PATH bash scripts/check.sh` passed with exit 0. It ran formatting, 120 Rust workspace tests (zero failures or ignored tests), Clippy with warnings denied, 9 Python release-helper tests, the native debug build and the version 0.1.0 arm64 packaging dry-run. No hosted build or release was dispatched. The existing `block` 0.1.6 dependency emits a future-Rust compatibility notice; current build and Clippy pass.

New tests cover player-attributed scoring and non-scoring actions, side-out rotation, open substitution/libero exchange, per-set reset, roster snapshot identity, wrong-side actors, stale revisions, exact retries and schema 5 migration. The migration test retains basketball child records, an existing index/trigger and a deleted high ID's autoincrement boundary. Existing schema 1/3/4 preservation tests continue through schema 6.

Field tests cover football count-up/stoppage, futsal countdown/expiry, own goals, separate shootouts, draws, counter overflow, serialized commands, five-second checkpoints, close/reopen, interrupted time, failed-save freeze/exact retry and completion undo preserving the observed clock paused. Native App tests exercise setup/rendering for all nine sports in both themes, staged/cancelled/confirmed volleyball exchanges and confirmed field closeout. The original six rally-sport journeys remain covered separately from the new field suites.

Follow-up audit: [resolved findings and regression evidence](audit.md). Period rollback now fences later-period clock observations; Futsal second-half fouls carry through extra time. Four added regression tests and two native captures cover these corrections.

## Native inspection

Captures use the 1100x760 content minimum, with 2200x1584 Retina PNGs including the title bar. All names and matches are synthetic. The baseline executable used `/tmp/hustlx-oss-sport-ui-before-20261021`; the port used `/tmp/hustlx-oss-sport-ui-after-20261021`. No operator library was opened and no database is committed.

- Inspected all nine running consoles. Volleyball, Padel, Pickleball and Futsal have both-theme captures; the remaining consoles have representative captures plus both-theme App-message tests.
- Volleyball: selected ATK on a named court card, chose Kill, observed score 2-1 become 3-1 and the player's K count increase. Quit/relaunched the entire app and reopened the match; both remained saved. Undo restored score 2-1 and the count, retaining the original event and correction. Six positions and their controls fit without scrolling at the minimum size; the bench remains independently scrollable.
- Futsal: Start changed 12:34 to a running 12:33 countdown; Pause retained the observed reading. Football showed its separate count-up clock, 45-minute period and field pad.
- Football: a home own-goal credited away, making 2-2. Review & finish opened a confirmation; confirming saved a final Draw. The final score, paused time, own-goal and completion remained in local history.
- Padel: selected a saved participant and inspected winner, ace, double-fault and error actions alongside 30/15 point labels and separate game/set counts. Pickleball's side-out buttons say RALLY, since a receiving rally need not add a point.

The first visual pass found oversized volleyball headers and scrollbar overlap in field controls. The final layout compacts the header, reserves scrollbar space and keeps Undo in the fixed sport toolbar. Volleyball actions stay on cards or in the focused outcome chooser rather than expanding a duplicate action list over the history.

## Captures

| Surface | Evidence |
| --- | --- |
| Prior volleyball UI | [Before](volleyball-before.png) |
| Volleyball after restart | [Dark](volleyball-dark.png), [Light](volleyball-light.png), [Undo](volleyball-undo.png) |
| Pickleball | [Dark](pickleball-dark.png), [Light](pickleball-light.png) |
| Padel | [Dark](padel-dark.png), [Light](padel-light.png) |
| Tennis | [Light](tennis-light.png) |
| Badminton | [Light](badminton-light.png) |
| Table Tennis | [Light](table-tennis-light.png) |
| Football | [Dark](football-dark.png), [Saved draw](football-draw.png) |
| Futsal | [Dark](futsal-dark.png), [Light](futsal-light.png) |
| Basketball regression | [Light](basketball-light.png) |

## Review and release boundary

Runtime handoff: [PR #8](https://github.com/HustlX-Apps/hustlx-oss/pull/8), targeting develop. Runtime commit: `f55d5fb`. The local merge flagger exited 0 with Decision FLAG for scoring/statistics/status keywords and no P0 blocker. Merge remains review-owned. This is local console functionality and macOS evidence. Windows GUI, signed distribution, physical power interruption and a full match certification for every sport remain separate release work. Complete reports/exports, remote sync, staff cards, reduction timers, detailed field corrections and hosted competition workflows are not part of this port. Swimming and Taekwondo remain excluded by the approved scope.
