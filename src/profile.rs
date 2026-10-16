use std::fmt;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Position {
    PointGuard,
    ShootingGuard,
    SmallForward,
    PowerForward,
    Center,
}
impl Position {
    pub const ALL: [Self; 5] = [
        Self::PointGuard,
        Self::ShootingGuard,
        Self::SmallForward,
        Self::PowerForward,
        Self::Center,
    ];
    pub fn code(self) -> &'static str {
        match self {
            Self::PointGuard => "PG",
            Self::ShootingGuard => "SG",
            Self::SmallForward => "SF",
            Self::PowerForward => "PF",
            Self::Center => "C",
        }
    }
}
impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}
pub fn encode(positions: &[Position]) -> String {
    Position::ALL
        .into_iter()
        .filter(|p| positions.contains(p))
        .map(Position::code)
        .collect::<Vec<_>>()
        .join(",")
}
pub fn decode(value: &str) -> Vec<Position> {
    Position::ALL
        .into_iter()
        .filter(|p| value.split(',').any(|v| v == p.code()))
        .collect()
}
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct PlayerDetails {
    pub jersey: String,
    pub positions: Vec<Position>,
}
