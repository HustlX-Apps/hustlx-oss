use hustlx_oss::{
    Side,
    field_sport::{FieldCommand as C, FieldEvent as E, FieldState},
};
use std::time::Duration;

fn event(side: Side, kind: E) -> C {
    C::Event {
        side,
        kind,
        player: None,
    }
}

#[test]
fn football_clock_counts_up_through_stoppage_and_period_advances_paused() {
    let mut state = FieldState::new(false, 45 * 60_000).unwrap();
    assert_eq!(state.clock_ms, 0);
    state.apply(C::Start).unwrap();
    state = state.elapsed(Duration::from_millis(46 * 60_000 + 500));
    assert_eq!(state.clock_ms, 46 * 60_000 + 500);
    assert!(state.running);
    state.apply(C::NextPeriod).unwrap();
    assert_eq!((state.period, state.clock_ms, state.running), (2, 0, false));
}

#[test]
fn futsal_clock_expires_without_advancing_or_running_into_negative_time() {
    let mut state = FieldState::new(true, 20 * 60_000).unwrap();
    state.apply(C::Start).unwrap();
    state = state.elapsed(Duration::from_millis(20 * 60_000 + 2_000));
    assert_eq!((state.clock_ms, state.running, state.period), (0, false, 1));
    let expired = state.clone();
    assert!(state.apply(C::Start).is_err());
    assert_eq!(state, expired);
    state.apply(C::NextPeriod).unwrap();
    assert_eq!(
        (state.clock_ms, state.running, state.period),
        (20 * 60_000, false, 2)
    );
}

#[test]
fn own_goals_credit_the_opponent_and_stat_events_do_not_change_score_or_clock() {
    let mut state = FieldState::new(false, 45 * 60_000).unwrap();
    state.apply(C::Start).unwrap();
    state.apply(event(Side::Home, E::Goal)).unwrap();
    state.apply(event(Side::Home, E::OwnGoal)).unwrap();
    state.apply(event(Side::Away, E::PenaltyGoal)).unwrap();
    for kind in [
        E::Shot,
        E::Save,
        E::Assist,
        E::PenaltyMiss,
        E::Yellow,
        E::Red,
        E::Foul,
    ] {
        state.apply(event(Side::Home, kind)).unwrap();
    }
    assert_eq!(state.score, [1, 2]);
    assert_eq!(state.yellow, [1, 0]);
    assert_eq!(state.red, [1, 0]);
    assert_eq!(state.fouls, [1, 0]);
    assert_eq!((state.clock_ms, state.running), (0, true));
}

#[test]
fn futsal_fouls_reset_per_period_without_clearing_discipline_or_score() {
    let mut state = FieldState::new(true, 20 * 60_000).unwrap();
    for _ in 0..6 {
        state.apply(event(Side::Home, E::Foul)).unwrap();
    }
    state.apply(event(Side::Home, E::Yellow)).unwrap();
    state.apply(event(Side::Away, E::Goal)).unwrap();
    state.apply(C::NextPeriod).unwrap();
    assert_eq!(state.fouls, [0, 0]);
    assert_eq!(state.yellow, [1, 0]);
    assert_eq!(state.score, [0, 1]);
}

#[test]
fn extra_time_and_shootout_keep_kicks_separate_from_match_score() {
    for futsal in [false, true] {
        let mut state = FieldState::new(futsal, 60_000).unwrap();
        state.apply(event(Side::Home, E::Goal)).unwrap();
        state.apply(event(Side::Away, E::Goal)).unwrap();
        let before = state.clone();
        assert!(state.apply(event(Side::Home, E::ShootoutGoal)).is_err());
        assert_eq!(state, before);
        state.apply(C::NextPeriod).unwrap();
        state.apply(C::NextPeriod).unwrap();
        assert_eq!(state.period, 3);
        assert_eq!(state.clock_ms, if futsal { 5 * 60_000 } else { 0 });
        state.apply(C::NextPeriod).unwrap();
        state.apply(C::NextPeriod).unwrap();
        assert_eq!((state.period, state.clock_ms, state.running), (5, 0, false));
        state.apply(event(Side::Home, E::ShootoutGoal)).unwrap();
        state.apply(event(Side::Away, E::ShootoutMiss)).unwrap();
        assert_eq!(state.score, [1, 1]);
        assert_eq!(state.shootout, [1, 0]);
        let shootout = state.clone();
        assert!(state.apply(C::NextPeriod).is_err());
        assert!(state.apply(event(Side::Home, E::Goal)).is_err());
        assert_eq!(state, shootout);
        state.apply(C::Complete).unwrap();
        assert!(state.completed);
    }
}

