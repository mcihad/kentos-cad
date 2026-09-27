//! The flow's part of the builder window (DESIGN.md §7.16, docs/adr/0101):
//! the canvas with its zoom tools and notes, and the selected node's
//! inspector over the help, as the web's FlowView and FlowInspector.

use iced::widget::tooltip::Position as Beside;
use iced::widget::{Column, Stack, button, canvas, column, container, row, space, text_input};
use iced::{Alignment, Center, Element, Fill, Length};
use kentos_expression::editor::flow::{FlowNode, NodeKind, Type};
use kentos_expression::editor::{self as core, Kind};
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::theme::{Mode, typography};
use kentos_ui::widget::segmented::Segmented;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::switch::Switch;
use kentos_ui::widget::{Tip, tip};

use super::flow::FlowEvent;
use super::flow_canvas::FlowCanvas;
use super::highlight::Syntax;
use super::{Builder, Event, ev};
use crate::app::Message;

/// The inspector's value field: a double click on a value node gives it the keyboard.
pub(crate) const VALUE: &str = "expression-flow-value";

fn fev(e: FlowEvent) -> Message {
    ev(Event::Flow(e))
}

/// Operators that can take each other's place (the same two inputs, the same kind of answer).
const FAMILIES: [&[&str]; 3] = [
    &["=", "!=", "<", "<=", ">", ">="],
    &["+", "-", "*", "/", "%", "^"],
    &["ve", "veya"],
];

fn type_name(ty: Type) -> &'static str {
    match ty {
        Type::Any => "değer",
        Type::Number => "sayı",
        Type::Text => "metin",
        Type::Bool => "koşul",
    }
}

fn kind_name(kind: NodeKind) -> &'static str {
    match kind {
        NodeKind::Result => "Sonuç",
        NodeKind::Field => "Alan",
        NodeKind::Variable => "Değişken",
        NodeKind::Number => "Sayı",
        NodeKind::Text => "Metin",
        NodeKind::Constant => "Sabit",
        NodeKind::Function => "İşlev",
        NodeKind::Operator => "İşleç",
        NodeKind::Keyword => "Sözcük",
    }
}

/// The canvas, its zoom tools, and a note when it is empty or the text does not read.
pub(crate) fn area(b: &Builder, mode: Mode) -> Element<'_, Message> {
    let Some(flow) = &b.flow.flow else {
        return space().height(Fill).into();
    };
    let drawing = canvas(FlowCanvas {
        flow,
        view: b.flow.view,
        selected: b.flow.selected.as_deref(),
        values: &b.flow.values,
        mode,
        carrying: b.flow.carrying.as_deref(),
    })
    .width(Fill)
    .height(Fill);
    let tool = |glyph: Icon, words: &'static str, e: FlowEvent| {
        tip(
            button(icon(glyph).size(15.0))
                .padding([3, 5])
                .style(style::button::ghost)
                .on_press(fev(e)),
            Tip::new(words),
            Beside::Top,
        )
    };
    let tools = container(
        row![
            tool(Icon::ZoomOut, "Uzaklaş", FlowEvent::Zoom(0.8)),
            tool(Icon::ZoomIn, "Yakınlaş", FlowEvent::Zoom(1.25)),
            tool(Icon::ZoomExtents, "Tümünü göster", FlowEvent::Fit),
        ]
        .spacing(1),
    )
    .padding(2)
    .style(style::container::bordered);
    let mut layers = Stack::new().push(drawing).push(
        container(tools)
            .width(Fill)
            .height(Fill)
            .align_right(Fill)
            .align_bottom(Fill)
            .padding(8),
    );
    let note = if let Some(e) = &flow.error {
        Some((
            Icon::Error,
            Tone::Danger,
            format!(
                "İfade okunamıyor: {} Akışta göstermek için Metin'de düzeltin.",
                e.text()
            ),
        ))
    } else if flow.nodes.len() == 1 {
        Some((
            Icon::Info,
            Tone::Muted,
            "Ağaçtan bir öğeyi buraya sürükleyin ya da çift tıklayın; değerini Sonuç’un girişine bağlayın."
                .to_owned(),
        ))
    } else {
        None
    };
    if let Some((glyph, tone, words)) = note {
        layers = layers.push(
            container(
                container(
                    row![
                        icon(glyph).size(15.0).tone(tone),
                        label::caption(words).width(Fill)
                    ]
                    .spacing(8)
                    .align_y(Alignment::Start),
                )
                .padding([7, 10])
                .width(Fill)
                .style(style::container::bordered),
            )
            .padding(10)
            .width(Fill),
        );
    }
    container(layers)
        .width(Fill)
        .height(Fill)
        .style(style::container::field_box)
        .into()
}

