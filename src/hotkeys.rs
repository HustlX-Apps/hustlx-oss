//! Supported v2 keyboard vocabulary; native event guards are explicit inputs.
use crate::{Game, Side, Stat, clock::ClockCommand};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    ToggleClock,
    Clock(ClockCommand),
    Stat(Stat),
    Undo,
    Clear,
    Help,
    Substitutions,
}
pub struct Input<'a> {
    pub key: &'a str,
    pub enabled: bool,
    pub repeat: bool,
    pub modified: bool,
    pub captured: bool,
}
#[derive(Debug, Default)]
pub struct Hotkeys {
    digits: String,
    side: Option<Side>,
}
impl Hotkeys {
    pub fn digits(&self) -> &str {
        &self.digits
    }
    pub fn side(&self) -> Option<Side> {
        self.side
    }
    pub fn clear(&mut self) {
        self.digits.clear();
        self.side = None;
    }
    pub fn target(&self, game: &Game) -> Result<(Side, i64), String> {
        if self.digits.is_empty() {
            return game
                .selected
                .filter(|(side, _)| self.side.is_none_or(|hint| hint == *side))
                .ok_or_else(|| "Select a player or type their jersey first.".into());
        }
        let mut matches = Vec::new();
        for side in [Side::Home, Side::Away] {
            if self.side.is_some_and(|hint| hint != side) {
                continue;
            }
            for p in &game.team(side).players {
                if p.jersey == self.digits {
                    matches.push((side, p.id));
                }
            }
        }
        match matches.as_slice() {
   [one]=>Ok(*one),[]=>Err(format!("No matching player wears #{}. Select a player directly.",self.digits)),
   _=>Err("That jersey is ambiguous. Use , for home or . for away, or select the player directly.".into()),
  }
    }
    pub fn press(&mut self, input: Input<'_>) -> Option<Action> {
        if !input.enabled || input.repeat || input.modified || input.captured {
            return None;
        }
        let key = input.key;
        if key.len() == 1 && key.as_bytes()[0].is_ascii_digit() {
            self.digits.push_str(key);
            if self.digits.len() > 2 {
                self.digits.remove(0);
            }
            return None;
        }
        match key {
            "," => {
                self.side = Some(Side::Home);
                return None;
            }
            "." => {
                self.side = Some(Side::Away);
                return None;
            }
            "Escape" => {
                self.clear();
                return Some(Action::Clear);
            }
            "Backspace" => {
                if self.digits.pop().is_some() {
                    return None;
                }
                return Some(Action::Undo);
            }
            " " => return Some(Action::ToggleClock),
            "[" => return Some(Action::Clock(ClockCommand::FullShot)),
            "]" => return Some(Action::Clock(ClockCommand::ShortShot)),
            "?" => return Some(Action::Help),
            _ => {}
        }
        let stat = match key.to_ascii_lowercase().as_str() {
            "q" | "/" => Stat::FreeThrowMade,
            "w" | "+" => Stat::TwoMade,
            "e" | "*" => Stat::ThreeMade,
            "a" => Stat::FreeThrowMiss,
            "s" | "-" => Stat::TwoMiss,
            "d" => Stat::ThreeMiss,
            "r" => Stat::DefensiveRebound,
            "y" => Stat::OffensiveRebound,
            "f" => Stat::Foul,
            "g" => Stat::Assist,
            "t" => Stat::Turnover,
            "v" => Stat::Steal,
            "b" => Stat::Block,
            "x" => return Some(Action::Substitutions),
            "p" => return Some(Action::Clock(ClockCommand::NextPeriod)),
            _ => return None,
        };
        Some(Action::Stat(stat))
    }
}
