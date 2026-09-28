//! The builder's window (DESIGN.md §7.16), as the web's: the operators, the
//! editor, the call's signature, the error or warning and the preview on the
//! left; the searchable tree in the middle; the help on the right; Vazgeç
//! and Tamam under them. Metin | Akış over the operators: in Akış the flow
//! of the same text stands in the editor's place, the tree is its palette
//! and the selected node's inspector stands over the help (docs/adr/0101).

use iced::widget::tooltip::Position as Beside;
use iced::widget::{
    Column, Row, button, column, container, row, scrollable, space, text, text_input,
};
use iced::{Alignment, Center, Element, Fill, Length};
use kentos_expression::editor::{self as core, Kind, Section};
use kentos_expression::{FieldSource, FieldType};
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::theme::{Mode, typography};
use kentos_ui::widget::tabs::{Tab, Tabs};
use kentos_ui::widget::tree_view::{Column as TreeColumn, Node, TreeView};
use kentos_ui::widget::{Dialog, Tip, horizontal_divider, overlay, tip};

use super::highlight::Syntax;
use super::{Builder, Event, Preview, ViewMode, editor, ev, flow_view, values_section};
use crate::app::{App, Message};

/// The window's width and highest height (at 12 px text; KentOS UI scales them).
const WIDTH: f32 = 980.0;
const HEIGHT: f32 = 700.0;
/// In Akış the window takes more of the screen: a flow is wider than a line of text.
const FLOW_WIDTH: f32 = 1240.0;
const FLOW_HEIGHT: f32 = 800.0;
/// The tree's and the help's columns.
const TREE: f32 = 220.0;
const HELP: f32 = 270.0;

/// The operator buttons over the editor: label, what goes in, how it stands.
const OPERATORS: [(&str, &str, Kind); 15] = [
    ("=", " = ", Kind::Operator),
    ("!=", " != ", Kind::Operator),
    ("<", " < ", Kind::Operator),
    (">", " > ", Kind::Operator),
    ("+", " + ", Kind::Operator),
    ("-", " - ", Kind::Operator),
    ("*", " * ", Kind::Operator),
    ("/", " / ", Kind::Operator),
    ("^", " ^ ", Kind::Operator),
    ("||", " || ", Kind::Operator),
    ("(", "(", Kind::Operator),
    (")", ")", Kind::Operator),
    ("ve", " ve ", Kind::Keyword),
    ("veya", " veya ", Kind::Keyword),
    ("değil", " değil ", Kind::Keyword),
];

impl App {
    /// The builder over the window that opened it, while it is open and not picking.
    pub(crate) fn builder_view(&self) -> Option<Element<'_, Message>> {
        let b = self.builder.as_ref().filter(|b| !b.is_picking())?;
        let flow = b.view_mode == ViewMode::Flow;
        let help_column: Element<'_, Message> =
            match flow_view::inspector(b, self.mode).filter(|_| flow) {
                Some(inspector) => column![
                    container(scrollable(inspector).direction(style::field::body_scrollbar()))
                        .max_height(typography::scaled(360.0)),
                    horizontal_divider(),
                    scrollable(help(b, self.mode))
                        .direction(style::field::body_scrollbar())
                        .height(Fill),
                ]
                .height(Fill)
                .into(),
                None => scrollable(help(b, self.mode))
                    .direction(style::field::body_scrollbar())
                    .height(Fill)
                    .into(),
            };
        let body = row![
            main(b, self.mode),
            container(tree(b, self.mode))
                .width(Length::Fixed(typography::scaled(TREE)))
                .height(Fill)
                .style(style::container::header),
            container(help_column)
                .width(Length::Fixed(typography::scaled(HELP)))
                .height(Fill)
                .style(style::container::header),
        ]
        .spacing(12)
        .height(Fill);
        let title = if b.context.is_empty() {
            "İfade oluşturucu".to_owned()
        } else {
            format!("İfade oluşturucu · {}", b.context)
        };
        Some(overlay::modal(
            Dialog::new(title)
                .push(body)
                .push(horizontal_divider())
                .push(footer(b))
                .width(if flow { FLOW_WIDTH } else { WIDTH })
                .max_height(if flow { FLOW_HEIGHT } else { HEIGHT }),
            ev(Event::Cancel),
        ))
    }
}

