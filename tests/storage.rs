use hustlx_oss::storage::{Error, MatchDraft, Store};
use hustlx_oss::{Side, Stat};
use rusqlite::Connection;
use tempfile::tempdir;
use uuid::Uuid;

fn fixture(store: &mut Store) -> (MatchDraft, i64, i64) {
    let home = store.create_team("Team A").unwrap();
    let away = store.create_team("Team B").unwrap();
    let p = store.create_player("Alex Rivera").unwrap();
    let q = store.create_player("Alex Rivera").unwrap();
    store.add_to_team(home, p, "00").unwrap();
    store.add_to_team(away, q, "7").unwrap();
    (
        MatchDraft {
            name: "Friday basketball".into(),
            date: "2026-10-16".into(),
            venue: "Local gym".into(),
            home_team_id: home,
            away_team_id: away,
        },
        p,
        q,
    )
}

#[test]
fn catalog_names_memberships_and_archives_survive_disk_reopen() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("library.sqlite3");
    let mut db = Store::open(&path).unwrap();
    let (d, p, _) = fixture(&mut db);
    db.rename_team(d.home_team_id, "  Blue squad  ").unwrap();
    db.rename_player(p, "  Alex Santos  ").unwrap();
    db.add_to_team(d.home_team_id, p, "12A").unwrap();
    db.archive_player(p, true).unwrap();
    db.archive_team(d.away_team_id, true).unwrap();
    drop(db);
    let db = Store::open(path).unwrap();
    assert_eq!(db.teams(false).unwrap().len(), 1);
    assert_eq!(db.teams(true).unwrap().len(), 2);
    assert!(
        db.teams(true)
            .unwrap()
            .iter()
            .any(|t| t.name == "Blue squad")
    );
    assert_eq!(db.players(false).unwrap().len(), 1);
    assert!(db.roster(d.home_team_id).unwrap().is_empty());
    db.archive_player(p, false).unwrap();
    db.archive_team(d.away_team_id, false).unwrap();
    let roster = db.roster(d.home_team_id).unwrap();
    assert_eq!(
        (
            roster[0].player_id,
            roster[0].name.as_str(),
            roster[0].jersey.as_str()
        ),
        (p, "Alex Santos", "12A")
    );
    db.remove_from_team(d.home_team_id, p).unwrap();
    assert!(db.roster(d.home_team_id).unwrap().is_empty());
}

#[test]
fn reopened_match_keeps_metadata_and_match_time_rosters() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("library.sqlite3");
    let mut db = Store::open(&path).unwrap();
    let (d, p, q) = fixture(&mut db);
    let saved = db.create_match(&d).unwrap();
    db.rename_team(d.home_team_id, "Renamed team").unwrap();
    db.rename_player(p, "Renamed player").unwrap();
    db.remove_from_team(d.home_team_id, p).unwrap();
    db.archive_player(q, true).unwrap();
    db.archive_team(d.away_team_id, true).unwrap();
    drop(db);
    let db = Store::open(path).unwrap();
    let reopened = db.load_match(saved.summary.id).unwrap();
    assert_eq!(reopened.summary, saved.summary);
    assert_eq!(reopened.game.team(Side::Home).name, "Team A");
    assert_eq!(
        reopened.game.player(Side::Home, p).unwrap().name,
        "Alex Rivera"
    );
    assert_eq!(reopened.game.player(Side::Home, p).unwrap().jersey, "00");
    assert!(reopened.game.player(Side::Away, q).is_some());
    assert_eq!(db.matches().unwrap(), vec![saved.summary]);
}

#[test]
fn real_calendar_date_distinct_teams_and_active_catalog_are_required() {
    let mut db = Store::open(":memory:").unwrap();
    let (mut d, _, _) = fixture(&mut db);
    for date in ["2026-02-29", "2026-13-01", "2026-2-03", "", "not a date"] {
        d.date = date.into();
        assert!(db.create_match(&d).is_err(), "{date}");
    }
    d.date = "2028-02-29".into();
    d.away_team_id = d.home_team_id;
    assert!(db.create_match(&d).is_err());
    d.away_team_id = 999;
    assert!(db.create_match(&d).is_err());
    assert!(db.matches().unwrap().is_empty());
    d.away_team_id = db.teams(false).unwrap()[1].id;
    db.archive_team(d.away_team_id, true).unwrap();
    assert!(db.create_match(&d).is_err());
    db.archive_team(d.away_team_id, false).unwrap();
    d.venue = String::new();
    assert!(db.create_match(&d).is_ok());
}

