//! Öznitelik tablosu on a data layer (docs/adr/9102 §5, plan/veri.md VY-04): the rows are read from the layer's
//! source a page at a time (`cad.layers.rows`' reader, `layers_rows::read`, off the interface's thread), only the
//! pages the table shows. Ara, Göster and İfade süzgeci are the panel's own; a header press orders by its field (a text
//! in the Turkish alphabet's order), then the other way, then by key. Without a filter the count, the order and the
//! pages are the source's; with one, its first page brings every kept key and the later pages are read by key.
//! Göster's Görünümdekiler reads the view's box, read again when the view has moved; Seçililer reads the rows pressed.
//! A row press selects its row (Ctrl turns one over). In a layer being edited a double-clicked cell is edited into the
//! session (docs/adr/9103 §5); once the layer's changes are written or let go the table reads its rows again where it is,
//! the rows read before shown until the new ones come.
//!
//! Ara and İfade süzgeci are asked when Enter is pressed: a search or a filter of a million rows is not asked at every
//! letter. A new question stops the reads of the one before it (their scans end at the next part).

use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use iced::widget::{column, container, row, text};
use iced::{Center, Element, Fill, Length, Task};
use kentos_commands::{LayerNode, LayerRow, LayerRowsAnswer, LayersRows, QueryOrder, RowsChecked};
use kentos_ui::label;
use kentos_ui::widget::table::{Column as TableColumn, Row as TableLine, SortOrder, Table};

use super::{Event, Show};
use crate::app::{App, Message};
use crate::gis;

/// Rows a page holds.
const PAGE: u64 = 200;
/// Pages kept: past it the pages farthest from the view are let go.
const KEPT: usize = 40;

/// The words.
pub mod words {
    pub const READING: &str = "Kaynaktan okunuyor…";
    pub const SOURCE: &str = "süzgeç ve sıra kaynakta";
    /// A new row's action: its record's form.
    pub const ENTER_ATTRS: &str = "Öznitelikleri gir";
    pub const HERE: &str = "süzgeç burada soruldu";
    pub const NONE: &str = "Kaynakta soruya uyan satır yok.";
    /// The question refused, its reason where the rows would be.
    pub const UNREAD: &str = "Satırlar okunmadı.";
    /// A new row's key cell: it has none until it is saved.
    pub const NEW: &str = "yeni";
    /// A pending changed or deleted row's mark beside its key while the source is unread (docs/adr/9151).
    pub const CHANGED: &str = "değişen";
    pub const DELETED: &str = "silinen";
    /// A taken row its source no longer has (docs/adr/9152).
    pub const GONE: &str = "kaynakta silinmiş";
    pub fn count(n: u64) -> String {
        format!("{} satır", crate::view::thousands(n as f64))
    }
    pub const SEARCH: &str = "Ara (Enter)";
    pub const FILTER: &str = "İfade süzgeci (Enter)";
}

/// What the pages are of.
#[derive(Clone, Debug, PartialEq)]
struct Asked {
    layer: String,
    search: String,
    show: Show,
    filter: String,
    order: Vec<(String, bool)>,
    /// Görünümdekiler's box, rounded to a metre.
    view: Option<[i64; 4]>,
    /// Seçililer's keys.
    selected: Vec<i64>,
    /// The layer's fields: one added or taken off asks again, the new column read at once (docs/adr/9348).
    fields: Vec<String>,
}

/// The data layer's table.
#[derive(Debug, Default)]
pub struct DataTable {
    asked: Option<Asked>,
    generation: u64,
    fields: Vec<String>,
    total: Option<u64>,
    keys: Option<Vec<i64>>,
    checked: Option<RowsChecked>,
    pages: HashMap<u64, Vec<LayerRow>>,
    reading: HashSet<u64>,
    /// Pages read before the layer's changes were written or let go: shown until they are read again.
    stale: HashSet<u64>,
    /// The first page read again for the count and the kept keys (after a write).
    recount: bool,
    /// The drawing's writes the pages were read after (`Document::data_written`).
    written: u64,
    range: (usize, usize),
    error: Option<String>,
    /// The layer's fields its table does not have (docs/adr/9318), from the first page's answer: shown empty, named.
    missing: Vec<String>,
    /// The order the headers give: the field and whether it goes down.
    order: Vec<(String, bool)>,
    /// The rows pressed, by key, and the layer they are of.
    pub selected: BTreeSet<i64>,
    selected_of: Option<String>,
    /// Ara and İfade süzgeci as Enter asked them.
    search: String,
    filter: String,
    /// Kaynağın tümü (docs/adr/9348): the layer's Katman süzgeci left out, every row of the source shown; by default
    /// the table shows the filtered layer.
    pub(crate) source_all: bool,
    /// The stop switch of the question's reads.
    stop: Arc<AtomicBool>,
    /// The cell being edited (docs/adr/9103 §5): the row's key and the field, with the text typed.
    editing: Option<(i64, String)>,
    edit_text: String,
    /// Why the cell's value was refused, said beside it while it stays open (docs/adr/9323).
    edit_error: Option<String>,
    /// The last edit sent to take its row: an answer to an earlier one is stale.
    edit_asked: u64,
    /// The reads started, as a test answers them (the core's tests and a module package's, `testing`).
    #[cfg(any(test, feature = "testing"))]
    inputs: Vec<(u64, u64, LayersRows)>,
}

