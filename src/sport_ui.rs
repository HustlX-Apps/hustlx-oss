use super::*;
use hustlx_oss::{
    sport::{Command, Format, Rules, Scoring, Sport, Status},
    storage::{Error, Result},
};
use iced::widget::{checkbox, column, pick_list, text_input};

#[derive(Debug, Clone, Copy)]
pub enum RuleField {
    Home,
    Away,
    BestOf,
    Target,
    Decider,
    WinBy,
    Cap,
    Tiebreak,
}

pub(super) struct SportSetup {
    pub sport: Sport,
    pub home: String,
    pub away: String,
    best_of: String,
    target: String,
    decider: String,
    win_by: String,
    cap: String,
    tiebreak: String,
    format: Format,
    scoring: Scoring,
    first_server: Side,
    match_tiebreak: bool,
}
impl Default for SportSetup {
    fn default() -> Self {
        Self::new(Sport::Basketball)
    }
}
impl SportSetup {
    fn new(sport: Sport) -> Self {
        let r = Rules::defaults(sport);
        Self {
            sport,
            home: String::new(),
            away: String::new(),
            best_of: r.best_of.to_string(),
            target: r.target.to_string(),
            decider: r.decider_target.to_string(),
            win_by: r.win_by.to_string(),
            cap: r.cap.map_or_else(String::new, |n| n.to_string()),
            tiebreak: r.tiebreak_target.to_string(),
            format: r.format,
            scoring: r.scoring,
            first_server: r.first_server,
            match_tiebreak: r.match_tiebreak,
        }
    }
    pub fn rules(&self) -> Result<Rules> {
        fn number(value: &str, label: &str) -> Result<u16> {
            value
                .trim()
                .parse()
                .map_err(|_| Error::Validation(format!("{label} must be a whole number.")))
        }
        Ok(Rules {
            best_of: number(&self.best_of, "Best of")?
                .try_into()
                .map_err(|_| Error::Validation("Best of is too large.".into()))?,
            target: number(&self.target, "Target")?,
            decider_target: number(&self.decider, "Deciding target")?,
            win_by: number(&self.win_by, "Win by")?,
            cap: if self.cap.trim().is_empty() {
                None
            } else {
                Some(number(&self.cap, "Cap")?)
            },
            format: self.format,
            scoring: self.scoring,
            tiebreak_target: number(&self.tiebreak, "Tiebreak target")?,
            match_tiebreak: self.match_tiebreak,
            first_server: self.first_server,
        })
    }
}

