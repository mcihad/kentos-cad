//! What the exchange windows share (the web's `ui/io/common.ts`): the
//! picked file's line, labelled fields, summary lines, a report item as a
//! sentence, object counts by kind and where the data lies.

use iced::widget::{Column, button, column, container, row, text};
use iced::{Center, Element, Fill};
use kentos_contracts::{Bounds, ReportItem};
use kentos_interaction::Format;
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::style;
use kentos_ui::theme::typography;
use kentos_ui::widget::tree_view::Check;
use kentos_ui::{label, widget::tree_view};

use crate::view::kind_name;

/// A summary line's mark: a green check, an amber warning, an info mark or a red error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Ok,
    Warn,
    Info,
    Error,
}

/// A line of the summary box.
pub fn line<'a, Message: 'a>(
    kind: Kind,
    content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    let (glyph, tone) = match kind {
        Kind::Ok => (Icon::Success, Tone::Success),
        Kind::Warn => (Icon::Warning, Tone::Warning),
        Kind::Info => (Icon::Info, Tone::Muted),
        Kind::Error => (Icon::Error, Tone::Danger),
    };
    row![
        container(icon(glyph).size(14.0).tone(tone)).padding([2, 0]),
        container(content).width(Fill)
    ]
    .spacing(8)
    .into()
}

/// A summary line with plain text.
pub fn text_line<'a, Message: 'a>(kind: Kind, words: impl Into<String>) -> Element<'a, Message> {
    line(kind, label::body(words.into()))
}

/// The summary box: its lines one under the other.
pub fn summary<'a, Message: 'a>(lines: Vec<Element<'a, Message>>) -> Element<'a, Message> {
    container(Column::with_children(lines).spacing(6))
        .padding([8, 10])
        .width(Fill)
        .style(style::container::bordered)
        .into()
}

/// The picked file: its name and a line of facts.
pub fn file_line<'a, Message: 'a>(name: &str, meta: String) -> Element<'a, Message> {
    row![
        icon(Icon::Open).size(18.0).tone(Tone::Accent),
        column![
            text(name.to_owned())
                .font(typography::ui_strong())
                .size(typography::body()),
            label::caption(meta),
        ]
        .spacing(2),
    ]
    .spacing(10)
    .align_y(Center)
    .into()
}

/// Label above, control below; an optional hint line under it.
pub fn field<'a, Message: 'a>(
    title: &'a str,
    control: impl Into<Element<'a, Message>>,
    hint: Option<String>,
) -> Element<'a, Message> {
    let mut parts = column![label::caption(title), control.into()].spacing(4);
    if let Some(hint) = hint {
        parts = parts.push(label::caption(hint));
    }
    parts.into()
}

/// A check box with its text (the web's `checkField` without its label).
pub fn check<'a, Message: Clone + 'a>(
    checked: bool,
    words: &'a str,
    on_toggle: Option<Message>,
) -> Element<'a, Message> {
    let state = if checked {
        Check::Checked
    } else {
        Check::Unchecked
    };
    let boxed = tree_view::check_box(state, on_toggle.clone());
    let face = row![boxed, label::body(words)].spacing(8).align_y(Center);
    button(face)
        .on_press_maybe(on_toggle)
        .padding(0)
        .style(style::button::ghost)
        .into()
}

/// A report item as a sentence (“IMAGE: 2, raster görüntüler alınmaz (satır 120, 488).”).
pub fn report_text(i: &ReportItem) -> String {
    let lines = if i.lines.is_empty() {
        String::new()
    } else {
        let listed: Vec<String> = i.lines.iter().map(u32::to_string).collect();
        let more = if i.count as usize > i.lines.len() {
            " …"
        } else {
            ""
        };
        format!(" (satır {}{more})", listed.join(", "))
    };
    format!("{}: {}, {}{lines}.", i.what, i.count, i.reason)
}

