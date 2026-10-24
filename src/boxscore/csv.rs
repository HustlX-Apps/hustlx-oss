//! Basketball stats CSV: a summary, the line score, player and team totals, then play-by-play.
//! Mirrors the web Game Day CSV layout. Shooting is split into made and attempted columns so
//! spreadsheets never read "4-9" as a date.
use super::{
    basketball::{add, efficiency, row_order, starters_for, status_line},
    minutes::{self, ParticipationTable},
    periods,
};
use crate::{Side, Stats, clock, storage::SavedMatch};

/// UTF-8 byte order mark, so spreadsheet apps detect the encoding of accented names.
const BOM: &str = "\u{feff}";

const PLAYER_HEADER: [&str; 30] = [
    "Team",
    "Number",
    "Player",
    "Starter",
    "MIN",
    "PTS",
    "FGM",
    "FGA",
    "FG%",
    "2PM",
    "2PA",
    "2P%",
    "3PM",
    "3PA",
    "3P%",
    "FTM",
    "FTA",
    "FT%",
    "OR",
    "DR",
    "TOT",
    "AS",
    "TO",
    "ST",
    "BS",
    "PF",
    "TECH",
    "UNSP",
    "PlusMinus",
    "EFF",
];

const PLAY_HEADER: [&str; 9] = [
    "Sequence",
    "Period",
    "GameClock",
    "ShotClock",
    "Team",
    "Number",
    "Player",
    "Stat",
    "Points",
];

/// Quotes cells with separators or quotes. Text that a spreadsheet would run as a formula gets
/// a leading apostrophe; numbers such as "-3" are left alone.
fn escape(value: &str) -> String {
    let formula =
        value.starts_with(['=', '+', '-', '@', '\t', '\r']) && value.parse::<f64>().is_err();
    let value = if formula {
        format!("'{value}")
    } else {
        value.to_owned()
    };
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value
    }
}

fn push_row<S: AsRef<str>>(out: &mut String, cells: &[S]) {
    let line: Vec<String> = cells.iter().map(|c| escape(c.as_ref())).collect();
    out.push_str(&line.join(","));
    out.push_str("\r\n");
}

/// Whole percent, blank when nothing was attempted.
fn pct(made: u32, attempted: u32) -> String {
    match super::basketball::pct(made, attempted).as_str() {
        "-" => String::new(),
        value => value.to_owned(),
    }
}

fn stat_cells(s: &Stats) -> Vec<String> {
    let p2_made = s.field_goals_made - s.threes_made;
    let p2_att = s.field_goals_attempted - s.threes_attempted;
    vec![
        s.points.to_string(),
        s.field_goals_made.to_string(),
        s.field_goals_attempted.to_string(),
        pct(s.field_goals_made, s.field_goals_attempted),
        p2_made.to_string(),
        p2_att.to_string(),
        pct(p2_made, p2_att),
        s.threes_made.to_string(),
        s.threes_attempted.to_string(),
        pct(s.threes_made, s.threes_attempted),
        s.free_throws_made.to_string(),
        s.free_throws_attempted.to_string(),
        pct(s.free_throws_made, s.free_throws_attempted),
        s.offensive_rebounds.to_string(),
        s.defensive_rebounds.to_string(),
        (s.offensive_rebounds + s.defensive_rebounds).to_string(),
        s.assists.to_string(),
        s.turnovers.to_string(),
        s.steals.to_string(),
        s.blocks.to_string(),
        s.fouls.to_string(),
        s.technicals.to_string(),
        s.unsportsmanlikes.to_string(),
    ]
}

