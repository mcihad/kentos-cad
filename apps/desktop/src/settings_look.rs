//! Uygulama ayarları → Görünüm's pickers, as the web draws them
//! (`AppSettingsDialog.ts` `appearance()`, `appearancePickers.ts`,
//! `styles/settings.css`; docs/adr/0128):
//!
//! - the themes as miniature workbenches in their own colours, whatever the
//!   theme in use: a bar, the floating toolbox, parcels on the drawing with a
//!   dashed selection in the accent, the side panel;
//! - the accents as swatches, half in the dark themes' tone and half in the
//!   light one's (135°), a check on the chosen one; beside them a colour of
//!   one's own as `#RRGGBB`;
//! - the typefaces as cards written in their own letters;
//! - the text size as the web's five steps, with the pixels beside them for
//!   the sizes between (11–18).
//!
//! Each writes its setting into the window's draft like the other fields.

use std::f32::consts::PI;
use std::fmt;

use iced::widget::canvas::{self, LineDash, Path, Stroke, path::Arc};
use iced::widget::text::Wrapping;
use iced::widget::{
    Column, Row, button, canvas as canvas_widget, column, container, row, rule, space,
};
use iced::{
    Center, Color, Element, Fill, Point, Radians, Rectangle, Renderer, Size, Theme, Vector, mouse,
};
use kentos_ui::theme::typography::{self, Family, Typography};
use kentos_ui::theme::{Accent, Mode, Tokens, shape};
use kentos_ui::widget::Segmented;
use kentos_ui::{label, style};
use serde_json::Value;

use crate::app::Message;
use crate::settings::schema;
use crate::settings_view::Edit;
use crate::viewport::palette;

/// A workbench's height (the web's 108 px, in a card of about 180 px).
const MOCK: f32 = 92.0;
/// An accent's option and its swatch (the web's 84 and 30 px).
const ACCENT_OPTION: f32 = 84.0;
const SWATCH: f32 = 30.0;
/// The own colour's option: its field needs room for `#RRGGBB`.
const OWN_OPTION: f32 = 116.0;
/// Accents in a row, and typeface cards (the web's grid of 170 px and more).
const ACCENTS_IN_ROW: usize = 5;
const TYPEFACES_IN_ROW: usize = 4;
/// Text's line height (iced's default, which the labels keep).
const LINE: f32 = 1.3;
/// What a typeface card writes (the web's sample).
const SAMPLE: &str = "Ağ Şı İ 123";
/// The text size's steps (the web's TEXT_SIZES).
pub(crate) const TEXT_SIZES: [(u8, &str); 5] = [
    (12, "Küçük"),
    (13, "Standart"),
    (14, "Büyük"),
    (15, "Çok büyük"),
    (16, "En büyük"),
];

fn set(key: &'static str, value: impl Into<Value>) -> Message {
    Message::Settings(Edit::Value(key, value.into()))
}

/// A choice's name in the shared schema ("dark" → "Koyu grafit").
fn named(key: &str, id: &str) -> &'static str {
    schema()
        .get(key)
        .and_then(|d| d.choices.iter().find(|c| c.value.as_str() == Some(id)))
        .map_or("", |c| c.label.as_str())
}

/// A group of the section, as the form's own sections: its title before a
/// hairline, a note, then its content.
pub(crate) fn group<'a>(
    title: &'a str,
    note: Option<&'a str>,
    content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    let mut group = Column::new().spacing(10).push(
        row![
            label::strong(title),
            container(rule::horizontal(1).style(style::field::hairline)).width(Fill)
        ]
        .spacing(10)
        .align_y(Center),
    );
    if let Some(note) = note {
        group = group.push(label::caption(note));
    }
    group.push(content).into()
}

// ── Themes ──────────────────────────────────────────────────────────────

/// A theme in miniature, in its own colours.
struct Workbench {
    mode: Mode,
    accent: Color,
}

