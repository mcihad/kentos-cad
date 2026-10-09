//! Arama, the bottom panel's data search (docs/adr/0178; the web's
//! `ui/bottom/SearchPanel.ts`): the search box over every object's label,
//! text, block name and attributes, the fields asked for, the layer and the
//! three options, and the objects found as rows (Katman, Tür, Alan, Değer).
//! Which objects answer, by which field and in what order is the shared
//! core's (`ops::data_search`), the objects' words and the scope
//! `kentos_interaction::data_search`: the web shows the same rows.
//!
//! - A click on a row selects the object and zooms to it; Ctrl turns one
//!   over and Shift takes the run from the last click without Shift,
//!   neither zooming. Hepsini seç selects and fits every object found (not
//!   only the rows listed), Göster zooms to the selected ones; Enter in the
//!   box takes the first row.
//! - A coordinate typed in the box (Y,X, X,Y in a CAD project) shows Git:
//!   the view comes to it and the place is marked over the drawing
//!   (`marks.rs`), until İşareti kaldır, another place or another drawing.
//! - The choices (words, fields, options, layer, sort) are kept for as long
//!   as the app lives.
//!
//! The rows are worked out once for the drawing, the selection and the
//! choices as they are, not at every frame (docs/adr/0120); the objects'
//! words once for the drawing. The view is `view.rs`.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use iced::Task;
use kentos_domain::Slot;
use kentos_geometry_core::ops::data_search::{Fields, Found, Query, Record, data_search};
use kentos_interaction::Format;
use kentos_interaction::Vec2;
use kentos_interaction::data_search::{Index, attribute_names, in_scope, index, typed_place};

use crate::app::{App, Message};
use crate::bottom::BottomTab;
use crate::document::Document;
use crate::traces::Control;

mod view;

/// The search box's id: the keyboard goes to it when the tab opens (the trace
/// player follows it: traces/command_line.rs).
pub(crate) const SEARCH_FIELD: &str = "search-box";

/// Rows listed at most; the count line says how many answer in all.
pub const LIMIT: usize = 5000;

/// The panel's height Veride ara opens it to when it is shorter (logical
/// pixels, the web's `SEARCH_PANEL_HEIGHT`): the bar takes two lines and the
/// rows want room.
pub const PANEL_HEIGHT: f64 = 280.0;

/// The columns: their header and the core's sort key (none: Sıra, the drawing's order).
pub const COLUMNS: [(&str, Option<&str>); 5] = [
    ("Sıra", None),
    ("Katman", Some("layer")),
    ("Tür", Some("kind")),
    ("Alan", Some("field")),
    ("Değer", Some("value")),
];

/// The panel's words (the web's `SEARCH_TEXTS`).
pub mod texts {
    pub const SEARCH: &str = "Veride ara";
    pub const PLACEHOLDER: &str = "Ara: öznitelik, yazı, ad, blok adı — ya da Y,X";
    pub const SEARCH_HINT: &str = "Nesnelerin etiketinde, yazısında, blok adında ve özniteliklerinde arar. * herhangi bir dizidir: Ada *, *101. Bir koordinat yazarsanız (Y,X) oraya gider ve işaretler.";
    pub const ALL_LAYERS: &str = "Bütün katmanlar";
    pub const FIELDS: &str = "Ara:";
    pub const LABEL_FIELD: &str = "Ad / etiket";
    pub const LABEL_HINT: &str = "Nokta adı, parsel numarası gibi nesnenin etiketinde arar";
    pub const TEXT_FIELD: &str = "Yazı";
    pub const TEXT_HINT: &str =
        "Yazı nesnesinin metninde, kılavuzun notunda ve ölçünün kendi yazısında arar";
    pub const BLOCK_FIELD: &str = "Blok adı";
    pub const BLOCK_HINT: &str = "Blok yerleştirmelerinin bloğunun adında arar";
    pub const ATTRS_FIELD: &str = "Öznitelikler";
    pub const ATTRS_HINT: &str =
        "Öznitelik değerlerinde arar (nokta kodu ve blok öznitelikleri dahil)";
    pub const ALL_ATTRS: &str = "Bütün öznitelikler";
    pub const MATCH_CASE: &str = "Büyük küçük harf eşleşsin";
    pub const WHOLE_WORD: &str = "Tam sözcük";
    pub const WHOLE_WORD_HINT: &str =
        "Eşleşmenin iki yanında harf, rakam ya da _ olmaz (* varken geçmez)";
    pub const ONLY_SELECTED: &str = "Yalnız seçimde";
    pub const SELECT_ALL: &str = "Hepsini seç";
    pub const SELECT_ALL_HINT: &str = "Bulunan nesnelerin hepsini seçer ve ekrana sığdırır";
    pub const SHOW: &str = "Göster";
    pub const SHOW_HINT: &str = "Seçili sonuçlara yakınlaşır";
    pub const GO: &str = "Git";
    pub const GO_HINT: &str = "Görünümü bu koordinata getirir ve çizimde işaretler (Enter)";
    pub const COORDINATE: &str = "Koordinat";
    pub const MARK: &str = "İşaret";
    pub const UNMARK: &str = "İşareti kaldır";
    pub const UNMARK_HINT: &str = "Koordinata git’in çizimdeki işaretini kaldırır";
    pub const NO_QUERY: &str = "Aranacak sözü yazın: öznitelik değeri, yazı, nokta adı ya da blok adı. * herhangi bir dizidir; Y,X yazarsanız koordinata gider.";
    pub const NO_DATA: &str =
        "Çizimde aranabilir değer yok: ad, etiket, yazı, blok adı ya da öznitelik.";
    pub const NO_FIELDS: &str = "Aranacak alan seçilmedi: Ad / etiket, Yazı, Blok adı ya da Öznitelikler düğmelerinden birini açın.";

