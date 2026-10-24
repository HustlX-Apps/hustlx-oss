//! Minutes played and plus-minus, replayed from clock-stamped lineups and plays.
use crate::{Side, lineup::LineupKind, storage::SavedMatch};
use std::collections::HashMap;

/// Shortest period length, matching the default game clock.
const BASE_PERIOD_MS: u64 = 600_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Participation {
    pub seconds: u64,
    pub plus_minus: i64,
}

#[derive(Debug, Clone, Default)]
pub struct ParticipationTable(pub HashMap<(Side, i64), Participation>);

impl ParticipationTable {
    pub fn get(&self, side: Side, player_id: i64) -> Participation {
        self.0.get(&(side, player_id)).copied().unwrap_or_default()
    }
}

/// Length of a period: the default 10 minutes, or the largest stamped
/// `game_ms` in that period across plays and lineup changes.
pub fn period_length_ms(saved: &SavedMatch, period: u32) -> u64 {
    let period = period.max(1);
    let plays = saved.game.plays().iter().filter_map(|p| p.clock);
    let lineups = saved.lineups.iter().map(|l| l.clock);
    plays
        .chain(lineups)
        .filter(|c| c.period.max(1) == period)
        .map(|c| c.game_ms)
        .fold(BASE_PERIOD_MS, u64::max)
}

