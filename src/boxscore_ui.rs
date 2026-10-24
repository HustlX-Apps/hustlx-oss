//! Box score screen: the printable sheet of a saved match, read from committed storage.
use super::*;
use crate::brand::BODY;
use hustlx_oss::boxscore::{Align, Block, Column as SheetColumn, Row, Sheet, SheetResult, build};
use hustlx_oss::storage::Result;
use iced::Task;
use iced::widget::column;
use iced::widget::scrollable::{Direction, Scrollbar};
use std::path::PathBuf;

// match_id and export are read by the export flow.
#[allow(dead_code)]
pub struct BoxscoreState {
    pub match_id: i64,
    pub sheet: SheetResult,
    pub export: ExportStatus,
}

// Saving, Saved and Failed are driven by the export flow.
#[allow(dead_code)]
pub enum ExportStatus {
    Idle,
    Saving,
    Saved(PathBuf),
    Failed(String),
}

const CHARS_PER_WEIGHT: usize = 7;

fn cell(value: &str, column: &SheetColumn) -> String {
    let w = usize::from(column.weight) * CHARS_PER_WEIGHT;
    match column.align {
        Align::Left => format!("{value:<w$}"),
        Align::Right => format!("{value:>w$}"),
        Align::Center => format!("{value:^w$}"),
    }
}

fn line(row: &Row, columns: &[SheetColumn]) -> String {
    columns
        .iter()
        .zip(&row.cells)
        .map(|(column, value)| cell(value, column))
        .collect()
}

fn wide<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    scrollable(content)
        .direction(Direction::Horizontal(Scrollbar::default()))
        .width(Fill)
        .into()
}

impl App {
    pub(super) fn update_boxscore(&mut self, message: Message) -> Result<Task<Message>> {
        if let Message::ShowBoxscore(id) = message {
            let saved = self.store()?.load_match(id)?;
            let generated_on = chrono::Local::now().format("%Y-%m-%d").to_string();
            let sheet = build(&saved, None, &generated_on, env!("CARGO_PKG_VERSION"));
            self.boxscore = Some(BoxscoreState {
                match_id: id,
                sheet,
                export: ExportStatus::Idle,
            });
            self.navigate(Screen::BoxScore)?;
        }
        Ok(Task::none())
    }

    /// Box score entry point; disabled while a save is pending so the sheet is committed truth.
    pub(super) fn boxscore_button<'a>(
        &self,
        label: iced::widget::Text<'a>,
        id: i64,
        c: Colors,
    ) -> Element<'a, Message> {
        let pending = self.pending();
        let mut out = row![
            button(label)
                .on_press_maybe((!pending).then_some(Message::ShowBoxscore(id)))
                .style(brand::secondary)
        ]
        .spacing(6)
        .align_y(alignment::Vertical::Center);
        if pending {
            out = out.push(text("Save pending").size(10).color(c.muted));
        }
        out.into()
    }

    pub(super) fn boxscore_view(&self, c: Colors) -> Element<'_, Message> {
        let bar = row![
            button(text("Back to Matches").size(12))
                .on_press(Message::Navigate(Screen::Matches))
                .padding([8, 12])
                .style(brand::secondary),
            Space::new().width(Fill),
            button(text("Export PDF").size(12))
                .on_press_maybe(None)
                .padding([8, 12]),
        ]
        .spacing(10)
        .align_y(alignment::Vertical::Center);
        let body: Element<'_, Message> = match self.boxscore.as_ref().map(|s| &s.sheet) {
            Some(SheetResult::Ready(sheet)) => sheet_view(sheet, c),
            _ => card(
                text("Box score is not available for this sport yet.")
                    .size(14)
                    .color(c.muted),
                c,
            )
            .width(Fill)
            .into(),
        };
        column![bar, scrollable(body).height(Fill)]
            .spacing(14)
            .height(Fill)
            .into()
    }
}

