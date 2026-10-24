//! Draws a box score `Layout` into a one-page A4 PDF. This is the only module that knows
//! `printpdf`; everything else works with `Layout` items.

use super::Align;
use super::layout::{FontId, Item, Layout, Metrics, PAGE_H, PAGE_W};
use printpdf::{
    Color, DateTime, Line, LinePoint, Op, PaintMode, ParsedFont, PdfDocument, PdfPage,
    PdfSaveOptions, Point, Polygon, PolygonRing, Pt, RawImage, Rect, Rgb, TextItem, WindingOrder,
    XObjectId, XObjectTransform,
};

/// Near-black used for text and rules.
const INK: f32 = 0.1;

/// Fixed document dates (2026-01-01T00:00:00Z) so the same layout renders to the same bytes.
const FIXED_TIMESTAMP: i64 = 1_767_225_600;

const ALL_FONTS: [FontId; 4] = [
    FontId::Body,
    FontId::BodyBold,
    FontId::Heading,
    FontId::Display,
];

/// Renders `layout` to exactly one A4 page, embedding only the fonts its text items use.
pub fn render(layout: &Layout) -> Vec<u8> {
    let metrics = Metrics::bundled();
    let mut warnings = Vec::new();
    let mut doc = PdfDocument::new(&layout.title);
    let fixed = DateTime::from_unix_timestamp(FIXED_TIMESTAMP).unwrap_or(DateTime::epoch());
    doc.metadata.info.creation_date = fixed;
    doc.metadata.info.modification_date = fixed;
    doc.metadata.info.metadata_date = fixed;

    // Fonts are registered under fixed resource names instead of `add_font`, whose ids are
    // random, so the output stays deterministic.
    let mut fonts: [Option<printpdf::FontId>; 4] = [None, None, None, None];
    for font in ALL_FONTS {
        let used = layout
            .items
            .iter()
            .any(|i| matches!(i, Item::Text { font: f, .. } if *f == font));
        if !used {
            continue;
        }
        let Some(parsed) = ParsedFont::from_bytes(Metrics::bytes(font), 0, &mut warnings) else {
            continue;
        };
        let id = printpdf::FontId(format!("F{}", font as usize));
        doc.resources.fonts.map.insert(id.clone(), parsed);
        fonts[font as usize] = Some(id);
    }

    let mut ops = Vec::new();
    let mut images = 0usize;
    for item in &layout.items {
        match item {
            Item::Text {
                x,
                y,
                size,
                font,
                text,
                align,
            } => {
                let Some(id) = fonts[*font as usize].clone() else {
                    continue;
                };
                // `x` is an anchor: left edge, right edge, or center depending on `align`.
                let left = match align {
                    Align::Left => *x,
                    Align::Right => x - metrics.width(*font, *size, text),
                    Align::Center => x - metrics.width(*font, *size, text) / 2.0,
                };
                ops.extend([
                    Op::StartTextSection,
                    Op::SetFillColor { col: gray(INK) },
                    Op::SetFontSize {
                        size: Pt(*size),
                        font: id.clone(),
                    },
                    Op::SetTextCursor {
                        pos: point(left, *y),
                    },
                    Op::WriteText {
                        items: vec![TextItem::Text(text.clone())],
                        font: id,
                    },
                    Op::EndTextSection,
                ]);
            }
            Item::Line {
                x1,
                y1,
                x2,
                y2,
                width,
            } => ops.extend([
                Op::SetOutlineColor { col: gray(INK) },
                Op::SetOutlineThickness { pt: Pt(*width) },
                Op::DrawLine {
                    line: Line {
                        points: vec![corner(*x1, *y1), corner(*x2, *y2)],
                        is_closed: false,
                    },
                },
            ]),
            Item::Rect {
                x,
                y,
                w,
                h,
                gray: g,
            } => ops.extend([
                Op::SetFillColor { col: gray(*g) },
                Op::DrawPolygon {
                    polygon: Polygon {
                        rings: vec![PolygonRing {
                            points: vec![
                                corner(*x, *y),
                                corner(x + w, *y),
                                corner(x + w, y + h),
                                corner(*x, y + h),
                            ],
                        }],
                        mode: PaintMode::Fill,
                        winding_order: WindingOrder::NonZero,
                    },
                },
            ]),
            Item::Image { x, y, w, h, png } => {
                // A logo that does not decode is skipped so the sheet still renders.
                let Ok(image) = RawImage::decode_from_bytes(png, &mut warnings) else {
                    continue;
                };
                if image.width == 0 || image.height == 0 {
                    continue;
                }
                let (px_w, px_h) = (image.width as f32, image.height as f32);
                let id = XObjectId(format!("Im{images}"));
                images += 1;
                doc.resources
                    .xobjects
                    .map
                    .insert(id.clone(), printpdf::XObject::Image(image));
                ops.push(Op::UseXobject {
                    id,
                    // At 72 dpi one pixel is one point, so the scale maps pixels to `w` by `h`.
                    transform: XObjectTransform {
                        translate_x: Some(Pt(*x)),
                        translate_y: Some(Pt(*y)),
                        rotate: None,
                        scale_x: Some(w / px_w),
                        scale_y: Some(h / px_h),
                        dpi: Some(72.0),
                    },
                });
            }
        }
    }

    let page_box = Rect::from_wh(Pt(PAGE_W), Pt(PAGE_H));
    doc.with_pages(vec![PdfPage {
        media_box: page_box.clone(),
        trim_box: page_box.clone(),
        crop_box: page_box,
        ops,
    }]);

    // printpdf 0.8.2 `PdfSaveOptions`: `subset_fonts: false` asks for whole fonts (0.8.2 ignores
    // it and always subsets the used glyphs), `optimize: false` skips stream compression. The
    // crate always writes a classic cross-reference table and never uses object streams, so
    // `page_count` can scan the bytes.
    let opts = PdfSaveOptions {
        optimize: false,
        subset_fonts: false,
        secure: true,
        image_optimization: None,
    };
    let mut bytes = doc.save(&opts, &mut warnings);
    fix_trailer_id(&mut bytes);
    bytes
}

