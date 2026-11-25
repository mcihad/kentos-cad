//! Veri katmanlarının canlı çizimi (docs/adr/9101 §6, plan/veri.md VY-03): a data layer's rows are read from its source
//! for the view, never kept in the drawing. Each frame the drawing area says what it shows ([`want`]); a layer whose
//! last read does not cover the view (a pan past its margin, a zoom past a step, its style or the colours changed) is
//! read again on a thread of its own, the earlier read stopped. The rows come a part at a time
//! ([`kentos_data::Source::features`]: the box, the columns the look reads, the geometry simplified for the screen),
//! become objects of the layer in a store of their own and go through the style engine as the drawing's layers do
//! (`kentos_plot::layers::build_whole`): a categorized, graduated or rule-based look draws them alike. Each part is drawn
//! as it comes; the scene takes the layer's parts ([`parts`]) in the layer tree's order.
//!
//! The read covers the view and half of it again on every side, so a pan within it reads nothing; the geometry is
//! simplified to half a pixel; a new read starts once the view stops moving. The earlier parts stay drawn until the new
//! read is whole.
//!
//! A layer whose labels are on, or that is being edited, keeps a whole read's objects, up to [`LABEL_ROWS`], in a document
//! of their own ([`take_labels`](DataView::take_labels)): the label engine places them as it places the drawing's, and
//! the drawing tools snap to them (data_labels.rs, docs/adr/9103 §5). A layer being edited is read whole, not
//! simplified: a snap lands on the row's own vertices.
//!
//! A layer of texts (docs/adr/9114 §3) makes each row the text it says (its words, height, angle, alignment and width
//! factor from its columns) and always keeps them: the drawing's texts are drawn from that document, as the drawing's
//! own texts are, while the style engine draws no symbol for a text.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Condvar, Mutex, OnceLock, PoisonError};

use kentos_commands::{DataLayer, Entity, LabelsMode, LayerNode, ProjectSettings, ProjectStyles};
use kentos_data::{Cancel, Feature, Request, Value};
use kentos_geometry_core::geometry::Bounds;
use kentos_geometry_core::store::Store;
use kentos_native_application::sources::TableSystem;
use kentos_native_style::library::StyleLibrary;
use kentos_render_wgpu::styled::StyledLayerPart;

use crate::style::scene::Look;

/// Rows built into one drawn part: the first one shows soon, the later ones keep the build's work bounded.
const PART_ROWS: usize = 8192;
/// Rows a read asks the source for at a time.
const CHUNK: usize = 4096;
/// Rows a read keeps for its labels at most: past it the view is too wide for words to be read, and the label engine
/// would leave nearly all of them out.
pub const LABEL_ROWS: usize = 20_000;

/// What a read was made for: the same table, look, style and library on a view inside its cover at its step.
#[derive(Clone, PartialEq)]
struct Key {
    data: DataLayer,
    node: LayerNode,
    look: Look,
    library: u64,
    /// The rows left out: an edit session's taken and deleted ones (docs/adr/9103 §5), drawn by the drawing's objects
    /// or not at all.
    hidden: Arc<BTreeSet<i64>>,
    /// Being edited: read whole (not simplified) and kept for the snap.
    exact: bool,
    /// The view's scale in quarter octaves: a zoom past a step reads again (the geometry's simplification follows it).
    step: i64,
}

/// A read: what it is for, the box it covers (the project's system), its parts so far, its stop switch.
struct Read {
    id: u64,
    key: Key,
    cover: [f64; 4],
    parts: Vec<Arc<StyledLayerPart>>,
    cancel: Cancel,
    /// Its objects in a document of their own for the labels, once the read is whole.
    labels: Option<Box<kentos_domain::Document>>,
}

/// A layer's reads: the one drawn and the one under way.
#[derive(Default)]
struct Layer {
    drawn: Option<Read>,
    going: Option<Read>,
    /// The last failure, said once.
    failed: Option<String>,
}

/// What a read's thread says back.
enum Said {
    Part(String, u64, Arc<StyledLayerPart>),
    Labels(String, u64, Box<kentos_domain::Document>),
    Done(String, u64),
    Failed(String, u64, String),
}

