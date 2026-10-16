//! Koşullu biçim (docs/adr/0413 §4, `data.tableStyles`): a layer's rules for the attribute table.
//!
//! ```text
//! ┌ Koşullu biçim ───────────────────────────────────────────────────────────┐
//! │ Katman [Bina                ▾]                                            │
//! │ Kurallar               │ Uygulandığı yer [Bütün satır          ▾]        │
//! │ [Aa] Durum = 'kotu'    │ Koşul [Durum = 'kotu'               ] [ε]       │
//! │ [Aa] Kat > 5 · Kat     │ ☑ Zemin rengi  [■]                              │
//! │ [Yeni kural]           │ ☑ Yazı rengi   [■]                              │
//! │ [↑] [↓]                │ ☐ Kalın                                         │
//! │ [Kuralı sil]                                       [Vazgeç] [Kaydet]      │
//! └──────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! The window edits a copy of the layer's rules; Kaydet writes them through `cad.layers.table` in one
//! step and closes the window. The rules' order is theirs: the first that holds gives each of the
//! background, the colour and bold.

use iced::widget::{Column, Row, button, column, container, row, scrollable, space, text_input};
use iced::{Center, Color, Element, Fill, Task, Theme};
use kentos_contracts::{LayerNodeType, LayersTable, TableStyle, table_styles_problem};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, layers_table};
use kentos_ui::icon::icon;
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::color::{ColorPicker, parse_hex, to_hex};
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::{Dialog as Frame, Elided, focus_ring, overlay};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::icons::from_web;
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Koşullu biçim";
const SAVE: &str = "Kaydet";
const CANCEL: &str = "Vazgeç";
const NEW: &str = "Yeni kural";
const REMOVE: &str = "Kuralı sil";
const UP: &str = "Yukarı";
const DOWN: &str = "Aşağı";
const ROW: &str = "Bütün satır";
/// The colours a new rule starts with: a soft warning background, its dark text.
const NEW_BACKGROUND: &str = "#FFF4E5";
const NEW_COLOR: &str = "#9A3412";

/// The window.
#[derive(Debug, Clone, Default)]
pub struct Window {
    /// The layer whose rules these are.
    pub layer: String,
    /// The copy being edited and the rules as the layer has them.
    pub rows: Vec<TableStyle>,
    kept: Vec<TableStyle>,
    pub chosen: Option<usize>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Layer(String),
    Choose(usize),
    New,
    Remove,
    Up,
    Down,
    /// Where the chosen rule paints: a field by name, none the whole row.
    Field(Option<String>),
    Condition(String),
    Builder,
    Background(Option<String>),
    Color(Option<String>),
    Bold(bool),
    Save,
    Cancel,
}

fn msg(event: Event) -> Message {
    Message::TableStyles(event)
}