/// The table's messages.
#[derive(Debug, Clone, PartialEq)]
pub enum DataEvent {
    /// The rows the table shows (first, last excluded).
    Range(usize, usize),
    /// A header pressed: its field.
    Sort(String),
    /// A page read: the question's generation, the page, the answer or why not.
    Answered(u64, u64, Result<Box<LayerRowsAnswer>, String>),
    /// A row pressed: its key and whether Ctrl turns it over.
    Press(i64, bool),
    /// A new row not saved yet pressed: its object chosen in the drawing (docs/adr/9111 §1).
    PressNew(kentos_domain::Uuid),
    /// Öznitelikleri gir on a new row (docs/adr/9346): its object chosen and its record's form opened.
    EnterNew(kentos_domain::Uuid),
    /// Enter in Ara or İfade süzgeci: what is typed is asked.
    Apply,
    /// Seçime yakınlaş: the selected rows framed.
    Zoom,
    /// A cell double-clicked while the layer is being edited: the row's key and the field.
    Edit(i64, String),
    /// The cell's text typed.
    Input(String),
    /// Enter: the value written into the layer's edit session.
    Enter,
    /// A listed field's code chosen in its cell (docs/adr/9323): written as Enter writes a typed value; empty for none.
    Choose(String),
    /// Esc, or a press elsewhere: the edit given up.
    Cancel,
    /// The row of an edit read to be taken: the edit's number, the key, the field, the value, the row read or why not.
    Taken(u64, i64, String, Option<String>, Result<Vec<(i64, kentos_commands::Entity)>, String>),
}

/// An edited cell's row read off the thread to be taken (data_write.rs).
fn data_write_task(
    app: &mut App,
    node: LayerNode,
    settings: kentos_commands::ProjectSettings,
    key: i64,
    field: String,
    value: Option<String>,
) -> Task<Message> {
    app.gis.features.data.edit_asked += 1;
    let asked = app.gis.features.data.edit_asked;
    crate::db_connections::off_thread(
        move || kentos_native_application::data_write::rows_to_take(&node, &settings, &[key], &|| false),
        move |rows| msg(DataEvent::Taken(asked, key, field, value, rows)),
    )
}

fn msg(e: DataEvent) -> Message {
    Message::Gis(gis::Event::Features(Event::Data(e)))
}

impl DataTable {
    /// How many rows the question keeps, once counted.
    #[cfg(test)]
    pub(crate) fn total_rows(&self) -> Option<u64> {
        self.total
    }

    /// The row at a place of the order, when its page is read.
    #[cfg(test)]
    pub(crate) fn row(&self, at: usize) -> Option<&LayerRow> {
        self.pages.get(&(at as u64 / PAGE))?.get((at as u64 % PAGE) as usize)
    }

    /// The reads under way and their inputs (a test answers them itself).
    #[cfg(any(test, feature = "testing"))]
    pub fn pending(&mut self) -> Vec<(u64, u64, LayersRows)> {
        std::mem::take(&mut self.inputs)
    }
}

/// Ara, İfade süzgeci and the order as Enter and the headers asked them, kept by layer (conditions.rs).
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct DataConditions {
    search: String,
    filter: String,
    order: Vec<(String, bool)>,
}

impl DataTable {
    /// The conditions asked, taken out: the table asks none until others are put.
    pub(super) fn take_conditions(&mut self) -> DataConditions {
        DataConditions {
            search: std::mem::take(&mut self.search),
            filter: std::mem::take(&mut self.filter),
            order: std::mem::take(&mut self.order),
        }
    }

    /// A layer's conditions asked again.
    pub(super) fn put_conditions(&mut self, c: DataConditions) {
        (self.search, self.filter, self.order) = (c.search, c.filter, c.order);
    }

    /// The question's error, when the source refused it or could not be read.
    #[cfg(test)]
    pub(crate) fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// The layer's fields the table's source does not have (docs/adr/9318).
    #[cfg(test)]
    pub(crate) fn missing(&self) -> &[String] {
        &self.missing
    }

    /// The question read again where the table is, its layer's changes written or let go (a taken row then shows what
    /// the table holds): the pages stay shown until their new reads come, the first page read again for the count and,
    /// filtered, the kept keys the later pages wait for.
    fn reread(&mut self) {
        self.generation += 1;
        self.stop.store(true, Ordering::Relaxed);
        self.stop = Arc::new(AtomicBool::new(false));
        self.reading.clear();
        self.stale = self.pages.keys().copied().collect();
        self.keys = None;
        self.recount = true;
        self.error = None;
    }

    /// The cell being edited: its row's key, its field and the text typed.
    #[cfg(test)]
    pub(crate) fn cell_editing(&self) -> Option<(i64, String, String)> {
        self.editing.clone().map(|(k, f)| (k, f, self.edit_text.clone()))
    }

    /// Why the open cell's value was refused.
    #[cfg(test)]
    pub(crate) fn cell_error(&self) -> Option<String> {
        self.edit_error.clone()
    }

    /// The rows pressed if they are of this layer (Seçilenleri farklı kaydet, docs/adr/9106): none of another's.
    pub(crate) fn selected_of(&self, layer: &str) -> Vec<i64> {
        if self.any_selected_of(layer) { self.selected.iter().copied().collect() } else { Vec::new() }
    }

    /// The layer the table's pressed rows are of, when rows are pressed.
    pub(crate) fn selected_layer(&self) -> Option<&str> {
        self.selected_of.as_deref().filter(|_| !self.selected.is_empty())
    }

    /// Whether rows of this layer are pressed.
    pub(crate) fn any_selected_of(&self, layer: &str) -> bool {
        self.selected_of.as_deref() == Some(layer) && !self.selected.is_empty()
    }

    /// The layer the table's selection is of.
    #[cfg(any(test, feature = "testing"))]
    pub fn selected_of_for_test(&self) -> Option<&str> {
        self.selected_of.as_deref()
    }

    /// Seçimi kaldır (docs/adr/9170): no row chosen; the table stays on its layer.
    pub(crate) fn clear_selection(&mut self) {
        self.selected.clear();
    }