/// The left column: Metin | Akış, operators, editor (or the flow),
/// signature, status and preview.
fn main(b: &Builder, mode: Mode) -> Element<'_, Message> {
    let syntax = Syntax::of(mode);
    let flow = b.view_mode == ViewMode::Flow;
    let tabs = Tabs::new(
        [
            Tab::new("Metin")
                .icon(crate::icons::from_web(Some("expression")))
                .closable(false),
            Tab::new("Akış")
                .icon(crate::icons::from_web(Some("modelNew")))
                .closable(false),
        ],
        usize::from(flow),
        |i| {
            ev(Event::Mode(if i == 0 {
                ViewMode::Text
            } else {
                ViewMode::Flow
            }))
        },
    );
    let ops = OPERATORS
        .iter()
        .fold(Row::new().spacing(3), |ops, &(face, insert, kind)| {
            let color = if kind == Kind::Keyword {
                syntax.keyword
            } else {
                syntax.operator
            };
            let tip_text = core::help(&format!("op:{face}"), &b.schema)
                .map_or_else(|| "Parantez".to_owned(), |h| h.description);
            ops.push(tip(
                button(
                    text(face)
                        .font(typography::mono())
                        .size(typography::body())
                        .color(color),
                )
                .padding([2, 7])
                .style(style::button::secondary)
                // The parentheses are the text's: the flow draws its own.
                .on_press_maybe(
                    (!flow || face != "(" && face != ")")
                        .then(|| ev(Event::Operator(face, insert, kind))),
                ),
                Tip::new(tip_text),
                Beside::Bottom,
            ))
        });
    let mut out = Column::new()
        .spacing(8)
        .width(Fill)
        .height(Fill)
        .push(tabs)
        .push(ops.wrap());
    out = if flow {
        out.push(container(flow_view::area(b, mode)).height(Fill))
    } else {
        out.push(container(editor::view(b, mode)).height(Fill))
            .push(signature(b))
    };
    out = out.push(status(b));
    out.push(preview(b)).into()
}

/// The call the cursor is in: its signature, the argument being written in
/// 600, and what that argument is.
fn signature(b: &Builder) -> Element<'_, Message> {
    let Some(s) = &b.signature else {
        return space().height(typography::scaled(18.0)).into();
    };
    let units: Vec<u16> = s.signature.encode_utf16().collect();
    let part = |from: usize, to: usize| {
        String::from_utf16_lossy(&units[from.min(units.len())..to.min(units.len())])
    };
    let mut face = Row::new();
    let arg = s.active.and_then(|i| s.args.get(i));
    match arg {
        Some(a) => {
            face = face
                .push(label::mono_caption(part(0, a.start)).style(style::text::muted))
                .push(label::mono_caption(part(a.start, a.end)).font(typography::mono_strong()))
                .push(label::mono_caption(part(a.end, units.len())).style(style::text::muted));
        }
        None => face = face.push(label::mono_caption(s.signature).style(style::text::muted)),
    }
    let note = match arg {
        Some(a) => format!(
            "{}{}: {}",
            a.name,
            if a.optional { " (isteğe bağlı)" } else { "" },
            a.description
        ),
        None => s.description.to_owned(),
    };
    row![
        face,
        // One line: a long description ends in “…” (the web's ellipsis).
        kentos_ui::widget::Elided::new(note)
            .size(typography::caption())
            .style(style::text::muted)
            .width(Fill),
    ]
    .spacing(10)
    .align_y(Center)
    .height(typography::scaled(18.0))
    .into()
}

