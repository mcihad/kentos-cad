//! The launcher's own looks, from KentOS UI's tokens (docs/adr/0800 §3): the
//! page and its panels, the cards and the one under the pointer, the tiles,
//! the picture's frame, the navigation, the outlined button. Sizes are KentOS
//! UI's (`theme::metrics`, `theme::shape`) or designed at the default text
//! size and grown with it ([`px`]); every one a whole pixel.

use iced::widget::{button, container};
use iced::{Background, Border, Color, Shadow, Theme, Vector, gradient};
use kentos_ui::theme::{Tokens, shape, typography};

use crate::tones::Paint;

/// A size designed at the default text size (13 px), at the current one: whole pixels.
pub fn px(size: f32) -> f32 {
    typography::from_default(size)
}

/// `inner` a pixel smaller where needed, so that centred in `outer` its
/// edges fall on whole pixels (both whole, their difference even).
pub fn centred_in(outer: f32, inner: f32) -> f32 {
    if (outer - inner).rem_euclid(2.0) == 1.0 {
        inner - 1.0
    } else {
        inner
    }
}

/// `text` cut with “…” to fit `room` pixels on one line, measured as it is
/// drawn (shaped, in its own weight), so a semibold name is not cut by a
/// regular one's width.
pub fn fit(text: &str, size: f32, room: f32, strong: bool) -> std::borrow::Cow<'_, str> {
    let width = |t: &str| typography::measured_width(t, size, strong);
    if width(text) <= room {
        return std::borrow::Cow::Borrowed(text);
    }
    let chars: Vec<char> = text.chars().collect();
    let cut = |n: usize| {
        let kept: String = chars[..n].iter().collect();
        format!("{}…", kept.trim_end())
    };
    // The longest start that fits with its ellipsis.
    let (mut fits, mut too_long) = (0, chars.len());
    while too_long - fits > 1 {
        let mid = (fits + too_long) / 2;
        if width(&cut(mid)) <= room {
            fits = mid;
        } else {
            too_long = mid;
        }
    }
    std::borrow::Cow::Owned(cut(fits))
}

/// `outer` a pixel larger where needed, for the same.
pub fn around(inner: f32, outer: f32) -> f32 {
    if (outer - inner).rem_euclid(2.0) == 1.0 {
        outer + 1.0
    } else {
        outer
    }
}

/// The navigation's width.
pub const SIDEBAR: f32 = 224.0;
/// The status bar's height.
pub const STATUS: f32 = 30.0;
/// The page's padding, across and down.
pub const PAGE_X: f32 = 24.0;
pub const PAGE_Y: f32 = 20.0;
/// Between the grid's cards, and between the page's parts.
pub const GAP: f32 = 16.0;
/// A card's padding and the space between its parts.
pub const CARD_PAD: f32 = 12.0;
pub const CARD_SPACING: f32 = 12.0;
/// A card is at least this wide: the grid has as many columns as fit (two to four).
pub const CARD_LEAST: f32 = 320.0;
/// The tile beside a card's name, and its version line.
pub const TILE: f32 = 48.0;
pub const META: f32 = 18.0;
/// The summary strip's height and a navigation item's.
pub const STRIP: f32 = 44.0;
pub const NAV_ITEM: f32 = 40.0;
/// The scroll bar and its space beside the grid (KentOS UI's body scroll bar).
pub const SCROLLBAR: f32 = 14.0;

/// What the panels stand on: the window's colour.
pub fn page(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Tokens::of(theme).window)),
        text_color: Some(Tokens::of(theme).text),
        ..container::Style::default()
    }
}

/// A panel's colour: the surface in the dark, paper white in the light.
pub fn panel_color(t: &Tokens) -> Color {
    if t.is_dark { t.surface } else { t.field }
}

/// The navigation and the status bar.
pub fn sidebar(theme: &Theme) -> container::Style {
    let t = Tokens::of(theme);
    container::Style {
        background: Some(Background::Color(panel_color(&t))),
        ..container::Style::default()
    }
}

