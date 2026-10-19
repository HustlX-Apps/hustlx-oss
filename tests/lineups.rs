use hustlx_oss::{
    Side, Stat,
    clock::{ClockCommand, ClockState},
    lineup::{LineupDraft, LineupKind as K},
    session::Session,
    storage::{Error, MatchDraft, Store},
};
use rusqlite::Connection;
use std::time::{Duration, Instant};
use tempfile::tempdir;
use uuid::Uuid;
fn fixture(db: &mut Store) -> (i64, Vec<i64>, i64) {
    let a = db.create_team("Home").unwrap();
    let b = db.create_team("Away").unwrap();
    let mut players = vec![];
    for n in 1..=7 {
        let p = db.create_player(&format!("Player {n}")).unwrap();
        db.add_to_team(a, p, &n.to_string()).unwrap();
        players.push(p);
    }
    // Same person on another side remains a separate match actor.
    db.add_to_team(b, players[0], "1").unwrap();
    let foreign = db.create_player("Outside match").unwrap();
    let m = db
        .create_match(&MatchDraft {
            name: "Lineup Test".into(),
            date: "2026-10-17".into(),
            venue: "Gym".into(),
            home_team_id: a,
            away_team_id: b,
        })
        .unwrap();
    (m.summary.id, players, foreign)
}
#[test]
fn starters_and_swaps_survive_reopen_without_changing_stats_or_original_starters() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("library.sqlite3");
    let mut db = Store::open(&path).unwrap();
    let (id, p, _) = fixture(&mut db);
    let new = db.load_match(id).unwrap();
    assert!(new.game.court_ids(Side::Home).is_empty());
    assert_eq!(new.starting_lineup(Side::Home), None);
    let s = db
        .save_lineup(
            id,
            0,
            Uuid::new_v4(),
            Side::Home,
            K::Starters,
            &p[..5],
            ClockState::default(),
        )
        .unwrap();
    let s = db
        .record(
            id,
            s.summary.revision,
            Uuid::new_v4(),
            Side::Home,
            p[0],
            Stat::ThreeMade,
        )
        .unwrap();
    let court = vec![p[5], p[1], p[2], p[3], p[6]];
    let s = db
        .save_lineup(
            id,
            s.summary.revision,
            Uuid::new_v4(),
            Side::Home,
            K::Substitution,
            &court,
            ClockState::default(),
        )
        .unwrap();
    assert_eq!(s.starting_lineup(Side::Home), Some(&p[..5]));
    assert_eq!(s.game.court_ids(Side::Home), court);
    assert_eq!(s.game.score(Side::Home), 3);
    assert!(s.game.court_ids(Side::Away).is_empty());
    assert_eq!(s.lineups.len(), 2);
    assert_eq!(s.lineups[1].before, &p[..5]);
    drop(db);
    let s = Store::open(path).unwrap().load_match(id).unwrap();
    assert_eq!(s.game.court_ids(Side::Home), court);
    assert_eq!(s.starting_lineup(Side::Home), Some(&p[..5]));
    assert_eq!(s.game.score(Side::Home), 3);
}
#[test]
fn exact_receipts_reject_changed_payload_foreign_duplicate_and_stale_commands() {
    let mut db = Store::open(":memory:").unwrap();
    let (id, p, foreign) = fixture(&mut db);
    let key = Uuid::new_v4();
    let clock = ClockState::default();
    let s = db
        .save_lineup(id, 0, key, Side::Home, K::Starters, &p[..5], clock)
        .unwrap();
    assert_eq!(
        db.save_lineup(id, 0, key, Side::Home, K::Starters, &p[..5], clock)
            .unwrap()
            .summary
            .revision,
        1
    );
    assert!(
        db.save_lineup(id, 1, key, Side::Home, K::Starters, &p[1..6], clock)
            .is_err()
    );
    assert!(
        db.save_lineup(id, 1, key, Side::Home, K::Substitution, &p[..5], clock)
            .is_err()
    );
    assert!(
        db.save_lineup(
            id,
            1,
            key,
            Side::Home,
            K::Starters,
            &p[..5],
            ClockState {
                shot_ms: 14000,
                ..clock
            }
        )
        .is_err()
    );
    for ids in [vec![p[0], p[0]], vec![foreign]] {
        assert!(
            db.save_lineup(
                id,
                1,
                Uuid::new_v4(),
                Side::Home,
                K::Substitution,
                &ids,
                clock
            )
            .is_err()
        );
    }
    assert!(
        db.save_lineup(
            id,
            1,
            Uuid::new_v4(),
            Side::Away,
            K::Starters,
            &[p[1]],
            clock
        )
        .is_err()
    );
    assert!(matches!(
        db.save_lineup(
            id,
            0,
            Uuid::new_v4(),
            Side::Home,
            K::Starters,
            &p[..5],
            clock
        ),
        Err(Error::Conflict)
    ));
    assert!(db.save_clock(id, 1, key, clock).is_err());
    assert!(
        db.record(id, 1, key, Side::Home, p[0], Stat::TwoMade)
            .is_err()
    );
    assert!(db.undo(id, 1, key).is_err());
    let stat_key = Uuid::new_v4();
    let s = db
        .record(
            id,
            s.summary.revision,
            stat_key,
            Side::Home,
            p[0],
            Stat::TwoMade,
        )
        .unwrap();
    assert!(
        db.save_lineup(
            id,
            s.summary.revision,
            stat_key,
            Side::Home,
            K::Substitution,
            &p[..5],
            clock
        )
        .is_err()
    );
    let clock_key = Uuid::new_v4();
    let s = db
        .save_clock(id, s.summary.revision, clock_key, clock)
        .unwrap();
    assert!(
        db.save_lineup(
            id,
            s.summary.revision,
            clock_key,
            Side::Home,
            K::Starters,
            &p[..5],
            clock
        )
        .is_err()
    );
    assert_eq!(db.load_match(id).unwrap().lineups.len(), 1);
}
#[test]
fn short_handed_empty_and_over_five_lineups_are_operator_choices() {
    let mut db = Store::open(":memory:").unwrap();
    let (id, p, _) = fixture(&mut db);
    let mut revision = 0;
    for court in [&p[..3], &p[..0], &p[..6]] {
        let s = db
            .save_lineup(
                id,
                revision,
                Uuid::new_v4(),
                Side::Home,
                K::Starters,
                court,
                ClockState::default(),
            )
            .unwrap();
        assert_eq!(s.game.court_ids(Side::Home), court);
        revision = s.summary.revision;
    }
    let s = db
        .save_lineup(
            id,
            revision,
            Uuid::new_v4(),
            Side::Away,
            K::Starters,
            &[p[0]],
            ClockState::default(),
        )
        .unwrap();
    assert!(s.game.player(Side::Home, p[0]).unwrap().on_court);
    assert!(s.game.player(Side::Away, p[0]).unwrap().on_court);
}
#[test]
fn drafts_cancel_cleanly_and_swaps_keep_slots_without_changing_starters() {
    let mut db = Store::open(":memory:").unwrap();
    let (id, p, _) = fixture(&mut db);
    let s = db
        .save_lineup(
            id,
            0,
            Uuid::new_v4(),
            Side::Home,
            K::Starters,
            &p[..5],
            ClockState::default(),
        )
        .unwrap();
    let mut draft = LineupDraft::new(&s, Side::Home, K::Substitution);
    draft.pick(&s.game, p[0]).unwrap();
    draft.pick(&s.game, p[5]).unwrap();
    assert_eq!(draft.court, vec![p[5], p[1], p[2], p[3], p[4]]);
    assert_eq!(s.game.court_ids(Side::Home), &p[..5]);
    draft.pick(&s.game, p[5]).unwrap();
    draft.pick(&s.game, p[0]).unwrap();
    assert!(!draft.changed());
    draft.clear();
    assert!(draft.warning().is_some());
    draft.pick(&s.game, p[6]).unwrap();
    assert_eq!(draft.court, vec![p[6]]);
    assert!(draft.pick(&s.game, 99999).is_err());
    drop(draft);
    assert_eq!(
        db.load_match(id).unwrap().game.court_ids(Side::Home),
        &p[..5]
    );
}
#[test]
fn failed_lineup_write_rolls_back_clock_and_court_then_retries_once_and_closes_paused() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("x.sqlite3");
    let mut db = Store::open(&path).unwrap();
    let (id, p, _) = fixture(&mut db);
    let raw = Connection::open(&path).unwrap();
    let mut session = Session::new(db);
    session.open(id).unwrap();
    let start = Instant::now();
    session.command_at(ClockCommand::Start, start).unwrap();
    raw.execute_batch("CREATE TRIGGER fail_lineup BEFORE INSERT ON lineup_commands BEGIN SELECT RAISE(ABORT,'test'); END;").unwrap();
    let revision = session.active().unwrap().summary.revision;
    assert!(
        session
            .save_lineup_at(
                Side::Home,
                K::Starters,
                p[..5].to_vec(),
                start + Duration::from_secs(1)
            )
            .is_err()
    );
    assert!(session.has_pending_save());
    assert!(
        session
            .active()
            .unwrap()
            .game
            .court_ids(Side::Home)
            .is_empty()
    );
    assert_eq!(session.active().unwrap().summary.revision, revision);
    assert!(
        session
            .pause_for_close_at(start + Duration::from_secs(2))
            .is_err()
    );
    let persisted = session.store().load_match(id).unwrap();
    assert!(persisted.game.court_ids(Side::Home).is_empty());
    assert_eq!(persisted.clock.game_ms, 600000);
    assert!(persisted.lineups.is_empty());
    raw.execute_batch("DROP TRIGGER fail_lineup").unwrap();
    session.retry().unwrap();
    assert_eq!(session.active().unwrap().clock.game_ms, 599000);
    assert_eq!(session.active().unwrap().lineups.len(), 1);
    session.pause_for_close_at(Instant::now()).unwrap();
    session.open(id).unwrap();
    assert!(!session.active().unwrap().clock.running);
    assert_eq!(
        session.active().unwrap().game.court_ids(Side::Home),
        &p[..5]
    );
}
#[test]
fn schema_two_upgrade_preserves_current_court_without_inventing_starters() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("legacy.sqlite3");
    let raw = Connection::open(&path).unwrap();
    raw.execute_batch(include_str!("../src/schema.sql"))
        .unwrap();
    raw.execute_batch(include_str!("../src/migration_v2.sql"))
        .unwrap();
    raw.pragma_update(None, "application_id", 0x48584f53)
        .unwrap();
    raw.pragma_update(None, "user_version", 2).unwrap();
    raw.execute_batch("INSERT INTO teams(id,name) VALUES(1,'A'),(2,'B'); INSERT INTO players(id,name) VALUES(1,'Legacy'); INSERT INTO matches(id,name,date,home_team_id,away_team_id,home_name,away_name) VALUES(1,'Old','2026-10-16',1,2,'A','B'); INSERT INTO match_players(match_id,side,player_id,name,jersey,on_court,sort_order) VALUES(1,'home',1,'Legacy','00',1,0); INSERT INTO match_clocks(match_id) VALUES(1);").unwrap();
    drop(raw);
    let db = Store::open(path).unwrap();
    let s = db.load_match(1).unwrap();
    assert_eq!(s.game.court_ids(Side::Home), vec![1]);
    assert_eq!(s.starting_lineup(Side::Home), None);
    assert!(s.lineups.is_empty());
}