/// The data layers' reads.
pub struct DataView {
    layers: Mutex<HashMap<String, Layer>>,
    to: Sender<Said>,
    from: Mutex<Receiver<Said>>,
    /// Bumped when a part comes: the scene is built again.
    generation: AtomicU64,
    /// The style library the reads build with, kept while its version stays.
    library: Mutex<Option<(u64, Arc<StyleLibrary>)>>,
    notices: Mutex<Vec<String>>,
    fresh: Mutex<bool>,
    told: Condvar,
}

static SERVICE: OnceLock<DataView> = OnceLock::new();
static NEXT_READ: AtomicU64 = AtomicU64::new(1);
/// Drawn parts' ids, apart from the drawing's and the services' (the renderer uploads an id once).
static NEXT_PART: AtomicU64 = AtomicU64::new(1 << 61);

/// The process's data view.
pub fn service() -> &'static DataView {
    SERVICE.get_or_init(DataView::new)
}

impl DataView {
    /// A data view of its own: the process's is [`service`]; a test's reads stay apart from the others'.
    pub(crate) fn new() -> Self {
        let (to, from) = channel();
        DataView {
            layers: Mutex::new(HashMap::new()),
            to,
            from: Mutex::new(from),
            generation: AtomicU64::new(0),
            library: Mutex::new(None),
            notices: Mutex::new(Vec::new()),
            fresh: Mutex::new(false),
            told: Condvar::new(),
        }
    }
}

/// Whether a data layer is being read or drawn (the window listens for parts then).
pub fn in_use() -> bool {
    SERVICE
        .get()
        .is_some_and(|s| !s.layers.lock().unwrap_or_else(PoisonError::into_inner).is_empty())
}

/// The quarter-octave step of a view's pixels per metre.
fn step_of(px_per_m: f64) -> i64 {
    (px_per_m.max(1e-12).log2() * 4.0).round() as i64
}

/// The box a read covers for a view: the view and half of it again on every side.
fn cover_of(view: &Bounds) -> [f64; 4] {
    let (w, h) = (view.max_x - view.min_x, view.max_y - view.min_y);
    [
        view.min_x - w * 0.5,
        view.min_y - h * 0.5,
        view.max_x + w * 0.5,
        view.max_y + h * 0.5,
    ]
}

fn inside(view: &Bounds, cover: &[f64; 4]) -> bool {
    view.min_x >= cover[0] && view.min_y >= cover[1] && view.max_x <= cover[2] && view.max_y <= cover[3]
}

/// Whether a layer's labels are on: a single label or rules with a class (an absent style labels nothing of a table). A
/// layer of texts is: its texts are drawn from its labels' document (docs/adr/9114 §3).
pub(crate) fn labelled(node: &LayerNode) -> bool {
    if node.data.as_ref().is_some_and(|d| d.text.is_some()) {
        return true;
    }
    match &node.style.labels {
        Some(l) if l.mode == LabelsMode::Off => false,
        Some(l) if l.mode == LabelsMode::Rules => !l.classes.is_empty(),
        _ => node.style.label.is_some(),
    }
}

/// The columns a look reads: the fields whose names its renderer, labels or filter name (a name met anywhere in their
/// JSON is read; a few columns too many cost less than a class drawn without its value).
fn columns_of(node: &LayerNode) -> Vec<String> {
    let mut text = String::new();
    if let Some(r) = &node.style.renderer {
        text.push_str(&r.to_string());
    }
    if let Some(l) = &node.style.labels {
        text.push_str(&serde_json::to_string(l).unwrap_or_default());
    }
    if let Some(l) = &node.style.label {
        text.push_str(&serde_json::to_string(l).unwrap_or_default());
    }
    if let Some(f) = &node.filter {
        text.push_str(&serde_json::to_string(f).unwrap_or_default());
    }
    // A layer of texts reads the columns its texts are made of.
    if let Some(t) = node.data.as_ref().and_then(|d| d.text.as_ref()) {
        text.push_str(&t.columns().join(" "));
    }
    node.fields
        .iter()
        .filter(|f| text.contains(f.name.as_str()))
        .map(|f| f.name.clone())
        .collect()
}

/// A value as the drawing's attributes keep it (`geopackage::attribute_text`).
pub(crate) fn attribute(v: &Value) -> Option<serde_json::Value> {
    kentos_native_application::geopackage::attribute_text(v).map(serde_json::Value::String)
}