/// The error (red, with its place) or the first warning; in Akış a change
/// the core refused, until the next change.
fn status(b: &Builder) -> Element<'_, Message> {
    if let Some(failed) = b
        .flow
        .failed
        .as_ref()
        .filter(|_| b.view_mode == ViewMode::Flow)
    {
        return row![
            icon(Icon::Error).size(14.0).tone(Tone::Danger),
            label::caption(failed.clone())
        ]
        .spacing(6)
        .align_y(Alignment::Start)
        .into();
    }
    let line = |glyph: Icon, tone: Tone, words: String| -> Element<'_, Message> {
        row![icon(glyph).size(14.0).tone(tone), label::caption(words)]
            .spacing(6)
            .align_y(Alignment::Start)
            .into()
    };
    match (&b.check.error, b.check.warnings.first()) {
        (Some(e), _) if !b.source.trim().is_empty() => line(Icon::Error, Tone::Danger, e.text()),
        (_, Some(w)) => {
            let more = b.check.warnings.len() - 1;
            let words = if more > 0 {
                format!("{} ({more} uyarı daha)", w.text())
            } else {
                w.text()
            };
            line(Icon::Warning, Tone::Warning, words)
        }
        _ => space().height(typography::scaled(16.0)).into(),
    }
}

/// The preview on one object and the stepper: ‹ n / N ›, the object's name, Sahneden seç.
fn preview(b: &Builder) -> Element<'_, Message> {
    let value: Element<'_, Message> = match &b.preview {
        Preview::Value(v) => label::mono(v.clone())
            .wrapping(iced::widget::text::Wrapping::None)
            .into(),
        Preview::Note(n) => label::caption(n.clone()).style(style::text::muted).into(),
    };
    let mut out = row![
        label::caption("Önizleme").style(style::text::muted),
        container(value).width(Fill).clip(true),
    ]
    .spacing(10)
    .align_y(Center);
    let n = b.objects.len();
    if n > 0 {
        let step = |glyph: Icon, by: i32, words: &'static str| {
            tip(
                button(icon(glyph).size(14.0))
                    .padding([2, 4])
                    .style(style::button::ghost)
                    .on_press_maybe((n > 1).then(|| ev(Event::Object(by)))),
                Tip::new(words),
                Beside::Top,
            )
        };
        out = out
            .push(label::caption("Nesne").style(style::text::muted))
            .push(step(Icon::ChevronLeft, -1, "Önceki nesne"))
            .push(label::mono_caption(format!("{} / {n}", b.index + 1)))
            .push(step(Icon::ChevronRight, 1, "Sonraki nesne"))
            .push(tip(
                button(icon(Icon::Target).size(14.0))
                    .padding([2, 4])
                    .style(style::button::ghost)
                    .on_press(ev(Event::Pick)),
                Tip::new("Sahneden seç").body("Önizlemenin nesnesini çizimde tıklayın."),
                Beside::Top,
            ))
            .push(
                container(
                    label::caption(b.described.clone())
                        .style(style::text::muted)
                        .wrapping(iced::widget::text::Wrapping::None),
                )
                .max_width(typography::scaled(140.0))
                .clip(true),
            );
    }
    container(out)
        .padding([6, 10])
        .width(Fill)
        .style(style::container::bordered)
        .into()
}

/// A group of the tree as it is drawn: its title and entries.
struct Shown {
    title: String,
    items: Vec<core::Item>,
}

