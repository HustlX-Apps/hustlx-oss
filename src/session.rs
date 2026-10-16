//! The visible match changes only after a successful durable command.
use crate::{
    Side, Stat,
    storage::{Error, MatchDraft, Result, SavedMatch, Store},
};
use uuid::Uuid;

#[derive(Debug, Clone, Copy)]
enum Action {
    Record(Side, i64, Stat),
    Undo,
}
#[derive(Debug, Clone, Copy)]
struct Pending {
    id: Uuid,
    match_id: i64,
    revision: i64,
    action: Action,
}

pub struct Session {
    store: Store,
    active: Option<SavedMatch>,
    pending: Option<Pending>,
}
impl Session {
    pub fn new(store: Store) -> Self {
        Self {
            store,
            active: None,
            pending: None,
        }
    }
    pub fn store(&self) -> &Store {
        &self.store
    }
    pub fn active(&self) -> Option<&SavedMatch> {
        self.active.as_ref()
    }
    pub fn has_pending_save(&self) -> bool {
        self.pending.is_some()
    }
    pub fn open(&mut self, id: i64) -> Result<()> {
        let saved = self.store.load_match(id)?;
        self.active = Some(saved);
        self.pending = None;
        Ok(())
    }
    pub fn create(&mut self, draft: &MatchDraft) -> Result<()> {
        let saved = self.store.create_match(draft)?;
        self.active = Some(saved);
        self.pending = None;
        Ok(())
    }
    pub fn select(&mut self, side: Side, player: i64) {
        if self.pending.is_none()
            && let Some(active) = &mut self.active
        {
            active.game.select(side, player);
        }
    }
    pub fn record(&mut self, stat: Stat) -> Result<()> {
        let Some((side, player)) = self.active.as_ref().and_then(|s| s.game.selected) else {
            return Err(Error::Validation("Select a player first.".into()));
        };
        self.begin(Action::Record(side, player, stat))
    }
    pub fn undo(&mut self) -> Result<()> {
        self.begin(Action::Undo)
    }
    fn begin(&mut self, action: Action) -> Result<()> {
        if self.pending.is_some() {
            return Err(Error::Validation(
                "Retry the pending save or reopen the match before recording another action."
                    .into(),
            ));
        }
        let active = self
            .active
            .as_ref()
            .ok_or_else(|| Error::Validation("Open a match first.".into()))?;
        self.pending = Some(Pending {
            id: Uuid::new_v4(),
            match_id: active.summary.id,
            revision: active.summary.revision,
            action,
        });
        self.retry()
    }
    pub fn retry(&mut self) -> Result<()> {
        let pending = self
            .pending
            .ok_or_else(|| Error::Validation("There is no pending save.".into()))?;
        let saved = match pending.action {
            Action::Record(side, player, stat) => self.store.record(
                pending.match_id,
                pending.revision,
                pending.id,
                side,
                player,
                stat,
            )?,
            Action::Undo => self
                .store
                .undo(pending.match_id, pending.revision, pending.id)?,
        };
        self.active = Some(saved);
        self.pending = None;
        Ok(())
    }
}
