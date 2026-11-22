//! A data layer's rows for the tabs that count them (docs/adr/9130): İstatistik and Grafik read a layer's objects, and a
//! data layer's rows are not the drawing's objects but its source's (docs/adr/9101). A tab asks for them by layer; they
//! are read from the source whole, off the interface's thread, and kept for the drawing as it is (its session and the
//! count of its writes to the sources): a write of the layer's changes reads them again. While they are read the tab
//! says so; a source that cannot be read says why. The rows the drawing holds (taken or new in the edit session) are
//! taken as the drawing has them now, not as the source last wrote them ([`App::data_layer_rows`]).

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use iced::Task;
use kentos_commands::Entity;

use crate::app::{App, Message};

/// What the rows' stamp is: the drawing's session and its writes to the sources.
type Stamp = (u64, u64);

/// A data layer's rows as kept.
#[derive(Clone, Debug)]
pub(crate) enum Rows {
    /// Being read.
    Reading,
    /// Read: every row of the source, its key and its object.
    Ready(Arc<Vec<(i64, Entity)>>),
    /// The source could not be read: why.
    Failed(String),
}

/// The kept rows, by layer.
#[derive(Default)]
pub(crate) struct DataRows {
    kept: RefCell<HashMap<String, (Stamp, Rows)>>,
    /// The layers a tab asked for whose rows are not kept for the drawing as it is.
    wanted: RefCell<HashSet<String>>,
}

/// What reading a layer's rows says.
#[derive(Debug, Clone)]
pub enum Event {
    Read(String, Stamp, Result<Vec<(i64, Entity)>, String>),
}

fn message(e: Event) -> Message {
    Message::Gis(crate::gis::Event::DataRows(e))
}

impl App {
    fn data_rows_stamp(&self) -> Option<Stamp> {
        let doc = self.document.as_ref()?;
        Some((doc.session, doc.model.data_written()))
    }

    /// A data layer's rows for a tab: kept ones, or none yet (asked for: read after this message); none for a layer
    /// that is not a data layer.
    pub(crate) fn data_rows_of(&self, layer: &str) -> Option<Rows> {
        let doc = self.document.as_ref()?;
        doc.model.layers().get(layer)?.data.as_ref()?;
        let stamp = self.data_rows_stamp()?;
        if let Some((at, rows)) = self.gis.data_rows.kept.borrow().get(layer)
            && *at == stamp
        {
            return Some(rows.clone());
        }
        self.gis.data_rows.wanted.borrow_mut().insert(layer.to_owned());
        Some(Rows::Reading)
    }

    /// A data layer's rows as the drawing has them now: the source's rows it does not hold, then its own (taken and new
    /// in the edit session); none while they are read, why when they could not be; none at all for another layer.
    pub(crate) fn data_layer_rows(&self, layer: &str) -> Option<Result<Option<Vec<Entity>>, String>> {
        let rows = self.data_rows_of(layer)?;
        let doc = self.document.as_ref()?;
        Some(match rows {
            Rows::Reading => Ok(None),
            Rows::Failed(why) => Err(why),
            Rows::Ready(read) => {
                let held = doc.model.data_hidden(layer);
                let mut out: Vec<Entity> =
                    read.iter().filter(|(k, _)| !held.contains(k)).map(|(_, e)| e.clone()).collect();
                out.extend(doc.model.by_layer(layer).cloned());
                Ok(Some(out))
            }
        })
    }

    /// After every message: the rows a tab asked for read from their sources, off the interface's thread.
    pub(crate) fn data_rows_tasks(&mut self) -> Task<Message> {
        let wanted: Vec<String> = self.gis.data_rows.wanted.borrow_mut().drain().collect();
        let (Some(doc), Some(stamp)) = (self.document.as_ref(), self.data_rows_stamp()) else {
            return Task::none();
        };
        let settings = doc.model.settings().clone();
        let mut tasks = Vec::new();
        for layer in wanted {
            let reading = self.gis.data_rows.kept.borrow().get(&layer).is_some_and(|(at, r)| *at == stamp && matches!(r, Rows::Reading));
            if reading {
                continue;
            }
            let Some(node) = doc.model.layers().get(&layer).cloned() else {
                continue;
            };
            self.gis.data_rows.kept.borrow_mut().insert(layer.clone(), (stamp, Rows::Reading));
            let settings = settings.clone();
            tasks.push(crate::db_connections::off_thread(
                move || {
                    kentos_native_application::data_write::all_rows(&node, &settings, &Default::default(), &|| false)
                        .map(|read| read.rows)
                },
                move |read| message(Event::Read(layer, stamp, read)),
            ));
        }
        Task::batch(tasks)
    }

    pub(crate) fn data_rows_event(&mut self, event: Event) -> Task<Message> {
        let Event::Read(layer, stamp, read) = event;
        if self.data_rows_stamp() != Some(stamp) {
            return Task::none();
        }
        let rows = match read {
            Ok(rows) => Rows::Ready(Arc::new(rows)),
            Err(why) => Rows::Failed(why),
        };
        self.gis.data_rows.kept.borrow_mut().insert(layer, (stamp, rows));
        // The tabs that count the layer read it again: its data, the summaries and the chart worked out from it.
        *self.gis.charts.layer_data.borrow_mut() = None;
        *self.gis.charts.stats.borrow_mut() = Default::default();
        *self.gis.charts.chart.borrow_mut() = Default::default();
        Task::none()
    }
}

#[cfg(test)]
#[path = "data_rows_tests.rs"]
mod data_rows_tests;
