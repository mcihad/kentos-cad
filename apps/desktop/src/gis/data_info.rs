//! Bilgi on data layers (docs/adr/9101 §6, plan/veri.md VY-03): a click of Bilgi on no object of the drawing asks the
//! shown data layers' tables, the top one first, for the row under the point. The first row found opens the card with
//! every column read from the source, through the layer's card template, and Bilgi rings it. The tables are asked off
//! the interface's thread: a database may take a moment to answer. A click asked again before the answer came makes
//! the earlier answer stale.

use std::collections::HashMap;

use iced::Task;
use kentos_commands::{Entity, LayerNode, ProjectSettings};
use kentos_data::{Cancel, Request};
use kentos_geometry_core::store::Store;
use kentos_interaction::Vec2;
use kentos_native_application::sources::TableSystem;

use crate::app::{App, Message};

/// Rows a click reads from one table at most: a point among many close ones is still found.
const ROWS: u64 = 64;

/// A row found under a click: its layer, its key, the row as an object of no drawing with every column, and where the
/// click was.
#[derive(Clone, Debug)]
pub struct Found {
    pub layer: String,
    pub key: i64,
    pub object: Entity,
    pub at: Vec2,
}

/// Bilgi's asks of the data layers and the row shown.
#[derive(Debug, Default)]
pub struct State {
    /// Clicks waiting to be asked after the update: the point and the reach (world units).
    pub wanted: Vec<(Vec2, f64)>,
    /// The last ask: an answer to an earlier one is stale.
    pub asked: u64,
    /// The row whose card is shown.
    pub card: Option<Found>,
}

/// What the tables said.
#[derive(Debug, Clone)]
pub enum Event {
    /// The answer to an ask: the row found, if any, and the tables that could not be asked.
    Answered(u64, Option<Box<Found>>, Vec<String>),
}

fn message(event: Event) -> Message {
    Message::Gis(crate::gis::Event::DataInfo(event))
}

/// The row of `node`'s table under `at` within `reach` (the project's units): the nearest edge, else the smallest area
/// holding the point, as a click on the drawing's objects picks. None for a table without geometry.
pub fn row_at(
    node: &LayerNode,
    settings: &ProjectSettings,
    at: Vec2,
    reach: f64,
) -> Result<Option<(i64, Entity)>, String> {
    let Some(data) = node.data.as_ref() else {
        return Ok(None);
    };
    let Some(geometry) = data.geometry.as_ref() else {
        return Ok(None);
    };
    let source = kentos_data::registry()
        .source(&data.address)
        .map_err(|e| e.to_string())?;
    let schema = source.schema(&data.table).map_err(|e| e.to_string())?;
    let system = TableSystem::of(settings, geometry.srid)?;
    let around = [at.x - reach, at.y - reach, at.x + reach, at.y + reach];
    let bbox = match &system {
        Some(s) => s
            .to_table(around)
            .ok_or("tıklanan yer tablonun sistemine çevrilemedi")?,
        None => around,
    };
    let names: Vec<String> = schema.fields.iter().map(|f| f.name.clone()).collect();
    let request = Request {
        bbox: Some(bbox),
        exact: true,
        columns: Some(names.clone()),
        geometry: true,
        limit: Some(ROWS),
        chunk: ROWS as usize,
        ..Request::default()
    };
    let mut objects: HashMap<u32, (i64, Entity)> = HashMap::new();
    let mut next = 1u32;
    source
        .features(&data.table, &request, &Cancel::new(), &mut |part| {
            for f in part {
                let Some(g) = f.shape() else {
                    continue;
                };
                let g = match &system {
                    Some(s) => match s.moved(g) {
                        Some(m) => m,
                        None => continue,
                    },
                    None => g.clone(),
                };
                let attrs: serde_json::Map<String, serde_json::Value> = names
                    .iter()
                    .zip(&f.values)
                    .filter_map(|(c, v)| crate::data_view::attribute(v).map(|a| (c.clone(), a)))
                    .collect();
                if let Some(e) =
                    kentos_native_application::geopackage::object_of(&g, &node.id, next, attrs)
                {
                    objects.insert(next, (f.fid, e));
                    next += 1;
                }
            }
            true
        })
        .map_err(|e| e.to_string())?;
    if objects.is_empty() {
        return Ok(None);
    }
    let mut store = Store::new();
    store.put_many(objects.values().map(|(_, e)| {
        (
            f64::from(e.base().id),
            node.id.as_str(),
            false,
            kentos_processing::geometry::record(e).3,
        )
    }));
    let hit = store.hit(kentos_geometry_core::Vec2::new(at.x, at.y), reach);
    Ok(hit.and_then(|id| objects.remove(&(id as u32))))
}

