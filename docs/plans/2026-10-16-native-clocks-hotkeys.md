# Native clocks, player details, and hotkeys

**Goal:** Run game/shot clocks with start/pause, presets and correction, save them on close/reopen, add player jerseys/positions, and reuse the supported web v2 keyboard mappings.

**Architecture:** Rust owns monotonic countdowns. SQLite schema 2 adds player defaults, match-time position snapshots, clock baselines, and idempotent clock receipts. Commands and stat clock side effects share the existing match revision and transaction lane. UI ticks only request redraw/projection; checkpoints store baselines every five seconds, plus every action, navigation pause, and graceful close. Reopen is paused. Long scheduling gaps pause conservatively with an operator notice; abrupt exits recover the last committed checkpoint paused.

**Source:** hustlx-oss runtime; read-only Nuxt useGameDayHotkeys.ts, GameDayClock.vue, basketball constants, and shared clock invariants. This desktop implementation does not call the web/API clock harness or change its contracts. Payments and hosted League workflows are unaffected.

**Execution:** Local terminal/native app. **Verification:** deterministic Rust clock/repository/session/hotkey tests, formatter, Clippy, build, native min-size controls/keyboard/reopen evidence, shared verifier boundary, merge flagger, Windows/macOS CI. **Docs:** update local storage, development, roadmap, shortcuts and evidence. No root docs edits.

## Behavior

- Start/pause both clocks; independently stop/resume shot countdown while game runs. Game and shot expiry pause both. Reset shot 24/14 without starting a paused clock; game reset returns to 10:00 and 24 paused. Allow explicit game and shot time correction. Next period resets both paused.
- Monotonic Instant anchors exclude wall-clock adjustments. Commands commit before displayed mutations. Failed writes freeze displayed elapsed time, preserve selection, and retain one retry UUID; closing remains blocked until saved or explicitly reopened/discarded.
- Stat events atomically snapshot clocks: field goals and possession changes reset full shot time, offensive rebounds reset short, called fouls pause both. Undo removes the stat projection without rewinding elapsed clock time.
- Forward-only schema 1 -> 2 migration preserves teams, rosters, matches and history. Existing saved matches get 10:00/24 paused, period 1. No profile data is backfilled into saved match identities.
- Player profiles hold optional default jersey and PG/SG/SF/PF/C multi-selection. Blank team jersey inherits default; explicit team jersey wins. Match creation snapshots effective jersey and positions.
- Game Day shortcuts only; ignore modifiers, held-key repeats, captured widget events and typing controls. Number entry retains leading zeros; ambiguous jerseys require an explicit side, avoiding silent wrong-player attribution. Escape clears, Backspace edits jersey or undoes, ? shows help. Unsupported substitutions/timeouts remain unbound.

## Checklist

- [ ] Add failing clock, storage migration/profile, session recovery/failure, and keyboard resolution tests.
- [ ] Implement schema 2 and Rust clocks/player metadata/atomic commands.
- [ ] Wire native countdown, control/correction forms, graceful close, player forms, keyboard guide and guarded subscriptions.
- [ ] Run automated checks and native clock/hotkey/player/reopen journey; document exact recovery and parity limits.
- [ ] Push/update PR targeting develop with required risk evidence and assess Windows/macOS CI.
