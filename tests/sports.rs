use hustlx_oss::{
    Side::{self, Away, Home},
    sport::{Command, Format, Rules, Scoring, Sport, State, Status},
};

fn state(sport: Sport) -> State {
    State::new(sport, Rules::defaults(sport)).unwrap()
}
fn points(s: &mut State, side: Side, n: usize) {
    for _ in 0..n {
        s.apply(Command::Rally(side)).unwrap();
    }
}
fn game(s: &mut State, side: Side) {
    points(s, side, 4);
}

#[test]
fn volleyball_deuce_decider_and_completion_do_not_spill_into_next_set() {
    let mut r = Rules::defaults(Sport::Volleyball);
    r.best_of = 3;
    let mut s = State::new(Sport::Volleyball, r).unwrap();
    points(&mut s, Home, 24);
    points(&mut s, Away, 24);
    points(&mut s, Home, 1);
    assert_eq!(s.units, [0, 0]);
    points(&mut s, Home, 1);
    assert_eq!(s.units, [1, 0]);
    points(&mut s, Away, 25);
    points(&mut s, Home, 14);
    points(&mut s, Away, 14);
    points(&mut s, Home, 2);
    assert_eq!(s.status, Status::Decided);
    assert_eq!(s.winner, Some(Home));
    assert_eq!(s.completed_units[2].score, [16, 14]);
    assert!(s.apply(Command::Rally(Away)).is_err());
    s.apply(Command::Complete).unwrap();
    assert_eq!(s.status, Status::Completed);
}

#[test]
fn pickleball_sideout_doubles_opening_and_two_server_cycle() {
    let mut s = state(Sport::Pickleball);
    assert_eq!((s.server, s.server_number), (Home, 2));
    points(&mut s, Away, 1);
    assert_eq!(s.points, [0, 0]);
    assert_eq!((s.server, s.server_number), (Away, 1));
    points(&mut s, Home, 1);
    assert_eq!((s.server, s.server_number), (Away, 2));
    points(&mut s, Home, 1);
    assert_eq!((s.server, s.server_number), (Home, 1));
    points(&mut s, Home, 1);
    assert_eq!(s.points, [1, 0]);
    let mut r = Rules::defaults(Sport::Pickleball);
    r.format = Format::Singles;
    let mut singles = State::new(Sport::Pickleball, r).unwrap();
    points(&mut singles, Away, 1);
    assert_eq!((singles.server, singles.server_number), (Away, 1));
}

#[test]
fn pickleball_rally_scores_receiver_wins() {
    let mut r = Rules::defaults(Sport::Pickleball);
    r.scoring = Scoring::Rally;
    let mut s = State::new(Sport::Pickleball, r).unwrap();
    points(&mut s, Away, 1);
    assert_eq!(s.points, [0, 1]);
    assert_eq!(s.server, Away);
}

#[test]
fn table_tennis_service_switches_every_two_then_every_one_at_deuce() {
    let mut s = state(Sport::TableTennis);
    points(&mut s, Home, 2);
    assert_eq!(s.server, Away);
    points(&mut s, Away, 10);
    points(&mut s, Home, 8);
    assert_eq!(s.points, [10, 10]);
    let before = s.server;
    points(&mut s, Home, 1);
    assert_ne!(s.server, before);
    points(&mut s, Away, 1);
    assert_eq!(s.server, before);
    points(&mut s, Home, 2);
    assert_eq!(s.units, [1, 0]);
    assert_eq!(s.server, Away);
}

#[test]
fn badminton_deuce_cap_and_decider_end_change() {
    let mut s = state(Sport::Badminton);
    points(&mut s, Home, 20);
    points(&mut s, Away, 20);
    for _ in 0..9 {
        points(&mut s, Home, 1);
        points(&mut s, Away, 1);
    }
    points(&mut s, Home, 1);
    assert_eq!(s.completed_units[0].score, [30, 29]);
    points(&mut s, Away, 21);
    let ends = s.ends_changed;
    points(&mut s, Home, 11);
    assert_eq!(s.ends_changed, ends + 1);
    points(&mut s, Away, 1);
    assert_eq!(s.ends_changed, ends + 1);
}

#[test]
fn tennis_advantage_no_ad_tiebreak_and_next_set_service() {
    let mut s = state(Sport::Tennis);
    points(&mut s, Home, 3);
    points(&mut s, Away, 3);
    points(&mut s, Home, 1);
    assert_eq!(s.point_label(Home), "AD");
    points(&mut s, Away, 1);
    assert_eq!(s.point_label(Home), "40");
    points(&mut s, Home, 2);
    assert_eq!(s.games, [1, 0]);
    assert_eq!(s.server, Away);
    for _ in 0..5 {
        game(&mut s, Away);
        game(&mut s, Home);
    }
    game(&mut s, Away);
    assert!(s.tiebreak);
    let first = s.server;
    points(&mut s, Home, 1);
    assert_ne!(s.server, first);
    points(&mut s, Home, 1);
    assert_ne!(s.server, first);
    points(&mut s, Home, 1);
    assert_eq!(s.server, first);
    points(&mut s, Home, 4);
    assert_eq!(s.completed_units[0].score, [7, 6]);
    assert_eq!(s.server, if first == Home { Away } else { Home });
    let mut r = Rules::defaults(Sport::Tennis);
    r.scoring = Scoring::NoAd;
    let mut no_ad = State::new(Sport::Tennis, r).unwrap();
    points(&mut no_ad, Home, 3);
    points(&mut no_ad, Away, 3);
    points(&mut no_ad, Away, 1);
    assert_eq!(no_ad.games, [0, 1]);
}

