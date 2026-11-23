//! A data layer's fields counted by their sources (docs/adr/9181). İstatistik over every row (Göster: Tümü) and a
//! chart that reads one field ask for a data layer's field by name. The file or the database counts it with its own
//! `GROUP BY`, off the interface's thread (`field_counts`): one row a value comes back, and no geometry is read. The
//! count is kept for the drawing as it is: its session, its writes to the sources and the rows it holds. The source
//! leaves the held rows out and they are counted here as the drawing has them. While a field is counted the tab says
//! so; a source that cannot count it says why. A field with too many values, or a drawing holding too many rows, is
//! counted from the rows read instead (data_rows.rs).
//!
//! [`Grouped`] is a counted field as the core counts it: a row a value, each row's place repeated as many times as the
//! table has rows holding its value. Every statistic and chart of the core then comes out as if each row had been
//! read. A press on counted values asks the source for the keys of their rows and selects them in the layer's table
//! ([`App::pick_values`]).

use std::cell::{Cell as Counter, RefCell};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::Arc;

use iced::Task;
use kentos_commands::{Entity, LayerField};
use kentos_domain::Slot;
use kentos_geometry_core::ops::charts::data::Column;
use kentos_interaction::chart_data::{cell_of, layer_columns};
use kentos_native_application::field_counts::{FieldGroup, HELD_MOST, field_counts, value_keys};

use crate::app::{App, Message};
use crate::document::Document;

/// The most rows a press on counted values selects.
const SELECT_MOST: usize = 10_000;

/// What the counts' stamp is: the drawing's session and its writes to the sources.
type Stamp = (u64, u64);

/// A field's count as kept.
#[derive(Clone, Debug)]
pub(crate) enum Counts {
    /// Being counted.
    Counting,
    /// Counted: its values with their rows, and the count's number (a new count, a new number).
    Ready(Arc<Vec<FieldGroup>>, u64),
    /// Not counted by the source (too many values, too many rows held): the rows are read and counted.
    Rows,
    /// The source could not count it: why.
    Failed(String),
}

struct Kept {
    stamp: Stamp,
    /// The rows the drawing held when it was asked (the source left them out).
    held: Arc<BTreeSet<i64>>,
    counts: Counts,
}

/// The kept counts, by layer and field.
#[derive(Default)]
pub(crate) struct DataCounts {
    kept: RefCell<HashMap<(String, String), Kept>>,
    /// The fields a tab asked for whose counts are not kept for the drawing as it is.
    wanted: RefCell<HashSet<(String, String)>>,
    /// The last count's number.
    counted: Counter<u64>,
}

/// How a press changes the table's selection: replaced, added to (Shift), taken from (Ctrl, ⌘).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pick {
    Replace,
    Add,
    Remove,
}

/// What counting a field and pressing a counted value say.
#[derive(Debug, Clone)]
pub enum Event {
    Counted {
        layer: String,
        field: String,
        stamp: Stamp,
        held: Arc<BTreeSet<i64>>,
        counted: Result<Option<Vec<FieldGroup>>, String>,
    },
    /// The keys of the rows holding the pressed values (at most [`SELECT_MOST`], and whether there were more), how they
    /// change the table's selection, and what was pressed.
    Keys {
        layer: String,
        label: Option<String>,
        pick: Pick,
        keys: Result<(Vec<i64>, bool), String>,
    },
}

fn message(e: Event) -> Message {
    Message::Gis(crate::gis::Event::DataCounts(e))
}

/// A counted field as the core counts it (see the module).
#[derive(Clone, Debug)]
pub(crate) struct Grouped {
    /// A cell a value: the source's values, then the drawing's rows one each.
    pub column: Column,
    /// Each cell's place once for every row of the table that holds its value.
    pub rows: Vec<u32>,
    /// What each cell stands for, for a press.
    pub picker: Picker,
}

/// What the cells of a counted field stand for: the source's values (the count itself, shared, not copied), then the
/// drawing's rows by their slots.
#[derive(Clone, Debug)]
pub(crate) struct Picker {
    groups: Arc<Vec<FieldGroup>>,
    drawn: Vec<Slot>,
}

impl Grouped {
    /// The field `f` (as the column `shape`) from the source's `groups`, then the drawing's rows of the layer.
    pub(crate) fn of(shape: &Column, f: &LayerField, groups: &Arc<Vec<FieldGroup>>, drawn: &[&Entity]) -> Grouped {
        let total = groups.iter().map(|g| g.rows as usize).sum::<usize>() + drawn.len();
        let mut cells = Vec::with_capacity(groups.len() + drawn.len());
        let mut rows = Vec::with_capacity(total);
        for (i, g) in groups.iter().enumerate() {
            cells.push(cell_of(f, g.text.as_deref()));
            rows.extend(std::iter::repeat_n(i as u32, g.rows as usize));
        }
        for e in drawn {
            rows.push(cells.len() as u32);
            cells.push(cell_of(f, e.base().attrs.get(&f.name).map(String::as_str)));
        }
        Grouped {
            column: Column { cells, ..shape.clone() },
            rows,
            picker: Picker { groups: groups.clone(), drawn: drawn.iter().map(|e| Slot(e.base().id)).collect() },
        }
    }
}

