use hustlx_oss::{
    Game, Side, Stat,
    clock::ClockCommand,
    hotkeys::{Action, Hotkeys, Input},
};
fn press(h: &mut Hotkeys, key: &str) -> Option<Action> {
    h.press(Input {
        key,
        enabled: true,
        repeat: false,
        modified: false,
        captured: false,
    })
}
#[test]
fn clock_and_stat_keys_match_supported_v2_web_actions() {
    let mut h = Hotkeys::default();
    assert_eq!(press(&mut h, " "), Some(Action::ToggleClock));
    assert_eq!(
        press(&mut h, "["),
        Some(Action::Clock(ClockCommand::FullShot))
    );
    assert_eq!(
        press(&mut h, "]"),
        Some(Action::Clock(ClockCommand::ShortShot))
    );
    for (k, s) in [
        ("q", Stat::FreeThrowMade),
        ("w", Stat::TwoMade),
        ("e", Stat::ThreeMade),
        ("a", Stat::FreeThrowMiss),
        ("s", Stat::TwoMiss),
        ("d", Stat::ThreeMiss),
        ("r", Stat::DefensiveRebound),
        ("y", Stat::OffensiveRebound),
        ("f", Stat::Foul),
        ("g", Stat::Assist),
        ("t", Stat::Turnover),
        ("v", Stat::Steal),
        ("b", Stat::Block),
        ("+", Stat::TwoMade),
        ("*", Stat::ThreeMade),
        ("/", Stat::FreeThrowMade),
        ("-", Stat::TwoMiss),
    ] {
        assert_eq!(press(&mut h, k), Some(Action::Stat(s)));
    }
    assert_eq!(
        press(&mut h, "p"),
        Some(Action::Clock(ClockCommand::NextPeriod))
    );
    assert_eq!(press(&mut h, "?"), Some(Action::Help));
}
#[test]
fn repeated_modified_captured_and_offscreen_keys_never_trigger() {
    let mut h = Hotkeys::default();
    for (enabled, repeat, modified, captured) in [
        (false, false, false, false),
        (true, true, false, false),
        (true, false, true, false),
        (true, false, false, true),
    ] {
        assert_eq!(
            h.press(Input {
                key: "w",
                enabled,
                repeat,
                modified,
                captured
            }),
            None
        );
    }
    assert_eq!(press(&mut h, "x"), Some(Action::Substitutions));
    assert_eq!(press(&mut h, "o"), None);
}
#[test]
fn jersey_entry_has_side_hints_leading_zeros_and_does_not_guess_ambiguities() {
    let mut h = Hotkeys::default();
    let mut g = Game::default();
    press(&mut h, "7");
    // #7 is unique in the sample away roster.
    assert_eq!(h.target(&g).unwrap().0, Side::Away);
    h.clear();
    press(&mut h, ",");
    press(&mut h, "4");
    assert_eq!(h.target(&g).unwrap(), (Side::Home, 1));
    h.clear();
    g.select(Side::Away, 11);
    assert_eq!(h.target(&g).unwrap(), (Side::Away, 11));
    press(&mut h, "9");
    press(&mut h, "9");
    assert!(h.target(&g).is_err());
    h.clear();
    press(&mut h, "0");
    press(&mut h, "0");
    assert_eq!(h.digits(), "00");
    assert_eq!(press(&mut h, "Backspace"), None);
    assert_eq!(h.digits(), "0");
    press(&mut h, "Backspace");
    assert_eq!(press(&mut h, "Backspace"), Some(Action::Undo));
    assert_eq!(press(&mut h, "Escape"), Some(Action::Clear));
}

#[test]
fn duplicate_jerseys_require_explicit_team_selection() {
    use hustlx_oss::storage::{MatchDraft, Store};
    let mut db = Store::open(":memory:").unwrap();
    let a = db.create_team("A").unwrap();
    let b = db.create_team("B").unwrap();
    let p = db.create_player("One").unwrap();
    let q = db.create_player("Two").unwrap();
    db.add_to_team(a, p, "7").unwrap();
    db.add_to_team(b, q, "7").unwrap();
    let game = db
        .create_match(&MatchDraft {
            name: "Test".into(),
            date: "2026-10-16".into(),
            venue: String::new(),
            home_team_id: a,
            away_team_id: b,
        })
        .unwrap()
        .game;
    let mut keys = Hotkeys::default();
    press(&mut keys, "7");
    assert!(keys.target(&game).is_err());
    press(&mut keys, ",");
    assert_eq!(keys.target(&game).unwrap(), (Side::Home, p));
    press(&mut keys, ".");
    assert_eq!(keys.target(&game).unwrap(), (Side::Away, q));
}

#[test]
fn substitution_shortcut_uses_existing_input_guards() {
    let mut keys = Hotkeys::default();
    assert_eq!(
        keys.press(Input {
            key: "x",
            enabled: true,
            repeat: false,
            modified: false,
            captured: false
        }),
        Some(Action::Substitutions)
    );
    for (repeat, modified, captured, enabled) in [
        (true, false, false, true),
        (false, true, false, true),
        (false, false, true, true),
        (false, false, false, false),
    ] {
        assert_eq!(
            keys.press(Input {
                key: "x",
                enabled,
                repeat,
                modified,
                captured
            }),
            None
        );
    }
}
