use super::*;
use hustlx_oss::{
    clock::{format_game, format_shot, parse_game},
    hotkeys::{Action, Input},
    storage::{Error, Result},
};
use iced::widget::{column, text_input};
use iced::{
    Event, Subscription, Task, event,
    keyboard::{self, Key, key::Named},
    window,
};
use std::time::Duration;

impl App {
    pub fn subscription(&self) -> Subscription<Message> {
        crate::native_quit::install();
        let events = iced::event::listen_with(|event, status, id| match event {
            Event::Window(window::Event::CloseRequested) => Some(Message::Close(id)),
            Event::Window(window::Event::Unfocused) => Some(Message::ClearKeys),
            Event::Keyboard(keyboard::Event::KeyPressed {
                key,
                modified_key,
                modifiers,
                repeat,
                ..
            }) => {
                let value = match modified_key {
                    Key::Character(s) => s.to_string(),
                    _ => match key {
                        Key::Named(Named::Space) => " ".into(),
                        Key::Named(Named::Escape) => "Escape".into(),
                        Key::Named(Named::Backspace) => "Backspace".into(),
                        _ => return None,
                    },
                };
                if !repeat && modifiers.logo() && value.eq_ignore_ascii_case("q") {
                    return Some(Message::Close(id));
                }
                Some(Message::Key {
                    key: value,
                    repeat,
                    modified: modifiers.control() || modifiers.logo() || modifiers.alt(),
                    captured: status == event::Status::Captured,
                })
            }
            _ => None,
        });
        let ticks = if cfg!(target_os = "macos")
            || (self.active().is_some_and(|a| a.clock.running) && !self.pending())
        {
            iced::time::every(Duration::from_millis(100)).map(Message::Tick)
        } else {
            Subscription::none()
        };
        Subscription::batch([events, ticks])
    }
    pub fn update(&mut self, message: Message) -> Task<Message> {
        self.now = Instant::now();
        let result = match message {
            Message::Tick(now) => {
                self.now = now;
                if !crate::native_quit::install() {
                    self.error = Some("Could not guard the macOS Quit menu. Use the window close button to save clocks before quitting.".into());
                }
                if crate::native_quit::take_request() {
                    return window::latest().and_then(|id| Task::done(Message::Close(id)));
                }
                if let Some(s) = &mut self.session
                    && let Err(error) = s.tick_live(now)
                {
                    self.error = Some(error.to_string());
                }
                return self.finish_update();
            }
            Message::Clock(command) => self.apply_clock(command),
            Message::ToggleClock => self.apply_clock(if self.current_clock().running {
                ClockCommand::Pause
            } else {
                ClockCommand::Start
            }),
            Message::Close(id) => {
                self.closing = Some(id);
                let now = self.now;
                if let Some(s) = &mut self.session {
                    s.pause_for_close_at(now)
                } else {
                    Ok(())
                }
            }
            Message::CancelClose => {
                self.closing = None;
                Ok(())
            }
            Message::ClearKeys => {
                self.hotkeys.clear();
                Ok(())
            }
            Message::GameTime(value) => {
                self.game_time = value;
                Ok(())
            }
            Message::ShotTime(value) => {
                self.shot_time = value;
                Ok(())
            }
            Message::ApplyGameTime => parse_game(&self.game_time)
                .and_then(|ms| self.apply_clock(ClockCommand::SetGame(ms))),
            Message::ApplyShotTime => match self.shot_time.trim().parse::<f64>() {
                Ok(seconds) if seconds.is_finite() && (0.0..=120.0).contains(&seconds) => {
                    self.apply_clock(ClockCommand::SetShot((seconds * 1000.0).round() as u64))
                }
                _ => Err(Error::Validation(
                    "Shot time must be from 0 to 120 seconds.".into(),
                )),
            },
            Message::ToggleClockEditor => {
                self.lineup_draft = None;
                self.lineup_history = false;
                let result = self.apply_clock(ClockCommand::Pause);
                if result.is_ok() {
                    self.clock_edit = !self.clock_edit;
                    self.show_hotkeys = false;
                    self.hotkeys.clear();
                    let clock = self.current_clock();
                    self.game_time = format_game(clock.game_ms);
                    self.shot_time = format!("{:.1}", clock.shot_ms as f64 / 1000.0);
                }
                result
            }
            Message::ToggleHotkeys => {
                self.lineup_draft = None;
                self.lineup_history = false;
                self.show_hotkeys = !self.show_hotkeys;
                self.clock_edit = false;
                self.hotkeys.clear();
                Ok(())
            }
            Message::DismissNotice => {
                self.notice = None;
                Ok(())
            }
            Message::Key {
                key,
                repeat,
                modified,
                captured,
            } => return self.keyboard(&key, repeat, modified, captured),
            other => {
                self.update_core(other);
                return self.finish_update();
            }
        };
        if let Err(error) = result {
            self.error = Some(error.to_string());
        } else if !self.pending() {
            self.error = None;
        }
        self.finish_update()
    }
    fn finish_update(&mut self) -> Task<Message> {
        if self.lineup_saving && !self.pending() {
            self.lineup_draft = None;
            self.lineup_saving = false;
            self.hotkeys.clear();
        }
        if let Some(notice) = self.session.as_mut().and_then(Session::take_clock_notice) {
            self.notice = Some(notice);
        }
        if let Some(id) = self.closing
            && !self.pending()
        {
            if let Some(s) = &mut self.session
                && let Err(error) = s.pause_for_close_at(Instant::now())
            {
                self.error = Some(error.to_string());
                return Task::none();
            }
            return window::close(id);
        }
        Task::none()
    }
    fn keyboard(
        &mut self,
        key: &str,
        repeat: bool,
        modified: bool,
        captured: bool,
    ) -> Task<Message> {
        if (self.lineup_draft.is_some() || self.lineup_history)
            && !repeat
            && !modified
            && !captured
            && key == "Escape"
        {
            return self.update(Message::CancelLineup);
        }
        if self.show_hotkeys && !repeat && !modified && !captured && (key == "?" || key == "Escape")
        {
            self.show_hotkeys = false;
            return Task::none();
        }
        let enabled = self.screen == Screen::GameDay
            && self.active().is_some()
            && !self.pending()
            && !self.clock_edit
            && !self.show_hotkeys
            && self.lineup_draft.is_none()
            && !self.lineup_history;
        let action = self.hotkeys.press(Input {
            key,
            enabled,
            repeat,
            modified,
            captured,
        });
        match action {
            Some(Action::Stat(stat)) => match self.hotkeys.target(self.game()) {
                Ok((side, id)) => {
                    if let Some(s) = &mut self.session {
                        s.select(side, id);
                    }
                    self.update_core(Message::Record(stat));
                    if self.error.is_none() && !self.pending() {
                        self.hotkeys.clear();
                    }
                }
                Err(error) => self.error = Some(error),
            },
            Some(Action::ToggleClock) => return self.update(Message::ToggleClock),
            Some(Action::Clock(command)) => return self.update(Message::Clock(command)),
            Some(Action::Undo) => return self.update(Message::Undo),
            Some(Action::Substitutions) => {
                let side = if self.hotkeys.digits().is_empty() {
                    self.hotkeys
                        .side()
                        .or_else(|| self.game().selected.map(|p| p.0))
                } else {
                    match self.hotkeys.target(self.game()) {
                        Ok((side, _)) => Some(side),
                        Err(error) => {
                            self.error = Some(error);
                            return self.finish_update();
                        }
                    }
                };
                if let Some(side) = side {
                    return self.update(Message::OpenLineup(
                        side,
                        hustlx_oss::lineup::LineupKind::Substitution,
                    ));
                }
                self.error =
                    Some("Select a player, or press , for home or . for away, then X.".into());
            }
            Some(Action::Help) => return self.update(Message::ToggleHotkeys),
            Some(Action::Clear) => {
                if let Some(s) = &mut self.session {
                    s.clear_selection();
                }
                self.error = None;
            }
            None => {}
        }
        self.finish_update()
    }
    fn apply_clock(&mut self, command: ClockCommand) -> Result<()> {
        let now = self.now;
        self.session_mut()?.command_at(command, now)
    }
    pub(super) fn current_clock(&self) -> ClockState {
        self.session
            .as_ref()
            .map_or_else(ClockState::default, |s| s.clock_at(self.now))
    }
    fn clock_button(&self, label: &str, command: ClockCommand) -> Element<'_, Message> {
        button(text(label.to_owned()).size(10))
            .on_press_maybe((!self.pending()).then_some(Message::Clock(command)))
            .padding([5, 7])
            .style(brand::secondary)
            .into()
    }
    pub(super) fn clock(&self, c: Colors) -> Element<'_, Message> {
        let clock = self.current_clock();
        column![
            text(format!(
                "Q{}  /  {}",
                clock.period,
                if self.pending() {
                    "SAVE PENDING"
                } else if clock.running {
                    "RUNNING"
                } else {
                    "PAUSED"
                }
            ))
            .size(10)
            .color(if self.pending() { c.red } else { c.muted }),
            row![
                column![
                    text(format_shot(clock.shot_ms))
                        .size(30)
                        .font(DISPLAY)
                        .color(c.away),
                    text(if clock.shot_running {
                        "SHOT"
                    } else {
                        "SHOT PAUSED"
                    })
                    .size(8)
                    .color(c.muted)
                ]
                .align_x(alignment::Horizontal::Center),
                text(format_game(clock.game_ms)).size(50).font(DISPLAY)
            ]
            .spacing(12)
            .align_y(alignment::Vertical::Center),
            row![
                button(
                    text(if clock.running {
                        "Pause  Space"
                    } else {
                        "Start  Space"
                    })
                    .size(10)
                )
                .on_press_maybe((!self.pending()).then_some(Message::ToggleClock))
                .padding([5, 8]),
                self.clock_button("24  [", ClockCommand::FullShot),
                self.clock_button("14  ]", ClockCommand::ShortShot),
                self.clock_button(
                    if clock.shot_running {
                        "Pause shot"
                    } else {
                        "Start shot"
                    },
                    ClockCommand::ToggleShot
                )
            ]
            .spacing(5),
            row![
                self.clock_button("Reset game", ClockCommand::ResetGame),
                self.clock_button("Next Q  P", ClockCommand::NextPeriod),
                button(text("Set times").size(10))
                    .on_press_maybe((!self.pending()).then_some(Message::ToggleClockEditor))
                    .padding([5, 7])
                    .style(brand::secondary)
            ]
            .spacing(5),
        ]
        .spacing(4)
        .align_x(alignment::Horizontal::Center)
        .width(Length::FillPortion(4))
        .into()
    }
    pub(super) fn clock_editor(&self, c: Colors) -> Element<'_, Message> {
        card(column![
   row![text("Set clock times").size(24).font(HEADING),Space::new().width(Fill),button("Back to Game Day").on_press(Message::ToggleClockEditor)].spacing(12),
   text("Time corrections pause both clocks. Game reset uses 10:00 and 24; shot presets retain the current run/pause state.").size(13).color(c.muted),
   self.clock(c),
   row![column![text("Game time / MM:SS").size(13),text_input("10:00",&self.game_time).on_input(Message::GameTime).on_submit(Message::ApplyGameTime).padding(12),button("Save game time").on_press_maybe((!self.pending()).then_some(Message::ApplyGameTime))].spacing(10).width(Fill),column![text("Shot time / seconds").size(13),text_input("24",&self.shot_time).on_input(Message::ShotTime).on_submit(Message::ApplyShotTime).padding(12),button("Save shot time").on_press_maybe((!self.pending()).then_some(Message::ApplyShotTime))].spacing(10).width(Fill)].spacing(24),
   text("Keyboard scoring shortcuts are disabled while editing these fields.").size(12).color(c.muted),
  ].spacing(22),c).width(Fill).height(Fill).into()
    }
    pub(super) fn shortcut_guide(&self, c: Colors) -> Element<'_, Message> {
        card(column![
   row![text("Game Day hotkeys").size(25).font(HEADING),Space::new().width(Fill),button("Back  ? / Esc").on_press(Message::ToggleHotkeys)].spacing(12),
   self.clock(c),
   text("Space: start/pause both     [ : 24-second reset     ] : 14-second reset     P: next period").size(15),
   text("Q / W / E: FT, 2PT, 3PT made     A / S / D: FT, 2PT, 3PT missed").size(15),
   text("R: defensive rebound     Y: offensive rebound     F: foul     G: assist").size(15),
   text("T: turnover     V: steal     B: block     Backspace: undo last play").size(15),
   text("Type a jersey (00 is supported), then a shot/stat key. Comma selects home; period selects away. You can also select a player with the mouse. Duplicate jerseys require an explicit team or player selection.").size(14),
   text("Escape clears jersey/player selection. Backspace edits a typed jersey before it undoes a play. Numpad / + * - map to FT made, 2PT made, 3PT made, 2PT miss.").size(13).color(c.muted),
   text("Scoring shortcuts are disabled in this guide, in time fields, outside Game Day, and while a save is pending. Held-key repeats and Ctrl/Command/Alt combinations are ignored. X opens the substitution board for a selected player or explicit team. Timeout O remains unbound.").size(12).color(c.muted),
  ].spacing(16),c).width(Fill).height(Fill).into()
    }
}
