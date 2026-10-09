//! The drawing's layers through the style engine, for the drawing area
//! (docs/STYLE.md §6, docs/adr/0090): the web's `syncLayers`. Every shown
//! layer is built by the style core next to the geometry store
//! (`kentos-native-style`): each object with its own symbol, else its
//! layer's renderer, else the layer's simple look; text stays with the
//! labels (labels.rs), dimensions keep their hairlines.
//!
//! A layer is built again only when something it draws with changed: its
//! objects (the document's journal says which), its style, the palette, the
//! symbol scale, the library, or, for a layer with construction lines, the
//! box they are clipped to. A pan or a zoom within the symbol scale's step
//! rebuilds nothing; the GPU keeps the batches (crates/render/wgpu styled).
//!
//! A large layer is built in parts (docs/adr/0121): its objects by run of
//! places in the document, which an edit keeps. A change builds its parts
//! again and leaves the others; the GPU skips the parts out of view; the
//! scene draws the parts' batches in the order of the layer built whole
//! (`batches::merged_order`), so the same numbers draw in the same order.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use kentos_contracts::{BlockDefinition, BlockId, Entity, LayerNode, LayerNodeType, LayerStyle};
use kentos_domain::{ChangeMark, Changes, Slot, SlotMap};
use kentos_geometry_core::store::Store;
use kentos_native_style::StylePalette;
use kentos_native_style::batches::{DecodeOptions, StyledLayer, decode, merged_order};
use kentos_native_style::library::StyleLibrary;
use kentos_native_style::program::{BuildOptions, build_layer};
use kentos_render_wgpu::styled::{StyledLayerPart, StyledScene};
use kentos_render_wgpu::{Bounds, Vec2};

static NEXT_LAYER: AtomicU64 = AtomicU64::new(1);

/// How the drawing is drawn: what every layer's build depends on beyond its objects.
#[derive(Clone, Debug, PartialEq)]
pub struct Look {
    pub palette: StylePalette,
    /// The scale symbols are compiled at (`symbol_scale_of`).
    pub symbol_scale: f64,
    /// Symbol sizes on the screen (Semboller → Ekranda sabit).
    pub screen: bool,
    /// Line weights hidden (Kalınlık off).
    pub hairlines: bool,
    pub origin: Vec2,
    /// Görünüm kipleri (docs/adr/0195): what the build leaves out, and the colours.
    pub view_build: kentos_native_style::View,
    pub view_colors: kentos_native_style::color::ViewColors,
}

/// Places per part of a large layer: a part holds the layer's objects whose
/// places in the document fall in one run of `PART_PLACES` (docs/adr/0121).
pub(crate) const PART_PLACES: u64 = 4096;
/// A layer is built in parts from this many objects, and whole again below half of it.
pub(crate) const PARTS_FROM: usize = 8192;

/// A part of a layer as built.
struct Part {
    part: Arc<StyledLayerPart>,
    /// It has infinite lines or rays: built again when their clip box moves.
    construction: bool,
}

struct Layer {
    style: LayerStyle,
    name: String,
    /// Its parts by run of places (`place / PART_PLACES`), in document
    /// order; a layer built whole has one, at 0.
    parts: BTreeMap<u64, Part>,
    /// Built in parts: large, and nothing of it reads `$sıra`.
    split: bool,
    /// An expression of it reads `$sıra`: it is built whole, as one run.
    reads_index: bool,
    /// Its objects changed while it was hidden.
    stale: bool,
    /// Its time setting when built (docs/adr/0210 §6): another one gives its objects other times.
    time: Option<kentos_contracts::LayerTime>,
    /// Its filter when built (docs/adr/0211 §3): another one lets other objects through.
    filter: Option<kentos_contracts::LayerFilter>,
}

/// What of a shown layer is built again: all of it, or some of its parts.
#[derive(Default)]
struct Dirty {
    parts: HashSet<u64>,
}

