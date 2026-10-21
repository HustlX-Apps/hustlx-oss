//! Participant bookkeeping shared by the native sport consoles. IDs never come from names.
use crate::{
    Side,
    sport::{ScoreError, Sport},
};
use serde::{Deserialize, Serialize};

pub fn index(side: Side) -> usize {
    if side == Side::Home { 0 } else { 1 }
}
pub fn opposite(side: Side) -> Side {
    if side == Side::Home {
        Side::Away
    } else {
        Side::Home
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlayerAction {
    Kill,
    Attack,
    Block,
    Ace,
    AttackError,
    ServiceError,
    BlockError,
    ReceptionError,
    Fault,
    Assist,
    Reception,
    Dig,
    Point,
    Winner,
    UnforcedError,
    ForcedError,
    DoubleFault,
}
impl PlayerAction {
    pub fn label(self) -> &'static str {
        match self {
            Self::Kill => "Kill",
            Self::Attack => "Attack / miss",
            Self::Block => "Block",
            Self::Ace => "Ace",
            Self::AttackError => "Attack error",
            Self::ServiceError => "Service error",
            Self::BlockError => "Block error",
            Self::ReceptionError => "Reception error",
            Self::Fault => "Fault",
            Self::Assist => "Assist",
            Self::Reception => "Reception",
            Self::Dig => "Dig",
            Self::Point => "Point",
            Self::Winner => "Winner",
            Self::UnforcedError => "Unforced error",
            Self::ForcedError => "Forced error",
            Self::DoubleFault => "Double fault",
        }
    }
    pub fn for_sport(sport: Sport) -> &'static [Self] {
        match sport {
            Sport::Volleyball => &[
                Self::Kill,
                Self::Attack,
                Self::Block,
                Self::Ace,
                Self::AttackError,
                Self::ServiceError,
                Self::BlockError,
                Self::ReceptionError,
                Self::Fault,
                Self::Assist,
                Self::Reception,
                Self::Dig,
                Self::Point,
            ],
            Sport::Basketball | Sport::Football | Sport::Futsal => &[],
            Sport::Tennis | Sport::Padel => &[
                Self::Winner,
                Self::Ace,
                Self::DoubleFault,
                Self::UnforcedError,
                Self::ForcedError,
            ],
            _ => &[
                Self::Winner,
                Self::Ace,
                Self::ServiceError,
                Self::UnforcedError,
                Self::ForcedError,
            ],
        }
    }
    pub fn winner(self, side: Side) -> Option<Side> {
        match self {
            Self::Attack | Self::Assist | Self::Reception | Self::Dig => None,
            Self::AttackError
            | Self::ServiceError
            | Self::BlockError
            | Self::ReceptionError
            | Self::Fault
            | Self::UnforcedError
            | Self::ForcedError
            | Self::DoubleFault => Some(opposite(side)),
            _ => Some(side),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerTally {
    pub side: Side,
    pub player: i64,
    counts: [u16; 17],
}
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorState {
    /// Volleyball: P1..P6. Other sports: explicit entrant/on-pitch order.
    pub court: [Vec<i64>; 2],
    pub starters: [Vec<i64>; 2],
    pub libero: [Option<i64>; 2],
    pub substitutions: [u16; 2],
    pub timeouts: [u16; 2],
    pub stats: Vec<PlayerTally>,
}
impl OperatorState {
    pub fn count(&self, side: Side, player: i64, action: PlayerAction) -> u16 {
        self.stats
            .iter()
            .find(|s| s.side == side && s.player == player)
            .map_or(0, |s| s.counts[action as usize])
    }
    pub fn record(
        &mut self,
        side: Side,
        player: Option<i64>,
        action: PlayerAction,
    ) -> Result<(), ScoreError> {
        let Some(player) = player else {
            return Ok(());
        };
        if player <= 0 {
            return Err(ScoreError("Choose a valid local player"));
        }
        if !self
            .stats
            .iter()
            .any(|s| s.side == side && s.player == player)
        {
            self.stats.push(PlayerTally {
                side,
                player,
                counts: [0; 17],
            });
        }
        let tally = self
            .stats
            .iter_mut()
            .find(|s| s.side == side && s.player == player)
            .unwrap();
        let n = &mut tally.counts[action as usize];
        *n = n
            .checked_add(1)
            .ok_or(ScoreError("Statistic limit reached"))?;
        Ok(())
    }
    pub fn lineup(&mut self, side: Side, players: [i64; 11]) -> Result<(), ScoreError> {
        let mut court = Vec::new();
        for player in players {
            if player == 0 {
                continue;
            }
            if player < 0 || court.contains(&player) {
                return Err(ScoreError("Lineups require distinct valid local players"));
            }
            court.push(player);
        }
        self.starters[index(side)] = court.clone();
        self.court[index(side)] = court;
        self.libero[index(side)] = None;
        Ok(())
    }
    pub fn exchange(
        &mut self,
        side: Side,
        outgoing: i64,
        incoming: i64,
        libero: bool,
    ) -> Result<(), ScoreError> {
        let i = index(side);
        let court = &mut self.court[i];
        if incoming <= 0 || court.contains(&incoming) {
            return Err(ScoreError("Choose an incoming player from the bench"));
        }
        let slot = court
            .iter()
            .position(|p| *p == outgoing)
            .ok_or(ScoreError("Choose an outgoing player on court"))?;
        court[slot] = incoming;
        if libero {
            self.libero[i] = if self.libero[i] == Some(outgoing) {
                None
            } else {
                Some(incoming)
            };
        } else {
            self.substitutions[i] = self.substitutions[i]
                .checked_add(1)
                .ok_or(ScoreError("Substitution limit reached"))?;
            if self.libero[i] == Some(outgoing) {
                self.libero[i] = None;
            }
        }
        Ok(())
    }
    pub fn rotate(&mut self, side: Side) {
        let court = &mut self.court[index(side)];
        if !court.is_empty() {
            court.rotate_left(1);
        }
    }
    pub fn timeout(&mut self, side: Side) -> Result<(), ScoreError> {
        let n = &mut self.timeouts[index(side)];
        *n = n
            .checked_add(1)
            .ok_or(ScoreError("Timeout count limit reached"))?;
        Ok(())
    }
    pub fn begin_unit(&mut self) {
        self.court = self.starters.clone();
        self.timeouts = [0; 2];
        self.substitutions = [0; 2];
        self.libero = [None; 2];
    }
}
