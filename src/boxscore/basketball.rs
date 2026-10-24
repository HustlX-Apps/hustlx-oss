//! Basketball box score: player rows, totals and the sheet around them.
use super::{
    Align, Block, Column, DESCRIPTOR, Footer, Header, Identity, LineRow, LineScore, Organizer, Row,
    Sheet, TeamBlock,
    minutes::{self, ParticipationTable},
    periods::{self, PeriodScores},
};
use crate::{
    Player, Side, Stats,
    clock::{self, ClockState},
    lineup::LineupKind,
    storage::SavedMatch,
};

const fn col(key: &'static str, label: &'static str, align: Align, weight: u8) -> Column {
    Column {
        key,
        label,
        align,
        weight,
    }
}

pub const COLUMNS: [Column; 22] = [
    col("no", "NO", Align::Center, 4),
    col("player", "PLAYER", Align::Left, 16),
    col("min", "MIN", Align::Right, 5),
    col("pts", "PTS", Align::Right, 4),
    col("fg", "FG", Align::Right, 6),
    col("fg_pct", "FG%", Align::Right, 4),
    col("p2", "2P", Align::Right, 6),
    col("p2_pct", "2P%", Align::Right, 4),
    col("p3", "3P", Align::Right, 6),
    col("p3_pct", "3P%", Align::Right, 4),
    col("ft", "FT", Align::Right, 6),
    col("ft_pct", "FT%", Align::Right, 4),
    col("or", "OR", Align::Right, 3),
    col("dr", "DR", Align::Right, 3),
    col("tot", "TOT", Align::Right, 4),
    col("as", "AS", Align::Right, 3),
    col("to", "TO", Align::Right, 3),
    col("st", "ST", Align::Right, 3),
    col("bs", "BS", Align::Right, 3),
    col("pf", "PF", Align::Right, 3),
    col("pm", "+/-", Align::Right, 4),
    col("eff", "EFF", Align::Right, 4),
];

const LEGEND_PARTS: [&str; 13] = [
    "No = jersey",
    "* = starter",
    "MIN = minutes",
    "M-A = made-attempted",
    "% = shooting percentage",
    "OR/DR/TOT = offensive/defensive/total rebounds",
    "AS = assists",
    "TO = turnovers",
    "ST = steals",
    "BS = blocks",
    "PF = personal fouls",
    "+/- = plus-minus",
    "EFF = efficiency",
];

/// Column legend; minutes and plus-minus appear only when participation is known.
pub fn legend(with_participation: bool) -> String {
    LEGEND_PARTS
        .iter()
        .filter(|part| {
            with_participation || !(part.starts_with("MIN ") || part.starts_with("+/- "))
        })
        .copied()
        .collect::<Vec<_>>()
        .join(" \u{b7} ")
}

fn signed(n: i64) -> String {
    if n > 0 {
        format!("+{n}")
    } else {
        n.to_string()
    }
}