#[test]
fn padel_star_point_and_match_tiebreak() {
    let mut r = Rules::defaults(Sport::Padel);
    r.scoring = Scoring::StarPoint;
    let mut s = State::new(Sport::Padel, r).unwrap();
    points(&mut s, Home, 3);
    points(&mut s, Away, 3);
    for _ in 0..2 {
        points(&mut s, Home, 1);
        points(&mut s, Away, 1);
    }
    points(&mut s, Away, 1);
    assert_eq!(s.games, [0, 1]);
    let mut r = Rules::defaults(Sport::Padel);
    r.match_tiebreak = true;
    let mut s = State::new(Sport::Padel, r).unwrap();
    for _ in 0..6 {
        game(&mut s, Home);
    }
    for _ in 0..6 {
        game(&mut s, Away);
    }
    assert!(s.tiebreak);
    points(&mut s, Home, 9);
    points(&mut s, Away, 9);
    points(&mut s, Home, 2);
    assert_eq!(s.winner, Some(Home));
    assert_eq!(s.completed_units[2].tiebreak_points, Some([11, 9]));
}

#[test]
fn rules_validation_early_complete_and_serialized_replay_are_deterministic() {
    for sport in Sport::ALL.into_iter().filter(|s| *s != Sport::Basketball) {
        let mut s = state(sport);
        assert!(s.apply(Command::Complete).is_err());
        let commands = [
            Command::SetServer(Away),
            Command::Rally(Home),
            Command::Rally(Away),
            Command::Rally(Home),
        ];
        for command in &commands {
            s.apply(*command).unwrap();
        }
        let json = serde_json::to_string(&s).unwrap();
        let restored: State = serde_json::from_str(&json).unwrap();
        assert_eq!(s, restored);
        let mut replay = state(sport);
        for command in commands {
            replay.apply(command).unwrap();
        }
        assert_eq!(s, replay);
        let mut bad = Rules::defaults(sport);
        bad.best_of = 2;
        assert!(State::new(sport, bad).is_err());
    }
}

#[test]
fn table_tennis_doubles_decider_changes_receivers_without_changing_server() {
    let mut r = Rules::defaults(Sport::TableTennis);
    r.format = Format::Doubles;
    r.best_of = 1;
    let mut s = State::new(Sport::TableTennis, r).unwrap();
    points(&mut s, Home, 4);
    assert_eq!((s.server, s.server_slot, s.receiver_slot), (Home, 2, 2));
    points(&mut s, Home, 1);
    assert_eq!((s.server, s.server_slot, s.receiver_slot), (Home, 2, 1));
    points(&mut s, Away, 1);
    assert_eq!((s.server, s.server_slot, s.receiver_slot), (Away, 1, 1));
    assert_eq!(s.ends_changed, 1);
}

#[test]
fn badminton_doubles_keeps_winning_server_and_parity_after_service_loss() {
    let mut s = state(Sport::Badminton);
    points(&mut s, Home, 1);
    assert_eq!((s.server, s.server_slot, s.receiver_slot), (Home, 1, 2));
    points(&mut s, Away, 1);
    assert_eq!((s.server, s.server_slot, s.receiver_slot), (Away, 2, 1));
    points(&mut s, Away, 1);
    assert_eq!((s.server, s.server_slot, s.receiver_slot), (Away, 2, 2));
    points(&mut s, Home, 1);
    assert_eq!((s.server, s.server_slot, s.receiver_slot), (Home, 2, 2));
}

#[test]
fn undo_by_replay_restores_every_sport_before_final_point() {
    for sport in Sport::ALL.into_iter().filter(|s| *s != Sport::Basketball) {
        let mut s = state(sport);
        let mut events = Vec::new();
        while s.status == Status::Live {
            events.push(Command::Rally(Home));
            s.apply(Command::Rally(Home)).unwrap();
        }
        let decided = s.clone();
        events.pop();
        let mut restored = state(sport);
        for event in events {
            restored.apply(event).unwrap();
        }
        assert_eq!(restored.status, Status::Live);
        assert_eq!(restored.winner, None);
        restored.apply(Command::Rally(Home)).unwrap();
        assert_eq!(restored, decided);
        let snapshot = restored.clone();
        assert!(restored.apply(Command::SetServer(Away)).is_err());
        assert_eq!(restored, snapshot);
    }
}

#[test]
fn pickleball_server_keeps_member_slot_after_reclaiming_serve_at_odd_score() {
    let mut s = state(Sport::Pickleball);
    points(&mut s, Home, 1);
    points(&mut s, Away, 1);
    points(&mut s, Home, 2);
    assert_eq!(s.server, Home);
    let server = s.server_slot;
    points(&mut s, Home, 1);
    assert_eq!(s.server_slot, server);
}

#[test]
fn table_tennis_server_correction_preserves_deuce_service_cadence() {
    let mut s = state(Sport::TableTennis);
    points(&mut s, Home, 9);
    points(&mut s, Away, 9);
    s.apply(Command::SetServer(Home)).unwrap();
    points(&mut s, Home, 1);
    assert_eq!(s.server, Home);
    points(&mut s, Away, 1);
    assert_eq!(s.server, Away);
    points(&mut s, Home, 1);
    assert_eq!(s.server, Home);
}