    /// The rows İfadeyle seç chose on a data layer (processing/data_select.rs): the table's selection, of that layer.
    pub(crate) fn choose_rows(&mut self, layer: &str, keys: BTreeSet<i64>) {
        self.selected_of = Some(layer.to_owned());
        self.selected = keys;
    }

    /// Rows taken out of the table's selection of `layer` (a press with Ctrl on counted values, data_counts.rs).
    pub(crate) fn unselect_rows(&mut self, layer: &str, keys: &[i64]) {
        if self.selected_of.as_deref() != Some(layer) {
            return;
        }
        for k in keys {
            self.selected.remove(k);
        }
    }

    /// Seç's rows of a data layer that is not being edited (data_pick.rs): the table's selection, of that layer.
    pub(crate) fn select_rows(&mut self, layer: &str, keys: Vec<i64>) {
        if self.selected_of.as_deref() != Some(layer) {
            self.selected.clear();
            self.selected_of = Some(layer.to_owned());
        }
        self.selected.extend(keys);
    }
}

impl App {
    /// The data layer the table shows while the Tablo tab is open, if the layer it shows is one.
    pub(crate) fn data_table_layer(&self) -> Option<LayerNode> {
        if !(self.command_expanded && self.bottom_tab == crate::bottom::BottomTab::Table) {
            return None;
        }
        let doc = self.document.as_ref()?;
        let layer = self.feature_layer(doc)?;
        let node = doc.model.layers().get(&layer)?;
        node.data.is_some().then(|| node.clone())
    }

    /// What the table asks now.
    fn data_asked(&self, node: &LayerNode) -> Asked {
        let panel = &self.gis.features;
        let data = &panel.data;
        let view = (panel.show == Show::InView).then(|| {
            let b = self.viewport.camera.visible_bounds();
            [b.min_x, b.min_y, b.max_x, b.max_y].map(|v| v.round() as i64)
        });
        let selected = if panel.show == Show::Selected && data.selected_of.as_deref() == Some(node.id.as_str()) {
            data.selected.iter().copied().collect()
        } else {
            Vec::new()
        };
        Asked {
            layer: node.id.clone(),
            search: data.search.clone(),
            show: panel.show,
            filter: match (self.table_layer_filter(node), data.filter.trim()) {
                (Some(layer), "") => layer,
                (Some(layer), own) => format!("({layer}) ve ({own})"),
                (None, _) => data.filter.clone(),
            },
            order: data.order.clone(),
            view,
            selected,
            fields: node.fields.iter().map(|f| f.name.clone()).collect(),
        }
    }

    /// The layer's Katman süzgeci the table applies (docs/adr/9348): none with Kaynağın tümü or without one.
    pub(crate) fn table_layer_filter(&self, node: &LayerNode) -> Option<String> {
        if self.gis.features.data.source_all {
            return None;
        }
        node.filter.as_ref().and_then(|f| f.expression.clone()).filter(|t| !t.trim().is_empty())
    }

    /// After every message while the table shows a data layer: a new question starts again from its first page, and
    /// the pages the table shows are read.
    pub(crate) fn data_table_tasks(&mut self) -> Task<Message> {
        // Each layer's own conditions, whichever way the table came to it (conditions.rs, docs/adr/9132).
        self.table_conditions_follow();
        let Some(node) = self.data_table_layer() else {
            return Task::none();
        };
        let asked = self.data_asked(&node);
        let written = self.document.as_ref().map_or(0, |d| d.model.data_written());
        let data = &mut self.gis.features.data;
        if data.asked.as_ref() == Some(&asked) && data.written != written {
            data.reread();
        }
        data.written = written;
        if data.asked.as_ref() != Some(&asked) {
            if data.selected_of.as_deref() != Some(node.id.as_str()) {
                data.selected.clear();
                data.selected_of = Some(node.id.clone());
            }
            data.generation += 1;
            data.stop.store(true, Ordering::Relaxed);
            data.stop = Arc::new(AtomicBool::new(false));
            data.asked = Some(asked.clone());
            data.pages.clear();
            data.reading.clear();
            data.stale.clear();
            data.recount = false;
            data.total = None;
            data.keys = None;
            data.checked = None;
            data.error = None;
        }
        // A refused question is not asked again by itself after every message (a source that cannot answer would be
        // asked without end): Enter asks it again, as another question or a write does.
        if data.error.is_some() {
            return Task::none();
        }
        let (first, last) = data.range;
        let mut wanted: Vec<u64> = (first as u64 / PAGE..=(last.max(first + 1) as u64 - 1) / PAGE).collect();
        if (data.total.is_none() || data.recount) && !wanted.contains(&0) {
            wanted.insert(0, 0);
        }
        let Some(doc) = self.document.as_ref() else {
            return Task::none();
        };
        let settings = doc.model.settings().clone();
        let data = &mut self.gis.features.data;
        let mut tasks = Vec::new();
        for page in wanted {
            if (data.pages.contains_key(&page) && !data.stale.contains(&page)) || data.reading.contains(&page) {
                continue;
            }
            let filtered = !asked.filter.is_empty();
            // A filtered question's later pages are read by key, once its first page has brought them.
            let keys = match (filtered, page, &data.keys) {
                (true, 0, _) => None,
                (true, _, None) => continue,
                (true, _, Some(all)) => {
                    let from = ((page * PAGE) as usize).min(all.len());
                    let to = (from + PAGE as usize).min(all.len());
                    Some(all[from..to].to_vec())
                }
                (false, ..) => None,
            };
            let by_keys = keys.is_some();
            let input = LayersRows {
                layer: asked.layer.clone(),
                bbox: asked.view.map(|v| v.map(|x| x as f64)),
                keys: keys.or_else(|| (asked.show == Show::Selected).then(|| asked.selected.clone())),
                search: (!asked.search.is_empty() && !by_keys).then(|| asked.search.clone()),
                filter: (filtered && !by_keys).then(|| asked.filter.clone()),
                order: (!asked.order.is_empty()).then(|| {
                    asked
                        .order
                        .iter()
                        .map(|(field, descending)| QueryOrder {
                            field: field.clone(),
                            descending: *descending,
                        })
                        .collect()
                }),
                offset: (!by_keys).then_some(page * PAGE),
                limit: Some(PAGE as u32),
                count: (page == 0).then_some(true),
                all_keys: (filtered && page == 0).then_some(true),
                ..LayersRows::default()
            };
            data.reading.insert(page);
            let generation = data.generation;
            #[cfg(any(test, feature = "testing"))]
            data.inputs.push((generation, page, input.clone()));
            let node = node.clone();
            let settings = settings.clone();
            let stop = Arc::clone(&data.stop);
            tasks.push(crate::db_connections::off_thread(
                move || {
                    kentos_native_application::layers_rows::read(&node, &settings, &input, &|| {
                        stop.load(Ordering::Relaxed)
                    })
                },
                move |r| {
                    msg(DataEvent::Answered(
                        generation,
                        page,
                        r.map(Box::new).map_err(|e| e.message),
                    ))
                },
            ));
        }
        Task::batch(tasks)
    }

