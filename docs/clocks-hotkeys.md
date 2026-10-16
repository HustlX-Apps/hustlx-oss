# Clocks, player details and shortcuts

Game Day starts at period 1 with 10:00 on the game clock and 24 on the shot clock, both paused. This slice uses fixed 5v5 defaults. It does not infer rule configurations from a league.

## Clock controls

Space or Start/Pause controls both clocks. The separate shot button stops/resumes only the shot countdown while the game clock runs. Shot resets set 24 or 14 without starting a paused countdown. Reset game restores 10:00/24 paused in the current period; Next Q advances the period and restores those times paused.

Set times pauses both clocks and allows a game time in MM:SS (up to 24 hours) and shot time from 0 to 120 seconds. Times remain paused after correction. A zero clock must be corrected or reset before starting. The shot display shows tenths at five seconds or less. Expiry of either running countdown pauses both at the first expiry and shows a notice. There is no audible buzzer yet.

Made field goals, defensive rebounds, steals and turnovers reset the shot clock to 24. Offensive rebounds reset it to 14. Called personal, technical, unsportsmanlike and disqualifying fouls pause both clocks. Free throws and warnings leave clock operation unchanged. These are the implemented v2 action mappings; operators can correct clocks manually. Undo removes the latest stat without rewinding elapsed clock time.

## Saving and recovery

Clock commands and stats share one SQLite revision lane. Stats and their clock side effects commit in the same transaction. A failed save freezes the displayed clock projection, leaves the committed score intact, and retains one command for Retry. No later actions run until it saves or the operator explicitly reopens a match to discard it.

Closing a match by navigation, closing the window, or using macOS Quit saves the remaining milliseconds paused. Reopening always starts paused and excludes the closed interval. Normal Quit stays open when a save is pending. The macOS adapter cancels AppKit's native termination request and forwards it to the same save path; recheck this behavior when upgrading Iced/winit. If adapter installation fails, an error directs the operator to use the window close button.

Running clocks also checkpoint every five seconds. Force Quit, power loss or a crash can recover only the last committed checkpoint, which reopens paused with a review notice. A scheduling gap over two seconds or a wall/monotonic time discrepancy pauses at the last observed time. This deliberately avoids guessing elapsed play during sleep or interruptions. Review and correct times before pressing Start. Pending intents remain memory-only.

## Player details

Players supports an optional default jersey and any combination of PG, SG, SF, PF and C. Team membership jerseys override the default; a blank membership jersey inherits it. New matches snapshot effective jerseys and positions. Editing a profile later preserves saved match history. Jerseys and names do not determine identity; duplicate jerseys are allowed.

## Keyboard guide

Shortcuts use the supported mappings from web v2. They work on Game Day, outside text fields, the help/editor panels and pending saves. Held-key repeats and Ctrl/Command/Alt combinations are ignored. Open **Hotkeys ?** for the in-app guide.

| Key | Action |
| --- | --- |
| Space | Start/pause both clocks |
| `[` / `]` | Reset shot to 24 / 14 |
| P | Next period, clocks paused |
| Q / W / E | Made free throw / two / three |
| A / S / D | Missed free throw / two / three |
| R / Y | Defensive / offensive rebound |
| F / G | Personal foul / assist |
| T / V / B | Turnover / steal / block |
| Backspace | Edit typed jersey, or undo latest play when no digits remain |
| Escape | Clear jersey, side hint and selected player |
| `?` | Show/hide guide |
| Numpad `/` / `+` / `*` / `-` | Made free throw / two / three / missed two |

Click a player, then a stat key, or type their jersey followed by a stat key. Two digits are supported, including `00`. Comma chooses home; period chooses away. For example, comma, 4, W records a home #4 made two. Period, 3, E records an away #3 made three. The jersey buffer clears after a successful stat. Ambiguous jerseys require an explicit side or clicked player; the application never guesses. Clicking a player clears the typed buffer.

Substitution and timeout shortcuts X/O remain unbound because those workflows are not implemented. There is no active-match roster editor, period-duration configuration, official closeout, minutes/plus-minus calculation, export or sync in this slice.