/// A card, the summary strip: the panel's colour, a hairline edge, the
/// card's corner. The card to start (`hot`) has the accent's edge and a
/// faint glow of it.
pub fn card(hot: bool) -> impl Fn(&Theme) -> container::Style {
    move |theme| {
        let t = Tokens::of(theme);
        container::Style {
            background: Some(Background::Color(panel_color(&t))),
            border: Border {
                color: if hot { t.accent } else { t.border },
                width: 1.0,
                radius: shape::lg().into(),
            },
            shadow: if hot {
                Shadow {
                    color: t.accent.scale_alpha(if t.is_dark { 0.32 } else { 0.22 }),
                    offset: Vector::ZERO,
                    blur_radius: 14.0,
                }
            } else {
                Shadow::default()
            },
            text_color: Some(t.text),
            ..container::Style::default()
        }
    }
}

/// A box inside a card (the update centre's lists): sunk into it.
pub fn inset(theme: &Theme) -> container::Style {
    let t = Tokens::of(theme);
    container::Style {
        background: Some(Background::Color(t.well())),
        border: Border {
            color: t.border,
            width: 1.0,
            radius: shape::md().into(),
        },
        ..container::Style::default()
    }
}

/// The edge drawn over a card's picture.
pub fn picture_edge(theme: &Theme) -> container::Style {
    let t = Tokens::of(theme);
    container::Style {
        border: Border {
            color: t.border,
            width: 1.0,
            radius: shape::md().into(),
        },
        ..container::Style::default()
    }
}

/// A picture's place when the app has none: its tone, faint.
pub fn placeholder(paint: Paint) -> impl Fn(&Theme) -> container::Style {
    move |theme| {
        let t = Tokens::of(theme);
        container::Style {
            background: Some(Background::Gradient(
                gradient::Linear::new(std::f32::consts::PI)
                    .add_stop(
                        0.0,
                        paint.top.scale_alpha(if t.is_dark { 0.22 } else { 0.18 }),
                    )
                    .add_stop(
                        1.0,
                        paint
                            .bottom
                            .scale_alpha(if t.is_dark { 0.08 } else { 0.06 }),
                    )
                    .into(),
            )),
            border: Border {
                color: t.border,
                width: 1.0,
                radius: shape::md().into(),
            },
            ..container::Style::default()
        }
    }
}

/// A tile: its tone from a light top, rounded about a quarter of its side.
pub fn tile(paint: Paint, side: f32) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: Some(Background::Gradient(
            gradient::Linear::new(std::f32::consts::PI)
                .add_stop(0.0, paint.top)
                .add_stop(1.0, paint.bottom)
                .into(),
        )),
        border: Border {
            color: Color::WHITE.scale_alpha(0.12),
            width: 1.0,
            radius: tile_radius(side).into(),
        },
        text_color: Some(paint.ink),
        ..container::Style::default()
    }
}

/// A tile's corner: about a quarter of its side, whole pixels.
pub fn tile_radius(side: f32) -> f32 {
    (side * 0.24).round()
}

/// A dot of a colour (a card's state, the bell's unread mark).
pub fn dot(color: Color) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: Some(Background::Color(color)),
        border: Border {
            radius: 999.0.into(),
            ..Border::default()
        },
        ..container::Style::default()
    }
}

/// The bell's dot: the accent, ringed in the page's colour so it stands off the bell.
pub fn bell_dot(theme: &Theme) -> container::Style {
    let t = Tokens::of(theme);
    container::Style {
        background: Some(Background::Color(t.accent)),
        border: Border {
            color: t.window,
            width: 2.0,
            radius: 999.0.into(),
        },
        ..container::Style::default()
    }
}

