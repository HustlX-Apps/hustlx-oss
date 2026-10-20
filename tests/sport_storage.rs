use hustlx_oss::{
    Side, Stat,
    clock::ClockState,
    session::Session,
    sport::{Command, Rules, Sport, Status},
    storage::{Error, MatchDraft, SportMatchDraft, Store},
};
use rusqlite::Connection;
use tempfile::tempdir;
use uuid::Uuid;

fn draft(sport: Sport) -> SportMatchDraft {
    SportMatchDraft {
        name: "Local final".into(),
        date: "2026-10-20".into(),
        venue: "Court".into(),
        home_name: "Alex".into(),
        away_name: "Alex".into(),
        sport,
        rules: Rules::defaults(sport),
    }
}

#[test]
fn all_six_sports_score_finish_undo_and_reopen_with_original_rules() {
    for sport in Sport::ALL.into_iter().filter(|s| *s != Sport::Basketball) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("match.sqlite3");
        let mut db = Store::open(&path).unwrap();
        let mut saved = db.create_sport_match(&draft(sport)).unwrap();
        let id = saved.summary.id;
        assert_eq!(saved.summary.sport, sport);
        assert_eq!(
            db.teams(false).unwrap().len(),
            2,
            "same names remain separate identities"
        );
        let rules = serde_json::to_value(saved.sport_state.as_ref().unwrap().rules()).unwrap();
        for _ in 0..1000 {
            if saved.sport_state.as_ref().unwrap().status == Status::Decided {
                break;
            }
            saved = db
                .record_sport(
                    id,
                    saved.summary.revision,
                    Uuid::new_v4(),
                    Command::Rally(Side::Home),
                )
                .unwrap();
        }
        assert_eq!(
            saved.sport_state.as_ref().unwrap().status,
            Status::Decided,
            "{sport:?}"
        );
        saved = db
            .record_sport(
                id,
                saved.summary.revision,
                Uuid::new_v4(),
                Command::Complete,
            )
            .unwrap();
        assert_eq!(
            saved.sport_state.as_ref().unwrap().status,
            Status::Completed
        );
        let expected = serde_json::to_value(saved.sport_state.as_ref().unwrap()).unwrap();
        drop(db);
        let mut db = Store::open(&path).unwrap();
        saved = db.load_match(id).unwrap();
        assert_eq!(
            serde_json::to_value(saved.sport_state.as_ref().unwrap()).unwrap(),
            expected
        );
        saved = db
            .undo_sport(id, saved.summary.revision, Uuid::new_v4())
            .unwrap();
        assert_eq!(saved.sport_state.as_ref().unwrap().status, Status::Decided);
        saved = db
            .undo_sport(id, saved.summary.revision, Uuid::new_v4())
            .unwrap();
        assert_eq!(saved.sport_state.as_ref().unwrap().status, Status::Live);
        assert_eq!(
            serde_json::to_value(saved.sport_state.as_ref().unwrap().rules()).unwrap(),
            rules
        );
        assert_eq!(saved.sport_history.iter().filter(|e| e.undone).count(), 2);
        let expected = serde_json::to_value(saved.sport_state.as_ref().unwrap()).unwrap();
        drop(db);
        assert_eq!(
            serde_json::to_value(
                Store::open(&path)
                    .unwrap()
                    .load_match(id)
                    .unwrap()
                    .sport_state
                    .unwrap()
            )
            .unwrap(),
            expected
        );
    }
}