impl canvas::Program<Message> for Workbench {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let t = Tokens::base(self.mode);
        let drawing = palette(self.mode);
        let paper = crate::view::rgba_color(drawing.background);
        let ink = crate::view::rgba_color(drawing.fg);
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let (w, h) = (bounds.width, bounds.height);
        let bar = 12.0;
        let panel = (w * 0.3).round();
        let line = Stroke::default().with_color(t.border).with_width(1.0);

        frame.fill_rectangle(Point::ORIGIN, bounds.size(), paper);
        // The parcels, tilted as the web's, the last one selected.
        let area = Size::new(w - panel, h - bar);
        let middle = Vector::new(area.width / 2.0, bar + area.height / 2.0);
        let parcel = |x: f32, y: f32, pw: f32, ph: f32| {
            Path::rectangle(
                Point::new(area.width * x - middle.x, bar + area.height * y - middle.y),
                Size::new(area.width * pw, area.height * ph),
            )
        };
        frame.with_save(|frame| {
            frame.translate(middle);
            frame.rotate(-12.0_f32.to_radians());
            let edge = Stroke::default()
                .with_color(ink.scale_alpha(0.75))
                .with_width(1.0);
            frame.stroke(&parcel(0.3, 0.22, 0.26, 0.34), edge);
            frame.stroke(&parcel(0.58, 0.3, 0.22, 0.3), edge);
            frame.stroke(
                &parcel(0.4, 0.6, 0.2, 0.18),
                Stroke {
                    line_dash: LineDash {
                        segments: &[3.0, 2.0],
                        offset: 0,
                    },
                    ..Stroke::default().with_color(self.accent).with_width(1.0)
                },
            );
        });
        // The bar over the drawing, the panel on its right with lines of text.
        frame.fill_rectangle(Point::ORIGIN, Size::new(w, bar), t.window);
        frame.fill_rectangle(Point::new(0.0, bar - 1.0), Size::new(w, 1.0), t.border);
        frame.fill_rectangle(
            Point::new(w - panel, bar),
            Size::new(panel, h - bar),
            t.surface,
        );
        frame.fill_rectangle(
            Point::new(w - panel, bar),
            Size::new(1.0, h - bar),
            t.border,
        );
        // Lines of text and idle tools: a fifth of the way from the panel to
        // the text (the web's --m-dim), not the high-contrast theme's white.
        let dim = mix(t.surface, t.text, 0.2);
        for i in 0..4 {
            let long = if i == 1 { 0.7 } else { 1.0 };
            frame.fill(
                &Path::rounded_rectangle(
                    Point::new(w - panel + 6.0, bar + 6.0 + i as f32 * 9.0),
                    Size::new(((panel - 12.0) * long).max(2.0), 5.0),
                    2.0.into(),
                ),
                dim,
            );
        }
        // The floating toolbox: three tools, the first one running.
        let tools = Rectangle::new(Point::new(6.0, bar + 6.0), Size::new(16.0, 39.0));
        let toolbox = Path::rounded_rectangle(tools.position(), tools.size(), 3.0.into());
        frame.fill(&toolbox, t.surface);
        frame.stroke(&toolbox, line);
        for i in 0..3 {
            frame.fill(
                &Path::rounded_rectangle(
                    Point::new(tools.x + 4.0, tools.y + 4.0 + i as f32 * 11.0),
                    Size::new(8.0, 8.0),
                    2.0.into(),
                ),
                if i == 0 { self.accent } else { dim },
            );
        }
        vec![frame.into_geometry()]
    }
}

/// `from` moved `k` of the way to `to`.
fn mix(from: Color, to: Color, k: f32) -> Color {
    Color::from_rgb(
        from.r + (to.r - from.r) * k,
        from.g + (to.g - from.g) * k,
        from.b + (to.b - from.b) * k,
    )
}

