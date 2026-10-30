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
use kentos_commands::{
    CommandResult, FieldChoice, FieldRename, LayerEditOperation, LayerField, LayerFieldKind,
    LayerNodeType, LayersEdit, check_value, fields_problem,
};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, layers_edit};
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
const MIGRATE: &str = "Değerleri biçime çevir";

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
    /// A listed text field holds several codes (docs/adr/0408 §1).
    pub multiple: bool,
    /// Bölünce and Birleşince (docs/adr/0419).
    pub split: Option<kentos_commands::SplitPolicy>,
    pub merge: Option<kentos_commands::MergePolicy>,
    /// The project's value list it links (docs/adr/0430); its values or bounds are that list's.
    pub domain: Option<String>,
    /// Tekil, the range's ends left out and the unit (docs/adr/0438 §3, §4).
    pub unique: bool,
    pub min_exclusive: bool,
    pub max_exclusive: bool,
    pub unit: String,
    /// Dışarı verilmez (docs/adr/0638 §5): its values stay in the drawing.
    pub withheld: bool,
    /// What the bound table's column says (docs/adr/0753): kept as it is, the table writes it.
    pub column: Option<kentos_commands::FieldColumn>,
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
        multiple: f.multiple,
        split: f.split,
        merge: f.merge,
        domain: f.domain.clone(),
        unique: f.unique,
        min_exclusive: f.min_exclusive,
        max_exclusive: f.max_exclusive,
        unit: f.unit.clone().unwrap_or_default(),
        withheld: f.withheld,
        column: f.column.clone(),
    }
}

/// What a bound table's column says of its field (docs/adr/0753), in a sentence or two; none without it.
pub fn column_said(column: Option<&kentos_commands::FieldColumn>) -> Option<String> {
    use kentos_commands::ColumnMade;
    let c = column?;
    let mut said = Vec::new();
    match c.making() {
        Some(ColumnMade::Default(m)) => said.push(format!("Yeni satırda değerini veritabanı verir: {m}.")),
        Some(ColumnMade::Identity { always: false }) => said.push(
            "Yeni satırda boş bırakılırsa veritabanı sıradaki numarayı verir (GENERATED BY DEFAULT AS IDENTITY)."
                .to_owned(),
        ),
        Some(ColumnMade::Identity { always: true }) => said.push(
            "Değerini veritabanı her zaman kendisi numaralar (GENERATED ALWAYS AS IDENTITY); düzenlenmez.".to_owned(),
        ),
        Some(ColumnMade::Generated(e)) => said.push(match e {
            Some(e) => format!(
                "Değerini veritabanı satırın diğer değerlerinden hesaplar: {e}. Düzenlenmez; kaynağa her yazmada tablodan gelir."
            ),
            None => "Değerini veritabanı hesaplar (GENERATED ALWAYS). Düzenlenmez; kaynağa her yazmada tablodan gelir."
                .to_owned(),
        }),
        None => {}
    }
    if let Some(t) = &c.refers {
        let named = match &t.label {
            Some(l) => format!("kodu “{}”, etiketi “{l}” sütunundan", t.column),
            None => format!("kodu ve etiketi “{}” sütunundan", t.column),
        };
        said.push(format!(
            "Değer listesi “{}” tablosunun satırları ({named}); tablo her okunduğunda yenilenir.",
            t.table
        ));
    }
    (!said.is_empty()).then(|| said.join(" "))
}

