//! One-page A4 layout for a box score `Sheet`.
//!
//! Pure geometry: a `Sheet` and font metrics in, absolute page items out. Coordinates are
//! PDF points with the origin at the bottom-left; a text `y` is its baseline and a text `x`
//! is its anchor for `align` (left edge, right edge or center).
use super::{Align, Column, Row, Sheet, TeamBlock};
use ttf_parser::Face;

pub const PAGE_W: f32 = 595.28;
pub const PAGE_H: f32 = 841.89;
pub const MARGIN: f32 = 24.0;

const USABLE: f32 = PAGE_W - 2.0 * MARGIN;
const TOP: f32 = PAGE_H - MARGIN;
/// Content must end above the footer zone.
const FOOTER_TOP: f32 = MARGIN + 14.0;
const GAP: f32 = 6.0;
const PAD: f32 = 2.0;
const ELLIPSIS: char = '\u{2026}';
const IDENTITY_H: f32 = 20.0;
const LOGO_H: f32 = 36.0;
const HEADER_H: f32 = 54.0;
const TEAM_BAND_H: f32 = 14.0;
const LINE_NAME_W: f32 = 160.0;
const LINE_CELL_W: f32 = 24.0;
const LEGEND_SIZE: f32 = 6.5;
const FOOTER_SIZE: f32 = 7.0;
const SHORT_NAME_CHARS: usize = 18;
const MAX_ROWS: usize = 18;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontId {
    Body,
    BodyBold,
    Heading,
    Display,
}

const FONTS: [FontId; 4] = [
    FontId::Body,
    FontId::BodyBold,
    FontId::Heading,
    FontId::Display,
];

/// Advance-width metrics for the bundled fonts, one parsed face per `FontId`.
pub struct Metrics {
    faces: [Face<'static>; 4],
}

impl Metrics {
    pub fn bundled() -> Metrics {
        Metrics {
            faces: FONTS.map(|f| Face::parse(Self::bytes(f), 0).expect("bundled font parses")),
        }
    }

    pub fn bytes(font: FontId) -> &'static [u8] {
        match font {
            FontId::Body => include_bytes!("../../assets/fonts/Inter-Regular.ttf"),
            FontId::BodyBold => include_bytes!("../../assets/fonts/Inter-Bold.ttf"),
            FontId::Heading => include_bytes!("../../assets/fonts/Exo-Bold.ttf"),
            FontId::Display => include_bytes!("../../assets/fonts/Exo-Black.ttf"),
        }
    }

    /// Sum of advance widths in points; characters without a glyph count as `?`.
    pub fn width(&self, font: FontId, size: f32, text: &str) -> f32 {
        let face = &self.faces[font as usize];
        let scale = size / f32::from(face.units_per_em());
        let advance = |ch: char| {
            face.glyph_index(ch)
                .and_then(|g| face.glyph_hor_advance(g))
                .map(|a| f32::from(a) * scale)
        };
        let fallback = advance('?').unwrap_or(size / 2.0);
        text.chars().map(|ch| advance(ch).unwrap_or(fallback)).sum()
    }

    /// `text` unchanged when it fits `max`, otherwise the longest prefix ending in `…` that does.
    fn fit(&self, font: FontId, size: f32, text: &str, max: f32) -> String {
        if self.width(font, size, text) <= max {
            return text.to_string();
        }
        let chars: Vec<char> = text.chars().collect();
        for n in (0..chars.len()).rev() {
            let mut cut: String = chars[..n].iter().collect::<String>().trim_end().to_string();
            cut.push(ELLIPSIS);
            if self.width(font, size, &cut) <= max {
                return cut;
            }
        }
        String::new()
    }