/// What a read's thread needs.
struct Job {
    layer: String,
    read: u64,
    key: Key,
    cover: [f64; 4],
    columns: Vec<String>,
    px_per_m: f64,
    settings: ProjectSettings,
    library: Arc<StyleLibrary>,
    cancel: Cancel,
    to: Sender<Said>,
    /// The project's styles, for the document of the read's objects; none: neither labels nor a snap keep them.
    labels: Option<ProjectStyles>,
}

/// A part of the rows drawn: their objects in a store of their own through the style engine.
fn build(job: &Job, entities: &[Entity]) -> Option<Arc<StyledLayerPart>> {
    if entities.is_empty() {
        return None;
    }
    let mut store = Store::new();
    store.put_many(entities.iter().map(|e| {
        (
            f64::from(e.base().id),
            job.layer.as_str(),
            false,
            kentos_processing::geometry::record(e).3,
        )
    }));
    let refs: Vec<&Entity> = entities.iter().collect();
    let names: HashMap<String, String> =
        HashMap::from([(job.layer.clone(), job.key.node.name.clone())]);
    let styles: HashMap<String, kentos_commands::LayerStyle> =
        HashMap::from([(job.layer.clone(), job.key.node.style.clone())]);
    let clip = Bounds {
        min_x: job.cover[0],
        min_y: job.cover[1],
        max_x: job.cover[2],
        max_y: job.cover[3],
    };
    let layer = kentos_plot::layers::build_whole(
        &store,
        &job.library,
        &job.key.node,
        &refs,
        &job.key.look,
        clip,
        kentos_plot::layers::LayerTables {
            names: &names,
            styles: &styles,
        },
    )
    .ok()?;
    Some(Arc::new(StyledLayerPart::new(
        NEXT_PART.fetch_add(1, Ordering::Relaxed),
        layer,
    )))
}

/// A read's objects in a document of their own: the layer as a layer of objects, the project's settings and styles.
fn label_document(job: &Job, styles: ProjectStyles, entities: Vec<Entity>) -> Option<kentos_domain::Document> {
    let mut node = job.key.node.clone();
    node.data = None;
    let snapshot = kentos_commands::DocumentSnapshotV1 {
        format: kentos_commands::DOCUMENT_FORMAT.into(),
        version: kentos_commands::DOCUMENT_VERSION,
        name: String::new(),
        settings: job.settings.clone(),
        origin: kentos_commands::Vec2 { x: 0.0, y: 0.0 },
        home_view: None,
        layers: vec![node],
        active_layer: job.layer.clone(),
        entities,
        styles,
        blocks: Vec::new(),
    };
    kentos_domain::Document::from_snapshot(snapshot).ok()
}

/// The objects the layer's style draws (docs/adr/9105 §3): those of a class switched off, or of no class, are neither
/// labelled nor snapped to, as they are not drawn.
fn drawn_only(job: &Job, objects: Vec<Entity>) -> Vec<Entity> {
    let unit = kentos_native_application::expression_unit::reading_unit(&job.settings);
    let Some(reader) = job
        .key
        .node
        .style
        .renderer
        .as_ref()
        .and_then(|r| kentos_native_application::style_classes::ClassReader::new(r, unit))
    else {
        return objects;
    };
    let refs: Vec<&Entity> = objects.iter().collect();
    let shapes: Vec<_> = if reader.reads_geometry() {
        refs.iter().map(|e| kentos_native_application::geometry::shape(e)).collect()
    } else {
        Vec::new()
    };
    let name = job.key.node.name.clone();
    let drawn = reader.drawn(&refs, &|_| name.clone(), &|i| shapes.get(i));
    objects.into_iter().zip(drawn).filter(|(_, d)| *d).map(|(e, _)| e).collect()
}

