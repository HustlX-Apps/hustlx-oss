# Local multi-sport matches

The standalone match flow covers Volleyball, Pickleball, Table Tennis, Tennis (Long Tennis), Badminton and Padel. Basketball retains its existing team, player, statistic, lineup and clock workflow.

## Run a match

1. In Matches, choose the sport. Enter a match name, date, optional venue and two side names. A side may be a team, a singles competitor or a pair. No account or league is required.
2. Review the sport's rules before creating the match. Settings are saved with the match, so later defaults cannot rewrite it.
3. Award a rally to the side that won it. In side-out Pickleball, a receiving-side rally changes service without adding a point. The console shows the current server and score.
4. Use Undo to correct the latest action. It restores scoring and service together, including across a completed game or set. Corrections append to the history rather than removing the original record.
5. When the required units are won, explicitly finish the match. Reopen it from Matches to see its saved result. Undoing completion returns it to the decided state; undoing the deciding rally permits corrected scoring.
6. Close and relaunch the app. Accepted actions and the result reopen from local SQLite storage.

A service override is an explicit recorded command. It helps operators correct the serving side; it does not identify a real player. Numbered service slots represent positions within a side, not verified player identities.

## Scoring families

| Sport | Local scoring behavior |
| --- | --- |
| Volleyball | Rally points, configurable regular and deciding-set targets and winning margin. |
| Pickleball | Side-out or rally scoring, singles or doubles, opening doubles server 2 and subsequent server changes. |
| Table Tennis | Rally points, two-serve turns before deuce and one-serve turns at deuce; singles or numbered doubles service slots. |
| Badminton | Rally points, configured winning margin and hard cap; singles or numbered doubles service slots. |
| Tennis | Tennis point labels, advantage or no-ad, games and sets, set tiebreaks and optional deciding match tiebreak. |
| Padel | Tennis-style point/game/set scoring with advantage, Golden Point or Star Point, and tiebreak settings. |

These are configurable local scoring policies. They are not federation certification. Missing external rule provenance never requires a network check or prevents local match creation.

## Durability and limits

An action becomes visible as saved only after its SQLite transaction commits. A failed write leaves the committed scoreboard unchanged and exposes Retry. Retry retains the command UUID; duplicate acceptance does not score twice. A stale second window must reopen the match before making another change. Pending intent is memory-only until committed and may be lost on abrupt process termination.

The six new consoles record side-level scoring and results. Volleyball rotation/substitution/libero management, advanced player statistics, real participant service-order editing, competition brackets/standings, exports, backups and remote sync are outside this slice. The existing basketball roster controls do not imply those capabilities in another sport.

No sync request, entitlement check or hosted API belongs to the local save path. Future linking must preserve these local identities and history. The hosted Nuxt/API sports release status remains independent of the standalone app.
