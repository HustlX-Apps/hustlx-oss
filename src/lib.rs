//! Renderer-independent basketball state. SQLite owns durable local matches.

pub mod clock;
pub mod hotkeys;
pub mod lineup;
pub mod profile;
pub mod session;
pub mod sport;
pub mod storage;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    Home,
    Away,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stat {
    FreeThrowMade,
    TwoMade,
    ThreeMade,
    FreeThrowMiss,
    TwoMiss,
    ThreeMiss,
    OffensiveRebound,
    DefensiveRebound,
    Assist,
    Turnover,
    Steal,
    Block,
    Foul,
    Technical,
    Unsportsmanlike,
    Disqualifying,
    Warning,
}

impl Stat {
    pub fn points(self) -> u32 {
        match self {
            Self::FreeThrowMade => 1,
            Self::TwoMade => 2,
            Self::ThreeMade => 3,
            _ => 0,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::FreeThrowMade => "FT made",
            Self::TwoMade => "2PT made",
            Self::ThreeMade => "3PT made",
            Self::FreeThrowMiss => "FT miss",
            Self::TwoMiss => "2PT miss",
            Self::ThreeMiss => "3PT miss",
            Self::OffensiveRebound => "Off reb",
            Self::DefensiveRebound => "Def reb",
            Self::Assist => "Assist",
            Self::Turnover => "Turnover",
            Self::Steal => "Steal",
            Self::Block => "Block",
            Self::Foul => "Foul",
            Self::Technical => "Technical",
            Self::Unsportsmanlike => "Unsportsmanlike",
            Self::Disqualifying => "Disqualifying",
            Self::Warning => "Warning",
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Stats {
    pub points: u32,
    pub free_throws_made: u32,
    pub free_throws_attempted: u32,
    pub field_goals_made: u32,
    pub field_goals_attempted: u32,
    pub threes_made: u32,
    pub threes_attempted: u32,
    pub offensive_rebounds: u32,
    pub defensive_rebounds: u32,
    pub assists: u32,
    pub turnovers: u32,
    pub steals: u32,
    pub blocks: u32,
    pub fouls: u32,
    pub technicals: u32,
    pub unsportsmanlikes: u32,
    pub disqualifications: u32,
    pub warnings: u32,
}

#[derive(Debug, Clone)]
pub struct Player {
    pub id: i64,
    pub name: String,
    pub jersey: String,
    pub positions: Vec<profile::Position>,
    pub on_court: bool,
}

#[derive(Debug, Clone)]
pub struct Team {
    pub name: String,
    pub players: Vec<Player>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Play {
    pub side: Side,
    pub player_id: i64,
    pub stat: Stat,
    /// Clock at entry, before stat effects. None means timing was never recorded.
    pub clock: Option<clock::ClockState>,
}

impl Play {
    pub fn clock_label(&self) -> String {
        match self.clock {
            Some(stamp) => format!(
                "P{} {} / SHOT {}",
                stamp.period,
                clock::format_game(stamp.game_ms),
                clock::format_shot(stamp.shot_ms)
            ),
            None => "Time not recorded".into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Game {
    pub selected: Option<(Side, i64)>,
    teams: [Team; 2],
    plays: Vec<Play>,
}

impl Default for Game {
    fn default() -> Self {
        fn team(name: &str, first_id: i64, roster: &[(&str, u8)]) -> Team {
            Team {
                name: name.to_owned(),
                players: roster
                    .iter()
                    .enumerate()
                    .map(|(index, (name, jersey))| Player {
                        id: first_id + index as i64,
                        name: (*name).to_owned(),
                        jersey: jersey.to_string(),
                        positions: vec![],
                        on_court: index < 5,
                    })
                    .collect(),
            }
        }

        Self {
            selected: None,
            teams: [
                team(
                    "Batangas Blazers",
                    1,
                    &[
                        ("Miguel Santos", 4),
                        ("Bryce Mcgowens", 6),
                        ("Ken Dela Cruz", 9),
                        ("Paolo Cruz", 11),
                        ("Jerome Reyes", 23),
                        ("Aldrin Bautista", 15),
                        ("Alex Rivera", 18),
                    ],
                ),
                team(
                    "Lipa City Hawks",
                    11,
                    &[
                        ("Rafael Mendoza", 3),
                        ("Carlo Villanueva", 7),
                        ("Joshua Ramos", 12),
                        ("Mark Aquino", 21),
                        ("Nico Garcia", 30),
                        ("Jayson Lim", 8),
                    ],
                ),
            ],
            plays: Vec::new(),
        }
    }
}

impl Game {
    pub(crate) fn from_saved(teams: [Team; 2], plays: Vec<Play>) -> Self {
        Self {
            selected: None,
            teams,
            plays,
        }
    }

    pub fn team(&self, side: Side) -> &Team {
        &self.teams[match side {
            Side::Home => 0,
            Side::Away => 1,
        }]
    }

    pub fn court_ids(&self, side: Side) -> Vec<i64> {
        self.team(side)
            .players
            .iter()
            .filter(|p| p.on_court)
            .map(|p| p.id)
            .collect()
    }

    pub fn player(&self, side: Side, id: i64) -> Option<&Player> {
        self.team(side)
            .players
            .iter()
            .find(|player| player.id == id)
    }

    pub fn player_stats(&self, side: Side, id: i64) -> Stats {
        let mut stats = Stats::default();
        for play in self
            .plays
            .iter()
            .filter(|play| play.side == side && play.player_id == id)
        {
            stats.points += play.stat.points();
            match play.stat {
                Stat::FreeThrowMade => {
                    stats.free_throws_made += 1;
                    stats.free_throws_attempted += 1;
                }
                Stat::FreeThrowMiss => stats.free_throws_attempted += 1,
                Stat::TwoMade | Stat::ThreeMade => {
                    stats.field_goals_made += 1;
                    stats.field_goals_attempted += 1;
                    if play.stat == Stat::ThreeMade {
                        stats.threes_made += 1;
                        stats.threes_attempted += 1;
                    }
                }
                Stat::TwoMiss => stats.field_goals_attempted += 1,
                Stat::ThreeMiss => {
                    stats.field_goals_attempted += 1;
                    stats.threes_attempted += 1;
                }
                Stat::OffensiveRebound => stats.offensive_rebounds += 1,
                Stat::DefensiveRebound => stats.defensive_rebounds += 1,
                Stat::Assist => stats.assists += 1,
                Stat::Turnover => stats.turnovers += 1,
                Stat::Steal => stats.steals += 1,
                Stat::Block => stats.blocks += 1,
                Stat::Foul => stats.fouls += 1,
                Stat::Technical => stats.technicals += 1,
                Stat::Unsportsmanlike => stats.unsportsmanlikes += 1,
                Stat::Disqualifying => stats.disqualifications += 1,
                Stat::Warning => stats.warnings += 1,
            }
        }
        stats
    }

    pub fn score(&self, side: Side) -> u32 {
        self.plays
            .iter()
            .filter(|play| play.side == side)
            .map(|play| play.stat.points())
            .sum()
    }

    pub fn select(&mut self, side: Side, id: i64) {
        self.selected = if self.player(side, id).is_some() && self.selected != Some((side, id)) {
            Some((side, id))
        } else {
            None
        };
    }

    pub fn record(&mut self, stat: Stat) -> bool {
        let Some((side, player_id)) = self.selected else {
            return false;
        };
        if self.player(side, player_id).is_none() {
            return false;
        }
        self.plays.push(Play {
            side,
            player_id,
            stat,
            clock: None,
        });
        self.selected = None;
        true
    }

    pub fn undo(&mut self) -> bool {
        self.selected = None;
        self.plays.pop().is_some()
    }

    pub fn plays(&self) -> &[Play] {
        &self.plays
    }
}