impl App {
    pub(super) fn is_sport_match(&self) -> bool {
        self.active().is_some_and(|m| m.sport_state.is_some())
    }
    pub(super) fn update_sports(&mut self, message: Message) -> Result<()> {
        match message {
            Message::ChooseSport(sport) => {
                let home = self.sport_setup.home.clone();
                let away = self.sport_setup.away.clone();
                self.sport_setup = SportSetup::new(sport);
                self.sport_setup.home = home;
                self.sport_setup.away = away;
            }
            Message::SportInput(field, value) => {
                let f = &mut self.sport_setup;
                *match field {
                    RuleField::Home => &mut f.home,
                    RuleField::Away => &mut f.away,
                    RuleField::BestOf => &mut f.best_of,
                    RuleField::Target => &mut f.target,
                    RuleField::Decider => &mut f.decider,
                    RuleField::WinBy => &mut f.win_by,
                    RuleField::Cap => &mut f.cap,
                    RuleField::Tiebreak => &mut f.tiebreak,
                } = value;
            }
            Message::SportFormat(value) => self.sport_setup.format = value,
            Message::SportScoring(value) => self.sport_setup.scoring = value,
            Message::SportFirstServer(value) => self.sport_setup.first_server = value,
            Message::SportMatchTiebreak(value) => self.sport_setup.match_tiebreak = value,
            Message::SportCommand(command) => self.session_mut()?.sport_command(command)?,
            _ => {}
        }
        Ok(())
    }
    pub(super) fn sport_setup_view(&self, c: Colors) -> Element<'_, Message> {
        let f = &self.sport_setup;
        let racquet_games = matches!(f.sport, Sport::Tennis | Sport::Padel);
        let mut form = column![
            input(
                "Side A / home",
                "Player, pair or team name",
                &f.home,
                RuleField::Home,
                c
            ),
            input(
                "Side B / away",
                "Player, pair or team name",
                &f.away,
                RuleField::Away,
                c
            ),
            text(
                "Names and rules are saved with this match. No team library or account is needed."
            )
            .size(12)
            .color(c.muted),
            text("Match rules").size(17).font(HEADING),
        ]
        .spacing(14);
        if !matches!(f.sport, Sport::Volleyball | Sport::Padel) {
            form = form.push(
                column![
                    text("Format").size(12).color(c.muted),
                    pick_list(
                        [Format::Singles, Format::Doubles],
                        Some(f.format),
                        Message::SportFormat
                    )
                    .width(Fill)
                    .padding(10),
                ]
                .spacing(6),
            );
        } else if f.sport == Sport::Padel {
            form = form.push(text("Doubles / pairs").size(12).color(c.muted));
        }
        if matches!(f.sport, Sport::Pickleball | Sport::Tennis | Sport::Padel) {
            let choices = match f.sport {
                Sport::Pickleball => vec![Scoring::SideOut, Scoring::Rally],
                Sport::Padel => vec![Scoring::Advantage, Scoring::NoAd, Scoring::StarPoint],
                _ => vec![Scoring::Advantage, Scoring::NoAd],
            };
            form = form.push(
                column![
                    text("Scoring").size(12).color(c.muted),
                    pick_list(choices, Some(f.scoring), Message::SportScoring)
                        .width(Fill)
                        .padding(10),
                ]
                .spacing(6),
            );
        }
        form = form
            .push(input(
                if f.sport == Sport::Volleyball || racquet_games {
                    "Best of sets"
                } else {
                    "Best of games"
                },
                "3",
                &f.best_of,
                RuleField::BestOf,
                c,
            ))
            .push(input(
                if racquet_games {
                    "Games to win a set"
                } else {
                    "Points to win"
                },
                "Target",
                &f.target,
                RuleField::Target,
                c,
            ));
        if racquet_games {
            form = form
                .push(input(
                    "Tiebreak points",
                    "7",
                    &f.tiebreak,
                    RuleField::Tiebreak,
                    c,
                ))
                .push(
                    checkbox(f.match_tiebreak)
                        .label("Match tiebreak in the deciding set")
                        .on_toggle(Message::SportMatchTiebreak),
                );
            if f.match_tiebreak {
                form = form.push(input(
                    "Match tiebreak points",
                    "10",
                    &f.decider,
                    RuleField::Decider,
                    c,
                ));
            }
        } else {
            form = form
                .push(input(
                    "Deciding set / game points",
                    "Target",
                    &f.decider,
                    RuleField::Decider,
                    c,
                ))
                .push(input("Win by points", "2", &f.win_by, RuleField::WinBy, c))
                .push(input(
                    "Point cap (optional)",
                    "No cap",
                    &f.cap,
                    RuleField::Cap,
                    c,
                ));
        }
        form = form.push(
            column![
                text("First server").size(12).color(c.muted),
                row![
                    button("Side A")
                        .on_press(Message::SportFirstServer(Side::Home))
                        .style(if f.first_server == Side::Home {
                            button::primary
                        } else {
                            brand::secondary
                        }),
                    button("Side B")
                        .on_press(Message::SportFirstServer(Side::Away))
                        .style(if f.first_server == Side::Away {
                            button::primary
                        } else {
                            brand::secondary
                        }),
                ]
                .spacing(8),
                text("Adjust the defaults to your agreed match rules before starting.")
                    .size(12)
                    .color(c.muted),
            ]
            .spacing(8),
        );
        form.into()
    }
    pub(super) fn sport_view(&self, c: Colors) -> Element<'_, Message> {
        let saved = self.active().expect("sport console requires a match");
        let state = saved
            .sport_state
            .as_ref()
            .expect("sport console requires sport state");
        let rules = state.rules();
        let live = state.status == Status::Live && !self.pending();
        let racquet_games = matches!(state.sport, Sport::Tennis | Sport::Padel);
        let unit = if state.sport == Sport::Volleyball || racquet_games {
            "Set"
        } else {
            "Game"
        };
        let side_name = |side| {
            if side == Side::Home {
                &saved.summary.home_name
            } else {
                &saved.summary.away_name
            }
        };
        let status = match state.status {
            Status::Live => format!(
                "{unit} {}{}",
                state.unit_number,
                if state.tiebreak { " / TIEBREAK" } else { "" }
            ),
            Status::Decided => "Result ready to save".into(),
            Status::Completed => "COMPLETED / RESULT SAVED".into(),
        };
        let mut header = column![
            row![
                column![
                    text(&saved.summary.name).size(22).font(HEADING),
                    text(format!(
                        "{} / {} / {}",
                        state.sport.label(),
                        saved.summary.date,
                        if saved.summary.venue.is_empty() {
                            "Venue not set"
                        } else {
                            &saved.summary.venue
                        }
                    ))
                    .size(12)
                    .color(c.muted)
                ]
                .spacing(5)
                .width(Fill),
                badge("LOCAL MATCH", c.green, c)
            ]
            .spacing(14),
            row![
                self.sport_score(Side::Home, c),
                column![
                    text(status).size(13).font(BOLD).color(c.muted),
                    text(format!("{} : {}", state.units[0], state.units[1]))
                        .size(38)
                        .font(DISPLAY),
                    text(if unit == "Set" { "SETS" } else { "GAMES" })
                        .size(10)
                        .color(c.muted)
                ]
                .spacing(7)
                .align_x(alignment::Horizontal::Center)
                .width(Length::FillPortion(2)),
                self.sport_score(Side::Away, c)
            ]
            .spacing(18)
            .align_y(alignment::Vertical::Center),
        ]
        .spacing(18);
        if let Some(winner) = state.winner {
            header = header.push(
                text(format!("Winner: {}", side_name(winner)))
                    .size(18)
                    .font(HEADING)
                    .color(c.green),
            );
        }
        if state.status == Status::Live {
            header = header.push(
                text(format!(
                    "SERVING: {}{}",
                    side_name(state.server),
                    if state.sport == Sport::Pickleball
                        && rules.scoring == Scoring::SideOut
                        && rules.format == Format::Doubles
                    {
                        format!(" / server {} of 2", state.server_number)
                    } else {
                        String::new()
                    }
                ))
                .size(13)
                .font(SEMIBOLD)
                .color(if state.server == Side::Home {
                    c.home
                } else {
                    c.away
                }),
            );
            if state.sport != Sport::Volleyball && rules.format == Format::Doubles {
                header = header.push(text(format!(
                    "Serving partner {} / receiving partner {}. Partner numbers follow the starting order on each side.",
                    state.server_slot, state.receiver_slot
                )).size(12).color(c.muted));
            }
        }
        let mut controls = column![
            text(
                if state.sport == Sport::Pickleball && rules.scoring == Scoring::SideOut {
                    "Record the rally winner. Only the serving side scores a point."
                } else {
                    "Record the point winner."
                }
            )
            .size(12)
            .color(c.muted),
            row![
                button(
                    text(format!(
                        "{} wins {}",
                        saved.summary.home_name,
                        if rules.scoring == Scoring::SideOut {
                            "rally"
                        } else {
                            "point"
                        }
                    ))
                    .size(17)
                    .font(SEMIBOLD)
                )
                .padding([18, 14])
                .width(Fill)
                .on_press_maybe(live.then_some(Message::SportCommand(Command::Rally(Side::Home)))),
                button(
                    text(format!(
                        "{} wins {}",
                        saved.summary.away_name,
                        if rules.scoring == Scoring::SideOut {
                            "rally"
                        } else {
                            "point"
                        }
                    ))
                    .size(17)
                    .font(SEMIBOLD)
                )
                .padding([18, 14])
                .width(Fill)
                .on_press_maybe(live.then_some(Message::SportCommand(Command::Rally(Side::Away)))),
            ]
            .spacing(12),
            row![
                button("Serve: Side A")
                    .style(brand::secondary)
                    .on_press_maybe(
                        live.then_some(Message::SportCommand(Command::SetServer(Side::Home)))
                    ),
                button("Serve: Side B")
                    .style(brand::secondary)
                    .on_press_maybe(
                        live.then_some(Message::SportCommand(Command::SetServer(Side::Away)))
                    ),
                Space::new().width(Fill),
                button("Undo last action")
                    .style(brand::secondary)
                    .on_press_maybe(
                        (!self.pending()
                            && saved
                                .sport_history
                                .iter()
                                .any(|e| e.command.is_some() && !e.undone))
                        .then_some(Message::Undo)
                    ),
            ]
            .spacing(10),
        ]
        .spacing(12);
        if state.status == Status::Decided {
            controls = controls.push(
                row![
                    text(
                        "The winning score is saved. Complete the match to save its final result."
                    )
                    .size(13)
                    .width(Fill),
                    button("Complete match & save result")
                        .padding([12, 16])
                        .on_press_maybe(
                            (!self.pending()).then_some(Message::SportCommand(Command::Complete))
                        )
                ]
                .spacing(12),
            );
        } else if state.status == Status::Completed {
            controls = controls.push(text("The result is saved on this computer. Undo reopens the match if a correction is needed.").size(13).color(c.green));
        }
        let mut units = column![text(format!("{unit} scores")).size(17).font(HEADING)].spacing(9);
        if state.completed_units.is_empty() {
            units = units.push(
                text(format!("No completed {} yet.", unit.to_lowercase()))
                    .size(12)
                    .color(c.muted),
            );
        }
        for (index, score) in state.completed_units.iter().enumerate() {
            let value = if score.score == [0, 0] {
                score.tiebreak_points.unwrap_or(score.score)
            } else {
                score.score
            };
            let extra = if score.score != [0, 0] {
                score
                    .tiebreak_points
                    .map_or_else(String::new, |p| format!(" (TB {} - {})", p[0], p[1]))
            } else {
                " (match tiebreak)".into()
            };
            units = units.push(
                text(format!(
                    "{unit} {} / {} - {}{}",
                    index + 1,
                    value[0],
                    value[1],
                    extra
                ))
                .size(13),
            );
        }
        if state.status == Status::Live && racquet_games {
            units = units.push(
                text(format!(
                    "Current set games: {} - {}",
                    state.games[0], state.games[1]
                ))
                .size(14)
                .font(SEMIBOLD),
            );
        }
        let mut settings = format!(
            "{} / {} / best of {} / target {}",
            state.sport.label(),
            rules.scoring,
            rules.best_of,
            rules.target
        );
        if state.sport != Sport::Volleyball {
            settings.push_str(&format!(" / {}", rules.format));
        }
        if racquet_games {
            settings.push_str(&format!(" / tiebreak to {}", rules.tiebreak_target));
            if rules.match_tiebreak {
                settings.push_str(&format!(
                    " / deciding match tiebreak to {}",
                    rules.decider_target
                ));
            }
        } else {
            settings.push_str(&format!(
                " / decider {} / win by {} / cap {}",
                rules.decider_target,
                rules.win_by,
                rules.cap.map_or_else(|| "none".into(), |n| n.to_string())
            ));
        }
        settings.push_str(&format!(
            " / first server: {}",
            side_name(rules.first_server)
        ));
        let mut history = column![
            text("Saved match history").size(17).font(HEADING),
            text("Undo keeps the original event and adds a correction to this history.")
                .size(12)
                .color(c.muted)
        ]
        .spacing(9);
        if saved.sport_history.is_empty() {
            history = history.push(text("Ready for the first point.").size(13).color(c.muted));
        }
        for event in saved.sport_history.iter().rev() {
            history = history.push(
                text(format!(
                    "#{}  {}{}",
                    event.id,
                    event.label(),
                    if event.undone { " / UNDONE" } else { "" }
                ))
                .size(13)
                .color(if event.undone { c.muted } else { c.text }),
            );
        }
        scrollable(
            column![
                card(header, c).padding(20).width(Fill),
                card(controls, c).width(Fill),
                row![
                    card(units, c).width(Fill),
                    card(
                        column![
                            text("Saved rules").size(17).font(HEADING),
                            text(settings).size(12).color(c.muted),
                            text(format!("Ends changed {} times", state.ends_changed))
                                .size(12)
                                .color(c.muted)
                        ]
                        .spacing(10),
                        c
                    )
                    .width(Fill)
                ]
                .spacing(14),
                card(history, c).width(Fill),
            ]
            .spacing(14)
            .width(Fill),
        )
        .height(Fill)
        .spacing(8)
        .into()
    }
    fn sport_score(&self, side: Side, c: Colors) -> Element<'_, Message> {
        let saved = self.active().unwrap();
        let state = saved.sport_state.as_ref().unwrap();
        let accent = if side == Side::Home { c.home } else { c.away };
        let name = if side == Side::Home {
            &saved.summary.home_name
        } else {
            &saved.summary.away_name
        };
        column![
            text(if side == Side::Home {
                "SIDE A / HOME"
            } else {
                "SIDE B / AWAY"
            })
            .size(10)
            .font(BOLD)
            .color(accent),
            text(name).size(22).font(HEADING).color(accent),
            text(state.point_label(side)).size(68).font(DISPLAY),
        ]
        .spacing(6)
        .width(Length::FillPortion(3))
        .align_x(if side == Side::Home {
            alignment::Horizontal::Left
        } else {
            alignment::Horizontal::Right
        })
        .into()
    }
}
fn input<'a>(
    label: &'a str,
    placeholder: &str,
    value: &'a str,
    field: RuleField,
    c: Colors,
) -> Element<'a, Message> {
    column![
        text(label).size(12).color(c.muted),
        text_input(placeholder, value)
            .on_input(move |s| Message::SportInput(field, s))
            .padding(10)
    ]
    .spacing(6)
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(store: Store) -> App {
        let mut app = App::empty();
        app.session = Some(Session::new(store));
        app
    }
    fn send(app: &mut App, message: Message) {
        let _ = app.update(message);
        assert!(app.error.is_none(), "{:?}", app.error);
    }
    fn create(app: &mut App, sport: Sport) {
        for message in [
            Message::ChooseSport(sport),
            Message::MatchName(format!("Local {sport}")),
            Message::MatchDate("2026-10-20".into()),
            Message::SportInput(RuleField::Home, "Alex / partner".into()),
            Message::SportInput(RuleField::Away, "Sam / partner".into()),
            Message::SportInput(RuleField::BestOf, "1".into()),
            Message::SportInput(RuleField::Target, "2".into()),
            Message::SportInput(RuleField::Decider, "2".into()),
            Message::CreateMatch,
        ] {
            send(app, message);
        }
    }
    #[test]
    fn all_six_native_flows_create_score_undo_complete_and_reopen() {
        let mut app = app(Store::open(":memory:").unwrap());
        for sport in Sport::ALL.into_iter().filter(|s| *s != Sport::Basketball) {
            create(&mut app, sport);
            assert_eq!(app.screen, Screen::GameDay);
            assert!(app.is_sport_match());
            let id = app.active().unwrap().summary.id;
            let original = app.active().unwrap().sport_state.clone();
            send(&mut app, Message::SportCommand(Command::Rally(Side::Home)));
            send(&mut app, Message::Undo);
            assert_eq!(app.active().unwrap().sport_state, original);
            for _ in 0..200 {
                if app.active().unwrap().sport_state.as_ref().unwrap().status != Status::Live {
                    break;
                }
                send(&mut app, Message::SportCommand(Command::Rally(Side::Home)));
            }
            assert_eq!(
                app.active().unwrap().sport_state.as_ref().unwrap().status,
                Status::Decided
            );
            send(&mut app, Message::SportCommand(Command::Complete));
            for dark in [false, true] {
                app.dark = dark;
                let _ = app.view();
            }
            send(&mut app, Message::Navigate(Screen::Matches));
            send(&mut app, Message::OpenMatch(id));
            let state = app.active().unwrap().sport_state.as_ref().unwrap();
            assert_eq!(state.status, Status::Completed);
            assert_eq!(state.winner, Some(Side::Home));
            send(&mut app, Message::Undo);
            assert_eq!(
                app.active().unwrap().sport_state.as_ref().unwrap().status,
                Status::Decided
            );
        }
    }
    #[test]
    fn sport_console_ignores_basketball_keys_and_stale_controls() {
        let mut app = app(Store::open(":memory:").unwrap());
        create(&mut app, Sport::Volleyball);
        assert!(
            !app.basketball_game_day(),
            "Basketball shortcuts must be hidden"
        );
        send(&mut app, Message::SportCommand(Command::Rally(Side::Home)));
        let revision = app.active().unwrap().summary.revision;
        for key in [
            " ",
            "[",
            "]",
            "p",
            "q",
            "w",
            "e",
            "x",
            "?",
            "1",
            "Backspace",
        ] {
            send(
                &mut app,
                Message::Key {
                    key: key.into(),
                    repeat: false,
                    modified: false,
                    captured: false,
                },
            );
        }
        for message in [
            Message::Clock(ClockCommand::Start),
            Message::ToggleClock,
            Message::ToggleClockEditor,
            Message::ToggleHotkeys,
            Message::Record(Stat::ThreeMade),
            Message::Select(Side::Home, 1),
            Message::OpenLineup(Side::Home, hustlx_oss::lineup::LineupKind::Starters),
            Message::ShowLineupHistory,
        ] {
            send(&mut app, message);
        }
        assert_eq!(app.active().unwrap().summary.revision, revision);
        assert!(!app.active().unwrap().clock.running);
        assert!(!app.clock_edit && !app.show_hotkeys && !app.lineup_history);
        assert!(app.lineup_draft.is_none());
        assert!(app.hotkeys.digits().is_empty());
    }
    #[test]
    fn failed_sport_save_keeps_display_committed_and_retry_applies_once() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ui.sqlite3");
        let mut app = app(Store::open(&path).unwrap());
        create(&mut app, Sport::Badminton);
        let raw = rusqlite::Connection::open(path).unwrap();
        raw.execute_batch("CREATE TRIGGER fail BEFORE UPDATE ON matches BEGIN SELECT RAISE(ABORT,'test save failure'); END;").unwrap();
        let initial = app.active().unwrap().sport_state.clone();
        let _ = app.update(Message::SportCommand(Command::Rally(Side::Home)));
        assert!(app.error.is_some() && app.pending());
        assert_eq!(app.active().unwrap().sport_state, initial);
        let _ = app.view();
        raw.execute_batch("DROP TRIGGER fail;").unwrap();
        send(&mut app, Message::RetrySave);
        assert!(!app.pending());
        assert_eq!(
            app.active().unwrap().sport_state.as_ref().unwrap().points,
            [1, 0]
        );
        assert_eq!(app.active().unwrap().sport_history.len(), 1);
    }
    #[test]
    fn basketball_still_routes_to_its_clock_and_roster_console() {
        let store = Store::open(":memory:").unwrap();
        store.create_team("Basketball A").unwrap();
        store.create_team("Basketball B").unwrap();
        let teams = store.teams(false).unwrap();
        let mut app = app(store);
        for message in [
            Message::ChooseSport(Sport::Basketball),
            Message::MatchName("Basketball regression".into()),
            Message::HomeTeam(teams[0].clone()),
            Message::AwayTeam(teams[1].clone()),
            Message::CreateMatch,
        ] {
            send(&mut app, message);
        }
        assert!(!app.is_sport_match());
        assert_eq!(app.screen, Screen::GameDay);
        assert!(
            app.basketball_game_day(),
            "Basketball retains its shortcuts guide"
        );
        send(
            &mut app,
            Message::Key {
                key: " ".into(),
                repeat: false,
                modified: false,
                captured: false,
            },
        );
        assert!(app.active().unwrap().clock.running);
        for dark in [false, true] {
            app.dark = dark;
            let _ = app.view();
        }
        send(&mut app, Message::ToggleClock);
        assert!(!app.active().unwrap().clock.running);
    }

    #[test]
    fn invalid_rule_input_stays_editable_and_does_not_create_a_match() {
        let mut app = app(Store::open(":memory:").unwrap());
        send(&mut app, Message::ChooseSport(Sport::Tennis));
        send(&mut app, Message::MatchName("Local tennis".into()));
        send(&mut app, Message::SportInput(RuleField::Home, "A".into()));
        send(&mut app, Message::SportInput(RuleField::Away, "B".into()));
        send(
            &mut app,
            Message::SportInput(RuleField::BestOf, "abc".into()),
        );
        let _ = app.update(Message::CreateMatch);
        assert!(app.error.as_ref().unwrap().contains("whole number"));
        assert!(app.active().is_none());
        assert_eq!(app.sport_setup.best_of, "abc");
        assert_eq!(app.screen, Screen::Matches);
    }
}
