//! Veri katmanlarının kapsamı ve önizlemesi (docs/adr/9327): a data layer's box and a small picture of its rows as its
//! source has them now, for Tümünü göster, Katmana yakınlaştır and Genel bakış.
//!
//! - **Why.** A layer keeps the box its table had when it was added. Rows written since (an edit session's save, a
//!   point of Noktalar) fall outside it, and a layer made empty kept none: Tümünü göster found nothing to show, and
//!   Genel bakış said the drawing was empty while the map drew the rows.
//! - **How.** Each box is read off the interface's thread (`data_extent`: the source's own, measured when it keeps none)
//!   once for the drawing as it is: its session and its writes to the sources, so a save reads it again. While a box is
//!   read, the one the layer kept stands in for it. Tümünü göster fits at once with what it has and fits again when the
//!   last box comes in, if the view is still where it left it.
//! - **The overview's outline.** The rows' geometry only, simplified to a pixel of the card, at most [`OUTLINE_ROWS`]
//!   rows (`data_outline`), read once the box is in and only while Genel bakış shows; the core draws them under the
//!   drawing's objects (`overview_picture_of`).

use std::cell::{Cell as Counter, RefCell};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use iced::Task;
use kentos_commands::{DataLayer, LayerNode};
use kentos_data::Cancel;
use kentos_geometry_core::store::Store;
use kentos_interaction::{Level, Viewpoint};
use kentos_native_application::data_extent::{TableBox, data_extent, data_outline};

use crate::app::{App, Message};

/// The rows an overview draws of a layer at most: past it, the first ones and the layer's box.
pub(crate) const OUTLINE_ROWS: u64 = 100_000;
/// The overview's width in pixels the outline is simplified for.
const OUTLINE_PIXELS: f64 = 256.0;

/// The drawing as the boxes were read for: its session and its writes to the sources.
type Stamp = (u64, u64);
/// The table a box was read of: a layer linked to another reads again.
type Table = (String, String);

/// A data layer's box as kept.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Extent {
    Reading,
    /// Read: in the table's system and the project's; none for a table without rows.
    Read(Option<TableBox>),
    /// The source could not say it: why.
    Failed(String),
}

/// A layer's outline for the overview.
#[derive(Clone)]
pub(crate) enum Outlined {
    Reading,
    /// Its rows' shapes (with more rows than were read, the table's box after them as an area).
    Read(Arc<Store>),
    /// Not readable: the picture has the layer's box without its rows.
    Failed,
}

/// An outline as an event carries it.
#[derive(Clone)]
pub struct Shapes(Arc<Store>);

impl std::fmt::Debug for Shapes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Shapes({} shapes)", self.0.len())
    }
}

struct Kept {
    stamp: Stamp,
    table: Table,
    extent: Extent,
    outline: Option<Outlined>,
}

/// The kept boxes and outlines, by layer.
#[derive(Default)]
pub(crate) struct DataExtents {
    kept: RefCell<HashMap<String, Kept>>,
    /// The layers whose boxes were asked for and are not kept for the drawing as it is.
    wanted: RefCell<HashSet<String>>,
    /// The layers whose outlines Genel bakış asked for.
    outlines_wanted: RefCell<HashSet<String>>,
    /// Bumped as each read comes in: the overview draws again.
    arrived: Counter<u64>,
    /// Tümünü göster asked while boxes were read: the view it left, fitted again once they are in if it is still there,
    /// and whether it said it waits for them.
    fit_after: RefCell<Option<(Viewpoint, bool)>>,
    /// Katmana yakınlaştır asked while the layer's box was read (the second round's T16): the layer and the view it
    /// left; framed once the box is in, if the view is still there.
    zoom_after: RefCell<Option<(String, Viewpoint)>>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Extent {
        layer: String,
        stamp: Stamp,
        table: Table,
        read: Result<Option<TableBox>, String>,
    },
    Outline {
        layer: String,
        stamp: Stamp,
        table: Table,
        read: Result<Shapes, String>,
    },
}

