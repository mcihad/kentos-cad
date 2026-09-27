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

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use kentos_contracts::{Entity, LayerNode, LayerNodeType, LayerStyle};
use kentos_domain::{ChangeMark, Changes, Slot};
use kentos_geometry_core::store::Store;
use kentos_native_style::StylePalette;
use kentos_native_style::batches::{DecodeOptions, decode};
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
}

struct Layer {
    style: LayerStyle,
    name: String,
    part: Arc<StyledLayerPart>,
    /// It has infinite lines or rays: built again when their clip box moves.
    construction: bool,
    /// Its objects changed while it was hidden.
    stale: bool,
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
    /// Each object's layer when last built, for objects since removed.
    slot_layer: HashMap<Slot, String>,
    scene: StyledScene,
    /// What the last rebuild cost, and how many layers it built.
    pub last_build: Option<(Duration, usize)>,
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
fn shown_layers(nodes: &[LayerNode]) -> Vec<&LayerNode> {
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

/// Every layer's name by id (`$katman`).
fn names(nodes: &[LayerNode], out: &mut HashMap<String, String>) {
    for n in nodes {
        out.insert(n.id.clone(), n.name.clone());
        names(&n.children, out);
    }
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
        {
            return self.scene.clone();
        }
        let started = Instant::now();
        let mut all = false;
        if self.drawing != Some(drawing) {
            self.layers.clear();
            self.slot_layer.clear();
            self.mark = None;
            self.drawing = Some(drawing);
            all = true;
        }
        if self.look.as_ref() != Some(look) || self.library != library.version() {
            all = true;
        }
        let mut dirty: HashSet<String> = HashSet::new();
        match self.mark.map(|m| doc.changes_since(m)) {
            None | Some(Changes::All) => all = true,
            Some(Changes::Slots(slots)) => {
                for slot in slots {
                    if let Some(e) = doc.get(*slot) {
                        dirty.insert(e.base().layer_id.clone());
                    }
                    if let Some(was) = self.slot_layer.get(slot) {
                        dirty.insert(was.clone());
                    }
                }
            }
        }
        let clip = match self.clip {
            Some(c) if !clip_stale(view, &c) => c,
            _ => {
                let c = construction_clip(view);
                self.clip = Some(c);
                dirty.extend(
                    self.layers
                        .iter()
                        .filter(|(_, l)| l.construction)
                        .map(|(id, _)| id.clone()),
                );
                c
            }
        };
        if all {
            dirty.extend(self.layers.keys().cloned());
            for l in self.layers.values_mut() {
                l.stale = true;
            }
        }
        let shown = shown_layers(doc.layers().nodes());
        let mut layer_names = HashMap::new();
        names(doc.layers().nodes(), &mut layer_names);
        let shown_ids: HashSet<&str> = shown.iter().map(|n| n.id.as_str()).collect();
        // Changed layers that are hidden wait until they show.
        for id in &dirty {
            if !shown_ids.contains(id.as_str())
                && let Some(l) = self.layers.get_mut(id)
            {
                l.stale = true;
            }
        }
        let mut changed_order = self.scene.layers.len() != shown.len();
        // The layers to build, with their objects.
        let mut todo: Vec<(&LayerNode, Vec<&Entity>)> = Vec::new();
        for node in &shown {
            let cached = self.layers.get(&node.id);
            let rebuild = all
                || dirty.contains(&node.id)
                || cached.is_none_or(|l| l.stale || l.style != node.style || l.name != node.name);
            if !rebuild {
                continue;
            }
            changed_order = true;
            let entities: Vec<&Entity> = doc.by_layer(&node.id).collect();
            for e in &entities {
                self.slot_layer.insert(Slot(e.base().id), node.id.clone());
            }
            todo.push((node, entities));
        }
        let built = todo.len();
        // Layers are independent: built side by side on the machine's cores (the store,
        // the library and the look are only read), the biggest first.
        todo.sort_by_key(|(_, e)| std::cmp::Reverse(e.len()));
        let build = |node: &LayerNode, entities: &[&Entity]| {
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
            };
            build_layer(store, &node.style, entities, &opts)
                .and_then(|(_, batches)| {
                    decode(
                        batches,
                        &DecodeOptions {
                            palette: &look.palette,
                            plot_scale: look.symbol_scale,
                            library,
                        },
                    )
                })
                .unwrap_or_default()
        };
        let threads = std::thread::available_parallelism()
            .map_or(1, |n| n.get())
            .clamp(1, 8)
            .min(todo.len());
        let layers: Vec<kentos_native_style::StyledLayer> = if threads <= 1 {
            todo.iter().map(|(n, e)| build(n, e)).collect()
        } else {
            // Each thread takes every `threads`-th layer, so the big ones spread out.
            let mut out: Vec<Option<kentos_native_style::StyledLayer>> = vec![None; todo.len()];
            std::thread::scope(|s| {
                let handles: Vec<_> = (0..threads)
                    .map(|t| {
                        let todo = &todo;
                        let build = &build;
                        s.spawn(move || {
                            (t..todo.len())
                                .step_by(threads)
                                .map(|i| (i, build(todo[i].0, &todo[i].1)))
                                .collect::<Vec<_>>()
                        })
                    })
                    .collect();
                for h in handles {
                    for (i, layer) in h.join().unwrap_or_default() {
                        out[i] = Some(layer);
                    }
                }
            });
            out.into_iter().map(Option::unwrap_or_default).collect()
        };
        for ((node, entities), layer) in todo.iter().zip(layers) {
            self.layers.insert(
                node.id.clone(),
                Layer {
                    style: node.style.clone(),
                    name: node.name.clone(),
                    part: Arc::new(StyledLayerPart {
                        id: NEXT_LAYER.fetch_add(1, Ordering::Relaxed),
                        layer,
                    }),
                    construction: entities.iter().any(|e| construction(e)),
                    stale: false,
                },
            );
        }
        // Layers gone from the tree go with their buffers.
        let known: HashSet<&str> = layer_names.keys().map(String::as_str).collect();
        self.layers.retain(|id, _| known.contains(id.as_str()));
        if changed_order
            || self.scene.under != under
            || !self.scene.layers.iter().zip(&shown).all(|(p, n)| {
                self.layers
                    .get(&n.id)
                    .is_some_and(|l| Arc::ptr_eq(&l.part, p))
            })
        {
            self.scene = StyledScene {
                layers: shown
                    .iter()
                    .filter_map(|n| self.layers.get(&n.id).map(|l| l.part.clone()))
                    .collect(),
                under,
            };
        }
        self.mark = Some(doc.change_mark());
        self.generation = Some(generation);
        self.look = Some(look.clone());
        self.library = library.version();
        if built > 0 {
            self.last_build = Some((started.elapsed(), built));
        }
        self.scene.clone()
    }
}