    /// Greedy word wrap to `max`; a single word wider than `max` is truncated.
    fn wrap(&self, font: FontId, size: f32, text: &str, max: f32) -> Vec<String> {
        let mut lines = Vec::new();
        let mut line = String::new();
        for word in text.split_whitespace() {
            let candidate = if line.is_empty() {
                word.to_string()
            } else {
                format!("{line} {word}")
            };
            if line.is_empty() || self.width(font, size, &candidate) <= max {
                line = candidate;
            } else {
                lines.push(std::mem::replace(&mut line, word.to_string()));
            }
        }
        if !line.is_empty() {
            lines.push(line);
        }
        lines
            .into_iter()
            .map(|l| self.fit(font, size, &l, max))
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Text {
        x: f32,
        y: f32,
        size: f32,
        font: FontId,
        text: String,
        align: Align,
    },
    Line {
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        width: f32,
    },
    /// `gray` runs from 0.0 black to 1.0 white.
    Rect {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        gray: f32,
    },
    /// Organizer logo; `x`, `y` is the bottom-left corner.
    Image {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        png: Vec<u8>,
    },
}

#[derive(Debug, Clone)]
pub struct Layout {
    /// PDF document title, `"<home> vs <away> box score"`.
    pub title: String,
    pub items: Vec<Item>,
    pub step: u8,
    pub body_size: f32,
}

/// Unreachable once step 7 caps rows, but kept so callers handle the `Result`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DoesNotFit {
    pub overflow_pt: f32,
}

/// One rung of the fit ladder; later rungs keep every reduction of earlier ones.
struct Step {
    body: f32,
    legend: bool,
    extra: bool,
    notes: bool,
    short_names: bool,
    cap_rows: bool,
}

const fn step(body: f32, drops: u8) -> Step {
    Step {
        body,
        legend: drops < 1,
        extra: drops < 2,
        notes: drops < 3,
        short_names: drops >= 4,
        cap_rows: drops >= 5,
    }
}

const LADDER: [Step; 8] = [
    step(8.0, 0),
    step(7.5, 0),
    step(7.0, 0),
    step(7.0, 1),
    step(7.0, 2),
    step(7.0, 3),
    step(7.0, 4),
    step(7.0, 5),
];

/// Lays the sheet out on one A4 page, walking the fit ladder until the content height fits
/// above the footer zone. Cell text wider than its column is truncated at every step.
pub fn layout(sheet: &Sheet, metrics: &Metrics) -> Result<Layout, DoesNotFit> {
    let mut overflow_pt = 0.0;
    for (index, rung) in LADDER.iter().enumerate() {
        let reduced = reduce(sheet, rung);
        let mut page = Page {
            m: metrics,
            body: rung.body,
            items: Vec::new(),
            y: TOP,
        };
        page.content(&reduced);
        if page.y >= FOOTER_TOP {
            page.footer(&reduced);
            return Ok(Layout {
                title: format!(
                    "{} vs {} box score",
                    sheet.header.home_name, sheet.header.away_name
                ),
                items: page.items,
                step: index as u8,
                body_size: rung.body,
            });
        }
        overflow_pt = (FOOTER_TOP - page.y).max(0.0);
    }
    Err(DoesNotFit { overflow_pt })
}

fn player_index(columns: &[Column]) -> Option<usize> {
    columns.iter().position(|c| c.key == "player")
}

fn reduce(sheet: &Sheet, rung: &Step) -> Sheet {
    let mut s = sheet.clone();
    if !rung.legend {
        s.legend = None;
    }
    if !rung.extra {
        s.extra = None;
    }
    for team in &mut s.teams {
        if !rung.notes {
            team.notes.clear();
        }
        let player = player_index(&team.columns);
        if let (true, Some(p)) = (rung.short_names, player) {
            for cell in team.rows.iter_mut().filter_map(|row| row.cells.get_mut(p)) {
                if cell.chars().count() > SHORT_NAME_CHARS {
                    let mut short: String = cell.chars().take(SHORT_NAME_CHARS - 1).collect();
                    short = short.trim_end().to_string();
                    short.push(ELLIPSIS);
                    *cell = short;
                }
            }
        }
        if rung.cap_rows && team.rows.len() > MAX_ROWS {
            let hidden = team.rows.len() - MAX_ROWS;
            team.rows.truncate(MAX_ROWS);
            let mut cells = vec![String::new(); team.columns.len().max(1)];
            cells[player.unwrap_or(0)] = format!("+{hidden} not shown");
            team.rows.push(Row {
                cells,
                emphasis: false,
            });
        }
    }
    s
}

