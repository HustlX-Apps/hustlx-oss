use super::*;
use chrono::{Local, NaiveDate};
use hustlx_oss::sport::Sport;
use hustlx_oss::storage::{Error, MatchDraft, Result, SportMatchDraft};
use iced::widget::{checkbox, column, pick_list, text_input};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MatchField {
    Name,
    Date,
    Venue,
    Home,
    Away,
    Rules,
}

pub(super) struct MatchForm {
    pub(super) attempted: bool,
    name: String,
    date: String,
    venue: String,
    home: Option<Entry>,
    away: Option<Entry>,
}
impl Default for MatchForm {
    fn default() -> Self {
        Self {
            attempted: false,
            name: String::new(),
            date: Local::now().format("%Y-%m-%d").to_string(),
            venue: String::new(),
            home: None,
            away: None,
        }
    }
}
impl App {
    pub(super) fn connect(&mut self) -> Result<()> {
        let store = Store::open(Store::default_path()?)?;
        self.session = Some(Session::new(store));
        self.refresh()
    }
    fn store(&self) -> Result<&Store> {
        self.session
            .as_ref()
            .map(Session::store)
            .ok_or_else(|| Error::Validation("Open the local library first.".into()))
    }
    pub(super) fn session_mut(&mut self) -> Result<&mut Session> {
        self.session
            .as_mut()
            .ok_or_else(|| Error::Validation("Open the local library first.".into()))
    }
    fn refresh(&mut self) -> Result<()> {
        let db = self.store()?;
        let teams = db.teams(true)?;
        let players = db.players(true)?;
        let matches = db.matches()?;
        let selected = self
            .roster_team
            .as_ref()
            .and_then(|old| teams.iter().find(|t| t.id == old.id && !t.archived))
            .cloned();
        let roster = match &selected {
            Some(team) => db.roster(team.id)?,
            None => vec![],
        };
        for selection in [&mut self.match_form.home, &mut self.match_form.away] {
            *selection = selection
                .as_ref()
                .and_then(|old| teams.iter().find(|t| t.id == old.id && !t.archived))
                .cloned();
        }
        self.roster_player = self
            .roster_player
            .as_ref()
            .and_then(|old| players.iter().find(|p| p.id == old.id && !p.archived))
            .cloned();
        self.teams = teams;
        self.details = players
            .iter()
            .map(|p| Ok((p.id, self.store()?.player_details(p.id)?)))
            .collect::<Result<_>>()?;
        self.players = players;
        self.matches = matches;
        self.roster_team = selected;
        self.roster = roster;
        Ok(())
    }
    pub(super) fn update_library(&mut self, message: Message) -> Result<()> {
        match message {
            Message::Navigate(screen) => {
                if screen != Screen::GameDay && !self.pending() {
                    self.session_mut()?.pause_for_close_at(Instant::now())?;
                }
                self.lineup_draft = None;
                self.lineup_history = false;
                self.hotkeys.clear();
                self.clock_edit = false;
                self.show_hotkeys = false;
                self.refresh()?;
                self.screen = screen;
            }
            Message::Refresh => self.refresh()?,
            Message::RetryStorage => self.connect()?,
            Message::OpenMatch(id) => {
                self.operator_draft = None;
                self.operator_chooser = None;
                self.operator_finish = false;
                self.operator_time.clear();
                self.session_mut()?.open(id)?;
                self.lineup_draft = None;
                self.lineup_history = false;
                self.lineup_saving = false;
                self.clock_edit = false;
                self.show_hotkeys = false;
                self.hotkeys.clear();
                self.screen = Screen::GameDay;
            }
            Message::CreateMatch => {
                self.match_form.attempted = true;
                if !self.match_issues().is_empty() {
                    return Ok(());
                }
                self.operator_draft = None;
                self.operator_chooser = None;
                self.operator_finish = false;
                self.operator_time.clear();
                if self.pending() {
                    return Err(Error::Validation(
                        "Retry the pending save or reopen the match before creating another match."
                            .into(),
                    ));
                }
                let f = &self.match_form;
                if self.sport_setup.sport == Sport::Basketball {
                    let draft = MatchDraft {
                        name: f.name.clone(),
                        date: f.date.clone(),
                        venue: f.venue.clone(),
                        home_team_id: f.home.as_ref().map_or(0, |t| t.id),
                        away_team_id: f.away.as_ref().map_or(0, |t| t.id),
                    };
                    self.session_mut()?.create(&draft)?;
                } else {
                    let draft = SportMatchDraft {
                        name: f.name.clone(),
                        date: f.date.clone(),
                        venue: f.venue.clone(),
                        home_name: self.sport_setup.home.clone(),
                        away_name: self.sport_setup.away.clone(),
                        sport: self.sport_setup.sport,
                        rules: self.sport_setup.rules()?,
                    };
                    let teams = [f.home.as_ref().map(|t| t.id), f.away.as_ref().map(|t| t.id)];
                    self.session_mut()?.create_sport_with_teams(&draft, teams)?;
                }
                self.lineup_draft = None;
                self.lineup_history = false;
                self.lineup_saving = false;
                self.clock_edit = false;
                self.show_hotkeys = false;
                self.hotkeys.clear();
                self.match_form = MatchForm::default();
                self.sport_setup = sports::SportSetup::default();
                self.screen = Screen::GameDay;
            }
            Message::ClearMatchRoster(side) => match side {
                Side::Home => self.match_form.home = None,
                Side::Away => self.match_form.away = None,
            },
            Message::MatchName(value) => self.match_form.name = value,
            Message::MatchDate(value) => self.match_form.date = value,
            Message::MatchVenue(value) => self.match_form.venue = value,
            Message::HomeTeam(team) => {
                self.sport_setup.home = team.name.clone();
                self.match_form.home = Some(team);
            }
            Message::AwayTeam(team) => {
                self.sport_setup.away = team.name.clone();
                self.match_form.away = Some(team);
            }
            Message::TeamName(value) => self.team_name = value,
            Message::SaveTeam => {
                self.team_attempted = true;
                if text_issue(&self.team_name, "Team name", true, 200).is_some() {
                    return Ok(());
                }
                let id = if let Some(id) = self.editing_team {
                    self.store()?.rename_team(id, &self.team_name)?;
                    id
                } else {
                    self.store()?.create_team(&self.team_name)?
                };
                self.team_attempted = false;
                self.team_name.clear();
                self.editing_team = None;
                self.refresh()?;
                self.roster_team = self
                    .teams
                    .iter()
                    .find(|t| t.id == id && !t.archived)
                    .cloned();
                self.refresh()?;
            }
            Message::EditTeam(id) => {
                self.team_attempted = false;
                if let Some(team) = self.teams.iter().find(|t| t.id == id) {
                    self.team_name = team.name.clone();
                    self.editing_team = Some(id);
                }
            }
            Message::CancelTeam => {
                self.team_attempted = false;
                self.editing_team = None;
                self.team_name.clear();
            }
            Message::ArchiveTeam(id, archived) => {
                self.store()?.archive_team(id, archived)?;
                self.refresh()?;
            }
            Message::ShowTeams(value) => self.show_teams = value,
            Message::PlayerName(value) => self.player_name = value,
            Message::PlayerJersey(value) => self.player_jersey = value,
            Message::PlayerPosition(position, enabled) => {
                self.player_positions.retain(|p| *p != position);
                if enabled {
                    self.player_positions.push(position);
                }
            }
            Message::SavePlayer => {
                self.player_attempted = true;
                if text_issue(&self.player_name, "Player name", true, 200).is_some()
                    || text_issue(&self.player_jersey, "Jersey", false, 12).is_some()
                {
                    return Ok(());
                }
                let name = self.player_name.clone();
                let jersey = self.player_jersey.clone();
                let positions = self.player_positions.clone();
                let id = self.editing_player;
                self.session_mut()?
                    .store_mut()
                    .save_player(id, &name, &jersey, &positions)?;
                self.player_attempted = false;
                self.player_jersey.clear();
                self.player_positions.clear();
                self.player_name.clear();
                self.editing_player = None;
                self.refresh()?;
            }
            Message::EditPlayer(id) => {
                self.player_attempted = false;
                if let Some(player) = self.players.iter().find(|p| p.id == id) {
                    self.player_name = player.name.clone();
                    let details = self.details.get(&id).cloned().unwrap_or_default();
                    self.player_jersey = details.jersey;
                    self.player_positions = details.positions;
                    self.editing_player = Some(id);
                }
            }
            Message::CancelPlayer => {
                self.player_attempted = false;
                self.editing_player = None;
                self.player_name.clear();
                self.player_jersey.clear();
                self.player_positions.clear();
            }
            Message::ArchivePlayer(id, archived) => {
                self.store()?.archive_player(id, archived)?;
                self.refresh()?;
            }
            Message::ShowPlayers(value) => self.show_players = value,
            Message::RosterTeam(team) => {
                self.roster_team = Some(team);
                self.roster_player = None;
                self.jersey.clear();
                self.refresh()?;
            }
            Message::RosterPlayer(player) => {
                self.jersey = self
                    .roster
                    .iter()
                    .find(|p| p.player_id == player.id)
                    .map_or_else(String::new, |p| p.jersey.clone());
                self.roster_player = Some(player);
            }
            Message::Jersey(value) => self.jersey = value,
            Message::SaveRoster => {
                let team = self
                    .roster_team
                    .as_ref()
                    .ok_or_else(|| Error::Validation("Choose a team first.".into()))?
                    .id;
                let player = self
                    .roster_player
                    .as_ref()
                    .ok_or_else(|| Error::Validation("Choose a player first.".into()))?
                    .id;
                self.store()?.add_to_team(team, player, &self.jersey)?;
                self.roster_player = None;
                self.jersey.clear();
                self.refresh()?;
            }
            Message::RemoveRoster(player) => {
                if let Some(team) = &self.roster_team {
                    self.store()?.remove_from_team(team.id, player)?;
                    self.refresh()?;
                }
            }
            Message::EditJersey(id) => {
                self.roster_player = self
                    .players
                    .iter()
                    .find(|p| p.id == id && !p.archived)
                    .cloned();
                self.jersey = self
                    .roster
                    .iter()
                    .find(|p| p.player_id == id)
                    .map_or_else(String::new, |p| p.jersey.clone());
            }
            _ => {}
        }
        Ok(())
    }
    pub(super) fn library_view(&self, c: Colors) -> Element<'_, Message> {
        match self.screen {
            Screen::Teams => self.teams_view(c),
            Screen::Players => self.players_view(c),
            _ => self.matches_view(c),
        }
    }
    fn match_issues(&self) -> Vec<(MatchField, String)> {
        let f = &self.match_form;
        let mut issues = Vec::new();
        for (field, value, label, required, limit) in [
            (MatchField::Name, f.name.as_str(), "Match name", true, 200),
            (MatchField::Venue, f.venue.as_str(), "Venue", false, 300),
        ] {
            if let Some(error) = text_issue(value, label, required, limit) {
                issues.push((field, error));
            }
        }
        let date = f.date.trim();
        if date.len() != 10 || NaiveDate::parse_from_str(date, "%Y-%m-%d").is_err() {
            issues.push((MatchField::Date, "Enter a real date as YYYY-MM-DD.".into()));
        }
        if self.sport_setup.sport == Sport::Basketball {
            for (field, team) in [(MatchField::Home, &f.home), (MatchField::Away, &f.away)] {
                if team.is_none() {
                    issues.push((field, "Choose a team from your library.".into()));
                }
            }
            if f.home
                .as_ref()
                .zip(f.away.as_ref())
                .is_some_and(|(a, b)| a.id == b.id)
            {
                issues.push((
                    MatchField::Away,
                    "Choose a different team for each side.".into(),
                ));
            }
        } else {
            for (field, value, label) in [
                (MatchField::Home, &self.sport_setup.home, "Home name"),
                (MatchField::Away, &self.sport_setup.away, "Away name"),
            ] {
                if let Some(error) = text_issue(value, label, true, 200) {
                    issues.push((field, error));
                }
            }
            if let Err(error) = self.sport_setup.rules() {
                issues.push((MatchField::Rules, error.to_string()));
            }
        }
        issues
    }
    fn matches_view(&self, c: Colors) -> Element<'_, Message> {
        let f = &self.match_form;
        let issues = self.match_issues();
        let error = |key| {
            f.attempted
                .then(|| {
                    issues
                        .iter()
                        .find(|(field, _)| *field == key)
                        .map(|(_, e)| e.clone())
                })
                .flatten()
        };
        let home_choices = self
            .teams
            .iter()
            .filter(|t| !t.archived && f.away.as_ref().is_none_or(|away| away.id != t.id))
            .cloned()
            .collect::<Vec<_>>();
        let away_choices = self
            .teams
            .iter()
            .filter(|t| !t.archived && f.home.as_ref().is_none_or(|home| home.id != t.id))
            .cloned()
            .collect::<Vec<_>>();
        let mut form = column![
            field(
                "Sport",
                pick_list(
                    Sport::ALL,
                    Some(self.sport_setup.sport),
                    Message::ChooseSport
                )
                .width(Fill)
                .text_size(14)
                .padding(11)
                .style(brand::select)
                .into(),
                c
            ),
            feedback(
                "Match name",
                text_input("e.g. Evening friendly", &f.name)
                    .on_input(Message::MatchName)
                    .size(14)
                    .padding(11)
                    .style(brand::input)
                    .into(),
                error(MatchField::Name),
                c
            ),
            pair(
                feedback(
                    "Date / YYYY-MM-DD",
                    text_input("YYYY-MM-DD", &f.date)
                        .on_input(Message::MatchDate)
                        .size(14)
                        .padding(11)
                        .style(brand::input)
                        .into(),
                    error(MatchField::Date),
                    c
                ),
                feedback(
                    "Venue (optional)",
                    text_input("Court or venue", &f.venue)
                        .on_input(Message::MatchVenue)
                        .size(14)
                        .padding(11)
                        .style(brand::input)
                        .into(),
                    error(MatchField::Venue),
                    c
                ),
            ),
            section(
                "Participants",
                "Names and rosters are saved with this match.",
                c
            ),
        ]
        .spacing(16);
        if self.sport_setup.sport == Sport::Basketball {
            form = form.push(pair(
                feedback(
                    "Team A / home",
                    pick_list(home_choices, f.home.as_ref(), Message::HomeTeam)
                        .placeholder("Choose a team")
                        .text_size(14)
                        .style(brand::select)
                        .padding(11)
                        .width(Fill)
                        .into(),
                    error(MatchField::Home),
                    c,
                ),
                feedback(
                    "Team B / away",
                    pick_list(away_choices, f.away.as_ref(), Message::AwayTeam)
                        .placeholder("Choose a team")
                        .text_size(14)
                        .style(brand::select)
                        .padding(11)
                        .width(Fill)
                        .into(),
                    error(MatchField::Away),
                    c,
                ),
            ));
            if self.teams.iter().filter(|t| !t.archived).count() < 2 {
                form = form
                    .push(
                        text("Basketball needs two saved teams. Empty rosters are welcome.")
                            .size(12)
                            .color(c.muted),
                    )
                    .push(
                        button("Add teams")
                            .on_press(Message::Navigate(Screen::Teams))
                            .style(brand::secondary)
                            .padding([10, 14]),
                    );
            } else {
                form = form.push(text("Active players are copied into the match. Later library edits keep its history intact.").size(12).color(c.muted));
            }
        } else {
            let (home_label, away_label) = self.sport_setup.side_labels();
            form = form.push(pair(
                feedback(
                    home_label,
                    text_input("Home name", &self.sport_setup.home)
                        .on_input(|v| Message::SportInput(sports::RuleField::Home, v))
                        .size(14)
                        .padding(11)
                        .style(brand::input)
                        .into(),
                    error(MatchField::Home),
                    c,
                ),
                feedback(
                    away_label,
                    text_input("Away name", &self.sport_setup.away)
                        .on_input(|v| Message::SportInput(sports::RuleField::Away, v))
                        .size(14)
                        .padding(11)
                        .style(brand::input)
                        .into(),
                    error(MatchField::Away),
                    c,
                ),
            ));
            form = form
                .push(pair(
                    roster_field(
                        "Home roster (optional)",
                        home_choices,
                        f.home.as_ref(),
                        Side::Home,
                        c,
                    ),
                    roster_field(
                        "Away roster (optional)",
                        away_choices,
                        f.away.as_ref(),
                        Side::Away,
                        c,
                    ),
                ))
                .push(
                    text("Add a saved roster for player actions, or score with names only.")
                        .size(12)
                        .color(c.muted),
                )
                .push(section(
                    "Match rules",
                    "Defaults are ready. Adjust them to your agreed format.",
                    c,
                ))
                .push(self.sport_setup_view(c));
        }
        let ready = issues.is_empty();
        let footer = column![
            text(if self.pending() {
                "Resolve the pending save before creating another match."
            } else if f.attempted && !ready {
                "Review the highlighted fields above."
            } else if ready {
                "Ready to create. Saved on this computer."
            } else {
                "Complete the match details to begin."
            })
            .size(12)
            .color(if f.attempted && !ready {
                c.red
            } else {
                c.muted
            }),
            button(text("Create match").size(14).font(SEMIBOLD))
                .on_press_maybe((!self.pending()).then_some(Message::CreateMatch))
                .padding([12, 18])
                .width(Fill),
        ]
        .spacing(9);
        let mut list = column![
            row![
                text("Your matches").size(23).font(HEADING),
                Space::new().width(Fill),
                button(text("Refresh").size(12)).on_press(Message::Refresh)
            ]
            .align_y(alignment::Vertical::Center),
            text("Open a saved match to continue or review its result.")
                .size(12)
                .color(c.muted)
        ]
        .spacing(14);
        if self.matches.is_empty() {
            list = list.push(empty(
                "No matches yet",
                "Choose a sport to begin. Basketball uses your team library; other sports need only side names.",
                c,
            ));
        }
        for m in &self.matches {
            list = list.push(card(
                row![
                    column![
                        text(&m.name).size(17).font(HEADING),
                        text(m.sport.label()).size(11).color(c.home),
                        text(format!("{} vs {}", m.home_name, m.away_name)).size(14),
                        text(format!(
                            "{} / {}",
                            m.date,
                            if m.venue.is_empty() {
                                "Venue not set"
                            } else {
                                &m.venue
                            }
                        ))
                        .size(12)
                        .color(c.muted)
                    ]
                    .spacing(7)
                    .width(Fill),
                    button("Open match")
                        .on_press(Message::OpenMatch(m.id))
                        .padding([10, 14]),
                ]
                .spacing(12)
                .align_y(alignment::Vertical::Center),
                c,
            ));
        }
        row![
            card(
                column![
                    column![
                        text("Create a match").size(23).font(HEADING),
                        text("Set up your match. Keep every point on this device.")
                            .size(12)
                            .color(c.muted)
                    ]
                    .spacing(5),
                    scrollable(form).spacing(8).height(Fill),
                    footer,
                ]
                .spacing(18)
                .height(Fill),
                c
            )
            .padding(20)
            .width(Length::FillPortion(6))
            .height(Fill),
            card(scrollable(list).spacing(8).height(Fill), c)
                .padding(20)
                .width(Length::FillPortion(5))
                .height(Fill),
        ]
        .spacing(16)
        .height(Fill)
        .into()
    }
    fn teams_view(&self, c: Colors) -> Element<'_, Message> {
        let visible_teams = self.teams.iter().filter(|t| self.show_teams || !t.archived);
        let count = visible_teams.clone().count();
        let mut controls = row![
            button(
                text(if self.editing_team.is_some() {
                    "Save team"
                } else {
                    "Create team"
                })
                .size(13)
            )
            .on_press(Message::SaveTeam)
            .padding([10, 14])
        ]
        .spacing(8);
        if self.editing_team.is_some() {
            controls = controls.push(
                button(text("Cancel").size(13))
                    .on_press(Message::CancelTeam)
                    .padding([10, 14])
                    .style(brand::secondary),
            );
        }
        let mut list = column![].spacing(8).width(Fill);
        for team in visible_teams {
            let selected = self.roster_team.as_ref().is_some_and(|t| t.id == team.id);
            list = list.push(
                container(
                    column![
                        button(
                            column![
                                text(&team.name)
                                    .color(if team.archived { c.muted } else { c.text })
                                    .size(15)
                                    .font(SEMIBOLD)
                                    .wrapping(iced::widget::text::Wrapping::WordOrGlyph),
                                text(if team.archived {
                                    "Archived"
                                } else if selected {
                                    "Viewing roster"
                                } else {
                                    "View roster"
                                })
                                .size(11)
                                .color(if selected {
                                    c.home
                                } else {
                                    c.muted
                                }),
                            ]
                            .spacing(5)
                            .width(Fill)
                        )
                        .on_press_maybe(
                            (!team.archived).then_some(Message::RosterTeam(team.clone()))
                        )
                        .width(Fill)
                        .padding(0)
                        .style(button::text),
                        row![
                            button(text("Rename").size(12))
                                .on_press(Message::EditTeam(team.id))
                                .padding([7, 10])
                                .style(brand::secondary),
                            button(
                                text(if team.archived { "Restore" } else { "Archive" }).size(12)
                            )
                            .on_press(Message::ArchiveTeam(team.id, !team.archived))
                            .padding([7, 10])
                            .style(brand::secondary),
                        ]
                        .spacing(8),
                    ]
                    .spacing(12)
                    .width(Fill),
                )
                .padding(14)
                .width(Fill)
                .style(move |_| {
                    container::Style::default()
                        .background(if selected {
                            blend(c.panel, c.home, 0.07)
                        } else {
                            c.panel
                        })
                        .color(c.text)
                        .border(Border {
                            color: if selected { c.home } else { c.border },
                            width: 1.0,
                            radius: 10.0.into(),
                        })
                }),
            );
        }
        if count == 0 {
            list = list.push(empty(
                "No teams to show",
                "Create a team or show archived teams to restore one.",
                c,
            ));
        }
        let sidebar = column![
            row![
                text("Teams").size(23).font(HEADING),
                Space::new().width(Fill),
                badge(format!("{count} teams"), c.home, c)
            ]
            .align_y(alignment::Vertical::Center),
            feedback(
                if self.editing_team.is_some() {
                    "Rename team"
                } else {
                    "Add a team"
                },
                text_input("Team name", &self.team_name)
                    .on_input(Message::TeamName)
                    .style(brand::input)
                    .on_submit(Message::SaveTeam)
                    .size(14)
                    .padding(10)
                    .into(),
                self.team_attempted
                    .then(|| text_issue(&self.team_name, "Team name", true, 200))
                    .flatten(),
                c
            ),
            controls,
            checkbox(self.show_teams)
                .label("Show archived teams")
                .text_size(12)
                .on_toggle(Message::ShowTeams),
            scrollable(list).spacing(8).height(Fill),
        ]
        .spacing(16)
        .height(Fill);
        let choices = self
            .teams
            .iter()
            .filter(|t| !t.archived)
            .cloned()
            .collect::<Vec<_>>();
        let player_choices = self
            .players
            .iter()
            .filter(|p| !p.archived)
            .cloned()
            .collect::<Vec<_>>();
        let mut roster =
            column![
            row![text("Team roster").size(23).font(HEADING), Space::new().width(Fill),
                badge(format!("{} players", self.roster.len()), c.home, c)]
                .align_y(alignment::Vertical::Center),
            field("Team", pick_list(choices, self.roster_team.as_ref(), Message::RosterTeam)
                .placeholder("Choose a team").text_size(14).style(brand::select)
                .width(Fill).padding(10).into(), c),
            text("Roster changes apply to new matches. Saved matches keep their original rosters.")
                .size(12).color(c.muted),
        ]
            .spacing(16);
        if self.roster_team.is_some() {
            let editing = self
                .roster_player
                .as_ref()
                .is_some_and(|player| self.roster.iter().any(|p| p.player_id == player.id));
            roster = roster.push(
                container(
                    column![
                        text(if editing {
                            "Update roster player"
                        } else {
                            "Add a player"
                        })
                        .size(15)
                        .font(SEMIBOLD),
                        row![
                            container(field(
                                "Player",
                                pick_list(
                                    player_choices,
                                    self.roster_player.as_ref(),
                                    Message::RosterPlayer
                                )
                                .placeholder("Choose an existing player")
                                .text_size(14)
                                .style(brand::select)
                                .width(Fill)
                                .padding(10)
                                .into(),
                                c
                            ))
                            .width(Length::FillPortion(3)),
                            container(field(
                                "Jersey label (optional)",
                                text_input("e.g. 12", &self.jersey)
                                    .on_input(Message::Jersey)
                                    .style(brand::input)
                                    .size(14)
                                    .padding(10)
                                    .into(),
                                c
                            ))
                            .width(Length::FillPortion(2)),
                        ]
                        .spacing(12),
                        button(
                            text(if editing {
                                "Update player"
                            } else {
                                "Add to roster"
                            })
                            .size(13)
                        )
                        .on_press_maybe(self.roster_player.as_ref().map(|_| Message::SaveRoster))
                        .padding([10, 14]),
                    ]
                    .spacing(12)
                    .width(Fill),
                )
                .padding(16)
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
            let mut members = column![].spacing(8).width(Fill);
            if self.roster.is_empty() {
                members = members.push(empty(
                    "No players on this roster",
                    "Create players in the Players tab, then add them here.",
                    c,
                ));
            }
            for p in &self.roster {
                let positions = hustlx_oss::profile::encode(&p.positions);
                members = members.push(
                    container(
                        row![
                            container(
                                text(jersey_label(&p.jersey))
                                    .size(if p.jersey.chars().count() > 3 { 13 } else { 22 })
                                    .font(HEADING)
                                    .color(c.home)
                                    .wrapping(iced::widget::text::Wrapping::WordOrGlyph)
                                    .width(Fill)
                                    .align_x(alignment::Horizontal::Center)
                            )
                            .padding([10, 6])
                            .width(76)
                            .clip(true)
                            .style(move |_| container::Style::default()
                                .background(blend(c.panel, c.home, 0.08))
                                .border(Border {
                                    radius: 8.0.into(),
                                    ..Border::default()
                                })),
                            column![
                                text(&p.name)
                                    .size(15)
                                    .font(SEMIBOLD)
                                    .wrapping(iced::widget::text::Wrapping::WordOrGlyph),
                                text(if positions.is_empty() {
                                    "No positions set".to_owned()
                                } else {
                                    positions
                                })
                                .size(11)
                                .color(c.muted),
                            ]
                            .spacing(5)
                            .width(Fill),
                            button(text("Edit jersey").size(12))
                                .on_press(Message::EditJersey(p.player_id))
                                .padding([8, 10])
                                .style(brand::secondary),
                            button(text("Remove").size(12))
                                .on_press(Message::RemoveRoster(p.player_id))
                                .padding([8, 10])
                                .style(brand::secondary),
                        ]
                        .spacing(12)
                        .align_y(alignment::Vertical::Center),
                    )
                    .padding(12)
                    .width(Fill)
                    .style(move |_| {
                        container::Style::default()
                            .background(c.panel)
                            .border(Border {
                                color: c.border,
                                width: 1.0,
                                radius: 10.0.into(),
                            })
                    }),
                );
            }
            roster = roster.push(scrollable(members).spacing(8).height(Fill));
        } else {
            roster = roster.push(empty(
                "Choose a team",
                "Select a team on the left to manage its roster.",
                c,
            ));
        }
        row![
            card(sidebar, c).width(340).height(Fill),
            card(roster.height(Fill), c).width(Fill).height(Fill),
        ]
        .spacing(16)
        .height(Fill)
        .into()
    }
    fn players_view(&self, c: Colors) -> Element<'_, Message> {
        let mut controls = row![
            button(if self.editing_player.is_some() {
                "Save player"
            } else {
                "Create player"
            })
            .on_press(Message::SavePlayer)
            .padding([10, 14])
        ]
        .spacing(8);
        if self.editing_player.is_some() {
            controls = controls.push(button("Cancel").on_press(Message::CancelPlayer));
        }
        let form = column![
            text(if self.editing_player.is_some() { "Edit player" } else { "Add a player" }).size(23).font(HEADING),
            text("Save a player once, then add them to your team rosters.").size(12).color(c.muted),
            feedback("Player name",text_input("Full name",&self.player_name).on_input(Message::PlayerName).on_submit(Message::SavePlayer).size(14).padding(11).style(brand::input).into(),self.player_attempted.then(|| text_issue(&self.player_name,"Player name",true,200)).flatten(),c),
            feedback("Default jersey (optional)",text_input("e.g. 00",&self.player_jersey).on_input(Message::PlayerJersey).on_submit(Message::SavePlayer).size(14).padding(11).style(brand::input).into(),self.player_attempted.then(|| text_issue(&self.player_jersey,"Jersey",false,12)).flatten(),c),
            text("Up to 12 characters. Blank team jerseys use this default.").size(12).color(c.muted),
            text("Basketball positions (optional)").size(12).font(SEMIBOLD),
            row(Position::ALL.into_iter().map(|p|checkbox(self.player_positions.contains(&p)).label(p.code()).text_size(13).on_toggle(move|enabled|Message::PlayerPosition(p,enabled)).into()).collect::<Vec<_>>()).spacing(12),
            controls,
            text("Saved matches keep their original player details. Archived players can be restored.").size(12).color(c.muted),
        ].spacing(16);
        let mut list = column![
            text("Player library").size(23).font(HEADING),
            checkbox(self.show_players)
                .label("Show archived players")
                .on_toggle(Message::ShowPlayers)
        ]
        .spacing(14);
        if self.players.is_empty() {
            list = list.push(empty(
                "No players yet",
                "Add a player, then assign them to a team in Teams.",
                c,
            ));
        }
        for p in self
            .players
            .iter()
            .filter(|p| self.show_players || !p.archived)
        {
            list = list.push(card(
                row![
                    column![
                        text(&p.name).size(17).font(HEADING),
                        text(
                            self.details
                                .get(&p.id)
                                .map_or_else(String::new, |d| format!(
                                    "#{}  {}",
                                    jersey_label(&d.jersey),
                                    hustlx_oss::profile::encode(&d.positions)
                                ))
                        )
                        .size(12)
                        .color(c.muted),
                        text(format!(
                            "Player #{}{}",
                            p.id,
                            if p.archived { " / archived" } else { "" }
                        ))
                        .size(11)
                        .color(c.muted)
                    ]
                    .spacing(4)
                    .width(Fill),
                    button(text("Edit").size(12)).on_press(Message::EditPlayer(p.id)),
                    button(text(if p.archived { "Restore" } else { "Archive" }).size(12))
                        .on_press(Message::ArchivePlayer(p.id, !p.archived))
                ]
                .spacing(10)
                .align_y(alignment::Vertical::Center),
                c,
            ));
        }
        row![
            card(scrollable(form).spacing(8).height(Fill), c)
                .width(380)
                .height(Fill),
            card(scrollable(list).spacing(8).height(Fill), c)
                .width(Fill)
                .height(Fill)
        ]
        .spacing(16)
        .height(Fill)
        .into()
    }
}
pub(super) fn field<'a>(
    label: &'a str,
    input: Element<'a, Message>,
    c: Colors,
) -> Element<'a, Message> {
    column![text(label).size(12).font(SEMIBOLD).color(c.text), input]
        .spacing(6)
        .into()
}
fn empty<'a>(title: &'a str, description: &'a str, c: Colors) -> Element<'a, Message> {
    container(
        column![
            text(title).size(18).font(HEADING),
            text(description).size(13).color(c.muted)
        ]
        .spacing(10),
    )
    .padding([20, 6])
    .into()
}