/// Picks the cells for `columns` by key; keys without a value get an empty cell.
fn cells_for(columns: &[Column], values: &[(&'static str, String)]) -> Vec<String> {
    columns
        .iter()
        .map(|c| {
            values
                .iter()
                .find(|(k, _)| *k == c.key)
                .map(|(_, v)| v.clone())
                .unwrap_or_default()
        })
        .collect()
}

pub fn period_label(period: u32) -> String {
    if period <= 4 {
        format!("P{period}")
    } else {
        format!("OT{}", period - 4)
    }
}

pub fn status_line(clock: &ClockState) -> String {
    format!(
        "{} {}",
        period_label(clock.period),
        clock::format_game(clock.game_ms)
    )
}

pub fn efficiency(s: &Stats) -> i64 {
    let credit =
        s.points + s.offensive_rebounds + s.defensive_rebounds + s.assists + s.steals + s.blocks;
    let debit = (s.field_goals_attempted - s.field_goals_made)
        + (s.free_throws_attempted - s.free_throws_made)
        + s.turnovers;
    i64::from(credit) - i64::from(debit)
}

pub fn pct(made: u32, attempted: u32) -> String {
    // Rounded to the nearest whole percent.
    (made * 100 + attempted / 2)
        .checked_div(attempted)
        .map_or_else(|| "-".to_string(), |v| v.to_string())
}

/// Starters first, then numeric jerseys ascending, then non-numeric jerseys by name.
pub fn row_order(players: &[Player], starters: &[i64]) -> Vec<i64> {
    let mut sorted: Vec<&Player> = players.iter().collect();
    sorted.sort_by(|a, b| {
        let key = |p: &Player| {
            let jersey = p.jersey.parse::<u32>();
            (
                !starters.contains(&p.id),
                jersey.is_err(),
                jersey.unwrap_or(0),
                p.name.clone(),
            )
        };
        key(a).cmp(&key(b))
    });
    sorted.into_iter().map(|p| p.id).collect()
}

fn starters_for(saved: &SavedMatch, side: Side) -> Vec<i64> {
    saved
        .lineups
        .iter()
        .find(|l| l.side == side && l.kind == LineupKind::Starters)
        .map(|l| l.court.clone())
        .unwrap_or_default()
}

fn add(total: &mut Stats, s: &Stats) {
    total.points += s.points;
    total.free_throws_made += s.free_throws_made;
    total.free_throws_attempted += s.free_throws_attempted;
    total.field_goals_made += s.field_goals_made;
    total.field_goals_attempted += s.field_goals_attempted;
    total.threes_made += s.threes_made;
    total.threes_attempted += s.threes_attempted;
    total.offensive_rebounds += s.offensive_rebounds;
    total.defensive_rebounds += s.defensive_rebounds;
    total.assists += s.assists;
    total.turnovers += s.turnovers;
    total.steals += s.steals;
    total.blocks += s.blocks;
    total.fouls += s.fouls;
}

/// Stat cells keyed by column, from `pts` to `eff`, excluding `min` and `pm`.
fn stat_cells(s: &Stats) -> Vec<(&'static str, String)> {
    let p2_made = s.field_goals_made - s.threes_made;
    let p2_att = s.field_goals_attempted - s.threes_attempted;
    let pair = |m: u32, a: u32| format!("{m}-{a}");
    vec![
        ("pts", s.points.to_string()),
        ("fg", pair(s.field_goals_made, s.field_goals_attempted)),
        ("fg_pct", pct(s.field_goals_made, s.field_goals_attempted)),
        ("p2", pair(p2_made, p2_att)),
        ("p2_pct", pct(p2_made, p2_att)),
        ("p3", pair(s.threes_made, s.threes_attempted)),
        ("p3_pct", pct(s.threes_made, s.threes_attempted)),
        ("ft", pair(s.free_throws_made, s.free_throws_attempted)),
        ("ft_pct", pct(s.free_throws_made, s.free_throws_attempted)),
        ("or", s.offensive_rebounds.to_string()),
        ("dr", s.defensive_rebounds.to_string()),
        (
            "tot",
            (s.offensive_rebounds + s.defensive_rebounds).to_string(),
        ),
        ("as", s.assists.to_string()),
        ("to", s.turnovers.to_string()),
        ("st", s.steals.to_string()),
        ("bs", s.blocks.to_string()),
        ("pf", s.fouls.to_string()),
        ("eff", efficiency(s).to_string()),
    ]
}

pub fn team_block(
    saved: &SavedMatch,
    side: Side,
    participation: Option<&ParticipationTable>,
) -> TeamBlock {
    // Without participation the min and pm columns are omitted.
    let keep = |key: &str| participation.is_some() || (key != "min" && key != "pm");
    let columns: Vec<Column> = COLUMNS.iter().copied().filter(|c| keep(c.key)).collect();
    let game = &saved.game;
    let team = game.team(side);
    let starters = starters_for(saved, side);

    let mut total = Stats::default();
    let mut total_seconds = 0u64;
    let mut rows = Vec::new();
    let mut notes = Vec::new();
    for id in row_order(&team.players, &starters) {
        let Some(p) = game.player(side, id) else {
            continue;
        };
        let s = game.player_stats(side, id);
        add(&mut total, &s);
        let mark = if starters.contains(&id) { "*" } else { "" };
        let mut values = vec![
            ("no", format!("{}{mark}", p.jersey)),
            ("player", p.name.clone()),
        ];
        if let Some(table) = participation {
            let part = table.get(side, id);
            total_seconds += part.seconds;
            values.push(("min", minutes::format_minutes(part.seconds)));
            values.push(("pm", signed(part.plus_minus)));
        }
        values.extend(stat_cells(&s));
        rows.push(Row {
            cells: cells_for(&columns, &values),
            emphasis: false,
        });
        let mut flags = Vec::new();
        for (label, n) in [
            ("T", s.technicals),
            ("U", s.unsportsmanlikes),
            ("D", s.disqualifications),
        ] {
            if n > 0 {
                flags.push(format!("{label}{n}"));
            }
        }
        if !flags.is_empty() {
            notes.push(format!("{} {}: {}", p.jersey, p.name, flags.join(" ")));
        }
    }
    // Team plus-minus is left blank; only the minutes total is shown.
    let mut totals = vec![("player", "Totals".to_string())];
    if participation.is_some() {
        totals.push(("min", minutes::format_minutes(total_seconds)));
    }
    totals.extend(stat_cells(&total));
    let totals = cells_for(&columns, &totals);
    TeamBlock {
        name: team.name.clone(),
        side_letter: if side == Side::Home { 'A' } else { 'B' },
        columns,
        rows,
        totals: Row {
            cells: totals,
            emphasis: true,
        },
        notes: if notes.is_empty() {
            vec![]
        } else {
            vec![notes.join(" \u{b7} ")]
        },
    }
}

fn line_score(saved: &SavedMatch, scores: &PeriodScores) -> LineScore {
    let row = |side: Side, name: &str, cells: &[u32]| LineRow {
        name: name.to_string(),
        cells: cells.iter().map(u32::to_string).collect(),
        total: saved.game.score(side).to_string(),
    };
    let mut columns = scores.labels.clone();
    columns.push("T".to_string());
    LineScore {
        title: "Scoring by period".to_string(),
        columns,
        rows: [
            row(Side::Home, &saved.summary.home_name, &scores.home),
            row(Side::Away, &saved.summary.away_name, &scores.away),
        ],
    }
}

fn game_flow_block(flow: &periods::GameFlow) -> Block {
    Block {
        title: "Game flow".to_string(),
        lines: vec![
            format!(
                "Biggest lead: A {} \u{b7} B {}",
                flow.biggest_lead[0], flow.biggest_lead[1]
            ),
            format!(
                "Biggest run: A {} \u{b7} B {}",
                flow.biggest_run[0], flow.biggest_run[1]
            ),
            format!("Lead changes: {}", flow.lead_changes),
            format!("Times tied: {}", flow.times_tied),
        ],
    }
}

pub fn sheet(
    saved: &SavedMatch,
    organizer: Option<&Organizer>,
    generated_on: &str,
    version: &str,
) -> Sheet {
    let summary = &saved.summary;
    let plays = saved.game.plays();
    let scores = periods::period_scores(plays);
    let participation = minutes::participation(saved);
    let mut teams = [
        team_block(saved, Side::Home, participation.as_ref()),
        team_block(saved, Side::Away, participation.as_ref()),
    ];
    if let Some(scores) = &scores {
        for (team, side) in teams.iter_mut().zip([Side::Home, Side::Away]) {
            let fouls = periods::team_fouls_by_period(plays, side, scores.labels.len());
            let joined: Vec<String> = fouls.iter().map(u32::to_string).collect();
            team.notes
                .push(format!("Team fouls by period: {}", joined.join(" \u{b7} ")));
        }
    }
    Sheet {
        identity: Identity {
            organizer_name: organizer.map(|o| o.name.clone()),
            organizer_logo: organizer.and_then(|o| o.logo_png.clone()),
            sport_label: summary.sport.label().to_string(),
            descriptor: DESCRIPTOR,
        },
        header: Header {
            match_name: summary.name.clone(),
            date: summary.date.clone(),
            venue: Some(summary.venue.clone()).filter(|v| !v.trim().is_empty()),
            home_name: summary.home_name.clone(),
            away_name: summary.away_name.clone(),
            home_score: saved.game.score(Side::Home).to_string(),
            away_score: saved.game.score(Side::Away).to_string(),
            status: status_line(&saved.clock),
        },
        line_score: scores.as_ref().map(|p| line_score(saved, p)),
        teams,
        // Game flow is hidden whenever the line score is hidden.
        extra: scores
            .as_ref()
            .and(periods::game_flow(plays))
            .map(|f| game_flow_block(&f)),
        legend: Some(legend(participation.is_some())),
        footer: Footer {
            center: "powered by HustlX".to_string(),
            right: format!("Generated {generated_on} by HustlX OSS {version}"),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boxscore::{SheetResult, build, fixtures};
    use crate::{Player, Side, Stats};

    fn ready(saved: &SavedMatch) -> Sheet {
        let SheetResult::Ready(sheet) = build(saved, None, "2026-10-24", "0.1.0") else {
            panic!("expected a ready sheet");
        };
        sheet
    }

    fn player(id: i64, name: &str, jersey: &str) -> Player {
        Player {
            id,
            name: name.into(),
            jersey: jersey.into(),
            positions: vec![],
            on_court: false,
        }
    }

    #[test]
    fn build_returns_unsupported_for_volleyball() {
        assert!(matches!(
            build(&fixtures::volleyball_minimal(), None, "2026-10-24", "0.1.0"),
            SheetResult::Unsupported { .. }
        ));
    }

    #[test]
    fn status_line_prints_saved_clock_not_final() {
        let sheet = ready(&fixtures::basketball_full());
        assert_eq!(sheet.header.status, "P4 00:00");
        assert!(!sheet.header.status.contains("FINAL"));
        assert_eq!(period_label(5), "OT1");
        assert_eq!(period_label(4), "P4");
        assert_eq!(period_label(6), "OT2");
    }

    #[test]
    fn team_block_has_22_columns_and_one_row_per_rostered_player() {
        assert_eq!(COLUMNS.len(), 22);
        let sheet = ready(&fixtures::basketball_full());
        for block in &sheet.teams {
            assert_eq!(block.columns.len(), 22);
            assert_eq!(block.rows.len(), 12);
            assert!(block.rows.iter().all(|r| r.cells.len() == 22));
            assert_eq!(block.totals.cells.len(), 22);
        }
        // Without participation min and pm are omitted.
        let plain = ready(&fixtures::basketball_unstamped());
        for block in &plain.teams {
            assert_eq!(block.columns.len(), 20);
            assert!(block.rows.iter().all(|r| r.cells.len() == 20));
            assert_eq!(block.totals.cells.len(), 20);
        }
    }

    #[test]
    fn did_not_play_player_gets_zero_row() {
        let sheet = ready(&fixtures::basketball_full());
        let row = sheet.teams[0]
            .rows
            .iter()
            .find(|r| r.cells[1].ends_with("Player 12"))
            .expect("did-not-play player is listed");
        let idx = |key: &str| {
            sheet.teams[0]
                .columns
                .iter()
                .position(|c| c.key == key)
                .unwrap()
        };
        assert_eq!(row.cells[idx("pts")], "0");
        assert_eq!(row.cells[idx("fg")], "0-0");
        assert_eq!(row.cells[idx("fg_pct")], "-");
    }

    #[test]
    fn starters_marked_and_sorted_first() {
        let sheet = ready(&fixtures::basketball_full());
        for block in &sheet.teams {
            for row in &block.rows[..5] {
                assert!(row.cells[0].ends_with('*'), "{:?}", row.cells[0]);
            }
            assert!(!block.rows[5].cells[0].ends_with('*'));
        }
    }

    #[test]
    fn jersey_sorting_handles_empty_double_zero_and_letters() {
        let players = vec![
            player(1, "Zed", "00"),
            player(2, "Yan", "7"),
            player(3, "Bea", "A"),
            player(4, "Ann", ""),
            player(5, "Xia", "23"),
        ];
        // Numeric jerseys ascending, then non-numeric by name ("" is Ann, "A" is Bea).
        assert_eq!(row_order(&players, &[]), vec![1, 2, 5, 4, 3]);
        // Starters come first.
        assert_eq!(row_order(&players, &[5]), vec![5, 1, 2, 4, 3]);
    }

    #[test]
    fn totals_recompute_percentages_and_eff() {
        let s = Stats {
            points: 10,
            offensive_rebounds: 2,
            defensive_rebounds: 3,
            assists: 4,
            steals: 1,
            blocks: 0,
            field_goals_made: 4,
            field_goals_attempted: 9,
            free_throws_made: 2,
            free_throws_attempted: 4,
            turnovers: 3,
            ..Stats::default()
        };
        assert_eq!(efficiency(&s), 10);
        assert_eq!(pct(0, 0), "-");
        assert_eq!(pct(1, 3), "33");

        let saved = fixtures::basketball_full();
        let block = team_block(&saved, Side::Home, None);
        let col = |key: &str| block.columns.iter().position(|c| c.key == key).unwrap();
        let sum = |key: &str| -> u32 {
            block
                .rows
                .iter()
                .map(|r| r.cells[col(key)].parse::<u32>().unwrap())
                .sum()
        };
        let (mut made, mut att) = (0, 0);
        for row in &block.rows {
            let (m, a) = row.cells[col("fg")].split_once('-').unwrap();
            made += m.parse::<u32>().unwrap();
            att += a.parse::<u32>().unwrap();
        }
        assert_eq!(block.totals.cells[col("fg")], format!("{made}-{att}"));
        assert_eq!(block.totals.cells[col("fg_pct")], pct(made, att));
        assert_eq!(block.totals.cells[col("pts")], sum("pts").to_string());
        assert_eq!(
            block.totals.cells[col("pts")],
            saved.game.score(Side::Home).to_string()
        );
        assert!(block.totals.emphasis);
    }

    #[test]
    fn fouls_note_lists_technical_and_unsportsmanlike() {
        let sheet = ready(&fixtures::basketball_full());
        assert_eq!(sheet.teams[0].notes.len(), 2);
        assert!(sheet.teams[0].notes[0].contains("T1"));
        assert!(sheet.teams[0].notes[0].contains("U1"));
        assert_eq!(sheet.teams[1].notes.len(), 1);
        assert!(sheet.teams[1].notes[0].starts_with("Team fouls by period"));
        let plain = ready(&fixtures::basketball_unstamped());
        assert!(plain.teams.iter().all(|t| t.notes.is_empty()));
    }

    #[test]
    fn footer_is_powered_by_hustlx_only() {
        let sheet = ready(&fixtures::basketball_full());
        assert_eq!(sheet.footer.center, "powered by HustlX");
        assert_eq!(
            sheet.footer.right,
            "Generated 2026-10-24 by HustlX OSS 0.1.0"
        );
        assert!(!sheet.header.match_name.contains("HustlX"));
        assert_eq!(sheet.identity.descriptor, "Reference-grade box score");
    }
}