impl App {
    /// Koşullu biçim: the window on the rules of the table's layer (the active layer when the table
    /// shows none).
    pub(crate) fn open_table_styles(&mut self) -> Task<Message> {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return Task::none();
        };
        let layers = self.table_style_layers();
        let wanted = self
            .features
            .layer
            .clone()
            .or_else(|| Some(doc.model.layers().active().to_owned()));
        let Some(layer) = wanted
            .filter(|l| layers.iter().any(|(id, _)| id == l))
            .or_else(|| layers.first().map(|(id, _)| id.clone()))
        else {
            self.warn("Çizimde nesne tutan katman yok; koşullu biçim bir katmanın tablosunundur.");
            return Task::none();
        };
        self.table_styles = Some(self.table_styles_window_of(layer));
        self.dialog = Some(Dialog::TableStyles);
        Task::none()
    }

    fn table_styles_window_of(&self, layer: String) -> Window {
        let rows = self
            .document
            .as_ref()
            .and_then(|d| d.model.layers().get(&layer))
            .map(|n| n.table_styles.clone())
            .unwrap_or_default();
        Window {
            layer,
            chosen: (!rows.is_empty()).then_some(0),
            kept: rows.clone(),
            rows,
        }
    }

    pub(crate) fn table_styles_event(&mut self, event: Event) -> Task<Message> {
        if let Event::Layer(id) = &event {
            self.table_styles = Some(self.table_styles_window_of(id.clone()));
            return Task::none();
        }
        if matches!(event, Event::Builder) {
            return self.open_builder_for_table_style();
        }
        let Some(w) = self.table_styles.as_mut() else {
            return Task::none();
        };
        let at = w.chosen.filter(|i| *i < w.rows.len());
        let chosen = at.and_then(|i| w.rows.get_mut(i));
        match event {
            Event::Layer(_) | Event::Builder => {}
            Event::Choose(i) => w.chosen = Some(i),
            Event::New => {
                w.rows.push(TableStyle {
                    field: None,
                    condition: String::new(),
                    background: Some(NEW_BACKGROUND.into()),
                    color: Some(NEW_COLOR.into()),
                    bold: false,
                });
                w.chosen = Some(w.rows.len() - 1);
            }
            Event::Remove => {
                if let Some(i) = at {
                    w.rows.remove(i);
                    w.chosen = (!w.rows.is_empty()).then_some(i.min(w.rows.len() - 1));
                }
            }
            Event::Up => {
                if let Some(i) = at.filter(|i| *i > 0) {
                    w.rows.swap(i, i - 1);
                    w.chosen = Some(i - 1);
                }
            }
            Event::Down => {
                if let Some(i) = at.filter(|i| i + 1 < w.rows.len()) {
                    w.rows.swap(i, i + 1);
                    w.chosen = Some(i + 1);
                }
            }
            Event::Field(f) => {
                if let Some(r) = chosen {
                    r.field = f;
                }
            }
            Event::Condition(text) => {
                if let Some(r) = chosen {
                    r.condition = text;
                }
            }
            Event::Background(c) => {
                if let Some(r) = chosen {
                    r.background = c;
                }
            }
            Event::Color(c) => {
                if let Some(r) = chosen {
                    r.color = c;
                }
            }
            Event::Bold(on) => {
                if let Some(r) = chosen {
                    r.bold = on;
                }
            }
            Event::Cancel => {
                self.table_styles = None;
                self.dialog = None;
            }
            Event::Save => return self.save_table_styles(),
        }
        Task::none()
    }

    /// ε: the expression builder on the chosen rule's condition, the layer's fields and objects.
    fn open_builder_for_table_style(&mut self) -> Task<Message> {
        let Some(w) = &self.table_styles else {
            return Task::none();
        };
        let Some(i) = w.chosen.filter(|i| *i < w.rows.len()) else {
            return Task::none();
        };
        let (layer, value) = (w.layer.clone(), w.rows[i].condition.clone());
        self.open_builder_for_table_rule(i, &layer, &value)
    }

    /// The builder's Tamam: the rule's new condition.
    pub(crate) fn table_rule_written(&mut self, at: usize, text: String) -> Task<Message> {
        if let Some(r) = self.table_styles.as_mut().and_then(|w| w.rows.get_mut(at)) {
            r.condition = text;
        }
        Task::none()
    }

    /// Kaydet: the rules through `cad.layers.table`, in one step; the window closes when they go in.
    fn save_table_styles(&mut self) -> Task<Message> {
        let Some(w) = self.table_styles.clone() else {
            return Task::none();
        };
        if w.rows == w.kept {
            self.table_styles = None;
            self.dialog = None;
            return Task::none();
        }
        let Some(doc) = self.document.as_mut() else {
            return Task::none();
        };
        let result = layers_table::execute(
            &mut ExecutionContext::new(&mut doc.model),
            LayersTable {
                layer: w.layer.clone(),
                styles: w.rows.clone(),
                expected_revision: None,
            },
        );
        if self.answered(result).is_none() {
            return Task::none();
        }
        self.table_styles = None;
        self.dialog = None;
        let name = self.form_layer_name(&w.layer);
        self.say(
            Level::Success,
            format!("“{name}” katmanının koşullu biçimi kaydedildi."),
        );
        Task::none()
    }

    /// The layers that hold objects, by their path.
    fn table_style_layers(&self) -> Vec<(String, String)> {
        let Some(doc) = &self.document else {
            return Vec::new();
        };
        let mut out = Vec::new();
        fn walk(nodes: &[kentos_contracts::LayerNode], at: &str, out: &mut Vec<(String, String)>) {
            for n in nodes {
                let path = if at.is_empty() {
                    n.name.clone()
                } else {
                    format!("{at} / {}", n.name)
                };
                if n.kind == LayerNodeType::Layer && n.service.is_none() {
                    out.push((n.id.clone(), path.clone()));
                }
                walk(&n.children, &path, out);
            }
        }
        walk(doc.model.layers().nodes(), "", &mut out);
        out
    }

    pub(crate) fn table_styles_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.table_styles else {
            return iced::widget::text("").into();
        };
        let layers = self.table_style_layers();
        let ids: Vec<String> = layers.iter().map(|(id, _)| id.clone()).collect();
        let layer = Select::new(
            layers.iter().map(|(_, n)| Choice::new(n.clone())),
            ids.iter().position(|id| *id == w.layer),
            move |i| msg(Event::Layer(ids[i].clone())),
        );
        let fields = self.layer_fields_of(&w.layer);
        let mut list = Column::new().spacing(1).padding(2);
        for (i, r) in w.rows.iter().enumerate() {
            let place = match &r.field {
                None => ROW.to_owned(),
                Some(f) => fields
                    .iter()
                    .find(|x| x.name == *f)
                    .map_or(f.clone(), |x| x.label().to_owned()),
            };
            let condition = if r.condition.trim().is_empty() {
                "(koşul yok)".to_owned()
            } else {
                r.condition.clone()
            };
            let face = row![
                sample(r),
                column![
                    Elided::new(condition)
                        .size(typography::body())
                        .font(typography::ui())
                        .width(Fill),
                    label::caption(place).style(muted),
                ]
                .spacing(1)
                .width(Fill),
            ]
            .spacing(8)
            .align_y(Center);
            list = list.push(
                button(face)
                    .on_press(msg(Event::Choose(i)))
                    .padding([4, 8])
                    .width(Fill)
                    .style(style::button::row(w.chosen == Some(i))),
            );
        }
        if w.rows.is_empty() {
            list = list.push(
                label::caption(
                    "Bu katmanın tablosunda kural yok. Yeni kural ile koşula uyan satırları ya da hücreleri boyayın.",
                )
                .style(muted),
            );
        }
        let at = w.chosen.filter(|i| *i < w.rows.len());
        let tool = |glyph: &'static str, tip: &'static str, on: Option<Message>| {
            kentos_ui::widget::tip(
                button(icon(from_web(Some(glyph))).size(16.0))
                    .style(style::button::ghost)
                    .padding(4)
                    .on_press_maybe(on),
                kentos_ui::widget::Tip::new(tip),
                iced::widget::tooltip::Position::Top,
            )
        };
        let left = column![
            label::strong("Kurallar"),
            container(scrollable(list).height(typography::scaled(260.0)))
                .style(style::container::field_box)
                .width(Fill),
            row![
                words::secondary(NEW, Some(msg(Event::New))),
                space::horizontal().width(Fill),
                tool(
                    "chevronUp",
                    "Yukarı taşı",
                    at.filter(|i| *i > 0).map(|_| msg(Event::Up))
                ),
                tool(
                    "chevronDown",
                    "Aşağı taşı",
                    at.filter(|i| i + 1 < w.rows.len())
                        .map(|_| msg(Event::Down))
                ),
            ]
            .spacing(4)
            .align_y(Center),
        ]
        .spacing(8);
        let right: Element<'_, Message> = match at.map(|i| (i, &w.rows[i])) {
            None => container(label::caption(
                "Bir kural seçin ya da yeni bir kural oluşturun.",
            ))
            .padding(12)
            .into(),
            Some((i, r)) => self.table_style_editor(i, r, &fields),
        };
        let problem = table_styles_problem(&w.rows).map(|(_, why)| why);
        let mut body = column![
            words::field("Katman", container(layer).width(320), None),
            row![container(left).width(280), container(right).width(Fill)].spacing(16),
        ]
        .spacing(14);
        if let Some(why) = &problem {
            body = body.push(words::text_line(Kind::Warn, why.clone()));
        }
        body = body.push(
            row![
                words::ghost(REMOVE, at.map(|_| msg(Event::Remove))),
                space::horizontal().width(Fill),
                words::secondary(CANCEL, Some(msg(Event::Cancel))),
                words::primary(SAVE, problem.is_none().then(|| msg(Event::Save))),
            ]
            .spacing(8)
            .align_y(Center),
        );
        overlay::modal(
            Frame::new(TITLE).push(body).width(780.0),
            msg(Event::Cancel),
        )
    }

    fn table_style_editor<'a>(
        &'a self,
        at: usize,
        r: &TableStyle,
        fields: &[kentos_contracts::LayerField],
    ) -> Element<'a, Message> {
        let names: Vec<String> = fields.iter().map(|f| f.name.clone()).collect();
        let mut choices = vec![Choice::new(ROW)];
        choices.extend(fields.iter().map(|f| Choice::new(f.label().to_owned())));
        let chosen = match &r.field {
            None => Some(0),
            Some(f) => names.iter().position(|n| n == f).map(|i| i + 1),
        };
        let place = Select::new(choices, chosen, move |i| {
            msg(Event::Field(i.checked_sub(1).map(|i| names[i].clone())))
        });
        let condition = text_input("Durum = 'kotu'", &r.condition)
            .size(typography::body())
            .on_input(|t| msg(Event::Condition(t)))
            .padding([5, 8])
            .style(style::field::validated(r.condition.trim().is_empty()));
        let builder = kentos_ui::widget::tip(
            button(icon(from_web(Some("expression"))).size(16.0))
                .style(style::button::ghost)
                .padding(4)
                .on_press(msg(Event::Builder)),
            kentos_ui::widget::Tip::new("İfade oluşturucu…"),
            iced::widget::tooltip::Position::Top,
        );
        let why = self.table_condition_problem(r);
        let mut col = Column::new()
            .spacing(10)
            .push(words::field(
                "Uygulandığı yer",
                place,
                Some(
                    "Bütün satır her hücreyi boyar; bir alan yalnız onun hücresini, satırın kurallarından önce."
                        .into(),
                ),
            ))
            .push(words::field(
                "Koşul",
                row![container(focus_ring(condition)).width(Fill), builder]
                    .spacing(4)
                    .align_y(Center),
                None,
            ));
        if let Some(why) = why {
            col = col.push(words::text_line(Kind::Warn, why));
        }
        col.push(paint_row(
            "Zemin rengi",
            r.background.as_deref(),
            NEW_BACKGROUND,
            |c| msg(Event::Background(c)),
        ))
        .push(paint_row(
            "Yazı rengi",
            r.color.as_deref(),
            NEW_COLOR,
            |c| msg(Event::Color(c)),
        ))
        .push(words::check(
            r.bold,
            "Kalın",
            Some(msg(Event::Bold(!r.bold))),
        ))
        .push(
            row![label::caption("Önizleme").style(muted), sample(r)]
                .spacing(8)
                .align_y(Center),
        )
        .push(label::caption(format!("{}. kural", at + 1)).style(muted))
        .into()
    }

    /// Why the chosen rule's condition cannot be read, in the project's reading unit; none when it can.
    fn table_condition_problem(&self, r: &TableStyle) -> Option<String> {
        let doc = self.document.as_ref()?;
        if r.condition.trim().is_empty() {
            return None;
        }
        let unit = kentos_native_application::expression_unit::reading_unit(doc.model.settings());
        layers_table::compile_condition(&r.condition, unit).err()
    }

    /// The window's controls by their words (a trace's `dialog` step).
    pub(crate) fn table_styles_control(
        &self,
        control: Control<'_>,
    ) -> Result<Option<Message>, String> {
        if self.table_styles.is_none() {
            return Err(format!("{TITLE} penceresi açık değil"));
        }
        Ok(match control {
            Control::Press(SAVE) => Some(msg(Event::Save)),
            Control::Press(CANCEL) => Some(msg(Event::Cancel)),
            Control::Press(NEW) => Some(msg(Event::New)),
            Control::Press(REMOVE) => Some(msg(Event::Remove)),
            Control::Press(UP) => Some(msg(Event::Up)),
            Control::Press(DOWN) => Some(msg(Event::Down)),
            Control::Press("Kalın") => {
                let on = self
                    .table_styles
                    .as_ref()
                    .and_then(|w| w.chosen.and_then(|i| w.rows.get(i)))
                    .is_some_and(|r| r.bold);
                Some(msg(Event::Bold(!on)))
            }
            Control::Fill("Koşul", text) => Some(msg(Event::Condition(text.to_owned()))),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}

/// A rule's look on a small “Aa 12” sample.
fn sample<'a>(r: &TableStyle) -> Element<'a, Message> {
    let colour = |hex: Option<&str>| hex.and_then(parse_hex);
    let background = colour(r.background.as_deref());
    let text_colour =
        colour(r.color.as_deref()).or_else(|| background.map(crate::features::view::readable_on));
    let words = if r.bold {
        label::strong("Aa 12")
    } else {
        label::body("Aa 12")
    };
    let words = match text_colour {
        Some(c) => words.style(move |_: &Theme| iced::widget::text::Style { color: Some(c) }),
        None => words,
    };
    container(words)
        .padding([2, 6])
        .style(move |t: &Theme| container::Style {
            background: background.map(iced::Background::Color),
            border: iced::Border {
                color: Tokens::of(t).border,
                width: 1.0,
                radius: kentos_ui::theme::shape::radius(3.0).into(),
            },
            ..container::Style::default()
        })
        .into()
}

/// A colour of the rule: its check (given or not) and, when given, the swatch that opens the picker.
fn paint_row<'a>(
    words: &'a str,
    value: Option<&str>,
    start: &'static str,
    on: impl Fn(Option<String>) -> Message + Clone + 'a,
) -> Element<'a, Message> {
    let given = value.is_some();
    let toggle = (on.clone())(if given { None } else { Some(start.to_owned()) });
    let mut r = Row::new()
        .spacing(8)
        .align_y(Center)
        .push(container(words::check(given, words, Some(toggle))).width(typography::scaled(140.0)));
    if let Some(c) = value.and_then(parse_hex) {
        let size = typography::scaled(22.0);
        let face = container(space())
            .width(size * 2.0)
            .height(size)
            .style(move |t: &Theme| container::Style {
                background: Some(iced::Background::Color(c)),
                border: iced::Border {
                    color: Tokens::of(t).muted,
                    width: 1.0,
                    radius: kentos_ui::theme::shape::radius(3.0).into(),
                },
                ..container::Style::default()
            });
        let picker = ColorPicker::new(c, move |c: Color| on(Some(to_hex(c)))).anchor(face);
        r = r.push(picker).push(label::caption(to_hex(c)).style(muted));
    }
    r.into()
}

fn muted(theme: &Theme) -> iced::widget::text::Style {
    iced::widget::text::Style {
        color: Some(Tokens::of(theme).muted),
    }
}