#[test]
fn sport_receipts_are_idempotent_revision_checked_and_family_isolated() {
    let mut db = Store::open(":memory:").unwrap();
    let saved = db.create_sport_match(&draft(Sport::Volleyball)).unwrap();
    let id = saved.summary.id;
    let command = Uuid::new_v4();
    let saved = db
        .record_sport(id, 0, command, Command::Rally(Side::Home))
        .unwrap();
    assert_eq!(saved.summary.revision, 1);
    assert_eq!(
        db.record_sport(id, 0, command, Command::Rally(Side::Home))
            .unwrap()
            .summary
            .revision,
        1
    );
    assert!(
        db.record_sport(id, 1, command, Command::Rally(Side::Away))
            .is_err()
    );
    assert!(matches!(
        db.record_sport(id, 0, Uuid::new_v4(), Command::Rally(Side::Away)),
        Err(Error::Conflict)
    ));
    assert!(
        db.save_clock(id, 1, command, ClockState::default())
            .is_err()
    );
    assert!(
        db.save_clock(id, 1, Uuid::new_v4(), ClockState::default())
            .is_err()
    );
    assert!(
        db.record(id, 1, Uuid::new_v4(), Side::Home, 1, Stat::TwoMade)
            .is_err()
    );
    let undo = Uuid::new_v4();
    assert_eq!(db.undo_sport(id, 1, undo).unwrap().summary.revision, 2);
    db.record_sport(id, 2, Uuid::new_v4(), Command::Rally(Side::Away))
        .unwrap();
    assert_eq!(db.undo_sport(id, 1, undo).unwrap().summary.revision, 3);
    assert_eq!(
        db.load_match(id).unwrap().sport_state.unwrap().points,
        [0, 1]
    );
}

#[test]
fn failed_sport_commit_keeps_visible_state_and_retries_same_intent_once() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("match.sqlite3");
    let mut session = Session::new(Store::open(&path).unwrap());
    session.create_sport(&draft(Sport::TableTennis)).unwrap();
    let db = Connection::open(&path).unwrap();
    db.execute_batch("CREATE TRIGGER fail_sport BEFORE INSERT ON sport_events BEGIN SELECT RAISE(ABORT,'test failure'); END;").unwrap();
    assert!(session.sport_command(Command::Rally(Side::Away)).is_err());
    assert!(session.has_pending_save());
    assert_eq!(
        session
            .active()
            .unwrap()
            .sport_state
            .as_ref()
            .unwrap()
            .points,
        [0, 0]
    );
    assert!(
        session
            .pause_for_close_at(std::time::Instant::now())
            .is_err()
    );
    db.execute_batch("DROP TRIGGER fail_sport;").unwrap();
    session.retry().unwrap();
    assert!(!session.has_pending_save());
    assert_eq!(
        session
            .active()
            .unwrap()
            .sport_state
            .as_ref()
            .unwrap()
            .points,
        [0, 1]
    );
    assert_eq!(session.active().unwrap().summary.revision, 1);
    session.undo().unwrap();
    assert_eq!(
        session
            .active()
            .unwrap()
            .sport_state
            .as_ref()
            .unwrap()
            .points,
        [0, 0]
    );
}

#[test]
fn version_three_basketball_migration_preserves_history_clock_and_identity() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("legacy.sqlite3");
    let db = Connection::open(&path).unwrap();
    db.execute_batch(include_str!("../src/schema.sql")).unwrap();
    db.execute_batch(include_str!("../src/migration_v2.sql"))
        .unwrap();
    db.execute_batch(include_str!("../src/migration_v3.sql"))
        .unwrap();
    db.execute_batch("PRAGMA application_id=1213747027; PRAGMA user_version=3;
    INSERT INTO teams(id,name) VALUES(1,'Home'),(2,'Away');
    INSERT INTO players(id,name) VALUES(7,'Taylor');
    INSERT INTO matches(id,name,date,home_team_id,away_team_id,home_name,away_name,revision) VALUES(1,'Legacy','2026-10-19',1,2,'Home','Away',1);
    INSERT INTO match_players(match_id,side,player_id,name,jersey,on_court,sort_order) VALUES(1,'home',7,'Taylor','9',1,0);
    INSERT INTO match_clocks(match_id,game_ms,shot_ms,period) VALUES(1,123000,14000,3);
    INSERT INTO match_events(match_id,command_id,kind,side,player_id,stat) VALUES(1,'old-command','record','home',7,'three_made');").unwrap();
    drop(db);
    let mut store = Store::open(&path).unwrap();
    let saved = store.load_match(1).unwrap();
    assert_eq!(saved.summary.sport, Sport::Basketball);
    assert!(saved.sport_state.is_none());
    assert_eq!(saved.game.score(Side::Home), 3);
    assert_eq!(saved.clock.game_ms, 123000);
    assert_eq!(saved.game.court_ids(Side::Home), vec![7]);
    store.undo(1, 1, Uuid::new_v4()).unwrap();
    assert_eq!(store.load_match(1).unwrap().game.score(Side::Home), 0);
}

