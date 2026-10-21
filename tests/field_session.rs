use hustlx_oss::{
    Side,
    field_sport::{FieldCommand as F, FieldEvent, FieldState},
    session::Session,
    sport::{Command, Rules, Sport},
    storage::{SportMatchDraft, Store},
};
use rusqlite::Connection;
use std::time::{Duration, Instant, SystemTime};
use tempfile::tempdir;

fn draft(sport: Sport) -> SportMatchDraft {
    let mut rules = Rules::defaults(sport);
    rules.period_minutes = 1;
    SportMatchDraft {
        name: "Local field match".into(),
        date: "2026-10-21".into(),
        venue: String::new(),
        home_name: "Home".into(),
        away_name: "Away".into(),
        sport,
        rules,
    }
}
fn field(session: &Session) -> &FieldState {
    session
        .active()
        .unwrap()
        .sport_state
        .as_ref()
        .unwrap()
        .field
        .as_ref()
        .unwrap()
}
fn goal() -> F {
    F::Event {
        side: Side::Home,
        kind: FieldEvent::Goal,
        player: None,
    }
}

#[test]
fn five_second_checkpoints_reopen_both_field_clocks_paused_at_the_saved_time() {
    for sport in [Sport::Football, Sport::Futsal] {
        let dir = tempdir().unwrap();
        let path = dir.path().join("clock.sqlite3");
        let mut session = Session::new(Store::open(&path).unwrap());
        session.create_sport(&draft(sport)).unwrap();
        let id = session.active().unwrap().summary.id;
        let t = Instant::now();
        session.field_command_at(F::Start, t).unwrap();
        for seconds in 1..=5 {
            session.tick_at(t + Duration::from_secs(seconds)).unwrap();
        }
        let expected = if sport == Sport::Futsal {
            55_000
        } else {
            5_000
        };
        assert_eq!(
            (field(&session).clock_ms, field(&session).running),
            (expected, true)
        );
        drop(session);
        let mut recovered = Session::new(Store::open(&path).unwrap());
        recovered.open(id).unwrap();
        assert_eq!(
            (field(&recovered).clock_ms, field(&recovered).running),
            (expected, false)
        );
        assert!(recovered.take_clock_notice().is_some());
    }
}

#[test]
fn event_time_and_pause_share_the_exact_elapsed_baseline() {
    let mut session = Session::new(Store::open(":memory:").unwrap());
    session.create_sport(&draft(Sport::Football)).unwrap();
    let t = Instant::now();
    session.field_command_at(F::Start, t).unwrap();
    session
        .field_command_at(goal(), t + Duration::from_millis(1_234))
        .unwrap();
    assert_eq!(
        (field(&session).clock_ms, field(&session).score),
        (1_234, [1, 0])
    );
    session
        .field_command_at(F::Pause, t + Duration::from_millis(2_345))
        .unwrap();
    assert_eq!(
        (field(&session).clock_ms, field(&session).running),
        (2_345, false)
    );
    assert_eq!(
        session.field_clock_at(t + Duration::from_secs(20)).clock_ms,
        2_345
    );
}

#[test]
fn failed_goal_save_freezes_visible_time_and_retry_records_the_same_goal_once() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("retry.sqlite3");
    let mut session = Session::new(Store::open(&path).unwrap());
    session.create_sport(&draft(Sport::Football)).unwrap();
    let t = Instant::now();
    session.field_command_at(F::Start, t).unwrap();
    let raw = Connection::open(&path).unwrap();
    raw.execute_batch("CREATE TRIGGER abort_field BEFORE INSERT ON sport_events BEGIN SELECT RAISE(ABORT,'field test'); END;").unwrap();
    assert!(
        session
            .field_command_at(goal(), t + Duration::from_millis(1_234))
            .is_err()
    );
    assert!(session.has_pending_save());
    assert_eq!(field(&session).score, [0, 0]);
    assert_eq!(
        session.field_clock_at(t + Duration::from_secs(30)).clock_ms,
        1_234
    );
    assert!(
        session
            .pause_for_close_at(t + Duration::from_secs(30))
            .is_err()
    );
    raw.execute_batch("DROP TRIGGER abort_field;").unwrap();
    session.retry().unwrap();
    assert_eq!(
        (field(&session).clock_ms, field(&session).score),
        (1_234, [1, 0])
    );
    assert_eq!(session.active().unwrap().sport_history.len(), 2);
    assert!(!session.has_pending_save());
    assert!(session.retry().is_err());
    assert_eq!(field(&session).score, [1, 0]);
}

