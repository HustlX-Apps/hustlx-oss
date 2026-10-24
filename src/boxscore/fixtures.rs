//! Deterministic saved matches for box score tests and tooling.
use crate::{
    Game, Play, Player, Side, Stat, Team,
    clock::ClockState,
    lineup::{LineupChange, LineupKind},
    sport::Sport,
    storage::{MatchSummary, SavedMatch},
};

const HOME_BASE: i64 = 0;
const AWAY_BASE: i64 = 100;

fn roster(label: &str, base: i64, count: usize, name_len: usize, starters: bool) -> Vec<Player> {
    (1..=count)
        .map(|n| {
            let mut name = format!("{label} Player {n}");
            while name.len() < name_len {
                name.push('x');
            }
            Player {
                id: base + n as i64,
                name,
                jersey: (n + 3).to_string(),
                positions: vec![],
                on_court: starters && n <= 5,
            }
        })
        .collect()
}

fn clock_at(period: u32, game_ms: u64) -> ClockState {
    ClockState {
        period,
        game_ms,
        ..ClockState::default()
    }
}

fn plays(stamped: bool, active: usize, periods: u32) -> Vec<Play> {
    const CYCLE: [Stat; 9] = [
        Stat::TwoMade,
        Stat::ThreeMiss,
        Stat::DefensiveRebound,
        Stat::Assist,
        Stat::FreeThrowMade,
        Stat::TwoMiss,
        Stat::ThreeMade,
        Stat::Turnover,
        Stat::Foul,
    ];
    let play = |side: Side, base: i64, n: usize, k: usize, period: u32, slot: (usize, usize)| {
        let (sn, sk) = slot;
        Play {
            side,
            player_id: base + n as i64,
            stat: CYCLE[(n + k + period as usize) % CYCLE.len()],
            clock: stamped
                .then(|| clock_at(period, 540_000 - (sn as u64 * 20_000) - sk as u64 * 5_000)),
        }
    };
    // Possessions alternate in clock order. Each player keeps the same stats per
    // period; the away roster runs in reverse so the lead swings both ways.
    let mut out = Vec::new();
    for period in 1..=periods {
        for n in 1..=active {
            for k in 0..3 {
                let home = play(Side::Home, HOME_BASE, n, k, period, (n, k));
                let away = play(Side::Away, AWAY_BASE, active + 1 - n, k, period, (n, k));
                if (n + k + period as usize).is_multiple_of(2) {
                    out.extend([home, away]);
                } else {
                    out.extend([away, home]);
                }
            }
        }
    }
    out
}

fn summary(name: &str, home: &str, away: &str, sport: Sport) -> MatchSummary {
    MatchSummary {
        sport,
        id: 1,
        name: name.to_string(),
        date: "2026-10-24".to_string(),
        venue: "Main Gym".to_string(),
        home_name: home.to_string(),
        away_name: away.to_string(),
        revision: 0,
    }
}

fn starters(side: Side, base: i64) -> LineupChange {
    LineupChange {
        side,
        kind: LineupKind::Starters,
        before: vec![],
        court: (1..=5).map(|n| base + n).collect(),
        clock: ClockState::default(),
    }
}

/// One swap: `before` is the current court and `court` replaces `out` with `incoming`.
fn sub(side: Side, before: Vec<i64>, out: i64, incoming: i64, period: u32) -> LineupChange {
    let court = before
        .iter()
        .map(|id| if *id == out { incoming } else { *id })
        .collect();
    LineupChange {
        side,
        kind: LineupKind::Substitution,
        before,
        court,
        clock: clock_at(period, 300_000),
    }
}

struct Spec<'a> {
    name: &'a str,
    home: &'a str,
    away: &'a str,
    count: usize,
    name_len: usize,
    stamped: bool,
    lineups: Vec<LineupChange>,
}

fn basketball(spec: Spec) -> SavedMatch {
    let has_lineups = !spec.lineups.is_empty();
    let teams = [
        Team {
            name: spec.home.to_string(),
            players: roster("Home", HOME_BASE, spec.count, spec.name_len, has_lineups),
        },
        Team {
            name: spec.away.to_string(),
            players: roster("Away", AWAY_BASE, spec.count, spec.name_len, has_lineups),
        },
    ];
    // The last two players on each roster never get a play (did-not-play rows).
    let mut plays = plays(spec.stamped, spec.count.saturating_sub(2).min(10), 4);
    if spec.stamped {
        plays.push(Play {
            side: Side::Home,
            player_id: HOME_BASE + 2,
            stat: Stat::Technical,
            clock: Some(clock_at(2, 200_000)),
        });
        plays.push(Play {
            side: Side::Home,
            player_id: HOME_BASE + 3,
            stat: Stat::Unsportsmanlike,
            clock: Some(clock_at(3, 200_000)),
        });
    }
    SavedMatch {
        sport_state: None,
        sport_history: vec![],
        summary: summary(spec.name, spec.home, spec.away, Sport::Basketball),
        game: Game::from_saved(teams, plays),
        clock: clock_at(4, 0),
        lineups: spec.lineups,
    }
}

/// 12 players a side, stamped plays over 4 periods, starters and 3 subs a side,
/// final clock P4 0:00, one technical and one unsportsmanlike foul.
pub fn basketball_full() -> SavedMatch {
    let mut lineups = vec![
        starters(Side::Home, HOME_BASE),
        starters(Side::Away, AWAY_BASE),
    ];
    // Each substitution starts from the court the previous one left.
    let mut courts = [lineups[0].court.clone(), lineups[1].court.clone()];
    for (i, (out, incoming)) in [(5, 6), (4, 7), (3, 8)].into_iter().enumerate() {
        for (court, (side, base)) in courts
            .iter_mut()
            .zip([(Side::Home, HOME_BASE), (Side::Away, AWAY_BASE)])
        {
            let change = sub(
                side,
                court.clone(),
                base + out,
                base + incoming,
                i as u32 + 1,
            );
            *court = change.court.clone();
            lineups.push(change);
        }
    }
    basketball(Spec {
        name: "Finals Game 1",
        home: "Home Hawks",
        away: "Away Eagles",
        count: 12,
        name_len: 0,
        stamped: true,
        lineups,
    })
}

/// 7 players a side, plays with no clock stamp and no lineups.
pub fn basketball_unstamped() -> SavedMatch {
    basketball(Spec {
        name: "Pickup Game",
        home: "Home Hawks",
        away: "Away Eagles",
        count: 7,
        name_len: 0,
        stamped: false,
        lineups: vec![],
    })
}

/// 20 players a side, 60 character home name, 40 character player names.
pub fn basketball_oversized() -> SavedMatch {
    let home = "H".repeat(60);
    basketball(Spec {
        name: "Oversized Roster Game",
        home: &home,
        away: "Away Eagles",
        count: 20,
        name_len: 40,
        stamped: true,
        lineups: vec![
            starters(Side::Home, HOME_BASE),
            starters(Side::Away, AWAY_BASE),
        ],
    })
}

/// A volleyball match, used only to prove `Unsupported`.
pub fn volleyball_minimal() -> SavedMatch {
    let teams = [
        Team {
            name: "Home Spikers".to_string(),
            players: vec![],
        },
        Team {
            name: "Away Setters".to_string(),
            players: vec![],
        },
    ];
    SavedMatch {
        sport_state: None,
        sport_history: vec![],
        summary: summary("Volley", "Home Spikers", "Away Setters", Sport::Volleyball),
        game: Game::from_saved(teams, vec![]),
        clock: ClockState::default(),
        lineups: vec![],
    }
}
