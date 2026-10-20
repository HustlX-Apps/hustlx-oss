use crate::brand::{self, BOLD, Colors, DISPLAY, HEADING, SEMIBOLD};
use hustlx_oss::session::Session;
use hustlx_oss::storage::{Entry, MatchSummary, RosterEntry, SavedMatch, Store};
use hustlx_oss::{
    Game, Side, Stat,
    clock::{ClockCommand, ClockState},
    hotkeys::Hotkeys,
    profile::{PlayerDetails, Position},
};
use iced::widget::{
    Column, Container, Space, button, column, container, row, scrollable, svg, text,
};
use iced::{Border, Color, Element, Fill, Length, Theme, alignment};
use std::{collections::BTreeMap, time::Instant};
#[path = "clock_ui.rs"]
mod clocks;
#[path = "library_ui.rs"]
mod library;
#[path = "lineup_ui.rs"]
mod lineups;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Matches,
    Teams,
    Players,
    GameDay,
}
#[derive(Debug, Clone)]
pub enum Message {
    OpenLineup(Side, hustlx_oss::lineup::LineupKind),
    CancelLineup,
    LineupPick(i64),
    LineupRemove(i64),
    ClearLineup,
    SaveLineup,
    ShowLineupHistory,
    Tick(Instant),
    Clock(ClockCommand),
    ToggleClock,
    Key {
        key: String,
        repeat: bool,
        modified: bool,
        captured: bool,
    },
    Close(iced::window::Id),
    CancelClose,
    ClearKeys,
    ToggleClockEditor,
    GameTime(String),
    ShotTime(String),
    ApplyGameTime,
    ApplyShotTime,
    ToggleHotkeys,
    DismissNotice,
    PlayerJersey(String),
    PlayerPosition(Position, bool),
    Select(Side, i64),
    Record(Stat),
    Undo,
    ToggleTheme,
    Navigate(Screen),
    Refresh,
    RetryStorage,
    RetrySave,
    OpenMatch(i64),
    CreateMatch,
    MatchName(String),
    MatchDate(String),
    MatchVenue(String),
    HomeTeam(Entry),
    AwayTeam(Entry),
    TeamName(String),
    SaveTeam,
    EditTeam(i64),
    CancelTeam,
    ArchiveTeam(i64, bool),
    ShowTeams(bool),
    PlayerName(String),
    SavePlayer,
    EditPlayer(i64),
    CancelPlayer,
    ArchivePlayer(i64, bool),
    ShowPlayers(bool),
    RosterTeam(Entry),
    RosterPlayer(Entry),
    Jersey(String),
    SaveRoster,
    RemoveRoster(i64),
    EditJersey(i64),
}

pub struct App {
    session: Option<Session>,
    dark: bool,
    screen: Screen,
    error: Option<String>,
    teams: Vec<Entry>,
    players: Vec<Entry>,
    matches: Vec<MatchSummary>,
    roster: Vec<RosterEntry>,
    match_form: library::MatchForm,
    team_name: String,
    editing_team: Option<i64>,
    show_teams: bool,
    player_name: String,
    editing_player: Option<i64>,
    show_players: bool,
    roster_team: Option<Entry>,
    roster_player: Option<Entry>,
    jersey: String,
    player_jersey: String,
    player_positions: Vec<Position>,
    details: BTreeMap<i64, PlayerDetails>,
    now: Instant,
    hotkeys: Hotkeys,
    show_hotkeys: bool,
    clock_edit: bool,
    game_time: String,
    shot_time: String,
    notice: Option<String>,
    closing: Option<iced::window::Id>,
    lineup_draft: Option<hustlx_oss::lineup::LineupDraft>,
    lineup_history: bool,
    lineup_saving: bool,
}
impl Default for App {
    fn default() -> Self {
        let mut app = Self {
            session: None,
            dark: true,
            screen: Screen::Matches,
            error: None,
            teams: vec![],
            players: vec![],
            matches: vec![],
            roster: vec![],
            match_form: library::MatchForm::default(),
            team_name: String::new(),
            editing_team: None,
            show_teams: false,
            player_name: String::new(),
            editing_player: None,
            show_players: false,
            roster_team: None,
            roster_player: None,
            jersey: String::new(),
            player_jersey: String::new(),
            player_positions: vec![],
            details: BTreeMap::new(),
            now: Instant::now(),
            hotkeys: Hotkeys::default(),
            show_hotkeys: false,
            clock_edit: false,
            game_time: String::new(),
            shot_time: String::new(),
            notice: None,
            closing: None,
            lineup_draft: None,
            lineup_history: false,
            lineup_saving: false,
        };
        if let Err(error) = app.connect() {
            app.error = Some(error.to_string());
        }
        app
    }
}
impl App {
    fn update_core(&mut self, message: Message) {
        let result = match message {
            message @ (Message::OpenLineup(..)
            | Message::CancelLineup
            | Message::LineupPick(..)
            | Message::LineupRemove(..)
            | Message::ClearLineup
            | Message::SaveLineup
            | Message::ShowLineupHistory) => self.update_lineups(message),
            Message::Select(side, id) => {
                if let Some(s) = &mut self.session {
                    s.select(side, id);
                    self.hotkeys.clear();
                }
                Ok(())
            }
            Message::Record(stat) => self.session_mut().and_then(|s| s.record(stat)),
            Message::Undo => self.session_mut().and_then(Session::undo),
            Message::RetrySave => self.session_mut().and_then(Session::retry),
            Message::ToggleTheme => {
                self.dark = !self.dark;
                Ok(())
            }
            other => self.update_library(other),
        };
        match result {
            Ok(()) => {
                if !self.pending() {
                    self.error = None;
                }
            }
            Err(error) => self.error = Some(error.to_string()),
        }
    }
    pub fn theme(&self) -> Theme {
        brand::theme(self.dark)
    }
    fn pending(&self) -> bool {
        self.session.as_ref().is_some_and(Session::has_pending_save)
    }
    fn active(&self) -> Option<&SavedMatch> {
        self.session.as_ref().and_then(Session::active)
    }
    fn game(&self) -> &Game {
        &self.active().expect("Game Day requires an open match").game
    }
    fn colors(&self) -> Colors {
        Colors::new(self.dark)
    }