fn message(e: Event) -> Message {
    Message::Gis(crate::gis::Event::DataExtents(e))
}

/// The shown data layers' boxes for a view to show.
#[derive(Default)]
pub(crate) struct ShownBoxes {
    /// In the project's system: as their sources say them, or the kept ones while they are read.
    pub boxes: Vec<[f64; 4]>,
    /// The layers whose boxes are being read, by name.
    pub reading: Vec<String>,
    /// The layers whose boxes could not be read: name and why.
    pub failed: Vec<(String, String)>,
}

/// What Genel bakış draws of the data layers.
#[derive(Default)]
pub(crate) struct OverviewData {
    pub boxes: Vec<[f64; 4]>,
    /// Their rows' outlines, in the layer tree's order from the bottom.
    pub outlines: Vec<Arc<Store>>,
    /// Shown data layers whose box or outline is being read.
    pub reading: usize,
    /// A shown layer whose box could not be read: its name and why.
    pub failed: Option<(String, String)>,
    /// Data layers that are hidden.
    pub hidden: usize,
}

fn table_of(data: &DataLayer) -> Table {
    (data.address.clone(), data.table.clone())
}

impl App {
    fn extents_stamp(&self) -> Option<Stamp> {
        let doc = self.document.as_ref()?;
        Some((doc.session, doc.model.data_written()))
    }

    /// A data layer's box as its source says it for the drawing as it is: kept, or being read (asked after this
    /// message); none for a layer that is not a data layer.
    pub(crate) fn data_extent_of(&self, layer: &str) -> Option<Extent> {
        let doc = self.document.as_ref()?;
        let data = doc.model.layers().get(layer)?.data.as_ref()?;
        let stamp = self.extents_stamp()?;
        if let Some(k) = self.gis.data_extents.kept.borrow().get(layer)
            && k.stamp == stamp
            && k.table == table_of(data)
        {
            return Some(k.extent.clone());
        }
        self.gis
            .data_extents
            .wanted
            .borrow_mut()
            .insert(layer.to_owned());
        Some(Extent::Reading)
    }

    /// A data layer's outline for the overview: kept, or being read once its box is in (asked after this message).
    fn data_outline_of(&self, layer: &str) -> Option<Outlined> {
        let Some(Extent::Read(Some(_))) = self.data_extent_of(layer) else {
            return None;
        };
        if let Some(outline) = self
            .gis
            .data_extents
            .kept
            .borrow()
            .get(layer)
            .and_then(|k| k.outline.clone())
        {
            return Some(outline);
        }
        self.gis
            .data_extents
            .outlines_wanted
            .borrow_mut()
            .insert(layer.to_owned());
        Some(Outlined::Reading)
    }

    /// The box `node` frames when its source's is read: the source's, or the kept one while it is read; none for a
    /// table without rows, a layer that is not a data layer or a box not readable.
    fn box_of(&self, node: &LayerNode) -> (Option<[f64; 4]>, Option<Extent>) {
        let extent = self.data_extent_of(&node.id);
        let b = match &extent {
            Some(Extent::Read(b)) => b.map(|b| b.project),
            Some(Extent::Reading) => self
                .document
                .as_ref()
                .and_then(|d| crate::data_layers::extent_in_project(node, d.settings())),
            _ => None,
        };
        (b, extent)
    }

    /// The shown data layers' boxes (Tümünü göster's), with the layers still read and those not readable.
    pub(crate) fn shown_data_boxes(&self) -> ShownBoxes {
        let Some(doc) = &self.document else {
            return ShownBoxes::default();
        };
        let mut out = ShownBoxes::default();
        for node in kentos_plot::layers::shown_layers(doc.model.layers().nodes()) {
            if node.data.is_none() {
                continue;
            }
            let (b, extent) = self.box_of(node);
            out.boxes.extend(b);
            match extent {
                Some(Extent::Reading) => out.reading.push(node.name.clone()),
                Some(Extent::Failed(why)) => out.failed.push((node.name.clone(), why)),
                _ => {}
            }
        }
        out
    }

