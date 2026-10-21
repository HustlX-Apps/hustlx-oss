//! Native field-sport operator controls, using the saved command lane for every action.

use super::*;
use hustlx_oss::{
    field_sport::{FieldCommand, FieldEvent, SHOOTOUT_PERIOD},
    sport::Command,
};
use iced::widget::column;
use iced::widget::text_input;

impl App {
    pub(super) fn field_controls(&self, c: Colors) -> Element<'_, Message> {
        let saved = self.active().expect("Field controls require an open match");
        let committed = saved
            .sport_state
            .as_ref()
            .and_then(|state| state.field.as_ref())
            .expect("Field controls require a field-sport state");
        let field = self
            .session
            .as_ref()
            .map(|session| session.field_clock_at(self.now))
            .unwrap_or_else(|| committed.clone());
        let enabled = !self.pending() && !field.completed;
        let futsal = field.is_futsal();
        let shootout = field.period == SHOOTOUT_PERIOD;
        let period_label = match field.period {
            1 => {
                if futsal {
                    "PERIOD 1"
                } else {
                    "1ST HALF"
                }
            }
            2 => {
                if futsal {
                    "PERIOD 2"
                } else {
                    "2ND HALF"
                }
            }
            3 => "EXTRA TIME 1",
            4 => "EXTRA TIME 2",
            _ => "PENALTY SHOOTOUT",
        };
        let clock_label = if field.completed {
            "FULL TIME"
        } else {
            period_label
        };
        let mut controls = column![
            row![
                text(clock_label)
                    .font(HEADING)
                    .size(12)
                    .color(c.muted)
                    .width(Fill),
                badge(
                    if field.running { "RUNNING" } else { "PAUSED" },
                    if field.running { c.green } else { c.muted },
                    c
                ),
            ]
            .align_y(alignment::Vertical::Center),
        ]
        .spacing(9);
        if shootout {
            controls = controls
                .push(
                    text(format!(
                        "PENALTIES  {} : {}",
                        field.shootout[0], field.shootout[1]
                    ))
                    .font(DISPLAY)
                    .size(28)
                    .color(c.away),
                )
                .push(
                    text("Kicks are recorded separately from the match score.")
                        .size(11)
                        .color(c.muted),
                );
        } else {
            controls = controls
                .push(
                    row![
                        text(field_time(field.clock_ms))
                            .font(DISPLAY)
                            .size(42)
                            .width(Fill),
                        column![
                            text(if futsal { "STOPPED CLOCK" } else { "COUNT UP" })
                                .size(10)
                                .font(BOLD)
                                .color(c.muted),
                            text(format!("{} period", field_time(field.period_ms())))
                                .size(10)
                                .color(c.muted),
                        ]
                        .spacing(3),
                    ]
                    .align_y(alignment::Vertical::Center),
                )
                .push(
                    row![
                        button(if field.running {
                            "Pause clock"
                        } else {
                            "Start clock"
                        })
                        .padding([8, 10])
                        .width(Fill)
                        .on_press_maybe(
                            (enabled && (field.running || !futsal || field.clock_ms > 0))
                                .then_some(Message::SportCommand(Command::Field(
                                    if field.running {
                                        FieldCommand::Pause
                                    } else {
                                        FieldCommand::Start
                                    }
                                )))
                        ),
                        button("Next period")
                            .style(brand::secondary)
                            .padding([8, 10])
                            .width(Fill)
                            .on_press_maybe(
                                (enabled && !field.running && field.period < SHOOTOUT_PERIOD)
                                    .then_some(Message::SportCommand(Command::Field(
                                        FieldCommand::NextPeriod
                                    )))
                            ),
                    ]
                    .spacing(7),
                )
                .push(
                    row![
                        text_input("Correct MM:SS", &self.operator_time)
                            .on_input_maybe(enabled.then_some(Message::OperatorTime))
                            .padding(8)
                            .size(12)
                            .width(Fill),
                        button(text("Set time").size(12))
                            .style(brand::secondary)
                            .padding([8, 10])
                            .on_press_maybe(enabled.then_some(Message::ApplyOperatorTime)),
                    ]
                    .spacing(7),
                );
        }
        let mut teams = row![].spacing(9);
        for (index, side) in [Side::Home, Side::Away].into_iter().enumerate() {
            let accent = if side == Side::Home { c.home } else { c.away };
            let team_name = if side == Side::Home {
                &saved.summary.home_name
            } else {
                &saved.summary.away_name
            };
            let player = self
                .game()
                .selected
                .filter(|(selected_side, _)| *selected_side == side)
                .map(|(_, id)| id);
            let actor = player
                .and_then(|id| self.game().player(side, id))
                .map_or_else(|| "Team event".to_owned(), |player| player.name.clone());
            let action = |label: &'static str, kind| {
                field_event_button(label, side, kind, player, enabled, accent, c)
            };
            let mut side_controls = column![
                text(team_name).size(12).font(BOLD).color(accent),
                text(actor).size(10).color(c.muted),
            ]
            .spacing(6)
            .width(Fill);
            if shootout {
                side_controls = side_controls
                    .push(action("Scored", FieldEvent::ShootoutGoal))
                    .push(action("Missed", FieldEvent::ShootoutMiss))
                    .push(
                        text(format!(
                            "{} kicks / {} scored",
                            field.shootout_attempts[index], field.shootout[index]
                        ))
                        .size(10)
                        .color(c.muted),
                    );
            } else {
                side_controls = side_controls
                    .push(
                        row![
                            action("Goal", FieldEvent::Goal),
                            action("Own goal", FieldEvent::OwnGoal)
                        ]
                        .spacing(5),
                    )
                    .push(
                        row![
                            action("PK scored", FieldEvent::PenaltyGoal),
                            action("PK missed", FieldEvent::PenaltyMiss)
                        ]
                        .spacing(5),
                    )
                    .push(
                        row![
                            action("Shot", FieldEvent::Shot),
                            action("Save", FieldEvent::Save)
                        ]
                        .spacing(5),
                    )
                    .push(
                        row![
                            action("Assist", FieldEvent::Assist),
                            action("Foul", FieldEvent::Foul)
                        ]
                        .spacing(5),
                    )
                    .push(
                        row![
                            action("Yellow", FieldEvent::Yellow),
                            action("Red", FieldEvent::Red)
                        ]
                        .spacing(5),
                    )
                    .push(
                        text(format!(
                            "SH {}  SV {}  AST {}",
                            field.shots[index], field.saves[index], field.assists[index]
                        ))
                        .size(10)
                        .color(c.muted),
                    );
            }
            side_controls = side_controls.push(
                text(format!(
                    "YC {}  RC {}  Fouls {}",
                    field.yellow[index], field.red[index], field.fouls[index]
                ))
                .size(10)
                .color(c.muted),
            );
            if futsal && !shootout && field.fouls[index] >= 6 {
                side_controls = side_controls.push(
                    text("6+ accumulated fouls. Review direct kick with officials.")
                        .size(10)
                        .color(c.away),
                );
            }
            teams = teams.push(side_controls);
        }
        controls = controls.push(teams);
        if futsal && !shootout {
            controls = controls.push(
                text("Fouls reset at halftime and carry through extra time. Guidance never blocks agreed play.")
                    .size(10)
                    .color(c.muted),
            );
        }
        controls = controls.push(
            button(
                text(if field.completed {
                    "Result saved"
                } else {
                    "Review & finish match"
                })
                .font(SEMIBOLD)
                .size(12),
            )
            .style(brand::secondary)
            .width(Fill)
            .padding([9, 10])
            .on_press_maybe(enabled.then_some(Message::ConfirmSportFinish)),
        );
        controls.into()
    }
}

fn field_event_button<'a>(
    label: &'static str,
    side: Side,
    kind: FieldEvent,
    player: Option<i64>,
    enabled: bool,
    accent: Color,
    c: Colors,
) -> Element<'a, Message> {
    button(text(label).size(11).font(SEMIBOLD))
        .on_press_maybe(enabled.then_some(Message::SportCommand(Command::Field(
            FieldCommand::Event { side, kind, player },
        ))))
        .padding([7, 5])
        .width(Fill)
        .style(move |_, status| {
            let disabled = status == button::Status::Disabled;
            button::Style {
                background: Some(
                    blend(
                        c.panel,
                        accent,
                        if disabled {
                            0.05
                        } else if status == button::Status::Hovered {
                            0.25
                        } else {
                            0.12
                        },
                    )
                    .into(),
                ),
                text_color: if disabled { c.muted } else { accent },
                border: Border {
                    color: blend(c.border, accent, if disabled { 0.05 } else { 0.25 }),
                    width: 1.0,
                    radius: 6.0.into(),
                },
                ..Default::default()
            }
        })
        .into()
}

fn field_time(milliseconds: u64) -> String {
    let seconds = milliseconds / 1_000;
    format!("{:02}:{:02}", seconds / 60, seconds % 60)
}