/// The first row found under `at` among `nodes` (the shown data layers, the top one first); a table that could not be
/// asked is said and the next one asked.
pub fn ask(
    nodes: &[LayerNode],
    settings: &ProjectSettings,
    at: Vec2,
    reach: f64,
) -> (Option<Found>, Vec<String>) {
    let mut said = Vec::new();
    for node in nodes {
        match row_at(node, settings, at, reach) {
            Ok(Some((key, object))) => {
                let found = Found {
                    layer: node.id.clone(),
                    key,
                    object,
                    at,
                };
                return (Some(found), said);
            }
            Ok(None) => {}
            Err(why) => said.push(format!("“{}” sorulamadı: {why}", node.name)),
        }
    }
    (None, said)
}

impl App {
    /// Bilgi's last click on no object of the drawing: the shown data layers asked off the thread, the top one first.
    pub(crate) fn data_info_tasks(&mut self) -> Task<Message> {
        let Some((at, reach)) = self.gis.data_info.wanted.drain(..).next_back() else {
            return Task::none();
        };
        let Some(doc) = &self.document else {
            return Task::none();
        };
        let mut nodes: Vec<LayerNode> = kentos_plot::layers::shown_layers(doc.model.layers().nodes())
            .into_iter()
            .filter(|n| n.data.as_ref().is_some_and(|d| d.geometry.is_some()))
            .cloned()
            .collect();
        if nodes.is_empty() {
            return Task::none();
        }
        nodes.reverse();
        self.gis.data_info.asked += 1;
        let asked = self.gis.data_info.asked;
        let settings = doc.model.settings().clone();
        let task = crate::db_connections::off_thread(
            move || ask(&nodes, &settings, at, reach),
            move |(found, said)| message(Event::Answered(asked, found.map(Box::new), said)),
        );
        self.tool_reply(task)
    }

    pub(crate) fn data_info_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Answered(asked, found, said) => {
                if asked != self.gis.data_info.asked {
                    return Task::none();
                }
                for why in said {
                    self.warn(why);
                }
                let object = found.as_ref().map(|f| f.object.clone());
                self.gis.data_info.card = found.map(|f| *f);
                self.with_tool(|s, cx| s.data_found(object.as_ref(), cx));
            }
        }
        Task::none()
    }

    /// The shown row's card: its layer's card template filled with every column; without a template, the layer's name
    /// and the columns in the table's order. Under the title, where the row is: the table and its key. None when the
    /// layer is gone.
    pub(crate) fn data_card(
        &self,
    ) -> Option<(
        crate::object_info::ObjectCard,
        Vec<crate::object_info::ObjectAction>,
        Vec2,
    )> {
        let found = self.gis.data_info.card.as_ref()?;
        let doc = self.document.as_ref()?;
        let node = doc.model.layers().get(&found.layer)?;
        let data = node.data.as_ref()?;
        let template = node.card.as_ref();
        let mut card = self.object_card(&found.object, template);
        if template.and_then(|c| c.title.as_ref()).is_none() {
            card.title = node.name.clone();
        }
        if template.is_none_or(|c| c.fields.is_empty()) {
            let attrs = &found.object.base().attrs;
            card.rows = node
                .fields
                .iter()
                .map(|f| (f.name.clone(), attrs.get(&f.name).cloned().unwrap_or_default()))
                .collect();
        }
        card.source = Some((
            format!("{} · {} {}", data.place_words(), data.key, found.key),
            false,
        ));
        let actions = if card.actions {
            self.object_actions(&found.object, Some(&node.actions))
        } else {
            Vec::new()
        };
        Some((card, actions, found.at))
    }
}