    pub fn no_match(word: &str) -> String {
        format!(
            "“{word}” ile eşleşen nesne yok. Alan düğmelerini, katmanı ve seçenekleri denetleyin."
        )
    }
}

/// A field the search looks in (the chips beside Ara:).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Label,
    Text,
    Block,
    Attrs,
}

/// The panel's messages.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// The box's words.
    Text(String),
    /// A field button, turned over.
    Field(Field),
    /// Öznitelik's choice: one attribute's name; none: all.
    AttrName(Option<String>),
    /// Katman's choice, by layer id; none: all.
    Layer(Option<String>),
    MatchCase(bool),
    WholeWord(bool),
    OnlySelected(bool),
    /// A header pressed, by its column.
    Sort(usize),
    /// A row pressed, by its place in the order shown.
    Press(usize),
    /// Hepsini seç.
    SelectAll,
    /// Göster.
    Show,
    /// Git.
    Go,
    /// Enter in the box: the place typed, else the first row.
    Submit,
    /// İşareti kaldır.
    Unmark,
}

/// The rows as worked out for a drawing, a selection and the choices.
#[derive(Clone, Debug, Default)]
pub(crate) struct Results {
    /// The objects with something to find, as of the drawing.
    index: Rc<Index>,
    /// The index positions the scope took.
    subset: Vec<usize>,
    /// The core's answer: the rows listed and how many objects answer.
    found: Found,
    /// The slot of each row, in the order shown.
    slots: Vec<Slot>,
    /// The layers that hold objects with something to find, in the layer
    /// list's order: id, name and count.
    layers: Vec<(String, String, usize)>,
    /// The attribute names the drawing carries.
    names: Vec<String>,
    /// Whether the box and the fields ask anything.
    asks: bool,
    /// The first selected row in the order shown.
    first_selected: Option<usize>,
}

/// The open drawing (its session), its changes, the selection's version and the choices'.
type Key = (u64, u64, u64, u64);

/// The objects' words and the drawing they were made for: its session and changes.
type Indexed = ((u64, u64), Rc<Index>);

/// The panel's state, kept for as long as the app lives.
pub(crate) struct SearchPanel {
    text: String,
    fields: Fields,
    match_case: bool,
    whole_word: bool,
    only_selected: bool,
    /// One layer by id; none: every layer.
    layer: Option<String>,
    sort: Option<&'static str>,
    descending: bool,
    /// The last click without Shift, in the order shown.
    anchor: Option<usize>,
    /// The tab opened: the keyboard goes to the box once.
    pub(crate) focus: bool,
    /// The place Koordinata git marked and the drawing it is in (its session).
    mark: Option<(u64, Vec2)>,
    /// Bumped at every change of the choices.
    version: u64,
    /// The objects' words, made again only when the drawing changes: its session and generation.
    indexed: RefCell<Option<Indexed>>,
    cache: RefCell<Option<(Key, Results)>>,
}

