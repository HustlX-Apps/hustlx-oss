use super::*;
#[cfg(test)]
use hustlx_oss::sport::{Command, Status};
use hustlx_oss::{
    sport::{Format, Rules, Scoring, Sport},
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
            period_minutes: if self.sport.field() {
                number(&self.target, "Period minutes")?
            } else {
                0
            },
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
        if f.sport.field() {
            return form.push(input("Minutes per regulation period","20",&f.target,RuleField::Target,c))
                .push(text("Two regulation periods. Extra time and shootout can be selected during the match. Local rules are advisory.").size(12).color(c.muted)).into();
        }
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
        self.operator_view(c)
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