/// Column widths scaled from weights; the player column absorbs the rounding remainder.
fn column_widths(columns: &[Column]) -> Vec<f32> {
    let total: f32 = columns.iter().map(|c| f32::from(c.weight)).sum();
    let n = columns.len().max(1) as f32;
    let mut widths: Vec<f32> = columns
        .iter()
        .map(|c| {
            if total > 0.0 {
                USABLE * f32::from(c.weight) / total
            } else {
                USABLE / n
            }
        })
        .collect();
    if let Some(p) = player_index(columns) {
        let others: f32 = widths
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != p)
            .map(|(_, w)| w)
            .sum();
        widths[p] = (USABLE - others).max(0.0);
    }
    widths
}

fn anchor(x: f32, w: f32, align: Align) -> f32 {
    match align {
        Align::Left => x + PAD,
        Align::Right => x + w - PAD,
        Align::Center => x + w / 2.0,
    }
}

/// Baseline that centers a line of `size` text in a band from `top` down `h`.
fn baseline(top: f32, h: f32, size: f32) -> f32 {
    top - (h + size * 0.7) / 2.0
}

struct Page<'a> {
    m: &'a Metrics,
    body: f32,
    items: Vec<Item>,
    /// Bottom of the content drawn so far.
    y: f32,
}

