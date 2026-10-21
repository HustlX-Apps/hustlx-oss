//! Functional E2E through real App messages and SQLite, with a fresh-process reopen.
//! Pixel layout and native keyboard routing require the separate native smoke lane.
use super::*;
use hustlx_oss::{
    field_sport::{FieldCommand as F, FieldEvent},
    operator::PlayerAction as A,
    sport::{Command as C, Sport, Status},
};
use serde_json::{Value, json};
use std::{fs, path::Path, process::Command};

fn send(app: &mut App, message: Message) {
    let _ = app.update(message);
    assert!(app.error.is_none(), "{:?}", app.error);
    assert!(!app.pending());
}

fn app(path: &Path) -> App {
    let mut app = App::empty();
    app.session = Some(Session::new(Store::open(path).unwrap()));
    app
}

fn snapshot(app: &App) -> Value {
    let saved = app.active().unwrap();
    json!({
        "sport": saved.summary.sport,
        "revision": saved.summary.revision,
        "score": [app.game().score(Side::Home), app.game().score(Side::Away)],
        "home": app.game().team(Side::Home).players.iter().map(|p| (&p.name, &p.jersey)).collect::<Vec<_>>(),
        "away": app.game().team(Side::Away).players.iter().map(|p| (&p.name, &p.jersey)).collect::<Vec<_>>(),
        "state": saved.sport_state,
        "sport_history": saved.sport_history.len(),
        "basketball_history": app.game().plays().len(),
    })
}

fn render(app: &mut App) {
    for dark in [false, true] {
        app.dark = dark;
        let _ = app.view();
    }
}