    pub(crate) fn data_table_event(&mut self, event: DataEvent) -> Task<Message> {
        if event == DataEvent::Zoom {
            self.zoom_to_data_selection();
            return Task::none();
        }
        if let DataEvent::PressNew(uid) = event {
            if let Some(slot) = self.document.as_ref().and_then(|d| d.model.slot_of(uid)) {
                self.selection.set([slot]);
            }
            return Task::none();
        }
        if let DataEvent::EnterNew(uid) = event {
            let Some(slot) = self.document.as_ref().and_then(|d| d.model.slot_of(uid)) else {
                return Task::none();
            };
            self.selection.set([slot]);
            return self.run("data.formEdit");
        }
        if matches!(
            event,
            DataEvent::Edit(..)
            | DataEvent::Input(_)
            | DataEvent::Enter
            | DataEvent::Choose(_)
            | DataEvent::Cancel
            | DataEvent::Taken(..)
        ) {
            return self.data_cell_event(event);
        }
        let data = &mut self.gis.features.data;
        match event {
            DataEvent::Range(first, last) => data.range = (first, last),
            DataEvent::Sort(field) => {
                // Up, then down, then by key.
                let next = match data.order.first() {
                    Some((f, false)) if *f == field => Some(true),
                    Some((f, true)) if *f == field => None,
                    _ => Some(false),
                };
                data.order = next.map(|d| vec![(field, d)]).unwrap_or_default();
            }
            DataEvent::Answered(generation, page, answer) => {
                if generation != data.generation {
                    return Task::none();
                }
                data.reading.remove(&page);
                data.stale.remove(&page);
                if page == 0 {
                    data.recount = false;
                }
                match answer {
                    Ok(a) => {
                        data.error = None;
                        // The source answered: the layer checked now (docs/adr/9163).
                        if let Some(asked) = &data.asked {
                            self.gis.data_edits.verified.insert(asked.layer.clone(), crate::data_edits::now_ms());
                        }
                        if data.fields.is_empty() || page == 0 {
                            data.fields = a.fields.clone();
                        }
                        if a.total.is_some() {
                            data.total = a.total;
                        }
                        if a.keys.is_some() {
                            data.keys = a.keys.clone();
                        }
                        if page == 0 {
                            data.checked = Some(a.checked);
                            data.missing = a.missing.clone();
                        }
                        data.pages.insert(page, a.rows);
                        // The pages farthest from the view let go.
                        if data.pages.len() > KEPT {
                            let at = data.range.0 as u64 / PAGE;
                            let mut far: Vec<u64> = data.pages.keys().copied().collect();
                            far.sort_by_key(|p| std::cmp::Reverse(p.abs_diff(at)));
                            for p in far.into_iter().take(data.pages.len() - KEPT) {
                                data.pages.remove(&p);
                                data.stale.remove(&p);
                            }
                        }
                    }
                    Err(why) => data.error = Some(why),
                }
            }
            DataEvent::Zoom | DataEvent::PressNew(_) | DataEvent::EnterNew(_) => {}
            DataEvent::Apply => {
                data.search = self.gis.features.search.trim().to_owned();
                data.filter = self.gis.features.filter.trim().to_owned();
                // The same question refused before is asked again.
                if data.error.is_some() {
                    data.asked = None;
                }
            }
            DataEvent::Press(key, toggle) => {
                if toggle {
                    if !data.selected.remove(&key) {
                        data.selected.insert(key);
                    }
                } else {
                    data.selected = BTreeSet::from([key]);
                }
            }
            // A cell's edit: data_cell_event above.
            DataEvent::Edit(..)
            | DataEvent::Input(_)
            | DataEvent::Enter
            | DataEvent::Choose(_)
            | DataEvent::Cancel
            | DataEvent::Taken(..) => {}
        }
        Task::none()
    }