/// The selected node's inspector (over the help), when a node is selected.
pub(crate) fn inspector(b: &Builder, mode: Mode) -> Option<Element<'_, Message>> {
    let n = b.flow.selected.as_deref().and_then(|s| b.flow.node(s))?;
    let syntax = Syntax::of(mode);
    let accent = match n.kind {
        NodeKind::Result => None,
        NodeKind::Field => Some(syntax.field),
        NodeKind::Variable => Some(syntax.variable),
        NodeKind::Number | NodeKind::Constant => Some(syntax.literal),
        NodeKind::Text => Some(syntax.text),
        NodeKind::Function => Some(syntax.function),
        NodeKind::Operator => Some(syntax.operator),
        NodeKind::Keyword => Some(syntax.keyword),
    };
    let mut title = label::mono(if n.kind == NodeKind::Result {
        "Sonuç".to_owned()
    } else {
        n.title.clone()
    })
    .font(typography::mono_strong());
    if let Some(c) = accent {
        title = title.color(c);
    }
    let mut out = Column::new().spacing(6).padding(14).width(Fill);
    out = out
        .push(label::caption(kind_name(n.kind)).style(style::text::muted))
        .push(
            row![title, pill(type_name(n.ty))]
                .spacing(8)
                .align_y(Center),
        );
    if let Some(editor) = editor(b, n) {
        out = out.push(editor);
    }
    if let Some(e) = &n.error {
        out = out.push(problem(Icon::Error, Tone::Danger, e.clone()));
    }
    for w in &n.warnings {
        out = out.push(problem(Icon::Warning, Tone::Warning, w.clone()));
    }
    if !n.ports.is_empty() {
        out = out.push(ports(n));
    }
    if n.kind == NodeKind::Result {
        out = out.push(
            label::caption(if n.text.is_empty() {
                "Bir düğümün çıkışını buraya bağlayın."
            } else {
                "Sonuç, girişine bağlı ifadenin değeridir; Tamam bu metni yazar."
            })
            .style(style::text::muted),
        );
    } else {
        out = out.push(
            row![
                space().width(Fill),
                button(
                    row![icon(Icon::Eraser).size(14.0), label::caption("Düğümü sil")]
                        .spacing(6)
                        .align_y(Center)
                )
                .padding([4, 10])
                .style(style::button::secondary)
                .on_press(fev(FlowEvent::Remove(n.id.clone()))),
            ]
            .align_y(Center),
        );
    }
    Some(out.into())
}

fn pill<'a>(words: &'a str) -> Element<'a, Message> {
    container(label::caption(words).style(style::text::muted))
        .padding([0, 6])
        .style(style::container::field_box)
        .into()
}

fn problem<'a>(glyph: Icon, tone: Tone, words: String) -> Element<'a, Message> {
    row![
        icon(glyph).size(14.0).tone(tone),
        label::caption(words).width(Fill)
    ]
    .spacing(6)
    .align_y(Alignment::Start)
    .into()
}

