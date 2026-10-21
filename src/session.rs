//! One ordered durable command lane for clock and stat actions.
use crate::{
    Side, Stat,
    clock::{ClockCommand, ClockState},
    field_sport::{FieldCommand, FieldState},
    lineup::LineupKind,
    sport::{Command as SportCommand, Sport},
    storage::{Error, MatchDraft, Result, SavedMatch, SportMatchDraft, Store},
};
use std::time::{Duration, Instant, SystemTime};
use uuid::Uuid;
#[derive(Debug, Clone)]
enum Action {
    Record(Side, i64, Stat),
    Undo,
    Sport(SportCommand),
    UndoSport,
    Clock,
    Lineup(Side, LineupKind, Vec<i64>),
}
#[derive(Debug, Clone)]
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
        let now = self.pending.as_ref().map_or(now, |p| p.freeze_at);
        active
            .clock
            .elapsed(now.saturating_duration_since(self.anchor))
    }
    fn active_field(&self) -> Option<&FieldState> {
        self.active.as_ref()?.sport_state.as_ref()?.field.as_ref()
    }
    fn clock_running(&self) -> bool {
        self.active_field().map_or_else(
            || {
                self.active
                    .as_ref()
                    .is_some_and(|active| active.clock.running)
            },
            |field| field.running,
        )
    }
    pub fn field_clock_at(&self, now: Instant) -> FieldState {
        let field = self
            .active_field()
            .expect("Field clock requires an open field match");
        let now = self
            .pending
            .as_ref()
            .map_or(now, |pending| pending.freeze_at);
        field.elapsed(now.saturating_duration_since(self.anchor))
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
        let recovered_field = saved
            .sport_state
            .as_ref()
            .and_then(|state| state.field.as_ref())
            .is_some_and(|field| field.running);
        self.accept(saved, Instant::now());
        if recovered_field {
            self.notice = Some("Recovered the field clock paused at its last saved checkpoint. Review the time before starting.".into());
            self.field_command_at(FieldCommand::Pause, self.anchor)?;
        } else if recovered {
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
    pub fn create_sport(&mut self, draft: &SportMatchDraft) -> Result<()> {
        self.pause_for_close_at(Instant::now())?;
        let saved = self.store.create_sport_match(draft)?;
        self.accept(saved, Instant::now());
        Ok(())
    }
    pub fn create_sport_with_teams(
        &mut self,
        draft: &SportMatchDraft,
        teams: [Option<i64>; 2],
    ) -> Result<()> {
        self.pause_for_close_at(Instant::now())?;
        let saved = self.store.create_sport_match_with_teams(draft, teams)?;
        self.accept(saved, Instant::now());
        Ok(())
    }
    pub fn sport_command(&mut self, command: SportCommand) -> Result<()> {
        self.sport_command_at(command, Instant::now())
    }
    pub fn sport_command_at(&mut self, command: SportCommand, now: Instant) -> Result<()> {
        if let SportCommand::Field(command) = command {
            return self.field_command_at(command, now);
        }
        let mut state = self
            .active
            .as_ref()
            .and_then(|a| a.sport_state.clone())
            .ok_or_else(|| Error::Validation("Open a sport match first.".into()))?;
        state
            .apply(command)
            .map_err(|e| Error::Validation(e.to_string()))?;
        if self.active_field().is_some_and(|field| field.running) {
            self.field_command_at(FieldCommand::Pause, now)?;
        }
        self.begin(Action::Sport(command), self.clock_at(now), now)
    }
    pub fn field_command_at(&mut self, command: FieldCommand, now: Instant) -> Result<()> {
        if self.active_field().is_none() {
            return Err(Error::Validation(
                "Open a football or futsal match first.".into(),
            ));
        }
        let field = self.field_clock_at(now);
        let wrapped = SportCommand::FieldAt {
            clock_ms: field.clock_ms,
            running: field.running,
            command,
        };
        let mut state = self
            .active
            .as_ref()
            .and_then(|active| active.sport_state.clone())
            .expect("Field command requires sport state");
        state
            .apply(wrapped)
            .map_err(|error| Error::Validation(error.to_string()))?;
        self.begin(Action::Sport(wrapped), self.clock_at(now), now)
    }
    fn require_basketball(&self) -> Result<()> {
        if self
            .active
            .as_ref()
            .is_some_and(|a| a.summary.sport != Sport::Basketball)
        {
            return Err(Error::Validation(
                "Use this sport's score controls. Basketball commands do not apply to this match."
                    .into(),
            ));
        }
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
        self.require_basketball()?;
        let Some((side, player)) = self.active.as_ref().and_then(|a| a.game.selected) else {
            return Err(Error::Validation("Select a player first.".into()));
        };
        self.begin(Action::Record(side, player, stat), self.clock_at(now), now)
    }
    pub fn undo(&mut self) -> Result<()> {
        let now = Instant::now();
        if self.active_field().is_some_and(|field| field.running) {
            self.field_command_at(FieldCommand::Pause, now)?;
        }
        let action =
            if let Some(active) = &self.active
                && active.sport_state.is_some()
            {
                if !active.sport_history.iter().any(|e| {
                    e.command.is_some_and(crate::storage::sport_action_undoable) && !e.undone
                }) {
                    return Err(Error::Validation(
                        "There is no sport action to undo.".into(),
                    ));
                }
                Action::UndoSport
            } else {
                Action::Undo
            };
        self.begin(action, self.clock_at(now), now)
    }
    pub fn save_lineup_at(
        &mut self,
        side: Side,
        kind: LineupKind,
        court: Vec<i64>,
        now: Instant,
    ) -> Result<()> {
        self.require_basketball()?;
        let game = &self
            .active
            .as_ref()
            .ok_or_else(|| Error::Validation("Open a match first.".into()))?
            .game;
        crate::lineup::validate(game, side, &court)?;
        self.begin(Action::Lineup(side, kind, court), self.clock_at(now), now)
    }
    pub fn command_at(&mut self, command: ClockCommand, now: Instant) -> Result<()> {
        self.require_basketball()?;
        let clock = self.clock_at(now).command(command)?;
        self.begin(Action::Clock, clock, now)
    }
    pub fn pause_for_close_at(&mut self, now: Instant) -> Result<()> {
        if self.pending.is_some() {
            return Err(Error::Validation("Retry the pending save before closing, or explicitly reopen the match to discard it.".into()));
        }
        if self.active_field().is_some_and(|field| field.running) {
            self.field_command_at(FieldCommand::Pause, now)?;
        } else if self.active.as_ref().is_some_and(|a| a.clock.running) {
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
        if discontinuity && self.clock_running() && self.pending.is_none() {
            return self.interrupted(now);
        }
        self.tick_at(now)
    }
    fn interrupted(&mut self, _now: Instant) -> Result<()> {
        self.notice=Some("Clocks paused after an interruption or system-time change. Review the saved times before restarting.".into());
        if self.active_field().is_some() {
            return self.field_command_at(FieldCommand::Pause, self.last_tick);
        }
        let clock = self.clock_at(self.last_tick).paused();
        self.begin(Action::Clock, clock, self.last_tick)
    }
    pub fn tick_at(&mut self, now: Instant) -> Result<()> {
        if self.pending.is_some() || !self.clock_running() {
            self.last_tick = now;
            return Ok(());
        }
        if now.saturating_duration_since(self.last_tick) > Duration::from_secs(2) {
            return self.interrupted(now);
        }
        self.last_tick = now;
        if self.active_field().is_some() {
            let field = self.field_clock_at(now);
            if !field.running
                || now.saturating_duration_since(self.last_checkpoint) >= Duration::from_secs(5)
            {
                if !field.running {
                    self.notice =
                        Some("Period clock expired. Review the time before continuing.".into());
                }
                self.field_command_at(
                    FieldCommand::Checkpoint {
                        clock_ms: field.clock_ms,
                        running: field.running,
                    },
                    now,
                )?;
            }
            return Ok(());
        }
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
            .clone()
            .ok_or_else(|| Error::Validation("There is no pending save.".into()))?;
        let saved = match &p.action {
            Action::Sport(command) => self
                .store
                .record_sport(p.match_id, p.revision, p.id, *command)?,
            Action::UndoSport => self.store.undo_sport(p.match_id, p.revision, p.id)?,
            Action::Record(side, player, stat) => self.store.record_with_clock(
                p.match_id,
                p.revision,
                p.id,
                *side,
                *player,
                *stat,
                Some(p.clock),
            )?,
            Action::Undo => {
                self.store
                    .undo_with_clock(p.match_id, p.revision, p.id, Some(p.clock))?
            }
            Action::Lineup(side, kind, court) => self
                .store
                .save_lineup(p.match_id, p.revision, p.id, *side, *kind, court, p.clock)?,
            Action::Clock => self
                .store
                .save_clock(p.match_id, p.revision, p.id, p.clock)?,
        };
        // Clock-only commands retain the operator's player selection.
        let selected = if matches!(p.action, Action::Clock | Action::Sport(_)) {
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