/// What a form says in an empty field whose value the bound table makes (docs/adr/0753 §2, 0754 §3): the default it
/// works out, an identity's next number, a generated column's expression.
pub fn made_said(column: Option<&kentos_commands::FieldColumn>) -> Option<String> {
    use kentos_commands::ColumnMade;
    Some(match column?.making()? {
        ColumnMade::Default(m) => format!("Veritabanı verir: {m}"),
        ColumnMade::Identity { .. } => "Veritabanı verir: sıradaki numara".to_owned(),
        ColumnMade::Generated(Some(e)) => format!("Veritabanı hesaplar: {e}"),
        ColumnMade::Generated(None) => "Veritabanı hesaplar".to_owned(),
    })
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
    // A number's range, and a date's, a date and time's or a time of day's (docs/adr/0436).
    if r.kind.is_number() || r.kind.is_temporal() {
        // A date typed as 7.10.2026 is kept in its one form; what does not read stays, to be said.
        let canonical = |t: &str| {
            if r.kind.is_temporal() {
                kentos_commands::check_value(&LayerField::new("aralık", r.kind), t)
                    .unwrap_or_else(|_| t.to_owned())
            } else {
                t.to_owned()
            }
        };
        if !r.min.trim().is_empty() {
            f.min = Some(canonical(&r.min));
            f.min_exclusive = r.min_exclusive;
        }
        if !r.max.trim().is_empty() {
            f.max = Some(canonical(&r.max));
            f.max_exclusive = r.max_exclusive;
        }
    }
    // A number's unit, as typed (docs/adr/0438 §4).
    if r.kind.is_number() && !r.unit.trim().is_empty() {
        f.unit = Some(r.unit.clone());
    }
    f.unique = r.unique && r.kind != LayerFieldKind::Boolean;
    if matches!(
        r.kind,
        LayerFieldKind::Text
            | LayerFieldKind::Integer
            | LayerFieldKind::BigInt
            | LayerFieldKind::Decimal
    ) {
        f.values = r.values.clone();
    }
    f.required = r.required;
    f.multiple = r.multiple && r.kind == LayerFieldKind::Text && f.values.is_some();
    if !r.default.trim().is_empty() {
        f.default = Some(r.default.clone());
    }
    f.split = r.split;
    f.merge = r.merge;
    f.domain = r.domain.clone();
    f.withheld = r.withheld;
    f.column = r.column.clone();
    f
}

