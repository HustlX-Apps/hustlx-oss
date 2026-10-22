use super::*;
#[cfg(test)]
use hustlx_oss::sport::{Command, Status};
use hustlx_oss::{
    sport::{Format, Rules, Scoring, Sport},
    storage::{Error, Result},
};
use iced::widget::{checkbox, column, pick_list, text_input};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
            target: if sport.field() {
                r.period_minutes.to_string()
            } else {
                r.target.to_string()
            },
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
    pub(super) fn side_labels(&self) -> (&'static str, &'static str) {
        if self.sport.field() || self.sport == Sport::Volleyball {
            ("Team A / home", "Team B / away")
        } else if self.format == Format::Doubles || self.sport == Sport::Padel {
            ("Pair A / home", "Pair B / away")
        } else {
            ("Player A / home", "Player B / away")
        }
    }
    pub(super) fn issues(&self) -> Vec<(RuleField, String)> {
        let mut issues = Vec::new();
        let tennis = matches!(self.sport, Sport::Tennis | Sport::Padel);
        let mut fields = vec![(
            RuleField::Target,
            &self.target,
            if self.sport.field() { 180 } else { u16::MAX },
        )];
        if !self.sport.field() {
            fields.push((RuleField::BestOf, &self.best_of, 255));
            if tennis {
                fields.push((RuleField::Tiebreak, &self.tiebreak, u16::MAX));
                if self.match_tiebreak {
                    fields.push((RuleField::Decider, &self.decider, u16::MAX));
                }
            } else {
                fields.push((RuleField::Decider, &self.decider, u16::MAX));
                fields.push((RuleField::WinBy, &self.win_by, u16::MAX));
                if !self.cap.trim().is_empty() {
                    fields.push((RuleField::Cap, &self.cap, u16::MAX));
                }
            }
        }
        for (field, value, max) in fields {
            match value.trim().parse::<u16>() {
                Ok(n) if n > 0 && n <= max => {
                    if field == RuleField::BestOf && n.is_multiple_of(2) {
                        issues.push((field, "Use an odd number: 1, 3, 5 or more.".into()));
                    }
                    if field == RuleField::Cap
                        && [&self.target, &self.decider]
                            .iter()
                            .any(|v| v.trim().parse::<u16>().is_ok_and(|target| n < target))
                    {
                        issues.push((field, "The cap must reach both point targets.".into()));
                    }
                }
                _ => issues.push((field, format!("Enter a whole number from 1 to {max}."))),
            }
        }
        issues
    }
    pub fn rules(&self) -> Result<Rules> {
        if let Some((_, error)) = self.issues().first() {
            return Err(Error::Validation(error.clone()));
        }
        let mut rules = Rules::defaults(self.sport);
        // Only visible, applicable controls override defaults. Hidden draft values
        // must not make an otherwise valid match impossible to create.
        let number = |value: &str| {
            value
                .trim()
                .parse::<u16>()
                .expect("validated visible field")
        };
        if self.sport.field() {
            rules.period_minutes = number(&self.target);
        } else {
            rules.best_of = number(&self.best_of) as u8;
            rules.target = number(&self.target);
            rules.format = self.format;
            rules.scoring = self.scoring;
            rules.first_server = self.first_server;
            if matches!(self.sport, Sport::Tennis | Sport::Padel) {
                rules.tiebreak_target = number(&self.tiebreak);
                rules.match_tiebreak = self.match_tiebreak;
                if self.match_tiebreak {
                    rules.decider_target = number(&self.decider);
                }
            } else {
                rules.decider_target = number(&self.decider);
                rules.win_by = number(&self.win_by);
                rules.cap = if self.cap.trim().is_empty() {
                    None
                } else {
                    Some(number(&self.cap))
                };
            }
        }
        hustlx_oss::sport::State::new(self.sport, rules.clone())
            .map_err(|e| Error::Validation(e.to_string()))?;
        Ok(rules)
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
        use library::{feedback, field, pair};
        let f = &self.sport_setup;
        let tennis = matches!(f.sport, Sport::Tennis | Sport::Padel);
        let issues = f.issues();
        let input = |label, placeholder, value, key| {
            let error = self
                .match_form
                .attempted
                .then(|| {
                    issues
                        .iter()
                        .find(|(field, _)| *field == key)
                        .map(|(_, e)| e.clone())
                })
                .flatten();
            feedback(
                label,
                text_input(placeholder, value)
                    .on_input(move |v| Message::SportInput(key, v))
                    .size(14)
                    .padding(11)
                    .style(brand::input)
                    .into(),
                error,
                c,
            )
        };
        if f.sport.field() {
            return column![
                input(
                    "Minutes per regulation period",
                    if f.sport == Sport::Football {
                        "45"
                    } else {
                        "20"
                    },
                    &f.target,
                    RuleField::Target
                ),
                text(if f.sport == Sport::Football {
                    "Two halves. The clock counts up into stoppage time."
                } else {
                    "Two periods. The clock counts down and pauses at zero."
                })
                .size(12)
                .color(c.muted),
                text("Extra time and shootout are available during the match.")
                    .size(12)
                    .color(c.muted),
            ]
            .spacing(10)
            .into();
        }
        let mut form = column![].spacing(16);
        let format: Element<'_, Message> = if matches!(f.sport, Sport::Volleyball | Sport::Padel) {
            field(
                "Format",
                text(if f.sport == Sport::Padel {
                    "Doubles / pairs"
                } else {
                    "Teams"
                })
                .size(14)
                .into(),
                c,
            )
        } else {
            field(
                "Format",
                pick_list(
                    [Format::Singles, Format::Doubles],
                    Some(f.format),
                    Message::SportFormat,
                )
                .width(Fill)
                .text_size(14)
                .style(brand::select)
                .padding(11)
                .into(),
                c,
            )
        };
        let scoring: Element<'_, Message> =
            if matches!(f.sport, Sport::Pickleball | Sport::Tennis | Sport::Padel) {
                let choices = match f.sport {
                    Sport::Pickleball => vec![Scoring::SideOut, Scoring::Rally],
                    Sport::Padel => vec![Scoring::Advantage, Scoring::NoAd, Scoring::StarPoint],
                    _ => vec![Scoring::Advantage, Scoring::NoAd],
                };
                field(
                    "Scoring",
                    pick_list(choices, Some(f.scoring), Message::SportScoring)
                        .width(Fill)
                        .text_size(14)
                        .style(brand::select)
                        .padding(11)
                        .into(),
                    c,
                )
            } else {
                field("Scoring", text("Rally points").size(14).into(), c)
            };
        form = form.push(pair(format, scoring));
        let sets = f.sport == Sport::Volleyball || tennis;
        form = form.push(pair(
            input(
                if sets {
                    "Best of sets"
                } else {
                    "Best of games"
                },
                "3",
                &f.best_of,
                RuleField::BestOf,
            ),
            input(
                if tennis {
                    "Games to win a set"
                } else {
                    "Points to win"
                },
                "Target",
                &f.target,
                RuleField::Target,
            ),
        ));
        if tennis {
            form = form
                .push(input(
                    "Tiebreak points",
                    "7",
                    &f.tiebreak,
                    RuleField::Tiebreak,
                ))
                .push(
                    checkbox(f.match_tiebreak)
                        .label("Match tiebreak in the deciding set")
                        .text_size(13)
                        .on_toggle(Message::SportMatchTiebreak),
                );
            if f.match_tiebreak {
                form = form.push(input(
                    "Match tiebreak points",
                    "10",
                    &f.decider,
                    RuleField::Decider,
                ));
            }
        } else {
            form = form
                .push(pair(
                    input(
                        if sets {
                            "Deciding set points"
                        } else {
                            "Deciding game points"
                        },
                        "Target",
                        &f.decider,
                        RuleField::Decider,
                    ),
                    input("Win by points", "2", &f.win_by, RuleField::WinBy),
                ))
                .push(input(
                    "Point cap (optional)",
                    "No cap",
                    &f.cap,
                    RuleField::Cap,
                ));
        }
        form = form.push(field(
            "First server",
            row![
                button("Home / A")
                    .on_press(Message::SportFirstServer(Side::Home))
                    .padding([10, 16])
                    .style(if f.first_server == Side::Home {
                        button::primary
                    } else {
                        brand::secondary
                    }),
                button("Away / B")
                    .on_press(Message::SportFirstServer(Side::Away))
                    .padding([10, 16])
                    .style(if f.first_server == Side::Away {
                        button::primary
                    } else {
                        brand::secondary
                    }),
            ]
            .spacing(10)
            .into(),
            c,
        ));
        if f.sport == Sport::Pickleball && f.scoring == Scoring::SideOut {
            form = form.push(text("Only the serving side earns a point. Record every rally winner; service changes automatically.").size(12).color(c.muted));
        }
        form.into()
    }
    pub(super) fn sport_view(&self, c: Colors) -> Element<'_, Message> {
        self.operator_view(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setup_feedback_targets_numeric_fields_and_ignores_hidden_match_tiebreak() {
        let mut setup = SportSetup::new(Sport::Pickleball);
        setup.best_of = "2".into();
        setup.cap = "5".into();
        let issues = setup.issues();
        assert!(issues.iter().any(|(field, _)| *field == RuleField::BestOf));
        assert!(issues.iter().any(|(field, _)| *field == RuleField::Cap));
        assert!(setup.rules().is_err());
        let mut setup = SportSetup::new(Sport::Tennis);
        setup.match_tiebreak = true;
        setup.decider = "oops".into();
        assert!(setup.rules().is_err());
        setup.match_tiebreak = false;
        assert!(
            setup.rules().is_ok(),
            "a hidden unused field must not block creation"
        );
        let mut setup = SportSetup::new(Sport::Football);
        setup.target = "181".into();
        assert!(
            setup
                .issues()
                .iter()
                .any(|(field, _)| *field == RuleField::Target)
        );
    }

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
        for sport in Sport::ALL
            .into_iter()
            .filter(|s| *s != Sport::Basketball && !s.field())
        {
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
    fn tab_and_shift_tab_request_focus_operations_but_guard_modified_or_captured_keys() {
        let mut app = app(Store::open(":memory:").unwrap());
        for key in ["Tab", "Shift+Tab"] {
            let task = app.update(Message::Key {
                key: key.into(),
                repeat: false,
                modified: false,
                captured: false,
            });
            assert!(
                task.units() > 0,
                "{key} must request a widget focus operation"
            );
            for (repeat, modified, captured) in [
                (true, false, false),
                (false, true, false),
                (false, false, true),
            ] {
                let task = app.update(Message::Key {
                    key: key.into(),
                    repeat,
                    modified,
                    captured,
                });
                assert_eq!(task.units(), 0);
            }
        }
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
        assert!(app.error.is_none());
        assert!(app.match_form.attempted);
        assert!(app.sport_setup.issues().iter().any(|(field, error)| *field == RuleField::BestOf && error.contains("whole number")));
        assert!(app.active().is_none());
        assert_eq!(app.sport_setup.best_of, "abc");
        assert_eq!(app.screen, Screen::Matches);
    }
}
