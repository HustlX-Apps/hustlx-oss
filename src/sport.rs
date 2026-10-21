//! Pure local scoring. An ordered command log can replay every score and service transition.
//! Presets mirror the existing HustlX consoles; they are editable house rules, not certification.

use crate::Side;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Sport {
    Basketball,
    Volleyball,
    Pickleball,
    TableTennis,
    Tennis,
    Badminton,
    Padel,
    Football,
    Futsal,
}
impl fmt::Display for Sport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}
impl Sport {
    pub const ALL: [Self; 9] = [
        Self::Basketball,
        Self::Volleyball,
        Self::Pickleball,
        Self::TableTennis,
        Self::Tennis,
        Self::Badminton,
        Self::Padel,
        Self::Football,
        Self::Futsal,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Basketball => "Basketball",
            Self::Volleyball => "Volleyball",
            Self::Pickleball => "Pickleball",
            Self::TableTennis => "Table tennis",
            Self::Tennis => "Tennis",
            Self::Badminton => "Badminton",
            Self::Padel => "Padel",
            Self::Football => "Football",
            Self::Futsal => "Futsal",
        }
    }
    pub fn slug(self) -> &'static str {
        match self {
            Self::Basketball => "basketball",
            Self::Volleyball => "volleyball",
            Self::Pickleball => "pickleball",
            Self::TableTennis => "table-tennis",
            Self::Tennis => "tennis",
            Self::Badminton => "badminton",
            Self::Padel => "padel",
            Self::Football => "football",
            Self::Futsal => "futsal",
        }
    }
    fn tennis(self) -> bool {
        matches!(self, Self::Tennis | Self::Padel)
    }
    pub fn field(self) -> bool {
        matches!(self, Self::Football | Self::Futsal)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Format {
    Singles,
    Doubles,
}
impl fmt::Display for Format {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Singles => "Singles",
            Self::Doubles => "Doubles",
        })
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scoring {
    Rally,
    SideOut,
    Advantage,
    NoAd,
    StarPoint,
}
impl fmt::Display for Scoring {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Rally => "Rally points",
            Self::SideOut => "Side-out",
            Self::Advantage => "Advantage",
            Self::NoAd => "No-ad / golden point",
            Self::StarPoint => "Star point",
        })
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rules {
    pub best_of: u8,
    /// Points to win a rally-scored unit; games to win a tennis/padel set.
    pub target: u16,
    /// Points for a deciding rally-scored unit, or a deciding match tiebreak.
    pub decider_target: u16,
    pub win_by: u16,
    pub cap: Option<u16>,
    pub format: Format,
    pub scoring: Scoring,
    pub tiebreak_target: u16,
    pub match_tiebreak: bool,
    pub first_server: Side,
    #[serde(default)]
    pub period_minutes: u16,
}
impl Rules {
    pub fn defaults(sport: Sport) -> Self {
        let mut r = Self {
            best_of: 3,
            target: 11,
            decider_target: 11,
            win_by: 2,
            cap: None,
            format: Format::Singles,
            scoring: Scoring::Rally,
            tiebreak_target: 7,
            match_tiebreak: false,
            first_server: Side::Home,
            period_minutes: if sport == Sport::Football {
                45
            } else if sport == Sport::Futsal {
                20
            } else {
                0
            },
        };
        match sport {
            Sport::Volleyball => {
                r.best_of = 5;
                r.target = 25;
                r.decider_target = 15;
                r.format = Format::Doubles;
            }
            Sport::Pickleball => {
                r.format = Format::Doubles;
                r.scoring = Scoring::SideOut;
            }
            Sport::TableTennis => {
                r.best_of = 5;
            }
            Sport::Badminton => {
                r.target = 21;
                r.decider_target = 21;
                r.cap = Some(30);
                r.format = Format::Doubles;
            }
            Sport::Tennis | Sport::Padel => {
                r.target = 6;
                r.decider_target = 10;
                r.scoring = Scoring::Advantage;
                if sport == Sport::Padel {
                    r.format = Format::Doubles;
                }
            }
            Sport::Basketball | Sport::Football | Sport::Futsal => {}
        }
        r
    }
    fn validate(&self, sport: Sport) -> Result<(), ScoreError> {
        if sport == Sport::Basketball {
            return Err(ScoreError("Basketball uses the existing Game Day console"));
        }
        if sport.field() {
            return if (1..=180).contains(&self.period_minutes) {
                Ok(())
            } else {
                Err(ScoreError("Period length must be 1 to 180 minutes"))
            };
        }
        if self.best_of == 0 || self.best_of.is_multiple_of(2) {
            return Err(ScoreError("Best of must be a positive odd number"));
        }
        if self.target == 0
            || self.decider_target == 0
            || self.win_by == 0
            || self.tiebreak_target == 0
        {
            return Err(ScoreError(
                "Scoring targets and winning margin must be positive",
            ));
        }
        if self
            .cap
            .is_some_and(|cap| cap < self.target || (!sport.tennis() && cap < self.decider_target))
        {
            return Err(ScoreError("Point cap must reach each game target"));
        }
        let valid = match sport {
            Sport::Tennis => matches!(self.scoring, Scoring::Advantage | Scoring::NoAd),
            Sport::Padel => matches!(
                self.scoring,
                Scoring::Advantage | Scoring::NoAd | Scoring::StarPoint
            ),
            Sport::Pickleball => matches!(self.scoring, Scoring::Rally | Scoring::SideOut),
            _ => self.scoring == Scoring::Rally,
        };
        if !valid {
            return Err(ScoreError("Scoring mode is not supported by this sport"));
        }
        if sport.tennis() && (self.cap.is_some() || self.win_by != 2) {
            return Err(ScoreError(
                "Tennis and padel use a two-point margin without a point cap",
            ));
        }
        if !sport.tennis() && self.match_tiebreak {
            return Err(ScoreError("Match tiebreak is a tennis or padel option"));
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Live,
    Decided,
    Completed,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Command {
    Field(crate::field_sport::FieldCommand),
    FieldAt {
        clock_ms: u64,
        running: bool,
        command: crate::field_sport::FieldCommand,
    },
    Rally(Side),
    SetServer(Side),
    Complete,
    Player {
        side: Side,
        player: Option<i64>,
        action: crate::operator::PlayerAction,
    },
    Lineup {
        side: Side,
        players: [i64; 11],
    },
    Exchange {
        side: Side,
        outgoing: i64,
        incoming: i64,
        libero: bool,
    },
    Rotate(Side),
    Timeout(Side),
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnitScore {
    pub number: u16,
    pub score: [u16; 2],
    pub tiebreak_points: Option<[u16; 2]>,
    pub winner: Side,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct State {
    pub field: Option<crate::field_sport::FieldState>,
    pub operator: crate::operator::OperatorState,
    pub sport: Sport,
    rules: Rules,
    pub status: Status,
    pub winner: Option<Side>,
    pub points: [u16; 2],
    pub games: [u16; 2],
    pub units: [u16; 2],
    pub unit_number: u16,
    pub server: Side,
    pub server_number: u8,
    pub server_slot: u8,
    pub receiver_slot: u8,
    pub tiebreak: bool,
    pub ends_changed: u16,
    pub completed_units: Vec<UnitScore>,
    // Stable player slots survive score replay independently of account identity.
    service_order: Vec<u8>,
    service_index: usize,
    unit_first_server: Side,
    service_anchor: u32,
    deuce_count: u32,
    decider_ends_changed: bool,
    match_tiebreak_active: bool,
    courts: [[u8; 2]; 2],
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScoreError(pub &'static str);
impl fmt::Display for ScoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}
impl std::error::Error for ScoreError {}
fn idx(side: Side) -> usize {
    if side == Side::Home { 0 } else { 1 }
}
fn other(side: Side) -> Side {
    if side == Side::Home {
        Side::Away
    } else {
        Side::Home
    }
}
fn add(value: &mut u16) -> Result<(), ScoreError> {
    *value = value
        .checked_add(1)
        .ok_or(ScoreError("Score limit reached"))?;
    Ok(())
}
impl State {
    pub fn new(sport: Sport, rules: Rules) -> Result<Self, ScoreError> {
        rules.validate(sport)?;
        let service_order = if rules.format == Format::Doubles {
            if rules.first_server == Side::Home {
                vec![0, 1, 2, 3]
            } else {
                vec![1, 0, 3, 2]
            }
        } else if rules.first_server == Side::Home {
            vec![0, 1]
        } else {
            vec![1, 0]
        };
        let opening_number = if sport == Sport::Pickleball
            && rules.scoring == Scoring::SideOut
            && rules.format == Format::Doubles
        {
            2
        } else {
            1
        };
        let first = rules.first_server;
        let match_tb = sport.tennis() && rules.match_tiebreak && rules.best_of == 1;
        Ok(Self {
            field: if sport.field() {
                Some(
                    crate::field_sport::FieldState::new(
                        sport == Sport::Futsal,
                        u64::from(rules.period_minutes) * 60_000,
                    )
                    .map_err(|e| ScoreError(e.0))?,
                )
            } else {
                None
            },
            operator: crate::operator::OperatorState::default(),
            sport,
            rules,
            status: Status::Live,
            winner: None,
            points: [0, 0],
            games: [0, 0],
            units: [0, 0],
            unit_number: 1,
            server: first,
            server_number: opening_number,
            server_slot: 1,
            receiver_slot: 1,
            tiebreak: match_tb,
            ends_changed: 0,
            completed_units: vec![],
            service_order,
            service_index: 0,
            unit_first_server: first,
            service_anchor: 0,
            deuce_count: 0,
            decider_ends_changed: false,
            match_tiebreak_active: match_tb,
            courts: [[1, 2], [1, 2]],
        })
    }
    pub fn rules(&self) -> &Rules {
        &self.rules
    }
    /// Errors leave state unchanged. Storage can persist the command before replacing its projection.
    pub fn apply(&mut self, command: Command) -> Result<(), ScoreError> {
        let mut next = self.clone();
        next.apply_inner(command)?;
        *self = next;
        Ok(())
    }
    fn apply_inner(&mut self, command: Command) -> Result<(), ScoreError> {
        if let Command::FieldAt {
            clock_ms,
            running,
            command,
        } = command
        {
            self.apply_inner(Command::Field(
                crate::field_sport::FieldCommand::Checkpoint { clock_ms, running },
            ))?;
            return self.apply_inner(Command::Field(command));
        }
        if let Command::Field(command) = command {
            let field = self.field.as_mut().ok_or(ScoreError(
                "Field controls apply only to football and futsal",
            ))?;
            field.apply(command).map_err(|e| ScoreError(e.0))?;
            self.points = field.score;
            self.unit_number = u16::from(field.period);
            self.winner = if field.completed {
                field.winner()
            } else {
                None
            };
            self.status = if field.completed {
                Status::Completed
            } else {
                Status::Live
            };
            return Ok(());
        }
        if self.sport.field()
            && !matches!(
                command,
                Command::Lineup { .. }
                    | Command::Exchange { libero: false, .. }
                    | Command::Timeout(_)
            )
        {
            return Err(ScoreError("Use football or futsal controls for this match"));
        }
        if command == Command::Complete {
            if self.status != Status::Decided {
                return Err(ScoreError("Finish the match before completing it"));
            }
            self.status = Status::Completed;
            return Ok(());
        }
        if self.status != Status::Live {
            return Err(ScoreError(
                "This match is decided; undo before changing its score",
            ));
        }
        match command {
            Command::Field(_) | Command::FieldAt { .. } => unreachable!(),
            Command::Player {
                side,
                player,
                action,
            } => {
                if !crate::operator::PlayerAction::for_sport(self.sport).contains(&action) {
                    return Err(ScoreError("This action does not apply to this sport"));
                }
                self.operator.record(side, player, action)?;
                if let Some(winner) = action.winner(side) {
                    self.apply_inner(Command::Rally(winner))?;
                }
                Ok(())
            }
            Command::Lineup { side, players } => self.operator.lineup(side, players),
            Command::Exchange {
                side,
                outgoing,
                incoming,
                libero,
            } => {
                if libero && self.sport != Sport::Volleyball {
                    return Err(ScoreError("Libero exchanges apply only to volleyball"));
                }
                self.operator.exchange(side, outgoing, incoming, libero)
            }
            Command::Rotate(side) => {
                if self.sport != Sport::Volleyball {
                    return Err(ScoreError("Rotation applies only to volleyball"));
                }
                self.operator.rotate(side);
                Ok(())
            }
            Command::Timeout(side) => self.operator.timeout(side),
            Command::SetServer(side) => {
                self.server = side;
                self.unit_first_server = side;
                self.service_anchor = if self.sport == Sport::TableTennis {
                    self.table_service_turn()
                } else {
                    self.total_points()
                };
                if let Some(i) = self
                    .service_order
                    .iter()
                    .position(|slot| ((*slot).is_multiple_of(2)) == (side == Side::Home))
                {
                    self.service_index = i;
                }
                self.server_number = if self.sport == Sport::Pickleball
                    && self.points == [0, 0]
                    && self.rules.format == Format::Doubles
                    && self.rules.scoring == Scoring::SideOut
                {
                    2
                } else {
                    1
                };
                self.server_slot = 1;
                self.receiver_slot = 1;
                self.refresh_service();
                Ok(())
            }
            Command::Rally(side) => {
                if self.sport.tennis() {
                    self.tennis_point(side)
                } else {
                    self.rally_point(side)
                }
            }
            Command::Complete => unreachable!(),
        }
    }
    pub fn point_label(&self, side: Side) -> String {
        let p = self.points[idx(side)];
        let o = self.points[idx(other(side))];
        if !self.sport.tennis() || self.tiebreak {
            return p.to_string();
        }
        if p >= 3 && o >= 3 {
            return if p > o { "AD" } else { "40" }.to_owned();
        }
        ["0", "15", "30", "40"][usize::from(p.min(3))].to_owned()
    }
    fn total_points(&self) -> u32 {
        u32::from(self.points[0]) + u32::from(self.points[1])
    }
    fn decider(&self) -> bool {
        self.unit_number == u16::from(self.rules.best_of)
    }
    fn rally_point(&mut self, side: Side) -> Result<(), ScoreError> {
        let old_server = self.server;
        if self.sport == Sport::Pickleball
            && self.rules.scoring == Scoring::SideOut
            && side != old_server
        {
            if self.rules.format == Format::Doubles && self.server_number == 1 {
                self.server_number = 2;
                self.server_slot = 3 - self.server_slot;
            } else {
                self.server = side;
                self.server_number = 1;
                self.server_slot = self.courts[idx(side)][0];
            }
            self.receiver_slot = self.courts[idx(other(self.server))]
                [usize::from(self.points[idx(self.server)] % 2)];
            return Ok(());
        }
        add(&mut self.points[idx(side)])?;
        if matches!(self.sport, Sport::Badminton | Sport::Pickleball) {
            if old_server == side && self.rules.format == Format::Doubles {
                self.courts[idx(side)].swap(0, 1);
            }
            self.server = side;
            let court = usize::from(self.points[idx(side)] % 2);
            self.server_slot = if self.rules.format == Format::Singles {
                1
            } else if self.sport == Sport::Pickleball && old_server == side {
                self.server_slot
            } else {
                self.courts[idx(side)][court]
            };
            let receiving_court = if self.sport == Sport::Pickleball {
                self.courts[idx(side)]
                    .iter()
                    .position(|slot| *slot == self.server_slot)
                    .unwrap_or(0)
            } else {
                court
            };
            self.receiver_slot = if self.rules.format == Format::Singles {
                1
            } else {
                self.courts[idx(other(side))][receiving_court]
            };
        } else if self.sport == Sport::Volleyball {
            self.server = side;
            if old_server != side {
                self.operator.rotate(side);
            }
        }
        let threshold = match self.sport {
            Sport::TableTennis => Some(5),
            Sport::Badminton => Some(self.rules.target.div_ceil(2)),
            Sport::Volleyball => Some(self.rules.decider_target.div_ceil(2)),
            _ => None,
        };
        if self.decider()
            && !self.decider_ends_changed
            && threshold.is_some_and(|n| self.points.iter().any(|p| *p >= n))
        {
            add(&mut self.ends_changed)?;
            self.decider_ends_changed = true;
            if self.sport == Sport::TableTennis && self.service_order.len() == 4 {
                self.refresh_service();
                let receiving = other(self.server);
                let slots: Vec<usize> = self
                    .service_order
                    .iter()
                    .enumerate()
                    .filter_map(|(i, slot)| {
                        if ((*slot).is_multiple_of(2)) == (receiving == Side::Home) {
                            Some(i)
                        } else {
                            None
                        }
                    })
                    .collect();
                self.service_order.swap(slots[0], slots[1]);
            }
        }
        let target = if self.decider() {
            self.rules.decider_target
        } else {
            self.rules.target
        };
        let lead = self.points[idx(side)];
        let trail = self.points[idx(other(side))];
        let won = lead > trail
            && ((lead >= target && lead - trail >= self.rules.win_by)
                || self.rules.cap.is_some_and(|cap| lead >= cap));
        if won {
            self.finish_unit(side, self.points, None)?;
        } else {
            self.refresh_service();
        }
        Ok(())
    }
    fn tennis_point(&mut self, side: Side) -> Result<(), ScoreError> {
        let deciding = self.deuce_count > 0
            && (self.rules.scoring == Scoring::NoAd
                || (self.rules.scoring == Scoring::StarPoint && self.deuce_count >= 3));
        add(&mut self.points[idx(side)])?;
        let high = self.points[idx(side)];
        let low = self.points[idx(other(side))];
        if self.tiebreak {
            let switched = self.total_points().is_multiple_of(6);
            if switched {
                add(&mut self.ends_changed)?;
            }
            let target = if self.match_tiebreak_active {
                self.rules.decider_target
            } else {
                self.rules.tiebreak_target
            };
            if high >= target && high.saturating_sub(low) >= 2 {
                if !self.match_tiebreak_active {
                    add(&mut self.games[idx(side)])?;
                    if !switched {
                        add(&mut self.ends_changed)?;
                    }
                }
                self.service_index = (self.service_index + 1) % self.service_order.len();
                self.finish_unit(side, self.games, Some(self.points))?;
            } else {
                self.refresh_service();
            }
            return Ok(());
        }
        if deciding || (high >= 4 && high.saturating_sub(low) >= 2) {
            add(&mut self.games[idx(side)])?;
            self.service_index = (self.service_index + 1) % self.service_order.len();
            if (u32::from(self.games[0]) + u32::from(self.games[1])) % 2 == 1 {
                add(&mut self.ends_changed)?;
            }
            if self.games[idx(side)] >= self.rules.target
                && self.games[idx(side)].saturating_sub(self.games[idx(other(side))]) >= 2
            {
                self.finish_unit(side, self.games, None)?;
            } else {
                self.points = [0, 0];
                self.service_anchor = 0;
                self.deuce_count = 0;
                self.tiebreak = self.games == [self.rules.target, self.rules.target];
                self.refresh_service();
            }
        } else {
            if high >= 3 && high == low {
                self.deuce_count += 1;
            }
            self.refresh_service();
        }
        Ok(())
    }
    fn finish_unit(
        &mut self,
        side: Side,
        score: [u16; 2],
        tb: Option<[u16; 2]>,
    ) -> Result<(), ScoreError> {
        self.completed_units.push(UnitScore {
            number: self.unit_number,
            score,
            tiebreak_points: tb,
            winner: side,
        });
        add(&mut self.units[idx(side)])?;
        if self.units[idx(side)] >= u16::from(self.rules.best_of / 2 + 1) {
            self.winner = Some(side);
            self.status = Status::Decided;
            return Ok(());
        }
        add(&mut self.unit_number)?;
        self.operator.begin_unit();
        self.points = [0, 0];
        self.games = [0, 0];
        self.deuce_count = 0;
        self.service_anchor = 0;
        self.decider_ends_changed = false;
        self.match_tiebreak_active =
            self.sport.tennis() && self.rules.match_tiebreak && self.decider();
        self.tiebreak = self.match_tiebreak_active;
        match self.sport {
            Sport::TableTennis => {
                self.service_order = if self.service_order.len() == 4 {
                    vec![
                        self.service_order[1],
                        self.service_order[0],
                        self.service_order[3],
                        self.service_order[2],
                    ]
                } else {
                    vec![self.service_order[1], self.service_order[0]]
                };
                self.service_index = 0;
                add(&mut self.ends_changed)?;
            }
            Sport::Badminton => {
                self.server = side;
                self.unit_first_server = side;
                self.server_slot = 1;
                self.receiver_slot = 1;
                self.courts = [[1, 2], [1, 2]];
                add(&mut self.ends_changed)?;
            }
            Sport::Pickleball => {
                self.server = self.rules.first_server;
                self.server_number = if self.rules.scoring == Scoring::SideOut
                    && self.rules.format == Format::Doubles
                {
                    2
                } else {
                    1
                };
                self.server_slot = 1;
                self.receiver_slot = 1;
                self.courts = [[1, 2], [1, 2]];
                add(&mut self.ends_changed)?;
            }
            Sport::Volleyball => {
                self.unit_first_server = other(self.unit_first_server);
                self.server = self.unit_first_server;
                add(&mut self.ends_changed)?;
            }
            _ => {}
        }
        self.refresh_service();
        Ok(())
    }
    fn table_service_turn(&self) -> u32 {
        let target = if self.decider() {
            self.rules.decider_target
        } else {
            self.rules.target
        };
        let threshold = u32::from(target - 1);
        let total = self.total_points();
        if self.points.iter().all(|p| u32::from(*p) >= threshold) {
            threshold + (total - threshold * 2)
        } else {
            total / 2
        }
    }
    fn refresh_service(&mut self) {
        if !(self.sport.tennis() || self.sport == Sport::TableTennis) {
            return;
        }
        let played = self.total_points().saturating_sub(self.service_anchor);
        let turn = if self.sport == Sport::TableTennis {
            self.table_service_turn()
                .saturating_sub(self.service_anchor)
        } else if self.tiebreak {
            played.div_ceil(2)
        } else {
            0
        };
        let position = (self.service_index + turn as usize) % self.service_order.len();
        let slot = self.service_order[position];
        self.server = if slot.is_multiple_of(2) {
            Side::Home
        } else {
            Side::Away
        };
        self.server_slot = slot / 2 + 1;
        let receiving = self.service_order[(position + 1) % self.service_order.len()];
        self.receiver_slot = receiving / 2 + 1;
    }
}