    /// The data layers' boxes at or below `node` (Katmana yakınlaştır's), as their sources say them now.
    pub(crate) fn data_boxes_below(&self, node: &LayerNode) -> Vec<[f64; 4]> {
        let mut out = Vec::new();
        let mut stack = vec![node];
        while let Some(n) = stack.pop() {
            if n.data.is_some()
                && let (Some(b), _) = self.box_of(n)
            {
                out.push(b);
            }
            stack.extend(n.children.iter());
        }
        out
    }

    /// What Genel bakış draws of the data layers: the shown ones' boxes and outlines, and what is still read.
    pub(crate) fn overview_data(&self) -> OverviewData {
        let Some(doc) = &self.document else {
            return OverviewData::default();
        };
        let tree = doc.model.layers();
        let shown = kentos_plot::layers::shown_layers(tree.nodes());
        let mut out = OverviewData {
            hidden: tree.leaves().iter().filter(|l| l.data.is_some()).count()
                - shown.iter().filter(|n| n.data.is_some()).count(),
            ..OverviewData::default()
        };
        // The tree lists the top layer first; the bottom one is drawn first.
        for node in shown.into_iter().rev().filter(|n| n.data.is_some()) {
            let (b, extent) = self.box_of(node);
            out.boxes.extend(b);
            match extent {
                Some(Extent::Reading) => out.reading += 1,
                Some(Extent::Failed(why)) => {
                    out.failed.get_or_insert((node.name.clone(), why));
                }
                _ => {}
            }
            match self.data_outline_of(&node.id) {
                Some(Outlined::Read(store)) => out.outlines.push(store),
                Some(Outlined::Reading) => out.reading += 1,
                _ => {}
            }
        }
        out
    }

    /// The reads that came in so far: the overview draws again when it changes.
    pub(crate) fn data_extents_arrived(&self) -> u64 {
        self.gis.data_extents.arrived.get()
    }

    /// Tümünü göster asked while some boxes were read: fitted again when they are in, if the view stays; `said`, it
    /// said it waits for them.
    pub(crate) fn fit_when_boxes_read(&self, said: bool) {
        *self.gis.data_extents.fit_after.borrow_mut() = Some((self.viewpoint(), said));
    }

    /// After every message: the boxes and outlines asked for, read off the interface's thread.
    pub(crate) fn data_extents_tasks(&mut self) -> Task<Message> {
        let boxes: Vec<String> = self.gis.data_extents.wanted.borrow_mut().drain().collect();
        let outlines: Vec<String> = self
            .gis
            .data_extents
            .outlines_wanted
            .borrow_mut()
            .drain()
            .collect();
        let (Some(doc), Some(stamp)) = (self.document.as_ref(), self.extents_stamp()) else {
            return Task::none();
        };
        let settings = doc.settings().clone();
        let mut tasks = Vec::new();
        for layer in boxes {
            let Some(data) = doc.model.layers().get(&layer).and_then(|n| n.data.clone()) else {
                continue;
            };
            let table = table_of(&data);
            let mut kept = self.gis.data_extents.kept.borrow_mut();
            let asked = kept.get(&layer).is_some_and(|k| {
                k.stamp == stamp && k.table == table && k.extent == Extent::Reading
            });
            if asked {
                continue;
            }
            kept.insert(
                layer.clone(),
                Kept {
                    stamp,
                    table: table.clone(),
                    extent: Extent::Reading,
                    outline: None,
                },
            );
            let settings = settings.clone();
            tasks.push(crate::db_connections::off_thread(
                move || data_extent(&settings, &data),
                move |read| {
                    message(Event::Extent {
                        layer,
                        stamp,
                        table,
                        read,
                    })
                },
            ));
        }
        for layer in outlines {
            let Some(node) = doc.model.layers().get(&layer) else {
                continue;
            };
            let Some(data) = node.data.clone() else {
                continue;
            };
            let table = table_of(&data);
            let mut kept = self.gis.data_extents.kept.borrow_mut();
            let Some(k) = kept
                .get_mut(&layer)
                .filter(|k| k.stamp == stamp && k.table == table)
            else {
                continue;
            };
            let (Extent::Read(Some(b)), None) = (&k.extent, &k.outline) else {
                continue;
            };
            let within = *b;
            k.outline = Some(Outlined::Reading);
            let settings = settings.clone();
            let id = layer.clone();
            tasks.push(crate::db_connections::off_thread(
                move || {
                    data_outline(
                        &settings,
                        &data,
                        &id,
                        &within,
                        OUTLINE_PIXELS,
                        OUTLINE_ROWS,
                        &Cancel::new(),
                    )
                    .map(|o| Shapes(Arc::new(o.store)))
                },
                move |read| {
                    message(Event::Outline {
                        layer,
                        stamp,
                        table,
                        read,
                    })
                },
            ));
        }
        Task::batch(tasks)
    }

