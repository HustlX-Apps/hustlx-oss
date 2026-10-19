//! Countdown baselines. Rendering never decrements or persists individual ticks.
use crate::{
    Stat,
    storage::{Error, Result},
};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClockState {
    pub game_ms: u64,
    pub shot_ms: u64,
    pub period: u32,
    pub running: bool,
    pub shot_running: bool,
}
impl Default for ClockState {
    fn default() -> Self {
        Self {
            game_ms: 600_000,
            shot_ms: 24_000,
            period: 1,
            running: false,
            shot_running: false,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClockCommand {
    Start,
    Pause,
    ToggleShot,
    FullShot,
    ShortShot,
    ResetGame,
    NextPeriod,
    SetGame(u64),
    SetShot(u64),
}
impl ClockState {
    pub fn validate(self) -> Result<()> {
        if self.game_ms > 86_400_000
            || self.shot_ms > 120_000
            || self.period == 0
            || self.period > 999
            || (self.shot_running && !self.running)
            || (self.running && self.game_ms == 0)
            || (self.shot_running && self.shot_ms == 0)
        {
            return Err(Error::Validation("Invalid clock state. Game time supports up to 24 hours; shot time up to 120 seconds.".into()));
        }
        Ok(())
    }
    pub fn paused(mut self) -> Self {
        self.running = false;
        self.shot_running = false;
        self
    }
    pub fn elapsed(mut self, elapsed: Duration) -> Self {
        if !self.running {
            return self;
        }
        let ms = (elapsed.as_millis().min(u64::MAX as u128) as u64)
            .min(self.game_ms)
            .min(if self.shot_running {
                self.shot_ms
            } else {
                u64::MAX
            });
        self.game_ms = self.game_ms.saturating_sub(ms);
        if self.shot_running {
            self.shot_ms = self.shot_ms.saturating_sub(ms);
        }
        if self.game_ms == 0 || (self.shot_running && self.shot_ms == 0) {
            self = self.paused();
        }
        self
    }
    pub fn command(mut self, command: ClockCommand) -> Result<Self> {
        match command {
            ClockCommand::Start => {
                if self.game_ms == 0 || self.shot_ms == 0 {
                    return Err(Error::Validation(
                        "Reset or set an expired clock before starting.".into(),
                    ));
                }
                self.running = true;
                self.shot_running = true;
            }
            ClockCommand::Pause => self = self.paused(),
            ClockCommand::ToggleShot => {
                if !self.running || self.shot_ms == 0 {
                    return Err(Error::Validation(
                        "Start the game clock and reset an expired shot clock first.".into(),
                    ));
                }
                self.shot_running = !self.shot_running;
            }
            ClockCommand::FullShot => self.shot_ms = 24_000,
            ClockCommand::ShortShot => self.shot_ms = 14_000,
            ClockCommand::ResetGame => {
                self = Self {
                    period: self.period,
                    ..Self::default()
                };
            }
            ClockCommand::NextPeriod => {
                self = Self {
                    period: self.period + 1,
                    ..Self::default()
                };
            }
            ClockCommand::SetGame(ms) => {
                self.game_ms = ms;
                self = self.paused();
            }
            ClockCommand::SetShot(ms) => {
                self.shot_ms = ms;
                self = self.paused();
            }
        }
        self.validate()?;
        Ok(self)
    }
    pub fn for_stat(mut self, stat: Stat) -> Self {
        match stat {
            Stat::TwoMade
            | Stat::ThreeMade
            | Stat::DefensiveRebound
            | Stat::Turnover
            | Stat::Steal => self.shot_ms = 24_000,
            Stat::OffensiveRebound => self.shot_ms = 14_000,
            Stat::Foul | Stat::Technical | Stat::Unsportsmanlike | Stat::Disqualifying => {
                self = self.paused()
            }
            _ => {}
        }
        self
    }
}
pub fn format_game(ms: u64) -> String {
    let s = ms.div_ceil(1000);
    format!("{:02}:{:02}", s / 60, s % 60)
}
pub fn format_shot(ms: u64) -> String {
    if ms <= 5000 {
        let tenths = ms.div_ceil(100);
        format!("{}.{:01}", tenths / 10, tenths % 10)
    } else {
        ms.div_ceil(1000).to_string()
    }
}
pub fn parse_game(value: &str) -> Result<u64> {
    let error =
        || Error::Validation("Enter game time as MM:SS, with seconds from 00 to 59.".into());
    let (m, s) = value.trim().split_once(':').ok_or_else(error)?;
    let m = m.parse::<u64>().map_err(|_| error())?;
    let s = s.parse::<u64>().map_err(|_| error())?;
    if s > 59 || m > 1440 || (m == 1440 && s > 0) {
        return Err(error());
    }
    Ok((m * 60 + s) * 1000)
}
