//! The rules of a rule-based layer style (the web's `rulesEditor.ts`, QGIS
//! “Kurala dayalı”): each rule has a condition (empty: every object), a
//! scale range, symbols and child rules that narrow it; “değilse” rules
//! catch what no sibling took. Every matching rule draws. Each rule says how
//! many objects it takes, among its parent's (`kentos_native_style::tally`).

use iced::widget::tooltip::Position;
use iced::widget::{Column, button, column, container, row, space};
use iced::{Center, Element, Fill, Length};
use kentos_native_style::renderer::{Rule, SymbolSet};
use kentos_native_style::tally::RuleCount;
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::theme::typography;
use kentos_ui::widget::{Tip, tip};

use super::widgets::{Env, check, ev, expression, help, input, number, problem, slots, tool};
use super::{Event, Field, LayerStyleWindow, RuleEdit, SetAt, Source, scale_key};
use crate::app::Message;

/// How far a child rule is set in, logical pixels (the web's 22).
const INDENT: f32 = 22.0;

pub(super) fn rules_panel<'a>(
    window: &LayerStyleWindow,
    env: &Env<'_>,
    src: &Source<'_>,
    fields: &[(String, usize)],
) -> Element<'a, Message> {
    let counts = window.rule_counts(src);
    let mut blocks = Vec::new();
    for (k, r) in window.rules.iter().enumerate() {
        block(
            &mut blocks,
            r,
            vec![k],
            window.rules.len(),
            window,
            env,
            &counts,
            fields,
        );
    }
    let list = Column::with_children(blocks).spacing(6);
    let add = button(
        row![icon(Icon::Plus).size(13.0), label::body("Kural ekle")]
            .spacing(5)
            .align_y(Center),
    )
    .padding(kentos_ui::theme::metrics::action())
    .style(style::button::secondary)
    .on_press(ev(Event::AddRule));
    let add_else = button(label::body("Değilse kuralı ekle"))
        .padding(kentos_ui::theme::metrics::action())
        .style(style::button::ghost)
        .on_press(ev(Event::AddElse));
    column![
        help("Koşulu sağlayan her kural çizer; alt kurallar üsttekinin nesnelerini daraltır. Ölçek aralığı boşsa her yakınlıkta görünür. Koşul örneği: Nitelik = 'Arsa' ve $alan > 500"),
        list,
        row![add, add_else].spacing(6),
    ]
    .spacing(10)
    .into()
}

