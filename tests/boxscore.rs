use hustlx_oss::{
    Side, Stat,
    boxscore::{
        SheetResult, build,
        layout::{Metrics, layout},
        pdf::{page_count, render},
    },
    clock::ClockCommand,
    lineup::LineupKind,
    session::Session,
    storage::{MatchDraft, Store},
};
use std::time::{Duration, Instant};
use tempfile::tempdir;

const JERSEYS: [&str; 7] = ["4", "6", "9", "11", "23", "15", "18"];

fn team(db: &mut Store, name: &str) -> (i64, Vec<i64>) {
    let team = db.create_team(name).unwrap();
    let players = JERSEYS
        .iter()
        .map(|jersey| {
            let p = db.create_player(&format!("{name} {jersey}")).unwrap();
            db.add_to_team(team, p, jersey).unwrap();
            p
        })
        .collect();
    (team, players)
}

#[test]
fn saved_match_from_storage_builds_fits_and_renders_one_page() {
    let dir = tempdir().unwrap();
    let mut db = Store::open(dir.path().join("library.sqlite3")).unwrap();
    let (home, hp) = team(&mut db, "Home");
    let (away, ap) = team(&mut db, "Away");
    let id = db
        .create_match(&MatchDraft {
            name: "Storage Box Score".into(),
            date: "2026-10-24".into(),
            venue: "Gym".into(),
            home_team_id: home,
            away_team_id: away,
        })
        .unwrap()
        .summary
        .id;

    let mut session = Session::new(db);
    session.open(id).unwrap();
    let start = Instant::now();
    let at = |secs: u64| start + Duration::from_secs(secs);
    session
        .save_lineup_at(Side::Home, LineupKind::Starters, hp[..5].to_vec(), at(0))
        .unwrap();
    session
        .save_lineup_at(Side::Away, LineupKind::Starters, ap[..5].to_vec(), at(0))
        .unwrap();
    session.command_at(ClockCommand::Start, at(0)).unwrap();
    session.select(Side::Home, hp[0]);
    session.record_at(Stat::TwoMade, at(30)).unwrap();
    session.select(Side::Away, ap[1]);
    session.record_at(Stat::ThreeMade, at(60)).unwrap();
    session.select(Side::Home, hp[2]);
    session.record_at(Stat::DefensiveRebound, at(75)).unwrap();
    session.pause_for_close_at(at(90)).unwrap();

    let saved = session.store().load_match(id).unwrap();
    let SheetResult::Ready(sheet) = build(&saved, None, "2026-10-24", "0.1.0") else {
        panic!("a basketball match builds a ready sheet");
    };
    assert_eq!(sheet.teams[0].rows.len(), JERSEYS.len());
    let pts = |block: usize| {
        let t = &sheet.teams[block];
        let col = t.columns.iter().position(|c| c.key == "pts").unwrap();
        t.totals.cells[col].clone()
    };
    assert_eq!((pts(0), pts(1)), ("2".to_string(), "3".to_string()));
    assert!(sheet.teams[0].columns.iter().any(|c| c.key == "min"));
    let l = layout(&sheet, &Metrics::bundled()).unwrap();
    assert_eq!(page_count(&render(&l)), 1);
}