    pub(crate) fn data_extents_event(&mut self, event: Event) -> Task<Message> {
        let fresh = |app: &App, layer: &str, stamp: Stamp, table: &Table| {
            app.extents_stamp() == Some(stamp)
                && app
                    .document
                    .as_ref()
                    .and_then(|d| d.model.layers().get(layer)?.data.as_ref().map(table_of))
                    .is_some_and(|t| &t == table)
        };
        match event {
            Event::Extent {
                layer,
                stamp,
                table,
                read,
            } => {
                if !fresh(self, &layer, stamp, &table) {
                    return Task::none();
                }
                let extent = match read {
                    Ok(b) => Extent::Read(b),
                    Err(why) => Extent::Failed(why),
                };
                self.gis.data_extents.kept.borrow_mut().insert(
                    layer,
                    Kept {
                        stamp,
                        table,
                        extent,
                        outline: None,
                    },
                );
            }
            Event::Outline {
                layer,
                stamp,
                table,
                read,
            } => {
                if !fresh(self, &layer, stamp, &table) {
                    return Task::none();
                }
                if let Some(k) = self.gis.data_extents.kept.borrow_mut().get_mut(&layer) {
                    k.outline = Some(match read {
                        Ok(Shapes(store)) => Outlined::Read(store),
                        Err(_) => Outlined::Failed,
                    });
                }
            }
        }
        let arrived = &self.gis.data_extents.arrived;
        arrived.set(arrived.get() + 1);
        self.fit_once_boxes_read();
        self.zoom_once_box_read();
        Task::none()
    }

    /// Katmana yakınlaştır's wait for a box being read (T16): the layer framed when no box below it is read any more,
    /// if the view is still where the click left it.
    fn zoom_once_box_read(&mut self) {
        let Some((layer, left)) = self.gis.data_extents.zoom_after.borrow().clone() else {
            return;
        };
        let Some(node) = self
            .document
            .as_ref()
            .and_then(|d| d.model.layers().get(&layer).cloned())
        else {
            *self.gis.data_extents.zoom_after.borrow_mut() = None;
            return;
        };
        if self
            .data_layers_below(&node)
            .iter()
            .any(|n| matches!(self.data_extent_of(&n.id), Some(Extent::Reading)))
        {
            return;
        }
        *self.gis.data_extents.zoom_after.borrow_mut() = None;
        if self.viewpoint() == left {
            self.zoom_to_layer(&layer);
        }
    }

    /// The data layers at or below `node`.
    pub(crate) fn data_layers_below(&self, node: &LayerNode) -> Vec<LayerNode> {
        let mut out = Vec::new();
        let mut stack = vec![node];
        while let Some(n) = stack.pop() {
            if n.data.is_some() {
                out.push(n.clone());
            }
            stack.extend(n.children.iter());
        }
        out
    }

