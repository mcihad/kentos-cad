//! Bilgi on data layers (docs/adr/9101 §6, plan/veri.md VY-03): a click of Bilgi on no object of the drawing asks the
//! shown data layers' tables, the top one first, for the row under the point. The first row found opens the card with
//! every column read from the source, through the layer's card template, and Bilgi rings it. The tables are asked off
//! the interface's thread: a database may take a moment to answer. A click asked again before the answer came makes
//! the earlier answer stale. The shown rasters under the point are asked with them (rasters/cell_info.rs,
//! docs/adr/9137): with no row their first cell is the card, beside a row or an object the card offers it.

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
    /// Bilgi's clicks on an object and their reach: the rasters and the shown data layers' rows under them asked after
    /// the update (docs/adr/9137, 9167).
    pub beside: Vec<(Vec2, f64)>,
    /// The last click's reach (world units), as the tool gave it with its hits.
    pub reach: f64,
    /// The shown data layers' rows under the last click on an object, the top layer's first (docs/adr/9167): counted
    /// with the objects under it, the card goes on to them after the objects.
    pub rows: Vec<Found>,
    /// The shown rasters' cells under the last click, the top one first.
    pub cells: Vec<crate::rasters::cell_info::Cell>,
    /// The card shows the first cell, not the object or the row.
    pub showing_raster: bool,
    /// The objects under the last click on an object, the first the click's (docs/adr/9145).
    pub hits: Vec<kentos_domain::Slot>,
}

/// What the tables said.
#[derive(Debug, Clone)]
pub enum Event {
    /// The answer to an ask: the row found, if any, the tables that could not be asked, the rasters' cells under the
    /// point and whether a shown raster was there at all.
    Answered(u64, Option<Box<Found>>, Vec<String>, Vec<crate::rasters::cell_info::Cell>, bool),
    /// What lies beside a click on an object (docs/adr/9137, 9167): the rasters' cells under it, the shown data layers'
    /// rows under it and the tables that could not be asked.
    Beside(u64, Vec<crate::rasters::cell_info::Cell>, Vec<Found>, Vec<String>),
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

/// The row under `at` of each of `nodes` (the shown data layers, the top one first), counted with the objects under a
/// click (docs/adr/9167); a table that could not be asked is said and the next one asked.
pub fn ask_all(nodes: &[LayerNode], settings: &ProjectSettings, at: Vec2, reach: f64) -> (Vec<Found>, Vec<String>) {
    let mut rows = Vec::new();
    let mut said = Vec::new();
    for node in nodes {
        match row_at(node, settings, at, reach) {
            Ok(Some((key, object))) => rows.push(Found { layer: node.id.clone(), key, object, at }),
            Ok(None) => {}
            Err(why) => said.push(format!("“{}” sorulamadı: {why}", node.name)),
        }
    }
    (rows, said)
}

impl App {
    /// The shown data layers with a geometry, the top one first: what Bilgi asks of a click.
    fn info_data_layers(&self) -> Vec<LayerNode> {
        let Some(doc) = &self.document else {
            return Vec::new();
        };
        let mut nodes: Vec<LayerNode> = kentos_plot::layers::shown_layers(doc.model.layers().nodes())
            .into_iter()
            .filter(|n| n.data.as_ref().is_some_and(|d| d.geometry.is_some()))
            .cloned()
            .collect();
        nodes.reverse();
        nodes
    }

    /// Beside Bilgi's click on an object: the rasters under it and the shown data layers' rows under it, read off the
    /// thread (docs/adr/9137, 9167).
    fn info_beside(&mut self, at: Vec2, reach: f64) -> Task<Message> {
        let (rasters, _) = self.rasters_under(at);
        self.gis.data_info.cells.clear();
        self.gis.data_info.showing_raster = false;
        self.gis.data_info.rows.clear();
        let nodes = self.info_data_layers();
        if rasters.is_empty() && nodes.is_empty() {
            return Task::none();
        }
        let Some(doc) = &self.document else {
            return Task::none();
        };
        self.gis.data_info.asked += 1;
        let asked = self.gis.data_info.asked;
        let settings = doc.model.settings().clone();
        crate::db_connections::off_thread(
            move || {
                let (rows, said) = ask_all(&nodes, &settings, at, reach);
                (crate::rasters::cell_info::read(rasters, at), rows, said)
            },
            move |(cells, rows, said)| message(Event::Beside(asked, cells, rows, said)),
        )
    }

