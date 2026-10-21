use hustlx_oss::{
    Side::{Away, Home},
    operator::PlayerAction,
    sport::{Command, Rules, Sport},
    storage::{Error, SportMatchDraft, Store},
};
use uuid::Uuid;
fn draft(sport: Sport) -> SportMatchDraft {
    SportMatchDraft {
        name: "Evening match".into(),
        date: "2026-10-21".into(),
        venue: "Court".into(),
        home_name: "Home".into(),
        away_name: "Away".into(),
        sport,
        rules: Rules::defaults(sport),
    }
}

#[test]
fn optional_team_rosters_are_snapshots_and_actions_retry_undo_and_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("local.sqlite3");
    let mut db = Store::open(&path).unwrap();
    let team = db.create_team("Club").unwrap();
    let player = db.create_player("Sam").unwrap();
    db.add_to_team(team, player, "7").unwrap();
    let mut saved = db
        .create_sport_match_with_teams(&draft(Sport::Volleyball), [Some(team), None])
        .unwrap();
    let id = saved.summary.id;
    assert_eq!(saved.game.player(Home, player).unwrap().name, "Sam");
    let cmd = Command::Player {
        side: Home,
        player: Some(player),
        action: PlayerAction::Kill,
    };
    let uuid = Uuid::new_v4();
    saved = db.record_sport(id, 0, uuid, cmd).unwrap();
    assert_eq!(
        db.record_sport(id, 0, uuid, cmd).unwrap().summary.revision,
        1
    );
    assert_eq!(saved.sport_state.as_ref().unwrap().points, [1, 0]);
    assert!(
        db.record_sport(
            id,
            1,
            Uuid::new_v4(),
            Command::Player {
                side: Away,
                player: Some(player),
                action: PlayerAction::Kill
            }
        )
        .is_err()
    );
    assert!(matches!(
        db.record_sport(id, 0, Uuid::new_v4(), Command::Rally(Home)),
        Err(Error::Conflict)
    ));
    db.rename_player(player, "New name").unwrap();
    drop(db);
    let mut db = Store::open(&path).unwrap();
    saved = db.load_match(id).unwrap();
    assert_eq!(saved.game.player(Home, player).unwrap().name, "Sam");
    saved = db.undo_sport(id, 1, Uuid::new_v4()).unwrap();
    assert_eq!(saved.sport_state.unwrap().points, [0, 0]);
    assert!(saved.sport_history[0].undone);
}

#[test]
fn schema_five_upgrade_preserves_legacy_child_rows_and_accepts_new_field_sports() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("old.sqlite3");
    let raw = rusqlite::Connection::open(&path).unwrap();
    for sql in [
        include_str!("../src/schema.sql"),
        include_str!("../src/migration_v2.sql"),
        include_str!("../src/migration_v3.sql"),
        include_str!("../src/migration_v4.sql"),
        include_str!("../src/migration_v5.sql"),
    ] {
        raw.execute_batch(sql).unwrap();
    }
    raw.execute_batch("PRAGMA application_id=1213747027; PRAGMA user_version=5; CREATE INDEX old_match_title ON matches(name); CREATE TRIGGER old_match_name BEFORE UPDATE OF name ON matches WHEN NEW.name='' BEGIN SELECT RAISE(ABORT,'empty'); END; INSERT INTO teams(name) VALUES('A'),('B'); INSERT INTO players(name) VALUES('Historic'); INSERT INTO matches(id,name,date,home_team_id,away_team_id,home_name,away_name) VALUES(44,'Old','2026-10-20',1,2,'A','B'); INSERT INTO matches(id,name,date,home_team_id,away_team_id,home_name,away_name) VALUES(100,'Removed fixture','2026-10-20',1,2,'A','B'); DELETE FROM matches WHERE id=100; INSERT INTO match_clocks(match_id) VALUES(44); INSERT INTO match_players(match_id,side,player_id,name,jersey,on_court,sort_order) VALUES(44,'home',1,'Historic','8',1,0); INSERT INTO match_events(match_id,command_id,kind,side,player_id,stat) VALUES(44,'old-event','record','home',1,'three_made');").unwrap();
    raw.pragma_update(None, "application_id", 0x48584f53_i64)
        .unwrap();
    drop(raw);
    let mut db = Store::open(&path).unwrap();
    assert_eq!(db.load_match(44).unwrap().game.score(Home), 3);
    for sport in [Sport::Football, Sport::Futsal] {
        let saved = db.create_sport_match(&draft(sport)).unwrap();
        assert_eq!(saved.summary.sport, sport);
        assert!(saved.summary.id > 100);
    }
    let raw = rusqlite::Connection::open(path).unwrap();
    assert_eq!(
        raw.query_row(
            "SELECT count(*) FROM sqlite_schema WHERE name IN ('old_match_title','old_match_name')",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        2
    );
    assert_eq!(
        raw.query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |r| r
            .get::<_, i64>(
            0
        ))
        .unwrap(),
        0
    );
}
