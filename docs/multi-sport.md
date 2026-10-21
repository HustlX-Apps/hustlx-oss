# Local sport consoles

HustlX OSS supports Basketball, Volleyball, Pickleball, Padel, Tennis (Long Tennis), Badminton, Table Tennis, Football and Futsal. Their native Rust interfaces follow the current web operator layout: scoreboard, participant panels, sport actions and saved history. No league, account or network is required.

## Create a match

Choose the sport in Matches, enter its name/date/venue and two side names, and review the local rules. Optionally choose catalog team rosters for either side. Active player names and jerseys are copied into the match and retain their identities when the catalog changes. Named sides without players can still score. Rules are saved with the match.

Use Lineup or Team sheet to select players in court or service order. Save confirms the draft; Cancel writes nothing. A substitution or libero board asks for one outgoing and one incoming player before confirmation. Count and sport-rule deviations are advisory. Valid saved side/player identity and on-court/bench membership remain structural requirements for exchanges.

## Volleyball

The six court positions use the same mirrored P1-P6 layout as the web console. Each card offers ATK, BLK, ACE, ERR, AST, REC, DIG and manual +1. ATK chooses a kill or a neutral attack attempt. ERR chooses an attack, service, block, reception or personal fault. Kills, blocks, aces and manual points award the player's side; errors award the opponent. Attack attempts, assists, receptions and digs do not change the score.

Side-outs rotate the receiving team's live court clockwise. Rotate records an explicit correction. Substitution and libero exchanges preserve the outgoing player's slot and accept agreed house rules. A new set restores the last explicit starters and resets timeout/substitution/libero bookkeeping. Local counts remain in the action history and can be undone; these are not complete official reports or playing-time statistics.

## Racquet sports

| Sport | Scoring and service |
| --- | --- |
| Pickleball | Side-out or rally scoring; singles/doubles, opening doubles server 2 and later service changes. Record the rally winner even when it earns no point. |
| Padel | Tennis-style point/game/set labels, advantage, Golden Point or Star Point, plus configured tiebreaks. |
| Tennis | Advantage or no-ad, games/sets, set tiebreaks and optional deciding match tiebreak. |
| Badminton | Rally scoring, winning margin and hard cap; singles/doubles service slots. |
| Table Tennis | Rally scoring, two-serve turns before deuce and one at deuce; singles or numbered doubles service order. |

Select a participant for winner, ace, error or double-fault attribution where that action applies. The action uses the sport's existing scoring rules, including side-out. The scoreboard shows current point labels, games/sets and service slot. Lineup order assigns local participants to numbered slots; it does not infer online player identities. Serve home/away is a saved operator correction.

## Football and Futsal

Football counts up from zero and continues into stoppage time. Futsal counts down to zero and pauses. Start/Pause controls the local monotonic clock; MM:SS correction pauses it. Default regulation periods are 45 and 20 minutes respectively and can be configured at creation. Next period is available while paused, progressing through two halves, two extra periods and shootout. Extra periods use 15 minutes for Football and 5 for Futsal.

Record goals, own goals, penalties, shots, saves, assists, fouls and yellow/red cards. Select a saved player for attribution or use a team event without a selection. Own goals credit the opposing side. Futsal displays period foul totals and a sixth-foul advisory. Shootout scored/missed attempts have a separate tally and do not add ordinary goals. Review & finish requires confirmation, derives the winner from goals then shootout, and saves tied results as draws.

The clock saves a checkpoint every five seconds and with each field action. Close/reopen pauses at the last saved reading. Sleep, a long tick gap or a wall-time discontinuity pauses at the last observed reading for operator review. Lineup/exchange and Undo pause a running field clock. No time while the app is closed is invented.

## Save, correction and limits

Accepted commands become visible only after their SQLite transaction commits. A failed write leaves committed scores unchanged, freezes the pending clock and exposes Retry. Retry retains the UUID and captured time; duplicate acceptance never scores twice. Stale windows must reopen. Unsaved pending intent is memory-only until committed.

Undo appends a correction and retains the original event. It restores player counts, score, service and court state. Football/Futsal clock-only receipts are not undo targets; retracting a timed event retains its captured clock paused. Undoing completion reopens the match. Rally sports require a decided match before confirmed completion; field sports permit the operator's reviewed draw or decisive result.

Basketball retains its existing v2 statistics, lineup and clock workflow. Basketball-only hotkeys and shot-clock controls do not appear in another sport.

Full official statistical reports, exports/backups, remote sync, staff-card attribution, field reduction timers, detailed event editing, tournament/league management and Swimming/Taekwondo desks are outside this port. Advanced hosted workflows are not implied by a matching native layout. Rules are configurable local policies, not federation certification. Missing external rule provenance never prevents local play.
