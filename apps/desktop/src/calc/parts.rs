//! The Hesap windows' shared parts, as the web's `common.ts` builds them
//! (docs/adr/0070, 0071): the known point field and its grid, the number
//! and text fields, the summary, the results table and the footer with the
//! layer the new points go to.

use iced::widget::text::IntoFragment;
use iced::widget::tooltip::Position;
use iced::widget::{Column, Row, button, column, container, row, text_input};
use iced::{Center, Element, Fill, Length};
use kentos_interaction::Format;
use kentos_ui::icon::{Icon, icon};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::theme::typography;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::{Dialog, Tip, tip};

use super::read::{self, Known};
use super::{Event, Field, MAX_HEIGHT, event};
use crate::app::Message;
use crate::exchange::words::{self, Kind as Line};

/// The layers points may go to, in tree order, as the layer select lists
/// them: id, name, locked.
pub(crate) fn leaves(model: &kentos_domain::Document) -> Vec<(String, String, bool)> {
    let layers = model.layers();
    layers
        .leaves()
        .into_iter()
        .map(|node| {
            (
                node.id.clone(),
                node.name.clone(),
                layers.is_locked(&node.id),
            )
        })
        .collect()
}

/// A known point field (the web's `knownField`): the text, Çizimden, and
/// under them what it resolves to: the point, the error, or the hint.
pub(crate) fn known_field<'a>(
    model: &kentos_domain::Document,
    format: &Format,
    title: &'a str,
    field: Field,
    text: &'a str,
    hint: Option<&'a str>,
) -> Element<'a, Message> {
    let input = text_input("Nokta adı ya da Y,X", text)
        .on_input(move |t| event(Event::Known(field, t)))
        .padding([5, 8])
        .width(Fill)
        .font(typography::ui())
        .size(typography::body())
        .style(style::field::input);
    let pick = tip(
        button(
            row![icon(Icon::Magnet).size(14.0), label::body("Çizimden")]
                .spacing(6)
                .align_y(Center),
        )
        .on_press(event(Event::Pick(field)))
        .padding([5, 10])
        .style(style::button::secondary),
        Tip::new("Çizimde gösterin (bir noktaya kenetlenirse adı alınır)"),
        Position::Bottom,
    );
    let resolved = match read::resolve_point(model, text) {
        Known::Empty => label::caption(hint.unwrap_or("Henüz verilmedi")),
        Known::Error(e) => label::caption(e).style(style::text::danger),
        Known::Point { p, .. } => label::caption(format.point(p)),
    };
    words::field(
        title,
        column![row![input, pick].spacing(6).align_y(Center), resolved].spacing(4),
        None,
    )
}

/// Known point fields in the web's grid (`calc-knowns`): three to a row at
/// these windows' widths, a short last row keeping its columns.
pub(crate) fn knowns<'a>(fields: Vec<Element<'a, Message>>) -> Element<'a, Message> {
    let mut rows = Column::new().spacing(12);
    let mut fields = fields.into_iter().peekable();
    while fields.peek().is_some() {
        let mut line = Row::new().spacing(18);
        for _ in 0..3 {
            line = line.push(match fields.next() {
                Some(field) => container(field).width(Fill).into(),
                None => Element::from(iced::widget::space().width(Fill)),
            });
        }
        rows = rows.push(line);
    }
    rows.into()
}

/// A number's field with its label (the web's `textField` with
/// `calc-num`: 130 px, the label as wide as it needs).
pub(crate) fn number_field<'a>(
    title: impl IntoFragment<'a>,
    value: &'a str,
    placeholder: &'a str,
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    labelled(title, value, placeholder, 130.0, true, on_input)
}

/// A text field with its label (the web's `textField`: 280 px).
pub(crate) fn text_field<'a>(
    title: impl IntoFragment<'a>,
    value: &'a str,
    placeholder: &'a str,
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    labelled(title, value, placeholder, 280.0, false, on_input)
}

