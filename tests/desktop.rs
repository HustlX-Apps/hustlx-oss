use hustlx_oss::{
    Side, Stat,
    clock::{ClockCommand as C, ClockState},
    profile::Position,
    session::Session,
    storage::{Error, MatchDraft, Store},
};
use rusqlite::Connection;
use std::time::{Duration, Instant};
use tempfile::tempdir;
use uuid::Uuid;
fn fixture(db: &mut Store) -> (i64, i64) {
    let a = db.create_team("A").unwrap();
    let b = db.create_team("B").unwrap();
    let p = db.create_player("Alex").unwrap();
    db.add_to_team(a, p, "").unwrap();
    let saved = db
        .create_match(&MatchDraft {
            name: "Test".into(),
            date: "2026-10-16".into(),
            venue: "Gym".into(),
            home_team_id: a,
            away_team_id: b,
        })
        .unwrap();
    (saved.summary.id, p)
}
#[test]
fn clock_receipts_reopen_deduplicate_and_reject_stale_or_invalid_writes() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("x.sqlite3");
    let mut db = Store::open(&path).unwrap();
    let (id, _) = fixture(&mut db);
    let clock = ClockState::default()
        .command(C::SetGame(123000))
        .unwrap()
        .command(C::SetShot(14000))
        .unwrap();
    let key = Uuid::new_v4();
    let s = db.save_clock(id, 0, key, clock).unwrap();
    assert_eq!(s.clock, clock);
    assert_eq!(s.summary.revision, 1);
    assert_eq!(
        db.save_clock(id, 0, key, clock).unwrap().summary.revision,
        1
    );
    assert!(db.save_clock(id, 1, key, ClockState::default()).is_err());
    assert!(matches!(
        db.save_clock(id, 0, Uuid::new_v4(), clock),
        Err(Error::Conflict)
    ));
    assert!(
        db.save_clock(
            id,
            1,
            Uuid::new_v4(),
            ClockState {
                shot_ms: 120001,
                ..clock
            }
        )
        .is_err()
    );
    drop(db);
    assert_eq!(
        Store::open(path).unwrap().load_match(id).unwrap().clock,
        clock
    );
}
#[test]
fn player_defaults_and_positions_snapshot_without_rewriting_history() {
    let mut db = Store::open(":memory:").unwrap();
    let (id, p) = fixture(&mut db);
    db.save_player_details(p, "00", &[Position::PointGuard, Position::ShootingGuard])
        .unwrap();
    let details = db.player_details(p).unwrap();
    assert_eq!(details.jersey, "00");
    assert_eq!(details.positions.len(), 2);
    let teams = db.teams(false).unwrap();
    let d = MatchDraft {
        name: "New".into(),
        date: "2026-10-17".into(),
        venue: "".into(),
        home_team_id: teams[0].id,
        away_team_id: teams[1].id,
    };
    let newer = db.create_match(&d).unwrap();
    assert_eq!(newer.game.player(Side::Home, p).unwrap().jersey, "00");
    assert_eq!(newer.game.player(Side::Home, p).unwrap().positions.len(), 2);
    db.save_player_details(p, "8", &[Position::Center]).unwrap();
    assert_eq!(
        db.load_match(newer.summary.id)
            .unwrap()
            .game
            .player(Side::Home, p)
            .unwrap()
            .jersey,
        "00"
    );
    assert!(
        db.load_match(id)
            .unwrap()
            .game
            .player(Side::Home, p)
            .unwrap()
            .positions
            .is_empty()
    );
    db.add_to_team(d.home_team_id, p, "12").unwrap();
    assert_eq!(
        db.create_match(&d)
            .unwrap()
            .game
            .player(Side::Home, p)
            .unwrap()
            .jersey,
        "12"
    );
}
#[test]
fn schema_one_upgrade_keeps_history_and_supplies_paused_clock_defaults() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("old.sqlite3");
    let raw = Connection::open(&path).unwrap();
    raw.execute_batch(include_str!("../src/schema.sql"))
        .unwrap();
    raw.pragma_update(None, "application_id", 0x48584f53)
        .unwrap();
    raw.pragma_update(None, "user_version", 1).unwrap();
    raw.execute_batch("INSERT INTO teams(name) VALUES ('Original A'),('Original B'); INSERT INTO players(name) VALUES ('Original player'); INSERT INTO matches(name,date,home_team_id,away_team_id,home_name,away_name) VALUES ('Historic','2026-10-01',1,2,'Original A','Original B'); INSERT INTO match_players VALUES(1,'home',1,'Snapshot','7',1,0); INSERT INTO match_events(match_id,command_id,kind,side,player_id,stat) VALUES(1,'old-command','record','home',1,'three_made');").unwrap();
    drop(raw);
    let db = Store::open(&path).unwrap();
    let m = db.load_match(1).unwrap();
    assert_eq!(m.game.score(Side::Home), 3);
    assert_eq!(m.clock, ClockState::default());
    assert_eq!(m.game.player(Side::Home, 1).unwrap().name, "Snapshot");
    drop(db);
    assert_eq!(
        Connection::open(path)
            .unwrap()
            .pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        2
    );
}
#[test]
fn session_closes_paused_and_running_checkpoint_recovers_without_wall_time() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("s.sqlite3");
    let mut db = Store::open(&path).unwrap();
    let (id, _) = fixture(&mut db);
    let mut s = Session::new(db);
    s.open(id).unwrap();
    let t = Instant::now();
    s.command_at(C::Start, t).unwrap();
    s.tick_at(t + Duration::from_secs(1)).unwrap();
    s.pause_for_close_at(t + Duration::from_millis(1250))
        .unwrap();
    drop(s);
    let mut s = Session::new(Store::open(&path).unwrap());
    s.open(id).unwrap();
    assert_eq!(
        (
            s.active().unwrap().clock.game_ms,
            s.active().unwrap().clock.shot_ms,
            s.active().unwrap().clock.running
        ),
        (598750, 22750, false)
    );
    let t = Instant::now();
    s.command_at(C::Start, t).unwrap();
    for sec in 1..=5 {
        s.tick_at(t + Duration::from_secs(sec)).unwrap();
    }
    drop(s);
    let mut s = Session::new(Store::open(path).unwrap());
    s.open(id).unwrap();
    assert!(!s.active().unwrap().clock.running);
    assert_eq!(s.active().unwrap().clock.game_ms, 593750);
}
#[test]
fn clock_and_stat_side_effects_commit_together_and_failures_retry_once() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("f.sqlite3");
    let mut db = Store::open(&path).unwrap();
    let (id, p) = fixture(&mut db);
    let mut s = Session::new(db);
    s.open(id).unwrap();
    let t = Instant::now();
    s.command_at(C::Start, t).unwrap();
    s.select(Side::Home, p);
    s.record_at(Stat::OffensiveRebound, t + Duration::from_secs(4))
        .unwrap();
    assert_eq!(
        (
            s.active().unwrap().clock.game_ms,
            s.active().unwrap().clock.shot_ms
        ),
        (596000, 14000)
    );
    let raw = Connection::open(&path).unwrap();
    raw.execute_batch("CREATE TRIGGER fail BEFORE UPDATE ON matches BEGIN SELECT RAISE(ABORT,'test failure'); END;").unwrap();
    s.select(Side::Home, p);
    assert!(s.record_at(Stat::Foul, t + Duration::from_secs(5)).is_err());
    assert!(s.has_pending_save());
    assert_eq!(
        s.active().unwrap().game.player_stats(Side::Home, p).fouls,
        0
    );
    assert!(s.pause_for_close_at(t + Duration::from_secs(8)).is_err());
    raw.execute_batch("DROP TRIGGER fail;").unwrap();
    s.retry().unwrap();
    assert_eq!(
        s.active().unwrap().game.player_stats(Side::Home, p).fouls,
        1
    );
    assert!(!s.active().unwrap().clock.running);
    assert_eq!(s.active().unwrap().clock.shot_ms, 13000);
}
#[test]
fn long_tick_gap_pauses_at_last_observation_instead_of_silently_advancing() {
    let mut db = Store::open(":memory:").unwrap();
    let (id, _) = fixture(&mut db);
    let mut s = Session::new(db);
    s.open(id).unwrap();
    let t = Instant::now();
    s.command_at(C::Start, t).unwrap();
    s.tick_at(t + Duration::from_secs(1)).unwrap();
    s.tick_at(t + Duration::from_secs(90)).unwrap();
    assert_eq!(
        (
            s.active().unwrap().clock.game_ms,
            s.active().unwrap().clock.shot_ms,
            s.active().unwrap().clock.running
        ),
        (599000, 23000, false)
    );
    assert!(s.take_clock_notice().is_some());
}

