//! Seç on data layers (docs/adr/9103 §5, plan/veri.md VY-05): a click of Seç on no object of the drawing, or its box,
//! asks the shown data layers for their rows under it, off the interface's thread. A layer being edited gives its rows
//! to the drawing (`Document::take_data_rows`: each an object of the layer, its key kept) and their objects join the
//! selection, so the drawing's tools move, change and delete them. A layer that is not being edited, the active one,
//! selects its rows in its table (Öznitelik tablosu's selection, outlined on the map while the table shows it).
//!
//! A click takes the row the drawing's own pick would (the nearest edge, else the smallest area holding the point); a
//! box every row wholly inside it, or every row meeting it when drawn right to left. A box keeping more rows than a
//! session takes at once is refused with the count.

use iced::Task;
use kentos_commands::{Entity, LAYER_SESSION_KEYS_MOST, LayerNode, ProjectSettings};
use kentos_data::{Cancel, Request};
use kentos_interaction::DataRegion;
use kentos_native_application::data_write;
use kentos_native_application::sources::TableSystem;

use crate::app::{App, Message};

/// Seç's asks waiting for the update, and the last one made.
#[derive(Debug, Default)]
pub(crate) struct DataPick {
    pub(crate) wanted: Vec<DataRegion>,
    pub(crate) asked: u64,
}

/// What a layer gave for an ask.
#[derive(Debug, Clone)]
pub enum Got {
    /// A layer being edited: its rows, to be taken.
    Rows(String, Vec<(i64, Entity)>),
    /// A layer not being edited: the keys to select in its table.
    Keys(String, Vec<i64>),
}

/// What the tables said.
#[derive(Debug, Clone)]
pub enum Event {
    Answered(u64, Vec<Got>, Vec<String>),
}

fn message(e: Event) -> Message {
    Message::Gis(crate::gis::Event::DataPick(e))
}

/// The keys of `node`'s rows in `region` (the project's units); a box past `most` rows is refused with the count.
pub(crate) fn keys_in(
    node: &LayerNode,
    settings: &ProjectSettings,
    region: DataRegion,
    most: usize,
) -> Result<Vec<i64>, String> {
    if let DataRegion::Point { at, reach } = region {
        return Ok(crate::data_info::row_at(node, settings, at, reach)?
            .map(|(key, _)| vec![key])
            .unwrap_or_default());
    }
    let DataRegion::Box { a, b, crossing } = region else {
        return Ok(Vec::new());
    };
    let (Some(data), true) = (node.data.as_ref(), node.data.as_ref().is_some_and(|d| d.geometry.is_some())) else {
        return Ok(Vec::new());
    };
    let srid = data.geometry.as_ref().and_then(|g| g.srid);
    let source = kentos_data::registry()
        .source(&data.address)
        .map_err(|e| format!("“{}” tablosu açılamadı: {e}", data.table))?;
    let system = TableSystem::of(settings, srid)?;
    let around = [a.x.min(b.x), a.y.min(b.y), a.x.max(b.x), a.y.max(b.y)];
    let bbox = match &system {
        Some(s) => s.to_table(around).ok_or("çerçeve tablonun sistemine çevrilemedi")?,
        None => around,
    };
    let request = Request {
        bbox: Some(bbox),
        exact: true,
        columns: Some(Vec::new()),
        geometry: !crossing,
        limit: Some(most as u64 + 1),
        chunk: 4096,
        ..Request::default()
    };
    let mut keys = Vec::new();
    source
        .features(&data.table, &request, &Cancel::new(), &mut |part| {
            for f in part {
                // A window keeps a row wholly inside it: its geometry's box in the project's system.
                let inside = crossing
                    || f
                        .shape()
                        .and_then(|g| match &system {
                            Some(s) => s.moved(g),
                            None => Some(g.clone()),
                        })
                        .and_then(|g| g.bounds())
                        .is_some_and(|r| r[0] >= around[0] && r[1] >= around[1] && r[2] <= around[2] && r[3] <= around[3]);
                if inside {
                    keys.push(f.fid);
                }
            }
            true
        })
        .map_err(|e| format!("“{}” tablosu okunamadı: {e}", data.table))?;
    if keys.len() > most {
        return Err(format!(
            "Çerçevede “{}” katmanının {most} satırından fazlası var; bir seferde en çok {most} satır seçilir. Daha küçük bir alan seçin.",
            node.name
        ));
    }
    Ok(keys)
}

