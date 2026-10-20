# Clock-stamped events implementation plan

**Goal:** Preserve the period and exact game/shot milliseconds at stat entry and show them in native play-by-play.

**Architecture:** Add nullable event clock fields in SQLite schema 4. The session freezes the pre-action clock in its existing pending command; storage commits that stamp, the stat, its clock effects and revision together. Old events retain unknown timing. No dependencies or hosted contracts change.

Repo set: hustlx-oss. Source of truth: Rust domain/session/storage and SQLite. Execution: terminal and native UI. Verification: Cargo tests, formatting, Clippy/build, native feed smoke check, shared verifier boundary and merge flagger. Documentation impact: local operator/storage/architecture/development docs and roadmap. The API/Nuxt clock and claim harnesses do not own this Rust-only change.

## Decisions

- Store the pre-action clock on each recorded event, including paused and zero readings. A shot reset or foul pause must not change its original timing.
- Save period, game milliseconds and shot milliseconds together; keep running flags too so an explicit retry with a changed clock payload can be rejected.
- Additive migration leaves all old event stamps null and preserves undo links, IDs, scores, lineups and clocks. Never backfill timing from a current clock.
- Native feed shows period/game/shot context, or “Time not recorded” for unknown timing. Keep event order by append order even after clock corrections.
- Undo remains append-only and stat-only. Original recorded stamps survive in storage; no clock rewind.
- Exact retries preserve the original stamp and never duplicate a play. Failures roll back stat/stamp/clock/revision together and preserve the pending stamp until Retry.
- Direct storage recording without an explicit live clock uses the saved clock baseline. The desktop session always supplies the live pre-action clock. Renderer-independent synthetic Game::record has no clock and keeps unknown timing.
- Schema upgrades remain forward-only; preserve a closed-library copy before upgrading.

## Checklist

- [x] Write failing migration, stamp/reopen, clock-effect, correction/order, undo, retry and rollback tests.
- [x] Implement schema 4, event context model, persistence and pre-action session capture; pass targeted tests.
- [x] Display native event timing and unknown legacy timing; inspect both themes at minimum size.
- [x] Update local docs and roadmap; run formatter, workspace tests, Clippy/build, shared gate and merge flagger. The shared gate excludes this Rust repo and was interrupted after selecting unrelated changes; see event timing evidence.
- [ ] Commit, push and open a PR targeting develop with evidence and remaining platform limits.
