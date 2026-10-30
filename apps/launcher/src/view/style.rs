//! The launcher's own looks, from KentOS UI's tokens (docs/adr/0803): the
//! table and the sheet that lies on it, the plates and the one under the
//! pointer, the tiles, the navigation, the quiet button and the hairlines.
//! Sizes are KentOS UI's (`theme::metrics`, `theme::shape`) or designed at the
//! default text size and grown with it ([`px`]); every one a whole pixel.

use iced::widget::{button, container, scrollable};
use iced::{Background, Border, Color, Theme, gradient};
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
pub const SIDEBAR: f32 = 216.0;
/// The status line's height.
pub const STATUS: f32 = 28.0;
/// Between the sheet and the window's top and right edges.
pub const MARGIN: f32 = 8.0;
/// The sheet's padding: across, above and below.
pub const PAGE_X: f32 = 24.0;
pub const PAGE_TOP: f32 = 22.0;
pub const PAGE_BOTTOM: f32 = 14.0;
/// The widest the page's content grows: past it the plates would blur and the eye would travel.
pub const PAGE_MOST: f32 = 1480.0;
/// Between a plate's ground and what is on it, and what lines up with it
/// (the header, the sections' heads, the hairlines).
pub const INSET: f32 = 8.0;
/// Between two plates' grounds: with their insets the same as the sheet's padding.
pub const GAP: f32 = 8.0;
/// Between the page's sections, and between the header and the first of them.
pub const SECTION: f32 = 24.0;
/// A plate is comfortable from this wide, and fits its caption from this wide.
pub const PLATE_COMFORTABLE: f32 = 300.0;
pub const PLATE_LEAST: f32 = 220.0;
/// The tile beside a plate's name; a table row's height and its tile; a section's head.
pub const TILE: f32 = 40.0;
pub const ROW: f32 = 40.0;
pub const ROW_TILE: f32 = 24.0;
pub const HEAD: f32 = 28.0;
/// A coming app's thumbnail.
pub const THUMB_WIDTH: f32 = 128.0;
pub const THUMB_HEIGHT: f32 = 72.0;
/// A navigation item's height.
pub const NAV_ITEM: f32 = 34.0;
/// The scroll bar and its space beside the content (KentOS UI's body scroll bar).
pub const SCROLLBAR: f32 = 14.0;

fn blend(color: Color, other: Color, amount: f32) -> Color {
    Color {
        r: color.r + (other.r - color.r) * amount,
        g: color.g + (other.g - color.g) * amount,
        b: color.b + (other.b - color.b) * amount,
        a: color.a,
    }
}

/// The table: the window's ground, a tone under the window's in the dark, the
/// window's grey in the light.
pub fn table_color(t: &Tokens) -> Color {
    if t.is_dark {
        blend(t.window, Color::BLACK, 0.18)
    } else {
        t.window
    }
}

/// The sheet the page is on: a tone over the window in the dark, paper in the light.
pub fn sheet_color(t: &Tokens) -> Color {
    if t.is_dark {
        blend(t.window, t.surface, 0.4)
    } else {
        t.popover
    }
}

/// The hairlines on the sheet.
pub fn rule_color(t: &Tokens) -> Color {
    t.layer(if t.is_dark { 0.07 } else { 0.09 })
}

/// Words that come second on the sheet: quieter than the muted tone, still
/// easy to read (at least 4.5:1, which the faint tone is not).
pub fn soft(theme: &Theme) -> iced::widget::text::Style {
    iced::widget::text::Style {
        color: Some(soft_color(&Tokens::of(theme))),
    }
}

fn soft_color(t: &Tokens) -> Color {
    blend(t.muted, t.faint, 0.33)
}

/// The table's colour fading to nothing: downward where `from_top`, else upward.
pub fn fade(from_top: bool) -> impl Fn(&Theme) -> container::Style {
    move |theme| {
        let ground = table_color(&Tokens::of(theme));
        let (start, end) = if from_top {
            (ground, ground.scale_alpha(0.0))
        } else {
            (ground.scale_alpha(0.0), ground)
        };
        container::Style {
            background: Some(Background::Gradient(
                gradient::Linear::new(std::f32::consts::PI)
                    .add_stop(0.0, start)
                    .add_stop(1.0, end)
                    .into(),
            )),
            ..container::Style::default()
        }
    }
}

