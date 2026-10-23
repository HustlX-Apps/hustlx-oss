//! Native operator layout translated from the web sport consoles.
use super::*;
use hustlx_oss::storage::{Error, Result};
use hustlx_oss::{
    field_sport::FieldCommand as F,
    operator::{PlayerAction as A, index},
    sport::{Command, Sport, State, Status},
};
use iced::widget::column;

#[derive(Debug, Clone)]
pub(super) struct OperatorDraft {
    side: Side,
    players: Vec<i64>,
    exchange: bool,
    libero: bool,
    outgoing: Option<i64>,
    incoming: Option<i64>,
}

fn operator_service_slot(state: &State, side: Side) -> Option<u8> {
    if state.status != Status::Live || state.sport.field() || state.sport == Sport::Basketball {
        return None;
    }
    if side == state.server {
        Some(if state.sport == Sport::Volleyball {
            1
        } else {
            state.server_slot
        })
    } else if state.sport == Sport::Volleyball {
        None
    } else {
        Some(state.receiver_slot)
    }
}

fn operator_service_role(state: &State, side: Side, player: i64) -> Option<&'static str> {
    let slot = operator_service_slot(state, side)?.checked_sub(1)?;
    (state.operator.court[index(side)].get(usize::from(slot)) == Some(&player)).then_some(
        if side == state.server {
            "SERVING"
        } else {
            "RECEIVING"
        },
    )
}

fn operator_match_tiebreak_result(state: &State) -> bool {
    matches!(state.sport, Sport::Tennis | Sport::Padel)
        && state.status != Status::Live
        && state.rules().match_tiebreak
        && state.unit_number == u16::from(state.rules().best_of)
}

fn operator_score_label(state: &State, side: Side) -> String {
    if matches!(state.sport, Sport::Tennis | Sport::Padel)
        && state.status != Status::Live
        && let Some(set) = state.completed_units.last()
    {
        let score = if operator_match_tiebreak_result(state) {
            set.tiebreak_points.unwrap_or(set.score)
        } else {
            set.score
        };
        return score[index(side)].to_string();
    }
    state.point_label(side)
}