/// A navigation item: the selected one on the accent's soft ground; under
/// the pointer a faint layer; the words muted until then.
pub fn nav(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme, status| {
        let t = Tokens::of(theme);
        let (background, text) = match (selected, status) {
            (true, _) => (t.selection(), t.text),
            (false, button::Status::Hovered) => (t.layer(0.05), t.text),
            (false, button::Status::Pressed) => (t.layer(0.09), t.text),
            (false, _) => (Color::TRANSPARENT, t.muted),
        };
        button::Style {
            background: Some(Background::Color(background)),
            text_color: text,
            border: Border {
                radius: shape::md().into(),
                ..Border::default()
            },
            ..button::Style::default()
        }
    }
}

/// The selected item's bar at its left edge.
pub fn nav_bar(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Tokens::of(theme).accent)),
        border: Border {
            radius: 2.0.into(),
            ..Border::default()
        },
        ..container::Style::default()
    }
}

/// A card's button when it is not the card to start: the accent's line,
/// the text's colour; under the pointer the accent's soft ground.
pub fn outline(theme: &Theme, status: button::Status) -> button::Style {
    let t = Tokens::of(theme);
    let (background, edge, text) = match status {
        button::Status::Active => (Color::TRANSPARENT, t.accent_line(), t.text),
        button::Status::Hovered => (t.selection(), t.accent, t.text),
        button::Status::Pressed => (t.accent.scale_alpha(0.3), t.accent, t.text),
        button::Status::Disabled => (t.layer(0.03), t.border, t.disabled()),
    };
    button::Style {
        background: Some(Background::Color(background)),
        text_color: text,
        border: Border {
            color: edge,
            width: 1.0,
            radius: shape::sm().into(),
        },
        ..button::Style::default()
    }
}

/// A button that says something it cannot do yet (Yakında): no edge of the accent.
pub fn idle(theme: &Theme, _status: button::Status) -> button::Style {
    let t = Tokens::of(theme);
    button::Style {
        background: Some(Background::Color(t.layer(0.04))),
        text_color: t.disabled(),
        border: Border {
            color: t.border,
            width: 1.0,
            radius: shape::sm().into(),
        },
        ..button::Style::default()
    }
}

/// A link-like button in the accent's text (Uygulama ekle, İndirmeleri görüntüle).
pub fn link(theme: &Theme, status: button::Status) -> button::Style {
    let t = Tokens::of(theme);
    let (background, text) = match status {
        button::Status::Active => (Color::TRANSPARENT, t.accent_hover),
        button::Status::Hovered => (t.selection(), t.accent_hover),
        button::Status::Pressed => (t.accent.scale_alpha(0.3), t.accent_hover),
        button::Status::Disabled => (Color::TRANSPARENT, t.disabled()),
    };
    let text = match status {
        button::Status::Disabled => text,
        // The accent's own tone reads better on paper than its lighter one.
        _ if !t.is_dark => t.accent,
        _ => text,
    };
    button::Style {
        background: Some(Background::Color(background)),
        text_color: text,
        border: Border {
            radius: shape::sm().into(),
            ..Border::default()
        },
        ..button::Style::default()
    }
}

/// A swatch of the add dialog's tones and glyphs: chosen, the accent's ring.
pub fn choice(chosen: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme, status| {
        let t = Tokens::of(theme);
        let edge = match (chosen, status) {
            (true, _) => t.accent,
            (false, button::Status::Hovered) => t.border_strong(),
            (false, _) => Color::TRANSPARENT,
        };
        button::Style {
            background: Some(Background::Color(if chosen {
                t.selection()
            } else {
                Color::TRANSPARENT
            })),
            text_color: t.text,
            border: Border {
                color: edge,
                width: 2.0,
                radius: shape::md().into(),
            },
            ..button::Style::default()
        }
    }
}

/// The status bar's top hairline is a rule; its ground.
pub fn status_bar(theme: &Theme) -> container::Style {
    let t = Tokens::of(theme);
    container::Style {
        background: Some(Background::Color(panel_color(&t))),
        text_color: Some(t.muted),
        ..container::Style::default()
    }
}