impl Default for SearchPanel {
    fn default() -> Self {
        Self {
            text: String::new(),
            fields: Fields {
                label: true,
                text: true,
                block: true,
                attrs: true,
                attr_name: None,
            },
            match_case: false,
            whole_word: false,
            only_selected: false,
            layer: None,
            sort: None,
            descending: false,
            anchor: None,
            focus: false,
            mark: None,
            version: 0,
            indexed: RefCell::default(),
            cache: RefCell::default(),
        }
    }
}

/// A header click: ascending, then descending, then the drawing's order;
/// Sıra is the drawing's order (the web's `nextSort`).
pub fn next_sort(
    sort: Option<&'static str>,
    descending: bool,
    column: Option<&'static str>,
) -> (Option<&'static str>, bool) {
    match column {
        None => (None, false),
        Some(c) if sort == Some(c) && descending => (None, false),
        Some(c) => (Some(c), sort == Some(c)),
    }
}

/// A row's Alan: the field that answered, an attribute by its name, “+n”
/// for the others that answer too (the web's `fieldCell`).
pub fn field_cell(row: &kentos_geometry_core::ops::data_search::Row) -> String {
    let name = match row.field {
        "label" => texts::LABEL_FIELD,
        "text" => texts::TEXT_FIELD,
        "block" => texts::BLOCK_FIELD,
        _ => row.name.as_deref().unwrap_or(texts::ATTRS_FIELD),
    };
    if row.more > 0 {
        format!("{name} +{}", row.more)
    } else {
        name.to_owned()
    }
}

/// The count line: how many objects answer, and how many are listed when
/// the list is cut (the web's `countText`).
pub fn count_text(listed: usize, total: usize) -> String {
    if listed < total {
        format!("{listed} / {total} sonuç")
    } else {
        format!("{total} sonuç")
    }
}

impl SearchPanel {
    fn changed(&mut self) {
        self.version += 1;
    }

    fn query(&self, limit: usize) -> Query {
        Query {
            pattern: self.text.clone(),
            match_case: self.match_case,
            whole_word: self.whole_word,
            fields: self.fields.clone(),
            sort: self.sort.map(str::to_owned),
            descending: self.descending,
            limit,
        }
    }

    fn fields_on(&self) -> bool {
        let f = &self.fields;
        f.label || f.text || f.block || f.attrs
    }

    /// Whether the box's words ask something.
    fn asks(&self) -> bool {
        !self.text.trim().is_empty() && self.fields_on()
    }
}

impl App {
    /// The place the box's words name, in metres, if they name one.
    pub(crate) fn data_place(&self) -> Option<Vec2> {
        let doc = self.document.as_ref()?;
        let format = Format::of(doc.settings());
        typed_place(&self.search.text, |v| format.to_metres(v))
    }

    /// The place Koordinata git marked, in the open drawing.
    pub(crate) fn data_mark(&self) -> Option<Vec2> {
        let doc = self.document.as_ref()?;
        self.search
            .mark
            .filter(|(session, _)| *session == doc.session)
            .map(|(_, p)| p)
    }

    /// The objects' words for the drawing as it is, made again only when it changes.
    fn data_index(&self, doc: &Document) -> Rc<Index> {
        let key = (doc.session, doc.model.generation());
        if let Some((known, index)) = self.search.indexed.borrow().as_ref()
            && *known == key
        {
            return index.clone();
        }
        let mut made = index(&doc.model);
        // A layer's filter leaves out what it does not pass (docs/adr/0211 §1).
        if made.slots.iter().any(|s| !self.spatial.filter_shown(*s)) {
            let keep: Vec<bool> = made
                .slots
                .iter()
                .map(|s| self.spatial.filter_shown(*s))
                .collect();
            let mut k = keep.iter();
            made.slots.retain(|_| *k.next().unwrap_or(&true));
            let mut k = keep.iter();
            made.layer_ids.retain(|_| *k.next().unwrap_or(&true));
            let mut k = keep.iter();
            made.records.retain(|_| *k.next().unwrap_or(&true));
        }
        let made = Rc::new(made);
        *self.search.indexed.borrow_mut() = Some((key, made.clone()));
        made
    }