#[allow(clippy::too_many_arguments)]
fn block<'a>(
    out: &mut Vec<Element<'a, Message>>,
    r: &Rule,
    path: Vec<usize>,
    siblings: usize,
    window: &LayerStyleWindow,
    env: &Env<'_>,
    counts: &std::collections::HashMap<Vec<usize>, RuleCount>,
    fields: &[(String, usize)],
) {
    let depth = path.len() - 1;
    let i = path[path.len() - 1];
    let edit = |e: RuleEdit| ev(Event::Rule(path.clone(), e));
    let on = r.enabled();
    let is_else = r.is_else();
    let empty = SymbolSet::default();
    let title = if r.label.is_empty() {
        "Kural".to_owned()
    } else {
        r.label.clone()
    };
    let label_path = path.clone();
    let name = input("Kural adı", &r.label)
        .on_input(move |t| ev(Event::Rule(label_path.clone(), RuleEdit::Label(t))))
        .width(Fill);
    let error = match counts.get(&path) {
        Some(RuleCount::Error(e)) => Some(e.clone()),
        _ => None,
    };
    let condition: Element<'a, Message> = if is_else {
        kentos_ui::widget::text_field("değilse: diğer kuralların almadıkları", "")
            .font(typography::mono())
            .style(style::field::validated(false))
            .width(Fill)
            .into()
    } else {
        expression(
            Field::Rule(path.clone()),
            &window.expr_text(&Field::Rule(path.clone())),
            "koşul yok: bütün nesneler",
            fields,
            error.is_some(),
            true,
        )
    };
    let else_box = row![
        check(is_else, edit(RuleEdit::Else(!is_else))),
        label::caption("değilse").style(style::text::muted)
    ]
    .spacing(5)
    .align_y(Center);
    let scale = |min: bool, value: Option<f64>| -> Element<'a, Message> {
        let text = window
            .typed
            .get(&scale_key(&path, min))
            .cloned()
            .unwrap_or_else(|| {
                value
                    .map(kentos_processing::text::js_number)
                    .unwrap_or_default()
            });
        let p = path.clone();
        let placeholder = if min { "en yakın" } else { "en uzak" };
        let typed = move |t: String| {
            ev(Event::Rule(
                p.clone(),
                if min {
                    RuleEdit::MinScale(t)
                } else {
                    RuleEdit::MaxScale(t)
                },
            ))
        };
        // Şimdiki: the view's scale as a whole N that keeps the view inside the rule's range, both of whose ends are
        // drawn (`StyledBatch::in_scale`): the nearest end the view's N or under, the farthest its N or over.
        let here = here_scale(env.view_scale, min).map(|n| typed(n.to_string()));
        row![
            number(placeholder, &text, 76.0, typed),
            tool("zoomSelection", "Şimdiki ölçek", here),
        ]
        .spacing(0)
        .align_y(Center)
        .into()
    };
    let (count_text, count_tip) = match counts.get(&path) {
        Some(RuleCount::Count(n)) => (
            n.to_string(),
            if is_else {
                "Değilse kuralının aldığı nesne: üsttekinin nesnelerinden hiçbir kardeş kuralın almadıkları".to_owned()
            } else if depth > 0 {
                "Koşulu sağlayan nesne, üst kuralın aldıkları arasından".to_owned()
            } else {
                "Koşulu sağlayan nesne".to_owned()
            },
        ),
        Some(RuleCount::Error(e)) => ("⚠".to_owned(), e.clone()),
        None => ("—".to_owned(), String::new()),
    };
    let count = tip(
        container(label::mono(count_text).style(if error.is_some() {
            style::text::danger
        } else {
            style::text::default
        }))
        .align_right(Length::Fixed(typography::scaled(40.0))),
        Tip::new(count_tip),
        Position::Top,
    );
    let tools = row![
        tool("chevronUp", "Yukarı", (i > 0).then(|| edit(RuleEdit::Up))),
        tool(
            "chevronDown",
            "Aşağı",
            (i + 1 < siblings).then(|| edit(RuleEdit::Down))
        ),
        tool("plus", "Alt kural ekle", Some(edit(RuleEdit::AddChild))),
        tool("trash", "Kuralı sil", Some(edit(RuleEdit::Remove))),
    ]
    .spacing(0)
    .align_y(Center);
    let head = container(
        row![
            check(on, edit(RuleEdit::Enabled(!on))),
            slots(
                env,
                SetAt::Rule(path.clone()),
                r.symbols.as_ref().unwrap_or(&empty),
                &title
            ),
            column![name, row![condition, else_box].spacing(8).align_y(Center)]
                .spacing(4)
                .width(Fill),
            row![
                label::caption("1:").style(style::text::muted),
                scale(true, r.min_scale),
                label::caption("– 1:").style(style::text::muted),
                scale(false, r.max_scale),
            ]
            .spacing(4)
            .align_y(Center),
            count,
            tools,
        ]
        .spacing(8)
        .align_y(Center),
    )
    .padding([6, 8])
    .width(Fill)
    .style(style::container::bordered);
    let indent = typography::scaled(INDENT * depth as f32);
    let mut part = column![row![space().width(indent), head]].spacing(4);
    if let Some(e) = error {
        part = part.push(row![space().width(indent + 8.0), problem(e)]);
    }
    if !on {
        part = part.push(row![
            space().width(indent + 8.0),
            row![
                icon(Icon::Info).size(12.0).tone(Tone::Muted),
                label::caption("Kapalı: bu kural ve alt kuralları çizmez.")
                    .style(style::text::muted)
            ]
            .spacing(5)
            .align_y(Center)
        ]);
    }
    out.push(part.into());
    let children = r.children();
    for (k, child) in children.iter().enumerate() {
        let mut p = path.clone();
        p.push(k);
        block(out, child, p, children.len(), window, env, counts, fields);
    }
}

/// A rule's end set to the view's scale 1:`n` (Şimdiki): a whole N the view stays inside of, both ends being drawn;
/// none for a scale that is not one.
fn here_scale(n: f64, nearest: bool) -> Option<u64> {
    (n.is_finite() && n > 0.0).then(|| {
        if nearest {
            n.floor().max(1.0) as u64
        } else {
            n.ceil() as u64
        }
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn simdiki_keeps_the_view_inside_the_rules_range() {
        // A rule's range draws both its ends (`StyledBatch::in_scale`): at 1:435.4 the nearest end is 1:435, the
        // farthest 1:436; a whole 1:500 is itself.
        assert_eq!(super::here_scale(435.4, true), Some(435));
        assert_eq!(super::here_scale(435.4, false), Some(436));
        assert_eq!(super::here_scale(500.0, true), Some(500));
        assert_eq!(super::here_scale(500.0, false), Some(500));
        assert_eq!(super::here_scale(0.4, true), Some(1));
        assert_eq!(super::here_scale(f64::NAN, true), None);
    }
}
