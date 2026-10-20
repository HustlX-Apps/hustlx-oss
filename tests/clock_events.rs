use hustlx_oss::{
    Side, Stat,
    clock::{ClockCommand as C, ClockState},
    session::Session,
    storage::{Error, MatchDraft, Store},
};
use rusqlite::Connection;
use std::time::{Duration, Instant};
use tempfile::tempdir;
use uuid::Uuid;

fn fixture(db: &mut Store) -> (i64, i64) {
    let home = db.create_team("Home").unwrap();
    let away = db.create_team("Away").unwrap();
    let player = db.create_player("Alex").unwrap();
    db.add_to_team(home, player, "7").unwrap();
    let saved = db
        .create_match(&MatchDraft {
            name: "Clock test".into(),
            date: "2026-10-20".into(),
            venue: "Gym".into(),
            home_team_id: home,
            away_team_id: away,
        })
        .unwrap();
    (saved.summary.id, player)
}

#[test]
fn schema_four_upgrade_preserves_sport_rules_history_and_legacy_basketball() {
    use hustlx_oss::sport::{Command, Rules, Sport};
    let dir = tempdir().unwrap();
    let path = dir.path().join("multi-sport-v4.sqlite3");
    let raw = Connection::open(&path).unwrap();
    for schema in [
        include_str!("../src/schema.sql"),
        include_str!("../src/migration_v2.sql"),
        include_str!("../src/migration_v3.sql"),
        include_str!("../src/migration_v4.sql"),
    ] {
        raw.execute_batch(schema).unwrap();
    }
    raw.pragma_update(None, "application_id", 0x48584f53)
        .unwrap();
    raw.pragma_update(None, "user_version", 4).unwrap();
    raw.execute_batch("INSERT INTO teams(name) VALUES('Home'),('Away');
        INSERT INTO players(name) VALUES('Alex');
        INSERT INTO matches(id,name,date,home_team_id,away_team_id,home_name,away_name,revision,sport) VALUES
            (1,'Basketball','2026-10-20',1,2,'Home','Away',1,'basketball'),
            (2,'Volleyball','2026-10-20',1,2,'Home','Away',3,'volleyball');
        INSERT INTO match_players(match_id,side,player_id,name,jersey,on_court,sort_order) VALUES(1,'home',1,'Alex','7',1,0);
        INSERT INTO match_clocks(match_id,period,game_ms,shot_ms) VALUES(1,2,310125,4750),(2,1,600000,24000);
        INSERT INTO match_events(id,match_id,command_id,kind,side,player_id,stat) VALUES(1,1,'old-basketball','record','home',1,'three_made');").unwrap();
    let rules = serde_json::to_string(&Rules::defaults(Sport::Volleyball)).unwrap();
    let rally = serde_json::to_string(&Command::Rally(Side::Home)).unwrap();
    let local_id = Uuid::new_v4().to_string();
    raw.execute(
        "INSERT INTO sport_matches(match_id,local_id,rules_version,rules_json) VALUES(2,?1,1,?2)",
        rusqlite::params![local_id, rules],
    )
    .unwrap();
    raw.execute("INSERT INTO sport_events(id,match_id,command_id,payload_version,kind,payload) VALUES(1,2,'rally-one',1,'record',?1),(2,2,'rally-two',1,'record',?1)", [&rally]).unwrap();
    raw.execute("INSERT INTO sport_events(id,match_id,command_id,payload_version,kind,target_id) VALUES(3,2,'undo-two',1,'undo',2)", []).unwrap();
    drop(raw);

    let mut db = Store::open(&path).unwrap();
    let basketball = db.load_match(1).unwrap();
    assert_eq!(basketball.game.score(Side::Home), 3);
    assert_eq!(basketball.game.plays()[0].clock, None);
    assert_eq!(basketball.summary.revision, 1);
    let volleyball = db.load_match(2).unwrap();
    assert_eq!(volleyball.summary.sport, Sport::Volleyball);
    assert_eq!(volleyball.summary.revision, 3);
    assert_eq!(volleyball.sport_history.len(), 3);
    assert!(!volleyball.sport_history[0].undone);
    assert!(volleyball.sport_history[1].undone);
    let state = serde_json::to_value(&volleyball.sport_state).unwrap();
    let stamped = db
        .record(1, 1, Uuid::new_v4(), Side::Home, 1, Stat::Assist)
        .unwrap();
    assert_eq!(stamped.game.plays()[0].clock, None);
    assert_eq!(stamped.game.plays()[1].clock, Some(basketball.clock));
    drop(db);
    let db = Store::open(&path).unwrap();
    assert_eq!(
        serde_json::to_value(db.load_match(2).unwrap().sport_state).unwrap(),
        state
    );
    let raw = Connection::open(path).unwrap();
    assert_eq!(
        raw.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        5
    );
    let stored: (String, String) = raw
        .query_row(
            "SELECT local_id,rules_json FROM sport_matches WHERE match_id=2",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(stored, (local_id, rules));
    assert_eq!(
        raw.query_row("SELECT payload FROM sport_events WHERE id=1", [], |r| {
            r.get::<_, String>(0)
        })
        .unwrap(),
        rally
    );
    assert_eq!(
        raw.query_row("SELECT target_id FROM sport_events WHERE id=3", [], |r| r
            .get::<_, i64>(
            0
        ))
        .unwrap(),
        2
    );
}

#[test]
fn live_events_capture_exact_pre_action_clocks_before_resets_and_foul_pause() {
    let mut db = Store::open(":memory:").unwrap();
    let (id, player) = fixture(&mut db);
    let mut session = Session::new(db);
    session.open(id).unwrap();
    let t = Instant::now();
    session.command_at(C::Start, t).unwrap();
    session.select(Side::Home, player);
    session
        .record_at(Stat::TwoMade, t + Duration::from_millis(1250))
        .unwrap();
    let saved = session.active().unwrap();
    assert_eq!(
        saved.game.plays()[0].clock,
        Some(ClockState {
            game_ms: 598750,
            shot_ms: 22750,
            period: 1,
            running: true,
            shot_running: true,
        })
    );
    assert_eq!(saved.clock.shot_ms, 24000);
    session.select(Side::Home, player);
    session
        .record_at(Stat::OffensiveRebound, t + Duration::from_millis(1500))
        .unwrap();
    assert_eq!(session.active().unwrap().clock.shot_ms, 14000);
    assert_eq!(
        session.active().unwrap().game.plays()[1]
            .clock
            .unwrap()
            .shot_ms,
        23750
    );
    session.select(Side::Home, player);
    session
        .record_at(Stat::Foul, t + Duration::from_millis(1750))
        .unwrap();
    let saved = session.active().unwrap();
    let stamp = saved.game.plays()[2].clock.unwrap();
    assert_eq!((stamp.game_ms, stamp.shot_ms), (598250, 13750));
    assert!(stamp.running && stamp.shot_running);
    assert!(!saved.clock.running);
}

#[test]
fn stamps_survive_clock_corrections_period_changes_undo_and_reopen() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("events.sqlite3");
    let mut db = Store::open(&path).unwrap();
    let (id, player) = fixture(&mut db);
    let stamp = ClockState {
        period: 2,
        game_ms: 310125,
        shot_ms: 4750,
        running: false,
        shot_running: false,
    };
    let first = Uuid::new_v4();
    let saved = db
        .record_with_clock(
            id,
            0,
            first,
            Side::Home,
            player,
            Stat::ThreeMade,
            Some(stamp),
        )
        .unwrap();
    let corrected = stamp.command(C::NextPeriod).unwrap();
    let saved = db
        .save_clock(id, saved.summary.revision, Uuid::new_v4(), corrected)
        .unwrap();
    let saved = db
        .record(
            id,
            saved.summary.revision,
            Uuid::new_v4(),
            Side::Home,
            player,
            Stat::Assist,
        )
        .unwrap();
    assert_eq!(saved.game.plays()[0].clock, Some(stamp));
    assert_eq!(saved.game.plays()[1].clock, Some(corrected));
    assert_eq!(saved.game.plays()[0].clock_label(), "P2 05:11 / SHOT 4.8");
    let saved = db.undo(id, saved.summary.revision, Uuid::new_v4()).unwrap();
    assert_eq!(saved.clock, corrected);
    drop(db);
    let mut db = Store::open(&path).unwrap();
    let saved = db.load_match(id).unwrap();
    assert_eq!(saved.game.score(Side::Home), 3);
    assert_eq!(saved.game.plays().len(), 1);
    assert_eq!(saved.game.plays()[0].clock, Some(stamp));
    assert_eq!(saved.clock, corrected);
    // An old command replay returns current truth without resurrecting or restamping a play.
    let replay = db
        .record_with_clock(
            id,
            0,
            first,
            Side::Home,
            player,
            Stat::ThreeMade,
            Some(stamp),
        )
        .unwrap();
    assert_eq!(replay.summary.revision, saved.summary.revision);
    assert_eq!(replay.game.plays(), saved.game.plays());
    let raw = Connection::open(&path).unwrap();
    let original: (i64, i64) = raw
        .query_row(
            "SELECT period,game_ms FROM match_events WHERE command_id=?1",
            [first.to_string()],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(original, (2, 310125));
}

#[test]
fn failed_save_rolls_back_stamp_and_retry_keeps_the_original_entry_time() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("retry.sqlite3");
    let mut db = Store::open(&path).unwrap();
    let (id, player) = fixture(&mut db);
    let mut session = Session::new(db);
    session.open(id).unwrap();
    let t = Instant::now();
    session.command_at(C::Start, t).unwrap();
    session.select(Side::Home, player);
    let raw = Connection::open(&path).unwrap();
    raw.execute_batch("CREATE TRIGGER fail BEFORE UPDATE ON matches BEGIN SELECT RAISE(ABORT,'test failure'); END;").unwrap();
    let entered = t + Duration::from_millis(1500);
    let stamp = session.clock_at(entered);
    assert!(session.record_at(Stat::ThreeMade, entered).is_err());
    assert!(session.has_pending_save());
    assert!(session.active().unwrap().game.plays().is_empty());
    assert_eq!(session.clock_at(t + Duration::from_secs(20)), stamp);
    let committed = session.store().load_match(id).unwrap();
    assert_eq!(committed.summary.revision, 1);
    assert_eq!(
        committed.clock,
        ClockState::default().command(C::Start).unwrap()
    );
    assert_eq!(
        raw.query_row("SELECT count(*) FROM match_events", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert!(
        session
            .record_at(Stat::Assist, t + Duration::from_secs(20))
            .is_err()
    );
    raw.execute_batch("DROP TRIGGER fail;").unwrap();
    session.retry().unwrap();
    assert!(!session.has_pending_save());
    let saved = session.active().unwrap();
    assert_eq!(saved.game.plays().len(), 1);
    assert_eq!(saved.game.plays()[0].clock, Some(stamp));
    assert_eq!(saved.clock.shot_ms, 24000);
    drop(session);
    assert_eq!(
        Store::open(path)
            .unwrap()
            .load_match(id)
            .unwrap()
            .game
            .plays()[0]
            .clock,
        Some(stamp)
    );
}

#[test]
fn explicit_clock_payload_reuse_and_invalid_stamps_cannot_mutate_history() {
    let mut db = Store::open(":memory:").unwrap();
    let (id, player) = fixture(&mut db);
    let stamp = ClockState::default();
    let command = Uuid::new_v4();
    db.record_with_clock(
        id,
        0,
        command,
        Side::Home,
        player,
        Stat::TwoMade,
        Some(stamp),
    )
    .unwrap();
    for changed in [
        ClockState { period: 2, ..stamp },
        ClockState {
            game_ms: 1,
            ..stamp
        },
        ClockState {
            shot_ms: 1,
            ..stamp
        },
        ClockState {
            running: true,
            ..stamp
        },
        ClockState {
            running: true,
            shot_running: true,
            ..stamp
        },
    ] {
        assert!(
            db.record_with_clock(
                id,
                0,
                command,
                Side::Home,
                player,
                Stat::TwoMade,
                Some(changed)
            )
            .is_err()
        );
    }
    for invalid in [
        ClockState { period: 0, ..stamp },
        ClockState {
            shot_ms: 120001,
            ..stamp
        },
        ClockState {
            game_ms: 86400001,
            ..stamp
        },
        ClockState {
            shot_running: true,
            ..stamp
        },
    ] {
        assert!(
            db.record_with_clock(
                id,
                1,
                Uuid::new_v4(),
                Side::Home,
                player,
                Stat::Foul,
                Some(invalid)
            )
            .is_err()
        );
    }
    assert!(matches!(
        db.record_with_clock(
            id,
            0,
            Uuid::new_v4(),
            Side::Home,
            player,
            Stat::Assist,
            Some(stamp)
        ),
        Err(Error::Conflict)
    ));
    let saved = db.load_match(id).unwrap();
    assert_eq!(saved.summary.revision, 1);
    assert_eq!(saved.game.plays().len(), 1);
    assert_eq!(saved.game.plays()[0].clock, Some(stamp));
}

#[test]
fn schema_three_upgrade_preserves_unknown_history_and_only_stamps_new_events() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("legacy.sqlite3");
    let raw = Connection::open(&path).unwrap();
    raw.execute_batch(include_str!("../src/schema.sql"))
        .unwrap();
    raw.execute_batch(include_str!("../src/migration_v2.sql"))
        .unwrap();
    raw.execute_batch(include_str!("../src/migration_v3.sql"))
        .unwrap();
    raw.pragma_update(None, "application_id", 0x48584f53)
        .unwrap();
    raw.pragma_update(None, "user_version", 3).unwrap();
    raw.execute_batch("INSERT INTO teams(name) VALUES('Old home'),('Old away');
        INSERT INTO players(name) VALUES('Old player');
        INSERT INTO matches(id,name,date,home_team_id,away_team_id,home_name,away_name,revision) VALUES(1,'Old game','2026-10-17',1,2,'Snapshot home','Snapshot away',3);
        INSERT INTO match_players(match_id,side,player_id,name,jersey,on_court,sort_order) VALUES(1,'home',1,'Snapshot player','7',1,0);
        INSERT INTO match_clocks(match_id,period,game_ms,shot_ms) VALUES(1,3,123456,4321);
        INSERT INTO match_events(id,match_id,command_id,kind,side,player_id,stat) VALUES(1,1,'old-one','record','home',1,'three_made'),(2,1,'old-two','record','home',1,'two_made');
        INSERT INTO match_events(id,match_id,command_id,kind,target_id) VALUES(3,1,'old-undo','undo',2);").unwrap();
    drop(raw);
    let mut db = Store::open(&path).unwrap();
    let saved = db.load_match(1).unwrap();
    assert_eq!(saved.summary.revision, 3);
    assert_eq!(saved.game.score(Side::Home), 3);
    assert!(saved.game.player(Side::Home, 1).unwrap().on_court);
    assert_eq!(saved.game.plays()[0].clock, None);
    assert_eq!(saved.game.plays()[0].clock_label(), "Time not recorded");
    assert_eq!(
        (saved.clock.period, saved.clock.game_ms, saved.clock.shot_ms),
        (3, 123456, 4321)
    );
    let saved = db
        .record(1, 3, Uuid::new_v4(), Side::Home, 1, Stat::Assist)
        .unwrap();
    assert_eq!(saved.game.plays()[0].clock, None);
    assert_eq!(saved.game.plays()[1].clock, Some(saved.clock));
    let raw = Connection::open(&path).unwrap();
    assert_eq!(
        raw.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        5
    );
    assert_eq!(
        raw.query_row("SELECT target_id FROM match_events WHERE id=3", [], |r| r
            .get::<_, i64>(
            0
        ))
        .unwrap(),
        2
    );
    // Reject partial or invalid context instead of loading fabricated clock readings.
    assert!(
        raw.execute("UPDATE match_events SET period=2 WHERE id=1", [])
            .is_err()
    );
    assert!(
        raw.execute("UPDATE match_events SET game_ms=-1 WHERE id=4", [])
            .is_err()
    );
    drop(db);
    assert_eq!(
        Store::open(path)
            .unwrap()
            .load_match(1)
            .unwrap()
            .game
            .plays()[0]
            .clock,
        None
    );
}

#[test]
fn zero_clocks_are_recorded_and_undo_retains_original_stamp_in_the_audit_trail() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("zero.sqlite3");
    let mut db = Store::open(&path).unwrap();
    let (id, player) = fixture(&mut db);
    let stamp = ClockState {
        period: 4,
        game_ms: 0,
        shot_ms: 0,
        running: false,
        shot_running: false,
    };
    let command = Uuid::new_v4();
    let saved = db
        .record_with_clock(
            id,
            0,
            command,
            Side::Home,
            player,
            Stat::FreeThrowMade,
            Some(stamp),
        )
        .unwrap();
    assert_eq!(saved.game.plays()[0].clock_label(), "P4 00:00 / SHOT 0.0");
    let saved = db.undo(id, 1, Uuid::new_v4()).unwrap();
    assert!(saved.game.plays().is_empty());
    let replay = db
        .record_with_clock(
            id,
            0,
            command,
            Side::Home,
            player,
            Stat::FreeThrowMade,
            Some(stamp),
        )
        .unwrap();
    assert!(replay.game.plays().is_empty());
    assert_eq!(replay.summary.revision, 2);
    let raw = Connection::open(path).unwrap();
    assert_eq!(
        raw.query_row(
            "SELECT game_ms FROM match_events WHERE command_id=?1",
            [command.to_string()],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
}