/// The styled layers as last built, and what they were built from.
#[derive(Default)]
pub struct StyledCache {
    /// Which opened drawing (the viewport's generation), its journal mark, and
    /// its generation (every change to it, the layer tree's included).
    drawing: Option<u64>,
    mark: Option<ChangeMark>,
    generation: Option<u64>,
    look: Option<Look>,
    library: u64,
    clip: Option<Bounds>,
    layers: HashMap<String, Layer>,
    /// Each object's layer and part when last built, for objects since
    /// removed or moved; the layer's id shared, not copied per object.
    slot_part: SlotMap<(Arc<str>, u64)>,
    /// The block definitions when last built: the document keeps an unchanged
    /// definition's `Arc`, so a changed one is found by pointer.
    blocks: Vec<Arc<BlockDefinition>>,
    scene: StyledScene,
    /// Each service layer's part (docs/adr/0208 §3): its service's key and
    /// opacity, and the one batch the service pass draws its tiles for.
    services: HashMap<String, (String, u64, Arc<StyledLayerPart>)>,
    /// What the last rebuild cost, and how many layer parts it built.
    pub last_build: Option<(Duration, usize)>,
    /// The time slider's window when last built, and the objects the view left out (by the window or a layer's filter)
    /// (docs/adr/0210 §6): a part whose objects came or went is built again.
    window: Option<kentos_geometry_core::time::Window>,
    time_hidden: SlotSet,
}

/// Slots with the quick hash (a temporal layer's hundred thousand objects are asked each step).
type SlotSet = HashSet<u32, std::hash::BuildHasherDefault<kentos_domain::SlotHasher>>;

/// The definitions made, changed or removed between two lists of the
/// document's (an unchanged definition keeps its `Arc`).
fn changed_blocks(
    before: &[Arc<BlockDefinition>],
    after: &[Arc<BlockDefinition>],
) -> HashSet<BlockId> {
    let was: HashMap<BlockId, &Arc<BlockDefinition>> = before.iter().map(|b| (b.id, b)).collect();
    let now: HashSet<BlockId> = after.iter().map(|b| b.id).collect();
    after
        .iter()
        .filter(|b| !was.get(&b.id).is_some_and(|w| Arc::ptr_eq(w, b)))
        .map(|b| b.id)
        .chain(before.iter().map(|b| b.id).filter(|id| !now.contains(id)))
        .collect()
}

/// Is `e` an infinite line or a ray (clipped to a box around the view)?
fn construction(e: &Entity) -> bool {
    matches!(e, Entity::Xline(_) | Entity::Ray(_))
}

/// The box construction lines are clipped to: the view and three times its size around it.
pub fn construction_clip(view: &Bounds) -> Bounds {
    let (w, h) = (view.max_x - view.min_x, view.max_y - view.min_y);
    Bounds {
        min_x: view.min_x - 3.0 * w,
        min_y: view.min_y - 3.0 * h,
        max_x: view.max_x + 3.0 * w,
        max_y: view.max_y + 3.0 * h,
    }
}

/// Whether construction lines must be clipped again: the view left the box, or zoomed far in.
fn clip_stale(view: &Bounds, clip: &Bounds) -> bool {
    let inside = view.min_x >= clip.min_x
        && view.min_y >= clip.min_y
        && view.max_x <= clip.max_x
        && view.max_y <= clip.max_y;
    !inside || (clip.max_x - clip.min_x) > 20.0 * (view.max_x - view.min_x)
}

/// The shown layers, bottom first (the top of the tree draws last), as the plain scene orders them.
pub(crate) fn shown_layers(nodes: &[LayerNode]) -> Vec<&LayerNode> {
    fn walk<'a>(nodes: &'a [LayerNode], visible: bool, out: &mut Vec<&'a LayerNode>) {
        for node in nodes {
            match node.kind {
                LayerNodeType::Layer => {
                    if visible && node.visible {
                        out.push(node);
                    }
                }
                LayerNodeType::Group => walk(&node.children, visible && node.visible, out),
            }
        }
    }
    let mut out = Vec::new();
    walk(nodes, true, &mut out);
    out.reverse();
    out
}

