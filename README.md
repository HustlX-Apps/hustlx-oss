# HustlX OSS

Offline-first Game Day for Windows and macOS, built with Rust, Iced and SQLite. Run Volleyball, Pickleball, Table Tennis, Tennis (Long Tennis), Badminton, Padel, Football and Futsal matches locally alongside the existing basketball console.

**Status:** native Rust application in development, licensed under MIT. The Iced interface now supports a SQLite team/player library, roster editing, match creation and reopening, durable stat entry/undo, game/shot clocks, jersey/position profiles, supported web v2 hotkeys, starting lineups and confirmed substitutions. The sport consoles use native participant panels, score/unit or clock strips, action pads and saved history. Optional roster snapshots support player attribution, lineups and exchanges. Volleyball includes a six-position court, rotation and libero exchanges; Football and Futsal include their own clocks, events, shootouts and draw-aware completion. Basketball matches print to a one-page box score PDF and export a stats CSV from inside the app. Cloud sync, statistical reports for the other sports, other exports and installers remain planned.

The interface uses the official HustlX wordmarks, Inter body text, Exo headings and Exo Black scores, bundled for offline use. See [branding](docs/branding.md).

## What we are building

- Manage teams, players, and rosters on your computer.
- Select Team A versus Team B without creating a league.
- Save matches with a name, date, and venue.
- Operate a basketball scoreboard, game clock, shot clock, lineups, and statistics offline.
- Reopen saved matches, view results and player statistics, and export files locally.
- Later, optionally sync with an eligible paid league on HustlX. Local game operation must remain independent of connectivity and subscription state.

## Run a local sport match

Run `cargo run --locked`, open **Matches**, choose a sport, enter two side names and match details, review the local rules, then create the match. Optionally select saved team rosters for player controls. Award rallies or field events, correct with Undo and review the final result before completing the match. Reopen it after restarting to see the saved score, service and result. See [sport workflows and limits](docs/multi-sport.md) and [current port evidence](docs/evidence/sport-ui-port/README.md).

No league, login, server or runtime network access is required. Side names may represent teams, individual players or pairs.

## Try the basketball library

Run `cargo run --locked`, then:

1. Open **Players** and create players with optional default jerseys and positions.
2. Open **Teams**, create two teams, and assign players and optional jersey labels.
3. Open **Matches**, choose Team A and Team B, enter a name and date, optionally enter a venue, then create the match.
4. In Game Day, use **Starters** for each side, choose players, and save. Use **Subs X** to stage swaps, then confirm. Saved starters and current lineups survive restart.
5. Select a player or type a jersey to record a stat. Space starts/pauses clocks, `[` resets 24, and `]` resets 14. Open **Hotkeys ?** for the full guide.
6. Close the app, reopen it, and choose **Open match**. Both clocks reopen paused. See [clocks and shortcuts](docs/clocks-hotkeys.md) for correction and recovery behavior.

Team and player names and rosters are copied into each new match. Later library edits preserve its history. The library starts empty; screenshots use synthetic test data.

New stat events also retain their period and game/shot readings before clock effects. Play-by-play shows that saved context after restart; older events without timing show **Time not recorded**. See [event timing evidence](docs/evidence/clock-stamped-events/README.md).

![Saved native match reopened after restart](docs/evidence/native-lineups/reopened-match.jpg)

[Match library](docs/evidence/hustlx-branding/matches-dark.jpg), [light theme](docs/evidence/hustlx-branding/game-day-light.jpg), and [current visual evidence](docs/evidence/hustlx-branding/README.md). [SQLite verification](docs/evidence/sqlite-library/README.md) records the original create/restart journey. See [local storage](docs/storage.md) for data locations and save/retry behavior.

## Print a box score

Open **Matches** and press **Box score** on a saved basketball match, or press it on the Game Day scoreboard. The screen shows the sheet and **Export PDF** opens the system save dialog; the one-page A4 PDF carries scoring by period, the full stat table with minutes and plus-minus when lineups were saved, and the game flow. **Export CSV** saves the same stats plus play-by-play for spreadsheets. See [printable box score](docs/printable-boxscore.md).

## Web UI reference

This is the existing web console rendered with sample data, not the desktop application:

[Basketball Game Day v2, dark mode](docs/references/basketball-game-day-v2-dark.jpg)

[Light-mode screenshot](docs/references/basketball-game-day-v2-light.jpg)

## Development

Install Rust with [rustup](https://rustup.rs/), then run `cargo run --locked`. SQLite is bundled; no database server is needed. See [development and verification](docs/development.md).

CI/CD is paused to save cost. GitHub Actions is disabled; all checks run locally with `bash scripts/check.sh` on macOS or `./scripts/check.ps1` in Windows PowerShell. Hosted builds and automated releases require an explicit decision to resume them. See [the local-only decision](docs/notes.md).

## Documentation

- [Branding](docs/branding.md): wordmarks, typography, palettes, and asset provenance.
- [Product brief](docs/product-brief.md): confirmed scope and open decisions.
- [Architecture](docs/architecture-proposal.md): implemented Rust/SQLite/clock boundaries and proposed sync.
- [Lineups and substitutions](docs/lineups-substitutions.md): starting players, swaps, history and save behavior.
- [Printable box score](docs/printable-boxscore.md): the one-page basketball PDF, the stats CSV, what prints, and the export flow.
- [Local storage](docs/storage.md): schema, data paths, snapshots, and failure handling.
- [Versions and local packages](docs/releases.md): local version preparation, Windows/macOS packaging and signing limits.
- [Roadmap](docs/roadmap.md): implementation slices and acceptance evidence.
- [UI reference](docs/references/README.md): capture provenance and visual parity notes.
- [Contributing](CONTRIBUTING.md): small changes, documentation, and verification expectations.

## License

Project code and documentation are licensed under the [MIT License](LICENSE). You may use, modify, redistribute, and sell the software, including in closed-source products, provided you retain the copyright and license notice. Contributions are accepted under the same license; see [contributing](CONTRIBUTING.md).

Bundled fonts and dependencies retain their respective licenses. HustlX logos and wordmarks are excluded from the MIT license; no HustlX trademark rights are granted. See [asset and third-party notices](THIRD_PARTY_NOTICES.md). The Rust application is a new implementation; the private web application's source has not been copied or licensed by this repository.
