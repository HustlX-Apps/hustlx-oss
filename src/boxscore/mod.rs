//! Sport-agnostic printable box score model. Pure data: no rendering.
use crate::{sport::Sport, storage::SavedMatch};

pub mod basketball;
pub mod fixtures;
pub mod minutes;
pub mod periods;

pub const DESCRIPTOR: &str = "Reference-grade box score";

#[derive(Debug, Clone)]
pub struct Organizer {
    pub name: String,
    pub logo_png: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Left,
    Right,
    Center,
}

#[derive(Debug, Clone, Copy)]
pub struct Column {
    pub key: &'static str,
    pub label: &'static str,
    pub align: Align,
    pub weight: u8,
}

#[derive(Debug, Clone)]
pub struct Row {
    pub cells: Vec<String>,
    pub emphasis: bool,
}

#[derive(Debug, Clone)]
pub struct TeamBlock {
    pub name: String,
    pub side_letter: char,
    pub columns: Vec<Column>,
    pub rows: Vec<Row>,
    pub totals: Row,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct LineRow {
    pub name: String,
    pub cells: Vec<String>,
    pub total: String,
}

#[derive(Debug, Clone)]
pub struct LineScore {
    pub title: String,
    pub columns: Vec<String>,
    pub rows: [LineRow; 2],
}

#[derive(Debug, Clone)]
pub struct Block {
    pub title: String,
    pub lines: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Identity {
    pub organizer_name: Option<String>,
    pub organizer_logo: Option<Vec<u8>>,
    pub sport_label: String,
    pub descriptor: &'static str,
}

#[derive(Debug, Clone)]
pub struct Header {
    pub match_name: String,
    pub date: String,
    pub venue: Option<String>,
    pub home_name: String,
    pub away_name: String,
    pub home_score: String,
    pub away_score: String,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct Footer {
    pub center: String,
    pub right: String,
}

#[derive(Debug, Clone)]
pub struct Sheet {
    pub identity: Identity,
    pub header: Header,
    pub line_score: Option<LineScore>,
    pub teams: [TeamBlock; 2],
    pub extra: Option<Block>,
    pub legend: Option<String>,
    pub footer: Footer,
}

// One short-lived value per print; boxing would change the public shape.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone)]
pub enum SheetResult {
    Ready(Sheet),
    Unsupported { sport_label: String },
}

pub fn build(
    saved: &SavedMatch,
    organizer: Option<&Organizer>,
    generated_on: &str,
    version: &str,
) -> SheetResult {
    match saved.summary.sport {
        Sport::Basketball => {
            SheetResult::Ready(basketball::sheet(saved, organizer, generated_on, version))
        }
        other => SheetResult::Unsupported {
            sport_label: other.label().to_string(),
        },
    }
}