/// A read on its thread: the source asked for the cover's rows, each part drawn and handed back as it comes.
fn run(job: Job) {
    let data = &job.key.data;
    let fail = |why: String| {
        let _ = job.to.send(Said::Failed(job.layer.clone(), job.read, why));
    };
    let source = match kentos_data::registry().source(&data.address) {
        Ok(s) => s,
        Err(e) => return fail(e.to_string()),
    };
    let srid = data.geometry.as_ref().and_then(|g| g.srid);
    let system = match TableSystem::of(&job.settings, srid) {
        Ok(s) => s,
        Err(why) => return fail(why),
    };
    let bbox = match &system {
        Some(s) => match s.to_table(job.cover) {
            Some(b) => b,
            None => return fail("görünüm tablonun sistemine çevrilemedi".into()),
        },
        None => job.cover,
    };
    // Half a pixel, in the table's units (a degree is about 111 km).
    let metres = 0.5 / job.px_per_m.max(1e-9);
    let geographic = matches!(srid, Some(4326) | Some(4258) | Some(4269));
    let tolerance = if geographic { metres / 111_320.0 } else { metres };
    // The layer filter's part the source answers itself (docs/adr/9109): fewer rows read; the drawing still asks the
    // filter of those it draws.
    let condition = job.key.node.filter.as_ref().and_then(|f| {
        let unit = kentos_native_application::expression_unit::reading_unit(&job.settings);
        let compiled = kentos_native_application::layer_filter::compile_filter(f, unit).ok()?;
        let schema = source.schema(&data.table).ok()?;
        let typed = kentos_native_application::data_condition::typed(source.driver());
        kentos_native_application::data_condition::pushed(compiled.expr()?, &schema, typed).condition
    });
    let request = Request {
        bbox: data.geometry.as_ref().map(|_| bbox),
        condition,
        columns: Some(job.columns.clone()),
        geometry: data.geometry.is_some(),
        simplify: (!job.key.exact).then_some(tolerance),
        chunk: CHUNK,
        ..Request::default()
    };
    let mut entities: Vec<Entity> = Vec::with_capacity(PART_ROWS);
    // The objects kept for the labels, while they are few enough.
    let mut kept: Option<Vec<Entity>> = job.labels.is_some().then(Vec::new);
    let mut next_id = 1u32;
    // A layer of texts: a row without its height takes the project's text height.
    let default_height = kentos_native_application::data_text::default_height(&job.settings);
    // Each part is said to the window as it comes: it is drawn while the rest is read.
    let flush = |entities: &mut Vec<Entity>| {
        if let Some(part) = build(&job, entities)
            && job.to.send(Said::Part(job.layer.clone(), job.read, part)).is_ok()
        {
            service().tell();
        }
        entities.clear();
    };
    let hidden = &job.key.hidden;
    let read = source.features(&data.table, &request, &job.cancel, &mut |part: Vec<Feature>| {
        for f in part {
            if hidden.contains(&f.fid) {
                continue;
            }
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
            let attrs: serde_json::Map<String, serde_json::Value> = job
                .columns
                .iter()
                .zip(&f.values)
                .filter_map(|(c, v)| attribute(v).map(|a| (c.clone(), a)))
                .collect();
            let made = match &data.text {
                Some(text) => kentos_native_application::data_text::point_of(&g).and_then(|at| {
                    kentos_native_application::data_text::object(text, at, &job.layer, next_id, attrs, default_height)
                }),
                None => kentos_native_application::geopackage::object_of(&g, &job.layer, next_id, attrs),
            };
            if let Some(e) = made {
                next_id = next_id.wrapping_add(1);
                if kept.as_ref().is_some_and(|k| k.len() >= LABEL_ROWS) {
                    kept = None;
                }
                if let Some(k) = kept.as_mut() {
                    k.push(e.clone());
                }
                entities.push(e);
            }
        }
        if entities.len() >= PART_ROWS {
            flush(&mut entities);
        }
        !job.cancel.is_cancelled()
    });
    match read {
        Err(e) => fail(e.to_string()),
        Ok(r) if r.stopped => {}
        Ok(_) => {
            flush(&mut entities);
            if let (Some(styles), Some(objects)) = (job.labels.clone(), kept.map(|k| drawn_only(&job, k)))
                && let Some(doc) = label_document(&job, styles, objects)
            {
                let _ = job.to.send(Said::Labels(job.layer.clone(), job.read, Box::new(doc)));
            }
            let _ = job.to.send(Said::Done(job.layer.clone(), job.read));
        }
    }
}

impl DataView {
    fn tell(&self) {
        *self.fresh.lock().unwrap_or_else(PoisonError::into_inner) = true;
        self.told.notify_one();
    }

    /// Bumps the generation the scene compares with.
    pub fn generation(&self) -> u64 {
        self.generation.load(Ordering::Relaxed)
    }