/// Counts `/Type /Page` (with or without the space) not followed by `s`.
pub fn page_count(pdf: &[u8]) -> usize {
    let mut count = 0;
    for needle in [&b"/Type /Page"[..], &b"/Type/Page"[..]] {
        count += pdf
            .windows(needle.len())
            .enumerate()
            .filter(|(i, w)| *w == needle && pdf.get(i + needle.len()) != Some(&b's'))
            .count();
    }
    count
}

/// printpdf fills the trailer `/ID` pair with two 32-letter strings from a global counter, so
/// every save differs. Both are overwritten in place, at the same length so the cross-reference
/// offsets stay valid, with letters derived from a hash of the document body.
fn fix_trailer_id(bytes: &mut [u8]) {
    let Some(trailer) = find_last(bytes, b"trailer") else {
        return;
    };
    let Some(id) = find_last(&bytes[trailer..], b"/ID").map(|i| trailer + i) else {
        return;
    };
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for b in &bytes[..trailer] {
        hash = (hash ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3);
    }
    let mut at = id;
    for _ in 0..2 {
        let Some(open) = bytes[at..].iter().position(|b| *b == b'(').map(|i| at + i) else {
            return;
        };
        let Some(close) = bytes[open..]
            .iter()
            .position(|b| *b == b')')
            .map(|i| open + i)
        else {
            return;
        };
        for slot in &mut bytes[open + 1..close] {
            hash = hash.wrapping_mul(0x5851_f42d_4c95_7f2d).wrapping_add(1);
            *slot = b'A' + (hash >> 60) as u8;
        }
        at = close + 1;
    }
}

fn find_last(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).rposition(|w| w == needle)
}

fn gray(level: f32) -> Color {
    let v = level.clamp(0.0, 1.0);
    Color::Rgb(Rgb::new(v, v, v, None))
}

fn point(x: f32, y: f32) -> Point {
    Point { x: Pt(x), y: Pt(y) }
}

fn corner(x: f32, y: f32) -> LinePoint {
    LinePoint {
        p: point(x, y),
        bezier: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boxscore::layout::layout;
    use crate::boxscore::{Sheet, SheetResult, build, fixtures};
    use crate::storage::SavedMatch;

    fn sheet(fixture: fn() -> SavedMatch) -> Sheet {
        let SheetResult::Ready(sheet) = build(&fixture(), None, "2026-10-24", "0.1.0") else {
            panic!("expected a ready sheet");
        };
        sheet
    }

    fn laid_out(fixture: fn() -> SavedMatch) -> Layout {
        layout(&sheet(fixture), &Metrics::bundled()).unwrap()
    }

    fn occurrences(haystack: &[u8], needle: &[u8]) -> usize {
        haystack
            .windows(needle.len())
            .filter(|w| *w == needle)
            .count()
    }

    #[test]
    fn renders_single_page_pdf_for_full_fixture() {
        let bytes = render(&laid_out(fixtures::basketball_full));
        assert!(bytes.starts_with(b"%PDF-"));
        assert_eq!(page_count(&bytes), 1);
        assert!(bytes.len() > 10_000);
    }

    #[test]
    fn renders_single_page_for_oversized_fixture() {
        assert_eq!(
            page_count(&render(&laid_out(fixtures::basketball_oversized))),
            1
        );
    }

    #[test]
    fn embeds_each_used_font_once() {
        let l = laid_out(fixtures::basketball_full);
        let mut used: Vec<FontId> = Vec::new();
        for item in &l.items {
            if let Item::Text { font, .. } = item
                && !used.contains(font)
            {
                used.push(*font);
            }
        }
        let bytes = render(&l);
        assert_eq!(occurrences(&bytes, b"/FontFile2"), used.len());
    }

    #[test]
    fn output_is_deterministic_for_same_layout() {
        let l = laid_out(fixtures::basketball_full);
        assert_eq!(render(&l), render(&l));
    }
}
