//! Match-specific ordered lineups and uncommitted substitution boards.
use crate::{
    Game, Side,
    clock::ClockState,
    storage::{Error, Result, SavedMatch},
};
use std::collections::HashSet;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineupKind {
    Starters,
    Substitution,
}
impl LineupKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Starters => "Starting lineup",
            Self::Substitution => "Substitution",
        }
    }
    pub(crate) fn key(self) -> &'static str {
        match self {
            Self::Starters => "starters",
            Self::Substitution => "substitution",
        }
    }
}
#[derive(Debug, Clone)]
pub struct LineupChange {
    pub side: Side,
    pub kind: LineupKind,
    pub before: Vec<i64>,
    pub court: Vec<i64>,
    pub clock: ClockState,
}
pub fn validate(game: &Game, side: Side, court: &[i64]) -> Result<()> {
    let mut seen = HashSet::new();
    for id in court {
        if game.player(side, *id).is_none() {
            return Err(Error::Validation(
                "Choose players from this side's saved match roster.".into(),
            ));
        }
        if !seen.insert(*id) {
            return Err(Error::Validation(
                "A player can occupy only one court slot.".into(),
            ));
        }
    }
    Ok(())
}
#[derive(Debug, Clone)]
pub struct LineupDraft {
    pub match_id: i64,
    pub side: Side,
    pub kind: LineupKind,
    pub court: Vec<i64>,
    pub outgoing: Option<i64>,
    initial: Vec<i64>,
}
impl LineupDraft {
    pub fn new(saved: &SavedMatch, side: Side, kind: LineupKind) -> Self {
        let court = match kind {
            LineupKind::Starters => saved
                .starting_lineup(side)
                .map_or_else(|| saved.game.court_ids(side), |p| p.to_vec()),
            LineupKind::Substitution => saved.game.court_ids(side),
        };
        Self {
            match_id: saved.summary.id,
            side,
            kind,
            initial: court.clone(),
            court,
            outgoing: None,
        }
    }
    pub fn changed(&self) -> bool {
        self.court != self.initial
    }
    pub fn warning(&self) -> Option<String> {
        (self.court.len()!=5).then(||format!("{} players selected. The usual lineup is 5; you can save this lineup for your game.",self.court.len()))
    }
    pub fn clear(&mut self) {
        self.court.clear();
        self.outgoing = None;
    }
    pub fn remove(&mut self, id: i64) {
        self.court.retain(|p| *p != id);
        self.outgoing = None;
    }
    pub fn pick(&mut self, game: &Game, id: i64) -> Result<()> {
        if game.player(self.side, id).is_none() {
            return Err(Error::Validation(
                "Choose a player from this side's match roster.".into(),
            ));
        }
        if self.court.contains(&id) {
            if self.kind == LineupKind::Starters {
                self.remove(id);
            } else {
                self.outgoing = if self.outgoing == Some(id) {
                    None
                } else {
                    Some(id)
                };
            }
        } else if let Some(out) = self.outgoing.take() {
            if let Some(slot) = self.court.iter_mut().find(|p| **p == out) {
                *slot = id;
            }
        } else {
            self.court.push(id);
        }
        Ok(())
    }
}
