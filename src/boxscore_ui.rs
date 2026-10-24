//! Box score screen: the printable sheet of a saved match, read from committed storage.
use super::*;
use crate::brand::BODY;
use hustlx_oss::boxscore::{
    Align, Block, Column as SheetColumn, LineScore, Row, Sheet, SheetResult, build, export,
    layout::{Metrics, layout},
    pdf::render,
};
use hustlx_oss::storage::Result;
use iced::Task;
use iced::widget::column;
use iced::widget::scrollable::{Direction, Scrollbar};
use std::path::PathBuf;

pub struct BoxscoreState {
    /// Suggested PDF name, fixed when the match is loaded so the dialog needs no storage read.
    pub file_name: String,
    pub sheet: SheetResult,
    pub export: ExportStatus,
}

pub enum ExportStatus {
    Idle,
    Saving,
    Saved(PathBuf),
    Failed(String),
}

/// Export button caption for each export state.
pub(super) fn export_label(status: &ExportStatus) -> &'static str {
    match status {
        ExportStatus::Failed(_) => "Retry export",
        ExportStatus::Saving => "Saving…",
        ExportStatus::Idle | ExportStatus::Saved(_) => "Export PDF",
    }
}

/// Lays out and renders the loaded sheet; the message is what the screen shows when it cannot.
pub(super) fn render_sheet(state: &BoxscoreState) -> std::result::Result<Vec<u8>, String> {
    match &state.sheet {
        SheetResult::Ready(sheet) => layout(sheet, &Metrics::bundled())
            .map(|laid_out| render(&laid_out))
            .map_err(|_| "The box score did not fit on one page.".to_owned()),
        SheetResult::Unsupported { .. } => {
            Err("Box score is not available for this sport yet.".to_owned())
        }
    }
}

/// Native save dialog on the Iced executor; `None` means the operator cancelled.
pub(super) fn export_task(default_name: String) -> Task<Message> {
    Task::perform(
        rfd::AsyncFileDialog::new()
            .set_file_name(default_name)
            .add_filter("PDF", &["pdf"])
            .save_file(),
        |handle| Message::BoxscoreTarget(handle.map(|h| h.path().to_path_buf())),
    )
}

fn save_error(error: &std::io::Error) -> String {
    format!("Could not save the PDF: {error}. Check the folder, then try again.")
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
        match message {
            Message::ShowBoxscore(id) => {
                let saved = self.store()?.load_match(id)?;
                let generated_on = chrono::Local::now().format("%Y-%m-%d").to_string();
                let sheet = build(&saved, None, &generated_on, env!("CARGO_PKG_VERSION"));
                self.boxscore = Some(BoxscoreState {
                    file_name: export::default_file_name(&saved.summary),
                    sheet,
                    export: ExportStatus::Idle,
                });
                self.navigate(Screen::BoxScore)?;
            }
            Message::ExportBoxscore => {
                if let Some(state) = self.boxscore.as_mut()
                    && matches!(state.sheet, SheetResult::Ready(_))
                {
                    state.export = ExportStatus::Saving;
                    return Ok((self.pick_export_target)(state.file_name.clone()));
                }
            }
            Message::BoxscoreTarget(None) => {
                if let Some(state) = self.boxscore.as_mut() {
                    state.export = ExportStatus::Idle;
                }
            }
            Message::BoxscoreTarget(Some(path)) => {
                if let Some(state) = self.boxscore.as_mut() {
                    match render_sheet(state) {
                        Ok(bytes) => {
                            state.export = ExportStatus::Saving;
                            return Ok(Task::perform(
                                async move {
                                    export::write_atomic(&path, &bytes)
                                        .map_err(|error| save_error(&error))
                                },
                                Message::BoxscoreWritten,
                            ));
                        }
                        Err(message) => state.export = ExportStatus::Failed(message),
                    }
                }
            }
            Message::BoxscoreWritten(result) => {
                if let Some(state) = self.boxscore.as_mut() {
                    state.export = match result {
                        Ok(path) => ExportStatus::Saved(path),
                        Err(message) => ExportStatus::Failed(message),
                    };
                }
            }
            Message::OpenBoxscoreFile | Message::RevealBoxscoreFile => {
                if let Some(state) = self.boxscore.as_mut()
                    && let ExportStatus::Saved(path) = &state.export
                {
                    let reveal = matches!(message, Message::RevealBoxscoreFile);
                    let target = if reveal {
                        path.parent().unwrap_or(path).to_path_buf()
                    } else {
                        path.clone()
                    };
                    if let Err(error) = open::that(&target) {
                        state.export = ExportStatus::Failed(format!(
                            "Could not open {}: {error}",
                            target.display()
                        ));
                    }
                }
            }
            _ => {}
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
        let state = self.boxscore.as_ref();
        let status = state.map_or(&ExportStatus::Idle, |s| &s.export);
        let can_export = state.is_some_and(|s| matches!(s.sheet, SheetResult::Ready(_)))
            && !matches!(status, ExportStatus::Saving);
        let bar = row![
            button(text("Back to Matches").size(12))
                .on_press(Message::Navigate(Screen::Matches))
                .padding([8, 12])
                .style(brand::secondary),
            Space::new().width(Fill),
            button(text(export_label(status)).size(12))
                .on_press_maybe(can_export.then_some(Message::ExportBoxscore))
                .padding([8, 12]),
        ]
        .spacing(10)
        .align_y(alignment::Vertical::Center);
        let status_line: Option<Element<'_, Message>> = match status {
            ExportStatus::Saved(path) => Some(
                row![
                    text(format!("Saved to {}", path.display()))
                        .size(11)
                        .color(c.muted)
                        .width(Fill),
                    button(text("Open").size(11))
                        .on_press(Message::OpenBoxscoreFile)
                        .style(brand::secondary),
                    button(text("Show in folder").size(11))
                        .on_press(Message::RevealBoxscoreFile)
                        .style(brand::secondary),
                ]
                .spacing(8)
                .align_y(alignment::Vertical::Center)
                .into(),
            ),
            ExportStatus::Failed(message) => Some(text(message).size(11).color(c.away).into()),
            ExportStatus::Idle | ExportStatus::Saving => None,
        };
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
        let mut page = column![bar].spacing(14).height(Fill);
        if let Some(line) = status_line {
            page = page.push(line);
        }
        page.push(scrollable(body).height(Fill)).into()
    }
}