/// The tree: the search and the groups with their entries.
fn tree(b: &Builder, mode: Mode) -> Element<'_, Message> {
    let syntax = Syntax::of(mode);
    let search = container(
        row![
            icon(Icon::Search).size(13.0).tone(Tone::Muted),
            text_input("Ara: alan, işlev, değişken…", &b.query)
                .on_input(|q| ev(Event::Query(q)))
                .on_submit(ev(Event::QuerySubmit))
                .font(typography::ui())
                .size(typography::body())
                .padding([3, 0])
                .style(style::field::bare_input),
        ]
        .spacing(6)
        .align_y(Center),
    )
    .padding([0, 8])
    .style(style::container::field_box);
    let searching = !b.query.trim().is_empty();
    let flow = b.view_mode == ViewMode::Flow;
    // Akış' palette: the values to write first; rows carry their place in the
    // palette (`Builder::palette`), so one can be carried out onto the flow.
    let mut groups: Vec<(String, String, Vec<core::Item>)> = Vec::new();
    if flow {
        let values = values_section(&b.query);
        if !values.is_empty() {
            groups.push(("values".to_owned(), "Sabit değerler".to_owned(), values));
        }
    }
    let sections: Vec<Section> = core::catalog(&b.schema, &b.query);
    groups.extend(
        sections
            .into_iter()
            .map(|s| (s.group.id().to_owned(), s.title.to_owned(), s.items)),
    );
    let mut place = 0usize;
    let roots = groups.into_iter().map(|(id, title, items)| {
        let open = searching || id == "values" || b.open.contains(&id);
        let count = items.len();
        let s = Shown { title, items };
        let mut node = Node::new(s.title)
            .folder()
            .cells([label::mono_caption(count.to_string())
                .style(style::text::muted)
                .into()])
            .expanded(open, ev(Event::Toggle(id)));
        if open {
            for item in s.items {
                let color = match item.kind {
                    Kind::Field => syntax.field,
                    Kind::Variable => syntax.variable,
                    Kind::Function => syntax.function,
                    Kind::Keyword => syntax.keyword,
                    Kind::Operator => syntax.operator,
                };
                let glyph = match item.kind {
                    Kind::Field => "▦",
                    Kind::Variable => "$",
                    Kind::Function => "ƒ",
                    Kind::Keyword | Kind::Operator => "≡",
                };
                let detail: Element<'_, Message> = match item.kind {
                    Kind::Field => {
                        label::caption(item.detail.split(" · ").next().unwrap_or("").to_owned())
                            .style(style::text::muted)
                            .into()
                    }
                    _ => space().into(),
                };
                let selected = b.chosen.as_deref() == Some(item.key.as_str());
                let color = match item.key.as_str() {
                    "lit:number" => syntax.literal,
                    "lit:text" => syntax.text,
                    _ => color,
                };
                let glyph = match item.key.as_str() {
                    "lit:number" => "#",
                    "lit:text" => "'",
                    _ => glyph,
                };
                node = node.push(
                    Node::new(item.label.clone())
                        .id(place)
                        .icon(
                            text(glyph)
                                .font(typography::mono())
                                .size(typography::caption())
                                .color(color),
                        )
                        .cells([detail])
                        .selected(selected)
                        .on_press(ev(Event::Row(item.key.clone()))),
                );
                place += 1;
            }
        }
        node
    });
    let empty = format!("“{}” için öğe yok.", b.query.trim());
    let mut list = TreeView::new([
        TreeColumn::new("Ad").width(Fill),
        TreeColumn::new("Tür").width(56).align_right(),
    ])
    .header(false)
    .extend(roots.collect::<Vec<_>>())
    .empty(empty)
    .height(Fill);
    if flow {
        list = list.on_carry(|place| ev(Event::Carry(place)));
    }
    let room = iced::Padding {
        top: 10.0,
        right: 8.0,
        bottom: 4.0,
        left: 8.0,
    };
    column![container(search).padding(room), list]
        .spacing(4)
        .height(Fill)
        .into()
}

