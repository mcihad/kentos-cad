//! The designer's layer list (the web's `renderList` and the left column):
//! Katmanlar and Katman ekle's menu, a row per layer (a marker's layers
//! under the layer that places them), the list's tools with Geri al and
//! Yinele, and the drawing order's note.

use iced::widget::tooltip::Position;
use iced::widget::{Column, button, column, container, row, scrollable, space};
use iced::{Border, Center, Color, Element, Fill, Theme};
use kentos_native_style::designer::{
    self as model, LayerPath, add_parent, can_move, can_remove, has_marker, label as layer_label,
    layer_types, summary, texts, type_of,
};
use kentos_ui::icon::icon;
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::tree_view::{Check, check_box};
use kentos_ui::widget::{Menu, MenuButton, Tip, horizontal_divider, tip};
use serde_json::Value;

use super::parts;
use super::{Designer, Event, ev};
use crate::app::Message;

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

/// The left: Katmanlar and Katman ekle, the rows, the tools and the order's note.
pub(super) fn layer_list(d: &Designer) -> Element<'_, Message> {
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
        label::caption(summary(l))
            .size(size)
            .style(style::text::muted),
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
            radius: kentos_ui::theme::shape::radius(4.0).into(),
        },
        ..button::Style::default()
    }
}
