use hustlx_oss::{
    Side::{Away, Home},
    operator::PlayerAction as A,
    session::Session,
    sport::{Command as C, Rules, Scoring, Sport, State, Status},
    storage::{Error, SportMatchDraft, Store},
};
use rusqlite::Connection;
use uuid::Uuid;

const RACQUETS: [Sport; 5] = [
    Sport::Pickleball,
    Sport::Padel,
    Sport::Tennis,
    Sport::Badminton,
    Sport::TableTennis,
];

fn action(side: hustlx_oss::Side, player: i64, action: A) -> C {
    C::Player {
        side,
        player: Some(player),
        action,
    }
}

#[test]
fn every_racquet_attributes_winners_and_errors_without_accepting_volleyball_actions() {
    for sport in RACQUETS {
        let mut rules = Rules::defaults(sport);
        if sport == Sport::Pickleball {
            rules.scoring = Scoring::Rally;
        }
        let error = if matches!(sport, Sport::Tennis | Sport::Padel) {
            A::DoubleFault
        } else {
            A::ServiceError
        };
        for (kind, score) in [
            (A::Winner, [1, 0]),
            (A::Ace, [1, 0]),
            (error, [0, 1]),
            (A::UnforcedError, [0, 1]),
            (A::ForcedError, [0, 1]),
        ] {
            let mut s = State::new(sport, rules.clone()).unwrap();
            s.apply(action(Home, 1, kind)).unwrap();
            assert_eq!(s.points, score, "{sport:?} {kind:?}");
            assert_eq!(s.operator.count(Home, 1, kind), 1);
            assert_eq!(s.operator.count(Away, 1, kind), 0);
            let before = s.clone();
            for rejected in [
                action(Home, 1, A::Kill),
                C::Rotate(Home),
                C::Exchange {
                    side: Home,
                    outgoing: 1,
                    incoming: 2,
                    libero: true,
                },
            ] {
                assert!(s.apply(rejected).is_err(), "{sport:?}");
                assert_eq!(s, before);
            }
        }
    }
}

#[test]
fn tennis_and_padel_double_faults_resolve_deuce_and_count_for_the_faulting_player() {
    for sport in [Sport::Tennis, Sport::Padel] {
        let mut s = State::new(sport, Rules::defaults(sport)).unwrap();
        for side in [Home, Away] {
            for _ in 0..3 {
                s.apply(C::Rally(side)).unwrap();
            }
        }
        s.apply(action(Home, 1, A::DoubleFault)).unwrap();
        assert_eq!(s.point_label(Away), "AD");
        s.apply(action(Home, 1, A::DoubleFault)).unwrap();
        assert_eq!((s.games, s.points, s.server), ([0, 1], [0, 0], Away));
        assert_eq!(s.operator.count(Home, 1, A::DoubleFault), 2);
        assert_eq!(s.operator.count(Away, 1, A::DoubleFault), 0);
    }
}

#[test]
fn badminton_attributed_cap_point_and_table_tennis_deuce_keep_service_and_unit_boundaries() {
    let mut b = State::new(Sport::Badminton, Rules::defaults(Sport::Badminton)).unwrap();
    for _ in 0..29 {
        b.apply(C::Rally(Home)).unwrap();
        b.apply(C::Rally(Away)).unwrap();
    }
    b.apply(action(Away, 2, A::ServiceError)).unwrap();
    assert_eq!(b.completed_units[0].score, [30, 29]);
    assert_eq!(b.points, [0, 0]);
    assert_eq!(b.operator.count(Away, 2, A::ServiceError), 1);
    let mut t = State::new(Sport::TableTennis, Rules::defaults(Sport::TableTennis)).unwrap();
    for side in [Home, Away] {
        for _ in 0..10 {
            t.apply(C::Rally(side)).unwrap();
        }
    }
    t.apply(action(Home, 1, A::Winner)).unwrap();
    assert_eq!((t.points, t.server), ([11, 10], Away));
    t.apply(action(Away, 2, A::UnforcedError)).unwrap();
    assert_eq!(t.completed_units[0].score, [12, 10]);
    assert_eq!((t.points, t.server), ([0, 0], Away));
}