/// The table.
pub fn table(theme: &Theme) -> container::Style {
    let t = Tokens::of(theme);
    container::Style {
        background: Some(Background::Color(table_color(&t))),
        text_color: Some(t.text),
        ..container::Style::default()
    }
}

/// The sheet: a hairline edge, the large corner.
pub fn sheet(theme: &Theme) -> container::Style {
    let t = Tokens::of(theme);
    container::Style {
        background: Some(Background::Color(sheet_color(&t))),
        border: Border {
            color: t.layer(if t.is_dark { 0.06 } else { 0.08 }),
            width: 1.0,
            radius: (shape::lg() + 2.0).into(),
        },
        text_color: Some(t.text),
        ..container::Style::default()
    }
}

/// A hairline across the sheet.
pub fn hairline(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(rule_color(&Tokens::of(theme)))),
        ..container::Style::default()
    }
}

/// A plate's corner, and its ground's around it: the same centre, so the two
/// curves run side by side.
pub fn plate_radius() -> f32 {
    shape::lg()
}

/// A plate's ground: nothing at rest, a faint layer under the pointer, the
/// accent's line around the selected one.
pub fn plate(hovered: bool, selected: bool) -> impl Fn(&Theme) -> container::Style {
    move |theme| {
        let t = Tokens::of(theme);
        container::Style {
            background: (hovered || selected)
                .then(|| Background::Color(t.layer(if t.is_dark { 0.045 } else { 0.035 }))),
            border: Border {
                color: if selected {
                    t.accent_line()
                } else {
                    Color::TRANSPARENT
                },
                width: 1.0,
                radius: (plate_radius() + px(INSET)).into(),
            },
            text_color: Some(t.text),
            ..container::Style::default()
        }
    }
}

/// The edge drawn over a picture: it holds a dark picture on a dark sheet and
/// a pale one on paper, and takes nothing from either.
pub fn picture_edge(radius: f32) -> impl Fn(&Theme) -> container::Style {
    move |theme| {
        let t = Tokens::of(theme);
        container::Style {
            border: Border {
                color: t.layer(if t.is_dark { 0.09 } else { 0.12 }),
                width: 1.0,
                radius: radius.into(),
            },
            ..container::Style::default()
        }
    }
}