#[test]
fn elapsed_futsal_period_is_checkpointed_once_and_stays_paused() {
    let mut session = Session::new(Store::open(":memory:").unwrap());
    session.create_sport(&draft(Sport::Futsal)).unwrap();
    let t = Instant::now();
    session.field_command_at(F::SetTime(1_000), t).unwrap();
    session.field_command_at(F::Start, t).unwrap();
    session.tick_at(t + Duration::from_secs(1)).unwrap();
    assert_eq!(
        (field(&session).clock_ms, field(&session).running),
        (0, false)
    );
    let revision = session.active().unwrap().summary.revision;
    session.tick_at(t + Duration::from_secs(2)).unwrap();
    assert_eq!(session.active().unwrap().summary.revision, revision);
    assert!(session.take_clock_notice().is_some());
}

#[test]
fn tick_gaps_and_wall_time_jumps_pause_at_the_last_observed_field_time() {
    for jump in [false, true] {
        let mut session = Session::new(Store::open(":memory:").unwrap());
        session.create_sport(&draft(Sport::Football)).unwrap();
        let t = Instant::now();
        let wall = SystemTime::now();
        session.field_command_at(F::Start, t).unwrap();
        session
            .tick_with_wall_at(t + Duration::from_secs(1), wall + Duration::from_secs(1))
            .unwrap();
        if jump {
            session
                .tick_with_wall_at(
                    t + Duration::from_millis(1_100),
                    wall + Duration::from_secs(3_600),
                )
                .unwrap();
        } else {
            session.tick_at(t + Duration::from_secs(90)).unwrap();
        }
        assert_eq!(
            (field(&session).clock_ms, field(&session).running),
            (1_000, false)
        );
        assert!(session.take_clock_notice().is_some());
    }
}

#[test]
fn graceful_close_saves_exact_paused_time_and_roster_commands_do_not_drop_elapsed_time() {
    let mut session = Session::new(Store::open(":memory:").unwrap());
    session.create_sport(&draft(Sport::Football)).unwrap();
    let t = Instant::now();
    session.field_command_at(F::Start, t).unwrap();
    session
        .sport_command_at(
            Command::Timeout(Side::Home),
            t + Duration::from_millis(1_234),
        )
        .unwrap();
    assert_eq!(
        (field(&session).clock_ms, field(&session).running),
        (1_234, false)
    );
    session
        .field_command_at(F::Start, t + Duration::from_millis(1_234))
        .unwrap();
    session
        .pause_for_close_at(t + Duration::from_millis(2_345))
        .unwrap();
    assert_eq!(
        (field(&session).clock_ms, field(&session).running),
        (2_345, false)
    );
}

#[test]
fn completion_undo_keeps_the_final_observed_clock_paused_and_skips_clock_receipts() {
    let mut s = Session::new(Store::open(":memory:").unwrap());
    s.create_sport(&draft(Sport::Football)).unwrap();
    let t = Instant::now();
    s.field_command_at(F::Start, t).unwrap();
    s.field_command_at(goal(), t + Duration::from_secs(1))
        .unwrap();
    s.field_command_at(F::Complete, t + Duration::from_secs(3))
        .unwrap();
    s.undo().unwrap();
    assert_eq!(
        (field(&s).clock_ms, field(&s).running, field(&s).completed),
        (3000, false, false)
    );
    s.undo().unwrap();
    assert_eq!(field(&s).score, [0, 0]);
}