    /// A cell edited into the layer's edit session (docs/adr/9103 §5): a row taken already gets the value at once (an
    /// undo step, `cad.entities.set`); another is read off the thread, taken, then given it.
    fn data_cell_event(&mut self, event: DataEvent) -> Task<Message> {
        let Some(node) = self.data_table_layer() else {
            return Task::none();
        };
        match event {
            DataEvent::Edit(key, field) => {
                let editing = self.document.as_ref().is_some_and(|d| d.model.editing(&node.id));
                if !editing {
                    // Why, and the way to open it one press away, in a notice that is read whole (docs/adr/9142), as
                    // a drawing tool says it (docs/adr/9128).
                    let why = format!(
                        "“{}” düzenlemeye açık değil; bir değeri değiştirmek için önce katmanın düzenleme oturumunu \
                         başlatın.",
                        node.name
                    );
                    self.warn(why.clone());
                    self.toast(
                        kentos_ui::widget::Toast::info("Katman düzenlemeye kapalı")
                            .body(why)
                            .action("Düzenlemeyi başlat", crate::data_edits::event(crate::data_edits::Event::Start(node.id.clone()))),
                    );
                    return Task::none();
                }
                let shown = self.data_cell_value(&node, key, &field).unwrap_or_default();
                let data = &mut self.gis.features.data;
                data.editing = Some((key, field));
                data.edit_text = shown;
                data.edit_error = None;
                crate::points::focus_field()
            }
            DataEvent::Input(text) => {
                self.gis.features.data.edit_text = text;
                self.gis.features.data.edit_error = None;
                Task::none()
            }
            DataEvent::Cancel => {
                self.gis.features.data.editing = None;
                self.gis.features.data.edit_error = None;
                Task::none()
            }
            DataEvent::Choose(code) => {
                self.gis.features.data.edit_text = code;
                self.data_cell_event(DataEvent::Enter)
            }
            DataEvent::Enter => {
                let data = &mut self.gis.features.data;
                let Some((key, field)) = data.editing.take() else {
                    return Task::none();
                };
                let text = std::mem::take(&mut data.edit_text);
                data.edit_error = None;
                let value = (!text.trim().is_empty()).then_some(text);
                let Some(doc) = self.document.as_ref() else {
                    return Task::none();
                };
                if let Some(uid) = doc.model.taken_uid(&node.id, key) {
                    self.write_data_cell(uid, key, field, value);
                    return Task::none();
                }
                let settings = doc.model.settings().clone();
                data_write_task(self, node, settings, key, field, value)
            }
            DataEvent::Taken(asked, key, field, value, rows) => {
                if asked != self.gis.features.data.edit_asked {
                    return Task::none();
                }
                let rows = match rows {
                    Ok(rows) if !rows.is_empty() => rows,
                    Ok(_) => {
                        self.warn(format!("{key} anahtarlı satır tabloda yok."));
                        return Task::none();
                    }
                    Err(why) => {
                        self.warn(why);
                        return Task::none();
                    }
                };
                let Some(doc) = self.document.as_mut() else {
                    return Task::none();
                };
                match doc.model.take_data_rows(&node.id, rows) {
                    Ok(uids) => {
                        if let Some(uid) = uids.into_iter().flatten().next() {
                            self.write_data_cell(uid, key, field, value);
                        }
                    }
                    Err(refusal) => self.warn(refusal.0),
                }
                Task::none()
            }
            _ => Task::none(),
        }
    }

    /// The value written to a taken row's object, as Öznitelik tablosu writes a drawing's (blank takes it away). A
    /// refused value keeps the cell open with what was typed and its reason beside it (docs/adr/9323).
    fn write_data_cell(&mut self, uid: kentos_domain::Uuid, key: i64, field: String, value: Option<String>) {
        let input = kentos_commands::EntitiesSetProperties {
            line_pattern: None,
            line_type: None,
            line_type_scale: None,
            transparency: None,
            wipeout: None,
            uids: vec![uid.to_string()],
            layer_id: None,
            color: None,
            line_weight: None,
            symbol: None,
            attrs: Some([(field.clone(), value.clone())].into_iter().collect()),
            label: None,
            unlink: false,
            name_text: false,
            operation: kentos_commands::PropertiesOperation::Attributes,
            expected_revision: None,
            attachments: None,
        };
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        if let Err(why) = super::write_attribute(&mut doc.model, input) {
            self.warn(why.clone());
            let data = &mut self.gis.features.data;
            data.editing = Some((key, field));
            data.edit_text = value.unwrap_or_default();
            data.edit_error = Some(why);
        }
    }

    /// What a cell shows: a taken row's value as its object holds it now, else the page's.
    fn data_cell_value(&self, node: &LayerNode, key: i64, field: &str) -> Option<String> {
        let doc = self.document.as_ref()?;
        if let Some(uid) = doc.model.taken_uid(&node.id, key) {
            let slot = doc.model.slot_of(uid)?;
            return doc.model.get(slot)?.base().attrs.get(field).cloned();
        }
        let data = &self.gis.features.data;
        data.pages
            .values()
            .flat_map(|p| p.iter())
            .find(|r| r.key == key)
            .and_then(|r| r.attrs.get(field).cloned())
    }

    /// The table's line as it reads with no session and no selection (a test's question).
    #[cfg(test)]
    pub(crate) fn data_table_line(&self) -> Option<String> {
        let data = &self.gis.features.data;
        let n = data.total?;
        let loaded: usize = data.pages.values().map(Vec::len).sum();
        let read = if (loaded as u64) < n { format!(" ({} yüklendi)", crate::view::thousands(loaded as f64)) } else { String::new() };
        Some(format!(
            "{}{read} · {}",
            words::count(n),
            match data.checked {
                Some(RowsChecked::Here) => words::HERE,
                _ => words::SOURCE,
            }
        ))
    }

    /// What the table's foot says of a layer's rows its source does not have yet (docs/adr/9346).
    pub(crate) fn data_unwritten_words(&self, layer: &str) -> Option<String> {
        let c = self.document.as_ref()?.model.data_changes(layer);
        (!c.is_empty()).then(|| unwritten_words(c.added.len(), c.changed.len(), c.deleted.len()))
    }