    /// Katmana yakınlaştır on data layers with no box to frame (T16): what each says, read now or not; the box being
    /// read is waited for and framed when it comes. Whether it waits.
    pub(crate) fn zoom_without_box(&mut self, node: &LayerNode) -> bool {
        let layers = self.data_layers_below(node);
        let mut reading = Vec::new();
        for n in &layers {
            if n.data.as_ref().is_some_and(|d| d.geometry.is_none()) {
                self.say(
                    Level::Info,
                    format!("“{}” tablosunun geometri sütunu yok; haritada yeri olmadığı için yakınlaştırılmaz.", n.name),
                );
                continue;
            }
            match self.data_extent_of(&n.id) {
                Some(Extent::Reading) => reading.push(n.name.clone()),
                Some(Extent::Read(None)) => self.say(
                    Level::Info,
                    format!(
                        "“{}” tablosunda geometrili satır yok; yakınlaştırılacak kapsam yok.",
                        n.name
                    ),
                ),
                Some(Extent::Failed(why)) => self.say(
                    Level::Warn,
                    format!("“{}” katmanının kapsamı okunamadı: {why}", n.name),
                ),
                _ => {}
            }
        }
        if reading.is_empty() {
            return false;
        }
        self.say(
            Level::Info,
            format!(
                "“{}” kapsamı kaynaktan okunuyor; gelince yakınlaştırılır.",
                reading.join("”, “")
            ),
        );
        *self.gis.data_extents.zoom_after.borrow_mut() = Some((node.id.clone(), self.viewpoint()));
        true
    }

    /// Tümünü göster's second fit: once no shown layer's box is read any more, the view is fitted again on what the
    /// sources said, if it is still where the first fit left it (a pan or a zoom since keeps the user's view).
    fn fit_once_boxes_read(&mut self) {
        let Some((left, said)) = self.gis.data_extents.fit_after.borrow().clone() else {
            return;
        };
        let shown = self.shown_data_boxes();
        if !shown.reading.is_empty() {
            return;
        }
        *self.gis.data_extents.fit_after.borrow_mut() = None;
        if self.viewpoint() != left {
            return;
        }
        // What it said it waits for has come: the view on it, said so.
        if self.extents_shown(&shown, true) && said {
            self.say(Level::Info, FITTED);
        }
    }

    /// Tümünü göster's view on the drawing's objects and `shown`, the data layers' boxes, and what it says: the boxes not
    /// read, the objects far from the rest, and, once nothing is read any more (`last`), that there was nothing to show.
    /// Whether there was a box to show.
    pub(crate) fn extents_shown(&mut self, shown: &ShownBoxes, last: bool) -> bool {
        let (shown_any, far) = self.show_extents(&shown.boxes);
        for (name, why) in &shown.failed {
            self.say(
                Level::Warn,
                format!("“{name}” katmanının kapsamı okunamadı: {why}"),
            );
        }
        if last && !shown_any && shown.failed.is_empty() {
            self.say(Level::Info, NOTHING_SHOWN);
        }
        if far > 0 {
            self.say(
                Level::Warn,
                format!("{far} nesne çizimin geri kalanından çok uzakta; görünüme alınmadı. Kapsam denetimi onları seçer."),
            );
        }
        shown_any
    }
}

/// What Tümünü göster says when the boxes it waited for have come and the view is on them.
pub(crate) const FITTED: &str = "Görünüm, katmanların kaynaktan okunan kapsamına sığdırıldı.";

/// What Tümünü göster says when nothing is shown.
pub(crate) const NOTHING_SHOWN: &str =
    "Görünen katmanlarda gösterilecek nesne yok; gizli ve dondurulmuş katmanlar sayılmaz.";

#[cfg(test)]
#[path = "data_extents_tests.rs"]
mod tests;
