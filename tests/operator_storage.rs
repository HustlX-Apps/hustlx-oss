use hustlx_oss::{
    Side::{Away, Home},
    operator::PlayerAction,
    sport::{Command, Rules, Sport},
    storage::{Error, SportMatchDraft, Store},
};
use uuid::Uuid;

#[test]
fn rotation_service_exchange_undo_survive_process_restart() {
    use hustlx_oss::sport::State;
    use std::{fs, path::Path, process::Command as Process};
    const REOPEN: &str = "HUSTLX_OSS_ROTATION_REOPEN";
    if let Some(root) = std::env::var_os(REOPEN) {
        let root = Path::new(&root);
        let expected: Vec<(i64, Vec<State>)> =
            serde_json::from_slice(&fs::read(root.join("expected.json")).unwrap()).unwrap();
        let mut db = Store::open(root.join("library.sqlite3")).unwrap();
        for (id, snapshots) in expected {
            let mut saved = db.load_match(id).unwrap();
            assert_eq!(saved.sport_state.as_ref(), snapshots.last());
            for before in snapshots.iter().rev().skip(1) {
                saved = db
                    .undo_sport(id, saved.summary.revision, Uuid::new_v4())
                    .unwrap();
                assert_eq!(saved.sport_state.as_ref(), Some(before));
                assert_eq!(
                    db.load_match(id).unwrap().sport_state.as_ref(),
                    Some(before)
                );
            }
        }
        return;
    }
    let root = tempfile::tempdir().unwrap();
    let mut db = Store::open(root.path().join("library.sqlite3")).unwrap();
    let mut teams = [0; 2];
    let mut roster = [Vec::new(), Vec::new()];
    for i in 0..2 {
        teams[i] = db.create_team("Same club").unwrap();
        for _ in 0..8 {
            let player = db.create_player("Alex").unwrap();
            db.add_to_team(teams[i], player, "7").unwrap();
            roster[i].push(player);
        }
    }
    let mut expected = Vec::new();
    for sport in [
        Sport::Volleyball,
        Sport::Pickleball,
        Sport::Badminton,
        Sport::TableTennis,
        Sport::Tennis,
        Sport::Padel,
    ] {
        let mut saved = db
            .create_sport_match_with_teams(&draft(sport), teams.map(Some))
            .unwrap();
        let id = saved.summary.id;
        let mut snapshots = vec![saved.sport_state.clone().unwrap()];
        let count = if sport == Sport::Volleyball { 6 } else { 2 };
        let mut commands = Vec::new();
        for (side, players) in [(Home, &roster[0]), (Away, &roster[1])] {
            let mut slots = [0; 11];
            slots[..count].copy_from_slice(&players[..count]);
            commands.push(Command::Lineup {
                side,
                players: slots,
            });
        }
        commands.extend([
            Command::Rally(Home),
            Command::Rally(Away),
            Command::Rally(Home),
            Command::SetServer(Home),
        ]);
        for command in commands {
            saved = db
                .record_sport(id, saved.summary.revision, Uuid::new_v4(), command)
                .unwrap();
            snapshots.push(saved.sport_state.clone().unwrap());
        }
        let outgoing = saved.sport_state.as_ref().unwrap().operator.court[0][0];
        if sport == Sport::Volleyball {
            assert_eq!(outgoing, roster[0][1]);
            assert_eq!(
                saved.sport_state.as_ref().unwrap().operator.starters[0][0],
                roster[0][0]
            );
        }
        let command = Command::Exchange {
            side: Home,
            outgoing,
            incoming: roster[0][6],
            libero: false,
        };
        let uuid = Uuid::new_v4();
        let revision = saved.summary.revision;
        saved = db.record_sport(id, revision, uuid, command).unwrap();
        snapshots.push(saved.sport_state.clone().unwrap());
        assert_eq!(
            db.record_sport(id, revision, uuid, command)
                .unwrap()
                .summary
                .revision,
            saved.summary.revision
        );
        assert_eq!(
            saved.sport_state.as_ref().unwrap().operator.court[0][0],
            roster[0][6]
        );
        if sport == Sport::Volleyball {
            saved = db
                .record_sport(
                    id,
                    saved.summary.revision,
                    Uuid::new_v4(),
                    Command::Exchange {
                        side: Home,
                        outgoing: roster[0][6],
                        incoming: roster[0][7],
                        libero: true,
                    },
                )
                .unwrap();
            assert_eq!(
                saved.sport_state.as_ref().unwrap().operator.libero[0],
                Some(roster[0][7])
            );
            snapshots.push(saved.sport_state.clone().unwrap());
            saved = db
                .record_sport(
                    id,
                    saved.summary.revision,
                    Uuid::new_v4(),
                    Command::Rotate(Home),
                )
                .unwrap();
            snapshots.push(saved.sport_state.clone().unwrap());
        }
        expected.push((id, snapshots));
    }
    fs::write(
        root.path().join("expected.json"),
        serde_json::to_vec(&expected).unwrap(),
    )
    .unwrap();
    drop(db);
    let output = Process::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "rotation_service_exchange_undo_survive_process_restart",
            "--nocapture",
        ])
        .env(REOPEN, root.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
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