/// The services of the layers drawn now (shown, with their groups), bottom first.
pub(crate) fn shown_service_layers(nodes: &[LayerNode]) -> Vec<&kentos_contracts::ServiceLayer> {
    shown_layers(nodes)
        .into_iter()
        .filter_map(|n| n.service.as_ref())
        .collect()
}

/// Every layer's name by id (`$katman`).
pub(crate) fn names(nodes: &[LayerNode], out: &mut HashMap<String, String>) {
    for n in nodes {
        out.insert(n.id.clone(), n.name.clone());
        names(&n.children, out);
    }
}

/// One layer through the style engine whole, as a sheet's map frame draws it (the web's
/// app/sheet/mapFrames.ts `styleAt` and `buildStyledLayer`): its objects' symbols at `look`'s
/// scale and palette, its construction lines clipped to `clip`; empty where the engine refuses it.
pub(crate) fn build_whole(
    store: &Store,
    library: &StyleLibrary,
    node: &LayerNode,
    entities: &[&Entity],
    look: &Look,
    clip: Bounds,
    names: &HashMap<String, String>,
) -> StyledLayer {
    let opts = BuildOptions {
        origin: look.origin,
        plot_scale: look.symbol_scale,
        screen: look.screen,
        hairlines: look.hairlines,
        clip: Some(clip),
        library,
        layer_name: &|id: &str| names.get(id).cloned().unwrap_or_else(|| id.to_owned()),
        view: look.view_build,
    };
    build_layer(store, &node.style, entities, &opts)
        .and_then(|(_, batches)| {
            decode(
                batches,
                &DecodeOptions {
                    palette: &look.palette,
                    plot_scale: look.symbol_scale,
                    library,
                    view: look.view_colors,
                },
            )
        })
        .unwrap_or_default()
}

impl StyledCache {
    /// The symbol scale the layers were last built at, and whether on the screen.
    pub fn built_scale(&self) -> Option<(f64, bool)> {
        self.look.as_ref().map(|l| (l.symbol_scale, l.screen))
    }

