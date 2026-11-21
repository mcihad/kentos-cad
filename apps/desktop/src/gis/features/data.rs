//! Öznitelik tablosu on a data layer (docs/adr/9102 §5, plan/veri.md VY-04): the rows are read from the layer's
//! source a page at a time (`cad.layers.rows`' reader, `layers_rows::read`, off the interface's thread), only the
//! pages the table shows. Ara, Göster and İfade süzgeci are the panel's own; a header press orders by its field (a text
//! in the Turkish alphabet's order), then the other way, then by key. Without a filter the count, the order and the
//! pages are the source's; with one, its first page brings every kept key and the later pages are read by key.
//! Göster's Görünümdekiler reads the view's box, read again when the view has moved; Seçililer reads the rows pressed.
//! A row press selects its row (Ctrl turns one over). The rows are read only here: they are edited in VY-05's session.
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
    pub const HERE: &str = "süzgeç burada soruldu";
    pub const NONE: &str = "Kaynakta soruya uyan satır yok.";
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
}

/// The data layer's table.
#[derive(Debug, Default)]
pub(crate) struct DataTable {
    asked: Option<Asked>,
    generation: u64,
    fields: Vec<String>,
    total: Option<u64>,
    keys: Option<Vec<i64>>,
    checked: Option<RowsChecked>,
    pages: HashMap<u64, Vec<LayerRow>>,
    reading: HashSet<u64>,
    range: (usize, usize),
    error: Option<String>,
    /// The order the headers give: the field and whether it goes down.
    order: Vec<(String, bool)>,
    /// The rows pressed, by key, and the layer they are of.
    pub(crate) selected: BTreeSet<i64>,
    selected_of: Option<String>,
    /// Ara and İfade süzgeci as Enter asked them.
    search: String,
    filter: String,
    /// The stop switch of the question's reads.
    stop: Arc<AtomicBool>,
    /// The reads started, as a test answers them.
    #[cfg(test)]
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
    /// Enter in Ara or İfade süzgeci: what is typed is asked.
    Apply,
    /// Seçime yakınlaş: the selected rows framed.
    Zoom,
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
    #[cfg(test)]
    pub(crate) fn pending(&mut self) -> Vec<(u64, u64, LayersRows)> {
        std::mem::take(&mut self.inputs)
    }
}

impl DataTable {
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
            filter: data.filter.clone(),
            order: data.order.clone(),
            view,
            selected,
        }
    }

    /// After every message while the table shows a data layer: a new question starts again from its first page, and
    /// the pages the table shows are read.
    pub(crate) fn data_table_tasks(&mut self) -> Task<Message> {
        let Some(node) = self.data_table_layer() else {
            return Task::none();
        };
        let asked = self.data_asked(&node);
        let data = &mut self.gis.features.data;
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
            data.total = None;
            data.keys = None;
            data.checked = None;
            data.error = None;
        }
        let (first, last) = data.range;
        let mut wanted: Vec<u64> = (first as u64 / PAGE..=(last.max(first + 1) as u64 - 1) / PAGE).collect();
        if data.total.is_none() && !wanted.contains(&0) {
            wanted.insert(0, 0);
        }
        let Some(doc) = self.document.as_ref() else {
            return Task::none();
        };
        let settings = doc.model.settings().clone();
        let data = &mut self.gis.features.data;
        let mut tasks = Vec::new();
        for page in wanted {
            if data.pages.contains_key(&page) || data.reading.contains(&page) {
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
            #[cfg(test)]
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
                match answer {
                    Ok(a) => {
                        data.error = None;
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
                        }
                        data.pages.insert(page, a.rows);
                        // The pages farthest from the view let go.
                        if data.pages.len() > KEPT {
                            let at = data.range.0 as u64 / PAGE;
                            let mut far: Vec<u64> = data.pages.keys().copied().collect();
                            far.sort_by_key(|p| std::cmp::Reverse(p.abs_diff(at)));
                            for p in far.into_iter().take(data.pages.len() - KEPT) {
                                data.pages.remove(&p);
                            }
                        }
                    }
                    Err(why) => data.error = Some(why),
                }
            }
            DataEvent::Zoom => {}
            DataEvent::Apply => {
                data.search = self.gis.features.search.trim().to_owned();
                data.filter = self.gis.features.filter.trim().to_owned();
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
        }
        Task::none()
    }

    /// The table's rows: a page's cells when it is read, else a row waiting for it.
    pub(crate) fn data_table_view(&self, node: &LayerNode) -> Element<'_, Message> {
        let data = &self.gis.features.data;
        let fields: Vec<String> = if data.fields.is_empty() {
            node.fields.iter().map(|f| f.name.clone()).collect()
        } else {
            data.fields.clone()
        };
        let mut columns = vec![TableColumn::new("Anahtar").width(Length::Fixed(80.0)).align_right()];
        for f in &fields {
            let order = data.order.first().filter(|(o, _)| o == f).map(|(_, d)| {
                if *d {
                    SortOrder::Descending
                } else {
                    SortOrder::Ascending
                }
            });
            columns.push(
                TableColumn::new(f.clone())
                    .width(Length::Fixed(140.0))
                    .sortable(order, msg(DataEvent::Sort(f.clone()))),
            );
        }
        let count = data.total.unwrap_or(0) as usize;
        let pages = &data.pages;
        let selected = &data.selected;
        let shown = fields.clone();
        let table = Table::new(columns)
            .horizontal()
            .virtualized(count, move |i| {
                let page = i as u64 / PAGE;
                let at = (i as u64 % PAGE) as usize;
                match pages.get(&page).and_then(|p| p.get(at)) {
                    Some(r) => {
                        let mut cells: Vec<Element<'_, Message>> = vec![kentos_ui::widget::table::number(r.key.to_string()).into()];
                        for f in &shown {
                            cells.push(
                                kentos_ui::widget::table::text(r.attrs.get(f).cloned().unwrap_or_default()).into(),
                            );
                        }
                        let key = r.key;
                        TableLine::new(cells)
                            .selected(selected.contains(&key))
                            .on_press(msg(DataEvent::Press(key, false)))
                    }
                    None => TableLine::new(
                        std::iter::once(label::muted("…").into()).chain(shown.iter().map(|_| text("").into())),
                    ),
                }
            })
            .on_range(|a, b| msg(DataEvent::Range(a, b)))
            .empty(if data.total == Some(0) { words::NONE } else { words::READING });
        let status = match (&data.error, data.total) {
            (Some(why), _) => label::caption(why.clone()).style(kentos_ui::style::text::warning),
            (None, None) => label::caption(words::READING),
            (None, Some(n)) => label::caption(format!(
                "{} · {}{}",
                words::count(n),
                match data.checked {
                    Some(RowsChecked::Here) => words::HERE,
                    _ => words::SOURCE,
                },
                if selected.is_empty() {
                    String::new()
                } else {
                    format!(" · {} seçili", selected.len())
                }
            )),
        };
        column![
            container(table).width(Fill).height(Fill),
            row![status].align_y(Center).padding([2, 8]),
        ]
        .width(Fill)
        .height(Fill)
        .into()
    }
}