#[test]
fn basketball_and_sport_commands_cannot_share_a_uuid() {
    let mut db = Store::open(":memory:").unwrap();
    let a = db.create_team("A").unwrap();
    let b = db.create_team("B").unwrap();
    let p = db.create_player("P").unwrap();
    db.add_to_team(a, p, "").unwrap();
    let basketball = db
        .create_match(&MatchDraft {
            name: "Basketball".into(),
            date: "2026-10-20".into(),
            venue: "".into(),
            home_team_id: a,
            away_team_id: b,
        })
        .unwrap();
    let sport = db.create_sport_match(&draft(Sport::Badminton)).unwrap();
    let key = Uuid::new_v4();
    db.record(basketball.summary.id, 0, key, Side::Home, p, Stat::TwoMade)
        .unwrap();
    assert!(
        db.record_sport(sport.summary.id, 0, key, Command::Rally(Side::Home))
            .is_err()
    );
    let key = Uuid::new_v4();
    db.record_sport(sport.summary.id, 0, key, Command::Rally(Side::Home))
        .unwrap();
    assert!(
        db.record(basketball.summary.id, 1, key, Side::Home, p, Stat::TwoMade)
            .is_err()
    );
    assert!(
        db.save_clock(basketball.summary.id, 1, key, ClockState::default())
            .is_err()
    );
}

#[test]
fn clocks_lineups_and_sport_commands_reject_collisions_in_both_directions() {
    use hustlx_oss::lineup::LineupKind;
    let mut db = Store::open(":memory:").unwrap();
    let a = db.create_team("A").unwrap();
    let b = db.create_team("B").unwrap();
    let p = db.create_player("P").unwrap();
    db.add_to_team(a, p, "").unwrap();
    let basketball = db
        .create_match(&MatchDraft {
            name: "Basketball".into(),
            date: "2026-10-20".into(),
            venue: "".into(),
            home_team_id: a,
            away_team_id: b,
        })
        .unwrap();
    let bid = basketball.summary.id;
    let sport = db.create_sport_match(&draft(Sport::Pickleball)).unwrap();
    let sid = sport.summary.id;
    let clock_key = Uuid::new_v4();
    db.save_clock(bid, 0, clock_key, ClockState::default())
        .unwrap();
    assert!(
        db.record_sport(sid, 0, clock_key, Command::Rally(Side::Home))
            .is_err()
    );
    let lineup_key = Uuid::new_v4();
    db.save_lineup(
        bid,
        1,
        lineup_key,
        Side::Home,
        LineupKind::Starters,
        &[p],
        ClockState::default(),
    )
    .unwrap();
    assert!(
        db.record_sport(sid, 0, lineup_key, Command::Rally(Side::Home))
            .is_err()
    );
    let sport_key = Uuid::new_v4();
    db.record_sport(sid, 0, sport_key, Command::Rally(Side::Home))
        .unwrap();
    assert!(
        db.save_lineup(
            bid,
            2,
            sport_key,
            Side::Home,
            LineupKind::Starters,
            &[p],
            ClockState::default()
        )
        .is_err()
    );
    assert!(
        db.save_lineup(
            sid,
            1,
            Uuid::new_v4(),
            Side::Home,
            LineupKind::Starters,
            &[],
            ClockState::default()
        )
        .is_err()
    );
    assert!(db.undo_sport(sid, 1, lineup_key).is_err());
    assert!(db.undo(bid, 2, sport_key).is_err());
    assert!(
        db.record_sport(bid, 2, Uuid::new_v4(), Command::Rally(Side::Home))
            .is_err()
    );
    assert_eq!(db.load_match(bid).unwrap().summary.revision, 2);
    assert_eq!(db.load_match(sid).unwrap().summary.revision, 1);
}