    /// The styled scene of `doc`, from the cache where nothing it depends on changed.
    #[allow(clippy::too_many_arguments)]
    pub fn scene(
        &mut self,
        drawing: u64,
        doc: &kentos_domain::Document,
        store: &Store,
        library: &StyleLibrary,
        look: &Look,
        view: &Bounds,
        under: usize,
    ) -> StyledScene {
        // Nothing it depends on changed: the frame costs a comparison (a pan, a zoom within a step).
        let generation = doc.generation();
        if self.drawing == Some(drawing)
            && self.generation == Some(generation)
            && self.look.as_ref() == Some(look)
            && self.library == library.version()
            && self.scene.under == under
            && self.clip.is_some_and(|c| !clip_stale(view, &c))
            && self.window == store.time_window()
        {
            return self.scene.clone();
        }
        let started = Instant::now();
        let mut all = false;
        if self.drawing != Some(drawing) {
            self.layers.clear();
            self.slot_part.clear();
            self.time_hidden.clear();
            self.mark = None;
            self.drawing = Some(drawing);
            all = true;
        }
        if self.look.as_ref() != Some(look) || self.library != library.version() {
            all = true;
        }
        // The parts the changes touched: where each changed object is now, and where it was.
        let mut dirty: HashMap<String, Dirty> = HashMap::new();
        match self.mark.map(|m| doc.changes_since(m)) {
            None | Some(Changes::All) => all = true,
            Some(Changes::Slots(slots)) => {
                // Each slot once: a run changes hundreds of thousands at once.
                let mut slots = slots.to_vec();
                slots.sort_unstable();
                slots.dedup();
                for slot in slots {
                    // A layer not built yet is built whole: its objects' parts
                    // need no finding.
                    if let Some(e) = doc.get(slot)
                        && self.layers.contains_key(e.base().layer_id.as_str())
                        && let Some(place) = doc.place(slot)
                    {
                        dirty_part(&mut dirty, &e.base().layer_id, place / PART_PLACES);
                    }
                    if let Some((layer, part)) = self.slot_part.get(&slot) {
                        dirty_part(&mut dirty, layer, *part);
                    }
                }
            }
        }
        // A definition changed (an edit, an undo, another editor's; docs/adr/0144): the
        // parts holding an insert of it, or of one placing it, are built again.
        let changed = changed_blocks(&self.blocks, doc.blocks());
        if !all && !changed.is_empty() {
            let reached = kentos_contracts::blocks::reaching(doc.blocks(), &changed);
            for e in doc.entities() {
                if let Entity::Insert(i) = e
                    && reached.contains(&i.block)
                    && self.layers.contains_key(e.base().layer_id.as_str())
                    && let Some(place) = doc.place(Slot(e.base().id))
                {
                    dirty_part(&mut dirty, &e.base().layer_id, place / PART_PLACES);
                }
            }
        }
        if !changed.is_empty() {
            self.blocks = doc.blocks().to_vec();
        }
        // Zaman sürgüsü (docs/adr/0210 §6): a temporal layer shows the objects the slider's window
        // shows; when the window moves, the parts whose objects come or go are built again.
        let window = store.time_window();
        if window != self.window {
            self.window = window;
            if !all {
                for node in doc.layers().leaves() {
                    if node.time.is_none() || !self.layers.contains_key(node.id.as_str()) {
                        continue;
                    }
                    // Who is where is enough: the objects themselves are not read. An object
                    // shown now and noted hidden (or the reverse) came or went.
                    let mut last = None;
                    for (place, Slot(id)) in doc.layer_slots(&node.id) {
                        let key = place / PART_PLACES;
                        if last != Some(key)
                            && store.view_shown(f64::from(id)) == self.time_hidden.contains(&id)
                        {
                            dirty_part(&mut dirty, &node.id, key);
                            last = Some(key);
                        }
                    }
                }
            }
        }
        let clip = match self.clip {
            Some(c) if !clip_stale(view, &c) => c,
            _ => {
                let c = construction_clip(view);
                self.clip = Some(c);
                for (id, layer) in &self.layers {
                    for (key, part) in &layer.parts {
                        if part.construction {
                            dirty.entry(id.clone()).or_default().parts.insert(*key);
                        }
                    }
                }
                c
            }
        };
        if all {
            for l in self.layers.values_mut() {
                l.stale = true;
            }
        }
        let shown = shown_layers(doc.layers().nodes());
        let mut layer_names = HashMap::new();
        names(doc.layers().nodes(), &mut layer_names);
        let shown_ids: HashSet<&str> = shown.iter().map(|n| n.id.as_str()).collect();
        // Changed layers that are hidden wait until they show.
        for id in dirty.keys() {
            if !shown_ids.contains(id.as_str())
                && let Some(l) = self.layers.get_mut(id)
            {
                l.stale = true;
            }
        }
        // What to build: a layer whole (all of its parts, or its one run), or
        // the parts of it that changed. `reset`: the layer's parts are replaced.
        struct Plan<'d> {
            node: &'d LayerNode,
            reset: bool,
            split: bool,
            parts: Vec<(u64, Vec<&'d Entity>)>,
        }
        let mut plans: Vec<Plan> = Vec::new();
        for node in &shown {
            // A service layer holds no objects: its tiles are the service pass's.
            if node.service.is_some() {
                continue;
            }
            let cached = self.layers.get(&node.id);
            let count = doc.count(&node.id);
            let reads_index = cached.is_some_and(|l| l.reads_index && l.style == node.style);
            let split = !reads_index
                && match cached {
                    Some(l) if l.split => count >= PARTS_FROM / 2,
                    _ => count >= PARTS_FROM,
                };
            let reset = all
                || cached.is_none_or(|l| {
                    l.stale
                        || l.style != node.style
                        || l.name != node.name
                        || l.split != split
                        || l.time != node.time
                        || l.filter != node.filter
                });
            let changed = dirty.get(&node.id);
            if !reset && changed.is_none() {
                continue;
            }
            // The slider's window over a temporal layer: the objects it shows, the others noted.
            // The view's filter over a layer: its own filter, or the slider's window over a temporal layer.
            let timed = node.filter.is_some() || (node.time.is_some() && window.is_some());
            let hidden = &mut self.time_hidden;
            let mut keep = |e: &&Entity| {
                let id = e.base().id;
                let shown = !timed || store.view_shown(f64::from(id));
                if shown {
                    hidden.remove(&id);
                } else {
                    hidden.insert(id);
                }
                shown
            };
            let parts = if !split {
                vec![(0, doc.by_layer(&node.id).filter(&mut keep).collect())]
            } else if reset {
                // Every part: the layer's objects by run of places.
                let mut parts: Vec<(u64, Vec<&Entity>)> = Vec::new();
                for (place, e) in doc.by_layer_placed(&node.id, 0..u64::MAX) {
                    let key = place / PART_PLACES;
                    // A part all of whose objects the window leaves out is still a part (an empty one).
                    let shown = keep(&e);
                    match parts.last_mut() {
                        Some((k, list)) if *k == key => {
                            if shown {
                                list.push(e);
                            }
                        }
                        _ => parts.push((key, if shown { vec![e] } else { Vec::new() })),
                    }
                }
                parts
            } else {
                let mut keys: Vec<u64> =
                    changed.map_or_else(Vec::new, |d| d.parts.iter().copied().collect());
                keys.sort_unstable();
                keys.into_iter()
                    .map(|key| {
                        let places = key * PART_PLACES..(key + 1) * PART_PLACES;
                        (
                            key,
                            doc.by_layer_placed(&node.id, places)
                                .map(|(_, e)| e)
                                .filter(&mut keep)
                                .collect(),
                        )
                    })
                    .collect()
            };
            plans.push(Plan {
                node,
                reset,
                split,
                parts,
            });
        }
        let build = |node: &LayerNode, entities: &[&Entity]| -> (StyledLayer, bool) {
            let opts = BuildOptions {
                origin: look.origin,
                plot_scale: look.symbol_scale,
                screen: look.screen,
                hairlines: look.hairlines,
                clip: Some(clip),
                library,
                layer_name: &|id: &str| {
                    layer_names
                        .get(id)
                        .cloned()
                        .unwrap_or_else(|| id.to_owned())
                },
                view: look.view_build,
            };
            build_layer(store, &node.style, entities, &opts)
                .and_then(|(call, batches)| {
                    decode(
                        batches,
                        &DecodeOptions {
                            palette: &look.palette,
                            plot_scale: look.symbol_scale,
                            library,
                            view: look.view_colors,
                        },
                    )
                    .map(|layer| (layer, call.reads_index))
                })
                .unwrap_or_default()
        };
        // Every part to build, the biggest first: built side by side on the
        // machine's cores (the store, the library and the look are only read).
        let mut jobs: Vec<(usize, usize)> = plans
            .iter()
            .enumerate()
            .flat_map(|(p, plan)| (0..plan.parts.len()).map(move |k| (p, k)))
            .collect();
        jobs.sort_by_key(|&(p, k)| std::cmp::Reverse(plans[p].parts[k].1.len()));
        let run = |&(p, k): &(usize, usize)| build(plans[p].node, &plans[p].parts[k].1);
        let threads = std::thread::available_parallelism()
            .map_or(1, |n| n.get())
            .clamp(1, 8)
            .min(jobs.len());
        let mut built: HashMap<(usize, usize), (StyledLayer, bool)> = HashMap::new();
        if threads <= 1 {
            built.extend(jobs.iter().map(|j| (*j, run(j))));
        } else {
            // Each thread takes every `threads`-th job, so the big ones spread out.
            std::thread::scope(|s| {
                let handles: Vec<_> = (0..threads)
                    .map(|t| {
                        let jobs = &jobs;
                        let run = &run;
                        s.spawn(move || {
                            (t..jobs.len())
                                .step_by(threads)
                                .map(|i| (jobs[i], run(&jobs[i])))
                                .collect::<Vec<_>>()
                        })
                    })
                    .collect();
                for h in handles {
                    built.extend(h.join().unwrap_or_default());
                }
            });
        }
        let count = jobs.len();
        for (p, plan) in plans.iter().enumerate() {
            let id = &plan.node.id;
            let mut results: Vec<(u64, &[&Entity], StyledLayer, bool)> = plan
                .parts
                .iter()
                .enumerate()
                .map(|(k, (key, entities))| {
                    let (layer, reads) = built.remove(&(p, k)).unwrap_or_default();
                    (*key, entities.as_slice(), layer, reads)
                })
                .collect();
            let mut split = plan.split;
            let mut reset = plan.reset;
            // A part reads `$sıra`: the layer is one run, built whole again.
            if split && results.iter().any(|r| r.3) {
                let timed =
                    plan.node.filter.is_some() || (plan.node.time.is_some() && window.is_some());
                let entities: Vec<&Entity> = doc
                    .by_layer(id)
                    .filter(|e| !timed || store.view_shown(f64::from(e.base().id)))
                    .collect();
                let (layer, _) = build(plan.node, &entities);
                split = false;
                reset = true;
                results = Vec::new();
                let shared: Arc<str> = Arc::from(id.as_str());
                self.slot_part.reserve(entities.len());
                for e in &entities {
                    self.slot_part
                        .insert(Slot(e.base().id), (Arc::clone(&shared), 0));
                }
                let entry = self.layers.entry(id.clone()).or_insert_with(|| Layer {
                    style: plan.node.style.clone(),
                    name: plan.node.name.clone(),
                    parts: BTreeMap::new(),
                    split: false,
                    reads_index: true,
                    stale: false,
                    time: plan.node.time.clone(),
                    filter: plan.node.filter.clone(),
                });
                entry.parts.clear();
                entry.parts.insert(0, new_part(layer, &entities));
                entry.reads_index = true;
            }
            let reads_index = results.iter().any(|r| r.3);
            let entry = self.layers.entry(id.clone()).or_insert_with(|| Layer {
                style: plan.node.style.clone(),
                name: plan.node.name.clone(),
                parts: BTreeMap::new(),
                split,
                reads_index,
                stale: false,
                time: plan.node.time.clone(),
                filter: plan.node.filter.clone(),
            });
            if reset {
                entry.style = plan.node.style.clone();
                entry.name = plan.node.name.clone();
                entry.split = split;
                entry.stale = false;
                entry.time = plan.node.time.clone();
                entry.filter = plan.node.filter.clone();
                if !results.is_empty() {
                    entry.parts.clear();
                    entry.reads_index = reads_index;
                }
            }
            let shared: Arc<str> = Arc::from(id.as_str());
            self.slot_part
                .reserve(results.iter().map(|r| r.1.len()).sum());
            for (key, entities, layer, _) in results {
                for e in entities {
                    // Most objects of a part built again were there before: their entry stays.
                    match self.slot_part.get_mut(&Slot(e.base().id)) {
                        Some((layer, at)) if *at == key && **layer == *shared => {}
                        Some(entry) => *entry = (Arc::clone(&shared), key),
                        None => {
                            self.slot_part
                                .insert(Slot(e.base().id), (Arc::clone(&shared), key));
                        }
                    }
                }
                // A part left with no objects goes; a layer built whole keeps its one.
                if entities.is_empty() && split {
                    entry.parts.remove(&key);
                } else {
                    entry.parts.insert(key, new_part(layer, entities));
                }
            }
        }
        // Layers gone from the tree go with their buffers.
        let known: HashSet<&str> = layer_names.keys().map(String::as_str).collect();
        self.layers.retain(|id, _| known.contains(id.as_str()));
        // The scene: the shown layers' parts, bottom first, and when a layer
        // comes in parts, the order of the layer built whole.
        let mut parts: Vec<Arc<StyledLayerPart>> = Vec::new();
        let mut order: Vec<(u32, u32)> = Vec::new();
        let mut split_any = false;
        let mut served = HashSet::new();
        for node in &shown {
            if let Some(service) = &node.service {
                let base = parts.len();
                let part = self.service_part(doc, node, service);
                served.insert(node.id.clone());
                order.push((base as u32, 0));
                parts.push(part);
                continue;
            }
            let Some(layer) = self.layers.get(&node.id) else {
                continue;
            };
            let base = parts.len();
            let list: Vec<&Arc<StyledLayerPart>> = layer.parts.values().map(|p| &p.part).collect();
            if list.len() > 1 {
                split_any = true;
                let layers: Vec<&StyledLayer> = list.iter().map(|p| &p.layer).collect();
                order.extend(
                    merged_order(&layers)
                        .into_iter()
                        .map(|(p, b)| ((base + p) as u32, b as u32)),
                );
            } else {
                for part in &list {
                    order.extend((0..part.layer.batches.len()).map(|b| (base as u32, b as u32)));
                }
            }
            parts.extend(list.into_iter().cloned());
        }
        self.services.retain(|id, _| served.contains(id));
        let same = self.scene.under == under
            && self.scene.layers.len() == parts.len()
            && self
                .scene
                .layers
                .iter()
                .zip(&parts)
                .all(|(a, b)| Arc::ptr_eq(a, b));
        if !same {
            self.scene = StyledScene {
                layers: parts,
                under,
                order: split_any.then(|| Arc::new(order)),
            };
        }
        self.mark = Some(doc.change_mark());
        self.generation = Some(generation);
        self.look = Some(look.clone());
        self.library = library.version();
        if count > 0 {
            self.last_build = Some((started.elapsed(), count));
        }
        self.scene.clone()
    }
}

