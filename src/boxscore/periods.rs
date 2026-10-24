//! Period line score, game flow and team fouls, derived from stamped plays.
use super::basketball::period_label;
use crate::{Play, Side, Stat};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeriodScores {
    pub labels: Vec<String>,
    pub home: Vec<u32>,
    pub away: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameFlow {
    pub biggest_lead: [u32; 2],
    pub biggest_run: [u32; 2],
    pub lead_changes: u32,
    pub times_tied: u32,
}

fn scoring_is_stamped(plays: &[Play]) -> bool {
    plays
        .iter()
        .all(|p| p.stat.points() == 0 || p.clock.is_some())
}

fn max_period(plays: &[Play]) -> Option<u32> {
    plays
        .iter()
        .filter_map(|p| p.clock.as_ref().map(|c| c.period.max(1)))
        .max()
}

fn period_index(play: &Play) -> usize {
    play.clock.as_ref().map_or(1, |c| c.period.max(1)) as usize - 1
}

pub fn period_scores(plays: &[Play]) -> Option<PeriodScores> {
    if !scoring_is_stamped(plays) {
        return None;
    }
    let count = max_period(plays)?.max(4) as usize;
    let mut home = vec![0; count];
    let mut away = vec![0; count];
    for play in plays {
        let idx = period_index(play);
        match play.side {
            Side::Home => home[idx] += play.stat.points(),
            Side::Away => away[idx] += play.stat.points(),
        }
    }
    Some(PeriodScores {
        labels: (1..=count as u32).map(period_label).collect(),
        home,
        away,
    })
}

pub fn game_flow(plays: &[Play]) -> Option<GameFlow> {
    if !scoring_is_stamped(plays) {
        return None;
    }
    let mut flow = GameFlow {
        biggest_lead: [0, 0],
        biggest_run: [0, 0],
        lead_changes: 0,
        times_tied: 0,
    };
    let mut diff: i64 = 0;
    let mut last_sign: i64 = 0;
    let mut run_side: Option<Side> = None;
    let mut run: u32 = 0;
    for play in plays {
        let pts = play.stat.points();
        if pts == 0 {
            continue;
        }
        let (idx, delta) = match play.side {
            Side::Home => (0, i64::from(pts)),
            Side::Away => (1, -i64::from(pts)),
        };
        if run_side == Some(play.side) {
            run += pts;
        } else {
            run_side = Some(play.side);
            run = pts;
        }
        flow.biggest_run[idx] = flow.biggest_run[idx].max(run);
        diff += delta;
        if diff == 0 {
            flow.times_tied += 1;
        } else {
            let sign = diff.signum();
            if last_sign != 0 && sign != last_sign {
                flow.lead_changes += 1;
            }
            last_sign = sign;
        }
        flow.biggest_lead[0] = flow.biggest_lead[0].max(u32::try_from(diff.max(0)).unwrap_or(0));
        flow.biggest_lead[1] = flow.biggest_lead[1].max(u32::try_from((-diff).max(0)).unwrap_or(0));
    }
    Some(flow)
}

pub fn team_fouls_by_period(plays: &[Play], side: Side, periods: usize) -> Vec<u32> {
    let mut out = vec![0; periods];
    for play in plays
        .iter()
        .filter(|p| p.side == side && p.stat == Stat::Foul)
    {
        if let Some(slot) = out.get_mut(period_index(play)) {
            *slot += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boxscore::{SheetResult, build, fixtures};
    use crate::clock::ClockState;
    use Side::{Away, Home};

    fn play(side: Side, stat: Stat, period: u32) -> Play {
        Play {
            side,
            player_id: 1,
            stat,
            clock: Some(ClockState {
                period,
                ..ClockState::default()
            }),
        }
    }

    fn unstamped(side: Side, stat: Stat) -> Play {
        Play {
            side,
            player_id: 1,
            stat,
            clock: None,
        }
    }

    #[test]
    fn period_scores_sum_points_per_period_and_label_overtime() {
        let plays = [
            play(Home, Stat::TwoMade, 1),
            play(Away, Stat::ThreeMade, 1),
            play(Home, Stat::FreeThrowMade, 5),
        ];
        let p = period_scores(&plays).unwrap();
        assert_eq!(p.labels, ["P1", "P2", "P3", "P4", "OT1"]);
        assert_eq!(p.home, [2, 0, 0, 0, 1]);
        assert_eq!(p.away, [3, 0, 0, 0, 0]);
    }

    #[test]
    fn period_scores_none_when_a_scoring_play_is_unstamped() {
        let plays = [play(Home, Stat::Assist, 1), unstamped(Home, Stat::TwoMade)];
        assert!(period_scores(&plays).is_none());
        assert!(game_flow(&plays).is_none());
    }

    #[test]
    fn unstamped_non_scoring_plays_do_not_hide_line_score() {
        let plays = [unstamped(Home, Stat::Assist), play(Home, Stat::TwoMade, 2)];
        let p = period_scores(&plays).unwrap();
        assert_eq!(p.home, [0, 2, 0, 0]);
    }

    #[test]
    fn game_flow_counts_lead_changes_ties_and_runs() {
        let plays = [
            play(Home, Stat::TwoMade, 1),
            play(Away, Stat::ThreeMade, 1),
            play(Away, Stat::TwoMade, 1),
            play(Home, Stat::ThreeMade, 2),
            play(Home, Stat::TwoMade, 2),
            play(Away, Stat::TwoMade, 2),
        ];
        let f = game_flow(&plays).unwrap();
        assert_eq!(f.lead_changes, 2);
        assert_eq!(f.times_tied, 2);
        assert_eq!(f.biggest_run, [5, 5]);
        assert_eq!(f.biggest_lead, [2, 3]);
    }

    #[test]
    fn team_fouls_by_period_counts_only_personal_fouls() {
        let plays = [
            play(Home, Stat::Foul, 2),
            play(Home, Stat::Foul, 2),
            play(Home, Stat::Technical, 2),
            play(Away, Stat::Foul, 3),
        ];
        assert_eq!(team_fouls_by_period(&plays, Home, 4), [0, 2, 0, 0]);
        assert_eq!(team_fouls_by_period(&plays, Away, 4), [0, 0, 1, 0]);
        let loose = [unstamped(Home, Stat::Foul)];
        assert_eq!(team_fouls_by_period(&loose, Home, 4), [1, 0, 0, 0]);
    }

    #[test]
    fn basketball_sheet_has_line_score_and_game_flow_for_stamped_fixture_and_none_for_unstamped() {
        let ready = |saved| {
            let SheetResult::Ready(s) = build(&saved, None, "2026-10-24", "0.1.0") else {
                panic!("ready")
            };
            s
        };
        let full_saved = fixtures::basketball_full();
        let full = ready(full_saved.clone());
        let ls = full.line_score.as_ref().expect("line score");
        assert_eq!(ls.title, "Scoring by period");
        assert_eq!(ls.columns.last().unwrap(), "T");
        assert_eq!(ls.rows[0].cells.len(), ls.columns.len() - 1);
        assert_eq!(
            ls.rows[0].total,
            full_saved.game.score(Side::Home).to_string()
        );
        let sum: u32 = ls.rows[0]
            .cells
            .iter()
            .map(|c| c.parse::<u32>().unwrap())
            .sum();
        assert_eq!(sum.to_string(), ls.rows[0].total);
        let flow = full.extra.as_ref().expect("game flow");
        assert_eq!(flow.title, "Game flow");
        assert_eq!(flow.lines.len(), 4);
        assert!(flow.lines[0].starts_with("Biggest lead: A "));
        let f = game_flow(full_saved.game.plays()).expect("stamped");
        assert!(f.lead_changes >= 1, "{f:?}");
        assert!(f.times_tied >= 1, "{f:?}");
        assert!(f.biggest_lead[0] > 0 && f.biggest_lead[1] > 0, "{f:?}");
        assert!(full.teams.iter().all(|t| {
            t.notes
                .last()
                .unwrap()
                .starts_with("Team fouls by period: ")
        }));

        let plain = ready(fixtures::basketball_unstamped());
        assert!(plain.line_score.is_none() && plain.extra.is_none());
        assert!(
            plain
                .teams
                .iter()
                .all(|t| t.notes.iter().all(|n| !n.contains("Team fouls")))
        );
    }
}