fn journey(sport: Sport) {
    if let Some(root) = std::env::var_os("HUSTLX_OSS_E2E_REOPEN") {
        let root = Path::new(&root);
        let mut app = app(&root.join("library.sqlite3"));
        send(&mut app, Message::OpenMatch(1));
        let expected: Value =
            serde_json::from_slice(&fs::read(root.join("expected.json")).unwrap()).unwrap();
        assert_eq!(
            snapshot(&app),
            expected,
            "fresh process must reproduce saved state"
        );
        assert_eq!(app.active().unwrap().summary.sport, sport);
        render(&mut app);
        send(&mut app, Message::Undo);
        if sport == Sport::Basketball {
            assert_eq!(app.game().score(Side::Home), 0);
        } else {
            let state = app.active().unwrap().sport_state.as_ref().unwrap();
            assert_eq!(
                state.status,
                if sport.field() {
                    Status::Live
                } else {
                    Status::Decided
                }
            );
            send(&mut app, Message::Undo);
            let state = app.active().unwrap().sport_state.as_ref().unwrap();
            assert_eq!(state.status, Status::Live);
            assert_eq!(state.winner, None);
            if sport.field() {
                assert_eq!(state.points, [0, 0]);
            }
        }
        let restored = snapshot(&app);
        send(&mut app, Message::Navigate(Screen::Matches));
        send(&mut app, Message::OpenMatch(1));
        assert_eq!(snapshot(&app), restored, "undo must remain saved on reopen");
        render(&mut app);
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let mut app = app(&dir.path().join("library.sqlite3"));
    // Build the library through the same form messages used by the native UI.
    for side in ["North", "South"] {
        send(&mut app, Message::TeamName(side.into()));
        send(&mut app, Message::SaveTeam);
        let team = app.teams.iter().find(|t| t.name == side).unwrap().clone();
        for n in 1..=2 {
            let name = format!("{side} {n}");
            send(&mut app, Message::PlayerName(name.clone()));
            send(&mut app, Message::SavePlayer);
            let player = app.players.iter().find(|p| p.name == name).unwrap().clone();
            send(&mut app, Message::RosterTeam(team.clone()));
            send(&mut app, Message::RosterPlayer(player));
            send(&mut app, Message::Jersey(n.to_string()));
            send(&mut app, Message::SaveRoster);
        }
    }
    let home = app
        .teams
        .iter()
        .find(|t| t.name == "North")
        .unwrap()
        .clone();
    let away = app
        .teams
        .iter()
        .find(|t| t.name == "South")
        .unwrap()
        .clone();
    for m in [
        Message::ChooseSport(sport),
        Message::HomeTeam(home),
        Message::AwayTeam(away),
        Message::MatchName(format!("{sport} E2E")),
        Message::MatchDate("2026-10-21".into()),
    ] {
        send(&mut app, m);
    }
    if sport != Sport::Basketball {
        for field in [
            sports::RuleField::BestOf,
            sports::RuleField::Target,
            sports::RuleField::Decider,
        ] {
            send(
                &mut app,
                Message::SportInput(
                    field,
                    if matches!(field, sports::RuleField::BestOf) {
                        "1"
                    } else {
                        "2"
                    }
                    .into(),
                ),
            );
        }
    }
    send(&mut app, Message::CreateMatch);
    assert_eq!(app.screen, Screen::GameDay);
    assert_eq!(app.game().team(Side::Home).players.len(), 2);
    render(&mut app);
    if sport == Sport::Basketball {
        send(&mut app, Message::Select(Side::Home, 1));
        send(&mut app, Message::Record(Stat::ThreeMade));
        assert_eq!(app.game().score(Side::Home), 3);
        send(&mut app, Message::Undo);
        send(&mut app, Message::Select(Side::Home, 1));
        send(&mut app, Message::Record(Stat::TwoMade));
        assert_eq!(app.game().score(Side::Home), 2);
    } else {
        send(
            &mut app,
            Message::OpenOperatorLineup(Side::Home, false, false),
        );
        send(&mut app, Message::OperatorPick(1));
        send(&mut app, Message::CancelOperatorLineup);
        assert_eq!(app.active().unwrap().summary.revision, 0);
        send(
            &mut app,
            Message::OpenOperatorLineup(Side::Home, false, false),
        );
        send(&mut app, Message::OperatorPick(1));
        send(&mut app, Message::SaveOperatorLineup);
        send(
            &mut app,
            Message::OpenOperatorLineup(Side::Home, true, false),
        );
        send(&mut app, Message::OperatorPick(1));
        send(&mut app, Message::OperatorPick(2));
        send(&mut app, Message::SaveOperatorLineup);
        assert_eq!(
            app.active()
                .unwrap()
                .sport_state
                .as_ref()
                .unwrap()
                .operator
                .court[0],
            [2]
        );
        send(&mut app, Message::Undo);
        assert_eq!(
            app.active()
                .unwrap()
                .sport_state
                .as_ref()
                .unwrap()
                .operator
                .court[0],
            [1]
        );
        let score = if sport.field() {
            C::Field(F::Event {
                side: Side::Home,
                kind: FieldEvent::Goal,
                player: Some(1),
            })
        } else {
            C::Player {
                side: Side::Home,
                player: Some(1),
                action: if sport == Sport::Volleyball {
                    A::Kill
                } else {
                    A::Winner
                },
            }
        };
        send(&mut app, Message::SportCommand(score));
        assert_eq!(
            app.active().unwrap().sport_state.as_ref().unwrap().points,
            [1, 0]
        );
        send(&mut app, Message::Undo);
        assert_eq!(
            app.active().unwrap().sport_state.as_ref().unwrap().points,
            [0, 0]
        );
        if sport.field() {
            send(&mut app, Message::SportCommand(score));
        } else {
            for _ in 0..100 {
                if app.active().unwrap().sport_state.as_ref().unwrap().status == Status::Decided {
                    break;
                }
                send(&mut app, Message::SportCommand(score));
            }
            assert_eq!(
                app.active().unwrap().sport_state.as_ref().unwrap().status,
                Status::Decided
            );
        }
        let revision = app.active().unwrap().summary.revision;
        send(&mut app, Message::ConfirmSportFinish);
        send(&mut app, Message::CancelSportFinish);
        assert_eq!(app.active().unwrap().summary.revision, revision);
        send(&mut app, Message::ConfirmSportFinish);
        send(&mut app, Message::FinishSport);
        assert_eq!(
            app.active().unwrap().sport_state.as_ref().unwrap().status,
            Status::Completed
        );
        render(&mut app);
    }
    send(&mut app, Message::Navigate(Screen::Matches));
    send(&mut app, Message::OpenMatch(1));
    fs::write(
        dir.path().join("expected.json"),
        serde_json::to_vec(&snapshot(&app)).unwrap(),
    )
    .unwrap();
    drop(app);
    let output = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            &format!("ui::e2e::{}", sport.slug().replace('-', "_")),
            "--nocapture",
        ])
        .env("HUSTLX_OSS_E2E_REOPEN", dir.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "restart failed: {}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("1 passed; 0 failed"),
        "restart selector ran no test"
    );
}

macro_rules! journeys { ($($name:ident => $sport:ident),+ $(,)?) => { $(#[test] fn $name() { journey(Sport::$sport); })+ }; }
journeys! { basketball => Basketball, volleyball => Volleyball, pickleball => Pickleball,
padel => Padel, tennis => Tennis, badminton => Badminton, table_tennis => TableTennis,
football => Football, futsal => Futsal }