#[test]
fn empty_rosters_and_duplicate_names_do_not_gate_creation() {
    let mut db = Store::open(":memory:").unwrap();
    let a = db.create_team("Same name").unwrap();
    let b = db.create_team("Same name").unwrap();
    assert_ne!(a, b);
    let d = MatchDraft {
        name: "Friendly".into(),
        date: "2026-10-16".into(),
        venue: String::new(),
        home_team_id: a,
        away_team_id: b,
    };
    let saved = db.create_match(&d).unwrap();
    assert!(saved.game.team(Side::Home).players.is_empty());
    assert!(saved.game.team(Side::Away).players.is_empty());
}

#[test]
fn committed_stats_and_undo_survive_restarting_the_repository() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("library.sqlite3");
    let mut db = Store::open(&path).unwrap();
    let (d, p, q) = fixture(&mut db);
    let s = db.create_match(&d).unwrap();
    let s = db
        .record(
            s.summary.id,
            0,
            Uuid::new_v4(),
            Side::Home,
            p,
            Stat::ThreeMade,
        )
        .unwrap();
    let s = db
        .record(
            s.summary.id,
            s.summary.revision,
            Uuid::new_v4(),
            Side::Away,
            q,
            Stat::TwoMade,
        )
        .unwrap();
    drop(db);
    let mut db = Store::open(&path).unwrap();
    let s = db.load_match(s.summary.id).unwrap();
    assert_eq!((s.game.score(Side::Home), s.game.score(Side::Away)), (3, 2));
    assert_eq!(s.game.player_stats(Side::Home, p).threes_attempted, 1);
    let s = db
        .undo(s.summary.id, s.summary.revision, Uuid::new_v4())
        .unwrap();
    drop(db);
    let db = Store::open(&path).unwrap();
    let s = db.load_match(s.summary.id).unwrap();
    assert_eq!((s.game.score(Side::Home), s.game.score(Side::Away)), (3, 0));
    assert_eq!(s.summary.revision, 3);
    let raw = Connection::open(path).unwrap();
    assert_eq!(
        raw.query_row("SELECT count(*) FROM match_events", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        3
    );
}

#[test]
fn exact_record_and_undo_retries_are_idempotent_even_after_later_commands() {
    let mut db = Store::open(":memory:").unwrap();
    let (d, p, _) = fixture(&mut db);
    let s = db.create_match(&d).unwrap();
    let id = s.summary.id;
    let key = Uuid::new_v4();
    db.record(id, 0, key, Side::Home, p, Stat::ThreeMade)
        .unwrap();
    let s = db
        .record(id, 0, key, Side::Home, p, Stat::ThreeMade)
        .unwrap();
    assert_eq!(s.game.score(Side::Home), 3);
    assert_eq!(s.summary.revision, 1);
    assert!(db.record(id, 1, key, Side::Home, p, Stat::TwoMade).is_err());
    let undo = Uuid::new_v4();
    db.undo(id, 1, undo).unwrap();
    db.record(id, 2, Uuid::new_v4(), Side::Home, p, Stat::TwoMade)
        .unwrap();
    let s = db.undo(id, 1, undo).unwrap();
    assert_eq!(s.game.score(Side::Home), 2);
    assert_eq!(s.summary.revision, 3);
}

#[test]
fn stale_window_and_foreign_player_cannot_change_a_saved_match() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("library.sqlite3");
    let mut a = Store::open(&path).unwrap();
    let (d, p, q) = fixture(&mut a);
    let id = a.create_match(&d).unwrap().summary.id;
    let mut b = Store::open(path).unwrap();
    assert!(
        a.record(id, 0, Uuid::new_v4(), Side::Home, q, Stat::ThreeMade)
            .is_err()
    );
    a.record(id, 0, Uuid::new_v4(), Side::Home, p, Stat::ThreeMade)
        .unwrap();
    assert!(matches!(
        b.record(id, 0, Uuid::new_v4(), Side::Home, p, Stat::TwoMade),
        Err(Error::Conflict)
    ));
    assert_eq!(b.load_match(id).unwrap().game.score(Side::Home), 3);
}