impl Page<'_> {
    fn row_h(&self) -> f32 {
        self.body * 1.35
    }

    #[allow(clippy::too_many_arguments)]
    fn text(
        &mut self,
        x: f32,
        y: f32,
        size: f32,
        font: FontId,
        text: &str,
        max: f32,
        align: Align,
    ) {
        let text = self.m.fit(font, size, text, max);
        if !text.is_empty() {
            self.items.push(Item::Text {
                x,
                y,
                size,
                font,
                text,
                align,
            });
        }
    }

    fn rule(&mut self, y: f32, width: f32) {
        self.items.push(Item::Line {
            x1: MARGIN,
            y1: y,
            x2: PAGE_W - MARGIN,
            y2: y,
            width,
        });
    }

    fn rect(&mut self, top: f32, w: f32, h: f32, gray: f32) {
        self.items.push(Item::Rect {
            x: MARGIN,
            y: top - h,
            w,
            h,
            gray,
        });
    }

    fn content(&mut self, sheet: &Sheet) {
        self.identity(sheet);
        self.header(sheet);
        if sheet.line_score.is_some() {
            self.line_score(sheet);
        }
        for team in &sheet.teams {
            self.team(team);
        }
        if let Some(extra) = &sheet.extra {
            self.y -= GAP;
            let top = self.y;
            self.text(
                MARGIN,
                top - 9.0,
                9.0,
                FontId::BodyBold,
                &extra.title,
                USABLE,
                Align::Left,
            );
            self.y -= 11.0;
            for line in &extra.lines {
                self.paragraph(FontId::Body, self.body, 2.0, line);
            }
        }
        if let Some(legend) = &sheet.legend {
            self.y -= GAP;
            self.paragraph(FontId::Body, LEGEND_SIZE, 1.5, legend);
        }
    }

    /// Wrapped lines of `size` text with `leading` below each.
    fn paragraph(&mut self, font: FontId, size: f32, leading: f32, text: &str) {
        for line in self.m.wrap(font, size, text, USABLE) {
            let y = self.y - size;
            self.text(MARGIN, y, size, font, &line, USABLE, Align::Left);
            self.y -= size + leading;
        }
    }

    fn identity(&mut self, sheet: &Sheet) {
        let id = &sheet.identity;
        let half = USABLE / 2.0 - 4.0;
        let base = TOP - 13.0;
        let mut h = IDENTITY_H;
        if let Some(png) = &id.organizer_logo {
            // Real logo aspect is settled in a later phase; assume 3:1.
            h = LOGO_H + 4.0;
            self.items.push(Item::Image {
                x: MARGIN,
                y: TOP - LOGO_H,
                w: LOGO_H * 3.0,
                h: LOGO_H,
                png: png.clone(),
            });
        } else if let Some(name) = &id.organizer_name {
            self.text(MARGIN, base, 10.0, FontId::Heading, name, half, Align::Left);
        }
        let right = format!("{} \u{b7} {}", id.sport_label, id.descriptor);
        self.text(
            PAGE_W - MARGIN,
            base,
            8.0,
            FontId::Body,
            &right,
            half,
            Align::Right,
        );
        self.y = TOP - h;
        self.rule(self.y, 0.5);
    }

    fn header(&mut self, sheet: &Sheet) {
        let h = &sheet.header;
        self.y -= GAP;
        let top = self.y;
        let right = PAGE_W - MARGIN;
        self.text(
            MARGIN,
            top - 13.0,
            14.0,
            FontId::Heading,
            &h.match_name,
            USABLE * 0.65,
            Align::Left,
        );
        self.text(
            right,
            top - 13.0,
            8.0,
            FontId::Body,
            &h.status,
            USABLE * 0.3,
            Align::Right,
        );
        let mut when = h.date.clone();
        if let Some(venue) = &h.venue {
            when = format!("{when} \u{b7} {venue}");
        }
        self.text(
            MARGIN,
            top - 25.0,
            8.0,
            FontId::Body,
            &when,
            USABLE,
            Align::Left,
        );

        let strip = top - 49.0;
        let cx = PAGE_W / 2.0;
        let score_w = USABLE / 4.0;
        let home_w = self
            .m
            .width(FontId::Display, 26.0, &h.home_score)
            .min(score_w);
        let away_w = self
            .m
            .width(FontId::Display, 26.0, &h.away_score)
            .min(score_w);
        self.text(
            cx - 10.0,
            strip,
            26.0,
            FontId::Display,
            &h.home_score,
            score_w,
            Align::Right,
        );
        self.text(cx, strip, 26.0, FontId::Display, "-", 20.0, Align::Center);
        self.text(
            cx + 10.0,
            strip,
            26.0,
            FontId::Display,
            &h.away_score,
            score_w,
            Align::Left,
        );
        let home_max = (cx - 10.0 - home_w - 8.0 - MARGIN).max(0.0);
        let away_max = (right - (cx + 10.0 + away_w) - 8.0).max(0.0);
        self.text(
            MARGIN,
            strip,
            11.0,
            FontId::BodyBold,
            &h.home_name,
            home_max,
            Align::Left,
        );
        self.text(
            right,
            strip,
            11.0,
            FontId::BodyBold,
            &h.away_name,
            away_max,
            Align::Right,
        );
        self.y = top - HEADER_H;
        self.rule(self.y, 0.5);
    }

    fn line_score(&mut self, sheet: &Sheet) {
        let Some(ls) = &sheet.line_score else {
            return;
        };
        self.y -= GAP;
        let top = self.y;
        self.text(
            MARGIN,
            top - 8.0,
            8.0,
            FontId::BodyBold,
            &ls.title,
            USABLE,
            Align::Left,
        );
        let rh = self.row_h();
        let n = ls.columns.len().max(1);
        let cw = LINE_CELL_W.min((USABLE - LINE_NAME_W) / n as f32);
        let table_w = LINE_NAME_W + cw * n as f32;
        let mut y = top - 10.0;
        self.rect(y, table_w, rh, 0.9);
        let body = self.body;
        for (i, label) in ls.columns.iter().enumerate() {
            let x = MARGIN + LINE_NAME_W + cw * i as f32;
            let b = baseline(y, rh, body);
            self.text(
                anchor(x, cw, Align::Center),
                b,
                body,
                FontId::BodyBold,
                label,
                cw - 2.0 * PAD,
                Align::Center,
            );
        }
        y -= rh;
        for row in &ls.rows {
            let b = baseline(y, rh, body);
            self.text(
                anchor(MARGIN, LINE_NAME_W, Align::Left),
                b,
                body,
                FontId::Body,
                &row.name,
                LINE_NAME_W - 2.0 * PAD,
                Align::Left,
            );
            let values = row.cells.iter().chain(std::iter::once(&row.total));
            for (i, value) in values.enumerate().take(n) {
                let font = if i + 1 == n {
                    FontId::BodyBold
                } else {
                    FontId::Body
                };
                let x = MARGIN + LINE_NAME_W + cw * i as f32;
                self.text(
                    anchor(x, cw, Align::Center),
                    b,
                    body,
                    font,
                    value,
                    cw - 2.0 * PAD,
                    Align::Center,
                );
            }
            y -= rh;
        }
        self.y = y;
    }

    fn team(&mut self, team: &TeamBlock) {
        self.y -= GAP;
        let top = self.y;
        self.rect(top, USABLE, TEAM_BAND_H, 0.85);
        let title = format!("{}  {}", team.side_letter, team.name);
        self.text(
            MARGIN + PAD,
            top - 10.5,
            10.0,
            FontId::BodyBold,
            &title,
            USABLE - 2.0 * PAD,
            Align::Left,
        );
        let rh = self.row_h();
        let widths = column_widths(&team.columns);
        let mut y = top - TEAM_BAND_H;
        self.rect(y, USABLE, rh, 0.9);
        let labels: Vec<String> = team.columns.iter().map(|c| c.label.to_string()).collect();
        self.cells(&team.columns, &widths, y, &labels, FontId::BodyBold);
        y -= rh;
        for row in &team.rows {
            let font = if row.emphasis {
                FontId::BodyBold
            } else {
                FontId::Body
            };
            self.cells(&team.columns, &widths, y, &row.cells, font);
            y -= rh;
        }
        self.items.push(Item::Line {
            x1: MARGIN,
            y1: y,
            x2: PAGE_W - MARGIN,
            y2: y,
            width: 0.5,
        });
        self.cells(
            &team.columns,
            &widths,
            y,
            &team.totals.cells,
            FontId::BodyBold,
        );
        y -= rh;
        self.y = y;
        if !team.notes.is_empty() {
            self.y -= 2.0;
            for note in &team.notes {
                self.paragraph(FontId::Body, self.body, 2.0, note);
            }
        }
    }

    /// One table row of cells from `top` down one row height.
    fn cells(
        &mut self,
        columns: &[Column],
        widths: &[f32],
        top: f32,
        cells: &[String],
        font: FontId,
    ) {
        let rh = self.row_h();
        let body = self.body;
        let b = baseline(top, rh, body);
        let mut x = MARGIN;
        for ((column, w), cell) in columns.iter().zip(widths).zip(cells) {
            self.text(
                anchor(x, *w, column.align),
                b,
                body,
                font,
                cell,
                w - 2.0 * PAD,
                column.align,
            );
            x += w;
        }
    }

    fn footer(&mut self, sheet: &Sheet) {
        self.rule(MARGIN + 10.0, 0.5);
        let center = &sheet.footer.center;
        let center_w = self
            .m
            .width(FontId::Body, FOOTER_SIZE, center)
            .min(USABLE / 3.0);
        self.text(
            PAGE_W / 2.0,
            MARGIN,
            FOOTER_SIZE,
            FontId::Body,
            center,
            USABLE / 3.0,
            Align::Center,
        );
        let right_max = (USABLE / 2.0 - center_w / 2.0 - 8.0).max(0.0);
        self.text(
            PAGE_W - MARGIN,
            MARGIN,
            FOOTER_SIZE,
            FontId::Body,
            &sheet.footer.right,
            right_max,
            Align::Right,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boxscore::{SheetResult, build, fixtures};
    use crate::storage::SavedMatch;

    fn sheet(fixture: fn() -> SavedMatch) -> Sheet {
        let SheetResult::Ready(sheet) = build(&fixture(), None, "2026-10-24", "0.1.0") else {
            panic!("expected a ready sheet");
        };
        sheet
    }

    fn texts(l: &Layout) -> impl Iterator<Item = (&str, f32, f32, FontId)> {
        l.items.iter().filter_map(|i| match i {
            Item::Text {
                text,
                y,
                size,
                font,
                ..
            } => Some((text.as_str(), *y, *size, *font)),
            _ => None,
        })
    }

    #[test]
    fn metrics_measure_bundled_fonts() {
        let m = Metrics::bundled();
        assert!(m.width(FontId::Body, 8.0, "HustlX") > 20.0);
        assert!(m.width(FontId::Body, 8.0, "") == 0.0);
    }

    #[test]
    fn header_labels_and_totals_never_truncate_for_fixtures() {
        let m = Metrics::bundled();
        for fixture in [fixtures::basketball_full, fixtures::basketball_oversized] {
            let l = layout(&sheet(fixture), &m).unwrap();
            // Header rows carry the "PTS" label; totals rows carry "Totals".
            let rows: Vec<f32> = texts(&l)
                .filter(|(t, _, size, font)| {
                    *size == l.body_size
                        && *font == FontId::BodyBold
                        && (*t == "PTS" || *t == "Totals")
                })
                .map(|(_, y, ..)| y)
                .collect();
            assert_eq!(rows.len(), 4, "two header rows and two totals rows");
            for (text, y, size, font) in texts(&l) {
                if size == l.body_size && font == FontId::BodyBold && rows.contains(&y) {
                    assert!(!text.ends_with(ELLIPSIS), "truncated: {text}");
                }
            }
        }
    }

    #[test]
    fn full_fixture_fits_at_step_zero() {
        let m = Metrics::bundled();
        let l = layout(&sheet(fixtures::basketball_full), &m).unwrap();
        assert_eq!(l.step, 0);
        assert_eq!(l.body_size, 8.0);
    }

    fn assert_inside_the_page(l: &Layout) {
        let inside_x = |x: f32| (0.0..=PAGE_W).contains(&x);
        let inside_y = |y: f32| (MARGIN..=PAGE_H - MARGIN).contains(&y);
        for item in &l.items {
            let ok = match item {
                Item::Text { x, y, .. } => inside_x(*x) && inside_y(*y),
                Item::Line { x1, y1, x2, y2, .. } => {
                    inside_x(*x1) && inside_x(*x2) && inside_y(*y1) && inside_y(*y2)
                }
                Item::Rect { x, y, w, h, .. } | Item::Image { x, y, w, h, .. } => {
                    inside_x(*x) && inside_x(x + w) && inside_y(*y) && inside_y(y + h)
                }
            };
            assert!(ok, "outside the page: {item:?}");
        }
    }

    #[test]
    fn oversized_fixture_fits_within_the_ladder_and_truncates_names() {
        let m = Metrics::bundled();
        let s = sheet(fixtures::basketball_oversized);
        let l = layout(&s, &m).unwrap();
        assert_eq!(l.step, 0);
        assert!(
            texts(&l).any(|(t, ..)| t.starts_with("Home Player") && t.ends_with('…')),
            "a long player name is truncated"
        );
        // Table cell text sits at its column anchor and never exceeds the column.
        let usable = PAGE_W - 2.0 * MARGIN;
        let mut columns = Vec::new();
        for team in &s.teams {
            let mut x = MARGIN;
            for (column, w) in team.columns.iter().zip(column_widths(&team.columns)) {
                columns.push((anchor(x, w, column.align), column.align, w));
                x += w;
            }
        }
        for item in &l.items {
            let Item::Text {
                x,
                size,
                font,
                text,
                align,
                ..
            } = item
            else {
                continue;
            };
            let width = m.width(*font, *size, text);
            assert!(width <= usable, "too wide: {text}");
            if *size == l.body_size {
                for (ax, calign, w) in &columns {
                    if (ax - x).abs() < 0.01 && calign == align {
                        assert!(width <= w - 2.0 * PAD, "wider than its column: {text}");
                    }
                }
            }
        }
        // The score strip has no room for the 60 character home name.
        assert!(texts(&l).any(|(t, ..)| t.starts_with("HHHH") && t.ends_with('…')));
        assert_inside_the_page(&l);
    }

    #[test]
    fn sixty_row_teams_walk_the_ladder_to_step_7_and_cap_rows() {
        let m = Metrics::bundled();
        let mut s = sheet(fixtures::basketball_full);
        for team in &mut s.teams {
            let base = team.rows.clone();
            team.rows = base.iter().cycle().take(60).cloned().collect();
        }
        let l = layout(&s, &m).unwrap();
        assert_eq!(l.step, 7);
        assert!(texts(&l).any(|(t, ..)| t == "+42 not shown"));
        // Everything but the footer (text at MARGIN, rule at MARGIN + 10) ends above its zone.
        for item in &l.items {
            let bottom = match item {
                Item::Text { y, .. } if *y == MARGIN => continue,
                Item::Line { y1, .. } if *y1 == MARGIN + 10.0 => continue,
                Item::Text { y, .. } => *y,
                Item::Line { y1, y2, .. } => y1.min(*y2),
                Item::Rect { y, .. } | Item::Image { y, .. } => *y,
            };
            assert!(bottom >= MARGIN + 14.0, "inside the footer zone: {item:?}");
        }
        assert_inside_the_page(&l);
    }

    #[test]
    fn every_item_lies_inside_the_page() {
        let m = Metrics::bundled();
        for fixture in [
            fixtures::basketball_full as fn() -> SavedMatch,
            fixtures::basketball_unstamped,
            fixtures::basketball_oversized,
        ] {
            let l = layout(&sheet(fixture), &m).unwrap();
            let inside_x = |x: f32| (0.0..=PAGE_W).contains(&x);
            let inside_y = |y: f32| (MARGIN..=PAGE_H - MARGIN).contains(&y);
            for item in &l.items {
                let ok = match item {
                    Item::Text { x, y, .. } => inside_x(*x) && inside_y(*y),
                    Item::Line { x1, y1, x2, y2, .. } => {
                        inside_x(*x1) && inside_x(*x2) && inside_y(*y1) && inside_y(*y2)
                    }
                    Item::Rect { x, y, w, h, .. } | Item::Image { x, y, w, h, .. } => {
                        inside_x(*x) && inside_x(x + w) && inside_y(*y) && inside_y(y + h)
                    }
                };
                assert!(ok, "outside the page: {item:?}");
            }
        }
    }

    #[test]
    fn layout_is_deterministic() {
        let m = Metrics::bundled();
        for fixture in [
            fixtures::basketball_full as fn() -> SavedMatch,
            fixtures::basketball_oversized,
        ] {
            let s = sheet(fixture);
            assert_eq!(
                format!("{:?}", layout(&s, &m).unwrap().items),
                format!("{:?}", layout(&s, &m).unwrap().items)
            );
        }
    }

    #[test]
    fn footer_text_is_powered_by_hustlx_and_nothing_above_mentions_hustlx() {
        let m = Metrics::bundled();
        let l = layout(&sheet(fixtures::basketball_full), &m).unwrap();
        let mentions: Vec<_> = texts(&l).filter(|(t, ..)| t.contains("HustlX")).collect();
        let generated: Vec<_> = mentions
            .iter()
            .filter(|(t, ..)| t.starts_with("Generated"))
            .collect();
        let others: Vec<_> = mentions
            .iter()
            .filter(|(t, ..)| !t.starts_with("Generated"))
            .collect();
        assert_eq!(generated.len(), 1);
        assert_eq!(others.len(), 1);
        assert_eq!(others[0].0, "powered by HustlX");
        assert!(mentions.iter().all(|(_, y, ..)| *y < 40.0));
    }

    #[test]
    fn unstamped_fixture_fits_without_a_line_score() {
        let m = Metrics::bundled();
        let s = sheet(fixtures::basketball_unstamped);
        assert!(s.line_score.is_none());
        assert_eq!(layout(&s, &m).unwrap().step, 0);
    }

    #[test]
    fn step_seven_caps_rows_and_reports_the_rest() {
        let m = Metrics::bundled();
        let mut s = sheet(fixtures::basketball_oversized);
        for team in &mut s.teams {
            let extra = team.rows.clone();
            team.rows.extend(extra);
        }
        let l = layout(&s, &m).unwrap();
        assert_eq!(l.step, 7);
        assert!(texts(&l).any(|(t, ..)| t == "+22 not shown"));
    }
}
