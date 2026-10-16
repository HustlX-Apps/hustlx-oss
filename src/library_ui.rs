use super::*;
use chrono::Local;
use hustlx_oss::storage::{Error, MatchDraft, Result};
use iced::widget::{checkbox, column, pick_list, text_input};

pub(super) struct MatchForm {
    name: String,
    date: String,
    venue: String,
    home: Option<Entry>,
    away: Option<Entry>,
}
impl Default for MatchForm {
    fn default() -> Self {
        Self {
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
        self.players = players;
        self.matches = matches;
        self.roster_team = selected;
        self.roster = roster;
        Ok(())
    }
    pub(super) fn update_library(&mut self, message: Message) -> Result<()> {
        match message {
            Message::Navigate(screen) => {
                self.refresh()?;
                self.screen = screen;
            }
            Message::Refresh => self.refresh()?,
            Message::RetryStorage => self.connect()?,
            Message::OpenMatch(id) => {
                self.session_mut()?.open(id)?;
                self.screen = Screen::GameDay;
            }
            Message::CreateMatch => {
                if self.pending() {
                    return Err(Error::Validation(
                        "Retry the pending save or reopen the match before creating another match."
                            .into(),
                    ));
                }
                let f = &self.match_form;
                let draft = MatchDraft {
                    name: f.name.clone(),
                    date: f.date.clone(),
                    venue: f.venue.clone(),
                    home_team_id: f.home.as_ref().map_or(0, |t| t.id),
                    away_team_id: f.away.as_ref().map_or(0, |t| t.id),
                };
                self.session_mut()?.create(&draft)?;
                self.match_form = MatchForm::default();
                self.screen = Screen::GameDay;
            }
            Message::MatchName(value) => self.match_form.name = value,
            Message::MatchDate(value) => self.match_form.date = value,
            Message::MatchVenue(value) => self.match_form.venue = value,
            Message::HomeTeam(team) => self.match_form.home = Some(team),
            Message::AwayTeam(team) => self.match_form.away = Some(team),
            Message::TeamName(value) => self.team_name = value,
            Message::SaveTeam => {
                let id = if let Some(id) = self.editing_team {
                    self.store()?.rename_team(id, &self.team_name)?;
                    id
                } else {
                    self.store()?.create_team(&self.team_name)?
                };
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
                if let Some(team) = self.teams.iter().find(|t| t.id == id) {
                    self.team_name = team.name.clone();
                    self.editing_team = Some(id);
                }
            }
            Message::CancelTeam => {
                self.editing_team = None;
                self.team_name.clear();
            }
            Message::ArchiveTeam(id, archived) => {
                self.store()?.archive_team(id, archived)?;
                self.refresh()?;
            }
            Message::ShowTeams(value) => self.show_teams = value,
            Message::PlayerName(value) => self.player_name = value,
            Message::SavePlayer => {
                if let Some(id) = self.editing_player {
                    self.store()?.rename_player(id, &self.player_name)?;
                } else {
                    self.store()?.create_player(&self.player_name)?;
                }
                self.player_name.clear();
                self.editing_player = None;
                self.refresh()?;
            }
            Message::EditPlayer(id) => {
                if let Some(player) = self.players.iter().find(|p| p.id == id) {
                    self.player_name = player.name.clone();
                    self.editing_player = Some(id);
                }
            }
            Message::CancelPlayer => {
                self.editing_player = None;
                self.player_name.clear();
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
    fn matches_view(&self, c: Colors) -> Element<'_, Message> {
        let f = &self.match_form;
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
        let form=column![
            text("Create a match").size(23).font(BOLD),
            text("Choose your teams and save a local game.").size(12).color(c.muted),
            field("Match name",text_input("Friday basketball",&f.name).on_input(Message::MatchName).padding(10).into(),c),
            field("Date / YYYY-MM-DD",text_input("2026-10-16",&f.date).on_input(Message::MatchDate).padding(10).into(),c),
            field("Venue (optional)",text_input("Court or venue",&f.venue).on_input(Message::MatchVenue).padding(10).into(),c),
            field("Team A / home",pick_list(home_choices,f.home.as_ref(),Message::HomeTeam).placeholder("Select Team A").width(Fill).padding(10).into(),c),
            field("Team B / away",pick_list(away_choices,f.away.as_ref(),Message::AwayTeam).placeholder("Select Team B").width(Fill).padding(10).into(),c),
            button(text("Create and open match").size(14)).padding([12,16]).width(Fill)
                .on_press_maybe((f.home.is_some() && f.away.is_some() && !self.pending()).then_some(Message::CreateMatch)),
            text("Team names and active rosters are copied into this match. Later library edits keep its history intact.").size(12).color(c.muted),
        ].spacing(16);
        let mut list = column![
            row![
                text("Your matches").size(23).font(BOLD),
                Space::new().width(Fill),
                button(text("Refresh").size(12)).on_press(Message::Refresh)
            ]
            .align_y(alignment::Vertical::Center),
            text("Open a saved game to continue recording stats.")
                .size(12)
                .color(c.muted)
        ]
        .spacing(14);
        if self.matches.is_empty() {
            list = list.push(empty(
                "No matches yet",
                "Start with Teams and Players, then create your first game.",
                c,
            ));
        }
        for m in &self.matches {
            list = list.push(card(
                row![
                    column![
                        text(&m.name).size(17).font(BOLD),
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
            card(scrollable(form).spacing(8).height(Fill), c)
                .width(350)
                .height(Fill),
            card(scrollable(list).spacing(8).height(Fill), c)
                .width(Fill)
                .height(Fill)
        ]
        .spacing(16)
        .height(Fill)
        .into()
    }
    fn teams_view(&self, c: Colors) -> Element<'_, Message> {
        let mut controls = row![
            button(if self.editing_team.is_some() {
                "Save team"
            } else {
                "Create team"
            })
            .on_press(Message::SaveTeam)
            .padding([10, 14])
        ]
        .spacing(8);
        if self.editing_team.is_some() {
            controls = controls.push(button("Cancel").on_press(Message::CancelTeam));
        }
        let mut list = column![
            text("Teams").size(23).font(BOLD),
            text_input("Team name", &self.team_name)
                .on_input(Message::TeamName)
                .on_submit(Message::SaveTeam)
                .padding(10),
            controls,
            checkbox(self.show_teams)
                .label("Show archived teams")
                .on_toggle(Message::ShowTeams)
        ]
        .spacing(14);
        for team in self.teams.iter().filter(|t| self.show_teams || !t.archived) {
            list = list.push(card(
                column![
                    text(format!(
                        "{}{}",
                        team.name,
                        if team.archived { " (archived)" } else { "" }
                    ))
                    .size(16)
                    .font(BOLD),
                    row![
                        button(text("Rename").size(11)).on_press(Message::EditTeam(team.id)),
                        button(text(if team.archived { "Restore" } else { "Archive" }).size(11))
                            .on_press(Message::ArchiveTeam(team.id, !team.archived)),
                        button(text("Roster").size(11)).on_press_maybe(
                            (!team.archived).then_some(Message::RosterTeam(team.clone()))
                        )
                    ]
                    .spacing(6),
                ]
                .spacing(10),
                c,
            ));
        }
        if self.teams.is_empty() {
            list = list.push(empty(
                "Your first team",
                "Create teams here, then add players to their rosters.",
                c,
            ));
        }
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
        let mut roster = column![
            text("Team roster").size(23).font(BOLD),
            pick_list(choices, self.roster_team.as_ref(), Message::RosterTeam)
                .placeholder("Choose a team")
                .width(Fill)
                .padding(10),
            text("Changes apply to new matches. Saved match rosters stay unchanged.")
                .size(12)
                .color(c.muted)
        ]
        .spacing(14);
        if self.roster_team.is_some() {
            roster = roster
                .push(
                    pick_list(
                        player_choices,
                        self.roster_player.as_ref(),
                        Message::RosterPlayer,
                    )
                    .placeholder("Choose an existing player")
                    .width(Fill)
                    .padding(10),
                )
                .push(
                    row![
                        text_input("Jersey label (optional)", &self.jersey)
                            .on_input(Message::Jersey)
                            .padding(10)
                            .width(Fill),
                        button("Add / update player")
                            .on_press_maybe(
                                self.roster_player.as_ref().map(|_| Message::SaveRoster)
                            )
                            .padding([10, 14])
                    ]
                    .spacing(10),
                );
            if self.roster.is_empty() {
                roster = roster.push(empty(
                    "No players on this roster",
                    "Create players in the Players tab, then add them here.",
                    c,
                ));
            }
            for p in &self.roster {
                roster = roster.push(card(
                    row![
                        text(jersey_label(&p.jersey))
                            .size(22)
                            .font(BOLD)
                            .color(c.home)
                            .width(45),
                        text(&p.name).size(15).width(Fill),
                        button(text("Jersey").size(11)).on_press(Message::EditJersey(p.player_id)),
                        button(text("Remove").size(11))
                            .on_press(Message::RemoveRoster(p.player_id))
                    ]
                    .spacing(8)
                    .align_y(alignment::Vertical::Center),
                    c,
                ));
            }
        }
        row![
            card(scrollable(list).spacing(8).height(Fill), c)
                .width(Length::FillPortion(4))
                .height(Fill),
            card(scrollable(roster).spacing(8).height(Fill), c)
                .width(Length::FillPortion(6))
                .height(Fill)
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
        let form=column![text(if self.editing_player.is_some(){"Edit player"}else{"Add a player"}).size(23).font(BOLD),
            text("Players can be reused across team rosters.").size(12).color(c.muted),
            field("Player name",text_input("Full name",&self.player_name).on_input(Message::PlayerName).on_submit(Message::SavePlayer).padding(10).into(),c),controls,
            text("Set jersey labels in each team's roster. Archiving hides a player from new match rosters and can be reversed.").size(12).color(c.muted),
        ].spacing(16);
        let mut list = column![
            text("Player library").size(23).font(BOLD),
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
                        text(&p.name).size(17).font(BOLD),
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
                    button(text("Rename").size(12)).on_press(Message::EditPlayer(p.id)),
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
                .width(330)
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
fn field<'a>(label: &'a str, input: Element<'a, Message>, c: Colors) -> Element<'a, Message> {
    column![text(label).size(12).color(c.muted), input]
        .spacing(6)
        .into()
}
fn empty<'a>(title: &'a str, description: &'a str, c: Colors) -> Element<'a, Message> {
    container(
        column![
            text(title).size(18).font(BOLD),
            text(description).size(13).color(c.muted)
        ]
        .spacing(10),
    )
    .padding([20, 6])
    .into()
}