    pub fn view(&self) -> Element<'_, Message> {
        let c = self.colors();
        let mut nav = row![
            svg(brand::wordmark(self.dark)).width(100).height(28),
            badge("OSS", c.away, c),
        ]
        .spacing(10)
        .align_y(alignment::Vertical::Center);
        for (label, screen) in [
            ("Matches", Screen::Matches),
            ("Teams", Screen::Teams),
            ("Players", Screen::Players),
        ] {
            nav = nav.push(
                button(text(label).size(12).font(SEMIBOLD))
                    .on_press(Message::Navigate(screen))
                    .padding([8, 12])
                    .style(if self.screen == screen {
                        button::primary
                    } else {
                        brand::secondary
                    }),
            );
        }
        if self.active().is_some() {
            nav = nav.push(
                button(text("Game Day").size(12))
                    .on_press(Message::Navigate(Screen::GameDay))
                    .style(if self.screen == Screen::GameDay {
                        button::primary
                    } else {
                        brand::secondary
                    })
                    .padding([8, 12]),
            );
        }
        if self.screen == Screen::GameDay {
            nav = nav.push(
                button(text("Hotkeys ?").size(12))
                    .on_press(Message::ToggleHotkeys)
                    .padding([8, 12])
                    .style(brand::secondary),
            );
        }
        nav = nav.push(Space::new().width(Fill)).push(
            button(
                text(if self.dark {
                    "Light theme"
                } else {
                    "Dark theme"
                })
                .size(12),
            )
            .on_press(Message::ToggleTheme)
            .padding([8, 12])
            .style(brand::secondary),
        );
        let mut page = column![nav].spacing(14);
        if let Some(notice) = &self.notice {
            page = page.push(
                row![
                    text(notice).size(11).color(c.muted).width(Fill),
                    button("Dismiss").on_press(Message::DismissNotice)
                ]
                .spacing(10),
            );
        }
        if let Some(error) = &self.error {
            let mut actions = row![text(error).size(12).color(c.red).width(Fill)]
                .spacing(10)
                .align_y(alignment::Vertical::Center);
            if self.pending() {
                actions =
                    actions.push(button(text("Retry save").size(12)).on_press(Message::RetrySave));
            }
            if self.closing.is_some() {
                actions = actions.push(button("Keep app open").on_press(Message::CancelClose));
            }
            page = page.push(card(actions, c).padding(10));
        }
        let body = if self.session.is_none() {
            card(column![text("Your library could not be opened").size(24).font(HEADING),
                text("Your existing data has not been replaced. Check the data folder, then retry.").size(14).color(c.muted),
                button("Retry opening library").on_press(Message::RetryStorage)].spacing(18),c).width(Fill).height(Fill).into()
        } else if self.screen == Screen::GameDay && self.active().is_some() {
            if self.lineup_draft.is_some() {
                self.lineup_editor(c)
            } else if self.lineup_history {
                self.lineup_history_view(c)
            } else if self.clock_edit {
                self.clock_editor(c)
            } else if self.show_hotkeys {
                self.shortcut_guide(c)
            } else {
                self.game_view(c)
            }
        } else {
            self.library_view(c)
        };
        page = page.push(body).push(
            row![
                text(if self.session.is_none() {
                    "STORAGE UNAVAILABLE"
                } else if self.pending() {
                    "SAVE PENDING"
                } else {
                    "SAVED ON THIS COMPUTER"
                })
                .size(10)
                .font(BOLD)
                .color(if self.session.is_none() || self.pending() {
                    c.red
                } else {
                    c.green
                }),
                text(if self.pending() {
                    "Retry the save, or reopen a match to discard the unsaved action."
                } else {
                    "Offline library. Clocks pause on close and reopen paused."
                })
                .size(11)
                .color(c.muted),
            ]
            .spacing(12),
        );
        container(page)
            .padding(20)
            .width(Fill)
            .height(Fill)
            .style(move |_| {
                container::Style::default()
                    .background(c.background)
                    .color(c.text)
            })
            .into()
    }
    fn game_view(&self, c: Colors) -> Element<'_, Message> {
        let saved = self.active().expect("Game Day requires an open match");
        let scoreboard = card(
            column![
                row![
                    column![
                        text(&saved.summary.name).size(15).font(HEADING),
                        text(format!(
                            "{}  /  {}",
                            saved.summary.date,
                            if saved.summary.venue.is_empty() {
                                "Venue not set"
                            } else {
                                &saved.summary.venue
                            }
                        ))
                        .size(11)
                        .color(c.muted)
                    ]
                    .spacing(4)
                    .width(Fill),
                    button(text("Lineup history").size(11))
                        .on_press(Message::ShowLineupHistory)
                        .style(brand::secondary),
                    badge("LOCAL MATCH", c.green, c)
                ],
                row![
                    self.score(Side::Home, c),
                    self.clock(c),
                    self.score(Side::Away, c)
                ]
                .spacing(18)
                .align_y(alignment::Vertical::Center),
            ]
            .spacing(8),
            c,
        )
        .padding([14, 22]);
        column![
            scoreboard,
            row![
                self.roster(Side::Home, c),
                self.command_center(c),
                self.roster(Side::Away, c)
            ]
            .spacing(14)
            .height(Fill)
        ]
        .spacing(14)
        .height(Fill)
        .into()
    }
    fn score(&self, side: Side, c: Colors) -> Element<'_, Message> {
        let accent = if side == Side::Home { c.home } else { c.away };
        let fouls: u32 = self
            .game()
            .team(side)
            .players
            .iter()
            .map(|p| self.game().player_stats(side, p.id).fouls)
            .sum();
        column![
            text(self.game().team(side).name.to_uppercase())
                .size(13)
                .font(HEADING)
                .color(accent),
            text(format!("{:02}", self.game().score(side)))
                .size(68)
                .font(DISPLAY),
            text(format!("{fouls} personal fouls"))
                .size(12)
                .color(c.muted),
        ]
        .width(Length::FillPortion(3))
        .align_x(if side == Side::Home {
            alignment::Horizontal::Left
        } else {
            alignment::Horizontal::Right
        })
        .into()
    }

    fn roster(&self, side: Side, c: Colors) -> Element<'_, Message> {
        let team = self.game().team(side);
        let accent = if side == Side::Home { c.home } else { c.away };
        let heading = row![
            text(&team.name).size(16).font(DISPLAY),
            Space::new().width(Fill),
            text(if side == Side::Home { "HOME" } else { "AWAY" })
                .size(10)
                .color(accent)
        ]
        .align_y(alignment::Vertical::Center);
        let mut players = Column::new().spacing(8);
        for on_court in [true, false] {
            players = players.push(
                container(
                    text(if on_court { "ON COURT" } else { "BENCH" })
                        .size(10)
                        .color(c.muted),
                )
                .padding([8, 2]),
            );
            for p in team.players.iter().filter(|p| p.on_court == on_court) {
                let stats = self.game().player_stats(side, p.id);
                let selected = self.game().selected == Some((side, p.id));
                let content = row![
                    text(jersey_label(&p.jersey))
                        .size(25)
                        .font(DISPLAY)
                        .color(accent)
                        .width(38),
                    column![
                        text(&p.name).size(14).font(DISPLAY),
                        text(format!(
                            "{}{}",
                            hustlx_oss::profile::encode(&p.positions),
                            if self
                                .active()
                                .and_then(|m| m.starting_lineup(side))
                                .is_some_and(|ids| ids.contains(&p.id))
                            {
                                "  / STARTER"
                            } else {
                                ""
                            }
                        ))
                        .size(10)
                        .color(c.muted),
                        text(format!(
                            "{} AST  /  {} REB",
                            stats.assists,
                            stats.offensive_rebounds + stats.defensive_rebounds
                        ))
                        .size(10)
                        .color(c.muted),
                    ]
                    .spacing(4)
                    .width(Fill),
                    text(stats.points).size(22).font(DISPLAY),
                    text(format!("PF {}", stats.fouls)).size(10).color(c.muted),
                ]
                .spacing(8)
                .align_y(alignment::Vertical::Center);
                players = players.push(
                    button(content)
                        .on_press_maybe((!self.pending()).then_some(Message::Select(side, p.id)))
                        .width(Fill)
                        .padding([12, 10])
                        .style(move |_, status| {
                            let active = selected || status == button::Status::Hovered;
                            button::Style {
                                background: Some(
                                    if active {
                                        blend(c.row, accent, 0.15)
                                    } else {
                                        c.row
                                    }
                                    .into(),
                                ),
                                text_color: c.text,
                                border: Border {
                                    color: if selected { accent } else { c.border },
                                    width: if selected { 1.5 } else { 1.0 },
                                    radius: 10.0.into(),
                                },
                                ..Default::default()
                            }
                        }),
                );
            }
        }
        card(
            column![
                heading,
                row![
                    button(text("Starters").size(11))
                        .on_press_maybe((!self.pending()).then_some(Message::OpenLineup(
                            side,
                            hustlx_oss::lineup::LineupKind::Starters
                        )))
                        .style(brand::secondary),
                    button(text("Subs X").size(11))
                        .on_press_maybe((!self.pending()).then_some(Message::OpenLineup(
                            side,
                            hustlx_oss::lineup::LineupKind::Substitution
                        )))
                        .style(brand::secondary),
                ]
                .spacing(8),
                text(format!(
                    "{} on court / starters {}",
                    self.game().court_ids(side).len(),
                    if self
                        .active()
                        .and_then(|m| m.starting_lineup(side))
                        .is_some()
                    {
                        "saved"
                    } else {
                        "not set"
                    }
                ))
                .size(10)
                .color(c.muted),
                scrollable(players.width(Fill))
                    .width(Fill)
                    .spacing(8)
                    .height(Fill)
            ]
            .spacing(14),
            c,
        )
        .width(Length::FillPortion(3))
        .height(Fill)
        .into()
    }

    fn command_center(&self, c: Colors) -> Element<'_, Message> {
        let target = self
            .game()
            .selected
            .and_then(|(side, id)| self.game().player(side, id));
        let title = if !self.hotkeys.digits().is_empty() || self.hotkeys.side().is_some() {
            format!(
                "{} #{}",
                self.hotkeys
                    .side()
                    .map_or("Jersey", |s| if s == Side::Home { "Home" } else { "Away" }),
                self.hotkeys.digits()
            )
        } else {
            match target {
                Some(player) => format!("Pad aimed at #{} {}", player.jersey, player.name),
                None => "Select a player or type a jersey".to_owned(),
            }
        };
        let pad = column![
            text(title).size(13).color(c.muted),
            row![
                self.stat_button(Stat::FreeThrowMade, c.green, c),
                self.stat_button(Stat::TwoMade, c.green, c),
                self.stat_button(Stat::ThreeMade, c.green, c)
            ]
            .spacing(6),
            row![
                self.stat_button(Stat::FreeThrowMiss, c.red, c),
                self.stat_button(Stat::TwoMiss, c.red, c),
                self.stat_button(Stat::ThreeMiss, c.red, c)
            ]
            .spacing(6),
            row![
                self.stat_button(Stat::OffensiveRebound, c.away, c),
                self.stat_button(Stat::DefensiveRebound, c.green, c),
                self.stat_button(Stat::Assist, c.home, c)
            ]
            .spacing(6),
            row![
                self.stat_button(Stat::Turnover, c.away, c),
                self.stat_button(Stat::Steal, c.home, c),
                self.stat_button(Stat::Block, c.home, c)
            ]
            .spacing(6),
            row![
                self.stat_button(Stat::Foul, c.red, c),
                self.stat_button(Stat::Technical, c.red, c),
                self.stat_button(Stat::Warning, c.muted, c)
            ]
            .spacing(6),
            row![
                self.stat_button(Stat::Unsportsmanlike, c.red, c),
                self.stat_button(Stat::Disqualifying, c.red, c)
            ]
            .spacing(6),
        ]
        .spacing(7);

        let undo = button(text("Undo last").size(11))
            .padding([7, 12])
            .on_press_maybe(
                (!self.game().plays().is_empty() && !self.pending()).then_some(Message::Undo),
            )
            .style(brand::secondary);
        let mut feed = Column::new().spacing(8);
        if self.game().plays().is_empty() {
            feed = feed.push(container(column![
                text("Ready for the first play").size(18).font(HEADING),
                text("Select a player, then record a shot or stat.\nYour saved plays will appear here.").size(13).color(c.muted),
            ].spacing(10)).padding([24, 8]));
        }
        for (index, play) in self.game().plays().iter().enumerate().rev() {
            let player = self
                .game()
                .player(play.side, play.player_id)
                .expect("preview plays contain known players");
            let accent = if play.side == Side::Home {
                c.home
            } else {
                c.away
            };
            feed = feed.push(
                container(
                    row![
                        text(format!("{:02}", index + 1))
                            .size(12)
                            .color(c.muted)
                            .width(26),
                        column![
                            text(play.stat.label()).size(12).font(BOLD).color(accent),
                            text(format!("#{} {}", player.jersey, player.name)).size(13),
                            text(play.clock_label()).size(10).color(c.muted),
                        ]
                        .spacing(5)
                        .width(Fill),
                        text(if play.stat.points() > 0 {
                            format!("+{}", play.stat.points())
                        } else {
                            String::new()
                        })
                        .size(17)
                        .font(HEADING)
                        .color(accent),
                    ]
                    .spacing(8)
                    .align_y(alignment::Vertical::Center),
                )
                .padding(12)
                .width(Fill)
                .style(move |_| {
                    container::Style::default()
                        .background(c.row)
                        .border(Border {
                            color: c.border,
                            width: 1.0,
                            radius: 10.0.into(),
                        })
                }),
            );
        }
        card(
            column![
                pad,
                row![
                    text(format!("PLAY-BY-PLAY  /  {}", self.game().plays().len()))
                        .size(10)
                        .color(c.muted),
                    Space::new().width(Fill),
                    undo
                ]
                .align_y(alignment::Vertical::Center),
                scrollable(feed.width(Fill))
                    .width(Fill)
                    .spacing(8)
                    .height(Fill),
            ]
            .spacing(16),
            c,
        )
        .width(Length::FillPortion(4))
        .height(Fill)
        .into()
    }

    fn stat_button(&self, stat: Stat, accent: Color, c: Colors) -> Element<'_, Message> {
        button(
            text(stat.label())
                .size(12)
                .font(BOLD)
                .width(Fill)
                .align_x(alignment::Horizontal::Center),
        )
        .on_press_maybe(
            self.game()
                .selected
                .filter(|_| !self.pending())
                .map(|_| Message::Record(stat)),
        )
        .padding([8, 4])
        .width(Fill)
        .style(move |_, status| {
            let disabled = status == button::Status::Disabled;
            button::Style {
                background: Some(
                    blend(
                        c.panel,
                        accent,
                        if disabled {
                            0.06
                        } else if status == button::Status::Hovered {
                            0.28
                        } else {
                            0.16
                        },
                    )
                    .into(),
                ),
                text_color: if disabled { c.muted } else { accent },
                border: Border {
                    color: blend(c.border, accent, if disabled { 0.08 } else { 0.3 }),
                    width: 1.0,
                    radius: 7.0.into(),
                },
                ..Default::default()
            }
        })
        .into()
    }
}

fn card<'a>(content: impl Into<Element<'a, Message>>, c: Colors) -> Container<'a, Message> {
    container(content).padding(14).style(move |_| {
        container::Style::default()
            .background(c.panel)
            .color(c.text)
            .border(Border {
                color: c.border,
                width: 1.0,
                radius: 14.0.into(),
            })
    })
}

fn badge<'a>(label: &'a str, accent: Color, c: Colors) -> Element<'a, Message> {
    container(text(label).size(10).font(BOLD).color(accent))
        .padding([6, 10])
        .style(move |_| {
            container::Style::default()
                .background(blend(c.panel, accent, 0.12))
                .border(Border {
                    color: blend(c.border, accent, 0.3),
                    width: 1.0,
                    radius: 20.0.into(),
                })
        })
        .into()
}

fn blend(base: Color, accent: Color, amount: f32) -> Color {
    Color::from_rgb(
        base.r + (accent.r - base.r) * amount,
        base.g + (accent.g - base.g) * amount,
        base.b + (accent.b - base.b) * amount,
    )
}

fn jersey_label(jersey: &str) -> &str {
    if jersey.is_empty() { "?" } else { jersey }
}
