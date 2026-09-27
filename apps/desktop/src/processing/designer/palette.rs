//! Model tasarımcısı's parts, the left column (the web's modelPalette):
//! the model inputs by type, and the tools by category with a search. A
//! click adds next to the selected box (and connects to it); a tool carried
//! out of the list is let go on the diagram.

use iced::widget::{Column, Row, button, column, container, row, text_input};
use iced::{Center, Element, Fill, Length};
use kentos_processing::designer::texts::palette as words;
use kentos_processing::model_edit::INPUT_TYPES;
use kentos_processing::registry::CategoryNode;
use kentos_processing::{Registry, Tool};
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::tree_view::{Column as TreeColumn, Node, TreeView};
use kentos_ui::widget::{Tip, horizontal_divider, tip};

use super::{Designer, Event};
use crate::app::Message;

fn ev(e: Event) -> Message {
    Message::ModelDesigner(e)
}

/// The parts' width (the web's 248 px, with the type size).
pub(super) const WIDTH: f32 = 248.0;

/// A row of the tool list: a category's name, or a tool.
enum Row_ {
    Group(&'static str),
    Tool(Box<Tool>),
}

/// The tool list as shown: categories flattened with their tools, only the
/// search's hits while one is typed.
fn rows(registry: &Registry, search: &str) -> Vec<Row_> {
    let query = search.trim();
    let hits: Option<Vec<String>> = (!query.is_empty()).then(|| {
        registry
            .search(query)
            .iter()
            .map(|t| t.id.clone())
            .collect()
    });
    let tree = registry.tree(&|t| hits.as_ref().is_none_or(|h| h.contains(&t.id)));
    fn flat(n: &CategoryNode, out: &mut Vec<Row_>) {
        out.push(Row_::Group(n.category.label));
        out.extend(n.tools.iter().cloned().map(|t| Row_::Tool(Box::new(t))));
        for child in &n.children {
            flat(child, out);
        }
    }
    let mut out = Vec::new();
    for n in &tree {
        flat(n, &mut out);
    }
    out
}

/// The tool at a place of the list (only tools have places), for a carry.
pub(super) fn tool_at(registry: &Registry, search: &str, place: usize) -> Option<String> {
    rows(registry, search)
        .into_iter()
        .filter_map(|r| match r {
            Row_::Tool(t) => Some(t.id),
            Row_::Group(_) => None,
        })
        .nth(place)
}

/// A column's title: small, bold, second tone, a line under it.
fn title<'a>(text: &'a str) -> Element<'a, Message> {
    column![
        label::caption(text)
            .font(typography::ui_strong())
            .style(style::text::muted),
        horizontal_divider(),
    ]
    .spacing(6)
    .into()
}

pub(super) fn palette<'a>(d: &'a Designer, registry: &'a Registry) -> Element<'a, Message> {
    // Girdi ekle: two to a row, the icon in the info blue.
    let mut grid = Column::new().spacing(4);
    for pair in INPUT_TYPES.chunks(2) {
        let mut line = Row::new().spacing(4);
        for kind in pair {
            let i = INPUT_TYPES
                .iter()
                .position(|t| t.type_name == kind.type_name)
                .unwrap_or(0);
            let face = row![
                container(icon(crate::icons::from_web(Some(kind.icon))).size(15.0)).style(
                    |theme: &iced::Theme| container::Style {
                        text_color: Some(Tokens::of(theme).info),
                        ..container::Style::default()
                    }
                ),
                label::body(kind.label).wrapping(iced::widget::text::Wrapping::None),
            ]
            .spacing(5)
            .align_y(Center);
            line = line.push(tip(
                // Left, and in the middle of the button's height (the web's).
                button(container(face).height(Fill).align_y(Center))
                    .padding([0, 7])
                    .height(Length::Fixed(typography::scaled(30.0)))
                    .width(Fill)
                    .style(style::button::secondary)
                    .on_press(ev(Event::AddInput(i))),
                Tip::new(kind.label).body(kind.description),
                iced::widget::tooltip::Position::Right,
            ));
        }
        grid = grid.push(line);
    }

    let search = container(
        row![
            icon(Icon::Search).size(14.0).tone(Tone::Muted),
            text_input(words::SEARCH, &d.search)
                .on_input(|t| ev(Event::Search(t)))
                .padding([4, 0])
                .size(typography::body())
                .style(style::field::bare_input),
        ]
        .spacing(6)
        .align_y(Center),
    )
    .padding([0, 8])
    .style(style::container::field_box);

    let mut place = 0usize;
    let nodes: Vec<Node<'a, Message>> = rows(registry, &d.search)
        .into_iter()
        .map(|r| match r {
            Row_::Group(name) => Node::new(name).heading(),
            Row_::Tool(t) => {
                let node = Node::new(t.label.clone())
                    .id(place)
                    .icon(icon(crate::icons::from_web(t.icon.as_deref())).size(15.0))
                    .on_press(ev(Event::AddTool {
                        tool: t.id.clone(),
                        at: None,
                    }));
                place += 1;
                node
            }
        })
        .collect();
    let list = TreeView::new([TreeColumn::new("Ad").width(Fill)])
        .header(false)
        .flat(true)
        .extend(nodes)
        .empty(words::EMPTY)
        .height(Fill)
        .on_carry(|place| ev(Event::Carry(place)));

    let body = column![
        title(words::INPUTS),
        grid,
        container(title(words::TOOLS)).padding(iced::Padding::ZERO.top(10.0)),
        search,
        list,
        label::caption(words::TIP).style(style::text::muted),
    ]
    .spacing(8)
    .padding(iced::Padding::new(12.0).right(10.0))
    .height(Fill);
    container(body)
        .width(Length::Fixed(typography::scaled(WIDTH)))
        .height(Fill)
        .style(style::container::header)
        .into()
}