/// One row per rostered player in box score order, then a Totals row. Minutes and plus-minus
/// stay blank when participation is unknown; team plus-minus is always blank.
fn player_rows(
    out: &mut String,
    saved: &SavedMatch,
    side: Side,
    participation: Option<&ParticipationTable>,
) {
    let game = &saved.game;
    let team = game.team(side);
    let starters = starters_for(saved, side);
    let mut total = Stats::default();
    let mut total_seconds = 0u64;
    for id in row_order(&team.players, &starters) {
        let Some(p) = game.player(side, id) else {
            continue;
        };
        let s = game.player_stats(side, id);
        add(&mut total, &s);
        let part = participation.map(|t| t.get(side, id));
        total_seconds += part.map_or(0, |p| p.seconds);
        let mut cells = vec![
            team.name.clone(),
            p.jersey.clone(),
            p.name.clone(),
            if starters.contains(&id) { "Yes" } else { "No" }.to_owned(),
            part.map_or_else(String::new, |p| minutes::format_minutes(p.seconds)),
        ];
        cells.extend(stat_cells(&s));
        cells.push(part.map_or_else(String::new, |p| p.plus_minus.to_string()));
        cells.push(efficiency(&s).to_string());
        push_row(out, &cells);
    }
    let mut cells = vec![
        team.name.clone(),
        String::new(),
        "Totals".to_owned(),
        String::new(),
        participation.map_or_else(String::new, |_| minutes::format_minutes(total_seconds)),
    ];
    cells.extend(stat_cells(&total));
    cells.push(String::new());
    cells.push(efficiency(&total).to_string());
    push_row(out, &cells);
}

