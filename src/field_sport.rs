//! Local football/futsal commands. Replay is independent of UI ticks and wall time.

use crate::Side;
use serde::{Deserialize, Serialize};
use std::{fmt, time::Duration};

/// Matches the native clock editor's three-digit minutes without overflowing arithmetic.
pub const MAX_CLOCK_MS: u64 = 999 * 60_000 + 59_999;
pub const SHOOTOUT_PERIOD: u8 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldEvent {
    Goal,
    OwnGoal,
    Yellow,
    Red,
    Foul,
    Shot,
    Save,
    Assist,
    PenaltyGoal,
    PenaltyMiss,
    ShootoutGoal,
    ShootoutMiss,
}

impl FieldEvent {
    pub fn label(self) -> &'static str {
        match self {
            Self::Goal => "Goal",
            Self::OwnGoal => "Own goal",
            Self::Yellow => "Yellow card",
            Self::Red => "Red card",
            Self::Foul => "Foul",
            Self::Shot => "Shot",
            Self::Save => "Save",
            Self::Assist => "Assist",
            Self::PenaltyGoal => "Penalty scored",
            Self::PenaltyMiss => "Penalty missed",
            Self::ShootoutGoal => "Shootout scored",
            Self::ShootoutMiss => "Shootout missed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldCommand {
    Start,
    Pause,
    SetTime(u64),
    Checkpoint {
        clock_ms: u64,
        running: bool,
    },
    NextPeriod,
    Complete,
    Event {
        side: Side,
        kind: FieldEvent,
        player: Option<i64>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldState {
    pub score: [u16; 2],
    /// Futsal resets at halftime; second-half fouls carry through extra time.
    /// Shootouts clear the accumulated count. Football keeps match totals.
    pub fouls: [u16; 2],
    pub yellow: [u16; 2],
    pub red: [u16; 2],
    pub shots: [u16; 2],
    pub saves: [u16; 2],
    pub assists: [u16; 2],
    pub shootout: [u16; 2],
    pub shootout_attempts: [u16; 2],
    pub period: u8,
    pub clock_ms: u64,
    pub running: bool,
    pub completed: bool,
    futsal: bool,
    regulation_period_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldError(pub &'static str);
impl fmt::Display for FieldError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}
impl std::error::Error for FieldError {}

impl FieldState {
    pub fn new(futsal: bool, period_ms: u64) -> Result<Self, FieldError> {
        if period_ms == 0 || period_ms > MAX_CLOCK_MS {
            return Err(FieldError(
                "Period length must be positive and no longer than 999:59.",
            ));
        }
        Ok(Self {
            score: [0; 2],
            fouls: [0; 2],
            yellow: [0; 2],
            red: [0; 2],
            shots: [0; 2],
            saves: [0; 2],
            assists: [0; 2],
            shootout: [0; 2],
            shootout_attempts: [0; 2],
            period: 1,
            clock_ms: if futsal { period_ms } else { 0 },
            running: false,
            completed: false,
            futsal,
            regulation_period_ms: period_ms,
        })
    }

    pub fn is_futsal(&self) -> bool {
        self.futsal
    }

    pub fn period_ms(&self) -> u64 {
        match self.period {
            1 | 2 => self.regulation_period_ms,
            3 | 4 => {
                if self.futsal {
                    5 * 60_000
                } else {
                    15 * 60_000
                }
            }
            _ => 0,
        }
    }

    pub fn winner(&self) -> Option<Side> {
        use std::cmp::Ordering;
        match self.score[0].cmp(&self.score[1]) {
            Ordering::Greater => Some(Side::Home),
            Ordering::Less => Some(Side::Away),
            Ordering::Equal => match self.shootout[0].cmp(&self.shootout[1]) {
                Ordering::Greater => Some(Side::Home),
                Ordering::Less => Some(Side::Away),
                Ordering::Equal => None,
            },
        }
    }

    /// The caller supplies time since its last committed baseline, not since its last UI tick.
    pub fn elapsed(&self, elapsed: Duration) -> Self {
        let mut next = self.clone();
        if !next.running || next.completed || next.period == SHOOTOUT_PERIOD {
            return next;
        }
        let delta = elapsed.as_millis().min(u128::from(u64::MAX)) as u64;
        next.clock_ms = if next.futsal {
            next.clock_ms.saturating_sub(delta)
        } else {
            next.clock_ms.saturating_add(delta).min(MAX_CLOCK_MS)
        };
        if (next.futsal && next.clock_ms == 0) || (!next.futsal && next.clock_ms == MAX_CLOCK_MS) {
            next.running = false;
        }
        next
    }

    /// Rejected commands cannot partially mutate score, discipline, or time.
    pub fn apply(&mut self, command: FieldCommand) -> Result<(), FieldError> {
        let mut next = self.clone();
        next.apply_inner(command)?;
        *self = next;
        Ok(())
    }

    fn validate_time(&self, clock_ms: u64, running: bool) -> Result<(), FieldError> {
        if clock_ms > MAX_CLOCK_MS || (self.futsal && clock_ms > self.period_ms()) {
            return Err(FieldError("Clock time exceeds the supported period range."));
        }
        if self.period == SHOOTOUT_PERIOD && (clock_ms != 0 || running) {
            return Err(FieldError("Shootout kicks use a stopped clock."));
        }
        if running && ((self.futsal && clock_ms == 0) || clock_ms == MAX_CLOCK_MS) {
            return Err(FieldError("Correct the clock before starting it."));
        }
        Ok(())
    }

    fn apply_inner(&mut self, command: FieldCommand) -> Result<(), FieldError> {
        if self.completed {
            return Err(FieldError(
                "The match is completed. Undo completion before making a correction.",
            ));
        }
        match command {
            FieldCommand::Start => {
                if self.running {
                    return Err(FieldError("The clock is already running."));
                }
                self.validate_time(self.clock_ms, true)?;
                self.running = true;
            }
            FieldCommand::Pause => {
                self.running = false;
            }
            FieldCommand::SetTime(clock_ms) => {
                self.validate_time(clock_ms, false)?;
                self.clock_ms = clock_ms;
                self.running = false;
            }
            FieldCommand::Checkpoint { clock_ms, running } => {
                self.validate_time(clock_ms, running)?;
                self.clock_ms = clock_ms;
                self.running = running;
            }
            FieldCommand::NextPeriod => {
                if self.period >= SHOOTOUT_PERIOD {
                    return Err(FieldError("Shootout is the final period."));
                }
                self.period += 1;
                self.running = false;
                self.clock_ms = if self.futsal { self.period_ms() } else { 0 };
                if self.futsal && matches!(self.period, 2 | SHOOTOUT_PERIOD) {
                    self.fouls = [0; 2];
                }
            }
            FieldCommand::Complete => {
                self.completed = true;
                self.running = false;
            }
            FieldCommand::Event { side, kind, player } => {
                if player.is_some_and(|id| id <= 0) {
                    return Err(FieldError("Choose a valid local player identity."));
                }
                let index = if side == Side::Home { 0 } else { 1 };
                match kind {
                    FieldEvent::Goal | FieldEvent::PenaltyGoal | FieldEvent::OwnGoal => {
                        if self.period == SHOOTOUT_PERIOD {
                            return Err(FieldError(
                                "Use shootout scored or missed for penalty shootout kicks.",
                            ));
                        }
                        let scorer = if kind == FieldEvent::OwnGoal {
                            1 - index
                        } else {
                            index
                        };
                        increment(&mut self.score[scorer])?;
                    }
                    FieldEvent::ShootoutGoal | FieldEvent::ShootoutMiss => {
                        if self.period != SHOOTOUT_PERIOD {
                            return Err(FieldError(
                                "Advance to the shootout before recording a shootout kick.",
                            ));
                        }
                        increment(&mut self.shootout_attempts[index])?;
                        if kind == FieldEvent::ShootoutGoal {
                            increment(&mut self.shootout[index])?;
                        }
                    }
                    FieldEvent::Yellow => increment(&mut self.yellow[index])?,
                    FieldEvent::Red => increment(&mut self.red[index])?,
                    FieldEvent::Foul => increment(&mut self.fouls[index])?,
                    FieldEvent::Shot => increment(&mut self.shots[index])?,
                    FieldEvent::Save => increment(&mut self.saves[index])?,
                    FieldEvent::Assist => increment(&mut self.assists[index])?,
                    FieldEvent::PenaltyMiss => {}
                }
            }
        }
        Ok(())
    }
}

fn increment(value: &mut u16) -> Result<(), FieldError> {
    *value = value
        .checked_add(1)
        .ok_or(FieldError("Event counter limit reached."))?;
    Ok(())
}
