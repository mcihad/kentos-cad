//! How Sembol tasarımcısı looks (the web's `SymbolDesigner` and its
//! `sdes__*` rules): the layer list on the left with Katman ekle, the
//! list's tools and the drawing order's note; the preview in the middle
//! under its bar (the sample object, the scale, 1:1); the chosen layer's
//! form on the right; name, category, what the window said, Vazgeç and
//! Kaydet (Uygula for a layer style's symbol) at the foot. The window has the
//! web's size (1320 × 860 at most, 92 % of the window's height) and stands
//! over the window that opened it.

use iced::mouse::ScrollDelta;
use iced::widget::tooltip::Position;
use iced::widget::{
    Column, Row, button, column, container, mouse_area, responsive, row, scrollable, space, stack,
    text_input,
};
use iced::{Border, Center, Color, Element, Fill, Length, Theme};
use kentos_native_style::designer::{
    self as model, LayerPath, add_parent, can_move, can_remove, has_marker, label as layer_label,
    layer_types, summary, texts, type_of,
};
use kentos_native_style::library::ItemKind;
use kentos_native_style::preview::Geometry;
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::tree_view::{Check, check_box};
use kentos_ui::widget::{
    Dialog, Menu, MenuButton, Segmented, Tip, horizontal_divider, overlay, tip, vertical_divider,
};
use serde_json::Value;

use super::parts::{self, Env};
use super::{Designer, Event, ev, field_id, form};
use crate::app::{App, Dialog as Under, Message};
use crate::style::fields::{self, ColorEnv};
use crate::style::thumbs::Look;

/// The window's size, the web's pixels at the default text size.
const WIDTH: f32 = 1320.0;
const HEIGHT: f32 = 860.0;
/// The list's and the form's columns; on a narrow window they give way to the preview.
const LEFT: f32 = 270.0;
const RIGHT: f32 = 360.0;
/// A child row's indent under the layer that places its marker.
const INDENT: f32 = 18.0;

/// The list's rows in order: each layer, and under a layer that places
/// markers its marker's layers.
pub(super) fn rows(symbol: &Value) -> Vec<LayerPath> {
    let mut out = Vec::new();
    let layers = symbol
        .get("layers")
        .and_then(Value::as_array)
        .map_or(&[][..], Vec::as_slice);
    for (i, l) in layers.iter().enumerate() {
        out.push(LayerPath::Top(i));
        if has_marker(l) {
            let n = l
                .get("marker")
                .and_then(|m| m.get("layers"))
                .and_then(Value::as_array)
                .map_or(0, Vec::len);
            out.extend((0..n).map(|j| LayerPath::Child(i, j)));
        }
    }
    out
}

/// A sample object of the preview's bar, named as the web's segmented control names it.
#[derive(Clone, Copy, PartialEq)]
struct Sample(Geometry, &'static str);

impl std::fmt::Display for Sample {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.1)
    }
}

