# Product brief

Recorded October 16, 2026. This captures the initial product request, not shipped functionality.

## Confirmed direction

HustlX OSS uses Rust throughout its interface and domain logic. The user confirmed Rust-rendered UI on October 16, 2026; Iced is the initial rendering toolkit. It is a desktop basketball Game Day application for Windows and macOS. Follow the existing web Game Day v2 UI as closely as practical. Document decisions, behavior, setup, and verification as the project develops.

The local product contains teams, players, and matches. A league, season, bracket, standings system, online account, or hosted backend is not required to run a game.

## Main journey

1. Create teams and players and assign roster memberships.
2. Create a match, select two different teams, and record its name, date, and venue.
3. Set up the match and starting lineups.
4. Run the game offline, recording scores, shots, fouls, substitutions, timeouts, and other basketball statistics.
5. Save and reopen the match, including recovery after the application closes unexpectedly.
6. View a local box score and play-by-play and download local statistics files.

## Operating principles

- Persist accepted actions locally. Never report an action as saved before durable storage succeeds.
- Keep match rosters and display names as match-time snapshots so later roster edits do not rewrite history.
- Identify people by stable IDs, never by matching names or jersey numbers.
- Support corrections and undo with an auditable history.
- Let operators configure and confirm local rules. Missing rule metadata must not block game operation.
- Package required UI assets locally so a disconnected computer can open and use the installed app.
- Keep local operation free of subscription and network gates.

## Deferred

Optional syncing to an eligible paid HustlX league comes later. Entitlement validation, account linking, match mapping, conflict resolution, and the server contract are not designed or implemented yet. Do not invent API routes or copy server assumptions into the desktop product.

Other sports, league management, payments, livestreaming, and simultaneous multi-device operation are outside the first release.

## Open decisions

- Open-source license. MIT is a candidate, not an adopted license.
- First export formats. CSV for analysis and JSON for portable backups are proposed; printable/PDF output can follow.
- Minimum OS versions, supported CPU architectures, installer signing, and release distribution.
- First-release basketball configuration defaults and correction behavior, grounded in the existing operator workflows before implementation.
