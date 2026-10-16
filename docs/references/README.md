# Basketball Game Day v2 reference

Captured October 16, 2026 from the existing local Nuxt application at `/dev/game-day-v2-preview` using Chrome. The preview mounts the real v2 UI components with sample data; it is not a live match, a full runtime verification, or the new desktop application.

- [Dark screenshot](basketball-game-day-v2-dark.jpg)
- [Light screenshot](basketball-game-day-v2-light.jpg)
- Browser viewport: 1728 × 997 CSS pixels, device pixel ratio 2.
- Reference repository: `HustlX-Apps/hustlx-nuxt4`.
- Source revision: `d1ed5be1abbf84edcbd279227f990a65aaaeae27`.
- Source working tree had no tracked changes at capture intake.

## Owning source files

- `app/pages/dev/game-day-v2-preview.vue`: sample-data capture harness.
- `app/pages/game-day/v2.vue`: actual operator page and orchestration.
- `app/components/game-day/v2/GameDayV2ScoreboardStrip.vue`: scoreboard and clock strip.
- `app/components/game-day/v2/GameDayV2RosterPanel.vue`: team actions, on-court players, and bench.
- `app/components/game-day/v2/GameDayV2CommandCenter.vue`: stat pad and pending player credits.
- `app/components/game-day/GameDayPlayByPlayFeed.vue`: event feed.

## Preserve visually

Keep the full-width scoreboard, central game/shot clocks, home purple and away yellow accents, home roster on the left, away roster on the right, and action pad/feed in the center. Preserve large scores, compact rows, clear selected-player feedback, shot made/missed controls, pending scorer credits, keyboard hints, and light/dark themes.

Adapt hosted-product concepts such as lead-table/controller badges, cloud sync status, and online insights to the single-computer local product. A local saved indicator should reflect actual durable storage. Do not reproduce inactive hosted controls merely to match the screenshot.

The sample harness does not establish correctness of its sample totals or prove interaction behavior. It shows existing presentation only. UI reference screenshots do not grant a license to unrelated assets or private repository source.

## Reproduce

From an authorized checkout of the web application, install its documented dependencies, run its dev server, and open `/dev/game-day-v2-preview`. Use a desktop viewport, wait for fonts and layout to load, then capture each theme. No match seeding or production credentials are needed for this harness.