#[test]
fn completing_a_draw_stops_clock_and_rejects_all_later_mutations_atomically() {
    let mut state = FieldState::new(false, 60_000).unwrap();
    state.apply(C::Start).unwrap();
    state.apply(C::Complete).unwrap();
    assert_eq!(state.score, [0, 0]);
    assert!(state.completed);
    assert!(!state.running);
    let finished = state.clone();
    for command in [
        C::Start,
        C::Pause,
        C::NextPeriod,
        C::Complete,
        C::SetTime(100),
        C::Checkpoint {
            clock_ms: 100,
            running: false,
        },
        event(Side::Home, E::Goal),
    ] {
        assert!(state.apply(command).is_err());
        assert_eq!(state, finished);
    }
}

#[test]
fn invalid_times_players_and_counter_overflow_leave_the_state_unchanged() {
    assert!(FieldState::new(false, 0).is_err());
    assert!(FieldState::new(true, u64::MAX).is_err());
    let mut state = FieldState::new(true, 60_000).unwrap();
    for command in [
        C::SetTime(60_001),
        C::Checkpoint {
            clock_ms: 60_001,
            running: false,
        },
        C::Checkpoint {
            clock_ms: 0,
            running: true,
        },
        C::Event {
            side: Side::Home,
            kind: E::Goal,
            player: Some(0),
        },
        C::Event {
            side: Side::Home,
            kind: E::Goal,
            player: Some(-1),
        },
    ] {
        let before = state.clone();
        assert!(state.apply(command).is_err());
        assert_eq!(state, before);
    }
    state.score[0] = u16::MAX;
    let before = state.clone();
    assert!(state.apply(event(Side::Home, E::Goal)).is_err());
    assert_eq!(state, before);
    state.yellow[0] = u16::MAX;
    let before = state.clone();
    assert!(state.apply(event(Side::Home, E::Yellow)).is_err());
    assert_eq!(state, before);
}

#[test]
fn exact_checkpoint_pause_and_serialized_command_replay_preserve_recovery_baseline() {
    let initial = FieldState::new(false, 60_000).unwrap();
    let commands = [
        C::Start,
        C::Checkpoint {
            clock_ms: 12_345,
            running: true,
        },
        C::Pause,
        C::Event {
            side: Side::Away,
            kind: E::Goal,
            player: Some(42),
        },
        C::Complete,
    ];
    let json = serde_json::to_string(&commands).unwrap();
    let decoded: Vec<C> = serde_json::from_str(&json).unwrap();
    let mut state = initial;
    for command in decoded {
        state.apply(command).unwrap();
    }
    assert_eq!(state.clock_ms, 12_345);
    assert_eq!(state.score, [0, 1]);
    assert!(state.completed);
    let saved = serde_json::to_string(&state).unwrap();
    assert_eq!(serde_json::from_str::<FieldState>(&saved).unwrap(), state);
    assert_eq!(state.elapsed(Duration::from_secs(300)), state);
}

#[test]
fn elapsed_is_bounded_even_for_extreme_durations_and_corrections_pause_before_restart() {
    let mut state = FieldState::new(false, 60_000).unwrap();
    state.apply(C::Start).unwrap();
    let capped = state.elapsed(Duration::MAX);
    assert_eq!(capped.clock_ms, hustlx_oss::field_sport::MAX_CLOCK_MS);
    assert!(!capped.running);
    let before = state.clone();
    assert!(state.apply(C::SetTime(u64::MAX)).is_err());
    assert_eq!(state, before);
    state.apply(C::SetTime(17_321)).unwrap();
    assert!(!state.running);
    assert_eq!(state.elapsed(Duration::from_secs(300)).clock_ms, 17_321);
    state.apply(C::Start).unwrap();
    state = state.elapsed(Duration::from_millis(123));
    assert_eq!(state.clock_ms, 17_444);
}

#[test]
fn a_shootout_attempt_that_overflows_its_goal_tally_does_not_partially_count_the_attempt() {
    let mut state = FieldState::new(false, 60_000).unwrap();
    for _ in 1..5 {
        state.apply(C::NextPeriod).unwrap();
    }
    state.shootout[0] = u16::MAX;
    let before = state.clone();
    assert!(state.apply(event(Side::Home, E::ShootoutGoal)).is_err());
    assert_eq!(state, before);
    assert!(state.apply(C::Start).is_err());
    assert_eq!(state, before);
}