/// What can be changed in place.
fn editor<'a>(b: &'a Builder, n: &'a FlowNode) -> Option<Element<'a, Message>> {
    let labelled = |words: &'a str, content: Element<'a, Message>| -> Element<'a, Message> {
        column![label::caption(words).style(style::text::muted), content]
            .spacing(4)
            .into()
    };
    match n.kind {
        NodeKind::Number | NodeKind::Text => {
            let number = n.kind == NodeKind::Number;
            let mut col = Column::new().spacing(4).push(
                text_input(if number { "0" } else { "" }, &b.flow.draft)
                    .id(VALUE)
                    .on_input(|s| fev(FlowEvent::Draft(s)))
                    .on_submit(fev(FlowEvent::Commit))
                    .font(typography::mono())
                    .size(typography::body())
                    .padding([4, 8])
                    .style(style::field::input),
            );
            if let Some(e) = &b.flow.draft_error {
                col = col.push(label::caption(e.clone()).style(style::text::danger));
            } else {
                col = col.push(label::caption("Enter yazar.").style(style::text::muted));
            }
            Some(labelled(if number { "Sayı" } else { "Metin" }, col.into()))
        }
        NodeKind::Constant if n.title == "doğru" || n.title == "yanlış" => {
            let current = if n.title == "doğru" {
                "doğru"
            } else {
                "yanlış"
            };
            Some(labelled(
                "Değer",
                Segmented::new(["doğru", "yanlış"], current, |v: &str| {
                    fev(FlowEvent::SetBool(v == "doğru"))
                })
                .into(),
            ))
        }
        NodeKind::Field => {
            let mut names: Vec<String> = b.schema.fields.iter().map(|f| f.name.clone()).collect();
            if !names.contains(&n.title) {
                names.insert(0, n.title.clone());
            }
            let selected = names.iter().position(|f| *f == n.title);
            let choices: Vec<Choice> = names.iter().map(Choice::new).collect();
            Some(labelled(
                "Alan",
                Select::new(choices, selected, move |i| {
                    fev(FlowEvent::SetField(names[i].clone()))
                })
                .into(),
            ))
        }
        NodeKind::Variable => {
            let names: Vec<String> = core::catalog(&b.schema, "")
                .into_iter()
                .flat_map(|s| s.items)
                .filter(|i| i.kind == Kind::Variable)
                .map(|i| i.label)
                .collect();
            let selected = names.iter().position(|v| *v == n.title);
            let choices: Vec<Choice> = names.iter().map(Choice::new).collect();
            Some(labelled(
                "Değişken",
                Select::new(choices, selected, move |i| {
                    fev(FlowEvent::SetVariable(
                        names[i].trim_start_matches('$').to_owned(),
                    ))
                })
                .into(),
            ))
        }
        NodeKind::Function => {
            let names: Vec<(String, &'static str)> = core::catalog(&b.schema, "")
                .into_iter()
                .flat_map(|s| {
                    let group = s.title;
                    s.items
                        .into_iter()
                        .filter(|i| i.kind == Kind::Function)
                        .map(move |i| (i.label, group))
                })
                .collect();
            let selected = names.iter().position(|(f, _)| *f == n.title);
            let choices: Vec<Choice> = names
                .iter()
                .map(|(f, group)| Choice::new(f.clone()).detail(*group))
                .collect();
            Some(labelled(
                "İşlev",
                Select::new(choices, selected, move |i| {
                    fev(FlowEvent::SetFunction(names[i].0.clone()))
                })
                .into(),
            ))
        }
        _ => {
            if let Some(family) = FAMILIES.iter().find(|f| f.contains(&n.title.as_str())) {
                let current = family
                    .iter()
                    .copied()
                    .find(|s| *s == n.title)
                    .unwrap_or(family[0]);
                return Some(labelled(
                    "İşleç",
                    Segmented::new(family.iter().copied(), current, |s: &'static str| {
                        fev(FlowEvent::SetOperator(s))
                    })
                    .into(),
                ));
            }
            let negated = n.negated?;
            let like = matches!(n.key.as_deref(), Some("op:gibi" | "op:benzer"));
            let empty = matches!(n.key.as_deref(), Some("op:boş" | "op:boş değil"));
            let mut col = Column::new().spacing(8).push(
                Switch::new(negated, |v| fev(FlowEvent::SetNegated(v))).label(if empty {
                    "Boş değil mi (tersi)"
                } else {
                    "değil (tersi)"
                }),
            );
            if like {
                col = col.push(
                    Switch::new(n.key.as_deref() == Some("op:benzer"), |v| {
                        fev(FlowEvent::SetFold(v))
                    })
                    .label("Harf farkı gözetme (benzer)"),
                );
            }
            Some(col.into())
        }
    }
}

/// The node's inputs: what each takes and whether something is connected.
fn ports(n: &FlowNode) -> Element<'_, Message> {
    let mut list = Column::new().spacing(5);
    for (k, p) in n.ports.iter().enumerate() {
        let state = if p.from.is_some() {
            "bağlı"
        } else if p.optional {
            "isteğe bağlı, boş"
        } else {
            "boş"
        };
        let mut line = row![
            label::mono_caption(p.name.clone()),
            pill(type_name(p.ty)),
            if p.from.is_none() && !p.optional {
                label::caption(state).style(style::text::danger)
            } else {
                label::caption(state).style(style::text::muted)
            },
            space().width(Fill),
        ]
        .spacing(6)
        .align_y(Center);
        if p.removable {
            line = line.push(tip(
                button(icon(Icon::Close).size(12.0))
                    .padding([2, 4])
                    .style(style::button::ghost)
                    .on_press(fev(FlowEvent::RemovePort(n.id.clone(), k))),
                Tip::new("Bu girişi kaldır"),
                Beside::Left,
            ));
        }
        let mut item = Column::new().spacing(2).push(line);
        if let Some(note) = &p.note {
            item = item.push(label::caption(note.clone()).style(style::text::muted));
        }
        list = list.push(item);
    }
    let mut out = column![
        label::caption("Girişler")
            .font(typography::ui_strong())
            .style(style::text::muted),
        list
    ]
    .spacing(6);
    if n.grows {
        out = out.push(
            button(
                row![
                    icon(Icon::Plus).size(13.0),
                    label::caption(if n.key.as_deref() == Some("op:durum") {
                        "Koşul ekle"
                    } else {
                        "Giriş ekle"
                    })
                ]
                .spacing(6)
                .align_y(Center),
            )
            .padding([4, 10])
            .style(style::button::secondary)
            .on_press(fev(FlowEvent::AddPort(n.id.clone()))),
        );
    }
    container(out)
        .padding(iced::Padding {
            top: 6.0,
            ..iced::Padding::ZERO
        })
        .width(Length::Fill)
        .into()
}