/// The full CSV text, starting with a byte order mark. `generated_at` is printed as given.
pub fn basketball(saved: &SavedMatch, generated_at: &str) -> String {
    let summary = &saved.summary;
    let game = &saved.game;
    let plays = game.plays();
    let mut out = String::from(BOM);

    push_row(&mut out, &["Section", "Field", "Value"]);
    let home_score = game.score(Side::Home).to_string();
    let away_score = game.score(Side::Away).to_string();
    let status = status_line(&saved.clock);
    let summary_rows = [
        ("GeneratedAt", generated_at),
        ("Sport", summary.sport.label()),
        ("Match", summary.name.as_str()),
        ("Date", summary.date.as_str()),
        ("Venue", summary.venue.as_str()),
        ("Status", status.as_str()),
        ("HomeTeam", summary.home_name.as_str()),
        ("HomeScore", home_score.as_str()),
        ("AwayTeam", summary.away_name.as_str()),
        ("AwayScore", away_score.as_str()),
    ];
    for (field, value) in summary_rows {
        push_row(&mut out, &["Summary", field, value]);
    }

    if let Some(scores) = periods::period_scores(plays) {
        out.push_str("\r\n");
        let mut header = vec!["Team".to_owned()];
        header.extend(scores.labels.iter().cloned());
        header.push("T".to_owned());
        push_row(&mut out, &header);
        for (side, name, cells) in [
            (Side::Home, &summary.home_name, &scores.home),
            (Side::Away, &summary.away_name, &scores.away),
        ] {
            let mut row = vec![name.clone()];
            row.extend(cells.iter().map(u32::to_string));
            row.push(game.score(side).to_string());
            push_row(&mut out, &row);
        }
    }

    out.push_str("\r\n");
    push_row(&mut out, &PLAYER_HEADER);
    let participation = minutes::participation(saved);
    for side in [Side::Home, Side::Away] {
        player_rows(&mut out, saved, side, participation.as_ref());
    }

    out.push_str("\r\n");
    push_row(&mut out, &PLAY_HEADER);
    for (index, play) in plays.iter().enumerate() {
        let player = game.player(play.side, play.player_id);
        let (period, game_clock, shot_clock) = play.clock.map_or_else(
            || (String::new(), String::new(), String::new()),
            |stamp| {
                (
                    super::basketball::period_label(stamp.period),
                    clock::format_game(stamp.game_ms),
                    clock::format_shot(stamp.shot_ms),
                )
            },
        );
        push_row(
            &mut out,
            &[
                (index + 1).to_string(),
                period,
                game_clock,
                shot_clock,
                game.team(play.side).name.clone(),
                player.map_or_else(String::new, |p| p.jersey.clone()),
                player.map_or_else(String::new, |p| p.name.clone()),
                play.stat.label().to_owned(),
                play.stat.points().to_string(),
            ],
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boxscore::fixtures;

    /// Sections split on blank lines, each a list of rows of cells. Quoting is not handled.
    fn sections(csv: &str) -> Vec<Vec<Vec<String>>> {
        csv.trim_start_matches(BOM)
            .split("\r\n\r\n")
            .map(|section| {
                section
                    .lines()
                    .filter(|l| !l.is_empty())
                    .map(|l| l.split(',').map(str::to_owned).collect())
                    .collect()
            })
            .collect()
    }

    #[test]
    fn escape_quotes_separators_and_neutralises_formulas() {
        assert_eq!(escape("Plain"), "Plain");
        assert_eq!(escape("Smith, Jr."), "\"Smith, Jr.\"");
        assert_eq!(escape("The \"Hawk\""), "\"The \"\"Hawk\"\"\"");
        assert_eq!(escape("=SUM(A1)"), "'=SUM(A1)");
        assert_eq!(escape("@cmd"), "'@cmd");
        assert_eq!(escape("-3"), "-3");
        assert_eq!(escape("+2"), "+2");
    }

    #[test]
    fn full_match_has_summary_line_score_players_and_plays() {
        let saved = fixtures::basketball_full();
        let csv = basketball(&saved, "2026-10-25T10:00:00+08:00");
        assert!(csv.starts_with(BOM));
        let parts = sections(&csv);
        assert_eq!(parts.len(), 4);

        let summary = &parts[0];
        assert_eq!(summary[0], ["Section", "Field", "Value"]);
        let field = |name: &str| {
            summary
                .iter()
                .find(|r| r[1] == name)
                .map(|r| r[2].clone())
                .unwrap()
        };
        assert_eq!(field("GeneratedAt"), "2026-10-25T10:00:00+08:00");
        assert_eq!(field("Sport"), "Basketball");
        assert_eq!(field("Status"), "P4 00:00");
        assert_eq!(field("HomeScore"), saved.game.score(Side::Home).to_string());

        let line = &parts[1];
        assert_eq!(line[0], ["Team", "P1", "P2", "P3", "P4", "T"]);
        assert_eq!(line.len(), 3);

        let players = &parts[2];
        assert_eq!(players[0], PLAYER_HEADER);
        // 12 players and a Totals row per side.
        assert_eq!(players.len(), 1 + 2 * 13);
        assert!(players[1..].iter().all(|r| r.len() == PLAYER_HEADER.len()));
        let col = |name: &str| PLAYER_HEADER.iter().position(|h| *h == name).unwrap();
        let home: Vec<_> = players[1..14].to_vec();
        assert_eq!(home[12][col("Player")], "Totals");
        let sum: u32 = home[..12]
            .iter()
            .map(|r| r[col("PTS")].parse::<u32>().unwrap())
            .sum();
        assert_eq!(home[12][col("PTS")], sum.to_string());
        assert_eq!(sum, saved.game.score(Side::Home));
        assert!(home[..5].iter().all(|r| r[col("Starter")] == "Yes"));
        assert!(!home[0][col("MIN")].is_empty());
        assert!(home[12][col("PlusMinus")].is_empty());

        let plays = &parts[3];
        assert_eq!(plays[0], PLAY_HEADER);
        assert_eq!(plays.len(), 1 + saved.game.plays().len());
        assert_eq!(plays[1][0], "1");
        assert!(plays[1][1].starts_with('P'));
    }

    #[test]
    fn unstamped_match_leaves_minutes_and_clock_cells_blank() {
        let saved = fixtures::basketball_unstamped();
        let parts = sections(&basketball(&saved, "now"));
        let players = parts
            .iter()
            .find(|s| s[0] == PLAYER_HEADER)
            .expect("player section");
        let col = |name: &str| PLAYER_HEADER.iter().position(|h| *h == name).unwrap();
        assert!(players[1..].iter().all(|r| r[col("MIN")].is_empty()));
        assert!(players[1..].iter().all(|r| r[col("PlusMinus")].is_empty()));
        let plays = parts.last().unwrap();
        assert_eq!(plays[0], PLAY_HEADER);
        assert!(
            plays[1..]
                .iter()
                .all(|r| r[1].is_empty() && r[2].is_empty())
        );
    }

    #[test]
    fn no_attempts_leave_percent_blank() {
        assert_eq!(pct(0, 0), "");
        assert_eq!(pct(1, 3), "33");
    }
}