impl Picker {
    /// What the cells at `rows` stand for: the source's values among them and the drawing's rows' slots, each once.
    pub(crate) fn picked(&self, rows: &[u32]) -> (Vec<kentos_data::Value>, Vec<Slot>) {
        let mut seen = BTreeSet::new();
        let (mut values, mut slots) = (Vec::new(), Vec::new());
        for &r in rows {
            if !seen.insert(r) {
                continue;
            }
            let i = r as usize;
            match self.groups.get(i) {
                Some(g) => values.push(g.value.clone()),
                None => slots.extend(self.drawn.get(i - self.groups.len()).copied()),
            }
        }
        (values, slots)
    }

    /// The cells of the drawing's rows among `chosen`.
    pub(crate) fn rows_of_slots(&self, chosen: &[Slot]) -> Vec<u32> {
        let chosen: HashSet<Slot> = chosen.iter().copied().collect();
        let first = self.groups.len();
        self.drawn
            .iter()
            .enumerate()
            .filter(|(_, s)| chosen.contains(s))
            .map(|(j, _)| (first + j) as u32)
            .collect()
    }
}

/// A counted field as a tab finds it now.
pub(crate) enum FieldNow {
    Counting,
    /// Counted, with the count's number.
    Ready(Grouped, u64),
    Rows,
    Failed(String),
}

/// A data layer's fields as columns without rows, each with its field: the keys, names and kinds the tabs offer
/// before any row is read (a form's `redact`ed field left out, docs/adr/0546 §1).
pub(crate) fn shape_columns(app: &App, doc: &Document, layer: &str) -> Vec<(Column, LayerField)> {
    let Some(node) = doc.model.layers().get(layer) else {
        return Vec::new();
    };
    let per_metre = doc.model.settings().unit().per_metre();
    let redacted = app.redacted_fields();
    layer_columns(&node.fields, &[], None, per_metre)
        .into_iter()
        .filter(|c| !redacted.contains(&c.key) && !redacted.contains(&c.label))
        .filter_map(|c| node.fields.iter().find(|f| f.name == c.key).cloned().map(|f| (c, f)))
        .collect()
}

impl App {
    fn counts_stamp(&self) -> Option<Stamp> {
        let doc = self.document.as_ref()?;
        Some((doc.session, doc.model.data_written()))
    }

    /// A data layer's field as its source counted it, for a tab: kept, or being counted (asked after this message);
    /// none for a layer that is not a data layer.
    pub(crate) fn data_counts_of(&self, layer: &str, field: &str) -> Option<Counts> {
        let doc = self.document.as_ref()?;
        doc.model.layers().get(layer)?.data.as_ref()?;
        let held = doc.model.data_hidden(layer);
        if held.len() > HELD_MOST {
            return Some(Counts::Rows);
        }
        let stamp = self.counts_stamp()?;
        let key = (layer.to_owned(), field.to_owned());
        if let Some(k) = self.gis.data_counts.kept.borrow().get(&key)
            && k.stamp == stamp
            && *k.held == held
        {
            return Some(k.counts.clone());
        }
        self.gis.data_counts.wanted.borrow_mut().insert(key);
        Some(Counts::Counting)
    }

    /// A data layer's field as a tab finds it now: counted with the drawing's rows of the layer, being counted, left
    /// to the rows read, or why the source could not count it.
    pub(crate) fn field_now(&self, layer: &str, shape: &Column, f: &LayerField) -> FieldNow {
        match self.data_counts_of(layer, &f.name) {
            Some(Counts::Ready(groups, n)) => {
                let Some(doc) = self.document.as_ref() else {
                    return FieldNow::Counting;
                };
                let drawn: Vec<&Entity> = doc.model.by_layer(layer).collect();
                FieldNow::Ready(Grouped::of(shape, f, &groups, &drawn), n)
            }
            Some(Counts::Counting) => FieldNow::Counting,
            Some(Counts::Failed(why)) => FieldNow::Failed(why),
            Some(Counts::Rows) | None => FieldNow::Rows,
        }
    }