impl App {
    /// Sembol tasarımcısı over the window that opened it.
    pub(crate) fn designer_view(&self) -> Element<'_, Message> {
        let Some(d) = &self.styles.designer else {
            return space().into();
        };
        let mut layers: Vec<Element<'_, Message>> = Vec::new();
        match d.under {
            Some(Under::StyleManager) if self.styles.manager.is_some() => {
                layers.push(self.style_manager_view());
            }
            Some(Under::LayerStyle) if self.styles.layer_style.is_some() => {
                layers.push(self.layer_style_view());
            }
            _ => {}
        }
        layers.push(
            responsive(move |room| {
                let width = typography::from_default(WIDTH).min(room.width - 60.0);
                let height = typography::from_default(HEIGHT).min(room.height * 0.92);
                overlay::modal(self.designer_window(d, width, height), ev(Event::Close))
            })
            .into(),
        );
        if d.asking {
            layers.push(overlay::modal(question(d), ev(Event::Stay)));
        }
        stack(layers).into()
    }

    fn designer_window<'a>(&'a self, d: &'a Designer, width: f32, height: f32) -> Element<'a, Message> {
        let left_w = typography::from_default(LEFT).min(width * 0.24).floor();
        let right_w = typography::from_default(RIGHT).min(width * 0.32).floor();
        // The dialog's padding, the frame's border and the two dividers.
        let center_w = width - 36.0 - left_w - right_w - 4.0;
        let cols = row![
            container(layer_list(d))
                .width(Length::Fixed(left_w))
                .height(Fill)
                .padding(12)
                .style(style::container::header),
            vertical_divider(),
            self.preview(d, center_w),
            vertical_divider(),
            container(
                scrollable(container(self.props(d)).padding([12, 14]))
                    .direction(style::field::body_scrollbar())
                    .height(Fill)
            )
            .width(Length::Fixed(right_w))
            .height(Fill)
            .style(style::container::header),
        ]
        .height(Fill);
        let frame = container(cols)
            .height(Fill)
            .clip(true)
            .style(|t: &Theme| container::Style {
                border: Border {
                    color: Tokens::of(t).border,
                    width: 1.0,
                    radius: 4.0.into(),
                },
                ..container::Style::default()
            });
        Dialog::new(d.title())
            .push(frame)
            .push(footer(d))
            .width(typography::unscaled(width))
            .max_height(typography::unscaled(height))
            .into()
    }

    /// The middle: the preview's bar over the picture of the symbol on its
    /// sample. On a narrow middle (a small window, large text) the scale's
    /// buttons go under the samples instead of over them.
    fn preview<'a>(&'a self, d: &'a Designer, width: f32) -> Element<'a, Message> {
        let samples: Vec<Sample> = model::geometries(d.kind())
            .iter()
            .map(|(g, l)| Sample(*g, l))
            .collect();
        let fits = bar_width(&samples, d.px_per_mm) <= width;
        let chooser: Element<'a, Message> = if samples.len() > 1 {
            let chosen = samples
                .iter()
                .copied()
                .find(|s| s.0 == d.sample)
                .unwrap_or(samples[0]);
            Segmented::new(samples, chosen, |s| ev(Event::Sample(s.0))).into()
        } else {
            label::caption(texts::ONE_POINT)
                .style(style::text::muted)
                .into()
        };
        let zoom_button = |glyph: &str, words: &str, press: Option<Message>| {
            tip(
                button(label::body(glyph.to_owned()))
                    .padding([2, 7])
                    .style(style::button::ghost)
                    .on_press_maybe(press),
                Tip::new(words.to_owned()),
                Position::Bottom,
            )
        };
        let px = d.px_per_mm;
        let zoom = row![
            zoom_button(
                "−",
                "Uzaklaş (tekerlek aşağı)",
                (px > model::zoom::MIN).then(|| ev(Event::Zoom(1.0 / model::zoom::STEP)))
            ),
            tip(
                label::caption(model::zoom_text(px)).style(style::text::muted),
                Tip::new("Kâğıt milimetresinin ekrandaki boyu"),
                Position::Bottom,
            ),
            zoom_button(
                "+",
                "Yakınlaş (tekerlek yukarı)",
                (px < model::zoom::MAX).then(|| ev(Event::Zoom(model::zoom::STEP)))
            ),
            zoom_button("1:1", "Gerçek boyut (96 dpi)", Some(ev(Event::RealSize))),
        ]
        .spacing(6)
        .align_y(Center);
        let bar: Element<'a, Message> = if fits {
            row![chooser, space::horizontal(), zoom]
                .spacing(6)
                .align_y(Center)
                .into()
        } else {
            column![chooser, row![space::horizontal(), zoom]]
                .spacing(6)
                .into()
        };
        let palette = self.style_palette();
        let images = self.styles.images.clone();
        let library = &self.styles.library;
        let symbol = &d.draft.symbol;
        let (sample, thumbs) = (d.sample, &d.thumbs);
        let picture = responsive(move |size| {
            let look = Look {
                palette: &palette,
                library,
                images: &images,
            };
            let w = (size.width - 2.0).max(8.0).floor();
            let h = (size.height - 2.0).max(8.0).floor();
            container(thumbs.picture(symbol, Some(sample), (w, h), Some(px), &look))
                .padding(1)
                .style(|t: &Theme| container::Style {
                    border: Border {
                        color: Tokens::of(t).border,
                        width: 1.0,
                        radius: 3.0.into(),
                    },
                    ..container::Style::default()
                })
                .into()
        });
        let stage = mouse_area(container(picture).padding(16).width(Fill).height(Fill)).on_scroll(
            |delta| {
                let dy = match delta {
                    ScrollDelta::Lines { y, .. } | ScrollDelta::Pixels { y, .. } => y,
                };
                ev(Event::Wheel(dy))
            },
        );
        container(column![
            container(bar).padding([8, 12]).width(Fill),
            horizontal_divider(),
            stage
        ])
        .width(Fill)
        .height(Fill)
        .style(style::container::field)
        .into()
    }

    /// The right: the chosen layer's name (and the layer whose marker it is in) over its form.
    fn props<'a>(&'a self, d: &'a Designer) -> Element<'a, Message> {
        let Some(layer) = d.layer() else {
            return label::caption(texts::PICK_LAYER)
                .style(style::text::muted)
                .into();
        };
        let parent = match d.selected {
            LayerPath::Child(i, _) => d
                .draft
                .symbol
                .get("layers")
                .and_then(|l| l.get(i))
                .map(|p| texts::in_marker(layer_label(type_of(p)))),
            LayerPath::Top(_) => None,
        };
        let mut title = row![label::body(layer_label(type_of(layer))).font(typography::ui_strong())]
            .spacing(8)
            .align_y(iced::alignment::Vertical::Bottom);
        if let Some(p) = parent {
            title = title.push(label::caption(p).style(style::text::muted));
        }
        let context = match d.selected {
            LayerPath::Child(..) => "marker",
            LayerPath::Top(_) => d.kind(),
        };
        let lib = &self.styles.library;
        let assets: Vec<(String, String, bool)> = lib
            .items(None)
            .into_iter()
            .filter(|(i, _)| i.kind() == ItemKind::Asset)
            .map(|(i, source)| {
                let whose = match source {
                    kentos_native_style::Source::System => "",
                    kentos_native_style::Source::User => " (Kitaplığım)",
                    kentos_native_style::Source::Project => " (Proje)",
                };
                (
                    i.id().to_owned(),
                    format!("{}{whose}", i.name()),
                    i.format() == Some("svg"),
                )
            })
            .collect();
        let palette = self.style_palette();
        let env = Env {
            at: d.selected,
            layer,
            context,
            typed: &d.typed,
            colors: ColorEnv {
                palette: &palette,
                recent: symbol_colors(&d.draft.symbol, &palette),
            },
            assets: &assets,
        };
        column![title, horizontal_divider(), form::form(&env)]
            .spacing(10)
            .width(Fill)
            .into()
    }
}