fn text_issue(value: &str, label: &str, required: bool, limit: usize) -> Option<String> {
    if required && value.trim().is_empty() {
        Some(format!("{label} is required."))
    } else if value.trim().chars().count() > limit {
        Some(format!("Use {limit} characters or fewer."))
    } else {
        None
    }
}
pub(super) fn feedback<'a>(
    label: &'a str,
    input: Element<'a, Message>,
    error: Option<String>,
    c: Colors,
) -> Element<'a, Message> {
    let mut field = column![field(label, input, c)].spacing(6).width(Fill);
    if let Some(error) = error {
        field = field.push(text(error).size(12).color(c.red));
    }
    field.into()
}
pub(super) fn pair<'a>(
    left: Element<'a, Message>,
    right: Element<'a, Message>,
) -> Element<'a, Message> {
    row![container(left).width(Fill), container(right).width(Fill)]
        .spacing(14)
        .into()
}
fn section<'a>(title: &'a str, help: &'a str, c: Colors) -> Element<'a, Message> {
    column![
        text(title).size(18).font(HEADING),
        text(help).size(12).color(c.muted)
    ]
    .spacing(5)
    .padding([6, 0])
    .into()
}

fn roster_field<'a>(
    label: &'a str,
    choices: Vec<Entry>,
    selected: Option<&'a Entry>,
    side: Side,
    c: Colors,
) -> Element<'a, Message> {
    let mut content = column![field(
        label,
        pick_list(choices, selected, move |t| if side == Side::Home {
            Message::HomeTeam(t)
        } else {
            Message::AwayTeam(t)
        })
        .placeholder("Named side only")
        .text_size(14)
        .style(brand::select)
        .padding(11)
        .width(Fill)
        .into(),
        c
    )]
    .spacing(6);
    if selected.is_some() {
        content = content.push(
            button(text("Remove roster").size(12))
                .on_press(Message::ClearMatchRoster(side))
                .style(brand::secondary)
                .padding([6, 10]),
        );
    }
    content.into()
}