impl StyledCache {
    /// A service layer's part: one batch the service pass draws its tiles
    /// for, under the key the hub knows the service by; made again only when
    /// the service, its connection or its opacity changes.
    fn service_part(
        &mut self,
        doc: &kentos_domain::Document,
        node: &LayerNode,
        service: &kentos_contracts::ServiceLayer,
    ) -> Arc<StyledLayerPart> {
        use kentos_native_style::batches::{BatchKind, FillPaintBatch, StyledBatch, Unit};
        let connection = service
            .connection
            .as_ref()
            .and_then(|c| doc.settings().connections.iter().find(|k| &k.id == c));
        let key = crate::services::key_of(service, connection);
        let opacity = service.opacity.unwrap_or(1.0);
        if let Some((k, o, part)) = self.services.get(&node.id)
            && *k == key
            && *o == opacity.to_bits()
        {
            return part.clone();
        }
        let part = Arc::new(StyledLayerPart {
            id: NEXT_LAYER.fetch_add(1, Ordering::Relaxed),
            layer: StyledLayer {
                data: Vec::new(),
                batches: vec![StyledBatch {
                    range: 0..0,
                    kind: BatchKind::Fill {
                        paint: FillPaintBatch::Service {
                            service: key.clone(),
                            opacity,
                        },
                    },
                    level: 0.0,
                    key: 0,
                    // Everywhere: the pass draws the tiles the view meets.
                    bounds: [
                        f64::NEG_INFINITY,
                        f64::NEG_INFINITY,
                        f64::INFINITY,
                        f64::INFINITY,
                    ],
                    origin: [0.0, 0.0],
                    reach: 0.0,
                    reach_unit: Unit::World,
                    min_scale: None,
                    max_scale: None,
                }],
            },
        });
        self.services
            .insert(node.id.clone(), (key, opacity.to_bits(), part.clone()));
        part
    }
}

/// A part of `layer` to build again; the layer's id is copied only when it
/// is not listed yet.
fn dirty_part(dirty: &mut HashMap<String, Dirty>, layer: &str, part: u64) {
    match dirty.get_mut(layer) {
        Some(d) => {
            d.parts.insert(part);
        }
        None => {
            dirty
                .entry(layer.to_owned())
                .or_default()
                .parts
                .insert(part);
        }
    }
}

/// A built part of a layer, under a new id (the GPU uploads what it has not seen).
fn new_part(layer: StyledLayer, entities: &[&Entity]) -> Part {
    Part {
        part: Arc::new(StyledLayerPart {
            id: NEXT_LAYER.fetch_add(1, Ordering::Relaxed),
            layer,
        }),
        construction: entities.iter().any(|e| construction(e)),
    }
}
