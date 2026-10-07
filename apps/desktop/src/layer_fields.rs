//! Alanlar (docs/adr/0199 §3; the web's `ui/layers/LayerFieldsDialog.ts`): a
//! layer's fields in a table, each its name, alias, kind, length or
//! decimals, whether it is required, its default, its range and its value
//! list (Liste… opens the codes and labels). Alan ekle, Sil, Yukarı, Aşağı;
//! Verilerden al adds a field for each key the layer's objects carry that the
//! table has not, of the first kind all its values take. The list's first
//! problem is said under the table and Kaydet waits for it. Kaydet writes the
//! fields and the renamed fields' keys on the objects as one undo step
//! (“Alanlar”, `Document::set_layer_fields`); the values that do not keep the
//! new rules are counted.
//!
//! The fields change the layer tree: a cloud project without `project.edit`
//! refuses them ([`crate::layering::TREE_LOCKED`]).

use iced::widget::{Column, button, container, row, text, text_input};
use iced::{Center, Element, Fill, Length, Task};
use kentos_contracts::{
    FieldChoice, LayerField, LayerFieldKind, LayerNodeType, check_value, fields_problem,
};
use kentos_interaction::Level;
use kentos_ui::icon::icon;
use kentos_ui::theme::typography;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::tree_view::{Check, check_box};
use kentos_ui::widget::{Dialog as Frame, overlay};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::icons::from_web;
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const FIELDS_TITLE: &str = "Alanlar";
const SAVE: &str = "Kaydet";
const CANCEL: &str = "Vazgeç";
const ADD: &str = "Alan ekle";
const REMOVE: &str = "Sil";
const UP: &str = "Yukarı";
const DOWN: &str = "Aşağı";
const FROM_DATA: &str = "Verilerden al";

/// The table's columns: header and width (logical pixels).
const HEADS: [(&str, f32); 9] = [
    ("Ad", 130.0),
    ("Takma ad", 120.0),
    ("Tür", 130.0),
    ("Uzunluk / Basamak", 80.0),
    ("Zorunlu", 60.0),
    ("Varsayılan", 100.0),
    ("En az", 80.0),
    ("En çok", 80.0),
    ("Değer listesi", 100.0),
];

/// A field as the window edits it: every value as typed, and the name it had
/// when the window opened (none: new). The web's `FieldRow`.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldRow {
    pub from: Option<String>,
    pub name: String,
    pub alias: String,
    pub kind: LayerFieldKind,
    /// A text's length or a decimal's fraction digits, as typed.
    pub size: String,
    pub required: bool,
    pub default: String,
    pub min: String,
    pub max: String,
    pub values: Option<Vec<FieldChoice>>,
}

/// A field as the window shows it (the web's `rowOf`).
pub fn row_of(f: &LayerField) -> FieldRow {
    let size = match f.kind {
        LayerFieldKind::Text => f.length.map(|n| n.to_string()),
        LayerFieldKind::Decimal => f.scale.map(|n| n.to_string()),
        _ => None,
    };
    FieldRow {
        from: Some(f.name.clone()),
        name: f.name.clone(),
        alias: f.alias.clone().unwrap_or_default(),
        kind: f.kind,
        size: size.unwrap_or_default(),
        required: f.required,
        default: f.default.clone().unwrap_or_default(),
        min: f.min.clone().unwrap_or_default(),
        max: f.max.clone().unwrap_or_default(),
        values: f.values.clone(),
    }
}

/// A size as typed: none blank, else its number; a text that is no number is
/// `Err` (the window says so).
fn count(text: &str) -> Result<Option<u32>, ()> {
    let t = text.trim();
    if t.is_empty() {
        return Ok(None);
    }
    if !t.bytes().all(|b| b.is_ascii_digit()) {
        return Err(());
    }
    Ok(Some(t.parse::<u32>().unwrap_or(u32::MAX)))
}