    /// The fields the table shows: the page's (else the layer's), less those Sütunlar hides, as on a drawing layer's
    /// table (docs/adr/9166), and less those the caller's role does not read (docs/adr/9150, 9188).
    pub(crate) fn data_shown_fields(&self, node: &LayerNode) -> Vec<String> {
        let data = &self.gis.features.data;
        let hidden = self.hidden_columns(Some(&node.id));
        let withheld = self.withheld_fields(&node.id);
        let fields: Vec<String> = if data.fields.is_empty() {
            node.fields.iter().map(|f| f.name.clone()).collect()
        } else {
            data.fields.clone()
        };
        fields.into_iter().filter(|f| !hidden.contains(f) && !withheld.contains(f)).collect()
    }

    /// The table's rows: a page's cells when it is read, else a row waiting for it.
    pub(crate) fn data_table_view(&self, node: &LayerNode) -> Element<'_, Message> {
        let data = &self.gis.features.data;
        let fields = self.data_shown_fields(node);
        // Wide enough for a key with its mark (“22 kaynakta silinmiş”) when the source lost a taken row (docs/adr/9152).
        let marked = self.gis.data_edits.gone.get(&node.id).is_some_and(|k| !k.is_empty());
        let key_width = if marked { 160.0 } else { 80.0 };
        let mut columns = vec![TableColumn::new("Anahtar").width(Length::Fixed(key_width)).align_right()];
        for f in &fields {
            let order = data.order.first().filter(|(o, _)| o == f).map(|(_, d)| {
                if *d {
                    SortOrder::Descending
                } else {
                    SortOrder::Ascending
                }
            });
            // The field's alias in the head (docs/adr/9348); sorting, conditions and writes go by its name.
            let head = node.fields.iter().find(|d| &d.name == f).map_or_else(|| f.clone(), |d| d.label().to_owned());
            columns.push(
                TableColumn::new(head)
                    .width(Length::Fixed(140.0))
                    .sortable(order, msg(DataEvent::Sort(f.clone()))),
            );
        }
        let pages = &data.pages;
        let selected = &data.selected;
        let shown = fields.clone();
        // The fields' rules, by name: a code shows its label (docs/adr/9188).
        let rules: HashMap<String, kentos_commands::LayerField> =
            node.fields.iter().map(|f| (f.name.clone(), f.clone())).collect();
        // An edit session's rows (docs/adr/9103 §5): a taken one shows its object's values, a deleted one fades.
        let model = self.document.as_ref().map(|d| &d.model);
        let editing_layer = model.is_some_and(|m| m.editing(&node.id));
        let taken: HashMap<i64, std::sync::Arc<kentos_commands::Entity>> = if editing_layer {
            model.map(|m| m.taken_objects(&node.id).into_iter().collect()).unwrap_or_default()
        } else {
            HashMap::new()
        };
        let changes = model.filter(|_| editing_layer).map(|m| m.data_changes(&node.id));
        let deleted: HashSet<i64> = changes.as_ref().map(|c| c.deleted.iter().copied().collect()).unwrap_or_default();
        // Its new rows, not saved yet, come first and marked “yeni” (docs/adr/9111 §1): the source's rows follow them, their
        // pages read for the places below the new ones.
        let added = changes.as_ref().map(|c| c.added.clone()).unwrap_or_default();
        let chosen: HashSet<kentos_domain::Uuid> = model
            .map(|m| self.selection.ids().iter().filter_map(|s| m.uid(*s)).collect())
            .unwrap_or_default();
        let new_rows = added.len();
        // The source unread (offline, the file gone): the session's changed and deleted rows are listed after the new
        // ones, from the drawing and its journal (docs/adr/9151).
        let pending: Vec<super::pending_rows::PendingLine> = match (&data.error, &changes) {
            (Some(_), Some(c)) => super::pending_rows::pending_lines(c, &fields),
            _ => Vec::new(),
        };
        let offline = data.error.is_some();
        // The taken rows a save found deleted in the source come after the new ones, with their local values, so their
        // drafts can be looked at (docs/adr/9152); the source's pages hold them no longer.
        let gone_rows: Vec<(i64, std::sync::Arc<kentos_commands::Entity>)> = if offline {
            Vec::new()
        } else {
            self.gis
                .data_edits
                .gone
                .get(&node.id)
                .map(|keys| keys.iter().filter_map(|k| taken.get(k).map(|e| (*k, e.clone()))).collect())
                .unwrap_or_default()
        };
        let before = new_rows + gone_rows.len();
        let count = if offline {
            new_rows + pending.len()
        } else {
            before + data.total.unwrap_or(0) as usize
        };
        // A taken row's fields not changed here show the source's value now, not the one it was taken with
        // (docs/adr/9152): only what is changed here is the draft's.
        let changed_by_key: HashMap<i64, (std::sync::Arc<kentos_commands::Entity>, std::sync::Arc<kentos_commands::Entity>)> =
            changes
                .as_ref()
                .map(|c| c.changed.iter().map(|d| (d.key, (d.now.clone(), d.row.clone()))).collect())
                .unwrap_or_default();
        let editing = data.editing.clone();
        let edit_text = data.edit_text.clone();
        let edit_error = data.edit_error.clone();
        // A listed field's cell edits through the drawing's table's list editor (docs/adr/9323): its fields, the
        // layer's subtypes and contingent values narrow what is offered.
        let defs: HashMap<String, kentos_commands::LayerField> =
            node.fields.iter().map(|f| (f.name.clone(), f.clone())).collect();
        let all_fields = node.fields.clone();
        let subtypes = node.subtypes.clone();
        let contingents = node.contingents.clone();
        let field_count = fields.len() + 1;
        let shown_new = fields.clone();
        let table = Table::new(columns)
            .horizontal()
            .virtualized(count, move |i| {
                if let Some((uid, e)) = added.get(i) {
                    // Öznitelikleri gir beside “yeni”: the new record's form (docs/adr/9346), its words in the tip.
                    let enter = kentos_ui::widget::tip(
                        iced::widget::button(
                            kentos_ui::icon::icon(crate::icons::from_web(Some("formEdit")))
                                .size(14.0)
                                .tone(kentos_ui::icon::Tone::Accent),
                        )
                        .padding([0, 2])
                        .style(kentos_ui::style::button::flat)
                        .on_press(msg(DataEvent::EnterNew(*uid))),
                        kentos_ui::widget::Tip::from(words::ENTER_ATTRS),
                        iced::widget::tooltip::Position::Bottom,
                    );
                    let mut cells: Vec<Element<'_, Message>> = vec![
                        row![kentos_ui::widget::table::text(words::NEW).style(kentos_ui::style::text::warning), enter]
                            .spacing(6)
                            .align_y(Center)
                            .into(),
                    ];
                    cells.extend(shown_new.iter().map(|f| {
                        kentos_ui::widget::table::text(e.base().attrs.get(f).cloned().unwrap_or_default()).into()
                    }));
                    return TableLine::new(cells)
                        .selected(chosen.contains(uid))
                        .on_press(msg(DataEvent::PressNew(*uid)));
                }
                if !offline && i < before {
                    let (key, e) = &gone_rows[i - new_rows];
                    let mut cells: Vec<Element<'_, Message>> = vec![
                        kentos_ui::widget::table::text(format!("{key} {}", words::GONE))
                            .style(kentos_ui::style::text::warning)
                            .into(),
                    ];
                    cells.extend(shown.iter().map(|f| {
                        kentos_ui::widget::table::text(e.base().attrs.get(f).cloned().unwrap_or_default()).into()
                    }));
                    return TableLine::new(cells);
                }
                let i = if offline { i - new_rows } else { i - before };
                if offline {
                    let Some(p) = pending.get(i) else {
                        return TableLine::new(std::iter::once(text("").into()).chain(shown.iter().map(|_| text("").into())));
                    };
                    let mark = if p.deleted { words::DELETED } else { words::CHANGED };
                    let mut cells: Vec<Element<'_, Message>> = vec![
                        kentos_ui::widget::table::text(format!("{} {mark}", p.key))
                            .style(kentos_ui::style::text::warning)
                            .into(),
                    ];
                    cells.extend(p.cells.iter().map(|c| {
                        let face = kentos_ui::widget::table::text(c.clone());
                        if p.deleted { face.style(kentos_ui::style::text::muted).into() } else { face.into() }
                    }));
                    return TableLine::new(cells);
                }
                let page = i as u64 / PAGE;
                let at = (i as u64 % PAGE) as usize;
                match pages.get(&page).and_then(|p| p.get(at)) {
                    Some(r) => {
                        let key = r.key;
                        let gone = deleted.contains(&key);
                        let mut cells: Vec<Element<'_, Message>> = vec![kentos_ui::widget::table::number(r.key.to_string()).into()];
                        for (j, f) in shown.iter().enumerate() {
                            if editing.as_ref().is_some_and(|(k, e)| *k == key && e == f) {
                                let def = defs.get(f).filter(|d| {
                                    d.kind == kentos_commands::LayerFieldKind::Boolean
                                        || d.values.as_ref().is_some_and(|v| !v.is_empty())
                                });
                                let cell: Element<'_, Message> = match def {
                                    Some(d) => {
                                        let raw = super::pending_rows::cell_raw(
                                            changed_by_key.get(&key).map(|(now, row)| (&**now, &**row)),
                                            &r.attrs,
                                            f,
                                        )
                                        .unwrap_or_default();
                                        let offered = kentos_commands::offered_on(
                                            subtypes.as_ref(),
                                            &contingents,
                                            &all_fields,
                                            d,
                                            |n| r.attrs.get(n).cloned(),
                                        );
                                        let (choices, codes, at) = super::view::choices_of(d, &raw, offered);
                                        kentos_ui::widget::Cell::new(
                                            kentos_ui::widget::select::Select::new(choices, at, move |k| {
                                                msg(DataEvent::Choose(codes[k].clone()))
                                            })
                                            .cell(),
                                        )
                                        .into()
                                    }
                                    None => crate::points::cell::edit_box(
                                        &edit_text,
                                        false,
                                        kentos_ui::widget::table::cell_pads(j + 1, field_count),
                                        |t| msg(DataEvent::Input(t)),
                                        msg(DataEvent::Enter),
                                        msg(DataEvent::Cancel),
                                        msg(DataEvent::Cancel),
                                    ),
                                };
                                // A refused value: its reason beside the cell, the value kept to be put right.
                                let cell = match &edit_error {
                                    Some(why) => row![
                                        container(cell).width(Fill),
                                        kentos_ui::widget::tip(
                                            kentos_ui::icon::icon(kentos_ui::icon::Icon::Warning)
                                                .size(13.0)
                                                .tone(kentos_ui::icon::Tone::Warning),
                                            kentos_ui::widget::Tip::new("Değer yazılmadı").body(why.clone()),
                                            iced::widget::tooltip::Position::Top,
                                        ),
                                    ]
                                    .spacing(4)
                                    .align_y(Center)
                                    .into(),
                                    None => cell,
                                };
                                cells.push(cell);
                                continue;
                            }
                            // A value never written apart from an empty text, a code by its label (docs/adr/9188).
                            let raw = super::pending_rows::cell_raw(
                                changed_by_key.get(&key).map(|(now, row)| (&**now, &**row)),
                                &r.attrs,
                                f,
                            );
                            let face: Element<'_, Message> = match raw {
                                None => super::null_cell().into(),
                                Some(raw) => {
                                    let value = super::pending_rows::shown_value(rules.get(f), raw);
                                    let face = kentos_ui::widget::table::text(value);
                                    if gone {
                                        face.style(kentos_ui::style::text::muted).into()
                                    } else {
                                        face.into()
                                    }
                                }
                            };
                            // The whole cell takes the double click, an empty value too; its press selects the row as
                            // the row's own does (a cell that takes a press keeps it from the row).
                            let face = container(face).width(Fill).height(Fill).align_y(Center);
                            cells.push(
                                iced::widget::mouse_area(face)
                                    .on_press(msg(DataEvent::Press(key, false)))
                                    .on_double_click(msg(DataEvent::Edit(key, f.clone())))
                                    .into(),
                            );
                        }
                        TableLine::new(cells)
                            .selected(selected.contains(&key))
                            .on_press(msg(DataEvent::Press(key, false)))
                    }
                    None => TableLine::new(
                        std::iter::once(label::muted("…").into()).chain(shown.iter().map(|_| text("").into())),
                    ),
                }
            })
            .on_range(move |a, b| msg(DataEvent::Range(a.saturating_sub(before), b.saturating_sub(before))))
            // A refused question says why where the rows would be, not that it reads: a field the layer lacks is no
            // empty answer (docs/adr/9132).
            .empty(match (&data.error, data.total) {
                (Some(why), _) => why.clone(),
                (None, Some(0)) => words::NONE.to_owned(),
                (None, _) => words::READING.to_owned(),
            });
        // Being edited: said; what waits for the source has its own line with Kaynağa yaz (docs/adr/9346).
        let session = changes.as_ref().map(|_| " · düzenleniyor".to_owned());
        // The pending rows listed while the source is unread are said as such (docs/adr/9151).
        let waiting = changes
            .as_ref()
            .filter(|c| offline && !c.is_empty())
            .map(|c| {
                // When the source last answered for the layer (docs/adr/9163).
                let verified = self.gis.data_edits.verified.get(&node.id).copied();
                super::pending_rows::pending_words(c.added.len(), c.changed.len(), c.deleted.len(), verified)
            });
        let status = match (&data.error, data.total) {
            // Said where the rows would be when none is shown: the line only says none was read.
            (Some(_), _) if count == 0 => label::caption(words::UNREAD).style(kentos_ui::style::text::muted),
            (Some(why), _) => label::caption(match &waiting {
                Some(w) => format!("{why} · {w}"),
                None => why.clone(),
            })
            .style(kentos_ui::style::text::warning),
            (None, None) => label::caption(words::READING),
            (None, Some(n)) => label::caption(format!(
                "{}{} · {}{}{}{}",
                words::count(n),
                // The rows read so far beside the table's count, while they are fewer (docs/adr/9142).
                {
                    let loaded: usize = pages.values().map(Vec::len).sum();
                    if (loaded as u64) < n {
                        format!(" ({} yüklendi)", crate::view::thousands(loaded as f64))
                    } else {
                        String::new()
                    }
                },
                match data.checked {
                    Some(RowsChecked::Here) => words::HERE,
                    _ => words::SOURCE,
                },
                // Which rows these are (docs/adr/9348): the filtered layer's, or the whole source's.
                match (self.table_layer_filter(node), node.filter.as_ref().and_then(|f| f.expression.as_ref())) {
                    (Some(f), _) => format!(" · katman süzgeci: {f}"),
                    (None, Some(e)) if !e.trim().is_empty() => " · kaynağın tümü (katman süzgeci uygulanmadı)".to_owned(),
                    _ => String::new(),
                },
                if selected.is_empty() {
                    String::new()
                } else {
                    format!(" · {} seçili", selected.len())
                },
                session.unwrap_or_default()
            )),
        };
        // The fields the caller's role does not read, named (docs/adr/9150, 9188).
        let withheld = self.withheld_fields(&node.id);
        let mut foot = row![status].align_y(Center).padding([2, 8]).spacing(12);
        // The rows the source does not have yet, and the way to write them (the same Kaynağa yaz as everywhere).
        if let Some(c) = changes.as_ref().filter(|c| !c.is_empty() && !offline) {
            foot = foot.push(
                label::caption(unwritten_words(c.added.len(), c.changed.len(), c.deleted.len()))
                    .style(kentos_ui::style::text::warning),
            );
            foot = foot.push(
                iced::widget::button(label::caption(crate::data_edits::WRITE))
                    .padding(kentos_ui::theme::metrics::action_small())
                    .style(kentos_ui::style::button::secondary)
                    .on_press(Message::Gis(crate::gis::Event::DataPending(crate::data_pending::Event::Save(vec![
                        node.id.clone(),
                    ])))),
            );
        }
        // The layer's fields its table does not have (docs/adr/9318): the others are read, these shown empty.
        if !data.missing.is_empty() {
            let names = data.missing.iter().map(|n| format!("“{n}”")).collect::<Vec<_>>().join(", ");
            foot = foot.push(
                label::caption(format!(
                    "Kaynak tablosunda olmayan alan: {names}; boş gösterilir. Alanlar › Kaynak şemasını yenile ya da sütunu ekleyin."
                ))
                .style(kentos_ui::style::text::warning),
            );
        }
        if !withheld.is_empty() {
            foot = foot.push(
                label::caption(super::role_fields::withheld_words(&withheld)).style(kentos_ui::style::text::muted),
            );
        }
        column![container(table).width(Fill).height(Fill), foot].width(Fill).height(Fill).into()
    }
}

#[cfg(test)]
#[path = "closed_cell_tests.rs"]
mod closed_cell_tests;
#[cfg(test)]
#[path = "typed_cell_tests.rs"]
mod typed_cell_tests;

/// “2 kayıt kaynağa yazılmadı (1 satır eklendi, 1 satır değişti)”.
fn unwritten_words(added: usize, changed: usize, deleted: usize) -> String {
    format!(
        "{} kayıt kaynağa yazılmadı ({})",
        added + changed + deleted,
        crate::data_edits::counts_words(added, changed, deleted)
    )
}