fn sheet_view(sheet: &Sheet, c: Colors) -> Element<'_, Message> {
    let h = &sheet.header;
    let place = match &h.venue {
        Some(venue) => format!("{}  /  {venue}", h.date),
        None => h.date.clone(),
    };
    let header = card(
        column![
            row![
                column![
                    text(&h.match_name).size(18).font(HEADING),
                    text(place).size(11).color(c.muted),
                ]
                .spacing(4)
                .width(Fill),
                badge(h.status.clone(), c.green, c),
            ]
            .align_y(alignment::Vertical::Center),
            row![
                text(&h.home_name).size(14).font(HEADING).color(c.home),
                text(&h.home_score).size(32).font(DISPLAY),
                text("-").size(24).color(c.muted),
                text(&h.away_score).size(32).font(DISPLAY),
                text(&h.away_name).size(14).font(HEADING).color(c.away),
            ]
            .spacing(14)
            .align_y(alignment::Vertical::Center),
        ]
        .spacing(10),
        c,
    )
    .width(Fill);
    let mut page = column![header].spacing(14);
    if let Some(ls) = &sheet.line_score {
        let fmt = |name: &str, cells: &[String], total: &str| {
            let mut out = format!("{name:<24}");
            for value in cells {
                out.push_str(&format!("{value:>6}"));
            }
            out.push_str(&format!("{total:>8}"));
            out
        };
        let labels = fmt("", &ls.columns, "T");
        let mut lines = column![
            text(&ls.title).size(13).font(HEADING),
            text(labels).size(11).font(BOLD).color(c.muted),
        ]
        .spacing(4);
        for r in &ls.rows {
            lines = lines.push(text(fmt(&r.name, &r.cells, &r.total)).size(11).font(BODY));
        }
        page = page.push(card(wide(lines), c).width(Fill));
    }
    for team in &sheet.teams {
        let labels = team
            .columns
            .iter()
            .map(|column| cell(column.label, column))
            .collect::<String>();
        let mut table = column![text(labels).size(11).font(BOLD).color(c.muted)].spacing(3);
        for r in &team.rows {
            table = table.push(text(line(r, &team.columns)).size(11).font(if r.emphasis {
                SEMIBOLD
            } else {
                BODY
            }));
        }
        table = table.push(text(line(&team.totals, &team.columns)).size(11).font(BOLD));
        let mut block = column![
            text(format!("{}  ({})", team.name, team.side_letter))
                .size(15)
                .font(HEADING),
            wide(table),
        ]
        .spacing(8);
        for note in &team.notes {
            block = block.push(text(note).size(11).color(c.muted));
        }
        page = page.push(card(block, c).width(Fill));
    }
    if let Some(Block { title, lines }) = &sheet.extra {
        let mut block = column![text(title).size(13).font(HEADING)].spacing(4);
        for l in lines {
            block = block.push(text(l).size(11));
        }
        page = page.push(card(block, c).width(Fill));
    }
    if let Some(legend) = &sheet.legend {
        page = page.push(text(legend).size(10).color(c.muted));
    }
    page.into()
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use super::ExportStatus;
    use hustlx_oss::boxscore::SheetResult;
    use hustlx_oss::sport::{Rules, Sport};
    use hustlx_oss::storage::{MatchDraft, SportMatchDraft};
    use std::path::Path;

    fn app(path: &Path) -> App {
        let mut app = App::empty();
        app.session = Some(Session::new(Store::open(path).unwrap()));
        app
    }

    fn send(app: &mut App, message: Message) {
        let _ = app.update(message);
        assert!(app.error.is_none(), "{:?}", app.error);
    }

    fn render(app: &mut App) {
        for dark in [false, true] {
            app.dark = dark;
            let _ = app.view();
        }
    }

    fn basketball(app: &mut App) -> i64 {
        let db = app.session.as_mut().unwrap().store_mut();
        let team = |name: &str| {
            let team = db.create_team(name).unwrap();
            for jersey in ["4", "6", "9", "11", "23"] {
                let p = db.create_player(&format!("{name} {jersey}")).unwrap();
                db.add_to_team(team, p, jersey).unwrap();
            }
            team
        };
        let (home, away) = (team("Home"), team("Away"));
        db.create_match(&MatchDraft {
            name: "Screen Box Score".into(),
            date: "2026-10-24".into(),
            venue: "Gym".into(),
            home_team_id: home,
            away_team_id: away,
        })
        .unwrap()
        .summary
        .id
    }

    #[test]
    fn show_boxscore_loads_committed_match_and_switches_screen() {
        let dir = tempfile::tempdir().unwrap();
        let mut app = app(&dir.path().join("library.sqlite3"));
        let id = basketball(&mut app);
        send(&mut app, Message::ShowBoxscore(id));
        assert_eq!(app.screen, Screen::BoxScore);
        assert!(app.active().is_none(), "box score must not open the match");
        let state = app.boxscore.as_ref().unwrap();
        assert_eq!(state.match_id, id);
        assert!(matches!(state.sheet, SheetResult::Ready(_)));
        assert!(matches!(state.export, ExportStatus::Idle));
        render(&mut app);
    }

    #[test]
    fn show_boxscore_for_volleyball_match_renders_unsupported_card() {
        let dir = tempfile::tempdir().unwrap();
        let mut app = app(&dir.path().join("library.sqlite3"));
        let id = app
            .session
            .as_mut()
            .unwrap()
            .store_mut()
            .create_sport_match(&SportMatchDraft {
                name: "Volley".into(),
                date: "2026-10-24".into(),
                venue: "Court".into(),
                home_name: "North".into(),
                away_name: "South".into(),
                sport: Sport::Volleyball,
                rules: Rules::defaults(Sport::Volleyball),
            })
            .unwrap()
            .summary
            .id;
        send(&mut app, Message::ShowBoxscore(id));
        assert_eq!(app.screen, Screen::BoxScore);
        assert!(matches!(
            &app.boxscore.as_ref().unwrap().sheet,
            SheetResult::Unsupported { sport_label } if sport_label == "Volleyball"
        ));
        render(&mut app);
    }

    #[test]
    fn show_boxscore_from_game_day_pauses_running_clock() {
        let dir = tempfile::tempdir().unwrap();
        let mut app = app(&dir.path().join("library.sqlite3"));
        let id = basketball(&mut app);
        send(&mut app, Message::OpenMatch(id));
        send(&mut app, Message::Clock(ClockCommand::Start));
        let session = app.session.as_ref().unwrap();
        assert!(session.clock_at(Instant::now()).running);
        send(&mut app, Message::ShowBoxscore(id));
        assert_eq!(app.screen, Screen::BoxScore);
        let session = app.session.as_ref().unwrap();
        assert!(!session.clock_at(Instant::now()).running);
        render(&mut app);
    }

    #[test]
    fn navigate_to_matches_keeps_nothing_stale() {
        let dir = tempfile::tempdir().unwrap();
        let mut app = app(&dir.path().join("library.sqlite3"));
        let id = basketball(&mut app);
        send(&mut app, Message::ShowBoxscore(id));
        send(&mut app, Message::Navigate(Screen::Matches));
        assert_eq!(app.screen, Screen::Matches);
        assert!(app.boxscore.is_none());
        render(&mut app);
    }
}
