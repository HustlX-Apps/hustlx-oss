use hustlx_oss::{Game, Side, Stat, Stats};

#[test]
fn scoring_changes_only_the_selected_player_and_team() {
    let mut game = Game::default();
    game.select(Side::Home, 1);
    assert!(game.record(Stat::ThreeMade));
    assert_eq!(game.score(Side::Home), 3);
    assert_eq!(game.score(Side::Away), 0);
    assert_eq!(game.player_stats(Side::Home, 1).points, 3);
    assert_eq!(game.player_stats(Side::Home, 2), Stats::default());
    assert_eq!(game.selected, None);
}

#[test]
fn shots_track_makes_attempts_and_free_throws_separately() {
    let mut game = Game::default();
    for stat in [
        Stat::ThreeMade,
        Stat::ThreeMiss,
        Stat::TwoMade,
        Stat::TwoMiss,
        Stat::FreeThrowMade,
        Stat::FreeThrowMiss,
    ] {
        game.select(Side::Away, 11);
        assert!(game.record(stat));
    }
    let stats = game.player_stats(Side::Away, 11);
    assert_eq!(stats.points, 6);
    assert_eq!(
        (stats.field_goals_made, stats.field_goals_attempted),
        (2, 4)
    );
    assert_eq!((stats.threes_made, stats.threes_attempted), (1, 2));
    assert_eq!(
        (stats.free_throws_made, stats.free_throws_attempted),
        (1, 2)
    );
}

#[test]
fn undo_restores_the_previous_score_and_stat_line() {
    let mut game = Game::default();
    game.select(Side::Home, 1);
    assert!(game.record(Stat::TwoMade));
    game.select(Side::Away, 11);
    assert!(game.record(Stat::ThreeMade));
    assert!(game.undo());
    assert_eq!(game.score(Side::Home), 2);
    assert_eq!(game.player_stats(Side::Away, 11), Stats::default());
    assert!(game.undo());
    assert_eq!(game.player_stats(Side::Home, 1), Stats::default());
    assert!(!game.undo());
}

#[test]
fn unknown_or_wrong_side_identity_cannot_receive_a_stat() {
    let mut game = Game::default();
    for (side, id) in [(Side::Home, 999), (Side::Home, 11), (Side::Away, 1)] {
        game.select(side, id);
        assert!(!game.record(Stat::TwoMade));
    }
    assert!(game.plays().is_empty());
}

#[test]
fn a_second_action_needs_a_new_player_selection() {
    let mut game = Game::default();
    game.select(Side::Home, 1);
    assert!(game.record(Stat::Assist));
    assert!(!game.record(Stat::Assist));
    assert_eq!(game.player_stats(Side::Home, 1).assists, 1);
}

#[test]
fn rebounds_and_discipline_are_player_scoped_and_undoable() {
    let mut game = Game::default();
    for stat in [
        Stat::OffensiveRebound,
        Stat::DefensiveRebound,
        Stat::Foul,
        Stat::Technical,
        Stat::Unsportsmanlike,
        Stat::Disqualifying,
        Stat::Warning,
        Stat::Turnover,
        Stat::Steal,
        Stat::Block,
    ] {
        game.select(Side::Home, 1);
        assert!(game.record(stat));
    }
    let s = game.player_stats(Side::Home, 1);
    assert_eq!(
        (s.offensive_rebounds, s.defensive_rebounds, s.fouls),
        (1, 1, 1)
    );
    assert_eq!(
        (
            s.technicals,
            s.unsportsmanlikes,
            s.disqualifications,
            s.warnings
        ),
        (1, 1, 1, 1)
    );
    assert_eq!((s.turnovers, s.steals, s.blocks), (1, 1, 1));
    assert_eq!(game.score(Side::Home), 0);
    assert_eq!(game.player_stats(Side::Away, 11), Stats::default());
    while game.undo() {}
    assert_eq!(game.player_stats(Side::Home, 1), Stats::default());
}