/// The four themes as cards (the web's `.theme-card`), each a workbench in
/// its own colours with the chosen accent.
pub(crate) fn theme_cards(current: &Value, accent: Accent) -> Element<'static, Message> {
    let cards = Mode::ALL.into_iter().map(|mode| {
        let id = theme_id(mode);
        let chosen = current.as_str() == Some(id);
        let workbench = canvas_widget(Workbench {
            mode,
            accent: accent.color(mode),
        })
        .width(Fill)
        .height(typography::scaled(MOCK));
        let mock = container(workbench)
            .width(Fill)
            .clip(true)
            .style(move |_: &Theme| container::Style {
                border: iced::Border {
                    color: Tokens::base(mode).border,
                    width: 1.0,
                    radius: shape::md().into(),
                },
                ..container::Style::default()
            });
        let name = label::text(named("appearance.theme", id))
            .wrapping(Wrapping::None)
            .width(Fill);
        Element::from(
            button(column![mock, container(name).padding([0, 4]).clip(true)].spacing(8))
                .on_press(set("appearance.theme", id))
                .padding(8)
                .width(Fill)
                .style(move |theme, status| card(theme, status, chosen)),
        )
    });
    Row::with_children(cards).spacing(12).into()
}

/// The setting's id of a theme.
pub(crate) fn theme_id(mode: Mode) -> &'static str {
    match mode {
        Mode::Dark => "dark",
        Mode::Light => "light",
        Mode::Night => "night",
        Mode::HighContrast => "highContrast",
    }
}

/// A theme card: a border that strengthens under the pointer; the chosen
/// one in the accent, two pixels wide (the web's border and ring).
fn card(theme: &Theme, status: button::Status, chosen: bool) -> button::Style {
    let t = Tokens::of(theme);
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    let (edge, width) = match (chosen, hovered) {
        (true, _) => (t.accent, 2.0),
        (false, true) => (t.border_strong(), 1.0),
        (false, false) => (t.border, 1.0),
    };
    button::Style {
        background: hovered.then(|| t.layer(0.04).into()),
        text_color: t.text,
        border: iced::Border {
            color: edge,
            width,
            radius: shape::lg().into(),
        },
        shadow: iced::Shadow::default(),
        snap: true,
    }
}

// ── Accents ─────────────────────────────────────────────────────────────

/// A swatch: half the dark themes' tone, half the light ones' (135°); the
/// chosen one checked and ringed. Without tones, an empty ring (a colour of
/// one's own not written yet).
struct Swatch {
    tones: Option<(Color, Color)>,
    chosen: bool,
}

impl canvas::Program<Message> for Swatch {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let t = Tokens::of(theme);
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let center = frame.center();
        let radius = typography::scaled(SWATCH) / 2.0;
        let Some((dark, light)) = self.tones else {
            frame.stroke(
                &Path::circle(center, radius - 0.5),
                Stroke {
                    line_dash: LineDash {
                        segments: &[3.0, 3.0],
                        offset: 0,
                    },
                    ..Stroke::default().with_color(t.muted).with_width(1.0)
                },
            );
            return vec![frame.into_geometry()];
        };
        frame.fill(&Path::circle(center, radius), light);
        // The upper left half, from the lower left round to the upper right.
        let half = Path::new(|b| {
            b.arc(Arc {
                center,
                radius,
                start_angle: Radians(0.75 * PI),
                end_angle: Radians(1.75 * PI),
            });
            b.close();
        });
        frame.fill(&half, dark);
        frame.stroke(
            &Path::circle(center, radius - 0.5),
            Stroke::default()
                .with_color(Color::BLACK.scale_alpha(0.2))
                .with_width(1.0),
        );
        if self.chosen {
            frame.stroke(
                &Path::circle(center, radius + 3.0),
                Stroke::default().with_color(t.muted).with_width(2.0),
            );
            let s = radius * 0.4;
            let check = Path::new(|b| {
                b.move_to(Point::new(center.x - s, center.y + s * 0.05));
                b.line_to(Point::new(center.x - s * 0.3, center.y + s * 0.75));
                b.line_to(Point::new(center.x + s, center.y - s * 0.6));
            });
            frame.stroke(
                &check,
                Stroke::default().with_color(Color::WHITE).with_width(2.0),
            );
        }
        vec![frame.into_geometry()]
    }
}