    /// The rows for the drawing, the selection and the choices as they are,
    /// worked out again only when one of them changed.
    pub(crate) fn data_rows(&self, doc: &Document) -> Results {
        let panel = &self.search;
        let key = (
            doc.session,
            doc.model.generation(),
            self.selection.version(),
            panel.version,
        );
        if let Some((known, rows)) = panel.cache.borrow().as_ref()
            && *known == key
        {
            return rows.clone();
        }
        let index = self.data_index(doc);
        let model = &doc.model;
        // A layer chosen that holds nothing to find any more shows all.
        let layer = panel
            .layer
            .clone()
            .filter(|l| index.layer_ids.iter().any(|id| id == l));
        let names = attribute_names(&index.records);
        let attr_name = panel.fields.attr_name.clone().filter(|n| names.contains(n));
        let selected = panel.only_selected.then(|| self.selection.ids());
        let subset = in_scope(&index, layer.as_deref(), selected);
        let asks = panel.asks();
        let mut query = panel.query(LIMIT);
        query.fields.attr_name = attr_name;
        let found = if !asks {
            Found {
                rows: Vec::new(),
                total: 0,
            }
        } else if subset.len() == index.records.len() {
            data_search(&index.records, &query)
        } else {
            let records: Vec<Record> = subset.iter().map(|&i| index.records[i].clone()).collect();
            data_search(&records, &query)
        };
        let slots: Vec<Slot> = found
            .rows
            .iter()
            .map(|r| index.slots[subset[r.record as usize]])
            .collect();
        // The layers, in the layer list's order.
        let mut counts: Vec<(String, usize)> = Vec::new();
        for id in &index.layer_ids {
            match counts.iter_mut().find(|(known, _)| known == id) {
                Some((_, n)) => *n += 1,
                None => counts.push((id.clone(), 1)),
            }
        }
        let order: Vec<&str> = model
            .layers()
            .leaves()
            .iter()
            .map(|n| n.id.as_str())
            .collect();
        counts.sort_by_key(|(id, _)| order.iter().position(|o| o == id).unwrap_or(usize::MAX));
        let layers = counts
            .into_iter()
            .map(|(id, n)| {
                let name = model
                    .layers()
                    .get(&id)
                    .map_or_else(|| id.clone(), |l| l.name.clone());
                (id, name, n)
            })
            .collect();
        let first_selected = slots.iter().position(|&s| self.selection.contains(s));
        let rows = Results {
            index,
            subset,
            found,
            slots,
            layers,
            names,
            asks,
            first_selected,
        };
        *panel.cache.borrow_mut() = Some((key, rows.clone()));
        rows
    }

    /// The rows shown now (none without a drawing).
    fn shown_data(&self) -> Vec<Slot> {
        self.document
            .as_ref()
            .map(|doc| self.data_rows(doc).slots)
            .unwrap_or_default()
    }

    /// `data.search`: the panel open on its Arama tab, grown to the height
    /// the results want when it is shorter, the keyboard in the box with
    /// its words chosen.
    pub(crate) fn open_data_search(&mut self) -> Task<Message> {
        self.show_bottom(BottomTab::Search);
        let bar = f64::from(kentos_ui::widget::tabs::height());
        if f64::from(self.bottom_log()) + bar < PANEL_HEIGHT {
            self.bottom_dragged(Some((PANEL_HEIGHT - bar) as f32), Instant::now());
        }
        self.search.focus = false;
        let id = iced::widget::Id::new(SEARCH_FIELD);
        Task::batch([
            iced::widget::operation::focus(id.clone()),
            iced::widget::operation::select_all(id),
        ])
    }