#[test]
fn every_racquet_preserves_actor_snapshots_exact_receipts_and_exchange_undo_on_disk() {
    for sport in RACQUETS {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("racquet.sqlite3");
        let mut db = Store::open(&path).unwrap();
        let home = db.create_team("Same club").unwrap();
        let away = db.create_team("Same club").unwrap();
        let a = db.create_player("Alex").unwrap();
        let b = db.create_player("Alex").unwrap();
        let bench = db.create_player("Bench").unwrap();
        db.add_to_team(home, a, "7").unwrap();
        db.add_to_team(home, bench, "8").unwrap();
        db.add_to_team(away, b, "7").unwrap();
        let mut saved = db
            .create_sport_match_with_teams(
                &SportMatchDraft {
                    name: format!("{sport} identities"),
                    date: "2026-10-21".into(),
                    venue: String::new(),
                    home_name: "Same club".into(),
                    away_name: "Same club".into(),
                    sport,
                    rules: Rules::defaults(sport),
                },
                [Some(home), Some(away)],
            )
            .unwrap();
        let id = saved.summary.id;
        let mut players = [0; 11];
        players[0] = a;
        saved = db
            .record_sport(
                id,
                0,
                Uuid::new_v4(),
                C::Lineup {
                    side: Home,
                    players,
                },
            )
            .unwrap();
        let stamp = Uuid::new_v4();
        let command = action(Home, a, A::Winner);
        saved = db
            .record_sport(id, saved.summary.revision, stamp, command)
            .unwrap();
        assert_eq!(
            saved
                .sport_state
                .as_ref()
                .unwrap()
                .operator
                .count(Home, a, A::Winner),
            1
        );
        assert_eq!(
            db.record_sport(id, 1, stamp, command)
                .unwrap()
                .summary
                .revision,
            2
        );
        assert!(
            db.record_sport(id, 2, stamp, action(Away, b, A::Winner))
                .is_err()
        );
        assert!(matches!(
            db.record_sport(id, 1, Uuid::new_v4(), command),
            Err(Error::Conflict)
        ));
        assert!(
            db.record_sport(id, 2, Uuid::new_v4(), action(Home, b, A::Winner))
                .is_err()
        );
        assert_eq!(db.load_match(id).unwrap().summary.revision, 2);
        saved = db
            .record_sport(
                id,
                2,
                Uuid::new_v4(),
                C::Exchange {
                    side: Home,
                    outgoing: a,
                    incoming: bench,
                    libero: false,
                },
            )
            .unwrap();
        assert_eq!(
            saved.sport_state.as_ref().unwrap().operator.court[0],
            [bench]
        );
        db.rename_player(a, "Renamed").unwrap();
        drop(db);
        let mut db = Store::open(&path).unwrap();
        let restored = db.load_match(id).unwrap();
        assert_eq!(restored.game.player(Home, a).unwrap().name, "Alex");
        assert!(restored.game.player(Home, b).is_none());
        assert_eq!(restored.sport_state, saved.sport_state);
        saved = db.undo_sport(id, 3, Uuid::new_v4()).unwrap();
        assert_eq!(saved.sport_state.as_ref().unwrap().operator.court[0], [a]);
        saved = db.undo_sport(id, 4, Uuid::new_v4()).unwrap();
        let state = saved.sport_state.unwrap();
        assert_eq!(state.points, [0, 0]);
        assert_eq!(state.operator.count(Home, a, A::Winner), 0);
        assert_eq!(state.status, Status::Live);
    }
}

#[test]
fn every_racquet_failed_attributed_save_retries_once_and_undo_survives_reopen() {
    for sport in RACQUETS {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("retry.sqlite3");
        let db = Store::open(&path).unwrap();
        let team = db.create_team("Home").unwrap();
        let player = db.create_player("Alex").unwrap();
        db.add_to_team(team, player, "1").unwrap();
        let mut s = Session::new(db);
        s.create_sport_with_teams(
            &SportMatchDraft {
                name: format!("{sport} retry"),
                date: "2026-10-21".into(),
                venue: String::new(),
                home_name: "Home".into(),
                away_name: "Away".into(),
                sport,
                rules: Rules::defaults(sport),
            },
            [Some(team), None],
        )
        .unwrap();
        let id = s.active().unwrap().summary.id;
        let raw = Connection::open(&path).unwrap();
        raw.execute_batch("CREATE TRIGGER reject_action BEFORE INSERT ON sport_events BEGIN SELECT RAISE(ABORT,'test-only failure'); END;").unwrap();
        assert!(s.sport_command(action(Home, player, A::Winner)).is_err());
        assert!(s.has_pending_save());
        assert_eq!(
            s.active().unwrap().sport_state.as_ref().unwrap().points,
            [0, 0]
        );
        assert!(s.sport_command(action(Home, player, A::Ace)).is_err());
        assert!(s.pause_for_close_at(std::time::Instant::now()).is_err());
        raw.execute_batch("DROP TRIGGER reject_action").unwrap();
        s.retry().unwrap();
        assert_eq!(s.active().unwrap().sport_history.len(), 1);
        assert_eq!(
            s.active()
                .unwrap()
                .sport_state
                .as_ref()
                .unwrap()
                .operator
                .count(Home, player, A::Winner),
            1
        );
        s.undo().unwrap();
        drop(s);
        let mut s = Session::new(Store::open(&path).unwrap());
        s.open(id).unwrap();
        assert_eq!(
            s.active().unwrap().sport_state.as_ref().unwrap().points,
            [0, 0]
        );
        assert_eq!(
            s.active()
                .unwrap()
                .sport_history
                .iter()
                .filter(|e| e.undone)
                .count(),
            1
        );
        assert!(!s.has_pending_save());
    }
}