    /// The failures said since last asked, each once.
    pub fn take_notices(&self) -> Vec<String> {
        std::mem::take(&mut *self.notices.lock().unwrap_or_else(PoisonError::into_inner))
    }

    /// The parts to draw for a layer: the read drawn, else the read under way so far.
    pub fn parts(&self, layer: &str) -> Vec<Arc<StyledLayerPart>> {
        let layers = self.layers.lock().unwrap_or_else(PoisonError::into_inner);
        let Some(l) = layers.get(layer) else {
            return Vec::new();
        };
        match (&l.drawn, &l.going) {
            (Some(d), _) => d.parts.clone(),
            (None, Some(g)) => g.parts.clone(),
            (None, None) => Vec::new(),
        }
    }

    /// The read drawn for a layer, by its id.
    /// Why a layer's last read failed, while it has not been read since (docs/adr/9187: a sheet printed without it).
    pub fn failure(&self, layer: &str) -> Option<String> {
        self.layers.lock().unwrap_or_else(PoisonError::into_inner).get(layer).and_then(|l| l.failed.clone())
    }

    pub fn drawn_read(&self, layer: &str) -> Option<u64> {
        let layers = self.layers.lock().unwrap_or_else(PoisonError::into_inner);
        layers.get(layer).and_then(|l| l.drawn.as_ref()).map(|r| r.id)
    }

    /// The documents of the drawn reads' objects for their labels that came since last asked: the layer, the read and
    /// the document (each handed once).
    pub fn take_labels(&self) -> Vec<(String, u64, Box<kentos_domain::Document>)> {
        self.drain();
        let mut layers = self.layers.lock().unwrap_or_else(PoisonError::into_inner);
        layers
            .iter_mut()
            .filter_map(|(id, l)| {
                let d = l.drawn.as_mut()?;
                Some((id.clone(), d.id, d.labels.take()?))
            })
            .collect()
    }