#[cfg(test)]
mod form_tests {
    use super::*;

    fn app() -> App {
        let mut app = App::empty();
        app.session = Some(Session::new(Store::open(":memory:").unwrap()));
        app
    }
    fn send(app: &mut App, message: Message) {
        let _ = app.update(message);
    }

    #[test]
    fn team_and_player_form_feedback_preserves_invalid_drafts_until_corrected() {
        let mut app = app();
        send(&mut app, Message::SaveTeam);
        assert!(app.error.is_none(), "team validation belongs in the form");
        assert!(app.team_attempted);
        assert!(app.teams.is_empty());
        send(&mut app, Message::TeamName("North".into()));
        send(&mut app, Message::SaveTeam);
        assert_eq!(app.teams.len(), 1);
        assert!(!app.team_attempted);
        send(&mut app, Message::PlayerName("Alex".into()));
        send(&mut app, Message::PlayerJersey("1234567890123".into()));
        send(&mut app, Message::SavePlayer);
        assert!(app.error.is_none(), "jersey validation belongs in the form");
        assert!(app.player_attempted);
        assert_eq!(app.player_name, "Alex");
        assert!(app.players.is_empty());
        send(&mut app, Message::PlayerJersey("00".into()));
        send(&mut app, Message::SavePlayer);
        assert_eq!(app.players.len(), 1);
        assert!(!app.player_attempted);
    }

