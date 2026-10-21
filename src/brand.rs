//! Offline HustlX identity, aligned with the web theme and native font assets.
use iced::widget::{button, pick_list, svg};
use iced::{
    Background, Border, Color, Font, Theme,
    theme::{
        Palette,
        palette::{Extended, Pair},
    },
};
use std::sync::LazyLock;

pub const BODY: Font = Font::with_name("Inter");
pub const BOLD: Font = Font {
    weight: iced::font::Weight::Bold,
    ..BODY
};
pub const SEMIBOLD: Font = Font {
    weight: iced::font::Weight::Semibold,
    ..BODY
};
pub const HEADING: Font = Font {
    weight: iced::font::Weight::Bold,
    ..Font::with_name("Exo")
};
// The native static Black face declares "Exo Black" as its internal family.
pub const DISPLAY: Font = Font {
    weight: iced::font::Weight::Black,
    ..Font::with_name("Exo Black")
};

#[derive(Clone, Copy)]
pub struct Colors {
    pub background: Color,
    pub panel: Color,
    pub row: Color,
    pub border: Color,
    pub text: Color,
    pub muted: Color,
    pub home: Color,
    pub away: Color,
    pub green: Color,
    pub red: Color,
}
impl Colors {
    pub fn new(dark: bool) -> Self {
        if dark {
            Self {
                background: iced::color!(0x09090b),
                panel: iced::color!(0x141416),
                row: iced::color!(0x1c1c1f),
                border: iced::color!(0x27272a),
                text: iced::color!(0xfafafa),
                muted: iced::color!(0xa1a1aa),
                home: iced::color!(0xa5b4fc),
                away: iced::color!(0xffd700),
                green: iced::color!(0x4ade80),
                red: iced::color!(0xf87171),
            }
        } else {
            Self {
                background: iced::color!(0xf8f9fb),
                panel: Color::WHITE,
                row: iced::color!(0xf1f5f9),
                border: iced::color!(0xe2e8f0),
                text: iced::color!(0x0f172a),
                muted: iced::color!(0x64748b),
                home: iced::color!(0x313a68),
                away: iced::color!(0x7a5500),
                green: iced::color!(0x15803d),
                red: iced::color!(0xb91c1c),
            }
        }
    }
}
fn make_theme(dark: bool) -> Theme {
    let c = Colors::new(dark);
    Theme::custom_with_fn(
        if dark { "HustlX Dark" } else { "HustlX Light" },
        Palette {
            background: c.background,
            text: c.text,
            primary: if dark {
                iced::color!(0x4f46e5)
            } else {
                iced::color!(0x313a68)
            },
            success: c.green,
            warning: c.away,
            danger: c.red,
        },
        move |palette| {
            let mut extended = Extended::generate(palette);
            // Filled controls use the accessible 600 shade in dark mode.
            extended.primary.base = Pair {
                color: palette.primary,
                text: Color::WHITE,
            };
            extended.primary.strong = Pair {
                color: if dark {
                    iced::color!(0x4338ca)
                } else {
                    iced::color!(0x454d74)
                },
                text: Color::WHITE,
            };
            extended
        },
    )
}
pub fn theme(dark: bool) -> Theme {
    static DARK: LazyLock<Theme> = LazyLock::new(|| make_theme(true));
    static LIGHT: LazyLock<Theme> = LazyLock::new(|| make_theme(false));
    if dark { DARK.clone() } else { LIGHT.clone() }
}
pub fn wordmark(dark: bool) -> svg::Handle {
    static WHITE: LazyLock<svg::Handle> = LazyLock::new(|| {
        svg::Handle::from_memory(
            include_bytes!("../assets/brand/hustlx-wordmark-white.svg").as_slice(),
        )
    });
    static NAVY: LazyLock<svg::Handle> = LazyLock::new(|| {
        svg::Handle::from_memory(
            include_bytes!("../assets/brand/hustlx-wordmark-navy.svg").as_slice(),
        )
    });
    if dark { WHITE.clone() } else { NAVY.clone() }
}

pub fn secondary(theme: &Theme, status: button::Status) -> button::Style {
    let c = Colors::new(theme.extended_palette().is_dark);
    let mut style = button::Style {
        background: Some(Background::Color(if status == button::Status::Hovered {
            c.row
        } else {
            c.panel
        })),
        text_color: c.text,
        border: Border {
            color: c.border,
            width: 1.0,
            radius: 6.0.into(),
        },
        ..button::Style::default()
    };
    if status == button::Status::Disabled {
        style.text_color = c.muted;
    }
    style
}

/// Neutral library selects retain a clear border without looking disabled.
pub fn select(theme: &Theme, status: pick_list::Status) -> pick_list::Style {
    let c = Colors::new(theme.extended_palette().is_dark);
    pick_list::Style {
        text_color: c.text,
        placeholder_color: c.muted,
        handle_color: c.muted,
        background: c.panel.into(),
        border: Border {
            color: if matches!(status, pick_list::Status::Active) {
                c.border
            } else {
                c.home
            },
            width: 1.0,
            radius: 6.0.into(),
        },
    }
}