impl App {
    pub(super) fn update_operator(&mut self, message: Message) -> Result<()> {
        match message {
            Message::OperatorTime(value) => self.operator_time = value,
            Message::ApplyOperatorTime => {
                let ms = hustlx_oss::clock::parse_game(&self.operator_time)?;
                self.session_mut()?
                    .sport_command(Command::Field(F::SetTime(ms)))?;
            }
            Message::ConfirmSportFinish => self.operator_finish = true,
            Message::CancelSportFinish => self.operator_finish = false,
            Message::FinishSport => {
                let field = self.active().is_some_and(|a| a.summary.sport.field());
                self.session_mut()?.sport_command(if field {
                    Command::Field(F::Complete)
                } else {
                    Command::Complete
                })?;
                self.operator_finish = false;
            }
            Message::OperatorSelect(side, id) => {
                self.session_mut()?.select(side, id);
                self.operator_chooser = None;
            }
            Message::OperatorChooser(side, id, attack) => {
                self.session_mut()?.select(side, id);
                self.operator_chooser = Some((side, id, attack));
            }
            Message::OperatorAction(side, player, action) => {
                self.session_mut()?.sport_command(Command::Player {
                    side,
                    player,
                    action,
                })?;
                self.operator_chooser = None;
            }
            Message::OpenOperatorLineup(side, exchange, libero) => {
                let state = self
                    .active()
                    .and_then(|a| a.sport_state.as_ref())
                    .ok_or_else(|| Error::Validation("Open a sport match first.".into()))?;
                self.operator_draft = Some(OperatorDraft {
                    side,
                    players: state.operator.court[index(side)].clone(),
                    exchange,
                    libero,
                    outgoing: None,
                    incoming: None,
                });
            }
            Message::OperatorPick(id) => {
                if let Some(d) = &mut self.operator_draft {
                    if d.exchange {
                        if d.players.contains(&id) {
                            d.outgoing = Some(id);
                        } else {
                            d.incoming = Some(id);
                        }
                    } else if let Some(i) = d.players.iter().position(|p| *p == id) {
                        d.players.remove(i);
                    } else if d.players.len() < 11 {
                        d.players.push(id);
                    }
                }
            }
            Message::SaveOperatorLineup => {
                let d = self
                    .operator_draft
                    .as_ref()
                    .ok_or_else(|| Error::Validation("Open the lineup board first.".into()))?;
                let command = if d.exchange {
                    Command::Exchange {
                        side: d.side,
                        outgoing: d.outgoing.ok_or_else(|| {
                            Error::Validation("Choose the outgoing player.".into())
                        })?,
                        incoming: d.incoming.ok_or_else(|| {
                            Error::Validation("Choose the incoming player.".into())
                        })?,
                        libero: d.libero,
                    }
                } else {
                    let mut players = [0; 11];
                    for (slot, id) in d.players.iter().enumerate() {
                        players[slot] = *id;
                    }
                    Command::Lineup {
                        side: d.side,
                        players,
                    }
                };
                self.session_mut()?.sport_command(command)?;
                self.operator_draft = None;
            }
            Message::CancelOperatorLineup => self.operator_draft = None,
            _ => {}
        }
        Ok(())
    }
    pub(super) fn operator_view(&self, c: Colors) -> Element<'_, Message> {
        let saved = self.active().unwrap();
        let state = saved.sport_state.as_ref().unwrap();
        let mut body = column![
            row![
                text(&saved.summary.name).font(HEADING).size(20).width(Fill),
                badge(state.sport.label(), c.away, c),
                badge("SAVED LOCALLY", c.green, c),
                button(text("Undo").size(12))
                    .style(brand::secondary)
                    .on_press_maybe(
                        (!self.pending()
                            && saved.sport_history.iter().any(|e| !e.undone
                                && e.command
                                    .is_some_and(hustlx_oss::storage::sport_action_undoable)))
                        .then_some(Message::Undo)
                    ),
            ]
            .spacing(10)
            .align_y(alignment::Vertical::Center),
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
            .color(c.muted),
            self.operator_scoreboard(c),
        ]
        .spacing(10);
        if self.operator_finish {
            body=body.push(card(column![
                text("Save the final result?").font(HEADING).size(19),
                text(if state.sport.field(){"The recorded goals and penalties determine the result. A tied result is saved as a draw."}else{"The decided score and completed sets or games will be saved as the result."}).size(12),
                row![button("Confirm result").on_press_maybe((!self.pending()).then_some(Message::FinishSport)),button("Keep scoring").style(brand::secondary).on_press(Message::CancelSportFinish)].spacing(8)
            ].spacing(10),c));
        }
        if self.operator_draft.is_some() {
            return body.push(self.operator_lineup_board(c)).into();
        }
        let mut center = column![].spacing(10);
        if state.field.is_some() {
            center = center.push(self.field_controls(c));
        } else {
            center = center.push(self.operator_point_pad(c));
            if state.status == Status::Decided {
                center = center.push(
                    button("Review & save final result")
                        .width(Fill)
                        .padding(12)
                        .on_press(Message::ConfirmSportFinish),
                );
            }
        }
        if state.status == Status::Completed {
            center = center.push(card(
                column![
                    text("FINAL RESULT").font(HEADING).color(c.green),
                    text(state.winner.map_or_else(
                        || "Draw".into(),
                        |s| format!("{} wins", self.game().team(s).name)
                    ))
                    .font(HEADING)
                    .size(19),
                    text("Saved on this computer. Undo completion to make a correction.")
                        .size(11)
                        .color(c.muted)
                ]
                .spacing(9),
                c,
            ));
        }
        center = center.push(self.operator_history(c));
        body.push(
            row![
                scrollable(self.operator_team(Side::Home, c))
                    .spacing(8)
                    .height(Fill)
                    .width(Length::FillPortion(3)),
                scrollable(center)
                    .spacing(8)
                    .height(Fill)
                    .width(Length::FillPortion(4)),
                scrollable(self.operator_team(Side::Away, c))
                    .spacing(8)
                    .height(Fill)
                    .width(Length::FillPortion(3)),
            ]
            .spacing(10)
            .height(Fill),
        )
        .height(Fill)
        .into()
    }
    fn operator_scoreboard(&self, c: Colors) -> Element<'_, Message> {
        let saved = self.active().unwrap();
        let s = saved.sport_state.as_ref().unwrap();
        let mut lines = column![].spacing(8);
        let status = if s.status == Status::Completed {
            "FINAL".into()
        } else if s.status == Status::Decided {
            "RESULT READY".into()
        } else if let Some(f) = &s.field {
            format!(
                "{} / {}",
                if f.period <= 2 {
                    if f.is_futsal() { "PERIOD" } else { "HALF" }
                } else if f.period < 5 {
                    "EXTRA TIME"
                } else {
                    "SHOOTOUT"
                },
                f.period
            )
        } else {
            format!(
                "{} {}{}",
                if matches!(s.sport, Sport::Volleyball | Sport::Tennis | Sport::Padel) {
                    "SET"
                } else {
                    "GAME"
                },
                s.unit_number,
                if s.tiebreak { " / TIEBREAK" } else { "" }
            )
        };
        let score = |side: Side| {
            column![
                text(&self.game().team(side).name)
                    .size(18)
                    .font(HEADING)
                    .color(if side == Side::Home { c.home } else { c.away }),
                text(operator_score_label(s, side)).size(48).font(DISPLAY)
            ]
            .spacing(1)
            .width(Fill)
            .align_x(if side == Side::Home {
                alignment::Horizontal::Left
            } else {
                alignment::Horizontal::Right
            })
        };
        lines = lines.push(
            row![
                score(Side::Home),
                column![
                    text(status).size(11).font(BOLD).color(c.muted),
                    text(if s.field.is_some() {
                        "GOALS".into()
                    } else {
                        format!("{} : {}", s.units[0], s.units[1])
                    })
                    .size(24)
                    .font(DISPLAY),
                    text(if s.field.is_some() {
                        ""
                    } else if matches!(s.sport, Sport::Volleyball | Sport::Tennis | Sport::Padel) {
                        "SETS WON"
                    } else {
                        "GAMES WON"
                    })
                    .size(10)
                    .color(c.muted),
                    text(if operator_match_tiebreak_result(s) {
                        "MATCH TIEBREAK POINTS".into()
                    } else if matches!(s.sport, Sport::Tennis | Sport::Padel)
                        && s.status != Status::Live
                    {
                        "LAST SET GAMES".into()
                    } else if s.field.is_none() && s.status == Status::Live {
                        format!(
                            "SERVE {} / {}{}",
                            if s.server == Side::Home {
                                "HOME"
                            } else {
                                "AWAY"
                            },
                            s.server_slot,
                            if s.sport == Sport::Pickleball {
                                format!(" / #{}", s.server_number)
                            } else {
                                String::new()
                            }
                        )
                    } else {
                        String::new()
                    })
                    .size(10)
                    .font(BOLD)
                    .color(if s.server == Side::Home {
                        c.home
                    } else {
                        c.away
                    })
                ]
                .spacing(4)
                .align_x(alignment::Horizontal::Center),
                score(Side::Away)
            ]
            .spacing(10)
            .align_y(alignment::Vertical::Center),
        );
        if s.field.is_none() {
            let unit_label = if matches!(s.sport, Sport::Volleyball | Sport::Tennis | Sport::Padel)
            {
                "S"
            } else {
                "G"
            };
            let mut strip = row![].spacing(6);
            for (i, unit) in s.completed_units.iter().enumerate() {
                let points = if unit.score == [0, 0] {
                    unit.tiebreak_points.unwrap_or(unit.score)
                } else {
                    unit.score
                };
                strip = strip.push(badge(
                    format!(
                        "{}{}  {}-{}{}",
                        unit_label,
                        i + 1,
                        points[0],
                        points[1],
                        if unit.tiebreak_points.is_some() {
                            " TB"
                        } else {
                            ""
                        }
                    ),
                    c.muted,
                    c,
                ));
            }
            if s.status == Status::Live {
                let score = if matches!(s.sport, Sport::Tennis | Sport::Padel) {
                    s.games
                } else {
                    s.points
                };
                strip = strip.push(badge(
                    format!(
                        "{}{}  {}-{}  LIVE",
                        unit_label, s.unit_number, score[0], score[1]
                    ),
                    c.away,
                    c,
                ));
            }
            lines = lines.push(scrollable(strip).direction(
                iced::widget::scrollable::Direction::Horizontal(
                    iced::widget::scrollable::Scrollbar::default(),
                ),
            ));
        }
        card(lines, c).padding([10, 16]).width(Fill).into()
    }
    fn operator_team(&self, side: Side, c: Colors) -> Element<'_, Message> {
        let s = self.active().unwrap().sport_state.as_ref().unwrap();
        let op = &s.operator;
        let team = self.game().team(side);
        let i = index(side);
        let live = s.status == Status::Live && !self.pending();
        let accent = if side == Side::Home { c.home } else { c.away };
        let mut tools = row![
            button(
                text(if s.sport.field() {
                    "Team sheet"
                } else {
                    "Lineup"
                })
                .size(11)
            )
            .style(brand::secondary)
            .on_press_maybe(live.then_some(Message::OpenOperatorLineup(side, false, false))),
            button(text("Sub").size(11))
                .style(brand::secondary)
                .on_press_maybe(
                    (live && !op.court[i].is_empty())
                        .then_some(Message::OpenOperatorLineup(side, true, false))
                ),
        ]
        .spacing(4);
        if s.sport == Sport::Volleyball {
            tools = tools
                .push(
                    button(text("Rotate").size(11))
                        .style(brand::secondary)
                        .on_press_maybe(
                            live.then_some(Message::SportCommand(Command::Rotate(side))),
                        ),
                )
                .push(
                    button(text("Libero").size(11))
                        .style(brand::secondary)
                        .on_press_maybe(
                            (live && !op.court[i].is_empty())
                                .then_some(Message::OpenOperatorLineup(side, true, true)),
                        ),
                );
        }
        let mut panel = column![
            row![
                text(&team.name)
                    .size(15)
                    .font(HEADING)
                    .color(accent)
                    .width(Fill),
                text(format!(
                    "{} / {}",
                    if side == Side::Home { "HOME" } else { "AWAY" },
                    team.players.len()
                ))
                .size(10)
                .color(c.muted)
            ]
            .spacing(5),
            tools
        ]
        .spacing(7);
        if s.sport == Sport::Volleyball {
            let positions = if side == Side::Home {
                [[4, 3], [5, 2], [0, 1]]
            } else {
                [[1, 0], [2, 5], [3, 4]]
            };
            for pair in positions {
                let mut line = row![].spacing(5);
                for slot in pair {
                    let node = if let Some(id) = op.court[i].get(slot) {
                        self.operator_player(side, *id, Some(slot + 1), c)
                    } else {
                        card(
                            column![
                                text(format!("P{}", slot + 1)).size(10).color(c.muted),
                                text("Empty position").size(11).color(c.muted)
                            ]
                            .spacing(6),
                            c,
                        )
                        .width(Fill)
                        .into()
                    };
                    line = line.push(node);
                }
                panel = panel.push(line);
            }
            for id in op.court[i].iter().skip(6) {
                panel = panel.push(self.operator_player(side, *id, None, c));
            }
        } else {
            for id in &op.court[i] {
                panel = panel.push(self.operator_player(side, *id, None, c));
            }
        }
        if op.court[i].is_empty() {
            panel = panel.push(
                text("Select a lineup to place players. Scoring remains available.")
                    .size(11)
                    .color(c.muted),
            );
        }
        panel = panel.push(
            text(if s.sport.field() {
                "BENCH"
            } else {
                "AVAILABLE PLAYERS"
            })
            .size(10)
            .font(BOLD)
            .color(c.muted),
        );
        for player in team.players.iter().filter(|p| !op.court[i].contains(&p.id)) {
            panel = panel.push(
                button(
                    row![
                        text(if player.jersey.is_empty() {
                            "-"
                        } else {
                            &player.jersey
                        })
                        .size(11)
                        .width(28),
                        text(&player.name).size(12).width(Fill)
                    ]
                    .spacing(5),
                )
                .width(Fill)
                .style(if self.game().selected == Some((side, player.id)) {
                    button::primary
                } else {
                    brand::secondary
                })
                .on_press_maybe(live.then_some(Message::OperatorSelect(side, player.id))),
            );
        }
        if team.players.is_empty() {
            panel=panel.push(text("Named-side scoring is ready. To use player controls, select a roster when creating a match.").size(11).color(c.muted));
        }
        if !s.sport.field() {
            panel = panel.push(
                button(text(format!("Timeout / {} used", op.timeouts[i])).size(11))
                    .style(brand::secondary)
                    .on_press_maybe(live.then_some(Message::SportCommand(Command::Timeout(side)))),
            );
        }
        card(panel, c).padding(10).width(Fill).into()
    }
    fn operator_player(
        &self,
        side: Side,
        id: i64,
        position: Option<usize>,
        c: Colors,
    ) -> Element<'_, Message> {
        let Some(p) = self.game().player(side, id) else {
            return text("Missing saved player").into();
        };
        let s = self.active().unwrap().sport_state.as_ref().unwrap();
        let live = s.status == Status::Live && !self.pending();
        let title = format!(
            "{}{} {}{}",
            position.map_or_else(String::new, |n| format!("P{n}/")),
            p.jersey,
            p.name,
            if s.operator.libero[index(side)] == Some(id) {
                " / L"
            } else {
                ""
            }
        );
        let mut content = column![
            button(text(title).size(11).font(SEMIBOLD))
                .width(Fill)
                .style(if self.game().selected == Some((side, id)) {
                    button::primary
                } else {
                    brand::secondary
                })
                .on_press_maybe(live.then_some(Message::OperatorSelect(side, id)))
        ]
        .spacing(4);
        if s.sport == Sport::Volleyball {
            content = content.push(
                row![
                    text(format!(
                        "K {} B {} A {} D {}",
                        s.operator.count(side, id, A::Kill),
                        s.operator.count(side, id, A::Block),
                        s.operator.count(side, id, A::Ace),
                        s.operator.count(side, id, A::Dig)
                    ))
                    .size(9)
                    .color(c.muted)
                    .width(Fill),
                    text(operator_service_role(s, side, id).unwrap_or(""))
                        .size(9)
                        .font(BOLD)
                        .color(c.green),
                ]
                .spacing(3),
            );
            content = content.push(
                text(format!(
                    "AST {} REC {} ERR {}",
                    s.operator.count(side, id, A::Assist),
                    s.operator.count(side, id, A::Reception),
                    u32::from(s.operator.count(side, id, A::AttackError))
                        + u32::from(s.operator.count(side, id, A::ServiceError))
                        + u32::from(s.operator.count(side, id, A::BlockError))
                        + u32::from(s.operator.count(side, id, A::ReceptionError))
                        + u32::from(s.operator.count(side, id, A::Fault))
                ))
                .size(9)
                .color(c.muted),
            );
            for actions in [
                [
                    ("ATK", None),
                    ("BLK", Some(A::Block)),
                    ("ACE", Some(A::Ace)),
                    ("ERR", None),
                ],
                [
                    ("AST", Some(A::Assist)),
                    ("REC", Some(A::Reception)),
                    ("DIG", Some(A::Dig)),
                    ("+1", Some(A::Point)),
                ],
            ] {
                let mut buttons = row![].spacing(2);
                for (label, action) in actions {
                    buttons = buttons.push(
                        button(text(label).size(10).font(BOLD))
                            .padding([7, 2])
                            .width(Fill)
                            .style(brand::secondary)
                            .on_press_maybe(live.then_some(action.map_or(
                                Message::OperatorChooser(side, id, label == "ATK"),
                                |a| Message::OperatorAction(side, Some(id), a),
                            ))),
                    );
                }
                content = content.push(buttons);
            }
        } else if !s.sport.field() {
            content = content.push(
                row![
                    text(format!(
                        "W {} / A {} / ERR {}",
                        s.operator.count(side, id, A::Winner),
                        s.operator.count(side, id, A::Ace),
                        u32::from(s.operator.count(side, id, A::UnforcedError))
                            + u32::from(s.operator.count(side, id, A::ForcedError))
                    ))
                    .size(10)
                    .color(c.muted)
                    .width(Fill),
                    text(operator_service_role(s, side, id).unwrap_or(""))
                        .size(9)
                        .font(BOLD)
                        .color(c.green),
                ]
                .spacing(3),
            );
        }
        card(content, c).padding(5).width(Fill).into()
    }
    fn operator_point_pad(&self, c: Colors) -> Element<'_, Message> {
        let s = self.active().unwrap().sport_state.as_ref().unwrap();
        let live = s.status == Status::Live && !self.pending();
        let mut pad = column![
            text("POINT & RALLY").size(11).font(BOLD).color(c.muted),
            row![
                button(column![
                    text("HOME").size(11),
                    text(
                        if s.sport == Sport::Pickleball
                            && s.rules().scoring == hustlx_oss::sport::Scoring::SideOut
                        {
                            "RALLY"
                        } else {
                            "+1"
                        }
                    )
                    .font(DISPLAY)
                    .size(28)
                ])
                .padding(12)
                .width(Fill)
                .on_press_maybe(live.then_some(Message::SportCommand(Command::Rally(Side::Home)))),
                button(column![
                    text("AWAY").size(11),
                    text(
                        if s.sport == Sport::Pickleball
                            && s.rules().scoring == hustlx_oss::sport::Scoring::SideOut
                        {
                            "RALLY"
                        } else {
                            "+1"
                        }
                    )
                    .font(DISPLAY)
                    .size(28)
                ])
                .padding(12)
                .width(Fill)
                .on_press_maybe(live.then_some(Message::SportCommand(Command::Rally(Side::Away))))
            ]
            .spacing(8)
        ]
        .spacing(10);
        if s.status == Status::Live {
            let mut service = column![
                text(format!(
                    "SERVING  {}",
                    self.operator_service_identity(s.server)
                ))
                .size(12)
                .font(SEMIBOLD)
                .color(c.green),
            ]
            .spacing(5);
            let receiving = hustlx_oss::operator::opposite(s.server);
            if operator_service_slot(s, receiving).is_some() {
                service = service.push(
                    text(format!(
                        "RECEIVING  {}",
                        self.operator_service_identity(receiving)
                    ))
                    .size(12)
                    .color(c.text),
                );
            }
            pad = pad.push(card(service, c).padding(10));
        }
        if s.sport == Sport::Pickleball {
            pad=pad.push(text("Record the rally winner. In side-out scoring, a receiving rally changes service without adding a point.").size(11).color(c.muted));
        }
        pad = pad.push(
            row![
                button(text("Serve home").size(11))
                    .style(brand::secondary)
                    .on_press_maybe(
                        live.then_some(Message::SportCommand(Command::SetServer(Side::Home)))
                    ),
                button(text("Serve away").size(11))
                    .style(brand::secondary)
                    .on_press_maybe(
                        live.then_some(Message::SportCommand(Command::SetServer(Side::Away)))
                    )
            ]
            .spacing(6),
        );
        let (actor, actions, title) = if let Some((side, id, attack)) = self.operator_chooser {
            (
                Some((side, id)),
                if attack {
                    vec![A::Kill, A::Attack]
                } else {
                    vec![
                        A::AttackError,
                        A::ServiceError,
                        A::BlockError,
                        A::ReceptionError,
                        A::Fault,
                    ]
                },
                if attack {
                    "Attack outcome"
                } else {
                    "Error outcome"
                },
            )
        } else {
            (
                self.game().selected,
                if s.sport == Sport::Volleyball {
                    vec![]
                } else {
                    A::for_sport(s.sport).to_vec()
                },
                "Player action",
            )
        };
        if let Some((side, id)) = actor {
            if let Some(p) = self.game().player(side, id) {
                pad = pad.push(text(format!("{title} / {}", p.name)).font(HEADING).size(15));
            }
            for chunk in actions.chunks(2) {
                let mut line = row![].spacing(6);
                for action in chunk {
                    line = line.push(
                        button(text(action.label()).size(11))
                            .padding([9, 6])
                            .width(Fill)
                            .style(brand::secondary)
                            .on_press_maybe(live.then_some(Message::OperatorAction(
                                side,
                                Some(id),
                                *action,
                            ))),
                    );
                }
                pad = pad.push(line);
            }
        } else {
            pad = pad.push(
                text("Select a player to attribute an action. Team scoring needs no selection.")
                    .size(11)
                    .color(c.muted),
            );
        }
        pad = pad.push(
            text(format!(
                "{} / best of {} / target {} / win by {}",
                s.rules().scoring,
                s.rules().best_of,
                s.rules().target,
                s.rules().win_by
            ))
            .size(10)
            .color(c.muted),
        );
        card(pad, c).padding(12).into()
    }
    fn operator_lineup_board(&self, c: Colors) -> Element<'_, Message> {
        let d = self.operator_draft.as_ref().unwrap();
        let mut board=column![text(if d.libero{"Libero exchange"}else if d.exchange{"Confirm substitution"}else{"Set the lineup"}).font(HEADING).size(22),text(if d.exchange{"Choose one player on court and one from the bench."}else{"Tap players in position or service order. Counts are advisory; save the agreed lineup."}).size(12).color(c.muted)].spacing(10);
        for player in &self.game().team(d.side).players {
            let active = d.players.contains(&player.id);
            let selected = if d.exchange {
                d.outgoing == Some(player.id) || d.incoming == Some(player.id)
            } else {
                active
            };
            board = board.push(
                button(
                    row![
                        text(format!("{} {}", player.jersey, player.name)).width(Fill),
                        text(if d.exchange {
                            if active {
                                "On court / outgoing"
                            } else {
                                "Bench / incoming"
                            }
                        } else if active {
                            "Selected"
                        } else {
                            "Add"
                        })
                        .size(11)
                    ]
                    .spacing(10),
                )
                .padding(10)
                .width(Fill)
                .style(if selected {
                    button::primary
                } else {
                    brand::secondary
                })
                .on_press_maybe((!self.pending()).then_some(Message::OperatorPick(player.id))),
            );
        }
        board = board.push(
            row![
                button("Confirm & save")
                    .on_press_maybe((!self.pending()).then_some(Message::SaveOperatorLineup)),
                button("Cancel")
                    .style(brand::secondary)
                    .on_press(Message::CancelOperatorLineup)
            ]
            .spacing(8),
        );
        scrollable(card(board, c).width(Fill)).height(Fill).into()
    }
    fn operator_history(&self, c: Colors) -> Element<'_, Message> {
        let saved = self.active().unwrap();
        let mut feed = column![text("PLAY BY PLAY").size(11).font(BOLD).color(c.muted)].spacing(8);
        for e in saved
            .sport_history
            .iter()
            .rev()
            .filter(|e| {
                !matches!(
                    e.command,
                    Some(Command::FieldAt {
                        command: F::Checkpoint { .. },
                        ..
                    }) | Some(Command::Field(F::Checkpoint { .. }))
                )
            })
            .take(100)
        {
            let label = match e.command {
                Some(Command::Player {
                    side,
                    player,
                    action,
                }) => format!(
                    "{} / {}",
                    player
                        .and_then(|id| self.game().player(side, id))
                        .map_or(&self.game().team(side).name, |p| &p.name),
                    action.label()
                ),
                Some(Command::Field(F::Event { side, player, kind })) => format!(
                    "{} / {}",
                    player
                        .and_then(|id| self.game().player(side, id))
                        .map_or(&self.game().team(side).name, |p| &p.name),
                    kind.label()
                ),
                Some(Command::FieldAt {
                    clock_ms,
                    command: F::Event { side, player, kind },
                    ..
                }) => format!(
                    "{} / {} / {}",
                    hustlx_oss::clock::format_game(clock_ms),
                    player
                        .and_then(|id| self.game().player(side, id))
                        .map_or(&self.game().team(side).name, |p| &p.name),
                    kind.label()
                ),
                _ => e.label(),
            };
            feed = feed.push(
                text(format!(
                    "#{} {}{}",
                    e.id,
                    label,
                    if e.undone { " / UNDONE" } else { "" }
                ))
                .size(11)
                .color(if e.undone { c.muted } else { c.text }),
            );
        }
        if saved.sport_history.is_empty() {
            feed = feed.push(text("Ready for the first action.").size(12).color(c.muted));
        }
        card(feed, c).padding(12).width(Fill).into()
    }

    fn operator_service_identity(&self, side: Side) -> String {
        let s = self.active().unwrap().sport_state.as_ref().unwrap();
        let Some(slot) = operator_service_slot(s, side) else {
            return String::new();
        };
        let player = slot
            .checked_sub(1)
            .and_then(|slot| s.operator.court[index(side)].get(usize::from(slot)))
            .and_then(|id| self.game().player(side, *id));
        format!(
            "{} / {}{slot}",
            player.map_or(self.game().team(side).name.as_str(), |p| p.name.as_str()),
            if s.sport == Sport::Volleyball {
                "P"
            } else {
                "slot "
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hustlx_oss::sport::{Format, Rules};
    use hustlx_oss::storage::Store;

    #[test]
    fn service_markers_follow_volleyball_live_slots_and_exchanges() {
        let mut s = State::new(Sport::Volleyball, Rules::defaults(Sport::Volleyball)).unwrap();
        assert_eq!(operator_service_role(&s, Side::Home, 1), None);
        for (side, players) in [
            (Side::Home, [1, 2, 3, 4, 5, 6, 0, 0, 0, 0, 0]),
            (Side::Away, [7, 8, 9, 10, 11, 12, 0, 0, 0, 0, 0]),
        ] {
            s.apply(Command::Lineup { side, players }).unwrap();
        }
        assert_eq!(operator_service_role(&s, Side::Home, 1), Some("SERVING"));
        assert_eq!(operator_service_role(&s, Side::Away, 7), None);
        s.apply(Command::Rally(Side::Away)).unwrap();
        assert_eq!(operator_service_role(&s, Side::Away, 8), Some("SERVING"));
        assert_eq!(operator_service_role(&s, Side::Away, 7), None);
        s.apply(Command::Exchange {
            side: Side::Away,
            outgoing: 8,
            incoming: 13,
            libero: true,
        })
        .unwrap();
        assert_eq!(operator_service_role(&s, Side::Away, 13), Some("SERVING"));
        assert_eq!(operator_service_role(&s, Side::Away, 8), None);
        s.status = Status::Completed;
        assert_eq!(operator_service_role(&s, Side::Away, 13), None);
    }

    #[test]
    fn racquet_markers_resolve_explicit_slots_without_roster_or_name_fallbacks() {
        for sport in [
            Sport::Pickleball,
            Sport::Badminton,
            Sport::TableTennis,
            Sport::Tennis,
            Sport::Padel,
        ] {
            let mut r = Rules::defaults(sport);
            r.format = Format::Doubles;
            let mut s = State::new(sport, r).unwrap();
            assert_eq!(operator_service_role(&s, Side::Home, 1), None);
            for (side, players) in [
                (Side::Home, [1, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
                (Side::Away, [3, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            ] {
                s.apply(Command::Lineup { side, players }).unwrap();
            }
            for _ in 0..5 {
                let server = s.operator.court[index(s.server)][usize::from(s.server_slot - 1)];
                let receiving = hustlx_oss::operator::opposite(s.server);
                let receiver = s.operator.court[index(receiving)][usize::from(s.receiver_slot - 1)];
                assert_eq!(
                    operator_service_role(&s, s.server, server),
                    Some("SERVING"),
                    "{sport:?}"
                );
                assert_eq!(
                    operator_service_role(&s, receiving, receiver),
                    Some("RECEIVING"),
                    "{sport:?}"
                );
                assert_eq!(operator_service_role(&s, receiving, 99), None);
                s.apply(Command::Rally(Side::Home)).unwrap();
            }
            s.status = Status::Decided;
            for id in 1..=4 {
                assert_eq!(operator_service_role(&s, Side::Home, id), None);
                assert_eq!(operator_service_role(&s, Side::Away, id), None);
            }
        }
        for sport in [Sport::Football, Sport::Futsal] {
            let mut s = State::new(sport, Rules::defaults(sport)).unwrap();
            s.operator.court[0] = vec![1];
            assert_eq!(operator_service_role(&s, Side::Home, 1), None);
        }
    }
    fn send(app: &mut App, message: Message) {
        let _ = app.update(message);
        assert!(app.error.is_none(), "{:?}", app.error);
    }
    fn fixture(sport: Sport) -> App {
        let store = Store::open(":memory:").unwrap();
        for side in ["North", "South"] {
            let team = store.create_team(side).unwrap();
            for n in 1..=8 {
                let player = store.create_player(&format!("{side} {n}")).unwrap();
                store.add_to_team(team, player, &n.to_string()).unwrap();
            }
        }
        let teams = store.teams(false).unwrap();
        let mut app = App::empty();
        app.session = Some(Session::new(store));
        for m in [
            Message::ChooseSport(sport),
            Message::HomeTeam(teams[0].clone()),
            Message::AwayTeam(teams[1].clone()),
            Message::MatchName(format!("{sport} final")),
            Message::MatchDate("2026-10-21".into()),
            Message::CreateMatch,
        ] {
            send(&mut app, m);
        }
        app
    }
    #[test]
    fn volleyball_confirmation_board_records_identity_rotation_and_exchange_without_cancel_writes()
    {
        let mut app = fixture(Sport::Volleyball);
        send(
            &mut app,
            Message::OpenOperatorLineup(Side::Home, false, false),
        );
        for id in 1..=6 {
            send(&mut app, Message::OperatorPick(id));
        }
        assert_eq!(app.active().unwrap().summary.revision, 0);
        send(&mut app, Message::SaveOperatorLineup);
        send(&mut app, Message::OperatorSelect(Side::Home, 1));
        assert_eq!(app.game().selected, Some((Side::Home, 1)));
        send(
            &mut app,
            Message::OperatorAction(Side::Home, Some(1), A::Kill),
        );
        assert_eq!(
            app.active().unwrap().sport_state.as_ref().unwrap().points,
            [1, 0]
        );
        assert_eq!(app.game().selected, Some((Side::Home, 1)));
        send(
            &mut app,
            Message::OpenOperatorLineup(Side::Home, true, false),
        );
        send(&mut app, Message::OperatorPick(1));
        send(&mut app, Message::OperatorPick(7));
        send(&mut app, Message::CancelOperatorLineup);
        assert_eq!(
            app.active()
                .unwrap()
                .sport_state
                .as_ref()
                .unwrap()
                .operator
                .court[0],
            vec![1, 2, 3, 4, 5, 6]
        );
        send(
            &mut app,
            Message::OpenOperatorLineup(Side::Home, true, true),
        );
        send(&mut app, Message::OperatorPick(1));
        send(&mut app, Message::OperatorPick(7));
        send(&mut app, Message::SaveOperatorLineup);
        assert_eq!(
            app.active()
                .unwrap()
                .sport_state
                .as_ref()
                .unwrap()
                .operator
                .libero[0],
            Some(7)
        );
        send(&mut app, Message::Undo);
        assert_eq!(
            app.active()
                .unwrap()
                .sport_state
                .as_ref()
                .unwrap()
                .operator
                .court[0],
            vec![1, 2, 3, 4, 5, 6]
        );
    }
    #[test]
    fn all_nine_sports_open_rosters_and_render_in_both_native_themes() {
        for sport in Sport::ALL {
            let mut app = fixture(sport);
            assert_eq!(app.active().unwrap().summary.sport, sport);
            assert_eq!(app.game().team(Side::Home).players.len(), 8);
            for dark in [false, true] {
                app.dark = dark;
                let _ = app.view();
            }
            if sport != Sport::Basketball {
                send(
                    &mut app,
                    Message::OpenOperatorLineup(Side::Away, false, false),
                );
                let _ = app.view();
                send(&mut app, Message::CancelOperatorLineup);
            }
        }
    }
    #[test]
    fn basketball_clicked_player_can_score_with_keys_without_toggling_selection_off() {
        let mut app = fixture(Sport::Basketball);
        for (side, id, key, score) in [(Side::Home, 1, "w", 2), (Side::Away, 9, "e", 3)] {
            send(&mut app, Message::Select(side, id));
            send(
                &mut app,
                Message::Key {
                    key: key.into(),
                    repeat: false,
                    modified: false,
                    captured: false,
                },
            );
            assert_eq!(app.game().score(side), score);
            assert_eq!(app.game().selected, None);
            for (repeat, modified, captured) in [
                (true, false, false),
                (false, true, false),
                (false, false, true),
            ] {
                send(
                    &mut app,
                    Message::Key {
                        key: key.into(),
                        repeat,
                        modified,
                        captured,
                    },
                );
                assert_eq!(app.game().score(side), score);
            }
        }
        send(
            &mut app,
            Message::Key {
                key: "Backspace".into(),
                repeat: false,
                modified: false,
                captured: false,
            },
        );
        assert_eq!(app.game().score(Side::Away), 0);
        assert_eq!(app.game().score(Side::Home), 2);
    }
    #[test]
    fn tennis_and_padel_result_labels_show_saved_set_games_instead_of_last_game_points() {
        for sport in [Sport::Tennis, Sport::Padel] {
            let mut rules = hustlx_oss::sport::Rules::defaults(sport);
            rules.best_of = 1;
            rules.target = 2;
            let mut state = State::new(sport, rules).unwrap();
            for _ in 0..3 {
                state.apply(Command::Rally(Side::Home)).unwrap();
            }
            assert_eq!(operator_score_label(&state, Side::Home), "40");
            for _ in 0..5 {
                state.apply(Command::Rally(Side::Home)).unwrap();
            }
            assert_eq!(state.status, Status::Decided);
            assert_eq!(operator_score_label(&state, Side::Home), "2");
            assert_eq!(operator_score_label(&state, Side::Away), "0");
            state.apply(Command::Complete).unwrap();
            assert_eq!(operator_score_label(&state, Side::Home), "2");
            let mut rules = hustlx_oss::sport::Rules::defaults(sport);
            rules.best_of = 1;
            rules.match_tiebreak = true;
            let mut state = State::new(sport, rules).unwrap();
            for side in [Side::Home, Side::Away] {
                for _ in 0..9 {
                    state.apply(Command::Rally(side)).unwrap();
                }
            }
            for _ in 0..2 {
                state.apply(Command::Rally(Side::Home)).unwrap();
            }
            assert_eq!(state.status, Status::Decided);
            assert!(operator_match_tiebreak_result(&state));
            assert_eq!(operator_score_label(&state, Side::Home), "11");
            assert_eq!(operator_score_label(&state, Side::Away), "9");
        }
    }
    #[test]
    fn escape_cancels_each_sport_draft_without_writing_or_leaking_basketball_keys() {
        for sport in Sport::ALL.into_iter().filter(|s| *s != Sport::Basketball) {
            let mut app = fixture(sport);
            let revision = app.active().unwrap().summary.revision;
            send(
                &mut app,
                Message::OpenOperatorLineup(Side::Home, false, false),
            );
            send(&mut app, Message::OperatorPick(1));
            for captured in [true, false] {
                send(
                    &mut app,
                    Message::Key {
                        key: "Escape".into(),
                        repeat: false,
                        modified: false,
                        captured,
                    },
                );
                assert_eq!(app.operator_draft.is_some(), captured, "{sport:?}");
            }
            send(&mut app, Message::ConfirmSportFinish);
            send(
                &mut app,
                Message::Key {
                    key: "Escape".into(),
                    repeat: false,
                    modified: false,
                    captured: false,
                },
            );
            assert!(!app.operator_finish);
            for key in [" ", "w", "e", "p", "Backspace"] {
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
            assert_eq!(app.active().unwrap().summary.revision, revision);
        }
    }
    #[test]
    fn football_and_futsal_closeout_is_confirmed_draw_aware_and_reopenable() {
        for sport in [Sport::Football, Sport::Futsal] {
            let mut app = fixture(sport);
            let id = app.active().unwrap().summary.id;
            send(
                &mut app,
                Message::SportCommand(Command::Field(F::Event {
                    side: Side::Home,
                    kind: hustlx_oss::field_sport::FieldEvent::OwnGoal,
                    player: Some(1),
                })),
            );
            assert_eq!(
                app.active().unwrap().sport_state.as_ref().unwrap().points,
                [0, 1]
            );
            send(&mut app, Message::Undo);
            send(&mut app, Message::ConfirmSportFinish);
            assert_eq!(
                app.active().unwrap().sport_state.as_ref().unwrap().status,
                Status::Live
            );
            send(&mut app, Message::FinishSport);
            send(&mut app, Message::OpenMatch(id));
            assert_eq!(
                app.active().unwrap().sport_state.as_ref().unwrap().status,
                Status::Completed
            );
            assert_eq!(
                app.active().unwrap().sport_state.as_ref().unwrap().winner,
                None
            );
            send(&mut app, Message::Undo);
            assert_eq!(
                app.active().unwrap().sport_state.as_ref().unwrap().status,
                Status::Live
            );
        }
    }
}
