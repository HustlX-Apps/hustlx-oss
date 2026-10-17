# Lineups and substitutions

New matches start with an empty court on each side. In Game Day, click **Starters** on a team, tap bench players in the order you want their court slots, then **Save starters**. Tap a selected starter or Remove to remove them; Clear all resets the draft. Saving starters also replaces the current court for that side. The console marks the most recently saved starting selection with STARTER.

To substitute, click **Subs X**, tap the player going out, then tap a bench replacement. The incoming player takes the outgoing player's slot. Repeat for multiple swaps, then **Confirm subs**. The other team's court and every player's recorded stats stay unchanged. Substitutions preserve the saved starting selection. Remove/Add and Clear all also support short-handed or house lineups. Five players is the usual target; another count, including an empty court, shows an advisory and can still be saved. Players remain identified by match side plus local ID, including when the same player belongs to both teams.

Draft changes do not affect the live court. Cancel or Escape discards them; navigation and closing also discard unconfirmed drafts. Confirmed changes commit immediately to SQLite and survive closing/reopening the app or match. A draft is not an automatic save. Editing reusable team memberships later does not alter this match's roster.

The board shows the current period and clocks. Opening, cancelling or confirming it does not start, pause or reset a clock. A running clock continues while you stage changes. The board's clock button lets you pause/start explicitly; keyboard scoring/clock actions are disabled within the board. A confirmed change stores the current clock snapshot atomically with the lineup. Existing pause-on-close and paused-reopen behavior remains.

**Lineup history** on the scoreboard shows every confirmed starter selection and substitution, newest first, with team, period, game time, incoming/outgoing players and the ordered resulting court. **Undo last** and Backspace remain stat-only. Reverse a substitution by confirming another swap; the earlier history stays intact. Saving starters again records a new starting-lineup selection without deleting earlier receipts.

Press comma for home or period for away, then X to open that side's substitution board. You can also click a player or type their jersey before X. Ambiguous jerseys must resolve to an explicit side/player. Unlike web v2's two-jersey/Enter sequence, X opens the native board and you confirm there. Held keys, modifier combinations, text-field events and pending-save shortcuts remain guarded. Timeout O is still unbound.

If saving fails, the committed court, score and clocks remain unchanged while the proposed board remains visible with SAVE PENDING. Retry save uses the same command ID and captured lineup/clock. No other game-day mutations or normal Quit can bypass it. Reopening a match explicitly discards a failed pending intent and loads committed truth. Force Quit or a crash can still lose memory-only drafts/pending intent. Clock interruption recovery follows the [clock guide](clocks-hotkeys.md).

Schema 3 upgrades preserve existing on-court flags/order, clocks and stats exactly. Earlier versions never recorded chosen starters, so those matches initially say starters not set. No historical starter selection is inferred. Use Starters to record one. The migration is forward-only: preserve a closed-library copy before upgrading; earlier binaries cannot read schema 3.

This feature does not calculate minutes played or plus-minus, automatically substitute fouled-out players, enforce official eligibility rules, add new match-roster identities, or sync with a hosted league.