/// The width the preview's bar takes on one line: the samples' segmented
/// control, the scale's buttons and text, their gaps and the bar's padding.
fn bar_width(samples: &[Sample], px_per_mm: f64) -> f32 {
    let body = typography::body();
    let text = |s: &str| typography::text_width(s, body);
    let segments: f32 = if samples.len() > 1 {
        samples.iter().map(|s| text(s.1) + 24.0 + 1.0).sum::<f32>() + 2.0
    } else {
        typography::text_width(texts::ONE_POINT, typography::caption())
    };
    let buttons = text("−") + text("+") + text("1:1") + 3.0 * 14.0;
    let scale = typography::text_width(&model::zoom_text(px_per_mm), typography::caption());
    segments + buttons + scale + 5.0 * 6.0 + 24.0
}

/// The symbol's own colours, each once: offered under the colour picker.
fn symbol_colors(symbol: &Value, palette: &kentos_native_style::StylePalette) -> Vec<Color> {
    fn walk(v: &Value, palette: &kentos_native_style::StylePalette, out: &mut Vec<Color>) {
        match v {
            Value::Object(o) => {
                for (k, x) in o {
                    if matches!(k.as_str(), "color" | "fill" | "stroke" | "fallback")
                        && let Some(c) = x.as_str().and_then(|s| fields::resolved(s, palette))
                        && !out.iter().any(|y| y.into_rgba8() == c.into_rgba8())
                    {
                        out.push(c);
                    } else {
                        walk(x, palette, out);
                    }
                }
            }
            Value::Array(a) => a.iter().for_each(|x| walk(x, palette, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    walk(symbol, palette, &mut out);
    out.truncate(20);
    out
}

/// The left: Katmanlar and Katman ekle, the rows, the tools and the order's note.
fn layer_list(d: &Designer) -> Element<'_, Message> {
    let symbol = &d.draft.symbol;
    let kind = d.kind().to_owned();
    let parent = add_parent(symbol, d.selected);
    let parent_label = parent
        .and_then(|i| symbol.get("layers").and_then(|l| l.get(i)))
        .map(|p| layer_label(type_of(p)));
    let add = MenuButton::new(
        container(
            row![
                icon(parts::glyph("plus")).size(13.0),
                label::body(texts::ADD)
            ]
            .spacing(5)
            .align_y(Center),
        )
        .padding([4, 10])
        .style(style::container::field_box),
        move || {
            let mut m = Menu::new().header(texts::INTO_SYMBOL);
            for t in layer_types(&kind) {
                m = m.item(layer_label(t), ev(Event::Add(t, None)));
            }
            if let (Some(i), Some(name)) = (parent, parent_label) {
                m = m.separator().header(texts::into_marker(name));
                for t in layer_types("marker") {
                    m = m.item(layer_label(t), ev(Event::Add(t, Some(i))));
                }
            }
            m
        },
    );
    let head = row![
        label::caption(texts::LAYERS)
            .font(typography::ui_strong())
            .style(style::text::muted),
        space::horizontal(),
        add
    ]
    .align_y(Center);
    let list = Column::with_children(
        rows(symbol)
            .into_iter()
            .filter_map(|p| model::layer_at(symbol, p).map(|l| layer_row(d, p, l))),
    )
    .spacing(2);
    let tool = |glyph: &str, words: &str, press: Option<Message>| -> Element<'_, Message> {
        tip(
            button(parts::tool_face(glyph))
                .padding([3, 5])
                .style(style::button::ghost)
                .on_press_maybe(press),
            Tip::new(words.to_owned()),
            Position::Top,
        )
    };
    let at = d.selected;
    let exists = model::layer_at(symbol, at).is_some();
    let tools = row![
        tool(
            "chevronUp",
            "Yukarı taşı (önce çizilir)",
            can_move(symbol, at, -1).then(|| ev(Event::Up))
        ),
        tool(
            "chevronDown",
            "Aşağı taşı (sonra çizilir)",
            can_move(symbol, at, 1).then(|| ev(Event::Down))
        ),
        tool("copy", "Çoğalt", exists.then(|| ev(Event::Duplicate))),
        tool(
            "trash",
            if can_remove(symbol, at) || !exists {
                "Sil"
            } else {
                "Sil: sembolün en az bir katmanı olmalı"
            },
            can_remove(symbol, at).then(|| ev(Event::Remove))
        ),
        space::horizontal(),
        tool(
            "undo",
            "Geri al (Ctrl+Z)",
            d.can_undo().then(|| ev(Event::Undo))
        ),
        tool(
            "redo",
            "Yinele (Ctrl+Y)",
            d.can_redo().then(|| ev(Event::Redo))
        ),
    ]
    .spacing(2)
    .align_y(Center);
    column![
        head,
        scrollable(list)
            .direction(style::field::thin_scrollbar())
            .height(Fill),
        horizontal_divider(),
        tools,
        label::caption(texts::ORDER).style(style::text::muted),
    ]
    .spacing(8)
    .into()
}

/// A row of the list: drawn or not, the layer's name and its summary.
fn layer_row<'a>(d: &Designer, p: LayerPath, l: &Value) -> Element<'a, Message> {
    let chosen = d.selected == p;
    let enabled = l.get("enabled");
    // A condition: the box shows it, and a press chooses the row, where Görünür edits it.
    let (state, press) = match enabled {
        Some(Value::Object(_)) => (Check::Mixed, ev(Event::Select(p))),
        Some(Value::Bool(false)) => (Check::Unchecked, ev(Event::Enabled(p, true))),
        _ => (Check::Checked, ev(Event::Enabled(p, false))),
    };
    let boxed: Element<'a, Message> = if matches!(state, Check::Mixed) {
        tip(
            check_box(state, Some(press)),
            Tip::new("Koşula bağlı").body("Görünür alanındaki ifade her nesnede karar verir."),
            Position::Right,
        )
    } else {
        check_box(state, Some(press))
    };
    let size = typography::caption();
    let words = column![
        label::body(layer_label(type_of(l))),
        label::caption(summary(l)).size(size).style(style::text::muted),
    ]
    .spacing(1)
    .width(Fill);
    let face = row![boxed, words].spacing(8).align_y(Center);
    let indent = if matches!(p, LayerPath::Child(..)) {
        typography::from_default(INDENT)
    } else {
        0.0
    };
    container(
        button(face)
            .on_press(ev(Event::Select(p)))
            .padding([6, 8])
            .width(Fill)
            .style(move |t: &Theme, status| row_style(t, status, chosen)),
    )
    .padding(iced::Padding {
        left: indent,
        ..iced::Padding::ZERO
    })
    .into()
}

/// A row: the chosen one on the accent's soft ground with its line (`sdes__row`).
fn row_style(theme: &Theme, status: button::Status, chosen: bool) -> button::Style {
    let t = Tokens::of(theme);
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    let (background, edge) = if chosen {
        (t.accent.scale_alpha(0.14), t.accent.scale_alpha(0.55))
    } else if hovered {
        (t.surface_hover, Color::TRANSPARENT)
    } else {
        (Color::TRANSPARENT, Color::TRANSPARENT)
    };
    button::Style {
        background: Some(iced::Background::Color(background)),
        text_color: t.text,
        border: Border {
            color: edge,
            width: 1.0,
            radius: 4.0.into(),
        },
        ..button::Style::default()
    }
}

/// The foot: name and category (or the note on a layer style's symbol), what
/// the window said, Vazgeç and Kaydet or Uygula.
fn footer(d: &Designer) -> Element<'_, Message> {
    let mut foot = Row::new().spacing(10).align_y(Center);
    if d.inline().is_some() {
        foot = foot.push(label::caption(texts::INLINE).style(style::text::muted));
    } else {
        let field = |id: &str, hint: &str, value: &str, width: f32, on: fn(String) -> Event| {
            text_input(hint, value)
                .id(field_id(id))
                .on_input(move |t| ev(on(t)))
                .padding([4, 7])
                .size(typography::body())
                .style(style::field::validated(false))
                .width(Length::Fixed(typography::from_default(width)))
        };
        foot = foot
            .push(
                row![
                    label::body("Ad").style(style::text::muted),
                    field("name", "", &d.draft.name, 190.0, Event::Name)
                ]
                .spacing(6)
                .align_y(Center),
            )
            .push(
                row![
                    label::body("Kategori").style(style::text::muted),
                    field("path", "Ana / Alt", &d.path_text, 230.0, Event::Path)
                ]
                .spacing(6)
                .align_y(Center),
            );
    }
    let status: Element<'_, Message> = match d.said() {
        Some((text, warn)) => {
            let warn = *warn;
            row![
                icon(if warn { Icon::Warning } else { Icon::Check })
                    .size(14.0)
                    .tone(if warn { Tone::Warning } else { Tone::Success }),
                label::body(text.clone()).style(move |t: &Theme| iced::widget::text::Style {
                    color: Some(if warn {
                        Tokens::of(t).warning
                    } else {
                        Tokens::of(t).muted
                    }),
                }),
            ]
            .spacing(6)
            .align_y(Center)
            .into()
        }
        None => space().into(),
    };
    let save_words = if d.inline().is_some() {
        "Uygula"
    } else {
        "Kaydet"
    };
    foot.push(container(status).width(Fill))
        .push(
            button(label::body("Vazgeç"))
                .padding([5, 14])
                .style(style::button::secondary)
                .on_press(ev(Event::Close)),
        )
        .push(
            button(
                row![
                    icon(Icon::Check).size(14.0).tone(Tone::OnAccent),
                    label::body(save_words).style(style::text::on_accent)
                ]
                .spacing(6)
                .align_y(Center),
            )
            .padding([5, 14])
            .style(style::button::primary)
            .on_press(ev(Event::Save)),
        )
        .into()
}