/// The help of what is chosen, highlighted or under the cursor; a field's values.
fn help(b: &Builder, mode: Mode) -> Element<'_, Message> {
    let Some(h) = &b.help else {
        if b.view_mode == ViewMode::Flow {
            return column![
                label::strong("Akış"),
                label::body("İfadenin düğümleri: değer bir düğümün çıkışından (sağ) ötekinin girişine (sol) gider; sonuç sağdadır."),
                label::caption("Ağaçtan bir öğeyi sürükleyin ya da çift tıklayın: düğüm olur. Çıkışı bir girişe sürükleyin: bağlanır; girişin noktasını çekin: ayrılır. Düğüme tıklayın: burada düzenlenir; Delete siler, Ctrl+Z geri alır. Tekerlek yakınlaştırır.")
                    .style(style::text::muted),
            ]
            .spacing(8)
            .padding(14)
            .into();
        }
        return column![
            label::strong("Yardım"),
            label::body("Ağaçtan bir öğe seçin ya da yazmaya başlayın: imlecin üstündeki adın yardımı burada görünür."),
            label::caption("Ctrl+Boşluk önerileri açar. Çift tık öğeyi imlecin yerine ekler. Ctrl+Enter Tamam.")
                .style(style::text::muted),
        ]
        .spacing(8)
        .padding(14)
        .into();
    };
    let syntax = Syntax::of(mode);
    let mut out = Column::new().spacing(6).padding(14).width(Fill);
    out = out
        .push(label::caption(h.group).style(style::text::muted))
        .push(label::heading(h.title.clone()).font(typography::ui_strong()))
        .push(
            container(colored(&h.signature, &syntax))
                .padding([5, 8])
                .width(Fill)
                .style(style::container::field_box),
        )
        .push(label::body(h.description.clone()));
    if let Some((ty, source)) = h.field {
        let ty = match ty {
            FieldType::Text => "metin",
            FieldType::Number => "sayı",
            FieldType::Bool => "doğru/yanlış",
            FieldType::Date => "tarih",
        };
        let source = match source {
            FieldSource::Attribute => "öznitelik",
            FieldSource::User => "kullanıcının tanımladığı alan",
            FieldSource::Builtin => "yerleşik",
        };
        out = out.push(label::caption(format!("Türü: {ty}; {source}.")).style(style::text::muted));
    }
    if !h.args.is_empty() {
        let mut args = Column::new().spacing(4);
        for a in &h.args {
            let mut name = Column::new().push(label::mono_caption(a.name.clone()));
            if a.optional {
                name = name.push(label::caption("isteğe bağlı").style(style::text::muted));
            }
            args = args.push(
                row![
                    container(name).width(Length::Fixed(typography::scaled(78.0))),
                    label::caption(a.description.clone())
                        .style(style::text::muted)
                        .width(Fill),
                ]
                .spacing(8),
            );
        }
        out = out.push(heading("Argümanlar")).push(args);
    }
    if !h.examples.is_empty() {
        let mut list = Column::new().spacing(6);
        for (expr, result) in &h.examples {
            list = list.push(column![
                colored(expr, &syntax),
                label::caption(format!("→ {result}")).style(style::text::muted),
            ]);
        }
        out = out.push(heading("Örnekler")).push(list);
    }
    if !h.aliases.is_empty() {
        out = out.push(
            label::caption(format!("Öbür adları: {}", h.aliases.join(", ")))
                .style(style::text::muted),
        );
    }
    if h.field.is_some() {
        out = out.push(values(b));
    }
    out.into()
}

fn heading(words: &str) -> Element<'_, Message> {
    container(
        label::caption(words)
            .font(typography::ui_strong())
            .style(style::text::muted),
    )
    .padding(iced::Padding {
        top: 8.0,
        ..iced::Padding::ZERO
    })
    .into()
}