/// Report items as summary lines, at most `max` and a line for the rest.
pub fn report_lines<'a, Message: 'a>(
    items: &[ReportItem],
    kind: Kind,
    max: usize,
) -> Vec<Element<'a, Message>> {
    let mut lines: Vec<Element<'a, Message>> = items
        .iter()
        .take(max)
        .map(|i| text_line(kind, report_text(i)))
        .collect();
    if items.len() > max {
        lines.push(text_line(
            kind,
            format!("… ve {} başka kalem.", items.len() - max),
        ));
    }
    lines
}

/// Object counts by kind, largest first (a stable sort of the order they came
/// in), without plurals: “12 çizgi, 3 yay”.
pub fn kind_counts(counts: &[(&str, u32)]) -> String {
    let mut sorted = counts.to_vec();
    sorted.sort_by_key(|&(_, n)| std::cmp::Reverse(n));
    sorted
        .iter()
        .map(|(kind, n)| format!("{n} {}", kind_name(kind)))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Adds one of `kind` to counts kept in the order kinds first come.
pub fn count(counts: &mut Vec<(&'static str, u32)>, kind: &'static str) {
    match counts.iter_mut().find(|(k, _)| *k == kind) {
        Some((_, n)) => *n += 1,
        None => counts.push((kind, 1)),
    }
}

/// Where the data lies, in the project's units (“Kapsam: Y (sağa) 452 345.123 – …”).
pub fn extent_text(format: &Format, b: &Bounds) -> String {
    format!(
        "Kapsam: Y (sağa) {} – {}, X (yukarı) {} – {}.",
        format.coord(b.min_x),
        format.coord(b.max_x),
        format.coord(b.min_y),
        format.coord(b.max_y)
    )
}

/// A name without its extension (“noktalar.ncn” → “noktalar”).
pub fn base_name(name: &str) -> String {
    match name.rfind('.') {
        Some(dot) if dot > 0 => name[..dot].to_owned(),
        _ => name.to_owned(),
    }
}

/// An empty table's line.
pub fn empty<'a, Message: 'a>(words: &'a str) -> Element<'a, Message> {
    container(label::muted(words))
        .padding(12)
        .width(Fill)
        .style(style::container::bordered)
        .into()
}

/// The window's main button.
pub fn primary<'a, Message: Clone + 'a>(
    caption: &'a str,
    on: Option<Message>,
) -> Element<'a, Message> {
    button(label::body(caption))
        .on_press_maybe(on)
        .padding([5, 16])
        .style(style::button::primary)
        .into()
}

pub fn secondary<'a, Message: Clone + 'a>(
    caption: &'a str,
    on: Option<Message>,
) -> Element<'a, Message> {
    button(label::body(caption))
        .on_press_maybe(on)
        .padding([5, 16])
        .style(style::button::secondary)
        .into()
}

/// A quiet button at the start of the footer (“Başka dosya…”).
pub fn ghost<'a, Message: Clone + 'a>(
    caption: &'a str,
    on: Option<Message>,
) -> Element<'a, Message> {
    button(label::body(caption))
        .on_press_maybe(on)
        .padding([5, 12])
        .style(style::button::ghost)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_items_read_as_the_webs_sentences() {
        let item = ReportItem {
            what: "IMAGE".into(),
            count: 3,
            reason: "raster görüntüler alınmaz".into(),
            lines: vec![120, 488],
        };
        assert_eq!(
            report_text(&item),
            "IMAGE: 3, raster görüntüler alınmaz (satır 120, 488 …)."
        );
        let plain = ReportItem {
            lines: Vec::new(),
            count: 1,
            ..item
        };
        assert_eq!(report_text(&plain), "IMAGE: 1, raster görüntüler alınmaz.");
    }

    #[test]
    fn counts_go_largest_first_and_keep_their_order_otherwise() {
        let mut counts = Vec::new();
        for kind in ["line", "arc", "line", "point", "arc", "line"] {
            count(&mut counts, kind);
        }
        count(&mut counts, "text");
        assert_eq!(kind_counts(&counts), "3 çizgi, 2 yay, 1 nokta, 1 yazı");
        assert_eq!(base_name("noktalar.ncn"), "noktalar");
        assert_eq!(base_name(".gizli"), ".gizli");
    }
}