fn swatch(tones: Option<(Color, Color)>, chosen: bool) -> Element<'static, Message> {
    // Room for the ring round the chosen one.
    let side = typography::scaled(SWATCH) + 12.0;
    canvas_widget(Swatch { tones, chosen })
        .width(side)
        .height(side)
        .into()
}

fn tones(accent: Accent) -> Option<(Color, Color)> {
    Some((accent.color(Mode::Dark), accent.color(Mode::Light)))
}

/// The ten accents as swatches with their names, in rows of five, and a
/// colour of one's own beside them (the web's `.accent-pick`). A colour
/// that does not read yet is kept as typed and said so under them.
pub(crate) fn accent_swatches(current: &str) -> Element<'static, Message> {
    let chosen = Accent::parse(current);
    let option = |accent: Accent| {
        let on = chosen == Some(accent);
        let mut name = label::text(accent.name())
            .size(typography::caption())
            .wrapping(Wrapping::None);
        if on {
            name = name.font(typography::ui_strong());
        }
        Element::from(
            button(
                column![swatch(tones(accent), on), name]
                    .spacing(4)
                    .width(Fill)
                    .align_x(Center),
            )
            .on_press(set("appearance.accent", accent.key()))
            .padding([8, 4])
            .width(typography::scaled(ACCENT_OPTION))
            .style(move |theme, status| accent_option(theme, status, on)),
        )
    };
    let presets = Accent::PRESETS.chunks(ACCENTS_IN_ROW).map(|chunk| {
        Element::from(Row::with_children(chunk.iter().map(|&a| option(a))).spacing(6))
    });
    let presets = Column::with_children(presets).spacing(6);

    let own = match chosen {
        Some(accent @ Accent::Custom(_)) => Some(accent),
        _ => None,
    };
    let typed = if current.starts_with('#') {
        current
    } else {
        ""
    };
    let field = kentos_ui::widget::focus_ring(
        iced::widget::text_input("#RRGGBB", typed)
            .on_input(|text| set("appearance.accent", text))
            .font(typography::mono())
            .size(typography::caption())
            .padding([3, 6])
            .style(style::field::input),
    );
    let own = container(
        column![
            swatch(own.and_then(tones), own.is_some()),
            field,
            label::caption("Özel renk"),
        ]
        .spacing(4)
        .align_x(Center),
    )
    .padding([8, 8])
    .width(typography::scaled(OWN_OPTION))
    .style(move |theme: &Theme| {
        let t = Tokens::of(theme);
        container::Style {
            background: own.map(|_| t.field.into()),
            border: iced::Border {
                color: if own.is_some() {
                    t.border_strong()
                } else {
                    Color::TRANSPARENT
                },
                width: 1.0,
                radius: shape::md().into(),
            },
            ..container::Style::default()
        }
    });

    let mut picker = column![row![presets, own].spacing(18)].spacing(8);
    if chosen.is_none() && !current.is_empty() {
        picker = picker.push(
            label::caption("Okunamadı: #RRGGBB biçiminde yazın (ör. #2F6FD0).")
                .style(style::text::danger),
        );
    }
    picker.into()
}

/// An accent's option: a hover layer; the chosen one on the field, bordered.
fn accent_option(theme: &Theme, status: button::Status, chosen: bool) -> button::Style {
    let t = Tokens::of(theme);
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    let (background, edge) = match (chosen, hovered) {
        (true, _) => (Some(t.field.into()), t.border_strong()),
        (false, true) => (Some(t.layer(0.06).into()), Color::TRANSPARENT),
        (false, false) => (None, Color::TRANSPARENT),
    };
    button::Style {
        background,
        text_color: if chosen || hovered { t.text } else { t.muted },
        border: iced::Border {
            color: edge,
            width: 1.0,
            radius: shape::md().into(),
        },
        shadow: iced::Shadow::default(),
        snap: true,
    }
}

// ── Typefaces ───────────────────────────────────────────────────────────

