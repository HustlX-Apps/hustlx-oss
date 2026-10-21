//! Deterministic sport evaluations. Expected values live in reviewed JSON fixtures.
use hustlx_oss::{
    Side, Stat,
    sport::{Command, Rules, Sport},
    storage::{MatchDraft, SavedMatch, SportMatchDraft, Store},
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Suite {
    version: u8,
    cases: Vec<Case>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    id: String,
    sport: Sport,
    #[serde(default)]
    rules: BTreeMap<String, Value>,
    steps: Vec<Step>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Step {
    command: Value,
    #[serde(default = "one")]
    repeat: usize,
    #[serde(default)]
    reject: bool,
    #[serde(default)]
    expect: BTreeMap<String, Value>,
}
fn one() -> usize {
    1
}

fn validate(suite: &Suite) -> Result<(), &'static str> {
    if suite.version != 1 {
        return Err("Unsupported eval fixture version");
    }
    let mut ids = BTreeSet::new();
    let mut sports = BTreeSet::new();
    for case in &suite.cases {
        if case.id.trim().is_empty() || !ids.insert(&case.id) {
            return Err("Empty or duplicate case ID");
        }
        sports.insert(case.sport.slug());
        if case.steps.is_empty() || !case.steps.iter().any(|s| !s.expect.is_empty()) {
            return Err("Each case needs steps and an expected outcome");
        }
        for step in &case.steps {
            if !(1..=1000).contains(&step.repeat) || step.expect.keys().any(|p| !p.starts_with('/'))
            {
                return Err("Invalid repeat or expectation pointer");
            }
        }
    }
    if sports != Sport::ALL.iter().map(|s| s.slug()).collect() {
        return Err("All nine sports require eval cases");
    }
    Ok(())
}

fn fixture() -> Suite {
    let bytes = match std::env::var_os("HUSTLX_OSS_EVAL_FIXTURE") {
        Some(path) => std::fs::read(path).expect("Read requested eval fixture"),
        None => include_bytes!("fixtures/sport-evals.json").to_vec(),
    };
    let suite: Suite = serde_json::from_slice(&bytes).expect("Strict eval fixture schema");
    validate(&suite).expect("Complete eval fixture");
    suite
}

fn projection(saved: &SavedMatch) -> Value {
    json!({ "state": saved.sport_state, "score": [saved.game.score(Side::Home), saved.game.score(Side::Away)],
        "plays": saved.game.plays().len(), "revision": saved.summary.revision })
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BasketballAction {
    side: Side,
    player: i64,
    stat: BasketballStat,
}
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum BasketballStat {
    TwoMade,
    ThreeMade,
    FreeThrowMade,
    Foul,
}

fn evaluate(case: &Case) -> usize {
    let mut db = Store::open(":memory:").unwrap();
    let home = db.create_team("Home").unwrap();
    let away = db.create_team("Away").unwrap();
    let a = db.create_player("Home actor").unwrap();
    let b = db.create_player("Away actor").unwrap();
    db.add_to_team(home, a, "1").unwrap();
    db.add_to_team(away, b, "1").unwrap();
    let mut saved = if case.sport == Sport::Basketball {
        assert!(
            case.rules.is_empty(),
            "Basketball evals use the existing fixed clock preset"
        );
        db.create_match(&MatchDraft {
            name: case.id.clone(),
            date: "2026-10-21".into(),
            venue: String::new(),
            home_team_id: home,
            away_team_id: away,
        })
        .unwrap()
    } else {
        let mut rules = serde_json::to_value(Rules::defaults(case.sport)).unwrap();
        for (key, value) in &case.rules {
            rules
                .as_object_mut()
                .unwrap()
                .insert(key.clone(), value.clone());
        }
        db.create_sport_match_with_teams(
            &SportMatchDraft {
                name: case.id.clone(),
                date: "2026-10-21".into(),
                venue: String::new(),
                home_name: "Home".into(),
                away_name: "Away".into(),
                sport: case.sport,
                rules: serde_json::from_value(rules).unwrap(),
            },
            [Some(home), Some(away)],
        )
        .unwrap()
    };
    let id = saved.summary.id;
    let mut assertions = 0;
    for (index, step) in case.steps.iter().enumerate() {
        for _ in 0..step.repeat {
            let before = projection(&saved);
            let revision = saved.summary.revision;
            let uuid = Uuid::new_v4();
            let result = if step.command == json!({"undo":true}) {
                if case.sport == Sport::Basketball {
                    db.undo(id, revision, uuid)
                } else {
                    db.undo_sport(id, revision, uuid)
                }
            } else if case.sport == Sport::Basketball {
                let action: BasketballAction = serde_json::from_value(step.command.clone())
                    .expect("Known basketball eval action");
                let stat = match action.stat {
                    BasketballStat::TwoMade => Stat::TwoMade,
                    BasketballStat::ThreeMade => Stat::ThreeMade,
                    BasketballStat::FreeThrowMade => Stat::FreeThrowMade,
                    BasketballStat::Foul => Stat::Foul,
                };
                db.record(id, revision, uuid, action.side, action.player, stat)
            } else {
                let command: Command =
                    serde_json::from_value(step.command.clone()).expect("Known sport eval command");
                db.record_sport(id, revision, uuid, command)
            };
            if step.reject {
                assert!(
                    result.is_err(),
                    "{} step {index}: action should be rejected",
                    case.id
                );
                saved = db.load_match(id).unwrap();
                assert_eq!(
                    projection(&saved),
                    before,
                    "{}: rejected write changed state",
                    case.id
                );
                assertions += 2;
            } else {
                saved = result.unwrap_or_else(|e| panic!("{} step {index}: {e}", case.id));
            }
        }
        let actual = projection(&saved);
        for (pointer, expected) in &step.expect {
            assert_eq!(
                actual.pointer(pointer),
                Some(expected),
                "{} step {index} {pointer}",
                case.id
            );
            assertions += 1;
        }
    }
    assertions
}

#[test]
fn all_nine_sport_evaluations_match_reviewed_outcomes() {
    let suite = fixture();
    let mut report = Vec::new();
    for case in &suite.cases {
        let assertions = evaluate(case);
        report.push(json!({"id":case.id,"sport":case.sport.slug(),"assertions":assertions}));
    }
    println!("SPORT_EVAL_REPORT {}", json!({"version":1,"cases":report}));
}

#[test]
fn evals_reject_empty_missing_duplicate_unknown_and_unasserted_cases() {
    let mut suite = fixture();
    suite.cases.clear();
    assert!(validate(&suite).is_err());
    let mut suite = fixture();
    suite.cases.retain(|c| c.sport != Sport::Padel);
    assert!(validate(&suite).is_err());
    let mut suite = fixture();
    suite.cases[1].id = suite.cases[0].id.clone();
    assert!(validate(&suite).is_err());
    let mut suite = fixture();
    for step in &mut suite.cases[0].steps {
        step.expect.clear();
    }
    assert!(validate(&suite).is_err());
    assert!(
        serde_json::from_str::<Suite>(r#"{"version":1,"cases":[],"ignore_failures":true}"#)
            .is_err()
    );
}

#[test]
fn a_wrong_expected_result_fails_the_eval() {
    let case: Case = serde_json::from_value(json!({"id":"bad-oracle","sport":"tennis","steps":[{"command":{"rally":"home"},"expect":{"/state/points":[99,0]}}]})).unwrap();
    assert!(std::panic::catch_unwind(|| evaluate(&case)).is_err());
}