/// "M:SS", for example 1830 seconds is "30:30".
pub fn format_minutes(seconds: u64) -> String {
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

/// A point on the game timeline: period, then elapsed ms within it.
type Instant = (u32, u64);

enum Event<'a> {
    Lineup(&'a [i64], Side),
    Score(Side, u32),
}

struct Replay {
    lengths: HashMap<u32, u64>,
    court: [Vec<i64>; 2],
    ms: HashMap<(Side, i64), u64>,
    plus_minus: HashMap<(Side, i64), i64>,
    cursor: Instant,
}

fn slot(side: Side) -> usize {
    match side {
        Side::Home => 0,
        Side::Away => 1,
    }
}

impl Replay {
    fn length(&self, period: u32) -> u64 {
        self.lengths.get(&period).copied().unwrap_or(BASE_PERIOD_MS)
    }

    /// Credits on-court time from the cursor to `to`. Players stay on court
    /// across period breaks, so their interval reopens at the next period start.
    fn advance(&mut self, to: Instant) {
        if to <= self.cursor {
            return;
        }
        let (mut period, mut elapsed) = self.cursor;
        let mut delta = 0u64;
        while period < to.0 {
            delta += self.length(period).saturating_sub(elapsed);
            period += 1;
            elapsed = 0;
        }
        delta += to.1.saturating_sub(elapsed);
        for side in [Side::Home, Side::Away] {
            for id in &self.court[slot(side)] {
                *self.ms.entry((side, *id)).or_default() += delta;
            }
        }
        self.cursor = to;
    }

    fn score(&mut self, side: Side, points: u32) {
        let points = i64::from(points);
        for (s, sign) in [(side, 1), (other(side), -1)] {
            for id in &self.court[slot(s)] {
                *self.plus_minus.entry((s, *id)).or_default() += sign * points;
            }
        }
    }
}

fn other(side: Side) -> Side {
    match side {
        Side::Home => Side::Away,
        Side::Away => Side::Home,
    }
}

/// Minutes and plus-minus per player. None when either side has no starting
/// lineup or any scoring play lacks a clock stamp.
pub fn participation(saved: &SavedMatch) -> Option<ParticipationTable> {
    let plays = saved.game.plays();
    let has_starters = |side: Side| {
        saved
            .lineups
            .iter()
            .any(|l| l.side == side && l.kind == LineupKind::Starters)
    };
    if !has_starters(Side::Home) || !has_starters(Side::Away) {
        return None;
    }
    if plays
        .iter()
        .any(|p| p.stat.points() > 0 && p.clock.is_none())
    {
        return None;
    }

    let stamped_max = plays
        .iter()
        .filter_map(|p| p.clock)
        .chain(saved.lineups.iter().map(|l| l.clock))
        .map(|c| c.period.max(1))
        .max()
        .unwrap_or(1);
    let saved_period = saved.clock.period.max(1);
    let final_period = saved_period.max(stamped_max);
    let lengths: HashMap<u32, u64> = (1..=final_period)
        .map(|p| (p, period_length_ms(saved, p)))
        .collect();
    let elapsed = |period: u32, game_ms: u64| -> Instant {
        let period = period.max(1);
        let length = lengths.get(&period).copied().unwrap_or(BASE_PERIOD_MS);
        (period, length.saturating_sub(game_ms))
    };

    // (instant, lineups before plays, original order, event)
    let mut timeline: Vec<(Instant, u8, usize, Event)> = Vec::new();
    for (i, l) in saved.lineups.iter().enumerate() {
        let at = elapsed(l.clock.period, l.clock.game_ms);
        timeline.push((at, 0, i, Event::Lineup(&l.court, l.side)));
    }
    for (i, p) in plays.iter().enumerate() {
        let (Some(clock), points) = (p.clock, p.stat.points()) else {
            continue;
        };
        if points > 0 {
            let at = elapsed(clock.period, clock.game_ms);
            timeline.push((at, 1, i, Event::Score(p.side, points)));
        }
    }
    timeline.sort_by_key(|(at, kind, i, _)| (*at, *kind, *i));

    let mut replay = Replay {
        lengths: lengths.clone(),
        court: [Vec::new(), Vec::new()],
        ms: HashMap::new(),
        plus_minus: HashMap::new(),
        cursor: (1, 0),
    };
    for (at, _, _, event) in timeline {
        replay.advance(at);
        match event {
            Event::Lineup(court, side) => replay.court[slot(side)] = court.to_vec(),
            Event::Score(side, points) => replay.score(side, points),
        }
    }
    // The saved clock closes the last interval. When it sits in an earlier
    // period than the latest stamp, the latest stamped period closes at its end.
    let end = if saved_period >= stamped_max {
        elapsed(saved_period, saved.clock.game_ms)
    } else {
        elapsed(final_period, 0)
    };
    replay.advance(end);

    let mut table = HashMap::new();
    for side in [Side::Home, Side::Away] {
        for player in &saved.game.team(side).players {
            table.insert((side, player.id), Participation::default());
        }
    }
    for (key, ms) in replay.ms {
        table.entry(key).or_default().seconds = ms / 1000;
    }
    for (key, pm) in replay.plus_minus {
        table.entry(key).or_default().plus_minus = pm;
    }
    Some(ParticipationTable(table))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boxscore::{SheetResult, build, fixtures};
    use crate::{
        Game, Play, Player,
        Side::*,
        Stat, Team,
        clock::ClockState,
        lineup::{LineupChange, LineupKind},
    };

    fn at(period: u32, game_ms: u64) -> ClockState {
        ClockState {
            period,
            game_ms,
            ..ClockState::default()
        }
    }

    fn team(name: &str, ids: std::ops::RangeInclusive<i64>) -> Team {
        Team {
            name: name.into(),
            players: ids
                .map(|id| Player {
                    id,
                    name: format!("P{id}"),
                    jersey: id.to_string(),
                    positions: vec![],
                    on_court: false,
                })
                .collect(),
        }
    }

    fn starters(side: crate::Side, court: Vec<i64>, clock: ClockState) -> LineupChange {
        LineupChange {
            side,
            kind: LineupKind::Starters,
            before: vec![],
            court,
            clock,
        }
    }

    fn sub(
        side: crate::Side,
        before: Vec<i64>,
        court: Vec<i64>,
        clock: ClockState,
    ) -> LineupChange {
        LineupChange {
            side,
            kind: LineupKind::Substitution,
            before,
            court,
            clock,
        }
    }

    fn play(side: crate::Side, player_id: i64, stat: Stat, clock: ClockState) -> Play {
        Play {
            side,
            player_id,
            stat,
            clock: Some(clock),
        }
    }

    /// Home ids 1..=6 (1..=5 start), away ids 101..=105, all starting at `start`.
    fn saved(
        start: ClockState,
        mut extra: Vec<LineupChange>,
        plays: Vec<Play>,
        clock: ClockState,
    ) -> SavedMatch {
        let mut s = fixtures::basketball_unstamped();
        s.game = Game::from_saved([team("H", 1..=6), team("A", 101..=105)], plays);
        let mut lineups = vec![
            starters(Home, (1..=5).collect(), start),
            starters(Away, (101..=105).collect(), start),
        ];
        lineups.append(&mut extra);
        s.lineups = lineups;
        s.clock = clock;
        s
    }

    #[test]
    fn starter_subbed_out_at_p1_4_00_plays_six_minutes() {
        let s = saved(
            at(1, 600_000),
            vec![sub(
                Home,
                (1..=5).collect(),
                vec![6, 2, 3, 4, 5],
                at(1, 240_000),
            )],
            vec![],
            at(1, 0),
        );
        let table = participation(&s).expect("stamped match has participation");
        assert_eq!(table.0[&(Home, 1)].seconds, 360);
        assert_eq!(table.0[&(Home, 6)].seconds, 240);
        assert_eq!(table.0[&(Home, 2)].seconds, 600);
        assert_eq!(format_minutes(360), "6:00");
        assert_eq!(format_minutes(1830), "30:30");
    }

    #[test]
    fn player_on_court_across_period_break_accumulates_both_periods() {
        let s = saved(at(1, 600_000), vec![], vec![], at(2, 300_000));
        let table = participation(&s).unwrap();
        assert_eq!(table.0[&(Home, 1)].seconds, 900);
        assert_eq!(table.0[&(Away, 101)].seconds, 900);
        assert_eq!(table.0[&(Home, 6)].seconds, 0);
    }

    #[test]
    fn plus_minus_credits_on_court_players_only() {
        let s = saved(
            at(1, 600_000),
            vec![sub(
                Home,
                (1..=5).collect(),
                vec![6, 2, 3, 4, 5],
                at(1, 400_000),
            )],
            vec![
                play(Home, 2, Stat::ThreeMade, at(1, 500_000)),
                play(Away, 101, Stat::TwoMade, at(1, 300_000)),
            ],
            at(1, 0),
        );
        let table = participation(&s).unwrap();
        assert_eq!(table.0[&(Home, 1)].plus_minus, 3);
        assert_eq!(table.0[&(Home, 6)].plus_minus, -2);
        assert_eq!(table.0[&(Home, 2)].plus_minus, 1);
        assert_eq!(table.0[&(Away, 101)].plus_minus, -1);
    }

    #[test]
    fn period_length_adapts_to_longer_clock_and_minutes_never_exceed_it() {
        let s = saved(
            at(1, 720_000),
            vec![],
            vec![play(Home, 1, Stat::TwoMade, at(1, 720_000))],
            at(1, 0),
        );
        assert_eq!(period_length_ms(&s, 1), 720_000);
        assert_eq!(period_length_ms(&s, 2), 600_000);
        let table = participation(&s).unwrap();
        assert_eq!(table.0[&(Home, 1)].seconds, 720);
        // A saved clock above the period length closes at zero elapsed, never negative.
        let early = saved(at(1, 600_000), vec![], vec![], at(1, 650_000));
        assert_eq!(participation(&early).unwrap().0[&(Home, 1)].seconds, 0);
    }

    #[test]
    fn saved_clock_before_latest_stamped_period_uses_latest_period() {
        let s = saved(
            at(1, 600_000),
            vec![],
            vec![play(Home, 1, Stat::TwoMade, at(2, 300_000))],
            at(1, 0),
        );
        let table = participation(&s).unwrap();
        assert_eq!(table.0[&(Home, 1)].seconds, 1200);
    }

    #[test]
    fn none_without_starters_or_with_unstamped_scoring_play() {
        assert!(participation(&fixtures::basketball_unstamped()).is_none());
        let mut no_away = saved(at(1, 600_000), vec![], vec![], at(1, 0));
        no_away.lineups.retain(|l| l.side == Home);
        assert!(participation(&no_away).is_none());
        let mut unstamped = saved(at(1, 600_000), vec![], vec![], at(1, 0));
        unstamped.game = Game::from_saved(
            [team("H", 1..=6), team("A", 101..=105)],
            vec![Play {
                side: Home,
                player_id: 1,
                stat: Stat::TwoMade,
                clock: None,
            }],
        );
        assert!(participation(&unstamped).is_none());
    }

    #[test]
    fn basketball_full_fixture_shows_min_and_pm_columns_and_unstamped_hides_them() {
        let full = fixtures::basketball_full();
        let SheetResult::Ready(sheet) = build(&full, None, "2026-10-24", "0.1.0") else {
            panic!("ready");
        };
        let legend = sheet.legend.clone().unwrap();
        assert!(legend.contains("MIN = minutes"));
        assert!(legend.contains("+/- = plus-minus"));
        for block in &sheet.teams {
            assert_eq!(block.columns.len(), 22);
            assert!(block.rows.iter().all(|r| r.cells.len() == 22));
            assert_eq!(block.totals.cells.len(), 22);
            // Five on court for four 10 minute periods.
            let min = block.columns.iter().position(|c| c.key == "min").unwrap();
            assert_eq!(block.totals.cells[min], "200:00");
        }
        // Substitutions chain: each starts from the previous court, one swap apart.
        for side in [Home, Away] {
            let changes: Vec<_> = full.lineups.iter().filter(|l| l.side == side).collect();
            for pair in changes.windows(2) {
                assert_eq!(pair[1].before, pair[0].court);
                let swaps = pair[1]
                    .before
                    .iter()
                    .zip(&pair[1].court)
                    .filter(|(a, b)| a != b)
                    .count();
                assert_eq!(swaps, 1);
            }
        }
        let table = participation(&full).unwrap();
        let total: u64 = table
            .0
            .iter()
            .filter(|((side, _), _)| *side == Home)
            .map(|(_, p)| p.seconds)
            .sum();
        assert_eq!(total, 12_000);

        let SheetResult::Ready(plain) = build(
            &fixtures::basketball_unstamped(),
            None,
            "2026-10-24",
            "0.1.0",
        ) else {
            panic!("ready");
        };
        let legend = plain.legend.unwrap();
        assert!(!legend.contains("MIN"));
        assert!(!legend.contains("+/-"));
        assert!(legend.starts_with("No = jersey"));
        for block in &plain.teams {
            assert_eq!(block.columns.len(), 20);
            assert!(
                block
                    .columns
                    .iter()
                    .all(|c| c.key != "min" && c.key != "pm")
            );
        }
        // The game flow block hides with the line score.
        assert!(plain.line_score.is_none());
        assert!(plain.extra.is_none());
    }
}