#[test]
fn period_undo_restores_its_clock_and_ignores_later_period_clock_receipts() {
    for sport in [Sport::Football, Sport::Futsal] {
        let dir = tempdir().unwrap();
        let path = dir.path().join("period-undo.sqlite3");
        let mut s = Session::new(Store::open(&path).unwrap());
        s.create_sport(&draft(sport)).unwrap();
        let id = s.active().unwrap().summary.id;
        let t = Instant::now();
        s.field_command_at(F::SetTime(12_000), t).unwrap();
        s.field_command_at(F::NextPeriod, t).unwrap();
        s.field_command_at(F::SetTime(25_000), t).unwrap();
        s.field_command_at(F::Start, t).unwrap();
        s.field_command_at(F::Pause, t + Duration::from_secs(1))
            .unwrap();
        s.undo().unwrap();
        assert_eq!(
            (field(&s).period, field(&s).clock_ms, field(&s).running),
            (1, 12_000, false)
        );
        drop(s);
        let mut s = Session::new(Store::open(&path).unwrap());
        s.open(id).unwrap();
        assert_eq!((field(&s).period, field(&s).clock_ms), (1, 12_000));
        // A new branch of play after undo must use its own clock and period.
        s.field_command_at(F::SetTime(8_000), t).unwrap();
        s.field_command_at(F::NextPeriod, t).unwrap();
        s.field_command_at(F::SetTime(7_000), t).unwrap();
        s.field_command_at(goal(), t).unwrap();
        s.undo().unwrap();
        assert_eq!((field(&s).period, field(&s).clock_ms), (2, 7_000));
        s.undo().unwrap();
        assert_eq!((field(&s).period, field(&s).clock_ms), (1, 8_000));
    }
}

#[test]
fn undoing_extra_time_cannot_replay_its_longer_clock_into_a_short_regulation_period() {
    let mut s = Session::new(Store::open(":memory:").unwrap());
    s.create_sport(&draft(Sport::Futsal)).unwrap();
    let t = Instant::now();
    s.field_command_at(F::NextPeriod, t).unwrap();
    s.field_command_at(F::SetTime(0), t).unwrap();
    s.field_command_at(F::NextPeriod, t).unwrap();
    s.field_command_at(F::Start, t).unwrap();
    s.field_command_at(F::Pause, t + Duration::from_secs(1))
        .unwrap();
    s.undo().unwrap();
    assert_eq!((field(&s).period, field(&s).clock_ms), (2, 0));
    assert!(!s.has_pending_save());
}

#[test]
fn futsal_foul_window_survives_period_undo_reentry_and_reopen() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("foul-window.sqlite3");
    let mut s = Session::new(Store::open(&path).unwrap());
    s.create_sport(&draft(Sport::Futsal)).unwrap();
    let id = s.active().unwrap().summary.id;
    let t = Instant::now();
    let foul = F::Event {
        side: Side::Home,
        kind: FieldEvent::Foul,
        player: None,
    };
    s.field_command_at(foul, t).unwrap();
    s.field_command_at(F::NextPeriod, t).unwrap();
    s.field_command_at(foul, t).unwrap();
    s.field_command_at(foul, t).unwrap();
    s.field_command_at(F::NextPeriod, t).unwrap();
    s.field_command_at(foul, t).unwrap();
    assert_eq!((field(&s).period, field(&s).fouls), (3, [3, 0]));
    s.undo().unwrap();
    s.undo().unwrap();
    assert_eq!((field(&s).period, field(&s).fouls), (2, [2, 0]));
    s.field_command_at(F::NextPeriod, t).unwrap();
    s.field_command_at(F::NextPeriod, t).unwrap();
    assert_eq!((field(&s).period, field(&s).fouls), (4, [2, 0]));
    s.field_command_at(F::NextPeriod, t).unwrap();
    assert_eq!(field(&s).fouls, [0, 0]);
    s.undo().unwrap();
    assert_eq!((field(&s).period, field(&s).fouls), (4, [2, 0]));
    drop(s);
    let mut s = Session::new(Store::open(&path).unwrap());
    s.open(id).unwrap();
    assert_eq!((field(&s).period, field(&s).fouls), (4, [2, 0]));
}