    /// After every message: the fields a tab asked for counted by their sources, off the interface's thread.
    pub(crate) fn data_counts_tasks(&mut self) -> Task<Message> {
        let wanted: Vec<(String, String)> = self.gis.data_counts.wanted.borrow_mut().drain().collect();
        let (Some(doc), Some(stamp)) = (self.document.as_ref(), self.counts_stamp()) else {
            return Task::none();
        };
        let mut held_of: HashMap<String, Arc<BTreeSet<i64>>> = HashMap::new();
        let mut tasks = Vec::new();
        for (layer, field) in wanted {
            let held = held_of.entry(layer.clone()).or_insert_with(|| Arc::new(doc.model.data_hidden(&layer))).clone();
            let key = (layer.clone(), field.clone());
            let asked = self
                .gis
                .data_counts
                .kept
                .borrow()
                .get(&key)
                .is_some_and(|k| k.stamp == stamp && k.held == held && matches!(k.counts, Counts::Counting));
            if asked {
                continue;
            }
            let Some(node) = doc.model.layers().get(&layer).cloned() else {
                continue;
            };
            self.gis.data_counts.kept.borrow_mut().insert(key, Kept { stamp, held: held.clone(), counts: Counts::Counting });
            let (name, leave) = (field.clone(), held.clone());
            tasks.push(crate::db_connections::off_thread(
                move || field_counts(&node, &name, &leave),
                move |counted| message(Event::Counted { layer, field, stamp, held, counted }),
            ));
        }
        Task::batch(tasks)
    }

    pub(crate) fn data_counts_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Counted { layer, field, stamp, held, counted } => {
                let fresh = self.counts_stamp() == Some(stamp)
                    && self.document.as_ref().is_some_and(|d| d.model.data_hidden(&layer) == *held);
                if !fresh {
                    return Task::none();
                }
                let counts = match counted {
                    Ok(Some(groups)) => {
                        let n = self.gis.data_counts.counted.get() + 1;
                        self.gis.data_counts.counted.set(n);
                        Counts::Ready(Arc::new(groups), n)
                    }
                    Ok(None) => Counts::Rows,
                    Err(why) => Counts::Failed(why),
                };
                self.gis.data_counts.kept.borrow_mut().insert((layer, field), Kept { stamp, held, counts });
            }
            Event::Keys { layer, label, pick, keys } => match keys {
                Ok((keys, more)) => {
                    let n = keys.len();
                    let data = &mut self.gis.features.data;
                    match pick {
                        Pick::Replace => data.choose_rows(&layer, keys.into_iter().collect()),
                        Pick::Add => data.select_rows(&layer, keys),
                        Pick::Remove => data.unselect_rows(&layer, &keys),
                    }
                    let n = crate::view::thousands(n as f64);
                    let head = label.map(|l| format!("“{l}”: ")).unwrap_or_default();
                    let words = match (pick, more) {
                        (Pick::Remove, _) => format!("{head}{n} satır seçimden çıkarıldı."),
                        (_, true) => format!("{head}ilk {n} satır seçildi (daha fazlası var)."),
                        (_, false) => format!("{head}{n} satır seçildi."),
                    };
                    self.output(words);
                }
                Err(why) => self.warn(format!("Değerin satırları okunamadı: {why}")),
            },
        }
        Task::none()
    }

    /// How a press changes a selection now: Shift adds, Ctrl (⌘) takes away, else it replaces.
    pub(crate) fn pick_now(&self) -> Pick {
        if self.modifiers.control() || self.modifiers.logo() {
            Pick::Remove
        } else if self.modifiers.shift() {
            Pick::Add
        } else {
            Pick::Replace
        }
    }

    /// A press on counted values (a value row of İstatistik, an element of a counted chart), as [`Picker::picked`]
    /// found them: the drawing's rows among them picked as its selection, the source's asked for their keys off the
    /// thread and picked in the layer's table (Shift adds, Ctrl takes away). `label` names what was pressed.
    pub(crate) fn pick_values(
        &mut self,
        layer: &str,
        field: &str,
        label: Option<String>,
        (values, slots): (Vec<kentos_data::Value>, Vec<Slot>),
    ) -> Task<Message> {
        let pick = self.pick_now();
        self.apply_pick(slots);
        let Some(doc) = self.document.as_ref() else {
            return Task::none();
        };
        let Some(node) = doc.model.layers().get(layer).cloned() else {
            return Task::none();
        };
        if values.is_empty() {
            if pick == Pick::Replace && self.gis.features.data.any_selected_of(layer) {
                self.gis.features.data.clear_selection();
            }
            return Task::none();
        }
        let held = doc.model.data_hidden(layer);
        let (layer, field) = (layer.to_owned(), field.to_owned());
        crate::db_connections::off_thread(
            move || value_keys(&node, &field, &values, &held, SELECT_MOST, &|| false),
            move |keys| message(Event::Keys { layer, label, pick, keys }),
        )
    }
}

#[cfg(test)]
#[path = "data_counts_tests.rs"]
mod data_counts_tests;
