//! A data layer's rows counted by its source for the inspector (docs/adr/9340). With nothing selected the inspector
//! said the drawing's “Nesne sayısı 0” for a project whose layers all read GeoPackage tables: a data layer's rows are
//! its source's, not the drawing's objects (docs/adr/9101). The drawing's own objects and the sources' rows are said
//! apart now. A source counts its table itself (`count`, its catalog never estimated), off the interface's thread,
//! and the count is kept for the drawing as it is (its session and its writes to the sources). While it is counted
//! the inspector says so; a source that cannot count says why. An unknown count is never shown as 0.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use iced::Task;

use crate::app::{App, Message};

/// What the counts' stamp is: the drawing's session, its writes to the sources and the layer's source (a layer
/// pointed at another file is counted again).
type Stamp = (u64, u64, String);

/// A layer's count as kept.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Counted {
    Counting,
    Ready(u64),
    Failed(String),
}

/// The kept counts, by layer.
#[derive(Default)]
pub(crate) struct SourceCounts {
    kept: RefCell<HashMap<String, (Stamp, Counted)>>,
    wanted: RefCell<HashSet<String>>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Counted(String, Stamp, Result<u64, String>),
}

fn message(e: Event) -> Message {
    Message::Gis(crate::gis::Event::SourceCounts(e))
}

impl App {
    fn source_counts_stamp(&self, layer: &str) -> Option<Stamp> {
        let doc = self.document.as_ref()?;
        let data = doc.model.layers().get(layer)?.data.as_ref()?;
        Some((
            doc.session,
            doc.model.data_written(),
            format!("{}\u{0}{}", data.address, data.table),
        ))
    }

    /// A data layer's rows as its source counts them: kept, or being counted (asked: counted after this message).
    pub(crate) fn source_count_of(&self, layer: &str) -> Counted {
        let Some(stamp) = self.source_counts_stamp(layer) else {
            return Counted::Counting;
        };
        if let Some((at, c)) = self.gis.source_counts.kept.borrow().get(layer)
            && *at == stamp
        {
            return c.clone();
        }
        self.gis
            .source_counts
            .wanted
            .borrow_mut()
            .insert(layer.to_owned());
        Counted::Counting
    }

    /// After every message: the counts asked for, each by its source, off the interface's thread.
    pub(crate) fn source_counts_tasks(&mut self) -> Task<Message> {
        let wanted: Vec<String> = self.gis.source_counts.wanted.borrow_mut().drain().collect();
        let Some(doc) = self.document.as_ref() else {
            return Task::none();
        };
        let mut tasks = Vec::new();
        for layer in wanted {
            let Some(stamp) = self.source_counts_stamp(&layer) else {
                continue;
            };
            let counting = self
                .gis
                .source_counts
                .kept
                .borrow()
                .get(&layer)
                .is_some_and(|(at, c)| *at == stamp && *c == Counted::Counting);
            let Some(data) = doc
                .model
                .layers()
                .get(&layer)
                .and_then(|n| n.data.clone())
                .filter(|_| !counting)
            else {
                continue;
            };
            self.gis
                .source_counts
                .kept
                .borrow_mut()
                .insert(layer.clone(), (stamp.clone(), Counted::Counting));
            tasks.push(crate::db_connections::off_thread(
                move || {
                    let source = kentos_data::registry()
                        .source(&data.address)
                        .map_err(|e| e.to_string())?;
                    source
                        .count(&data.table, &kentos_data::Request::all(), false)
                        .map_err(|e| e.to_string())
                },
                move |n| message(Event::Counted(layer, stamp, n)),
            ));
        }
        Task::batch(tasks)
    }

    pub(crate) fn source_counts_event(&mut self, event: Event) -> Task<Message> {
        let Event::Counted(layer, stamp, n) = event;
        if self.source_counts_stamp(&layer).as_ref() == Some(&stamp) {
            let c = match n {
                Ok(n) => Counted::Ready(n),
                Err(why) => Counted::Failed(why),
            };
            self.gis
                .source_counts
                .kept
                .borrow_mut()
                .insert(layer, (stamp, c));
        }
        Task::none()
    }

    /// The sources' rows as the inspector says them with nothing selected: in all, how many layers, and each layer's
    /// (its name and count, the first eight); none without a data layer. A count not known yet is “sayılıyor…”, never 0.
    pub(crate) fn source_counts_said(&self) -> Option<(String, String, Vec<(String, String)>)> {
        let doc = self.document.as_ref()?;
        let layers: Vec<(String, String)> = doc
            .model
            .layers()
            .leaves()
            .into_iter()
            .filter(|n| n.data.is_some())
            .map(|n| (n.id.clone(), n.name.clone()))
            .collect();
        if layers.is_empty() {
            return None;
        }
        let counted: Vec<(String, Counted)> = layers
            .iter()
            .map(|(id, name)| (name.clone(), self.source_count_of(id)))
            .collect();
        let ready: u64 = counted
            .iter()
            .filter_map(|(_, c)| {
                if let Counted::Ready(n) = c {
                    Some(*n)
                } else {
                    None
                }
            })
            .sum();
        let counting = counted.iter().any(|(_, c)| *c == Counted::Counting);
        let failed = counted
            .iter()
            .filter(|(_, c)| matches!(c, Counted::Failed(_)))
            .count();
        let total = if counting {
            "sayılıyor…".to_owned()
        } else if failed > 0 {
            format!(
                "{} (+ {failed} katman sayılamadı)",
                crate::view::thousands(ready as f64)
            )
        } else {
            crate::view::thousands(ready as f64)
        };
        const SHOWN: usize = 8;
        let mut each: Vec<(String, String)> = counted
            .iter()
            .take(SHOWN)
            .map(|(name, c)| {
                let said = match c {
                    Counted::Ready(n) => format!("{} satır", crate::view::thousands(*n as f64)),
                    Counted::Counting => "sayılıyor…".to_owned(),
                    Counted::Failed(why) => format!("sayılamadı ({why})"),
                };
                (name.clone(), said)
            })
            .collect();
        if counted.len() > SHOWN {
            each.push((
                "…".to_owned(),
                format!("{} katman daha", counted.len() - SHOWN),
            ));
        }
        Some((total, format!("{} katman", layers.len()), each))
    }
}

#[cfg(test)]
#[path = "source_counts_tests.rs"]
mod source_counts_tests;