    #[test]
    fn invalid_match_submission_keeps_draft_and_exposes_field_feedback() {
        let mut app = app();
        send(&mut app, Message::ChooseSport(Sport::Pickleball));
        send(&mut app, Message::MatchDate("2026-02-30".into()));
        send(&mut app, Message::CreateMatch);
        assert!(app.error.is_none(), "validation belongs beside the fields");
        assert!(app.match_form.attempted);
        assert_eq!(app.match_form.date, "2026-02-30");
        assert!(
            app.match_issues()
                .iter()
                .any(|(field, _)| *field == MatchField::Date)
        );
        assert!(app.store().unwrap().matches().unwrap().is_empty());
        for message in [
            Message::MatchName("Evening doubles".into()),
            Message::MatchDate("2026-10-22".into()),
            Message::SportInput(sports::RuleField::Home, "North".into()),
            Message::SportInput(sports::RuleField::Away, "South".into()),
        ] {
            send(&mut app, message);
        }
        assert!(
            app.match_issues().is_empty(),
            "corrections clear feedback before resubmitting"
        );
        send(&mut app, Message::CreateMatch);
        assert_eq!(app.screen, Screen::GameDay);
        assert_eq!(app.active().unwrap().summary.name, "Evening doubles");
        assert!(!app.match_form.attempted);
    }