    pub(crate) fn data_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Text(text) => {
                self.search.text = text;
                self.search.changed();
            }
            Event::Field(field) => {
                let f = &mut self.search.fields;
                let on = match field {
                    Field::Label => &mut f.label,
                    Field::Text => &mut f.text,
                    Field::Block => &mut f.block,
                    Field::Attrs => &mut f.attrs,
                };
                *on = !*on;
                self.search.changed();
            }
            Event::AttrName(name) => {
                self.search.fields.attr_name = name;
                self.search.changed();
            }
            Event::Layer(layer) => {
                self.search.layer = layer;
                self.search.changed();
            }
            Event::MatchCase(on) => {
                self.search.match_case = on;
                self.search.changed();
            }
            Event::WholeWord(on) => {
                self.search.whole_word = on;
                self.search.changed();
            }
            Event::OnlySelected(on) => {
                self.search.only_selected = on;
                self.search.changed();
            }
            Event::Sort(column) => {
                let p = &mut self.search;
                (p.sort, p.descending) = next_sort(p.sort, p.descending, COLUMNS[column].1);
                p.changed();
            }
            Event::Press(at) => {
                let shown = self.shown_data();
                if at >= shown.len() {
                    return Task::none();
                }
                let (ctrl, shift) = (self.modifiers.control(), self.modifiers.shift());
                if ctrl || shift {
                    let ids = crate::points::click_pick(
                        self.selection.ids(),
                        &shown,
                        at,
                        self.search.anchor,
                        ctrl,
                        shift,
                    );
                    if !shift {
                        self.search.anchor = Some(at);
                    }
                    self.selection.set(ids);
                } else {
                    self.search.anchor = Some(at);
                    self.selection.set([shown[at]]);
                    self.navigating(Self::zoom_selection);
                }
            }
            Event::SelectAll => {
                let Some(doc) = self.document.as_ref() else {
                    return Task::none();
                };
                let rows = self.data_rows(doc);
                let all = if rows.found.total as usize > rows.found.rows.len() {
                    // Every object found, not only the rows listed.
                    let mut query = self.search.query(0);
                    query.fields.attr_name = self
                        .search
                        .fields
                        .attr_name
                        .clone()
                        .filter(|n| rows.names.contains(n));
                    let records: Vec<Record> = rows
                        .subset
                        .iter()
                        .map(|&i| rows.index.records[i].clone())
                        .collect();
                    data_search(&records, &query)
                        .rows
                        .iter()
                        .map(|r| rows.index.slots[rows.subset[r.record as usize]])
                        .collect()
                } else {
                    rows.slots.clone()
                };
                if all.is_empty() {
                    return Task::none();
                }
                self.selection.set(all);
                self.navigating(Self::zoom_selection);
            }
            Event::Show => self.navigating(Self::zoom_selection),
            Event::Go => self.data_go(),
            Event::Submit => {
                if self.data_place().is_some() {
                    self.data_go();
                } else if let Some(&slot) = self.shown_data().first() {
                    self.search.anchor = None;
                    self.selection.set([slot]);
                    self.navigating(Self::zoom_selection);
                }
            }
            Event::Unmark => self.search.mark = None,
        }
        Task::none()
    }

    /// Git: the view comes to the place typed, which is marked over the drawing.
    fn data_go(&mut self) {
        let (Some(p), Some(session)) =
            (self.data_place(), self.document.as_ref().map(|d| d.session))
        else {
            return;
        };
        self.search.mark = Some((session, p));
        self.navigating(|app| app.viewport.camera.center_on(p));
        let said = self
            .document
            .as_ref()
            .map(|d| Format::of(d.settings()).point(p));
        if let Some(said) = said {
            self.say(
                kentos_interaction::Level::Info,
                format!("Koordinata gidildi: {said}."),
            );
        }
    }

    /// The place marked, as the drawing shows it: the place and its coordinates' words.
    pub(crate) fn data_mark_label(&self) -> Option<(Vec2, String)> {
        let p = self.data_mark()?;
        let doc = self.document.as_ref()?;
        Some((p, Format::of(doc.settings()).point(p)))
    }

    /// The panel's controls by their words (a trace's `panel` step): the
    /// search box, the field buttons and boxes, the lists, a header, a row,
    /// the buttons.
    pub(crate) fn data_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        let msg = |e| Some(Message::Search(e));
        let doc = self.document.as_ref().ok_or("açık çizim yok")?;
        let rows = self.data_rows(doc);
        let panel = &self.search;
        let set = |now: bool, on: bool, event: Event| (now != on).then(|| Message::Search(event));
        Ok(match control {
            Control::Fill(texts::SEARCH, t) => msg(Event::Text(t.to_owned())),
            Control::Check(texts::LABEL_FIELD, on) => {
                set(panel.fields.label, on, Event::Field(Field::Label))
            }
            Control::Check(texts::TEXT_FIELD, on) => {
                set(panel.fields.text, on, Event::Field(Field::Text))
            }
            Control::Check(texts::BLOCK_FIELD, on) => {
                set(panel.fields.block, on, Event::Field(Field::Block))
            }
            Control::Check(texts::ATTRS_FIELD, on) => {
                set(panel.fields.attrs, on, Event::Field(Field::Attrs))
            }
            Control::Check(texts::MATCH_CASE, on) => {
                set(panel.match_case, on, Event::MatchCase(on))
            }
            Control::Check(texts::WHOLE_WORD, on) => {
                set(panel.whole_word, on, Event::WholeWord(on))
            }
            Control::Check(texts::ONLY_SELECTED, on) => {
                // Off while nothing is selected, unless it is on already.
                if self.selection.is_empty() && !panel.only_selected {
                    None
                } else {
                    set(panel.only_selected, on, Event::OnlySelected(on))
                }
            }
            Control::Pick("Katman", item) => {
                if item == texts::ALL_LAYERS {
                    msg(Event::Layer(None))
                } else {
                    let (id, _, _) = rows
                        .layers
                        .iter()
                        .find(|(_, name, n)| format!("{name} ({n})") == item)
                        .ok_or_else(|| format!("“{item}” öğesi (Katman) yok"))?;
                    msg(Event::Layer(Some(id.clone())))
                }
            }
            Control::Pick("Öznitelik", item) => {
                if panel.fields.attrs {
                    if item == texts::ALL_ATTRS {
                        msg(Event::AttrName(None))
                    } else if rows.names.iter().any(|n| n == item) {
                        msg(Event::AttrName(Some(item.to_owned())))
                    } else {
                        return Err(format!("“{item}” öğesi (Öznitelik) yok"));
                    }
                } else {
                    None
                }
            }
            Control::Sort(header) => {
                let column = COLUMNS
                    .iter()
                    .position(|(title, _)| *title == header)
                    .ok_or_else(|| format!("“{header}” başlığı yok"))?;
                msg(Event::Sort(column))
            }
            Control::Row(n) => {
                if n == 0 || n > rows.slots.len() {
                    return Err(format!("{n}. sonuç satırı yok"));
                }
                msg(Event::Press(n - 1))
            }
            Control::Press(texts::SELECT_ALL) => {
                (rows.found.total > 0).then(|| Message::Search(Event::SelectAll))
            }
            Control::Press(texts::SHOW) => {
                (!self.selection.is_empty()).then(|| Message::Search(Event::Show))
            }
            Control::Press(texts::GO) => self.data_place().map(|_| Message::Search(Event::Go)),
            Control::Press(texts::UNMARK) => {
                self.data_mark().map(|_| Message::Search(Event::Unmark))
            }
            Control::Key("Enter") => msg(Event::Submit),
            Control::Key("Esc") => {
                if panel.text.is_empty() {
                    None
                } else {
                    msg(Event::Text(String::new()))
                }
            }
            other => return Err(format!("“Arama” sekmesinde {other} yok")),
        })
    }

    /// What a trace's `search` expectation reads: the count line and the
    /// rows (Katman, Tür, Alan, Değer, joined by “ | ”), as the page shows them.
    pub(crate) fn data_seen(&self) -> Option<(String, Vec<String>)> {
        if !self.command_expanded || self.bottom_tab != BottomTab::Search {
            return None;
        }
        let doc = self.document.as_ref()?;
        let rows = self.data_rows(doc);
        let count = if rows.found.total > 0 || !self.search.text.trim().is_empty() {
            count_text(rows.found.rows.len(), rows.found.total as usize)
        } else {
            String::new()
        };
        let lines = rows
            .found
            .rows
            .iter()
            .map(|r| {
                let record = &rows.index.records[rows.subset[r.record as usize]];
                let alan = field_cell(r);
                [
                    record.layer.as_str(),
                    record.kind.as_str(),
                    alan.as_str(),
                    r.value.as_str(),
                ]
                .join(" | ")
            })
            .collect();
        Some((count, lines))
    }
}

#[cfg(test)]
mod tests;