/// Closing with changes (the web's `askUnsaved`, DESIGN.md §7.9.1): the
/// leaving answer aside on the left, Vazgeç, and saving first in weight.
fn question(d: &Designer) -> Element<'_, Message> {
    let inline = d.inline();
    let (title, state, save, without) = if inline.is_some() {
        ("Uygulanmamış değişiklikler", "uygulanmamış", "Uygula", "Uygulamadan")
    } else {
        ("Kaydedilmemiş değişiklikler", "kaydedilmemiş", "Kaydet", "Kaydetmeden")
    };
    let name = inline.map_or_else(|| d.saved_as().0, str::to_owned);
    let symbol = container(icon(Icon::Warning).size(18.0).tone(Tone::Warning))
        .center_x(36)
        .center_y(36)
        .style(|t: &Theme| container::Style {
            background: Some(iced::Background::Color(
                Tokens::of(t).warning.scale_alpha(0.14),
            )),
            border: iced::border::rounded(18.0),
            ..container::Style::default()
        });
    let text = column![
        label::title(title),
        label::body(format!(
            "“{name}” içinde {state} değişiklikler var. Pencere kapanırsa bu değişiklikler kaybolur."
        )),
    ]
    .spacing(6)
    .width(Fill);
    let answer = |words: String, e: Event| {
        button(label::body(words))
            .padding([5, 14])
            .style(style::button::secondary)
            .on_press(ev(e))
    };
    let actions = row![
        answer(format!("{without} kapat"), Event::Discard),
        space::horizontal(),
        answer("Vazgeç".to_owned(), Event::Stay),
        button(label::body(format!("{save} ve kapat")).style(style::text::on_accent))
            .padding([5, 14])
            .style(style::button::primary)
            .on_press(ev(Event::SaveAndClose)),
    ]
    .spacing(6)
    .align_y(Center);
    container(column![row![symbol, text].spacing(14), actions].spacing(18))
        .width(typography::scaled(480.0))
        .padding(18)
        .style(style::container::popover)
        .into()
}
