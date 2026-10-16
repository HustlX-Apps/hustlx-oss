//! One ordered durable command lane for clock and stat actions.
use crate::{
    Side, Stat,
    clock::{ClockCommand, ClockState},
    storage::{Error, MatchDraft, Result, SavedMatch, Store},
};
use std::time::{Duration, Instant, SystemTime};
use uuid::Uuid;
#[derive(Debug, Clone, Copy)]
enum Action {
    Record(Side, i64, Stat),
    Undo,
    Clock,
}
#[derive(Debug, Clone, Copy)]
struct Pending {
    id: Uuid,
    match_id: i64,
    revision: i64,
    action: Action,
    clock: ClockState,
    freeze_at: Instant,
}
pub struct Session {
    store: Store,
    active: Option<SavedMatch>,
    pending: Option<Pending>,
    anchor: Instant,
    last_tick: Instant,
    last_checkpoint: Instant,
    last_wall: SystemTime,
    notice: Option<String>,
}
impl Session {
    pub fn new(store: Store) -> Self {
        let now = Instant::now();
        Self {
            store,
            active: None,
            pending: None,
            anchor: now,
            last_tick: now,
            last_checkpoint: now,
            last_wall: SystemTime::now(),
            notice: None,
        }
    }
    pub fn store(&self) -> &Store {
        &self.store
    }
    pub fn store_mut(&mut self) -> &mut Store {
        &mut self.store
    }
    pub fn active(&self) -> Option<&SavedMatch> {
        self.active.as_ref()
    }
    pub fn has_pending_save(&self) -> bool {
        self.pending.is_some()
    }
    pub fn take_clock_notice(&mut self) -> Option<String> {
        self.notice.take()
    }
    pub fn clock_at(&self, now: Instant) -> ClockState {
        let Some(active) = &self.active else {
            return ClockState::default();
        };
        let now = self.pending.map_or(now, |p| p.freeze_at);
        active
            .clock
            .elapsed(now.saturating_duration_since(self.anchor))
    }
    fn accept(&mut self, saved: SavedMatch, now: Instant) {
        self.active = Some(saved);
        self.pending = None;
        self.anchor = now;
        self.last_tick = now;
        self.last_checkpoint = now;
        self.last_wall = SystemTime::now();
    }
    pub fn open(&mut self, id: i64) -> Result<()> {
        if self.pending.is_none() {
            self.pause_for_close_at(Instant::now())?;
        }
        let saved = self.store.load_match(id)?;
        let recovered = saved.clock.running;
        self.accept(saved, Instant::now());
        if recovered {
            self.notice=Some("Recovered a running clock at its last saved checkpoint. Review the times, then press Start.".into());
            let clock = self.active.as_ref().expect("Opened match").clock.paused();
            self.begin(Action::Clock, clock, Instant::now())?;
        }
        Ok(())
    }
    pub fn create(&mut self, draft: &MatchDraft) -> Result<()> {
        self.pause_for_close_at(Instant::now())?;
        let saved = self.store.create_match(draft)?;
        self.accept(saved, Instant::now());
        Ok(())
    }
    pub fn select(&mut self, side: Side, player: i64) {
        if self.pending.is_none()
            && let Some(a) = &mut self.active
        {
            a.game.select(side, player);
        }
    }
    pub fn clear_selection(&mut self) {
        if let Some(a) = &mut self.active {
            a.game.selected = None;
        }
    }
    pub fn record(&mut self, stat: Stat) -> Result<()> {
        self.record_at(stat, Instant::now())
    }
    pub fn record_at(&mut self, stat: Stat, now: Instant) -> Result<()> {
        let Some((side, player)) = self.active.as_ref().and_then(|a| a.game.selected) else {
            return Err(Error::Validation("Select a player first.".into()));
        };
        self.begin(
            Action::Record(side, player, stat),
            self.clock_at(now).for_stat(stat),
            now,
        )
    }
    pub fn undo(&mut self) -> Result<()> {
        let now = Instant::now();
        self.begin(Action::Undo, self.clock_at(now), now)
    }
    pub fn command_at(&mut self, command: ClockCommand, now: Instant) -> Result<()> {
        let clock = self.clock_at(now).command(command)?;
        self.begin(Action::Clock, clock, now)
    }
    pub fn pause_for_close_at(&mut self, now: Instant) -> Result<()> {
        if self.pending.is_some() {
            return Err(Error::Validation("Retry the pending save before closing, or explicitly reopen the match to discard it.".into()));
        }
        if self.active.as_ref().is_some_and(|a| a.clock.running) {
            self.command_at(ClockCommand::Pause, now)?;
        }
        Ok(())
    }
    pub fn tick_live(&mut self, now: Instant) -> Result<()> {
        self.tick_with_wall_at(now, SystemTime::now())
    }
    pub fn tick_with_wall_at(&mut self, now: Instant, wall: SystemTime) -> Result<()> {
        let gap = wall.duration_since(self.last_wall).ok();
        let mono = now.saturating_duration_since(self.last_tick);
        let discontinuity = gap.is_none_or(|d| d.abs_diff(mono) > Duration::from_secs(2));
        self.last_wall = wall;
        if discontinuity
            && self.active.as_ref().is_some_and(|a| a.clock.running)
            && self.pending.is_none()
        {
            return self.interrupted(now);
        }
        self.tick_at(now)
    }
    fn interrupted(&mut self, _now: Instant) -> Result<()> {
        let clock = self.clock_at(self.last_tick).paused();
        self.notice=Some("Clocks paused after an interruption or system-time change. Review the saved times before restarting.".into());
        self.begin(Action::Clock, clock, self.last_tick)
    }
    pub fn tick_at(&mut self, now: Instant) -> Result<()> {
        if self.pending.is_some() || self.active.as_ref().is_none_or(|a| !a.clock.running) {
            self.last_tick = now;
            return Ok(());
        }
        if now.saturating_duration_since(self.last_tick) > Duration::from_secs(2) {
            return self.interrupted(now);
        }
        self.last_tick = now;
        let clock = self.clock_at(now);
        if !clock.running
            || now.saturating_duration_since(self.last_checkpoint) >= Duration::from_secs(5)
        {
            if !clock.running {
                self.notice = Some(
                    if clock.game_ms == 0 {
                        "Period clock expired. Both clocks are paused."
                    } else {
                        "Shot clock expired. Both clocks are paused."
                    }
                    .into(),
                );
            }
            self.begin(Action::Clock, clock, now)?;
        }
        Ok(())
    }
    fn begin(&mut self, action: Action, clock: ClockState, now: Instant) -> Result<()> {
        if self.pending.is_some() {
            return Err(Error::Validation(
                "Retry the pending save or reopen the match before recording another action."
                    .into(),
            ));
        }
        let a = self
            .active
            .as_ref()
            .ok_or_else(|| Error::Validation("Open a match first.".into()))?;
        self.pending = Some(Pending {
            id: Uuid::new_v4(),
            match_id: a.summary.id,
            revision: a.summary.revision,
            action,
            clock,
            freeze_at: now,
        });
        self.commit(now)
    }
    fn commit(&mut self, now: Instant) -> Result<()> {
        let p = self
            .pending
            .ok_or_else(|| Error::Validation("There is no pending save.".into()))?;
        let saved = match p.action {
            Action::Record(side, player, stat) => self.store.record_with_clock(
                p.match_id,
                p.revision,
                p.id,
                side,
                player,
                stat,
                Some(p.clock),
            )?,
            Action::Undo => {
                self.store
                    .undo_with_clock(p.match_id, p.revision, p.id, Some(p.clock))?
            }
            Action::Clock => self
                .store
                .save_clock(p.match_id, p.revision, p.id, p.clock)?,
        };
        // Clock-only commands retain the operator's player selection.
        let selected = if matches!(p.action, Action::Clock) {
            self.active.as_ref().and_then(|a| a.game.selected)
        } else {
            None
        };
        self.accept(saved, now);
        if let Some(a) = &mut self.active {
            a.game.selected = selected;
        }
        Ok(())
    }
    pub fn retry(&mut self) -> Result<()> {
        self.commit(Instant::now())
    }
}