/// A picture's place when the app has none: its tone, faint.
pub fn placeholder(paint: Paint, radius: f32) -> impl Fn(&Theme) -> container::Style {
    move |theme| {
        let t = Tokens::of(theme);
        container::Style {
            background: Some(Background::Gradient(
                gradient::Linear::new(std::f32::consts::PI)
                    .add_stop(
                        0.0,
                        paint.top.scale_alpha(if t.is_dark { 0.14 } else { 0.16 }),
                    )
                    .add_stop(
                        1.0,
                        paint
                            .bottom
                            .scale_alpha(if t.is_dark { 0.04 } else { 0.05 }),
                    )
                    .into(),
            )),
            border: Border {
                color: t.layer(if t.is_dark { 0.09 } else { 0.12 }),
                width: 1.0,
                radius: radius.into(),
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
            radius: tile_radius(side).into(),
            ..Border::default()
        },
        text_color: Some(paint.ink),
        ..container::Style::default()
    }
}

/// A tile's corner: about a quarter of its side, whole pixels.
pub fn tile_radius(side: f32) -> f32 {
    (side * 0.24).round()
}

/// A dot of a colour (an app's state, the bell's unread mark).
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

/// The bell's dot: the accent, ringed in the sheet's colour so it stands off the bell.
pub fn bell_dot(theme: &Theme) -> container::Style {
    let t = Tokens::of(theme);
    container::Style {
        background: Some(Background::Color(t.accent)),
        border: Border {
            color: sheet_color(&t),
            width: 2.0,
            radius: 999.0.into(),
        },
        ..container::Style::default()
    }
}

/// A navigation item: the selected one on a neutral layer of the table (its
/// mark in the accent says it is the one); under the pointer a fainter layer;
/// the words muted until then.
pub fn nav(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme, status| {
        let t = Tokens::of(theme);
        let (background, text) = match (selected, status) {
            (true, _) => (t.layer(0.07), t.text),
            (false, button::Status::Hovered) => (t.layer(0.04), t.text),
            (false, button::Status::Pressed) => (t.layer(0.08), t.text),
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

/// A plate's button when it is not the plate to start: a neutral layer, the
/// text's colour; the accent is for the one that will start (DESIGN.md §1.2).
pub fn quiet(theme: &Theme, status: button::Status) -> button::Style {
    let t = Tokens::of(theme);
    let (background, text) = match status {
        button::Status::Active => (t.layer(0.06), t.text),
        button::Status::Hovered => (t.layer(0.1), t.text),
        button::Status::Pressed => (t.layer(0.14), t.text),
        button::Status::Disabled => (t.layer(0.03), t.disabled()),
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

/// A table's row that opens something: nothing at rest, a faint layer under the pointer.
pub fn row(theme: &Theme, status: button::Status) -> button::Style {
    let t = Tokens::of(theme);
    let background = match status {
        button::Status::Hovered => t.layer(0.04),
        button::Status::Pressed => t.layer(0.07),
        _ => Color::TRANSPARENT,
    };
    button::Style {
        background: Some(Background::Color(background)),
        text_color: t.text,
        border: Border {
            radius: shape::sm().into(),
            ..Border::default()
        },
        ..button::Style::default()
    }
}

/// A word beside a section's head that leads to its page: muted until the pointer is on it.
pub fn aside(theme: &Theme, status: button::Status) -> button::Style {
    let t = Tokens::of(theme);
    let (background, text) = match status {
        button::Status::Active => (Color::TRANSPARENT, t.muted),
        button::Status::Hovered => (t.layer(0.05), t.text),
        button::Status::Pressed => (t.layer(0.09), t.text),
        button::Status::Disabled => (Color::TRANSPARENT, t.disabled()),
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

/// A link-like button in the accent's text (Tümünü temizle).
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

/// The scroll bar on the sheet: no rail, a neutral handle that the pointer
/// wakes; the accent is not for moving about (DESIGN.md §1.2).
pub fn scroller(theme: &Theme, status: scrollable::Status) -> scrollable::Style {
    let t = Tokens::of(theme);
    let handle = match status {
        scrollable::Status::Active { .. } => t.layer(0.14),
        scrollable::Status::Hovered {
            is_vertical_scrollbar_hovered: true,
            ..
        } => t.layer(0.26),
        scrollable::Status::Hovered { .. } => t.layer(0.18),
        scrollable::Status::Dragged { .. } => t.layer(0.32),
    };
    let rail = scrollable::Rail {
        background: None,
        border: Border::default(),
        scroller: scrollable::Scroller {
            background: Background::Color(handle),
            border: Border {
                radius: 999.0.into(),
                ..Border::default()
            },
        },
    };
    scrollable::Style {
        container: container::Style::default(),
        vertical_rail: rail,
        horizontal_rail: rail,
        gap: None,
        auto_scroll: scrollable::AutoScroll {
            background: Background::Color(t.popover),
            border: Border {
                color: t.border,
                width: 1.0,
                radius: 999.0.into(),
            },
            shadow: iced::Shadow::default(),
            icon: t.text,
        },
    }
}

/// The add dialog's preview of the app: a faint layer of the dialog.
pub fn preview(theme: &Theme) -> container::Style {
    let t = Tokens::of(theme);
    container::Style {
        background: Some(Background::Color(t.layer(0.03))),
        border: Border {
            radius: shape::md().into(),
            ..Border::default()
        },
        text_color: Some(t.text),
        ..container::Style::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn luminance(c: Color) -> f32 {
        let f = |v: f32| {
            if v <= 0.040_45 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * f(c.r) + 0.7152 * f(c.g) + 0.0722 * f(c.b)
    }

    fn contrast(a: Color, b: Color) -> f32 {
        let (x, y) = (luminance(a), luminance(b));
        (x.max(y) + 0.05) / (x.min(y) + 0.05)
    }

    #[test]
    fn the_sheet_and_the_table_part_and_the_words_read_on_them() {
        for t in [Tokens::DARK, Tokens::LIGHT] {
            let sheet = sheet_color(&t);
            let table = table_color(&t);
            assert!(
                contrast(sheet, table) > 1.08,
                "{:?}: the sheet shows",
                t.mode
            );
            for (name, words) in [
                ("text", t.text),
                ("muted", t.muted),
                ("soft", soft_color(&t)),
            ] {
                let on_sheet = contrast(words, sheet);
                assert!(on_sheet >= 4.5, "{:?} {name}: {on_sheet}", t.mode);
            }
            // The status line's words, on the table.
            let on_table = contrast(soft_color(&t), table);
            assert!(
                on_table >= 4.5,
                "{:?} soft on the table: {on_table}",
                t.mode
            );
        }
    }
}