/// The field a row writes: what is typed, a kind's own parts only (the web's `fieldOf`).
pub fn field_of(r: &FieldRow) -> LayerField {
    let mut f = LayerField::new(r.name.clone(), r.kind);
    if !r.alias.trim().is_empty() {
        f.alias = Some(r.alias.clone());
    }
    let size = count(&r.size).ok().flatten();
    match r.kind {
        LayerFieldKind::Text => f.length = size,
        LayerFieldKind::Decimal => f.scale = size,
        _ => {}
    }
    if r.kind.is_number() {
        if !r.min.trim().is_empty() {
            f.min = Some(r.min.clone());
        }
        if !r.max.trim().is_empty() {
            f.max = Some(r.max.clone());
        }
    }
    if matches!(
        r.kind,
        LayerFieldKind::Text | LayerFieldKind::Integer | LayerFieldKind::Decimal
    ) {
        f.values = r.values.clone();
    }
    f.required = r.required;
    if !r.default.trim().is_empty() {
        f.default = Some(r.default.clone());
    }
    f
}

/// The renamed fields' keys: (old, new) for each field kept under another
/// name (the web's `renamesOf`).
pub fn renames_of(rows: &[FieldRow]) -> Vec<(String, String)> {
    rows.iter()
        .filter_map(|r| {
            r.from
                .as_ref()
                .filter(|from| **from != r.name)
                .map(|from| (from.clone(), r.name.clone()))
        })
        .collect()
}

/// The rows' first problem: a size that is no number, else the fields'.
pub fn problem_of(rows: &[FieldRow]) -> Option<String> {
    for r in rows {
        if count(&r.size).is_err() {
            let what = if r.kind == LayerFieldKind::Text {
                "uzunluğu"
            } else {
                "ondalık basamağı"
            };
            return Some(format!("“{}” alanının {what} bir tam sayı olmalı.", r.name));
        }
    }
    let fields: Vec<LayerField> = rows.iter().map(field_of).collect();
    fields_problem(&fields)
}

/// The value list being edited: its row and its codes and labels.
#[derive(Debug, Clone)]
pub struct Choices {
    row: usize,
    list: Vec<(String, String)>,
}

/// The window: the layer, its rows, the fields as saved, the chosen row and
/// what it last said; the value list's window over it.
#[derive(Debug, Clone)]
pub struct Window {
    layer: String,
    rows: Vec<FieldRow>,
    saved: Vec<FieldRow>,
    chosen: Option<usize>,
    said: Option<(Kind, String)>,
    choices: Option<Choices>,
}

/// A cell of a row typed into.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cell {
    Name,
    Alias,
    Size,
    Default,
    Min,
    Max,
}

#[derive(Debug, Clone)]
pub enum Event {
    /// A row's cell typed into.
    Type(usize, Cell, String),
    /// A row's kind chosen (by its place in the list).
    Kind(usize, usize),
    Required(usize, bool),
    /// A row pressed: it is the one Sil, Yukarı and Aşağı take.
    Choose(usize),
    Add,
    Remove,
    Up,
    Down,
    FromData,
    Save,
    Cancel,
    /// Liste…: the row's value list.
    OpenList(usize),
    ListCode(usize, String),
    ListLabel(usize, String),
    ListAdd,
    ListRemove(usize),
    ListOk,
    ListCancel,
}

fn msg(event: Event) -> Message {
    Message::LayerFields(event)
}

impl Window {
    /// Whether the value list's window is open over it.
    pub(crate) fn listing(&self) -> bool {
        self.choices.is_some()
    }

    fn changed(&self) -> bool {
        self.rows.iter().map(field_of).ne(self.saved.iter().map(field_of))
            || !renames_of(&self.rows).is_empty()
    }
}

