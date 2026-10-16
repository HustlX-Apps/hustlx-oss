use hustlx_oss::clock::{ClockCommand as C, ClockState, format_game, format_shot};
use std::time::Duration;

#[test]
fn elapsed_time_uses_milliseconds_and_pause_freezes_both() {
    let start = ClockState::default().command(C::Start).unwrap();
    let at = start.elapsed(Duration::from_millis(1250));
    assert_eq!((at.game_ms, at.shot_ms), (598750, 22750));
    let paused = at.command(C::Pause).unwrap();
    assert_eq!(paused.elapsed(Duration::from_secs(99)), paused);
    assert_eq!(
        paused
            .command(C::Start)
            .unwrap()
            .elapsed(Duration::from_millis(750))
            .game_ms,
        598000
    );
}
#[test]
fn reset_preserves_running_state_and_does_not_restore_elapsed_game_time() {
    let at = ClockState::default()
        .command(C::Start)
        .unwrap()
        .elapsed(Duration::from_secs(8));
    let short = at.command(C::ShortShot).unwrap();
    assert_eq!(
        (short.game_ms, short.shot_ms, short.running),
        (592000, 14000, true)
    );
    assert_eq!(short.command(C::FullShot).unwrap().shot_ms, 24000);
    let paused = short
        .command(C::Pause)
        .unwrap()
        .command(C::FullShot)
        .unwrap();
    assert!(!paused.running);
}
#[test]
fn expiry_clamps_to_zero_and_pauses_both_clocks() {
    let s = ClockState::default()
        .command(C::Start)
        .unwrap()
        .elapsed(Duration::from_secs(30));
    assert_eq!(
        (s.game_ms, s.shot_ms, s.running, s.shot_running),
        (576000, 0, false, false)
    );
    let s = ClockState::default()
        .command(C::SetGame(1000))
        .unwrap()
        .command(C::Start)
        .unwrap()
        .elapsed(Duration::from_secs(2));
    assert_eq!((s.game_ms, s.shot_ms, s.running), (0, 23000, false));
    assert!(s.command(C::Start).is_err());
}
#[test]
fn shot_can_be_paused_independently_but_not_started_without_game() {
    assert!(ClockState::default().command(C::ToggleShot).is_err());
    let s = ClockState::default()
        .command(C::Start)
        .unwrap()
        .command(C::ToggleShot)
        .unwrap()
        .elapsed(Duration::from_secs(6));
    assert_eq!(
        (s.game_ms, s.shot_ms, s.running, s.shot_running),
        (594000, 24000, true, false)
    );
    let s = s
        .command(C::ToggleShot)
        .unwrap()
        .elapsed(Duration::from_secs(2));
    assert_eq!(s.shot_ms, 22000);
}
#[test]
fn game_reset_period_and_explicit_corrections_are_paused_and_bounded() {
    let s = ClockState::default()
        .command(C::Start)
        .unwrap()
        .elapsed(Duration::from_secs(5));
    let next = s.command(C::NextPeriod).unwrap();
    assert_eq!(
        (next.period, next.game_ms, next.shot_ms, next.running),
        (2, 600000, 24000, false)
    );
    assert_eq!(next.command(C::ResetGame).unwrap().period, 2);
    let set = s
        .command(C::SetGame(900000))
        .unwrap()
        .command(C::SetShot(12000))
        .unwrap();
    assert_eq!(
        (set.game_ms, set.shot_ms, set.running),
        (900000, 12000, false)
    );
    assert!(set.command(C::SetShot(120001)).is_err());
    assert!(set.command(C::SetGame(86400001)).is_err());
}
#[test]
fn display_rounds_up_without_showing_zero_before_expiry() {
    assert_eq!(format_game(598750), "09:59");
    assert_eq!(format_game(1), "00:01");
    assert_eq!(format_game(0), "00:00");
    assert_eq!(format_shot(4999), "5.0");
    assert_eq!(format_shot(1), "0.1");
}

#[test]
fn corrections_parse_real_clock_values_without_overflow_or_invalid_seconds() {
    use hustlx_oss::clock::parse_game;
    assert_eq!(parse_game("01:30").unwrap(), 90000);
    assert_eq!(parse_game("1440:00").unwrap(), 86400000);
    for value in [
        "-1:30",
        "01:60",
        "NaN",
        "1",
        "1440:01",
        "99999999999999999999999999:00",
    ] {
        assert!(parse_game(value).is_err(), "{value}");
    }
}