/// Fixed-width line score: the header row first, then one row per side.
fn line_score_lines(ls: &LineScore) -> Vec<String> {
    let fmt = |name: &str, cells: &[String], total: &str| {
        let mut out = format!("{name:<24}");
        for value in cells {
            out.push_str(&format!("{value:>6}"));
        }
        out.push_str(&format!("{total:>8}"));
        out
    };
    // The sheet's columns already end with the total label, so split it off instead of adding one.
    let (total_label, period_labels) = ls
        .columns
        .split_last()
        .map_or(("", &ls.columns[..]), |(t, rest)| (t.as_str(), rest));
    let mut out = vec![fmt("", period_labels, total_label)];
    for r in &ls.rows {
        out.push(fmt(&r.name, &r.cells, &r.total));
    }
    out
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
        let rows = line_score_lines(ls);
        let mut lines = column![
            text(&ls.title).size(13).font(HEADING),
            text(rows[0].clone()).size(11).font(BOLD).color(c.muted),
        ]
        .spacing(4);
        for r in &rows[1..] {
            lines = lines.push(text(r.clone()).size(11).font(BODY));
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
    use iced::Task;
    use std::path::Path;

    fn app(path: &Path) -> App {
        let mut app = App::empty();
        app.session = Some(Session::new(Store::open(path).unwrap()));
        // Never open a native dialog from a test thread.
        app.pick_export_target = |_| Task::none();
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

    #[test]
    fn line_score_header_lists_each_column_once_with_a_single_total() {
        let SheetResult::Ready(sheet) = hustlx_oss::boxscore::build(
            &hustlx_oss::boxscore::fixtures::basketball_full(),
            None,
            "2026-10-24",
            "0.1.0",
        ) else {
            panic!("full fixture must be Ready");
        };
        let ls = sheet.line_score.as_ref().unwrap();
        let rows = super::line_score_lines(ls);
        let header: Vec<&str> = rows[0].split_whitespace().collect();
        assert_eq!(header, ["P1", "P2", "P3", "P4", "T"]);
        let first: Vec<&str> = rows[1].split_whitespace().collect();
        assert_eq!(
            first.len(),
            header.len() + 2,
            "name has two words, then one value per header column"
        );
    }

    fn state(app: &App) -> &super::BoxscoreState {
        app.boxscore.as_ref().unwrap()
    }

    #[test]
    fn cancelled_dialog_returns_to_idle_without_error() {
        let dir = tempfile::tempdir().unwrap();
        let mut app = app(&dir.path().join("library.sqlite3"));
        let id = basketball(&mut app);
        send(&mut app, Message::ShowBoxscore(id));
        assert_eq!(
            state(&app).file_name,
            "2026-10-24-home-vs-away-boxscore.pdf"
        );
        send(&mut app, Message::ExportBoxscore);
        assert!(matches!(state(&app).export, ExportStatus::Saving));
        send(&mut app, Message::BoxscoreTarget(None));
        assert!(matches!(state(&app).export, ExportStatus::Idle));
        assert!(app.error.is_none());
        let leftovers: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .is_some_and(|x| x == "pdf" || x == "tmp")
            })
            .collect();
        assert!(leftovers.is_empty(), "cancel must write nothing");
        render(&mut app);
    }

    #[test]
    fn chosen_target_writes_pdf_and_reports_saved() {
        let dir = tempfile::tempdir().unwrap();
        let mut app = app(&dir.path().join("library.sqlite3"));
        let id = basketball(&mut app);
        send(&mut app, Message::ShowBoxscore(id));
        let target = dir.path().join("out");
        send(&mut app, Message::BoxscoreTarget(Some(target.clone())));
        assert!(matches!(state(&app).export, ExportStatus::Saving));
        // The returned Task is not polled in tests; drive the write the way the task would.
        let bytes = super::render_sheet(state(&app)).unwrap();
        let written = hustlx_oss::boxscore::export::write_atomic(&target, &bytes).unwrap();
        send(&mut app, Message::BoxscoreWritten(Ok(written.clone())));
        assert!(matches!(&state(&app).export, ExportStatus::Saved(p) if *p == written));
        assert_eq!(written, dir.path().join("out.pdf"));
        assert!(std::fs::read(&written).unwrap().starts_with(b"%PDF-"));
        assert_eq!(super::export_label(&state(&app).export), "Export PDF");
        render(&mut app);
    }

    #[test]
    fn failed_write_shows_retry_message() {
        let dir = tempfile::tempdir().unwrap();
        let mut app = app(&dir.path().join("library.sqlite3"));
        let id = basketball(&mut app);
        send(&mut app, Message::ShowBoxscore(id));
        send(
            &mut app,
            Message::BoxscoreWritten(Err("Could not save the PDF: denied".into())),
        );
        assert!(matches!(&state(&app).export, ExportStatus::Failed(m) if m.contains("denied")));
        assert_eq!(super::export_label(&state(&app).export), "Retry export");
        assert_eq!(super::export_label(&ExportStatus::Saving), "Saving…");
        render(&mut app);
    }
}