/// A listed value's first (`from`) or last day as typed (docs/adr/0438 §5): a day typed as 7.10.2026 is
/// kept in its one form; what does not read stays, for the window to say.
fn set_day(w: &mut Window, i: usize, typed: String, from: bool) {
    let day = (!typed.trim().is_empty()).then(|| {
        kentos_commands::check_value(&LayerField::new("gün", LayerFieldKind::Date), &typed)
            .unwrap_or(typed.clone())
    });
    if let Some(c) = w
        .choices
        .as_mut()
        .filter(|c| c.domain.is_none())
        .and_then(|c| c.list.get_mut(i))
    {
        if from {
            c.from = day;
        } else {
            c.until = day;
        }
    }
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
    list: Vec<FieldChoice>,
    /// Several of the values may be chosen (a text field's).
    multiple: bool,
    /// The project's list it is linked to (docs/adr/0430): its codes are that list's, read-only here.
    domain: Option<String>,
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
    /// Değerleri biçime çevir (docs/adr/0439 §8): the layer's values in their saved fields' canonical texts.
    Migrate,
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
    /// Kullanımdan kalktı (docs/adr/0418).
    ListRetired(usize, bool),
    /// A value's description and its days (docs/adr/0438 §5), as typed.
    ListDescription(usize, String),
    ListFrom(usize, String),
    ListUntil(usize, String),
    /// The chosen row's Tekil, ends left out and unit (docs/adr/0438 §3, §4).
    Unique(bool),
    /// Dışarı verilmez (docs/adr/0638 §5).
    Withheld(bool),
    MinExclusive(bool),
    MaxExclusive(bool),
    Unit(String),
    /// Bölünce and Birleşince of a row (docs/adr/0419).
    Split(usize, Option<kentos_commands::SplitPolicy>),
    Merge(usize, Option<kentos_commands::MergePolicy>),
    ListAdd,
    ListRemove(usize),
    /// Birden çok değer seçilebilir (docs/adr/0408 §1).
    ListMultiple(bool),
    /// Projenin listesi (docs/adr/0430): the row linked to a project's list; none: its own list.
    ListDomain(Option<String>),
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
        self.rows
            .iter()
            .map(field_of)
            .ne(self.saved.iter().map(field_of))
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

    /// Where a saved field of `layer` is named (docs/adr/0414 §1), by its saved name.
    pub(crate) fn saved_field_uses(
        &self,
        layer: &str,
        name: &str,
    ) -> Vec<kentos_commands::FieldUse> {
        let Some(doc) = &self.document else {
            return Vec::new();
        };
        let model = &doc.model;
        model
            .layers()
            .get(layer)
            .and_then(|n| n.fields.iter().find(|f| f.name == name))
            .map(|f| {
                kentos_forms::field_refs::uses(model.settings(), model.layers().nodes(), layer, f)
            })
            .unwrap_or_default()
    }

    pub(crate) fn layer_fields_event(&mut self, event: Event) -> Task<Message> {
        // Sil on a field named elsewhere says what it breaks (docs/adr/0414 §4).
        let removed = match &event {
            Event::Remove => self.layer_fields.as_ref().and_then(|w| {
                let r = w.chosen.and_then(|i| w.rows.get(i))?;
                let from = r.from.as_deref()?;
                Some((r.name.clone(), self.saved_field_uses(&w.layer, from).len()))
            }),
            _ => None,
        };
        let domains = self
            .document
            .as_ref()
            .map(|d| d.model.settings().domains.clone())
            .unwrap_or_default();
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
                    multiple: false,
                    split: None,
                    merge: None,
                    domain: None,
                    unique: false,
                    min_exclusive: false,
                    max_exclusive: false,
                    unit: String::new(),
                    withheld: false,
                    column: None,
                });
                w.chosen = Some(w.rows.len() - 1);
            }
            Event::Remove => {
                if let Some(i) = w.chosen.filter(|&i| i < w.rows.len()) {
                    w.rows.remove(i);
                    w.chosen = (!w.rows.is_empty()).then(|| i.min(w.rows.len() - 1));
                }
                if let Some((name, n)) = removed.filter(|(_, n)| *n > 0) {
                    w.said = Some((
                        Kind::Warn,
                        format!(
                            "“{name}” alanı {n} yerde kullanılıyor; kaydederseniz oralar bozulur. Vazgeç alanları kaydetmeden kapatır."
                        ),
                    ));
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
            Event::Migrate => {
                if w.changed() {
                    w.said = Some((
                        Kind::Warn,
                        "Önce alanları kaydedin; değerler kaydedilmiş alanların biçimine çevrilir."
                            .to_owned(),
                    ));
                    return Task::none();
                }
                let layer = w.layer.clone();
                return self.migrate_layer_values(&layer);
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
                    .unwrap_or_default();
                let multiple = w.rows.get(i).is_some_and(|r| r.multiple);
                let domain = w.rows.get(i).and_then(|r| r.domain.clone());
                w.chosen = Some(i);
                w.choices = Some(Choices {
                    row: i,
                    list,
                    multiple,
                    domain,
                });
            }
            Event::ListCode(i, t) => {
                if let Some(c) = w
                    .choices
                    .as_mut()
                    .filter(|c| c.domain.is_none())
                    .and_then(|c| c.list.get_mut(i))
                {
                    c.code = t;
                }
            }
            Event::ListLabel(i, t) => {
                if let Some(c) = w
                    .choices
                    .as_mut()
                    .filter(|c| c.domain.is_none())
                    .and_then(|c| c.list.get_mut(i))
                {
                    c.label = t;
                }
            }
            Event::Split(i, p) => {
                if let Some(r) = w.rows.get_mut(i) {
                    r.split = p;
                }
            }
            Event::Merge(i, p) => {
                if let Some(r) = w.rows.get_mut(i) {
                    r.merge = p;
                }
            }
            Event::ListRetired(i, on) => {
                if let Some(c) = w
                    .choices
                    .as_mut()
                    .filter(|c| c.domain.is_none())
                    .and_then(|c| c.list.get_mut(i))
                {
                    c.retired = on;
                }
            }
            Event::ListDescription(i, t) => {
                if let Some(c) = w
                    .choices
                    .as_mut()
                    .filter(|c| c.domain.is_none())
                    .and_then(|c| c.list.get_mut(i))
                {
                    c.description = (!t.trim().is_empty()).then_some(t);
                }
            }
            // A day typed as 7.10.2026 is kept in its one form; what does not read stays, to be said.
            Event::ListFrom(i, t) => set_day(w, i, t, true),
            Event::ListUntil(i, t) => set_day(w, i, t, false),
            Event::Unique(on) => {
                if let Some(r) = w.chosen.and_then(|i| w.rows.get_mut(i)) {
                    r.unique = on;
                }
            }
            Event::Withheld(on) => {
                if let Some(r) = w.chosen.and_then(|i| w.rows.get_mut(i)) {
                    r.withheld = on;
                }
            }
            Event::MinExclusive(on) => {
                if let Some(r) = w.chosen.and_then(|i| w.rows.get_mut(i)) {
                    r.min_exclusive = on;
                }
            }
            Event::MaxExclusive(on) => {
                if let Some(r) = w.chosen.and_then(|i| w.rows.get_mut(i)) {
                    r.max_exclusive = on;
                }
            }
            Event::Unit(t) => {
                if let Some(r) = w.chosen.and_then(|i| w.rows.get_mut(i)) {
                    r.unit = t;
                }
            }
            Event::ListAdd => {
                if let Some(c) = w.choices.as_mut().filter(|c| c.domain.is_none()) {
                    c.list.push(FieldChoice::new("", ""));
                }
            }
            Event::ListRemove(i) => {
                if let Some(c) = w
                    .choices
                    .as_mut()
                    .filter(|c| i < c.list.len() && c.domain.is_none())
                {
                    c.list.remove(i);
                }
            }
            Event::ListMultiple(on) => {
                if let Some(c) = w.choices.as_mut() {
                    c.multiple = on;
                }
            }
            Event::ListDomain(id) => {
                let found = id
                    .as_ref()
                    .and_then(|id| domains.iter().find(|d| d.id == *id));
                if let Some(c) = w.choices.as_mut() {
                    if let Some(d) = found {
                        c.list = d.values.clone().unwrap_or_default();
                    }
                    c.domain = id;
                }
            }
            Event::ListOk => {
                if let Some(c) = w.choices.take()
                    && let Some(r) = w.rows.get_mut(c.row)
                {
                    let found = c
                        .domain
                        .as_ref()
                        .and_then(|id| domains.iter().find(|d| d.id == *id));
                    match found {
                        // A range's bounds are the row's; a list's codes its values.
                        Some(d) if !d.coded() => {
                            r.values = None;
                            r.min = d.min.clone().unwrap_or_default();
                            r.max = d.max.clone().unwrap_or_default();
                        }
                        _ => r.values = (!c.list.is_empty()).then_some(c.list),
                    }
                    r.domain = c.domain;
                    r.multiple = c.multiple && r.values.is_some();
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
        // Through `cad.layers.edit`'s fields (docs/adr/0248 §1); its refusal in the window.
        let mut input = LayersEdit::new(LayerEditOperation::Fields);
        input.layer = Some(w.layer.clone());
        input.fields = Some(fields.clone());
        input.renames = Some(
            renames_of(&w.rows)
                .into_iter()
                .map(|(from, to)| FieldRename { from, to })
                .collect(),
        );
        match layers_edit::execute(&mut ExecutionContext::new(&mut doc.model), input) {
            CommandResult::Completed { .. } => {}
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => {
                w.said = Some((Kind::Error, error.message));
                return Task::none();
            }
            CommandResult::Queued { .. } | CommandResult::Cancelled => return Task::none(),
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
            format!(
                " {bad} değer alanların kurallarına uymuyor; Tablo'da uyarı rengiyle gösterilir."
            )
        } else {
            String::new()
        };
        self.announce(
            Level::Info,
            format!("“{name}” katmanının alanları kaydedildi.{tail}"),
        );
        Task::none()
    }

    /// Kullanıldığı yerler (docs/adr/0414 §4): the chosen saved field's places; a rename says what
    /// follows it and what a published release keeps.
    fn field_usage_box<'a>(&self, w: &Window) -> Element<'a, Message> {
        let Some(r) = w.chosen.and_then(|i| w.rows.get(i)) else {
            return iced::widget::space().into();
        };
        let Some(from) = r.from.as_deref() else {
            return container(label::muted(
                "Yeni alan: kaydedilince formlarda, koşullarda ve kurallarda kullanılabilir.",
            ))
            .into();
        };
        let uses = self.saved_field_uses(&w.layer, from);
        if uses.is_empty() {
            return container(label::muted(format!(
                "“{from}” alanı başka bir yerde kullanılmıyor."
            )))
            .into();
        }
        let kept = uses.iter().filter(|u| u.kept).count();
        let mut list = Column::new().spacing(2);
        for u in &uses {
            let mut line = row![
                icon(from_web(Some(match u.how {
                    kentos_commands::FieldUseHow::Key => "formEdit",
                    kentos_commands::FieldUseHow::Name => "layerFields",
                    kentos_commands::FieldUseHow::Expression => "expression",
                    kentos_commands::FieldUseHow::Template => "text",
                })))
                .size(14.0),
                label::body(u.place.clone()),
            ]
            .spacing(6)
            .align_y(Center);
            if u.kept {
                line = line.push(label::muted("yayımlanmış sürüm, değişmez"));
            }
            list = list.push(line);
        }
        let mut out = Column::new().spacing(6).push(label::strong(format!(
            "Kullanıldığı yerler ({})",
            uses.len()
        )));
        if r.name != from {
            let follows = uses
                .iter()
                .filter(|u| !u.kept && u.how != kentos_commands::FieldUseHow::Key)
                .count();
            let mut said = format!(
                "“{from}” adı “{}” olunca {follows} yer yeni adla güncellenecek; alanın anahtarıyla bağlı olanlar bağlı kalır.",
                r.name
            );
            if kept > 0 {
                said.push_str(&format!(
                    " Yayımlanmış sürümlerdeki {kept} yer değişmez: formu yeniden yayınlayın."
                ));
            }
            out = out.push(words::text_line(Kind::Warn, said));
        }
        out.push(
            container(
                iced::widget::scrollable(container(list).padding([4, 8]).width(Fill))
                    .width(Fill)
                    .height(Length::Fixed(typography::scaled(100.0))),
            )
            .style(style::container::field_box)
            .width(Fill),
        )
        .into()
    }

    pub(crate) fn layer_fields_view(&self) -> Element<'_, Message> {
        let (Some(w), Some(doc)) = (&self.layer_fields, &self.document) else {
            return text("").into();
        };
        let domains = self
            .document
            .as_ref()
            .map(|d| d.model.settings().domains.clone())
            .unwrap_or_default();
        let cell_width = |j: usize| Length::Fixed(typography::scaled(HEADS[j].1));
        let head = HEADS
            .iter()
            .enumerate()
            .fold(row![].spacing(6), |r, (j, (h, _))| {
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
            let ranged = r.kind.is_number() || r.kind.is_temporal();
            let listed = matches!(
                r.kind,
                LayerFieldKind::Text
                    | LayerFieldKind::Integer
                    | LayerFieldKind::BigInt
                    | LayerFieldKind::Decimal
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
            let linked = r
                .domain
                .as_ref()
                .and_then(|id| domains.iter().find(|d| d.id == *id));
            let list_words = match (linked, &r.values) {
                (Some(d), _) => format!("↗ {}", d.name),
                (None, Some(v)) if !v.is_empty() => format!("Liste ({})", v.len()),
                _ => "Liste…".to_owned(),
            };
            // A field linked to a project's range takes its bounds from it (docs/adr/0430).
            let ranged = ranged && r.domain.is_none();
            let list = button(
                kentos_ui::widget::Elided::new(list_words)
                    .size(typography::body())
                    .width(Length::Fixed(typography::scaled(HEADS[8].1) - 18.0)),
            )
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
        // As tall as its rows, at most 300 px; 200 px while the places of a field are listed under it.
        let most = if w.chosen.is_some() { 200.0 } else { 300.0 };
        let rows_height = (w.rows.len().max(3) as f32 * 36.0 + 8.0).min(most);
        let table = container(
            Column::new()
                .push(container(head).padding([4, 8]))
                .push(kentos_ui::widget::horizontal_divider())
                .push(
                    iced::widget::scrollable(rows)
                        .height(Length::Fixed(typography::scaled(rows_height))),
                ),
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
            tool(
                "chevronUp",
                UP,
                chosen.filter(|&i| i > 0).map(|_| msg(Event::Up))
            ),
            tool(
                "chevronDown",
                DOWN,
                chosen.filter(|&i| i + 1 < n).map(|_| msg(Event::Down))
            ),
            iced::widget::space::horizontal(),
            tool("fieldsFromData", FROM_DATA, Some(msg(Event::FromData))),
            tool("conversion", MIGRATE, Some(msg(Event::Migrate))),
        ]
        .spacing(8)
        .align_y(Center);
        // The chosen row's Tekil, ends left out and unit (docs/adr/0438 §3, §4).
        let extras: Element<'_, Message> = match chosen.and_then(|i| w.rows.get(i)) {
            Some(r) => {
                let ranged = (r.kind.is_number() || r.kind.is_temporal()) && r.domain.is_none();
                let mut line = row![
                    label::strong(r.name.clone()),
                    words::check(
                        r.unique,
                        "Tekil",
                        (r.kind != LayerFieldKind::Boolean).then(|| msg(Event::Unique(!r.unique)))
                    ),
                    words::check(
                        r.min_exclusive,
                        "En az hariç",
                        (ranged && !r.min.trim().is_empty())
                            .then(|| msg(Event::MinExclusive(!r.min_exclusive)))
                    ),
                    words::check(
                        r.max_exclusive,
                        "En çok hariç",
                        (ranged && !r.max.trim().is_empty())
                            .then(|| msg(Event::MaxExclusive(!r.max_exclusive)))
                    ),
                    kentos_ui::widget::tip(
                        words::check(
                            r.withheld,
                            "Dışarı verilmez",
                            Some(msg(Event::Withheld(!r.withheld)))
                        ),
                        kentos_ui::widget::Tip::new(
                            "Değerleri çizimde kalır: atlas, tablonun kopyası ve dosyası, istatistik, grafik ve komut kaydı onu dışarıda bırakır."
                        ),
                        iced::widget::tooltip::Position::Top,
                    ),
                ]
                .spacing(16)
                .align_y(Center);
                if r.kind.is_number() {
                    line = line.push(label::body("Birim")).push(
                        container(
                            text_input("m², kg …", &r.unit)
                                .size(typography::body())
                                .padding([3, 6])
                                .style(style::field::input)
                                .on_input(|t| msg(Event::Unit(t))),
                        )
                        .width(Length::Fixed(typography::scaled(90.0))),
                    );
                }
                // What the bound table's column says (docs/adr/0753): written by binding and reading, shown here.
                match column_said(r.column.as_ref()) {
                    Some(said) => iced::widget::column![line, label::muted(said)]
                        .spacing(6)
                        .into(),
                    None => line.into(),
                }
            }
            None => label::caption("Tekil, hariç uçlar, birim ve dışarı verilmez için bir alan seçin.").into(),
        };
        let usage =
            iced::widget::column![extras, policy_box(w), self.field_usage_box(w)].spacing(10);
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
        // The body scrolls in a low window, the buttons stay in sight (docs/adr/0749, 0753).
        let main = overlay::modal(
            Frame::new(FIELDS_TITLE)
                .scroll(
                    Column::new()
                        .spacing(10)
                        .push(label::muted(format!("Katman: {path}")))
                        .push(table)
                        .push(actions)
                        .push(usage)
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
        let name = w
            .rows
            .get(c.row)
            .map(|r| r.name.clone())
            .unwrap_or_default();
        let mut list = Column::new().spacing(4).push(
            row![
                container(label::caption("Kod")).width(Length::Fixed(100.0)),
                container(label::caption("Etiket")).width(Length::Fixed(150.0)),
                container(label::caption("Açıklama")).width(Length::Fixed(170.0)),
                container(label::caption("Geçerlilik başı")).width(Length::Fixed(108.0)),
                container(label::caption("sonu")).width(Length::Fixed(108.0)),
                label::caption("Kalktı"),
            ]
            .spacing(6),
        );
        // Projenin listesi (docs/adr/0430): the project's lists of this field's kind.
        let row_kind = w.rows.get(c.row).map(|r| r.kind);
        let fitting: Vec<&kentos_commands::FieldDomain> = domains
            .iter()
            .filter(|d| Some(d.kind) == row_kind)
            .collect();
        let linked = c
            .domain
            .as_ref()
            .and_then(|id| fitting.iter().find(|d| d.id == *id).copied());
        let own = linked.is_none();
        for (i, choice) in c.list.iter().enumerate() {
            let (code, label_text) = (&choice.code, &choice.label);
            let retired = choice.retired;
            let code_input = text_input("", code)
                .padding([3, 6])
                .style(style::field::input);
            let label_input = text_input("", label_text)
                .padding([3, 6])
                .style(style::field::input);
            // What the value means and the days it holds (docs/adr/0438 §5).
            let typed = |placeholder: &'static str, value: &str, on: fn(usize, String) -> Event| {
                let field = text_input(placeholder, value)
                    .padding([3, 6])
                    .style(style::field::input);
                if own {
                    field.on_input(move |t| msg(on(i, t)))
                } else {
                    field
                }
            };
            let shown_day = |d: &Option<String>| {
                d.as_deref().map_or_else(String::new, |d| {
                    kentos_commands::display_value(
                        &LayerField::new("gün", LayerFieldKind::Date),
                        d,
                    )
                })
            };
            list = list.push(
                row![
                    container(if own {
                        code_input.on_input(move |t| msg(Event::ListCode(i, t)))
                    } else {
                        code_input
                    })
                    .width(Length::Fixed(100.0)),
                    container(if own {
                        label_input.on_input(move |t| msg(Event::ListLabel(i, t)))
                    } else {
                        label_input
                    })
                    .width(Length::Fixed(150.0)),
                    container(typed(
                        "",
                        choice.description.as_deref().unwrap_or(""),
                        Event::ListDescription
                    ))
                    .width(Length::Fixed(170.0)),
                    container(typed("GG.AA.YYYY", &shown_day(&choice.from), Event::ListFrom))
                        .width(Length::Fixed(108.0)),
                    container(typed("GG.AA.YYYY", &shown_day(&choice.until), Event::ListUntil))
                        .width(Length::Fixed(108.0)),
                    kentos_ui::widget::tip(
                        words::check(retired, "", own.then(|| msg(Event::ListRetired(i, !retired)))),
                        kentos_ui::widget::Tip::new("Kullanımdan kalktı").body(
                            "Tutan kayıtta kalır ve görünür; yeni seçimde sunulmaz, formla yeniden yazılmaz."
                        ),
                        iced::widget::tooltip::Position::Top,
                    ),
                    button(icon(kentos_ui::icon::Icon::Close).size(12.0))
                        .style(style::button::subtle)
                        .padding(4)
                        .on_press_maybe(own.then(|| msg(Event::ListRemove(i)))),
                ]
                .spacing(6)
                .align_y(Center),
            );
        }
        let ids: Vec<String> = fitting.iter().map(|d| d.id.clone()).collect();
        let mut offered = vec![Choice::new("Alanın kendi listesi")];
        offered.extend(fitting.iter().map(|d| {
            Choice::new(if d.coded() {
                format!("{} ({} kod)", d.name, d.values.as_ref().map_or(0, Vec::len))
            } else {
                format!(
                    "{} ({} – {})",
                    d.name,
                    d.min.as_deref().unwrap_or("…"),
                    d.max.as_deref().unwrap_or("…")
                )
            })
        }));
        let now = match &c.domain {
            None => Some(0),
            Some(id) => ids.iter().position(|x| x == id).map(|p| p + 1),
        };
        let pick = Select::new(offered, now, move |k| {
            msg(Event::ListDomain(k.checked_sub(1).map(|k| ids[k].clone())))
        });
        let mut body = Column::new().spacing(10).push(words::field(
            "Projenin listesi",
            container(pick).width(320),
            Some(match linked {
                Some(d) => format!(
                    "Kodlar “{}” listesinindir; Değer listeleri penceresinde düzenlenir, bağlı bütün alanlar izler.",
                    d.name
                ),
                None if fitting.is_empty() => {
                    "Projede bu türde değer listesi yok; Değer listeleri penceresinden ekleyin.".to_owned()
                }
                None => "Bir liste seçilirse alan ona bağlanır; liste değişince alan da değişir.".to_owned(),
            }),
        ));
        if linked.is_some_and(|d| !d.coded()) {
            let d = linked.unwrap_or_else(|| unreachable!());
            body = body.push(label::caption(format!(
                "Aralık: {} – {}",
                d.min.as_deref().unwrap_or("…"),
                d.max.as_deref().unwrap_or("…")
            )));
        } else {
            body = body
                .push(label::caption(
                    "Nesneye kod yazılır, tabloda ve formda etiketi gösterilir; etiket yazılırsa kodu yazılır.",
                ))
                .push(list);
            if own {
                body = body.push(tool("plus", "Değer ekle", Some(msg(Event::ListAdd))));
            }
        }
        // A text field may hold several of its values (docs/adr/0408 §1).
        if w.rows
            .get(c.row)
            .is_some_and(|r| r.kind == LayerFieldKind::Text)
        {
            body = body.push(words::check(
                c.multiple,
                "Birden çok değer seçilebilir",
                Some(msg(Event::ListMultiple(!c.multiple))),
            ));
        }
        let sub = overlay::modal(
            Frame::new(format!("Değer listesi · {name}"))
                .push(body)
                .action(words::secondary(CANCEL, Some(msg(Event::ListCancel))))
                .action(words::primary("Tamam", Some(msg(Event::ListOk))))
                .width(820.0),
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
                    other => {
                        return Err(format!("“{FIELDS_TITLE}” penceresinde “{other}” alanı yok"));
                    }
                };
                Some(msg(Event::Type(chosen()?, cell, t.to_owned())))
            }
            Control::Check("Zorunlu", on) => {
                let i = chosen()?;
                (w.rows[i].required != on).then(|| msg(Event::Required(i, on)))
            }
            Control::Row(n) if n >= 1 && n <= w.rows.len() => Some(msg(Event::Choose(n - 1))),
            Control::Press(SAVE) => {
                (problem_of(&w.rows).is_none() && w.changed()).then(|| msg(Event::Save))
            }
            Control::Press(CANCEL) => Some(msg(Event::Cancel)),
            Control::Press(ADD) => Some(msg(Event::Add)),
            Control::Press(REMOVE) => w.chosen.map(|_| msg(Event::Remove)),
            Control::Press(UP) => w.chosen.filter(|&i| i > 0).map(|_| msg(Event::Up)),
            Control::Press(DOWN) => w
                .chosen
                .filter(|&i| i + 1 < w.rows.len())
                .map(|_| msg(Event::Down)),
            Control::Press(FROM_DATA) => Some(msg(Event::FromData)),
            Control::Press(MIGRATE) => Some(msg(Event::Migrate)),
            other => return Err(format!("“{FIELDS_TITLE}” penceresinde {other} yok")),
        })
    }
}

/// Bölünce and Birleşince of the chosen row (docs/adr/0419): only what its kind takes.
fn policy_box<'a>(w: &Window) -> Element<'a, Message> {
    use kentos_commands::{MergePolicy, SplitPolicy};
    let Some((i, r)) = w.chosen.and_then(|i| w.rows.get(i).map(|r| (i, r))) else {
        return iced::widget::space().into();
    };
    let units = matches!(r.kind, LayerFieldKind::Integer | LayerFieldKind::BigInt)
        || (r.kind == LayerFieldKind::Decimal && !r.size.trim().is_empty());
    let number = r.kind.is_number();
    let splits: Vec<SplitPolicy> = SplitPolicy::ALL
        .into_iter()
        .filter(|p| *p != SplitPolicy::Ratio || units)
        .collect();
    let merges: Vec<MergePolicy> = MergePolicy::ALL
        .into_iter()
        .filter(|p| match p {
            MergePolicy::Sum => units,
            MergePolicy::Min | MergePolicy::Max => number,
            _ => true,
        })
        .collect();
    let split_at = splits
        .iter()
        .position(|p| Some(*p) == r.split.or(Some(SplitPolicy::Copy)));
    let merge_at = merges
        .iter()
        .position(|p| Some(*p) == r.merge.or(Some(MergePolicy::First)));
    let split = {
        let picks = splits.clone();
        Select::new(
            splits.iter().map(|p| Choice::new(p.label())),
            split_at,
            move |k| {
                let p = picks[k];
                msg(Event::Split(i, (p != SplitPolicy::Copy).then_some(p)))
            },
        )
    };
    let merge = {
        let picks = merges.clone();
        Select::new(
            merges.iter().map(|p| Choice::new(p.label())),
            merge_at,
            move |k| {
                let p = picks[k];
                msg(Event::Merge(i, (p != MergePolicy::First).then_some(p)))
            },
        )
    };
    row![
        words::field(
            "Nesne bölününce",
            container(split).width(Length::Fixed(220.0)),
            None
        ),
        words::field(
            "Nesneler birleşince",
            container(merge).width(Length::Fixed(180.0)),
            None
        ),
    ]
    .spacing(16)
    .align_y(iced::Alignment::End)
    .into()
}

impl App {
    /// The layer's values written in their fields' canonical texts through `cad.fields.migrate`, as one
    /// undo step; the window says what changed and what did not fit (docs/adr/0439 §8).
    fn migrate_layer_values(&mut self, layer: &str) -> Task<Message> {
        let Some(doc) = self.document.as_mut() else {
            return Task::none();
        };
        let result = kentos_native_application::fields_migrate::execute(
            &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
            kentos_commands::FieldsMigrate {
                layer: layer.to_owned(),
                fields: Vec::new(),
                expected_revision: None,
            },
        );
        let (said, done) = match result {
            kentos_commands::CommandResult::Completed { output, warnings } => {
                let changed: u32 = output.fields.iter().map(|f| f.changed).sum();
                let unfit: u32 = output.fields.iter().map(|f| f.unfit).sum();
                let mut text = if changed == 0 {
                    "Değerler zaten alanların biçiminde; bir şey değişmedi.".to_owned()
                } else {
                    format!(
                        "{} kayıtta {changed} değer alanların biçimine çevrildi.",
                        output.objects
                    )
                };
                if unfit > 0 {
                    text.push_str(&format!(
                        " {unfit} değer alanına uymadığı için yazıldığı gibi kaldı."
                    ));
                }
                if !output.unknown.is_empty() {
                    text.push_str(&format!(
                        " Alanı olmayan öznitelikler korundu: {}.",
                        output.unknown.join(", ")
                    ));
                }
                let kind = if warnings.is_empty() {
                    Kind::Ok
                } else {
                    Kind::Warn
                };
                ((kind, text), changed > 0)
            }
            kentos_commands::CommandResult::Failed { error }
            | kentos_commands::CommandResult::Conflict { error }
            | kentos_commands::CommandResult::NeedsInput { error } => {
                ((Kind::Error, error.message), false)
            }
            _ => return Task::none(),
        };
        if done {
            self.say(kentos_interaction::Level::Success, said.1.clone());
        }
        if let Some(w) = self.layer_fields.as_mut() {
            w.said = Some(said);
        }
        Task::none()
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
        assert_eq!(
            (f.length, f.scale),
            (None, None),
            "an integer takes no size"
        );
        assert_eq!(f.min.as_deref(), Some("1"));
        assert_eq!(
            renames_of(&[r.clone()]),
            [("Kat".to_owned(), "Katlar".to_owned())]
        );
        r.kind = LayerFieldKind::Text;
        r.size = "x".into();
        assert!(problem_of(&[r]).is_some_and(|p| p.contains("uzunluğu bir tam sayı")));
    }
}