impl App {
    /// After every message: Seç's asks made of the shown data layers, off the interface's thread.
    pub(crate) fn data_pick_tasks(&mut self) -> Task<Message> {
        if self.gis.data_pick.wanted.is_empty() {
            return Task::none();
        }
        let wanted = std::mem::take(&mut self.gis.data_pick.wanted);
        let Some(doc) = self.document.as_ref() else {
            return Task::none();
        };
        let model = &doc.model;
        let shown: Vec<&LayerNode> = crate::style::scene::shown_layers(model.layers().nodes())
            .into_iter()
            .filter(|n| n.data.is_some())
            .collect();
        // The layers being edited give their rows; else the active layer, a data layer, selects in its table.
        let edited: Vec<LayerNode> = shown.iter().filter(|n| model.editing(&n.id)).map(|n| (*n).clone()).collect();
        let active = model.layers().active().to_owned();
        let table: Option<LayerNode> = shown
            .iter()
            .find(|n| n.id == active && !model.editing(&n.id))
            .map(|n| (*n).clone());
        if edited.is_empty() && table.is_none() {
            return Task::none();
        }
        let settings = model.settings().clone();
        self.gis.data_pick.asked += 1;
        let asked = self.gis.data_pick.asked;
        crate::db_connections::off_thread(
            move || {
                let mut got = Vec::new();
                let mut failed = Vec::new();
                for region in wanted {
                    for node in &edited {
                        let rows = keys_in(node, &settings, region, LAYER_SESSION_KEYS_MOST).and_then(|keys| {
                            if keys.is_empty() {
                                Ok(Vec::new())
                            } else {
                                data_write::rows_to_take(node, &settings, &keys, &|| false)
                            }
                        });
                        match rows {
                            Ok(rows) if !rows.is_empty() => got.push(Got::Rows(node.id.clone(), rows)),
                            Ok(_) => {}
                            Err(why) => failed.push(why),
                        }
                    }
                    if let Some(node) = &table {
                        match keys_in(node, &settings, region, LAYER_SESSION_KEYS_MOST) {
                            Ok(keys) if !keys.is_empty() => got.push(Got::Keys(node.id.clone(), keys)),
                            Ok(_) => {}
                            Err(why) => failed.push(why),
                        }
                    }
                }
                (got, failed)
            },
            move |(got, failed)| message(Event::Answered(asked, got, failed)),
        )
    }

    pub(crate) fn data_pick_event(&mut self, event: Event) -> Task<Message> {
        let Event::Answered(asked, got, failed) = event;
        for why in failed {
            self.warn(why);
        }
        if asked != self.gis.data_pick.asked {
            return Task::none();
        }
        for g in got {
            match g {
                Got::Rows(layer, rows) => {
                    let Some(doc) = self.document.as_mut() else {
                        continue;
                    };
                    match doc.model.take_data_rows(&layer, rows) {
                        Ok(uids) => {
                            let slots: Vec<kentos_domain::Slot> =
                                uids.into_iter().flatten().filter_map(|u| doc.model.slot_of(u)).collect();
                            self.selection.add(slots);
                        }
                        Err(refusal) => self.warn(refusal.0),
                    }
                }
                Got::Keys(layer, keys) => self.gis.features.data.select_rows(&layer, keys),
            }
        }
        Task::none()
    }
}