    /// Takes what the reads said: parts added to their reads, a whole read becoming the drawn one.
    fn drain(&self) {
        let from = self.from.lock().unwrap_or_else(PoisonError::into_inner);
        let mut layers = self.layers.lock().unwrap_or_else(PoisonError::into_inner);
        let mut changed = false;
        while let Ok(said) = from.try_recv() {
            match said {
                Said::Part(layer, read, part) => {
                    if let Some(g) = layers.get_mut(&layer).and_then(|l| l.going.as_mut())
                        && g.id == read
                    {
                        g.parts.push(part);
                        changed = true;
                    }
                }
                Said::Labels(layer, read, doc) => {
                    if let Some(g) = layers.get_mut(&layer).and_then(|l| l.going.as_mut())
                        && g.id == read
                    {
                        g.labels = Some(doc);
                    }
                }
                Said::Done(layer, read) => {
                    if let Some(l) = layers.get_mut(&layer)
                        && l.going.as_ref().is_some_and(|g| g.id == read)
                    {
                        l.drawn = l.going.take();
                        l.failed = None;
                        changed = true;
                    }
                }
                Said::Failed(layer, read, why) => {
                    if let Some(l) = layers.get_mut(&layer)
                        && l.going.as_ref().is_some_and(|g| g.id == read)
                    {
                        l.going = None;
                        if l.failed.as_deref() != Some(why.as_str()) {
                            self.notices
                                .lock()
                                .unwrap_or_else(PoisonError::into_inner)
                                .push(format!("Veri katmanı okunamadı: {why}"));
                        }
                        l.failed = Some(why);
                        changed = true;
                    }
                }
            }
        }
        if changed {
            self.generation.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// The drawing area's view each frame: each shown data layer read again where its last read does not cover what is
    /// asked, or when the rows it leaves out (`hidden`, an edit session's) changed; layers no longer shown are let go
    /// (their reads stopped).
    #[allow(clippy::too_many_arguments)]
    pub fn want(
        &self,
        nodes: &[&LayerNode],
        settings: &ProjectSettings,
        styles: &ProjectStyles,
        view: &Bounds,
        px_per_m: f64,
        look: &Look,
        library: &StyleLibrary,
        settled: bool,
        hidden: &dyn Fn(&str) -> BTreeSet<i64>,
        editing: &dyn Fn(&str) -> bool,
    ) {
        self.drain();
        let shown: HashSet<&str> = nodes.iter().map(|n| n.id.as_str()).collect();
        let mut layers = self.layers.lock().unwrap_or_else(PoisonError::into_inner);
        layers.retain(|id, l| {
            let keep = shown.contains(id.as_str());
            if !keep && let Some(g) = &l.going {
                g.cancel.cancel();
            }
            keep
        });
        if nodes.is_empty() {
            return;
        }
        let version = library.version();
        let library = {
            let mut kept = self.library.lock().unwrap_or_else(PoisonError::into_inner);
            match kept.as_ref() {
                Some((v, l)) if *v == version => Arc::clone(l),
                _ => {
                    let l = Arc::new(library.clone());
                    *kept = Some((version, Arc::clone(&l)));
                    l
                }
            }
        };
        let step = step_of(px_per_m);
        for node in nodes {
            let Some(data) = &node.data else {
                continue;
            };
            let key = Key {
                data: data.clone(),
                node: (*node).clone(),
                look: look.clone(),
                library: version,
                hidden: Arc::new(hidden(&node.id)),
                exact: editing(&node.id),
                step,
            };
            let l = layers.entry(node.id.clone()).or_default();
            let covered = |r: &Read| r.key == key && inside(view, &r.cover);
            if l.going.as_ref().is_some_and(covered) || (l.going.is_none() && l.drawn.as_ref().is_some_and(covered)) {
                continue;
            }
            // A new look or table is read at once; a moved view once it stops (a pan within the cover reads nothing).
            let same_look = l
                .drawn
                .as_ref()
                .or(l.going.as_ref())
                .is_some_and(|r| {
                    r.key.data == key.data
                        && r.key.node == key.node
                        && r.key.look == key.look
                        && r.key.hidden == key.hidden
                        && r.key.exact == key.exact
                });
            if same_look && !settled {
                continue;
            }
            if let Some(g) = l.going.take() {
                g.cancel.cancel();
            }
            let cover = cover_of(view);
            let read = Read {
                id: NEXT_READ.fetch_add(1, Ordering::Relaxed),
                key: key.clone(),
                cover,
                parts: Vec::new(),
                cancel: Cancel::new(),
                labels: None,
            };
            let job = Job {
                layer: node.id.clone(),
                read: read.id,
                key,
                cover,
                columns: columns_of(node),
                px_per_m,
                settings: settings.clone(),
                library: Arc::clone(&library),
                cancel: read.cancel.clone(),
                to: self.to.clone(),
                labels: (labelled(node) || editing(&node.id)).then(|| styles.clone()),
            };
            l.going = Some(read);
            let _ = std::thread::Builder::new()
                .name("kentos-data-view".into())
                .spawn(move || {
                    let to = job.to.clone();
                    run(job);
                    drop(to);
                    service().tell();
                });
        }
    }
}

/// The window's message each time parts came (at most one a frame).
pub fn ready() -> impl iced::futures::Stream<Item = crate::app::Message> {
    use iced::futures::SinkExt;
    let (mut out, stream) = iced::futures::channel::mpsc::channel(1);
    std::thread::spawn(move || {
        let s = service();
        loop {
            {
                let mut fresh = s.fresh.lock().unwrap_or_else(PoisonError::into_inner);
                while !*fresh {
                    fresh = s.told.wait(fresh).unwrap_or_else(PoisonError::into_inner);
                }
                *fresh = false;
            }
            if iced::futures::executor::block_on(out.send(crate::app::Message::DataReady)).is_err() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(16));
        }
    });
    stream
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_read_covers_half_the_view_again_on_every_side() {
        let view = Bounds {
            min_x: 0.0,
            min_y: 0.0,
            max_x: 100.0,
            max_y: 50.0,
        };
        assert_eq!(cover_of(&view), [-50.0, -25.0, 150.0, 75.0]);
        let moved = Bounds {
            min_x: 40.0,
            min_y: 10.0,
            max_x: 140.0,
            max_y: 60.0,
        };
        assert!(inside(&moved, &cover_of(&view)));
        assert_eq!(step_of(1.0), 0);
        assert_eq!(step_of(2.0), 4);
        assert_eq!(attribute(&Value::Bool(true)), Some(serde_json::Value::String("true".into())));
        assert_eq!(attribute(&Value::Null), None);
    }
}

#[cfg(test)]
#[path = "style_policy_tests.rs"]
mod style_policy_tests;
