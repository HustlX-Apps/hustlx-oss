use hustlx_oss::{
    Side::{Away, Home},
    operator::PlayerAction,
    sport::{Command, Rules, Sport, State},
};

#[test]
fn volleyball_sideout_rotates_live_court_and_attributed_actions_keep_stats() {
    let mut s = State::new(Sport::Volleyball, Rules::defaults(Sport::Volleyball)).unwrap();
    s.apply(Command::Lineup {
        side: Home,
        players: [1, 2, 3, 4, 5, 6, 0, 0, 0, 0, 0],
    })
    .unwrap();
    s.apply(Command::Lineup {
        side: Away,
        players: [7, 8, 9, 10, 11, 12, 0, 0, 0, 0, 0],
    })
    .unwrap();
    s.apply(Command::Player {
        side: Away,
        player: Some(8),
        action: PlayerAction::Kill,
    })
    .unwrap();
    assert_eq!(s.points, [0, 1]);
    assert_eq!(s.operator.court[1], vec![8, 9, 10, 11, 12, 7]);
    assert_eq!(s.operator.count(Away, 8, PlayerAction::Kill), 1);
    s.apply(Command::Player {
        side: Away,
        player: Some(8),
        action: PlayerAction::Dig,
    })
    .unwrap();
    assert_eq!(s.points, [0, 1]);
    s.apply(Command::Player {
        side: Away,
        player: Some(8),
        action: PlayerAction::ServiceError,
    })
    .unwrap();
    assert_eq!(s.points, [1, 1]);
    assert_eq!(s.operator.court[0], vec![2, 3, 4, 5, 6, 1]);
}

#[test]
fn substitutions_and_libero_preserve_slots_with_open_rules() {
    let mut s = State::new(Sport::Volleyball, Rules::defaults(Sport::Volleyball)).unwrap();
    s.apply(Command::Lineup {
        side: Home,
        players: [1, 2, 3, 4, 5, 6, 0, 0, 0, 0, 0],
    })
    .unwrap();
    s.apply(Command::Exchange {
        side: Home,
        outgoing: 2,
        incoming: 7,
        libero: false,
    })
    .unwrap();
    assert_eq!(s.operator.court[0], vec![1, 7, 3, 4, 5, 6]);
    s.apply(Command::Exchange {
        side: Home,
        outgoing: 7,
        incoming: 8,
        libero: true,
    })
    .unwrap();
    s.apply(Command::Exchange {
        side: Home,
        outgoing: 8,
        incoming: 2,
        libero: true,
    })
    .unwrap();
    assert_eq!(s.operator.court[0], vec![1, 2, 3, 4, 5, 6]);
    let before = s.clone();
    assert!(
        s.apply(Command::Exchange {
            side: Home,
            outgoing: 1,
            incoming: 2,
            libero: false
        })
        .is_err()
    );
    assert_eq!(s, before);
}

#[test]
fn racquet_attribution_respects_sideout_and_errors_credit_opponent() {
    let mut s = State::new(Sport::Pickleball, Rules::defaults(Sport::Pickleball)).unwrap();
    s.apply(Command::Player {
        side: Away,
        player: Some(4),
        action: PlayerAction::Winner,
    })
    .unwrap();
    assert_eq!(s.points, [0, 0]);
    assert_eq!(s.server, Away);
    s.apply(Command::Player {
        side: Home,
        player: Some(3),
        action: PlayerAction::UnforcedError,
    })
    .unwrap();
    assert_eq!(s.points, [0, 1]);
    assert_eq!(s.operator.count(Home, 3, PlayerAction::UnforcedError), 1);
}

#[test]
fn a_new_volleyball_set_restores_explicit_starters_and_resets_set_counters() {
    let mut r = Rules::defaults(Sport::Volleyball);
    r.best_of = 3;
    r.target = 2;
    r.decider_target = 2;
    let mut s = State::new(Sport::Volleyball, r).unwrap();
    s.apply(Command::Lineup {
        side: Home,
        players: [1, 2, 3, 4, 5, 6, 0, 0, 0, 0, 0],
    })
    .unwrap();
    s.apply(Command::Timeout(Home)).unwrap();
    s.apply(Command::Exchange {
        side: Home,
        outgoing: 1,
        incoming: 7,
        libero: false,
    })
    .unwrap();
    s.apply(Command::Rally(Home)).unwrap();
    s.apply(Command::Rally(Home)).unwrap();
    assert_eq!(s.unit_number, 2);
    assert_eq!(s.operator.court[0], vec![1, 2, 3, 4, 5, 6]);
    assert_eq!(s.operator.timeouts, [0, 0]);
    assert_eq!(s.operator.substitutions, [0, 0]);
}

#[test]
fn six_volleyball_sideouts_restore_both_courts_and_serving_rallies_do_not_rotate() {
    let mut s = State::new(Sport::Volleyball, Rules::defaults(Sport::Volleyball)).unwrap();
    for (side, players) in [
        (Home, [1, 2, 3, 4, 5, 6, 0, 0, 0, 0, 0]),
        (Away, [7, 8, 9, 10, 11, 12, 0, 0, 0, 0, 0]),
    ] {
        s.apply(Command::Lineup { side, players }).unwrap();
    }
    let starters = s.operator.starters.clone();
    for turn in 1..=6 {
        s.apply(Command::Rally(Away)).unwrap();
        let mut expected = starters[1].clone();
        expected.rotate_left(turn % 6);
        assert_eq!(s.operator.court[1], expected);
        s.apply(Command::Rally(Away)).unwrap();
        assert_eq!(s.operator.court[1], expected);
        s.apply(Command::Player {
            side: Away,
            player: Some(7),
            action: PlayerAction::Dig,
        })
        .unwrap();
        assert_eq!(s.operator.court[1], expected);
        s.apply(Command::Rally(Home)).unwrap();
        let mut expected = starters[0].clone();
        expected.rotate_left(turn % 6);
        assert_eq!(s.operator.court[0], expected);
    }
    assert_eq!(s.operator.court, starters);
    assert_eq!(s.points, [6, 12]);
}