#[test]
fn invalid_sport_intent_does_not_create_pending_save_and_server_correction_is_undoable() {
    use hustlx_oss::{clock::ClockCommand, lineup::LineupKind};
    let mut session = Session::new(Store::open(":memory:").unwrap());
    session.create_sport(&draft(Sport::Tennis)).unwrap();
    assert!(session.sport_command(Command::Complete).is_err());
    assert!(!session.has_pending_save());
    assert!(session.undo().is_err());
    assert!(!session.has_pending_save());
    assert!(
        session
            .command_at(ClockCommand::Start, std::time::Instant::now())
            .is_err()
    );
    assert!(session.record(Stat::TwoMade).is_err());
    assert!(
        session
            .save_lineup_at(
                Side::Home,
                LineupKind::Starters,
                vec![],
                std::time::Instant::now()
            )
            .is_err()
    );
    assert!(!session.has_pending_save());
    session
        .sport_command(Command::SetServer(Side::Away))
        .unwrap();
    assert_eq!(
        session
            .active()
            .unwrap()
            .sport_state
            .as_ref()
            .unwrap()
            .server,
        Side::Away
    );
    session.undo().unwrap();
    assert_eq!(
        session
            .active()
            .unwrap()
            .sport_state
            .as_ref()
            .unwrap()
            .server,
        Side::Home
    );
    session
        .pause_for_close_at(std::time::Instant::now())
        .unwrap();
}

#[test]
fn creation_failure_rolls_back_named_identities_and_match_metadata() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("local.sqlite3");
    let mut db = Store::open(&path).unwrap();
    let raw = Connection::open(&path).unwrap();
    raw.execute_batch("CREATE TRIGGER fail_rules BEFORE INSERT ON sport_matches BEGIN SELECT RAISE(ABORT,'test failure'); END;").unwrap();
    assert!(db.create_sport_match(&draft(Sport::Padel)).is_err());
    assert!(db.teams(true).unwrap().is_empty());
    assert!(db.matches().unwrap().is_empty());
    raw.execute_batch("DROP TRIGGER fail_rules;").unwrap();
    let mut invalid = draft(Sport::Badminton);
    invalid.rules.best_of = 0;
    assert!(db.create_sport_match(&invalid).is_err());
    assert!(db.teams(true).unwrap().is_empty());
    db.create_sport_match(&draft(Sport::Padel)).unwrap();
    assert_eq!(db.teams(true).unwrap().len(), 2);
}

#[test]
fn migration_failure_is_atomic_and_preserves_the_version_three_library() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("legacy.sqlite3");
    let raw = Connection::open(&path).unwrap();
    raw.execute_batch(include_str!("../src/schema.sql"))
        .unwrap();
    raw.execute_batch(include_str!("../src/migration_v2.sql"))
        .unwrap();
    raw.execute_batch(include_str!("../src/migration_v3.sql"))
        .unwrap();
    raw.pragma_update(None, "application_id", 0x48584f53_i64)
        .unwrap();
    raw.pragma_update(None, "user_version", 3).unwrap();
    raw.execute_batch(
        "INSERT INTO teams(name) VALUES('Preserve'); CREATE TABLE sport_events(existing TEXT);",
    )
    .unwrap();
    assert!(Store::open(&path).is_err());
    assert_eq!(
        raw.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        3
    );
    let sports_column: i64 = raw
        .query_row(
            "SELECT count(*) FROM pragma_table_info('matches') WHERE name='sport'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(sports_column, 0);
    assert_eq!(
        raw.query_row("SELECT name FROM teams", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "Preserve"
    );
}

#[test]
fn unknown_saved_rules_fail_visibly_without_modifying_committed_state() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("local.sqlite3");
    let mut db = Store::open(&path).unwrap();
    let id = db
        .create_sport_match(&draft(Sport::Volleyball))
        .unwrap()
        .summary
        .id;
    db.record_sport(id, 0, Uuid::new_v4(), Command::Rally(Side::Away))
        .unwrap();
    let raw = Connection::open(&path).unwrap();
    raw.execute(
        "UPDATE sport_matches SET rules_json=?1 WHERE match_id=?2",
        rusqlite::params!["{}", id],
    )
    .unwrap();
    assert!(db.load_match(id).is_err());
    assert!(
        db.record_sport(id, 1, Uuid::new_v4(), Command::Rally(Side::Home))
            .is_err()
    );
    assert_eq!(
        raw.query_row("SELECT revision FROM matches WHERE id=?1", [id], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        raw.query_row(
            "SELECT count(*) FROM sport_events WHERE match_id=?1",
            [id],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
}
