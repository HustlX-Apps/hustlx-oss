# Printable box score

The printable box score is a one-page A4 PDF of a saved match, generated inside the app with no browser, network, account or subscription. It is the console owner's own record of a match they ran. This slice covers basketball; the other eight sports follow in later slices.

## Where it is

Every row in **Matches** has a **Box score** button, and the basketball Game Day scoreboard has one beside **Lineup history**. The button is disabled with a "Save pending" hint while a stat or clock save is still awaiting retry, because the sheet reads committed storage only. Opening the box score for the live match goes through normal navigation, which pauses a running clock.

The Box score screen shows the same sheet as native rows and carries **Export PDF**. The export opens the operating system's save dialog with a suggested name such as `2026-10-24-batangas-blazers-vs-lipa-city-hawks-boxscore.pdf`, writes a temporary file beside the target and renames it over the target, then shows the saved path with **Open** and **Show in folder**. Cancelling the dialog writes nothing. A failed write shows the error inline and the button reads **Retry export**. A path without `.pdf` gets the extension appended.

## What prints for basketball

The page carries the match name, date and venue, the two side names with the saved score, and the saved clock reading such as `P4 00:00` in place of a status word, because the app has no basketball completion state and the sheet never invents one.

Scoring by period and the game flow block (biggest lead, biggest run, lead changes, times tied) are derived from clock-stamped plays. They print only when every scoring play carries a stamp; matches recorded before event timing existed show neither.

Each team table has the columns `NO`, `PLAYER`, `MIN`, `PTS`, `FG` made-attempted and `%`, `2P` and `%`, `3P` and `%`, `FT` and `%`, `OR`, `DR`, `TOT`, `AS`, `TO`, `ST`, `BS`, `PF`, `+/-` and `EFF`, with a totals row. Starters carry `*` after the jersey. `MIN` and `+/-` are replayed from the saved starting lineups, substitutions and play stamps; they are omitted as a pair, and dropped from the legend, when either side never saved a starting lineup or when a scoring play has no stamp. A note under a table lists technical, unsportsmanlike and disqualifying fouls, and another lists team fouls by period.

The footer reads "powered by HustlX" with the generation date and app version. HustlX appears nowhere else on the page.

## What never prints

Officials, coaches, timeouts and signature lines are not recorded by the app and are not printed as blanks. No federation or vendor is named; the only descriptor is "Reference-grade box score". Nothing is estimated: a block, column or line appears only when the saved match recorded the data behind it.

## One page, always

The layout measures every string with the bundled Inter and Exo fonts and fits the sheet by a fixed ladder: body 8 pt, then 7.5, then 7, then drop the legend, the game flow block and the notes, then shorten player names, then cap each table at 18 rows with a "+N not shown" line. Cell text wider than its column is shortened with an ellipsis at every step. The same match always renders the same bytes.

## Limits in this slice

Only basketball matches produce a sheet; other sports show "Box score is not available for this sport yet." There is no organizer name or logo yet. Matches without event timing lose the line score, game flow and minutes.

## Verification

```bash
cargo +stable test --locked --offline --lib boxscore
cargo +stable test --locked --offline --bin hustlx-oss boxscores
cargo +stable test --locked --offline --test boxscore
cargo +stable xtask boxscore-fixtures
bash scripts/check.sh
```

The fixture command writes `basketball-full.pdf`, `basketball-unstamped.pdf` and `basketball-oversized.pdf` to `target/boxscore/`. See [evidence](evidence/printable-boxscore/README.md) for rendered pages and the recorded checks.