fn labelled<'a>(
    title: impl IntoFragment<'a>,
    value: &'a str,
    placeholder: &'a str,
    width: f32,
    numeric: bool,
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    let input = text_input(placeholder, value)
        .on_input(on_input)
        .padding([5, 8])
        .width(Length::Fixed(typography::scaled(width)))
        .font(if numeric {
            typography::mono()
        } else {
            typography::ui()
        })
        .size(typography::body())
        .style(style::field::input);
    column![label::caption(title), input].spacing(4).into()
}

/// The summary box: at most six problems, or what was computed.
pub(crate) fn summary<'a>(lines: Vec<(Line, String)>) -> Option<Element<'a, Message>> {
    (!lines.is_empty()).then(|| {
        words::summary(
            lines
                .into_iter()
                .map(|(kind, t)| words::text_line(kind, t))
                .collect(),
        )
    })
}

/// A results table: a header and rows of text, numbers right-aligned.
pub(crate) fn result_table<'a>(
    head: &[&'a str],
    rows: Vec<Vec<String>>,
    numeric: &[bool],
) -> Element<'a, Message> {
    let cell = |t: String, n: bool, strong: bool| -> Element<'a, Message> {
        let text = if n { label::mono(t) } else { label::body(t) };
        let text = if strong {
            text.font(typography::ui_strong())
        } else {
            text
        };
        let c = container(text).width(Fill).padding([4, 8]);
        if n {
            c.align_x(iced::Right).into()
        } else {
            c.into()
        }
    };
    let head_row = row(head
        .iter()
        .zip(numeric)
        .map(|(h, &n)| cell((*h).to_owned(), n, true)));
    let mut table = Column::new().push(
        container(head_row)
            .width(Fill)
            .style(style::container::header),
    );
    for r in rows {
        table = table.push(row(r
            .into_iter()
            .zip(numeric)
            .map(|(t, &n)| cell(t, n, false))));
    }
    container(table)
        .width(Fill)
        .style(style::container::bordered)
        .into()
}

/// The layer select of the footer (the web's `layerChoice`): locked layers marked.
pub(crate) fn layer_select<'a>(
    model: &kentos_domain::Document,
    chosen: Option<&str>,
) -> Element<'a, Message> {
    let leaves = leaves(model);
    let selected = chosen.and_then(|chosen| leaves.iter().position(|(id, _, _)| id == chosen));
    let choices: Vec<Choice> = leaves
        .iter()
        .map(|(_, name, locked)| {
            Choice::new(if *locked {
                format!("{name} (kilitli)")
            } else {
                name.clone()
            })
        })
        .collect();
    container(Select::new(choices, selected, |i| event(Event::Layer(i))).searchable(false))
        .width(Length::Fixed(200.0))
        .into()
}

/// The footer's buttons as the web lays them out.
pub(crate) fn footer_button<'a>(
    caption: &'a str,
    on: Option<Message>,
    primary: bool,
) -> Element<'a, Message> {
    if primary {
        words::primary(caption, on)
    } else {
        words::secondary(caption, on)
    }
}

/// The footer of a window that adds points (the web's): Raporu kopyala,
/// the layer, Çizime ekle, Kapat; and the windows' height.
pub(crate) fn footer<'a>(
    dialog: Dialog<'a, Message>,
    model: &kentos_domain::Document,
    layer: Option<&str>,
    report: bool,
    add: bool,
) -> Dialog<'a, Message> {
    dialog
        .action(footer_button(
            "Raporu kopyala",
            report.then(|| event(Event::CopyReport)),
            false,
        ))
        .action(
            row![label::caption("Katman"), layer_select(model, layer)]
                .spacing(8)
                .align_y(Center),
        )
        .action(footer_button(
            "Çizime ekle",
            add.then(|| event(Event::AddPoints)),
            true,
        ))
        .action(footer_button("Kapat", Some(event(Event::Close)), false))
        .max_height(MAX_HEIGHT)
}
