//! The menus of the ribbon's own panels (ribbon_panels.rs): the active
//! layer's list, the object templates, the current properties' choices, the
//! plot scales, and the selection's kinds as the Seçim panel lists them.

use iced::widget::{column, container, row, space, text};
use iced::{Color, Element, Fill};
use kentos_contracts::LineType;
use kentos_ui::icon::Icon;
use kentos_ui::label;
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::Menu;

use super::{DRAW_COLORS, Event, LINE_TYPES, LINE_WEIGHTS, weight_text};
use crate::app::Message;

/// A line of the layer field's menu.
#[derive(Debug, Clone)]
pub(super) enum LayerLine {
    Header(String),
    Layer {
        id: String,
        name: String,
        color: Color,
        count: usize,
        active: bool,
        locked: bool,
    },
}

pub(super) fn layer_menu(lines: &[LayerLine]) -> Menu<Message> {
    lines.iter().fold(Menu::new(), |menu, line| match line {
        LayerLine::Header(path) => menu.header(path.clone()),
        LayerLine::Layer {
            id,
            name,
            color,
            count,
            active,
            locked,
        } => menu
            .radio(
                name.clone(),
                *active,
                (!*locked).then(|| Message::Layer(crate::layering::Event::Activate(id.clone()))),
            )
            .swatch(*color)
            .shortcut(count.to_string()),
    })
}

/// A row of the Şablonlar field's menu: a category's (or the recent ones')
/// header, or a template with its tool's icon and name.
#[derive(Debug, Clone)]
pub(super) enum TemplateLine {
    Header(String),
    Template {
        id: String,
        name: String,
        icon: Icon,
        tool: String,
        chosen: bool,
    },
}

/// The templates to draw with (the web's `templateField`); a choice draws
/// with it, the one being drawn with is marked.
pub(super) fn template_menu(lines: &[TemplateLine]) -> Menu<Message> {
    if lines.is_empty() {
        return Menu::new().item("Kitaplıkta nesne şablonu yok", None);
    }
    lines.iter().fold(Menu::new(), |menu, line| match line {
        TemplateLine::Header(label) => menu.header(label.clone()),
        TemplateLine::Template {
            id,
            name,
            icon,
            tool,
            chosen,
        } => menu
            .radio(name.clone(), *chosen, Message::DrawTemplate(id.clone()))
            .icon(*icon)
            .hint(tool.clone()),
    })
}

/// “Katmana göre”, then the choices (the web's `colorItems`, `lineTypeItems`, `weightItems`).
pub(super) fn color_menu(current: Option<&str>, swatches: &[Color]) -> Menu<Message> {
    let set = |color| Message::RibbonPanel(Event::Color(color));
    DRAW_COLORS.iter().zip(swatches).fold(
        Menu::new()
            .radio("Katmana göre", current.is_none(), set(None))
            .separator(),
        |menu, ((name, value), swatch)| {
            menu.radio(*name, current == Some(*value), set(Some(*value)))
                .swatch(*swatch)
        },
    )
}

pub(super) fn line_type_menu(current: Option<LineType>) -> Menu<Message> {
    let set = |t| Message::RibbonPanel(Event::LineType(t));
    LINE_TYPES.iter().fold(
        Menu::new()
            .radio("Katmana göre", current.is_none(), set(None))
            .separator(),
        |menu, (t, name)| menu.radio(*name, current == Some(*t), set(Some(*t))),
    )
}

pub(super) fn weight_menu(current: Option<f64>) -> Menu<Message> {
    let set = |w| Message::RibbonPanel(Event::Weight(w));
    LINE_WEIGHTS.iter().fold(
        Menu::new()
            .radio("Katmana göre", current.is_none(), set(None))
            .separator(),
        |menu, w| menu.radio(weight_text(*w), current == Some(*w), set(Some(*w))),
    )
}

/// Ölçek (docs/adr/0205 §4): the project's type's scales and the current
/// one when it is none of them, then Ölçek yaz….
pub(super) fn scale_menu(current: f64, cad: bool) -> Menu<Message> {
    kentos_project::wizard::project_scales(cad, current)
        .into_iter()
        .fold(Menu::new(), |menu, scale| {
            menu.radio(
                kentos_project::wizard::scale_text(scale),
                current == scale,
                Message::RibbonPanel(Event::Scale(scale)),
            )
        })
        .separator()
        .item(
            "Ölçek yaz…",
            Message::PlotScale(crate::annotation_scale::Event::Open),
        )
}

/// The kinds' column: up to three lines, else two and “n tür daha”, a line
/// before it (the web's `.rsel__kinds`).
pub(super) fn kinds_column(kinds: &[(usize, String)], width: f32) -> Element<'static, Message> {
    let shown = if kinds.len() > 3 { &kinds[..2] } else { kinds };
    let room = width - 12.0;
    let size = typography::body();
    let mut lines = column![].spacing(3);
    for (n, kind) in shown {
        let line = format!("{n} {kind}");
        lines = lines.push(
            text(typography::elide(&line, size, room).into_owned())
                .font(typography::ui())
                .size(size)
                .wrapping(iced::widget::text::Wrapping::None),
        );
    }
    if kinds.len() > 3 {
        lines = lines.push(label::muted(format!("{} tür daha", kinds.len() - 2)));
    }
    row![
        container(space::vertical())
            .width(1)
            .height(Fill)
            .style(|theme: &iced::Theme| container::Style {
                background: Some(Tokens::of(theme).border.scale_alpha(0.6).into()),
                ..container::Style::default()
            }),
        container(lines)
            .width(width - 1.0)
            .height(Fill)
            .padding([0, 6])
            .center_y(Fill),
    ]
    .height(Fill)
    .into()
}