    /// Bilgi's last click on no object of the drawing: the shown data layers asked off the thread, the top one first.
    pub(crate) fn data_info_tasks(&mut self) -> Task<Message> {
        // A click on an object: the rasters and the data layers' rows under it, for the card to offer and count
        // (docs/adr/9137, 9167).
        let beside = self.gis.data_info.beside.drain(..).next_back();
        if let Some((at, reach)) = beside {
            return self.info_beside(at, reach);
        }
        let Some((at, reach)) = self.gis.data_info.wanted.drain(..).next_back() else {
            return Task::none();
        };
        let (rasters, any_raster) = self.rasters_under(at);
        let nodes = self.info_data_layers();
        if nodes.is_empty() && !any_raster {
            return Task::none();
        }
        let Some(doc) = &self.document else {
            return Task::none();
        };
        self.gis.data_info.asked += 1;
        let asked = self.gis.data_info.asked;
        let settings = doc.model.settings().clone();
        let task = crate::db_connections::off_thread(
            move || {
                let (found, said) = ask(&nodes, &settings, at, reach);
                (found, said, crate::rasters::cell_info::read(rasters, at))
            },
            move |(found, said, cells)| message(Event::Answered(asked, found.map(Box::new), said, cells, any_raster)),
        );
        self.tool_reply(task)
    }

    pub(crate) fn data_info_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Answered(asked, found, said, cells, any_raster) => {
                if asked != self.gis.data_info.asked {
                    return Task::none();
                }
                for why in said {
                    self.warn(why);
                }
                let object = found.as_ref().map(|f| f.object.clone());
                // No row: the first raster's cell is the card; a click outside every shown raster is said.
                self.gis.data_info.showing_raster = found.is_none() && !cells.is_empty();
                if found.is_none() && cells.is_empty() && any_raster {
                    self.say(kentos_interaction::Level::Info, "Tıklanan nokta gösterilen rasterlerin kapsamı dışında.");
                }
                self.gis.data_info.cells = cells;
                self.gis.data_info.card = found.map(|f| *f);
                self.with_tool(|s, cx| s.data_found(object.as_ref(), cx));
            }
            Event::Beside(asked, cells, rows, said) => {
                if asked == self.gis.data_info.asked {
                    for why in said {
                        self.warn(why);
                    }
                    self.gis.data_info.cells = cells;
                    self.gis.data_info.rows = rows;
                }
            }
        }
        Task::none()
    }

    /// The shown row's card: its layer's card template filled with every column; without a template, the layer's name
    /// and the row's name (docs/adr/9160), and the columns in the table's order. Under the title, where the row is: the table and its key. None when the
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
            // The layer's name and the row's name (docs/adr/9160): its display field's value.
            card.title = match self.display_value(&found.object, &self.withheld_fields(&node.id)) {
                Some((_, name)) => format!("{} · {name}", node.name),
                None => node.name.clone(),
            };
        }
        if template.is_none_or(|c| c.fields.is_empty()) {
            // A field never written says NULL, a code its label; the role's unread fields are left out and named
            // (docs/adr/9150, 9188).
            let attrs = &found.object.base().attrs;
            let withheld = self.withheld_fields(&node.id);
            card.rows = node
                .fields
                .iter()
                .filter(|f| !withheld.contains(&f.name))
                .map(|f| {
                    let v = match attrs.get(&f.name) {
                        Some(raw) => crate::features::pending_rows::shown_value(Some(f), raw.clone()),
                        None => crate::features::texts::NULL.to_owned(),
                    };
                    (f.name.clone(), v)
                })
                .collect();
            if !withheld.is_empty() {
                card.rows.push(("Rolünüz okumuyor".to_owned(), withheld.join(", ")));
            }
        }
        // The layer too: a card template's title may not name it (docs/adr/9145).
        card.source = Some((
            format!("{} katmanı · {} · {} {}", node.name, data.place_words(), data.key, found.key),
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
