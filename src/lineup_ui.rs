use super::*;
use hustlx_oss::{
    clock::{format_game, format_shot},
    lineup::{LineupDraft, LineupKind},
    storage::{Error, Result},
};
use iced::widget::column;
impl App {
    pub(super) fn update_lineups(&mut self, message: Message) -> Result<()> {
        match message {
            Message::OpenLineup(side, kind) => {
                if self.pending() {
                    return Err(Error::Validation(
                        "Retry the pending save before editing a lineup.".into(),
                    ));
                }
                let saved = self
                    .active()
                    .ok_or_else(|| Error::Validation("Open a match first.".into()))?;
                self.lineup_draft = Some(LineupDraft::new(saved, side, kind));
                self.lineup_history = false;
                self.clock_edit = false;
                self.show_hotkeys = false;
                self.hotkeys.clear();
            }
            Message::CancelLineup => {
                if !self.pending() {
                    self.lineup_draft = None;
                    self.lineup_history = false;
                    self.hotkeys.clear();
                }
            }
            Message::LineupPick(id) => {
                if !self.pending()
                    && let Some(mut draft) = self.lineup_draft.take()
                {
                    let result = draft.pick(self.game(), id);
                    self.lineup_draft = Some(draft);
                    result?;
                }
            }
            Message::LineupRemove(id) => {
                if !self.pending()
                    && let Some(draft) = &mut self.lineup_draft
                {
                    draft.remove(id);
                }
            }
            Message::ClearLineup => {
                if !self.pending()
                    && let Some(draft) = &mut self.lineup_draft
                {
                    draft.clear();
                }
            }
            Message::SaveLineup => {
                let draft = self
                    .lineup_draft
                    .as_ref()
                    .ok_or_else(|| Error::Validation("Open a lineup first.".into()))?
                    .clone();
                if self.active().is_none_or(|s| s.summary.id != draft.match_id) {
                    return Err(Error::Validation(
                        "This draft belongs to another match. Reopen its lineup.".into(),
                    ));
                }
                let result = self.session_mut()?.save_lineup_at(
                    draft.side,
                    draft.kind,
                    draft.court,
                    Instant::now(),
                );
                if result.is_ok() {
                    self.lineup_draft = None;
                    self.hotkeys.clear();
                } else if self.pending() {
                    self.lineup_saving = true;
                }
                result?;
            }
            Message::ShowLineupHistory => {
                self.lineup_draft = None;
                self.lineup_history = true;
                self.clock_edit = false;
                self.show_hotkeys = false;
                self.hotkeys.clear();
            }
            _ => {}
        }
        Ok(())
    }
    pub(super) fn lineup_editor(&self, c: Colors) -> Element<'_, Message> {
        let draft = self.lineup_draft.as_ref().expect("Open lineup draft");
        let team = self.game().team(draft.side);
        let clock = self.current_clock();
        let blocked = self.pending();
        let title = format!("{} / {}", draft.kind.label(), team.name);
        let instructions = if draft.kind == LineupKind::Starters {
            "Tap bench players to add starters; tap a starter to remove. Saving also replaces the current on-court lineup."
        } else if draft.outgoing.is_some() {
            "Player going out is highlighted. Tap a bench player to swap them into the same court slot."
        } else {
            "Tap an on-court player, then a bench replacement. You can stage several swaps before confirming."
        };
        let mut top = column![
            row![
                text(title).size(24).font(HEADING).width(Fill),
                button("Cancel")
                    .on_press_maybe((!blocked).then_some(Message::CancelLineup))
                    .style(brand::secondary)
            ]
            .spacing(12),
            row![
                text(format!(
                    "Q{} / {} game / {} shot / {}",
                    clock.period,
                    format_game(clock.game_ms),
                    format_shot(clock.shot_ms),
                    if clock.running { "RUNNING" } else { "PAUSED" }
                ))
                .size(12)
                .color(c.muted)
                .width(Fill),
                button(
                    text(if clock.running {
                        "Pause clock"
                    } else {
                        "Start clock"
                    })
                    .size(11)
                )
                .on_press_maybe((!blocked).then_some(Message::ToggleClock))
                .style(brand::secondary)
            ]
            .spacing(12)
            .align_y(alignment::Vertical::Center),
            text(instructions).size(13).color(c.muted)
        ]
        .spacing(12);
        if let Some(warning) = draft.warning() {
            top = top.push(text(warning).size(12).color(c.away));
        }
        let mut court = Column::new().spacing(8);
        let mut bench = Column::new().spacing(8);
        for (slot, id) in draft.court.iter().enumerate() {
            if let Some(player) = self.game().player(draft.side, *id) {
                let chosen = draft.outgoing == Some(*id);
                court = court.push(
                    row![
                        self.lineup_tile(player, Some(slot + 1), chosen, c),
                        button(text("Remove").size(11))
                            .on_press_maybe((!blocked).then_some(Message::LineupRemove(*id)))
                            .style(brand::secondary)
                    ]
                    .spacing(8)
                    .align_y(alignment::Vertical::Center),
                );
            }
        }
        if draft.court.is_empty() {
            court = court.push(
                text("Court is empty. Add players from the bench.")
                    .size(13)
                    .color(c.muted),
            );
        }
        for player in team.players.iter().filter(|p| !draft.court.contains(&p.id)) {
            bench = bench.push(self.lineup_tile(player, None, false, c));
        }
        if draft.court.len() == team.players.len() {
            bench = bench.push(text("No bench players.").size(13).color(c.muted));
        }
        let board = row![
            card(
                column![
                    row![
                        text(format!("ON COURT / {}", draft.court.len()))
                            .font(BOLD)
                            .size(12)
                            .width(Fill),
                        button(text("Clear all").size(11))
                            .on_press_maybe((!blocked).then_some(Message::ClearLineup))
                            .style(brand::secondary)
                    ]
                    .spacing(10),
                    scrollable(court).height(Fill)
                ]
                .spacing(12),
                c
            )
            .width(Fill)
            .height(Fill),
            card(
                column![
                    text("BENCH").font(BOLD).size(12),
                    scrollable(bench).height(Fill)
                ]
                .spacing(12),
                c
            )
            .width(Fill)
            .height(Fill)
        ]
        .spacing(14)
        .height(Fill);
        let label = if blocked {
            "Saving..."
        } else if draft.kind == LineupKind::Starters {
            "Save starters"
        } else {
            "Confirm subs"
        };
        card(
            column![
                top,
                board,
                row![
                    text(if draft.changed() {
                        "Unconfirmed changes stay in this board until you save."
                    } else {
                        "Changes to this board are saved only when you confirm."
                    })
                    .size(11)
                    .color(c.muted)
                    .width(Fill),
                    button(text(label).font(SEMIBOLD).size(13))
                        .padding([10, 16])
                        .on_press_maybe((!blocked).then_some(Message::SaveLineup))
                ]
                .spacing(12)
                .align_y(alignment::Vertical::Center)
            ]
            .spacing(16),
            c,
        )
        .height(Fill)
        .width(Fill)
        .into()
    }
    fn lineup_tile<'a>(
        &self,
        p: &'a hustlx_oss::Player,
        slot: Option<usize>,
        selected: bool,
        c: Colors,
    ) -> Element<'a, Message> {
        let label = match slot {
            Some(n) => format!("Slot {n} / #{}", jersey_label(&p.jersey)),
            None => format!("#{}", jersey_label(&p.jersey)),
        };
        button(
            row![
                text(label).size(16).font(DISPLAY).width(105),
                column![
                    text(&p.name).size(15).font(SEMIBOLD),
                    text(hustlx_oss::profile::encode(&p.positions))
                        .size(11)
                        .color(c.muted)
                ]
                .spacing(4)
                .width(Fill)
            ]
            .spacing(10)
            .align_y(alignment::Vertical::Center),
        )
        .on_press_maybe((!self.pending()).then_some(Message::LineupPick(p.id)))
        .padding([12, 10])
        .width(Fill)
        .style(move |theme, status| {
            let mut style = brand::secondary(theme, status);
            if selected {
                style.border.color = c.home;
                style.border.width = 2.0;
                style.background = Some(blend(c.row, c.home, 0.15).into());
            }
            style
        })
        .into()
    }
    pub(super) fn lineup_history_view(&self, c: Colors) -> Element<'_, Message> {
        let saved = self.active().expect("Match history");
        let mut entries = Column::new().spacing(12);
        if saved.lineups.is_empty() {
            entries = entries.push(
                text("No confirmed lineup changes yet.")
                    .size(16)
                    .color(c.muted),
            );
        }
        for change in saved.lineups.iter().rev() {
            let names = |ids: &[i64]| {
                ids.iter()
                    .map(|id| {
                        self.game().player(change.side, *id).map_or_else(
                            || format!("Player {id}"),
                            |p| format!("#{} {}", jersey_label(&p.jersey), p.name),
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            let out: Vec<_> = change
                .before
                .iter()
                .filter(|p| !change.court.contains(p))
                .copied()
                .collect();
            let incoming: Vec<_> = change
                .court
                .iter()
                .filter(|p| !change.before.contains(p))
                .copied()
                .collect();
            let mut content = column![
                text(format!(
                    "{} / {} / Q{} / {}",
                    self.game().team(change.side).name,
                    change.kind.label(),
                    change.clock.period,
                    format_game(change.clock.game_ms)
                ))
                .font(SEMIBOLD)
                .size(14)
            ]
            .spacing(8);
            if change.kind == LineupKind::Substitution {
                content = content.push(
                    text(format!(
                        "OUT: {}",
                        if out.is_empty() {
                            "None".into()
                        } else {
                            names(&out)
                        }
                    ))
                    .size(12)
                    .color(c.muted),
                );
                content = content.push(
                    text(format!(
                        "IN: {}",
                        if incoming.is_empty() {
                            "None".into()
                        } else {
                            names(&incoming)
                        }
                    ))
                    .size(12)
                    .color(c.green),
                );
            }
            content = content.push(
                text(format!(
                    "Court: {}",
                    if change.court.is_empty() {
                        "Empty".into()
                    } else {
                        names(&change.court)
                    }
                ))
                .size(12),
            );
            entries = entries.push(card(content, c));
        }
        card(column![row![text("Lineup history").font(HEADING).size(24).width(Fill),button("Back to Game Day").on_press(Message::CancelLineup).style(brand::secondary)].spacing(12),text("Confirmed changes are saved with the match. Undo last play affects stats; reverse a substitution with another confirmed swap.").size(12).color(c.muted),scrollable(entries).height(Fill)].spacing(16),c).width(Fill).height(Fill).into()
    }
}