/// The typefaces as cards written in their own letters (the web's
/// `.font-pick`), in rows of four.
pub(crate) fn typeface_cards(current: &Value) -> Element<'static, Message> {
    let card = |family: Family| {
        let id = family_id(family);
        let on = current.as_str() == Some(id);
        let face = Typography {
            family,
            ..Typography::DEFAULT
        }
        .ui_strong();
        Element::from(
            button(
                column![
                    label::text(SAMPLE)
                        .font(face)
                        .size(typography::scaled(22.0))
                        .wrapping(Wrapping::None),
                    label::text(family.name())
                        .font(face)
                        .wrapping(Wrapping::None),
                    // Two lines' room: the cards of a row stay as high as each other.
                    container(label::caption(family_note(family)))
                        .height(2.0 * LINE * typography::caption()),
                ]
                .spacing(2),
            )
            .on_press(set("appearance.uiFont", id))
            .padding([10, 12])
            .width(Fill)
            .clip(true)
            .style(move |theme, status| typeface_card(theme, status, on)),
        )
    };
    let rows = Family::ALL.chunks(TYPEFACES_IN_ROW).map(|chunk| {
        let mut row = Row::with_children(chunk.iter().map(|&f| card(f))).spacing(8);
        // The last row's cards as wide as the others.
        for _ in chunk.len()..TYPEFACES_IN_ROW {
            row = row.push(space::horizontal());
        }
        Element::from(row)
    });
    Column::with_children(rows).spacing(8).into()
}

/// A typeface card: on the field; the chosen one on the soft accent,
/// bordered in the accent.
fn typeface_card(theme: &Theme, status: button::Status, chosen: bool) -> button::Style {
    let t = Tokens::of(theme);
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    let (background, edge, width) = match (chosen, hovered) {
        (true, _) => (t.selection(), t.accent, 2.0),
        (false, true) => (t.surface_hover, t.muted, 1.0),
        (false, false) => (t.field, t.border_strong(), 1.0),
    };
    button::Style {
        background: Some(background.into()),
        text_color: t.text,
        border: iced::Border {
            color: edge,
            width,
            radius: shape::md().into(),
        },
        shadow: iced::Shadow::default(),
        snap: true,
    }
}

/// The setting's id of a typeface (the web's UI_FONTS).
pub(crate) fn family_id(family: Family) -> &'static str {
    match family {
        Family::PlusJakartaSans => "jakarta",
        Family::Inter => "inter",
        Family::IbmPlexSans => "plex",
        Family::SourceSans3 => "source",
        Family::NotoSans => "noto",
        Family::Roboto => "roboto",
        Family::System => "system",
    }
}

/// The web's notes on the typefaces (UI_FONTS).
fn family_note(family: Family) -> &'static str {
    match family {
        Family::PlusJakartaSans => "Geniş ve modern; varsayılan",
        Family::Inter => "Ekran için çizilmiş, sık ve net",
        Family::IbmPlexSans => "Teknik ve kurumsal",
        Family::SourceSans3 => "Dar; dar ekranda çok yazı sığar",
        Family::NotoSans => "Geniş dil desteği",
        Family::Roboto => "Tanıdık ve dengeli",
        Family::System => "İşletim sisteminin yazı tipi; indirme yok",
    }
}

// ── Text size ───────────────────────────────────────────────────────────

/// A step of the text size, as a segment.
#[derive(Clone, Copy, PartialEq)]
struct SizeStep(u8, &'static str);

impl fmt::Display for SizeStep {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.1)
    }
}

/// The web's five steps (12–16 px); none chosen when the size is between
/// or beyond them, which the pixels beside them write (11–18).
pub(crate) fn text_sizes<'a>(
    current: &Value,
    pixels: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    let now = current.as_f64().unwrap_or(13.0);
    let steps = TEXT_SIZES.map(|(px, name)| SizeStep(px, name));
    let chosen = steps
        .into_iter()
        .find(|s| f64::from(s.0) == now)
        .unwrap_or(SizeStep(0, ""));
    let steps = Segmented::new(steps, chosen, |s: SizeStep| {
        set("appearance.textSize", i64::from(s.0))
    });
    Row::new()
        .push(steps)
        .push(pixels)
        .spacing(10)
        .align_y(Center)
        .wrap()
        .vertical_spacing(6)
        .into()
}