#[test]
fn wall_time_jumps_pause_at_the_last_observation_without_inventing_elapsed_time() {
    use std::time::SystemTime;
    let mut db = Store::open(":memory:").unwrap();
    let (id, _) = fixture(&mut db);
    let mut s = Session::new(db);
    s.open(id).unwrap();
    let t = Instant::now();
    s.command_at(C::Start, t).unwrap();
    let wall = SystemTime::now();
    s.tick_with_wall_at(t + Duration::from_secs(1), wall + Duration::from_secs(1))
        .unwrap();
    s.tick_with_wall_at(
        t + Duration::from_millis(1100),
        wall + Duration::from_secs(3600),
    )
    .unwrap();
    assert_eq!(
        (
            s.active().unwrap().clock.game_ms,
            s.active().unwrap().clock.running
        ),
        (599000, false)
    );
    assert!(s.take_clock_notice().is_some());
}
#[test]
fn expiry_is_saved_once_and_remains_paused() {
    let mut db = Store::open(":memory:").unwrap();
    let (id, _) = fixture(&mut db);
    let mut s = Session::new(db);
    s.open(id).unwrap();
    let t = Instant::now();
    s.command_at(C::SetShot(1000), t).unwrap();
    s.command_at(C::Start, t).unwrap();
    s.tick_at(t + Duration::from_secs(1)).unwrap();
    let rev = s.active().unwrap().summary.revision;
    assert_eq!(
        (
            s.active().unwrap().clock.shot_ms,
            s.active().unwrap().clock.game_ms
        ),
        (0, 599000)
    );
    s.tick_at(t + Duration::from_secs(2)).unwrap();
    assert_eq!(s.active().unwrap().summary.revision, rev);
}