#[test]
fn failed_transaction_leaves_no_partial_event_and_the_same_command_can_retry() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("library.sqlite3");
    let mut db = Store::open(&path).unwrap();
    let (d, p, _) = fixture(&mut db);
    let id = db.create_match(&d).unwrap().summary.id;
    let raw = Connection::open(&path).unwrap();
    raw.execute_batch("CREATE TRIGGER fail_save BEFORE UPDATE ON matches BEGIN SELECT RAISE(ABORT, 'injected save failure'); END;").unwrap();
    let key = Uuid::new_v4();
    assert!(
        db.record(id, 0, key, Side::Home, p, Stat::ThreeMade)
            .is_err()
    );
    assert_eq!(db.load_match(id).unwrap().game.score(Side::Home), 0);
    assert_eq!(db.load_match(id).unwrap().summary.revision, 0);
    assert_eq!(
        raw.query_row("SELECT count(*) FROM match_events", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    raw.execute_batch("DROP TRIGGER fail_save").unwrap();
    assert_eq!(
        db.record(id, 0, key, Side::Home, p, Stat::ThreeMade)
            .unwrap()
            .game
            .score(Side::Home),
        3
    );
}

#[test]
fn a_failed_save_preserves_visible_state_and_selection_until_explicit_retry() {
    use hustlx_oss::session::Session;
    let dir = tempdir().unwrap();
    let path = dir.path().join("library.sqlite3");
    let mut db = Store::open(&path).unwrap();
    let (d, p, q) = fixture(&mut db);
    let id = db.create_match(&d).unwrap().summary.id;
    let mut session = Session::new(db);
    session.open(id).unwrap();
    session.select(Side::Home, p);
    let raw = Connection::open(&path).unwrap();
    raw.execute_batch("CREATE TRIGGER fail_save BEFORE UPDATE ON matches BEGIN SELECT RAISE(ABORT, 'injected save failure'); END;").unwrap();
    assert!(session.record(Stat::ThreeMade).is_err());
    assert_eq!(session.active().unwrap().game.score(Side::Home), 0);
    assert_eq!(
        session.active().unwrap().game.selected,
        Some((Side::Home, p))
    );
    session.select(Side::Away, q);
    assert_eq!(
        session.active().unwrap().game.selected,
        Some((Side::Home, p))
    );
    assert!(session.has_pending_save());
    assert!(session.record(Stat::TwoMade).is_err());
    raw.execute_batch("DROP TRIGGER fail_save").unwrap();
    session.retry().unwrap();
    assert_eq!(session.active().unwrap().game.score(Side::Home), 3);
    assert_eq!(session.active().unwrap().game.selected, None);
    assert!(!session.has_pending_save());
}

#[test]
fn invalid_catalog_input_and_unknown_memberships_leave_the_library_unchanged() {
    let db = Store::open(":memory:").unwrap();
    assert!(db.create_team("   ").is_err());
    assert!(db.create_player("").is_err());
    let team = db.create_team("A").unwrap();
    assert!(db.add_to_team(team, 999, "7").is_err());
    assert!(db.rename_player(999, "Unknown").is_err());
    assert!(db.roster(team).unwrap().is_empty());
    assert!(db.players(true).unwrap().is_empty());
}

#[test]
fn unsupported_schema_and_corrupt_files_are_never_reset_to_empty_databases() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("library.sqlite3");
    let db = Store::open(&path).unwrap();
    db.create_team("Keep me").unwrap();
    drop(db);
    let raw = Connection::open(&path).unwrap();
    raw.pragma_update(None, "user_version", 999).unwrap();
    drop(raw);
    assert!(matches!(Store::open(&path), Err(Error::UnsupportedSchema)));
    let raw = Connection::open(&path).unwrap();
    assert_eq!(
        raw.query_row("SELECT name FROM teams", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "Keep me"
    );
    let corrupt = dir.path().join("corrupt.sqlite3");
    std::fs::write(&corrupt, b"not a database").unwrap();
    assert!(Store::open(&corrupt).is_err());
    assert_eq!(std::fs::read(corrupt).unwrap(), b"not a database");
}

#[test]
fn all_stat_types_round_trip_through_sqlite_and_remain_undoable() {
    use hustlx_oss::Game;
    let dir = tempdir().unwrap();
    let path = dir.path().join("library.sqlite3");
    let mut db = Store::open(&path).unwrap();
    let (d, p, _) = fixture(&mut db);
    let mut saved = db.create_match(&d).unwrap();
    let mut expected = Game::default();
    for stat in [
        Stat::FreeThrowMade,
        Stat::TwoMade,
        Stat::ThreeMade,
        Stat::FreeThrowMiss,
        Stat::TwoMiss,
        Stat::ThreeMiss,
        Stat::OffensiveRebound,
        Stat::DefensiveRebound,
        Stat::Assist,
        Stat::Turnover,
        Stat::Steal,
        Stat::Block,
        Stat::Foul,
        Stat::Technical,
        Stat::Unsportsmanlike,
        Stat::Disqualifying,
        Stat::Warning,
    ] {
        expected.select(Side::Home, 1);
        expected.record(stat);
        saved = db
            .record(
                saved.summary.id,
                saved.summary.revision,
                Uuid::new_v4(),
                Side::Home,
                p,
                stat,
            )
            .unwrap();
    }
    drop(db);
    let mut db = Store::open(&path).unwrap();
    let saved = db.load_match(saved.summary.id).unwrap();
    assert_eq!(
        saved.game.player_stats(Side::Home, p),
        expected.player_stats(Side::Home, 1)
    );
    assert_eq!(saved.game.plays().len(), 17);
    assert_eq!(
        db.undo(saved.summary.id, saved.summary.revision, Uuid::new_v4())
            .unwrap()
            .game
            .player_stats(Side::Home, p)
            .warnings,
        0
    );
}