    #[test]
    fn clearing_optional_roster_keeps_names_and_does_not_copy_players() {
        let mut app = app();
        let team = app.store().unwrap().create_team("North").unwrap();
        let player = app.store().unwrap().create_player("Alex").unwrap();
        app.store().unwrap().add_to_team(team, player, "7").unwrap();
        app.refresh().unwrap();
        let entry = app.teams[0].clone();
        for message in [
            Message::ChooseSport(Sport::Badminton),
            Message::HomeTeam(entry),
            Message::ClearMatchRoster(Side::Home),
            Message::MatchName("Local friendly".into()),
            Message::SportInput(sports::RuleField::Away, "South".into()),
        ] {
            send(&mut app, message);
        }
        assert_eq!(app.sport_setup.home, "North");
        assert!(app.match_form.home.is_none());
        send(&mut app, Message::CreateMatch);
        assert!(app.error.is_none(), "{:?}", app.error);
        assert!(app.game().team(Side::Home).players.is_empty());
    }

    #[test]
    fn nine_sport_forms_validate_defaults_and_preserve_names_on_sport_change() {
        let mut app = app();
        for name in ["North", "South"] {
            app.store().unwrap().create_team(name).unwrap();
        }
        app.refresh().unwrap();
        let home = app.teams[0].clone();
        let away = app.teams[1].clone();
        send(&mut app, Message::HomeTeam(home));
        send(&mut app, Message::AwayTeam(away));
        send(&mut app, Message::MatchName("House rules".into()));
        for sport in Sport::ALL {
            send(&mut app, Message::ChooseSport(sport));
            assert!(app.match_issues().is_empty(), "{sport}");
            assert_eq!(app.sport_setup.home, "North");
            assert_eq!(app.sport_setup.away, "South");
            for dark in [false, true] {
                app.dark = dark;
                let _ = app.view();
            }
        }
    }
}