impl App {
    /// Alanlar for `layer` (a layer, not a group).
    pub(crate) fn open_layer_fields(&mut self, layer: &str) -> Task<Message> {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return Task::none();
        };
        let Some(node) = doc
            .model
            .layers()
            .get(layer)
            .filter(|n| n.kind == LayerNodeType::Layer)
        else {
            self.warn("Alanlar yalnız bir katmanın olur; Katmanlar panelinden bir katman seçin.");
            return Task::none();
        };
        let rows: Vec<FieldRow> = node.fields.iter().map(row_of).collect();
        self.layer_fields = Some(Window {
            layer: layer.to_owned(),
            chosen: (!rows.is_empty()).then_some(0),
            saved: rows.clone(),
            rows,
            said: None,
            choices: None,
        });
        self.dialog = Some(Dialog::LayerFields);
        Task::none()
    }

    pub(crate) fn layer_fields_event(&mut self, event: Event) -> Task<Message> {
        let Some(w) = self.layer_fields.as_mut() else {
            return Task::none();
        };
        w.said = None;
        match event {
            Event::Type(i, cell, text) => {
                if let Some(r) = w.rows.get_mut(i) {
                    *match cell {
                        Cell::Name => &mut r.name,
                        Cell::Alias => &mut r.alias,
                        Cell::Size => &mut r.size,
                        Cell::Default => &mut r.default,
                        Cell::Min => &mut r.min,
                        Cell::Max => &mut r.max,
                    } = text;
                }
                w.chosen = Some(i);
            }
            Event::Kind(i, k) => {
                if let (Some(r), Some(kind)) = (w.rows.get_mut(i), LayerFieldKind::ALL.get(k)) {
                    r.kind = *kind;
                }
                w.chosen = Some(i);
            }
            Event::Required(i, on) => {
                if let Some(r) = w.rows.get_mut(i) {
                    r.required = on;
                }
                w.chosen = Some(i);
            }
            Event::Choose(i) => w.chosen = Some(i),
            Event::Add => {
                let n = w.rows.len() + 1;
                w.rows.push(FieldRow {
                    from: None,
                    name: format!("Alan {n}"),
                    alias: String::new(),
                    kind: LayerFieldKind::Text,
                    size: String::new(),
                    required: false,
                    default: String::new(),
                    min: String::new(),
                    max: String::new(),
                    values: None,
                });
                w.chosen = Some(w.rows.len() - 1);
            }
            Event::Remove => {
                if let Some(i) = w.chosen.filter(|&i| i < w.rows.len()) {
                    w.rows.remove(i);
                    w.chosen = (!w.rows.is_empty()).then(|| i.min(w.rows.len() - 1));
                }
            }
            Event::Up | Event::Down => {
                let up = matches!(event, Event::Up);
                if let Some(i) = w.chosen {
                    let to = if up { i.checked_sub(1) } else { Some(i + 1) };
                    if let Some(to) = to.filter(|&t| t < w.rows.len()) {
                        w.rows.swap(i, to);
                        w.chosen = Some(to);
                    }
                }
            }
            Event::FromData => {
                let found = self
                    .document
                    .as_ref()
                    .map(|doc| doc.model.fields_from_data(&w.layer))
                    .unwrap_or_default();
                let taken: Vec<String> = w
                    .rows
                    .iter()
                    .flat_map(|r| [r.name.clone(), r.from.clone().unwrap_or_default()])
                    .collect();
                let new: Vec<FieldRow> = found
                    .iter()
                    .filter(|f| !taken.contains(&f.name))
                    .map(row_of)
                    .collect();
                if new.is_empty() {
                    w.said = Some((
                        Kind::Info,
                        "Nesnelerin her anahtarı tabloda: eklenecek alan yok.".to_owned(),
                    ));
                }
                w.rows.extend(new);
                if w.chosen.is_none() && !w.rows.is_empty() {
                    w.chosen = Some(0);
                }
            }
            Event::Save => return self.save_layer_fields(),
            Event::Cancel => {
                self.layer_fields = None;
                self.dialog = None;
            }
            Event::OpenList(i) => {
                let list = w
                    .rows
                    .get(i)
                    .and_then(|r| r.values.clone())
                    .unwrap_or_default()
                    .into_iter()
                    .map(|c| (c.code, c.label))
                    .collect();
                w.chosen = Some(i);
                w.choices = Some(Choices { row: i, list });
            }
            Event::ListCode(i, t) => {
                if let Some(c) = w.choices.as_mut().and_then(|c| c.list.get_mut(i)) {
                    c.0 = t;
                }
            }
            Event::ListLabel(i, t) => {
                if let Some(c) = w.choices.as_mut().and_then(|c| c.list.get_mut(i)) {
                    c.1 = t;
                }
            }
            Event::ListAdd => {
                if let Some(c) = w.choices.as_mut() {
                    c.list.push((String::new(), String::new()));
                }
            }
            Event::ListRemove(i) => {
                if let Some(c) = w.choices.as_mut().filter(|c| i < c.list.len()) {
                    c.list.remove(i);
                }
            }
            Event::ListOk => {
                if let Some(c) = w.choices.take()
                    && let Some(r) = w.rows.get_mut(c.row)
                {
                    r.values = (!c.list.is_empty()).then(|| {
                        c.list
                            .into_iter()
                            .map(|(code, label)| FieldChoice { code, label })
                            .collect()
                    });
                }
            }
            Event::ListCancel => w.choices = None,
        }
        Task::none()
    }

    /// Kaydet: the fields and the renamed keys, one undo step; says what it
    /// did and how many values do not keep the new rules, or why not.
    fn save_layer_fields(&mut self) -> Task<Message> {
        if let Some(locked) = self.tree_locked() {
            if let Some(w) = self.layer_fields.as_mut() {
                w.said = Some((Kind::Warn, locked.to_owned()));
            }
            return Task::none();
        }
        let (Some(w), Some(doc)) = (self.layer_fields.as_mut(), self.document.as_mut()) else {
            return Task::none();
        };
        if let Some(p) = problem_of(&w.rows) {
            w.said = Some((Kind::Error, p));
            return Task::none();
        }
        let fields: Vec<LayerField> = w.rows.iter().map(field_of).collect();
        let renames = renames_of(&w.rows);
        if let Err(refused) = doc.model.set_layer_fields(&w.layer, fields.clone(), &renames) {
            w.said = Some((Kind::Error, refused.0));
            return Task::none();
        }
        let model = &doc.model;
        let bad = model
            .by_layer(&w.layer)
            .flat_map(|e| {
                fields.iter().filter(|f| {
                    e.base()
                        .attrs
                        .get(&f.name)
                        .is_some_and(|v| !v.trim().is_empty() && check_value(f, v).is_err())
                })
            })
            .count();
        let name = model
            .layers()
            .get(&w.layer)
            .map_or_else(|| w.layer.clone(), |n| n.name.clone());
        self.layer_fields = None;
        self.dialog = None;
        let tail = if bad > 0 {
            format!(" {bad} değer alanların kurallarına uymuyor; Tablo'da uyarı rengiyle gösterilir.")
        } else {
            String::new()
        };
        self.say(
            Level::Info,
            format!("“{name}” katmanının alanları kaydedildi.{tail}"),
        );
        Task::none()
    }

    pub(crate) fn layer_fields_view(&self) -> Element<'_, Message> {
        let (Some(w), Some(doc)) = (&self.layer_fields, &self.document) else {
            return text("").into();
        };
        let cell_width = |j: usize| Length::Fixed(typography::scaled(HEADS[j].1));
        let head = HEADS.iter().enumerate().fold(row![].spacing(6), |r, (j, (h, _))| {
            r.push(container(label::caption(*h)).width(cell_width(j)))
        });
        let input = |i: usize, cell: Cell, value: &str, label_words: &'static str, on: bool| {
            let field = text_input(label_words, value)
                .size(typography::body())
                .padding([3, 6])
                .style(style::field::input);
            let field = if on {
                field.on_input(move |t| msg(Event::Type(i, cell, t)))
            } else {
                field
            };
            Element::from(field)
        };
        let mut rows = Column::new().spacing(4);
        for (i, r) in w.rows.iter().enumerate() {
            let sized = matches!(r.kind, LayerFieldKind::Text | LayerFieldKind::Decimal);
            let ranged = r.kind.is_number();
            let listed = matches!(
                r.kind,
                LayerFieldKind::Text | LayerFieldKind::Integer | LayerFieldKind::Decimal
            );
            let kind = Select::new(
                LayerFieldKind::ALL.iter().map(|k| Choice::new(k.label())),
                LayerFieldKind::ALL.iter().position(|k| *k == r.kind),
                move |k| msg(Event::Kind(i, k)),
            );
            let must = check_box(
                if r.required {
                    Check::Checked
                } else {
                    Check::Unchecked
                },
                Some(msg(Event::Required(i, !r.required))),
            );
            let list_words = match &r.values {
                Some(v) if !v.is_empty() => format!("Liste ({})", v.len()),
                _ => "Liste…".to_owned(),
            };
            let list = button(label::body(list_words))
                .style(style::button::secondary)
                .padding([3, 8])
                .on_press_maybe(listed.then(|| msg(Event::OpenList(i))));
            let line = row![
                container(input(i, Cell::Name, &r.name, "Ad", true)).width(cell_width(0)),
                container(input(i, Cell::Alias, &r.alias, "Takma ad", true)).width(cell_width(1)),
                container(kind).width(cell_width(2)),
                container(input(i, Cell::Size, &r.size, "", sized)).width(cell_width(3)),
                container(must).width(cell_width(4)).center_x(cell_width(4)),
                container(input(i, Cell::Default, &r.default, "", true)).width(cell_width(5)),
                container(input(i, Cell::Min, &r.min, "", ranged)).width(cell_width(6)),
                container(input(i, Cell::Max, &r.max, "", ranged)).width(cell_width(7)),
                container(list).width(cell_width(8)),
            ]
            .spacing(6)
            .align_y(Center);
            let chosen = w.chosen == Some(i);
            rows = rows.push(
                button(line)
                    .on_press(msg(Event::Choose(i)))
                    .padding([2, 4])
                    .style(style::button::table_row(chosen, false)),
            );
        }
        if w.rows.is_empty() {
            rows = rows.push(
                container(label::muted(
                    "Alan yok: katmanın öznitelikleri serbest. Alan ekle ya da Verilerden al ile ekleyin.",
                ))
                .padding(8),
            );
        }
        let table = container(
            Column::new()
                .push(container(head).padding([4, 8]))
                .push(kentos_ui::widget::horizontal_divider())
                .push(iced::widget::scrollable(rows).height(Length::Fixed(typography::scaled(300.0)))),
        )
        .style(style::container::field_box)
        .width(Fill);
        let tool = |glyph: &str, words: &'static str, on: Option<Message>| {
            button(
                row![icon(from_web(Some(glyph))).size(14.0), label::body(words)]
                    .spacing(6)
                    .align_y(Center),
            )
            .style(style::button::secondary)
            .padding([4, 10])
            .on_press_maybe(on)
        };
        let chosen = w.chosen;
        let n = w.rows.len();
        let actions = row![
            tool("plus", ADD, Some(msg(Event::Add))),
            tool("erase", REMOVE, chosen.map(|_| msg(Event::Remove))),
            tool("chevronUp", UP, chosen.filter(|&i| i > 0).map(|_| msg(Event::Up))),
            tool(
                "chevronDown",
                DOWN,
                chosen.filter(|&i| i + 1 < n).map(|_| msg(Event::Down))
            ),
            iced::widget::space::horizontal(),
            tool("fieldsFromData", FROM_DATA, Some(msg(Event::FromData))),
        ]
        .spacing(8)
        .align_y(Center);
        let problem = problem_of(&w.rows);
        let renamed = renames_of(&w.rows).len();
        let line = match (&w.said, &problem) {
            (Some((kind, text)), _) => words::text_line(*kind, text.clone()),
            (None, Some(p)) => words::text_line(Kind::Error, p.clone()),
            (None, None) if w.rows.is_empty() => words::text_line(
                Kind::Ok,
                "Alan yok: katmanın öznitelikleri serbest.".to_owned(),
            ),
            (None, None) => words::text_line(
                Kind::Ok,
                if renamed > 0 {
                    format!(
                        "{n} alan; {renamed} alanın adı değişiyor, değerleri yeni adla taşınacak."
                    )
                } else {
                    format!("{n} alan.")
                },
            ),
        };
        let path = doc.model.layers().path(&w.layer);
        let ready = problem.is_none() && w.changed();
        let main = overlay::modal(
            Frame::new(FIELDS_TITLE)
                .push(
                    Column::new()
                        .spacing(10)
                        .push(label::muted(format!("Katman: {path}")))
                        .push(table)
                        .push(actions)
                        .push(words::summary(vec![line])),
                )
                .action(words::secondary(CANCEL, Some(msg(Event::Cancel))))
                .action(words::primary(SAVE, ready.then(|| msg(Event::Save))))
                .width(1040.0),
            msg(Event::Cancel),
        );
        let Some(c) = &w.choices else {
            return main;
        };
        let name = w.rows.get(c.row).map(|r| r.name.clone()).unwrap_or_default();
        let mut list = Column::new().spacing(4).push(
            row![
                container(label::caption("Kod")).width(Length::Fixed(140.0)),
                container(label::caption("Etiket")).width(Length::Fixed(200.0)),
            ]
            .spacing(6),
        );
        for (i, (code, label_text)) in c.list.iter().enumerate() {
            list = list.push(
                row![
                    container(
                        text_input("", code)
                            .on_input(move |t| msg(Event::ListCode(i, t)))
                            .padding([3, 6])
                            .style(style::field::input)
                    )
                    .width(Length::Fixed(140.0)),
                    container(
                        text_input("", label_text)
                            .on_input(move |t| msg(Event::ListLabel(i, t)))
                            .padding([3, 6])
                            .style(style::field::input)
                    )
                    .width(Length::Fixed(200.0)),
                    button(icon(kentos_ui::icon::Icon::Close).size(12.0))
                        .style(style::button::subtle)
                        .padding(4)
                        .on_press(msg(Event::ListRemove(i))),
                ]
                .spacing(6)
                .align_y(Center),
            );
        }
        let sub = overlay::modal(
            Frame::new(format!("Değer listesi · {name}"))
                .push(
                    Column::new()
                        .spacing(10)
                        .push(label::caption(
                            "Nesneye kod yazılır, tabloda ve formda etiketi gösterilir; etiket yazılırsa kodu yazılır.",
                        ))
                        .push(list)
                        .push(tool("plus", "Değer ekle", Some(msg(Event::ListAdd)))),
                )
                .action(words::secondary(CANCEL, Some(msg(Event::ListCancel))))
                .action(words::primary("Tamam", Some(msg(Event::ListOk))))
                .width(460.0),
            msg(Event::ListCancel),
        );
        iced::widget::stack![main, sub].into()
    }

    /// The window's controls by their words (a trace's `dialog` step): the
    /// chosen row's fields by their headers (Ad, Takma ad, Varsayılan, En
    /// az, En çok, Uzunluk / Basamak), its Zorunlu box, a row by its number,
    /// the buttons.
    pub(crate) fn layer_fields_control(
        &self,
        control: Control<'_>,
    ) -> Result<Option<Message>, String> {
        let Some(w) = &self.layer_fields else {
            return Err(format!("{FIELDS_TITLE} penceresi açık değil"));
        };
        let chosen = || w.chosen.ok_or_else(|| "seçili alan yok".to_owned());
        Ok(match control {
            Control::Fill(label_words, t) => {
                let cell = match label_words {
                    "Ad" => Cell::Name,
                    "Takma ad" => Cell::Alias,
                    "Uzunluk / Basamak" => Cell::Size,
                    "Varsayılan" => Cell::Default,
                    "En az" => Cell::Min,
                    "En çok" => Cell::Max,
                    other => return Err(format!("“{FIELDS_TITLE}” penceresinde “{other}” alanı yok")),
                };
                Some(msg(Event::Type(chosen()?, cell, t.to_owned())))
            }
            Control::Check("Zorunlu", on) => {
                let i = chosen()?;
                (w.rows[i].required != on).then(|| msg(Event::Required(i, on)))
            }
            Control::Row(n) if n >= 1 && n <= w.rows.len() => Some(msg(Event::Choose(n - 1))),
            Control::Press(SAVE) => (problem_of(&w.rows).is_none() && w.changed()).then(|| msg(Event::Save)),
            Control::Press(CANCEL) => Some(msg(Event::Cancel)),
            Control::Press(ADD) => Some(msg(Event::Add)),
            Control::Press(REMOVE) => w.chosen.map(|_| msg(Event::Remove)),
            Control::Press(UP) => w.chosen.filter(|&i| i > 0).map(|_| msg(Event::Up)),
            Control::Press(DOWN) => w
                .chosen
                .filter(|&i| i + 1 < w.rows.len())
                .map(|_| msg(Event::Down)),
            Control::Press(FROM_DATA) => Some(msg(Event::FromData)),
            other => return Err(format!("“{FIELDS_TITLE}” penceresinde {other} yok")),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_row_writes_its_kinds_own_parts_and_renames_say_old_and_new() {
        let mut r = row_of(&LayerField {
            alias: Some("Kat sayısı".into()),
            min: Some("1".into()),
            ..LayerField::new("Kat", LayerFieldKind::Integer)
        });
        r.size = "5".into();
        r.name = "Katlar".into();
        let f = field_of(&r);
        assert_eq!((f.length, f.scale), (None, None), "an integer takes no size");
        assert_eq!(f.min.as_deref(), Some("1"));
        assert_eq!(renames_of(&[r.clone()]), [("Kat".to_owned(), "Katlar".to_owned())]);
        r.kind = LayerFieldKind::Text;
        r.size = "x".into();
        assert!(problem_of(&[r]).is_some_and(|p| p.contains("uzunluğu bir tam sayı")));
    }
}