/// An expression in the syntax colours (the help's signatures and examples).
fn colored<'a>(src: &str, syntax: &Syntax) -> Element<'a, Message> {
    let units: Vec<u16> = src.encode_utf16().collect();
    let piece = |a: usize, b: usize| String::from_utf16_lossy(&units[a..b]);
    let mut spans: Vec<iced::widget::text::Span<'a>> = Vec::new();
    let mut at = 0;
    for t in core::tokens(src) {
        if t.start > at {
            spans.push(iced::widget::span(piece(at, t.start)));
        }
        let color = match t.class {
            core::Class::Number | core::Class::Constant => syntax.literal,
            core::Class::Text => syntax.text,
            core::Class::Field => syntax.field,
            core::Class::Variable => syntax.variable,
            core::Class::Function => syntax.function,
            core::Class::Keyword => syntax.keyword,
            _ => syntax.operator,
        };
        spans.push(iced::widget::span(piece(t.start, t.end)).color(color));
        at = t.end;
    }
    if at < units.len() {
        spans.push(iced::widget::span(piece(at, units.len())));
    }
    iced::widget::rich_text(spans)
        .font(typography::mono())
        .size(typography::caption())
        .into()
}

/// A field's values: the two buttons, then the list.
fn values(b: &Builder) -> Element<'_, Message> {
    let any = !b.objects.is_empty();
    let small = |words: &'static str, all: bool| {
        button(label::caption(words))
            .padding([3, 8])
            .style(style::button::secondary)
            .on_press_maybe(any.then(|| ev(Event::Values(all))))
    };
    let mut out = column![
        heading("Değerler"),
        row![
            small("Örnek değerler (10)", false),
            small("Tüm değerler", true)
        ]
        .spacing(6),
    ]
    .spacing(6);
    match &b.listed {
        Some(listed) if !listed.items.is_empty() => {
            let rows = listed
                .items
                .iter()
                .enumerate()
                .fold(Column::new(), |rows, (i, v)| {
                    rows.push(
                        button(
                            label::mono_caption(v.text.clone())
                                .wrapping(iced::widget::text::Wrapping::None),
                        )
                        .padding([3, 8])
                        .width(Fill)
                        .style(style::button::list_item(listed.chosen == Some(i)))
                        .on_press(ev(Event::Value(i))),
                    )
                });
            let shown = listed.items.len();
            out = out
                .push(
                    container(
                        scrollable(rows)
                            .direction(style::field::body_scrollbar())
                            .height(Length::Fixed(
                                typography::scaled(24.0) * shown.min(8) as f32 + 6.0,
                            )),
                    )
                    .padding(2)
                    .style(style::container::field_box),
                )
                .push(
                    label::caption(if listed.total > shown {
                        format!(
                            "{} değer; ilk {shown}'i listelendi. Çift tık ifadeye ekler.",
                            listed.total
                        )
                    } else {
                        format!("{shown} değer. Çift tık ifadeye ekler.")
                    })
                    .style(style::text::muted),
                );
        }
        Some(_) => {
            out = out
                .push(label::caption("Bu alanın nesnelerde değeri yok.").style(style::text::muted))
        }
        None if !any => {
            out = out.push(
                label::caption("Değerleri göstermek için nesne yok.").style(style::text::muted),
            )
        }
        None => {}
    }
    out.into()
}

/// The keys note, Vazgeç and Tamam (off while the text has an error).
fn footer(b: &Builder) -> Element<'_, Message> {
    let blocked = b.check.error.is_some() && !b.source.trim().is_empty();
    let keys = if b.view_mode == ViewMode::Flow {
        "Sürükle: bağla · Delete: düğümü sil · Ctrl+Z: geri al · Ctrl+Enter: Tamam"
    } else {
        "Ctrl+Boşluk: öneriler · Ctrl+Enter: Tamam"
    };
    row![
        label::caption(keys).style(style::text::muted).width(Fill),
        button(label::body("Vazgeç"))
            .padding([5, 14])
            .style(style::button::secondary)
            .on_press(ev(Event::Cancel)),
        button(label::body("Tamam"))
            .padding([5, 14])
            .style(style::button::primary)
            .on_press_maybe((!blocked).then(|| ev(Event::Ok))),
    ]
    .spacing(10)
    .align_y(Center)
    .into()
}
