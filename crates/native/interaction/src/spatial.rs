//! The desktop's copy of the drawing for picking, object snap and window
//! selection (docs/adr/0029): the shared geometry store
//! (`kentos_geometry_core::store`), which the web feeds through WASM
//! (`apps/web/src/viewport/picking.ts`, `PickIndex`), kept in step with the
//! native document.
//!
//! - **Incrementally.** The document notes every object an applied op
//!   touches (`Document::changes_since`, the web's `touched` event); a sync
//!   puts the touched objects as they are now and removes the ones that are
//!   gone, in the order they were first touched, so new objects land in the
//!   document's order and returning ones take their places. Only a journal
//!   that no longer reaches back (or another drawing) reads everything again.
//! - **Directly.** An object becomes the store's shape in one place,
//!   [`record`], with no JSON on the way; the shared cases in
//!   `fixtures/store-records/v1` hold it to the records the web packs
//!   (`apps/web/src/wasm/pack.ts`).
//! - **Layers as the web gives them.** Every node of the layer tree with
//!   its visibility and lock resolved through the groups above it, interior
//!   picking and the label rule, as `layerTable` sends them; sent again only
//!   when the table changed.
//!
//! - **Blocks as the document holds them** (docs/adr/0144): the definitions
//!   are sent again when one of them changed (the document keeps each
//!   shared, so a pointer says it), before the objects, so every insert is
//!   placed with the definitions of its revision.
//!
//! Queries answer by slot. What they decide is the store's, rule for rule
//! the web's: points and edges before interiors, the smallest area, window
//! and crossing boxes, the snap kinds' weights.

use std::collections::HashSet;
use std::hash::BuildHasherDefault;
use std::sync::Arc;

use kentos_commands::{
    BlockDefinition, Entity, EntityId, LayerFilter, LayerNode, LayerSnap, LayerTime,
};
use kentos_domain::{ChangeMark, Changes, Document, LayerTree, Slot, SlotHasher, Uuid};
use kentos_geometry_core::entity::{Shape, entity_area, entity_length, entity_vertices};
use kentos_geometry_core::geom::dimension::dimension_measure;
use kentos_geometry_core::geometry::Bounds;
use kentos_geometry_core::ops::holes::HoleAt;
use kentos_geometry_core::store::labels::{
    DIMENSION_PREFIXES, DIMENSION_UNITS, LABEL_CELL, LABEL_DIMENSION, LABEL_LEADER, LABEL_LINE,
    LABEL_PARAGRAPH_MASK, LABEL_PIECE_DIMENSION, LABEL_PIECE_LEADER, LABEL_PIECE_LINE,
    LABEL_PIECE_TEXT, LABEL_STRIDE, LABEL_TEXT,
};
use kentos_geometry_core::store::legible::LABEL_SHOWN_STRIDE;
pub use kentos_geometry_core::store::legible::LabelSize;
use kentos_geometry_core::store::placing::{
    LABEL_PLACED, LABEL_PLACED_CALLOUT, LABEL_PLACED_LETTER, LABEL_PLACED_LINE, LabelHit,
    ObjectLabels, Pin, PlaceOptions, label_at,
};
use kentos_geometry_core::store::polygon::PolygonMode;
use kentos_geometry_core::store::snap::{Extension, SnapExtras, SnapHit};
use kentos_geometry_core::store::{LayerFlags, Store};
use kentos_native_application::blocks::{core_blocks, piece_entities};
use kentos_native_application::expression_unit::{Unit, reading_unit};
use kentos_native_application::geometry::{drawing_font, shape};
use kentos_native_application::label_texts::{
    label_defaults_json, label_layers_json, layer_texts, object_texts, texts_key,
};
use kentos_native_application::layer_filter::{CompiledFilter, compile_filter};

use crate::Vec2;

/// The geometry store and where it stands in the document's changes.
#[derive(Default)]
pub struct Spatial {
    store: Store,
    /// The scale a sheet's map draws the walls at, none: the drawing's (docs/adr/0339 §4).
    wall_scale: Option<f64>,
    /// Ephemeral model view denominator; never changes the project's paper scale.
    screen_wall_scale: Option<f64>,
    sheet_view:Option<kentos_native_application::viewport_scope::Viewport>,
    sheet_layers:Option<HashSet<String>>,
    /// The document's changes the store has taken.
    mark: ChangeMark,
    /// The document's revision at the last sync: nothing to do while it holds.
    revision: Option<u64>,
    /// The layer table sent last.
    layers: Vec<(String, LayerFlags)>,
    /// The block definitions sent last.
    blocks: Vec<Arc<BlockDefinition>>,
    /// The objects whose label a text writes, as sent last (docs/adr/0175 §4).
    text_labelled: Vec<Slot>,
    /// The spaces that have a tag, as sent last (docs/adr/0344's 27 October addendum).
    tagged_spaces: Vec<Slot>,
    /// How many times every object was read (a drawing opened, or the journal fell behind).
    reloads: u64,
    /// The temporal layers' time settings the objects' times were made with (docs/adr/0210 §6).
    rules: std::collections::HashMap<String, LayerTime>,
    /// Whether an object has had a time since the store was emptied: a put object off a temporal layer loses its own.
    timed: bool,
    /// The layers' filters the objects' marks were made with (docs/adr/0211 §3).
    filters: std::collections::HashMap<String, Filtered>,
    /// New objects their layer's filter left out since the app last asked, by layer, in the order met
    /// (docs/adr/0211 §3: said once for the change).
    hidden_new: Vec<(String, usize)>,
    /// The label engine's layers as last sent (docs/adr/0212 §3.1), and each layer's texts' setting the
    /// texts were made with.
    label_layers: String,
    label_keys: std::collections::HashMap<String, String>,
    /// The main view's last labels (`label_at`): their records, stride and scale.
    kept: std::cell::RefCell<(Vec<f64>, usize, f64)>,
    /// Where the main view's labels went last (docs/adr/0533 §3): the next placing of a view that moved a little
    /// puts a label back where it was when that is free. Only the main view keeps one.
    prior: std::cell::RefCell<kentos_geometry_core::store::placing::Prior>,
    /// The active view's theme (docs/adr/0523 §2): its layer state's visibility and freeze are what is picked
    /// and snapped to; none, the drawing's.
    theme: Option<kentos_commands::LayerState>,
    /// Kat görünümü (docs/adr/0715) and the active level it reads; what it hides and draws faint as last made, and
    /// how many times that changed (the drawing area builds its looks again then).
    level_view: crate::level_view::LevelView,
    active_level: Option<String>,
    level_looks: Arc<crate::level_view::LevelLooks>,
    level_version: u64,
    /// The objects on a level other than the active one, whatever the view: a click takes the others first where
    /// objects lie over each other (docs/adr/0715 §2).
    elsewhere: HashSet<u32>,
    /// The stairs put as the plan of the level above theirs shows them (docs/adr/0724 §4).
    upper: HashSet<u32>,
}

/// A layer's filter as the store's marks were made with it: compiled in the
/// project's unit (or why not: then nothing passes), and how many of the
/// layer's objects pass.
#[derive(Debug)]
struct Filtered {
    filter: LayerFilter,
    unit: Unit,
    compiled: Result<CompiledFilter, String>,
    passed: usize,
    total: usize,
}

impl Spatial {
    /// An empty store; [`Spatial::reload`] fills it.
    pub fn new() -> Self {
        Self::default()
    }

    /// A store holding `doc`.
    pub fn of(doc: &Document) -> Self {
        let mut spatial = Self::new();
        spatial.reload(doc);
        spatial
    }

    /// Draws the walls as a sheet's map at 1:`scale` does (docs/adr/0339 §4), none: at the drawing's scale. Set
    /// before [`Spatial::reload`].
    pub fn set_wall_scale(&mut self, scale: Option<f64>) {
        self.wall_scale = scale;
    }

    pub fn has_screen_wall_scale(&self) -> bool {
        self.screen_wall_scale.is_some()
    }

    /// Rebuild only when the chosen detail changes; a camera move does not edit the document.
    pub fn set_screen_wall_scale(&mut self, doc: &Document, scale: Option<f64>) {
        let previous = self
            .screen_wall_scale
            .map(|_| self.store.wall_view().detail);
        self.screen_wall_scale = scale.filter(|scale| scale.is_finite() && *scale > 0.0);
        let view = match (self.wall_scale, self.screen_wall_scale) {
            (Some(scale), _) => kentos_native_application::walls::view_at(doc.settings(), scale),
            (None, Some(scale)) => {
                kentos_native_application::walls::screen_view(doc.settings(), scale, previous)
            }
            _ => kentos_native_application::walls::view_of(doc.settings()),
        };
        self.store.set_wall_view(view);
    }

    pub fn has_sheet_view(&self) -> bool {
        self.sheet_view.is_some()
    }

    /// Kat görünümü with `active` the active level (docs/adr/0715): what it hides is neither picked nor snapped to,
    /// what it draws faint is snapped to only. Made again at once when either changed.
    pub fn set_level_view(
        &mut self,
        doc: &Document,
        view: crate::level_view::LevelView,
        active: Option<String>,
    ) {
        if self.level_view == view && self.active_level == active {
            return;
        }
        self.level_view = view;
        self.active_level = active;
        self.sync_phases(doc);
    }

    /// Kat görünümü as set.
    pub fn level_view(&self) -> crate::level_view::LevelView {
        self.level_view
    }

    /// What Kat görünümü hides and draws faint now, and its version: a new one whenever that changed.
    pub fn level_looks(&self) -> (u64, Arc<crate::level_view::LevelLooks>) {
        (self.level_version, Arc::clone(&self.level_looks))
    }

    /// Whether the store draws some objects otherwise than the drawing's own plan does: a sheet's map edited in place
    /// (docs/adr/0716), or the stairs under the active level drawn from above (docs/adr/0724 §4). A sheet's map then
    /// wants a store of its own.
    pub fn draws_otherwise(&self) -> bool {
        self.sheet_view.is_some() || !self.upper.is_empty()
    }

    /// Explicit command input for tools that query the same source through the public catalog.
    pub fn sheet_model_scope(&self)->Option<kentos_commands::SheetModelScope> {
        let view=self.sheet_view.as_ref()?;
        Some(kentos_commands::SheetModelScope{sheet:view.sheet.clone()?,item:view.item.clone()})
    }

    /// Change only the active derived geometry index. The source document and its uids are untouched.
    pub fn set_sheet_view(
        &mut self,
        doc: &Document,
        view: Option<kentos_native_application::viewport_scope::Viewport>,
    ) {
        let layers = view
            .as_ref()
            .map(|v| kentos_native_application::viewport_scope::visible_layers(doc, v));
        let geometry = self
            .sheet_view
            .as_ref()
            .map(|v| (v.denominator, v.annotation_scale))
            != view.as_ref().map(|v| (v.denominator, v.annotation_scale));
        let changed = self.sheet_view != view || self.sheet_layers != layers;
        self.wall_scale = view.as_ref().map(|v| v.denominator);
        self.sheet_view = view;
        self.sheet_layers = layers;
        if geometry {
            self.reload(doc);
        } else if changed {
            self.sync_layers_of(doc);
            self.sync_sheet_boundary();
            // The map's level may be what changed (docs/adr/0716).
            self.sync_phases(doc);
        }
    }
    fn sync_sheet_boundary(&mut self) {
        if let Some(view) = &self.sheet_view {
            kentos_native_application::viewport_scope::apply_boundary(&mut self.store, view);
        } else {
            self.store.set_scope_visible(None);
        }
    }
    /// Whether a sheet's map at 1:`scale` draws `doc`'s walls otherwise than the drawing does (docs/adr/0339 §4):
    /// then the map wants a store of its own.
    pub fn walls_differ_at(doc: &Document, scale: f64) -> bool {
        doc.has_scale_dependent_geometry()
            && kentos_native_application::walls::view_at(doc.settings(), scale)
                != kentos_native_application::walls::view_of(doc.settings())
    }

    fn wall_view_of(&self, doc: &Document) -> kentos_geometry_core::geom::wall::View {
        match self.wall_scale {
            Some(scale) => kentos_native_application::walls::view_at(doc.settings(), scale),
            None => match self.screen_wall_scale {
                Some(scale) => kentos_native_application::walls::screen_view(
                    doc.settings(),
                    scale,
                    Some(self.store.wall_view().detail),
                ),
                None => kentos_native_application::walls::view_of(doc.settings()),
            },
        }
    }

    /// Reads every object of `doc` again: a drawing was opened, or the
    /// document's journal no longer reaches back to the store's mark.
    pub fn reload(&mut self, doc: &Document) {
        self.reloads += 1;
        self.store.clear();
        if let Err(e) = self.store.set_label_defaults_json(&label_defaults_json()) {
            debug_assert!(false, "the kinds' label styles: {e}");
        }
        self.store
            .set_font(drawing_font(doc.settings().drawing_font));
        self.store.set_wall_view(self.wall_view_of(doc));
        self.blocks = doc.blocks().to_vec();
        self.store.set_blocks(core_blocks(&self.blocks));
        // Before the objects: a tagged space is drawn without its label at once.
        self.tagged_spaces.clear();
        self.sync_tagged_spaces(doc);
        let scale=self.sheet_view.as_ref().map(|v|v.annotation_scale);
        self.store.put_many(doc.entities().filter_map(|e|kentos_native_application::viewport_scope::record_at(e,scale)));
        self.store
            .set_hosts(kentos_native_application::geometry_store::opening_hosts(doc, doc.entities()));
        // The masks are picked by their boundary only (docs/adr/0310 §4).
        self.store.set_masks(
            doc.entities()
                .filter(|e| e.base().look.wipeout.is_some())
                .map(|e| (f64::from(e.base().id), true)),
        );
        self.text_labelled.clear();
        self.sync_text_labelled(doc);
        self.mark = doc.change_mark();
        self.revision = Some(doc.revision());
        self.layers.clear();
        self.sync_layers_of(doc);
        self.rules.clear();
        self.timed = false;
        self.sync_times(doc, &[]);
        self.filters.clear();
        self.sync_filters(doc, &[], false);
        self.label_layers.clear();
        self.label_keys.clear();
        // Another drawing's labels were somewhere else.
        *self.prior.borrow_mut() = Default::default();
        let all: Vec<&Entity> = doc.entities().collect();
        self.sync_labels(doc, &all, true);
        self.sync_phases(doc);
        self.sync_sheet_boundary();
    }

    /// The objects the project's phase view hides (docs/adr/0346 §2): every object again while a view other than
    /// Tümü is set (an object's phase may come from its wall or its tag's object), none to do otherwise.
    /// Kat görünümü (docs/adr/0715) joins it: the levels it hides are hidden as well, the level it draws faint is
    /// snapped to only. A sheet's map edited in place shows its own level instead (docs/adr/0716).
    fn sync_phases(&mut self, doc: &Document) {
        // A sheet's map edited in place shows its own level (docs/adr/0716), not the session's.
        let (view, active) = match &self.sheet_view {
            Some(v) => match &v.level {
                Some(l) => (
                    if l.below {
                        crate::level_view::LevelView::Below
                    } else {
                        crate::level_view::LevelView::Active
                    },
                    Some(l.id.clone()),
                ),
                None => (crate::level_view::LevelView::All, None),
            },
            None => (self.level_view, self.active_level.clone()),
        };
        let looks = crate::level_view::looks(doc, view, active.as_deref());
        self.sync_upper(doc, &looks.upper);
        // What is faint and what is seen from above is snapped to, never picked.
        let unpicked = |l: &crate::level_view::LevelLooks| {
            l.faded
                .iter()
                .chain(&l.upper)
                .map(|&id| f64::from(id))
                .collect::<Vec<_>>()
        };
        if self.sheet_view.is_some() {
            self.elsewhere.clear();
            let shown = crate::level_view::LevelLooks {
                hidden: looks.hidden,
                faded: looks.faded,
                upper: looks.upper,
                elsewhere: Vec::new(),
            };
            let mut hidden = kentos_native_application::viewport_scope::hidden_entities(doc);
            hidden.extend(shown.hidden.iter().map(|&id| f64::from(id)));
            self.store.set_phase_hidden(hidden);
            self.store.set_unpicked(unpicked(&shown));
            if *self.level_looks != shown {
                self.level_looks = Arc::new(shown);
                self.level_version += 1;
            }
            return;
        }
        self.elsewhere = looks.elsewhere.iter().copied().collect();
        let shown = crate::level_view::LevelLooks {
            hidden: looks.hidden,
            faded: looks.faded,
            upper: looks.upper,
            elsewhere: Vec::new(),
        };
        if *self.level_looks != shown {
            self.level_looks = Arc::new(shown);
            self.level_version += 1;
        }
        if self.level_looks.faded.is_empty() && self.level_looks.upper.is_empty() {
            if !self.store.no_unpicked() {
                self.store.set_unpicked(std::iter::empty());
            }
        } else {
            self.store.set_unpicked(unpicked(&self.level_looks));
        }
        let view = doc
            .settings()
            .arch
            .as_ref()
            .and_then(|a| a.phase_view)
            .unwrap_or_default();
        let levels = self.level_looks.hidden.iter().map(|&id| f64::from(id));
        if view == kentos_commands::PhaseView::All {
            if self.level_looks.hidden.is_empty() {
                if !self.store.no_phase_hidden() {
                    self.store.set_phase_hidden(std::iter::empty());
                }
            } else {
                self.store.set_phase_hidden(levels);
            }
            return;
        }
        let by_uid = |u: &str| {
            Uuid::parse_str(u)
                .ok()
                .and_then(|u| doc.slot_of(u))
                .and_then(|s| doc.get(s))
        };
        let hidden: Vec<f64> = doc
            .entities()
            .filter(|e| view.look(e.phase(by_uid)) == kentos_commands::PhaseLook::Hidden)
            .map(|e| f64::from(e.base().id))
            .chain(levels)
            .collect();
        self.store.set_phase_hidden(hidden);
    }

    /// Puts the stairs of `upper` as the plan of the level above theirs shows them (docs/adr/0724 §4) and those that
    /// left it as their own level's plan does; a stair put again by a sync came back as its own plan's and is put
    /// once more. Their labels' texts come again with them.
    fn sync_upper(&mut self, doc: &Document, upper: &[u32]) {
        let now: HashSet<u32> = upper.iter().copied().collect();
        let is_upper = |store: &Store, id: u32| {
            store
                .get(f64::from(id))
                .is_some_and(|it| matches!(&it.shape, Shape::Stair { stair, .. } if stair.upper))
        };
        let mut again: Vec<u32> = now
            .iter()
            .copied()
            .filter(|&id| !is_upper(&self.store, id))
            .collect();
        again.extend(self.upper.iter().copied().filter(|id| !now.contains(id)));
        self.upper = now;
        if again.is_empty() {
            return;
        }
        again.sort_unstable();
        let scale = self.sheet_view.as_ref().map(|v| v.annotation_scale);
        let entities: Vec<&Entity> = again.iter().filter_map(|&id| doc.get(Slot(id))).collect();
        let upper = &self.upper;
        let records: Vec<_> = entities
            .iter()
            .filter_map(|e| {
                let (id, layer, label, mut shape) =
                    kentos_native_application::viewport_scope::record_at(e, scale)?;
                if let Shape::Stair { stair, .. } = &mut shape {
                    stair.upper = upper.contains(&e.base().id);
                }
                Some((id, layer, label, shape))
            })
            .collect();
        self.store.put_many(records);
        self.sync_labels(doc, &entities, false);
    }

    /// Brings the store up to date with `doc`: the block definitions and the
    /// objects touched since the last sync, and the layer table, when they
    /// changed. No object is read while the document's revision and journal
    /// are where they were.
    pub fn sync(&mut self, doc: &Document) {
        // The definitions first, whatever else moved: another editor's change of
        // them moves neither the journal nor the revision (docs/adr/0144 §5).
        let blocks = doc.blocks();
        if blocks.len() != self.blocks.len()
            || blocks
                .iter()
                .zip(&self.blocks)
                .any(|(a, b)| !Arc::ptr_eq(a, b))
        {
            self.blocks = blocks.to_vec();
            self.store.set_blocks(core_blocks(&self.blocks));
        }
        let mark = doc.change_mark();
        if mark == self.mark && self.revision == Some(doc.revision()) {
            return;
        }
        self.store
            .set_font(drawing_font(doc.settings().drawing_font));
        // Walls at the detail the settings say now (docs/adr/0339 §4); the same view expands none again.
        self.store.set_wall_view(self.wall_view_of(doc));
        match doc.changes_since(self.mark) {
            Changes::All => self.reload(doc),
            Changes::Slots(slots) => {
                // Each slot once, in slot order: new objects go in in the
                // order they were made (slots are given in turn).
                let mut slots = slots.to_vec();
                slots.sort_unstable();
                slots.dedup();
                let mut gone = Vec::new();
                let mut changed = Vec::with_capacity(slots.len());
                for slot in slots {
                    match doc.get(slot) {
                        Some(entity) => changed.push(entity),
                        None => gone.push(f64::from(slot.0)),
                    }
                }
                // Objects new to the store on a filtered layer, when the change is this editor's (one from
                // outside moves the journal, not the revision): those the filter leaves out are said.
                let local = self.revision != Some(doc.revision());
                let fresh: Vec<&Entity> = if local {
                    changed
                        .iter()
                        .copied()
                        .filter(|e| {
                            doc.layers()
                                .get(&e.base().layer_id)
                                .is_some_and(|n| n.filter.is_some())
                                && self.store.get(f64::from(e.base().id)).is_none()
                        })
                        .collect()
                } else {
                    Vec::new()
                };
                self.store.remove(&gone);
                let scale=self.sheet_view.as_ref().map(|v|v.annotation_scale);
                for entity in &changed {
                    match kentos_native_application::viewport_scope::record_at(entity,scale) {Some(record)=>{self.store.put_many(std::iter::once(record));},None=>self.store.remove(&[f64::from(entity.base().id)])}
                }
                // The openings put, in their walls (docs/adr/0341 §2).
                self.store.set_hosts(kentos_native_application::geometry_store::opening_hosts(
                    doc,
                    changed.iter().copied(),
                ));
                self.store.set_masks(
                    changed
                        .iter()
                        .map(|e| (f64::from(e.base().id), e.base().look.wipeout.is_some())),
                );
                self.sync_text_labelled(doc);
                self.sync_tagged_spaces(doc);
                self.mark = mark;
                self.revision = Some(doc.revision());
                self.sync_layers_of(doc);
                self.sync_times(doc, &changed);
                self.sync_filters(doc, &changed, !gone.is_empty());
                self.sync_labels(doc, &changed, false);
                self.sync_phases(doc);
                self.sync_sheet_boundary();
                for e in fresh {
                    if self.store.filter_shown(f64::from(e.base().id)) {
                        continue;
                    }
                    let layer = &e.base().layer_id;
                    match self.hidden_new.iter_mut().find(|(l, _)| l == layer) {
                        Some((_, n)) => *n += 1,
                        None => self.hidden_new.push((layer.clone(), 1)),
                    }
                }
            }
        }
    }

    /// The label engine's inputs (docs/adr/0212 §3.1): the layers' labelling
    /// when it changed; the texts of a layer whose texts' setting changed
    /// (its classes' texts, conditions and templates; its name, for
    /// `$katman`) whole, then those of the objects just put (`all`: every
    /// object, after a reload); the objects' pins (a put object lost its own).
    fn sync_labels(&mut self, doc: &Document, put: &[&Entity], all: bool) {
        let leaves: Vec<&LayerNode> = doc
            .layers()
            .leaves()
            .into_iter()
            .filter(|l| l.service.is_none())
            .collect();
        let table = label_layers_json(&leaves);
        if table != self.label_layers {
            if let Err(e) = self.store.set_label_layers_json(&table) {
                debug_assert!(false, "the layers' labelling: {e}");
            }
            self.label_layers = table;
        }
        // A label reads lengths and areas in the project's unit: another unit makes every layer's texts again.
        let unit = reading_unit(doc.settings());
        let mut whole: Vec<String> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for l in &leaves {
            seen.insert(l.id.clone());
            let key = format!("{}|{}", texts_key(l), unit.factor());
            if self.label_keys.get(&l.id) != Some(&key) {
                if self.label_keys.contains_key(&l.id) && !all {
                    whole.push(l.id.clone());
                }
                self.label_keys.insert(l.id.clone(), key);
            }
        }
        self.label_keys.retain(|id, _| seen.contains(id));
        let names = |id: &str| {
            doc.layers()
                .get(id)
                .map_or_else(|| id.to_owned(), |n| n.name.clone())
        };
        let mut by_layer: Vec<(String, Vec<&Entity>)> = Vec::new();
        for id in &whole {
            by_layer.push((
                id.clone(),
                doc.layer_slots(id)
                    .filter_map(|(_, s)| doc.get(s))
                    .collect(),
            ));
        }
        for e in put {
            let layer = &e.base().layer_id;
            if whole.contains(layer) {
                continue;
            }
            match by_layer.iter_mut().find(|(id, _)| id == layer) {
                Some((_, list)) => list.push(e),
                None => by_layer.push((layer.clone(), vec![e])),
            }
        }
        for (layer, list) in by_layer {
            let t = layer_texts(doc.layers().get(&layer), unit);
            let store = &self.store;
            let texts = object_texts(&list, &t, &names, &|i| {
                store.get(f64::from(list[i].base().id)).map(|it| &it.shape)
            });
            for (e, (texts, z)) in list.iter().zip(texts) {
                self.store
                    .set_object_labels(f64::from(e.base().id), ObjectLabels { texts, z });
            }
        }
        for e in put {
            let pins = &e.base().label_pins;
            if !pins.is_empty() {
                self.store.set_label_pins(
                    f64::from(e.base().id),
                    pins.iter()
                        .map(|p| Pin {
                            class: p.class.clone(),
                            at: p.at.map(|a| Vec2::new(a.x, a.y)),
                            rotation: p.rotation.unwrap_or(0.0),
                            hidden: p.hidden == Some(true),
                        })
                        .collect(),
                );
            }
        }
    }

    /// The label under `at` among the main view's last labels, within `tol`
    /// px; `all` counts the unplaced and hidden (docs/adr/0212 §4).
    pub fn label_at(&self, at: Vec2, tol: f64, all: bool) -> Option<LabelHit> {
        let kept = self.kept.borrow();
        label_at(&kept.0, kept.1, kept.2, at, tol, all)
    }

    /// The labels whose middle is inside the box `from`–`to` among the main
    /// view's last labels (Etiketi sabitle's window, docs/adr/0212 §4).
    pub fn labels_in(&self, from: Vec2, to: Vec2, all: bool) -> Vec<LabelHit> {
        let kept = self.kept.borrow();
        kentos_geometry_core::store::placing::labels_in(&kept.0, kept.1, from, to, all)
    }

    /// Where an object's labels are pinned from: its anchor (docs/adr/0212 §2).
    pub fn label_anchor(&self, slot: Slot) -> Option<Vec2> {
        self.store.label_anchor(f64::from(slot.0))
    }

    /// The new objects their layer's filter left out since the last call, by layer id, with how many
    /// (docs/adr/0211 §3); the list starts again empty.
    pub fn take_hidden_new(&mut self) -> Vec<(String, usize)> {
        std::mem::take(&mut self.hidden_new)
    }

    /// The layers' filters' marks (docs/adr/0211 §3): a layer whose filter
    /// changed (or that has one since the store was emptied) whole, then the
    /// objects just put that are on a filtered layer; an object put off one
    /// is let in again. The layers' counts follow when anything moved
    /// (`removed`: objects went).
    fn sync_filters(&mut self, doc: &Document, put: &[&Entity], removed: bool) {
        let next: std::collections::HashMap<String, LayerFilter> = doc
            .layers()
            .leaves()
            .into_iter()
            .filter(|l| l.service.is_none())
            .filter_map(|l| l.filter.clone().map(|f| (l.id.clone(), f)))
            .collect();
        let gone: Vec<String> = self
            .filters
            .keys()
            .filter(|id| !next.contains_key(*id))
            .cloned()
            .collect();
        for id in gone {
            self.filters.remove(&id);
            let ids: Vec<f64> = doc.layer_slots(&id).map(|(_, s)| f64::from(s.0)).collect();
            self.store
                .set_filtered(ids.into_iter().map(|id| (id, false)));
        }
        // Another drawing unit compares the conditions' lengths and areas in it (docs/adr/0165 §2).
        let unit = reading_unit(doc.settings());
        let whole: Vec<String> = next
            .iter()
            .filter(|(id, f)| {
                self.filters.get(*id).map(|x| (&x.filter, x.unit)) != Some((*f, unit))
            })
            .map(|(id, _)| id.clone())
            .collect();
        for id in &whole {
            let filter = next[id].clone();
            let compiled = compile_filter(&filter, unit);
            // A list: only its objects can pass. It is turned into slots once (the document's index
            // of persistent ids), and the layer's other objects are left out unread (docs/adr/0211 §6).
            let listed: Option<HashSet<Slot, BuildHasherDefault<SlotHasher>>> = compiled
                .as_ref()
                .ok()
                .and_then(CompiledFilter::listed)
                .map(|ids| {
                    ids.iter()
                        .filter_map(|u| doc.slot_of(Uuid::from_bytes(u.0)))
                        .collect()
                });
            // Without a condition the list alone decides: nothing is read. With one, it is asked
            // only of the listed objects.
            let condition = compiled.as_ref().is_ok_and(CompiledFilter::has_condition);
            let fails = compiled.is_err();
            self.filters.insert(
                id.clone(),
                Filtered {
                    filter,
                    unit,
                    compiled,
                    passed: 0,
                    total: doc.count(id),
                },
            );
            let mut marks: Vec<(f64, bool)> = Vec::with_capacity(doc.count(id));
            let mut asked: Vec<&Entity> = Vec::new();
            for (_, slot) in doc.layer_slots(id) {
                let at = f64::from(slot.0);
                // A condition that does not compile lets nothing through (docs/adr/0211 §3).
                if fails || listed.as_ref().is_some_and(|l| !l.contains(&slot)) {
                    marks.push((at, true));
                } else if !condition {
                    marks.push((at, false));
                } else if let Some(e) = doc.get(slot) {
                    asked.push(e);
                }
            }
            let listed_in = marks.iter().filter(|(_, left)| !left).count();
            self.store.set_filtered(marks);
            // The whole layer asked: its count is what passed, no second pass over it.
            let passed = listed_in + self.filter_objects(doc, &asked, id);
            if let Some(f) = self.filters.get_mut(id) {
                f.passed = passed;
            }
        }
        let mut off = Vec::new();
        if !put.is_empty() {
            let mut by_layer: std::collections::HashMap<&str, Vec<&Entity>> =
                std::collections::HashMap::new();
            let mut left = Vec::new();
            for e in put {
                let layer = e.base().layer_id.as_str();
                if whole.iter().any(|w| w == layer) {
                    continue;
                }
                let Some(f) = self.filters.get(layer) else {
                    off.push(f64::from(e.base().id));
                    continue;
                };
                // The list first, by the object's persistent id; the condition then.
                let listed = match &f.compiled {
                    Err(_) => false,
                    Ok(c) => c.listed().is_none_or(|ids| {
                        doc.uid(Slot(e.base().id))
                            .is_some_and(|u| ids.contains(&EntityId(*u.as_bytes())))
                    }),
                };
                if listed {
                    by_layer.entry(layer).or_default().push(e);
                } else {
                    left.push((f64::from(e.base().id), true));
                }
            }
            self.store.set_filtered(left);
            for (layer, list) in by_layer {
                let layer = layer.to_owned();
                self.filter_objects(doc, &list, &layer);
            }
            self.store.set_filtered(off.iter().map(|&id| (id, false)));
        }
        if !put.is_empty() || removed {
            self.recount(doc);
        }
    }

    /// The marks of `list`, listed objects of filtered layer `layer`, into the store by the layer's
    /// condition; how many of them pass.
    fn filter_objects(&mut self, doc: &Document, list: &[&Entity], layer: &str) -> usize {
        let Some(f) = self.filters.get(layer) else {
            return 0;
        };
        let out: Vec<(f64, bool)> = match &f.compiled {
            // A condition that does not compile lets nothing through (docs/adr/0211 §3).
            Err(_) => list
                .iter()
                .map(|e| (f64::from(e.base().id), true))
                .collect(),
            Ok(c) => {
                let names = |id: &str| {
                    doc.layers()
                        .get(id)
                        .map_or_else(|| id.to_owned(), |n| n.name.clone())
                };
                let store = &self.store;
                let met = c.condition(list, &names, |i| {
                    store.get(f64::from(list[i].base().id)).map(|it| &it.shape)
                });
                list.iter()
                    .zip(met)
                    .map(|(e, p)| (f64::from(e.base().id), !p))
                    .collect()
            }
        };
        let passed = out.iter().filter(|(_, left)| !left).count();
        self.store.set_filtered(out);
        passed
    }

    /// Each filtered layer's objects that pass, and all of them (Katmanlar's counts).
    fn recount(&mut self, doc: &Document) {
        let store = &self.store;
        for (id, f) in self.filters.iter_mut() {
            let mut passed = 0;
            let mut total = 0;
            for (_, s) in doc.layer_slots(id) {
                total += 1;
                passed += usize::from(store.filter_shown(f64::from(s.0)));
            }
            (f.passed, f.total) = (passed, total);
        }
    }

    /// Whether the object passes its layer's filter (always without one).
    pub fn filter_shown(&self, slot: Slot) -> bool {
        self.store.filter_shown(f64::from(slot.0))
    }

    /// Whether the view shows the object: it passes its layer's filter and shows at the slider's window.
    pub fn view_shown(&self, slot: Slot) -> bool {
        self.store.view_shown(f64::from(slot.0))
    }

    /// A filtered layer's objects that pass its filter, and all of them; none without a filter.
    pub fn filter_counts(&self, layer: &str) -> Option<(usize, usize)> {
        self.filters.get(layer).map(|f| (f.passed, f.total))
    }

    /// Why a layer's filter lets nothing through: its condition does not compile.
    pub fn filter_error(&self, layer: &str) -> Option<&str> {
        self.filters
            .get(layer)
            .and_then(|f| f.compiled.as_ref().err())
            .map(String::as_str)
    }

    /// The temporal layers' objects' times (docs/adr/0210 §6): a layer whose
    /// time setting changed (or that has one since the store was emptied)
    /// whole, then the objects just put that are on a temporal layer, each
    /// from its start and end attributes; an object put off a temporal layer
    /// loses its time.
    fn sync_times(&mut self, doc: &Document, put: &[&Entity]) {
        let next: std::collections::HashMap<String, LayerTime> = doc
            .layers()
            .leaves()
            .into_iter()
            .filter(|l| l.service.is_none())
            .filter_map(|l| l.time.clone().map(|t| (l.id.clone(), t)))
            .collect();
        let gone: Vec<String> = self
            .rules
            .keys()
            .filter(|id| !next.contains_key(*id))
            .cloned()
            .collect();
        for id in gone {
            let ids: Vec<f64> = doc.by_layer(&id).map(|e| f64::from(e.base().id)).collect();
            self.store.set_recorded([], ids.iter().copied());
            self.store.set_times(ids.into_iter().map(|id| (id, None)));
        }
        let whole: Vec<String> = next
            .iter()
            .filter(|(id, t)| self.rules.get(*id) != Some(*t))
            .map(|(id, _)| id.clone())
            .collect();
        self.rules = next;
        for id in &whole {
            let list: Vec<&Entity> = doc.by_layer(id).collect();
            self.time(&list, id);
        }
        if put.is_empty() {
            return;
        }
        let mut by_layer: std::collections::HashMap<&str, Vec<&Entity>> =
            std::collections::HashMap::new();
        let mut off = Vec::new();
        for e in put {
            let layer = e.base().layer_id.as_str();
            if whole.iter().any(|w| w == layer) {
                continue;
            }
            if self.rules.contains_key(layer) {
                by_layer.entry(layer).or_default().push(e);
            } else if self.timed {
                off.push(f64::from(e.base().id));
            }
        }
        for (layer, list) in by_layer {
            let layer = layer.to_owned();
            self.time(&list, &layer);
        }
        self.store.set_recorded([], off.iter().copied());
        self.store.set_times(off.into_iter().map(|id| (id, None)));
    }

    /// The times of `list`, objects of temporal layer `layer`, into the store.
    fn time(&mut self, list: &[&Entity], layer: &str) {
        let Some(rule) = self.rules.get(layer) else {
            return;
        };
        if list.is_empty() {
            return;
        }
        let ranged = rule.end.is_some();
        let core = kentos_geometry_core::time::Rule {
            ranged,
            cumulative: rule.cumulative,
        };
        let (times, _) = kentos_geometry_core::time::layer_times(
            core,
            list.iter().map(|e| {
                let attrs = &e.base().attrs;
                (
                    attrs.get(&rule.start).map(String::as_str),
                    rule.end
                        .as_ref()
                        .and_then(|k| attrs.get(k))
                        .map(String::as_str),
                )
            }),
        );
        let ids: Vec<f64> = list.iter().map(|e| f64::from(e.base().id)).collect();
        // The versions' system time (docs/adr/0537): a layer recording them gives each its interval.
        match &rule.recorded_start {
            Some(start) => {
                let read = |e: &Entity, key: Option<&String>| {
                    key.and_then(|k| e.base().attrs.get(k))
                        .map_or(kentos_geometry_core::time::Read::Empty, |v| {
                            kentos_geometry_core::time::read(v)
                        })
                };
                let recorded = list.iter().map(|e| {
                    kentos_geometry_core::time::recorded_interval(
                        read(e, Some(start)),
                        read(e, rule.recorded_end.as_ref()),
                    )
                });
                self.store
                    .set_recorded(ids.iter().copied().zip(recorded), []);
            }
            None => self.store.set_recorded([], ids.iter().copied()),
        }
        self.store.set_times(ids.into_iter().zip(times));
        self.timed = true;
    }

    /// The day the drawing is shown as known (docs/adr/0537): the versions not known then are left out; none ends it.
    pub fn set_known_at(&mut self, at: Option<f64>) {
        self.store.set_known_at(at);
    }

    /// The time slider's window (docs/adr/0210 §5): queries leave out the
    /// temporal layers' objects it does not show; none ends the filter.
    pub fn set_time_window(&mut self, window: Option<kentos_geometry_core::time::Window>) {
        self.store.set_time_window(window);
    }

    /// Whether the object shows at the slider's window (always without one, or for an object without a time).
    pub fn time_shown(&self, slot: Slot) -> bool {
        self.store.time_shown(f64::from(slot.0))
    }

    /// How many objects on shown layers have a time, and their extent (the slider's range).
    pub fn time_summary(&self) -> (usize, Option<(f64, f64)>) {
        self.store.time_summary()
    }

    /// Sends the spaces that have a tag when they differ from those sent last: they draw no label of their own, their
    /// tag is it (docs/adr/0344's 27 October addendum).
    fn sync_tagged_spaces(&mut self, doc: &Document) {
        let now = doc.tagged_spaces();
        if now != self.tagged_spaces {
            let ids: Vec<f64> = now.iter().map(|slot| f64::from(slot.0)).collect();
            self.store.set_tagged_spaces(&ids);
            self.tagged_spaces = now;
        }
    }

    /// Sends the objects whose label a text writes when they differ from
    /// those sent last: they show no label of their own (docs/adr/0175 §4).
    fn sync_text_labelled(&mut self, doc: &Document) {
        let now = doc.text_labelled();
        if now != self.text_labelled {
            let ids: Vec<f64> = now.iter().map(|slot| f64::from(slot.0)).collect();
            self.store.set_text_labelled(&ids);
            self.text_labelled = now;
        }
    }

    /// The active view's theme (docs/adr/0523 §2): what is picked and snapped to follows its layer state;
    /// none, the drawing's tree. Sent at once.
    pub fn set_theme(&mut self, doc: &Document, theme: Option<kentos_commands::LayerState>) {
        if self.theme != theme {
            self.theme = theme;
            self.sync_layers_of(doc);
        }
    }

    /// The rows round a bound layer's part that only hold room for the labels near its edge (docs/adr/0578 §1); none
    /// takes them away. The labels are placed again against them.
    pub fn set_ghosts(
        &mut self,
        layer: &str,
        ghosts: Vec<(Shape, Option<kentos_geometry_core::time::Time>)>,
    ) {
        self.store.set_ghosts_timed(layer, ghosts);
        *self.prior.borrow_mut() = Default::default();
    }

    /// A layer's obstacle-only neighbours with their times (a copy of the drawing at another scale takes them,
    /// docs/adr/0636 §2).
    pub fn ghosts_of(&self, layer: &str) -> Vec<(Shape, Option<kentos_geometry_core::time::Time>)> {
        self.store.ghosts_of(layer)
    }

    /// Takes every layer's obstacle-only neighbours away (another drawing).
    pub fn clear_all_ghosts(&mut self) {
        self.store.clear_ghosts();
        *self.prior.borrow_mut() = Default::default();
    }

    /// How many obstacle-only neighbours the store holds, of every layer.
    pub fn ghost_total(&self) -> usize {
        self.store.ghost_count(None)
    }

    /// How many obstacle-only neighbours the layer has in the store.
    pub fn ghost_count(&self, layer: &str) -> usize {
        self.store.ghost_count(Some(layer))
    }

    /// The active view's theme.
    pub fn theme(&self) -> Option<&kentos_commands::LayerState> {
        self.theme.as_ref()
    }

    /// The layer table of the drawing, or of the active view's theme.
    fn sync_layers_of(&mut self, doc: &Document) {
        match &self.theme {
            Some(state) => {
                let tree = kentos_domain::layer_states::themed(doc.layers(), state);
                self.sync_layers(&tree);
            }
            None => self.sync_layers(doc.layers()),
        }
    }

    /// Sends the layer table when it differs from the one sent last.
    fn sync_layers(&mut self, tree: &LayerTree) {
        let mut rows = layer_rows(tree);
        if let Some(visible)=&self.sheet_layers {for(id,flags)in &mut rows {flags.visible=visible.contains(id);}}
        if rows != self.layers {
            self.store
                .set_layers(rows.iter().map(|(id, flags)| (id.as_str(), *flags)));
            self.layers = rows;
        }
    }

    /// The store, for queries this type does not wrap.
    pub fn store(&self) -> &Store {
        &self.store
    }

    /// Geometry for transient highlighting; the persistent source is always borrowed unless this annotation differs.
    pub fn represented<'a>(&self, entity: &'a Entity) -> Option<std::borrow::Cow<'a, Entity>> {
        use std::borrow::Cow;
        let Some(view) = &self.sheet_view else {
            return Some(Cow::Borrowed(entity));
        };
        let item = self.store.get(f64::from(entity.base().id))?;
        if !self.store.shown(item) {
            return None;
        }
        kentos_native_application::viewport_scope::entity_at(entity, Some(view.annotation_scale))
    }

    /// A block's insert as the objects it draws (docs/adr/0144): its pieces
    /// as the store placed them, each with its own colour and line weight,
    /// else the insert's. None for any other object, or an insert of a
    /// block the drawing does not define.
    pub fn pieces(&self, e: &Entity) -> Option<Vec<Entity>> {
        if !matches!(e, Entity::Insert(_)) {
            return None;
        }
        let item = self.store.get(f64::from(e.base().id))?;
        item.expanded.as_ref().map(|x| piece_entities(e, x))
    }

    /// The most specific visible object within `tol` world units of `at`
    /// (`PickIndex.hit`): points and edges first, then the smallest area
    /// around it; layers without interior picking by their edges only.
    pub fn pick(&self, at: Vec2, tol: f64) -> Option<Slot> {
        if !self.elsewhere.is_empty() {
            return self.hits(at, tol).into_iter().next();
        }
        self.store.hit(at, tol).and_then(slot)
    }

    /// Every visible object a click at `at` could mean, the most specific
    /// first (`Store::hits`, docs/adr/0187 §1): the first is [`Self::pick`]'s.
    /// The active level's objects and those on no level come before the other levels' (docs/adr/0715 §2): a wall
    /// over another one, a door goes in the active level's.
    pub fn hits(&self, at: Vec2, tol: f64) -> Vec<Slot> {
        self.on_level_first(
            self.store
                .hits(at, tol)
                .into_iter()
                .filter_map(slot)
                .collect(),
        )
    }

    /// `slots` with those on another level than the active one after the rest, each part in its order.
    fn on_level_first(&self, slots: Vec<Slot>) -> Vec<Slot> {
        if self.elsewhere.is_empty() {
            return slots;
        }
        let (mut here, there): (Vec<Slot>, Vec<Slot>) = slots
            .into_iter()
            .partition(|s| !self.elsewhere.contains(&s.0));
        here.extend(there);
        here
    }

    /// The nearest visible object whose edge comes within `tol` world units
    /// of `at` and that `accept` takes (`PickIndex.hitEdge`): what the edge
    /// tools act on. Points and text have no edges; among equal distances
    /// the document's order decides.
    pub fn pick_edge(&self, at: Vec2, tol: f64, accept: impl Fn(Slot) -> bool) -> Option<Slot> {
        let taken: Vec<Slot> = self
            .store
            .hit_edge(at, tol)
            .into_iter()
            .filter_map(|(id, _)| slot(id))
            .filter(|s| accept(*s))
            .collect();
        self.on_level_first(taken).into_iter().next()
    }

    /// The smallest visible closed shape around `at` (a polygon, circle,
    /// full ellipse or closed spline; `PickIndex.enclosing`), whatever its
    /// kind: what a click inside an area away from its edges falls in.
    pub fn enclosing(&self, at: Vec2) -> Option<Slot> {
        self.store.enclosing(at).and_then(|(id, _)| slot(id))
    }

    /// The visible objects inside the box from `a` to `b` (window), or
    /// touching it too (crossing), in the document's order (`PickIndex.inRect`).
    pub fn in_rect(&self, a: Vec2, b: Vec2, crossing: bool) -> Vec<Slot> {
        let r = Bounds {
            min_x: a.x.min(b.x),
            min_y: a.y.min(b.y),
            max_x: a.x.max(b.x),
            max_y: a.y.max(b.y),
        };
        self.store
            .in_rect(&r, crossing)
            .into_iter()
            .filter_map(slot)
            .collect()
    }

    /// Kapsam denetimi (docs/adr/0141, `Store::extent_outliers`): the visible
    /// objects lying far from the rest of the drawing, in the document's order.
    pub fn extent_outliers(&self) -> Vec<Slot> {
        self.store
            .extent_outliers()
            .into_iter()
            .filter_map(slot)
            .collect()
    }

    /// The visible closed shapes around `at` with their areas, smallest first
    /// (`Store::containing`, docs/adr/0141): a parcel, then the block and the
    /// district that hold it. Equal areas keep the document's order.
    pub fn containing(&self, at: Vec2) -> Vec<(Slot, f64)> {
        self.store
            .containing(at)
            .into_iter()
            .filter_map(|(id, area)| Some((slot(id)?, area)))
            .collect()
    }

    /// The visible areas with a hole around `at`, the smallest hole first
    /// (`Store::holes_at`, docs/adr/0173 §5), each with the hole's part and place.
    pub fn holes_at(&self, at: Vec2) -> Vec<(Slot, HoleAt)> {
        self.store
            .holes_at(at)
            .into_iter()
            .filter_map(|(id, hole)| Some((slot(id)?, hole)))
            .collect()
    }

    /// The visible objects a fence (an open path) crosses (`Store::in_fence`):
    /// an edge, a text's body, or a point within `tol` world units of it.
    pub fn in_fence(&self, fence: &[Vec2], tol: f64) -> Vec<Slot> {
        self.store
            .in_fence(fence, tol)
            .into_iter()
            .filter_map(slot)
            .collect()
    }

    /// The visible objects wholly inside a circle, and with `crossing` those
    /// it touches too (`Store::in_circle`).
    pub fn in_circle(&self, centre: Vec2, radius: f64, crossing: bool) -> Vec<Slot> {
        self.store
            .in_circle(centre, radius, crossing)
            .into_iter()
            .filter_map(slot)
            .collect()
    }

    /// Çokgenle seç (`Store::in_polygon`, docs/adr/0187 §2): the visible
    /// objects wholly inside a simple ring, touching it too, or not touching
    /// it, in the document's order.
    pub fn in_polygon(&self, ring: &[Vec2], mode: PolygonMode) -> Vec<Slot> {
        self.store
            .in_polygon(ring, mode)
            .into_iter()
            .filter_map(slot)
            .collect()
    }

    /// The object snap near `at` within `tol` world units among `kinds`
    /// (`SnapKind::bit`s); `from` is the running command's last point, for
    /// perpendicular and tangent snaps (`PickIndex.snap`).
    pub fn snap(&self, at: Vec2, tol: f64, kinds: u32, from: Option<Vec2>) -> Option<SnapHit> {
        self.snap_ex(at, tol, kinds, from, &SnapExtras::default())
    }

    /// The object snap with what the running command adds (docs/adr/0163):
    /// acquired extensions and parallels, the object being drawn, Karelaj's
    /// spacings (`PickIndex.snapEx`).
    pub fn snap_ex(
        &self,
        at: Vec2,
        tol: f64,
        kinds: u32,
        from: Option<Vec2>,
        extras: &SnapExtras,
    ) -> Option<SnapHit> {
        if self.sheet_view.is_some() {
            self.snap_candidates(at, tol, kinds, from, extras)
                .into_iter()
                .next()
        } else {
            self.store.snap_ex(at, tol, kinds, from, extras)
        }
    }

    /// Every snap point near `at`, best first (docs/adr/0271): the first is [`Spatial::snap_ex`]'s.
    pub fn snap_candidates(
        &self,
        at: Vec2,
        tol: f64,
        kinds: u32,
        from: Option<Vec2>,
        extras: &SnapExtras,
    ) -> Vec<SnapHit> {
        let mut hits = self.store.snap_candidates(at, tol, kinds, from, extras);
        if let Some(view) = &self.sheet_view {
            hits.retain(|h| view.contains_model([h.point.x, h.point.y]));
        }
        hits
    }

    /// The extensions of object `id`'s edges that end at `at` (Uzantı's
    /// acquisition, docs/adr/0163 §2; the web's `PickIndex.extensionsAt`).
    pub fn extensions_at(&self, id: f64, at: Vec2) -> Vec<Extension> {
        self.store.extensions_at(id, at)
    }

    /// The direction of the straight edge nearest `p` within `tol`, unit
    /// (Paralel's acquisition; the web's `PickIndex.directionAt`).
    pub fn direction_at(&self, p: Vec2, tol: f64) -> Option<Vec2> {
        self.store.direction_at(p, tol)
    }

    /// The grips of these objects, unknown ones left out, in the given order
    /// (`PickIndex.grips`, the store's `entityGrips`): the order `move_grip`
    /// reads an index in.
    pub fn grips(&self, slots: &[Slot]) -> Vec<GripSet> {
        let ids: Vec<f64> = slots.iter().map(|s| f64::from(s.0)).collect();
        let f = self.store.grips(&ids);
        let mut out = Vec::new();
        let mut i = 0;
        while i + 2 < f.len() {
            let (id, count, vertices) = (f[i], f[i + 1] as usize, f[i + 2] as usize);
            i += 3;
            let records = f.get(i..i + 3 * count).unwrap_or_default();
            i += 3 * count;
            let Some(slot) = slot(id) else { continue };
            out.push(GripSet::new(
                slot,
                records
                    .chunks_exact(3)
                    .map(|r| Vec2::new(r[0], r[1]))
                    .collect(),
                records
                    .chunks_exact(3)
                    .map(|r| (r[2] >= 0.0).then_some(r[2] as usize))
                    .collect(),
                vertices,
            ));
        }
        out
    }

    /// The parameter grips of the selected dynamic blocks' inserts, on the drawing (docs/adr/0319 §1): each with its
    /// object. None past the grips' limit, as the other grips.
    pub fn param_grips(
        &self,
        slots: &[Slot],
    ) -> Vec<(Slot, kentos_geometry_core::block_dynamic::ParamGrip)> {
        let mut out = Vec::new();
        for &slot in slots {
            for g in self.store.param_grips(f64::from(slot.0)) {
                out.push((slot, g));
            }
        }
        out
    }

    /// The value insert `slot`'s parameter `name` takes with its grip at `at`, its rule kept (docs/adr/0319 §1).
    pub fn param_value(
        &self,
        slot: Slot,
        name: &str,
        at: Vec2,
    ) -> Option<kentos_geometry_core::entity::ParamValue> {
        self.store.param_value(f64::from(slot.0), name, at)
    }

    /// Insert `slot` as `values` would draw it: its pieces' outlines (`outline_paths`'s layout).
    pub fn varied_outlines(
        &self,
        slot: Slot,
        values: &kentos_geometry_core::entity::Params,
    ) -> Vec<f64> {
        self.store.varied_outlines(f64::from(slot.0), values)
    }

    /// The box around every object, as zoom to extents takes it (`PickIndex.extent`).
    pub fn extent(&self) -> Option<Bounds> {
        self.store.extent(None)
    }

    /// What a view from `min` to `max` at `scale` pixels per unit draws as
    /// text, in the document's order (`Store::labels`, the web's
    /// `drawLabels`): dimension values, text objects and object labels on
    /// visible layers, near the view, readable at this size and inside
    /// their label style's scale range.
    ///
    /// The objects' labels are the label engine's (docs/adr/0212 §3.8): a
    /// frame, then its lines, letters and callout.
    pub fn labels(
        &self,
        min: Vec2,
        max: Vec2,
        scale: f64,
        options: PlaceOptions,
    ) -> Vec<LabelSpot> {
        let view = Bounds {
            min_x: min.x,
            min_y: min.y,
            max_x: max.x,
            max_y: max.y,
        };
        let shown = self.store.labels(&view, scale, None, options);
        shown
            .records
            .chunks_exact(LABEL_STRIDE)
            .filter_map(|r| self.spot(r, &shown.texts))
            .collect()
    }

    /// Exact paper labels; a viewport's layer mask applies before placing or reserving obstacles.
    pub fn labels_print(
        &self,
        min: Vec2,
        max: Vec2,
        scale: f64,
        options: PlaceOptions,
        shown: &dyn Fn(&str) -> bool,
    ) -> Vec<LabelSpot> {
        let view = Bounds {
            min_x: min.x,
            min_y: min.y,
            max_x: max.x,
            max_y: max.y,
        };
        let labels = self.store.labels_print(&view, scale, options, Some(shown));
        labels
            .records
            .chunks_exact(LABEL_STRIDE)
            .filter_map(|r| self.spot(r, &labels.texts))
            .collect()
    }

    /// The labels as the view shows them under `size` (docs/adr/0205 §5):
    /// each with how it grows (its factor, 1 as it is, and the point it grows
    /// about); a grown text that would cover another is left out. `keep`:
    /// the main view's, kept for `label_at`.
    pub fn labels_shown(
        &self,
        min: Vec2,
        max: Vec2,
        scale: f64,
        size: LabelSize,
        options: PlaceOptions,
        keep: bool,
    ) -> Vec<(LabelSpot, Grow)> {
        self.labels_shown_in(min, max, scale, size, options, keep, None)
    }

    /// [`Spatial::labels_shown`] for a view with a theme (docs/adr/0523 §2): the labels of the layers `tree`
    /// shows, whatever the active view shows; none, the store's.
    #[allow(clippy::too_many_arguments)]
    pub fn labels_shown_in(
        &self,
        min: Vec2,
        max: Vec2,
        scale: f64,
        size: LabelSize,
        options: PlaceOptions,
        keep: bool,
        tree: Option<&LayerTree>,
    ) -> Vec<(LabelSpot, Grow)> {
        let view = Bounds {
            min_x: min.x,
            min_y: min.y,
            max_x: max.x,
            max_y: max.y,
        };
        // The main view (`keep`) places where its labels last were when that is free: a pan of a few pixels moves
        // the labels with the ground (docs/adr/0533 §3).
        let before = keep.then(|| self.prior.borrow().clone());
        let (shown, next) = match tree {
            Some(tree) => {
                let visible = |id: &str| tree.is_visible(id);
                self.store.labels_shown_after(
                    &view,
                    scale,
                    None,
                    size,
                    options,
                    Some(&visible),
                    before.as_ref(),
                )
            }
            None => self
                .store
                .labels_shown_after(&view, scale, None, size, options, None, before.as_ref()),
        };
        if keep {
            *self.prior.borrow_mut() = next;
        }
        let out = shown
            .records
            .chunks_exact(LABEL_SHOWN_STRIDE)
            .filter_map(|r| {
                let grow = Grow {
                    k: r[LABEL_STRIDE],
                    anchor: Vec2::new(r[LABEL_STRIDE + 1], r[LABEL_STRIDE + 2]),
                };
                self.spot(&r[..LABEL_STRIDE], &shown.texts)
                    .map(|spot| (spot, grow))
            })
            .collect();
        if keep {
            *self.kept.borrow_mut() = (shown.records, LABEL_SHOWN_STRIDE, scale);
        }
        out
    }

    /// One label record, typed.
    fn spot(&self, r: &[f64], texts: &[String]) -> Option<LabelSpot> {
        {
            {
                let slot = slot(r[0])?;
                let at = Vec2::new(r[2], r[3]);
                let what = r[1];
                Some(if what == LABEL_PLACED {
                    LabelSpot::Placed {
                        slot,
                        at,
                        angle: r[4],
                        width: r[5],
                        height: r[6],
                        class: r[7] as u16,
                        state: r[8] as u32,
                    }
                } else if what == LABEL_PLACED_LINE {
                    LabelSpot::PlacedLine {
                        slot,
                        at,
                        angle: r[4],
                        size: r[5],
                        text: texts.get(r[6] as usize).cloned().unwrap_or_default(),
                    }
                } else if what == LABEL_PLACED_LETTER {
                    LabelSpot::PlacedLetter {
                        slot,
                        at,
                        angle: r[4],
                        size: r[5],
                        letter: texts
                            .get(r[6] as usize)
                            .and_then(|t| t.chars().nth(r[7] as usize))
                            .unwrap_or(' '),
                        advance: r[8],
                    }
                } else if what == LABEL_PLACED_CALLOUT {
                    LabelSpot::PlacedCallout {
                        slot,
                        from: at,
                        to: Vec2::new(r[4], r[5]),
                    }
                } else if what == LABEL_DIMENSION {
                    LabelSpot::Dimension {
                        slot,
                        at,
                        angle: r[4],
                        value: r[5],
                        unit: DIMENSION_UNITS
                            .get(r[6] as usize)
                            .copied()
                            .unwrap_or("length"),
                        prefix: DIMENSION_PREFIXES.get(r[7] as usize).copied().unwrap_or(""),
                        mask: r[8] == 1.0,
                    }
                } else if what == LABEL_TEXT || what == LABEL_LEADER {
                    LabelSpot::Text {
                        slot,
                        at,
                        rotation: r[4],
                        width_factor: r[5],
                        mask: r[6],
                    }
                } else if what == LABEL_PIECE_TEXT || what == LABEL_PIECE_LEADER {
                    let piece = self.piece(r[0], r[6])?;
                    let (text, attribute, face) = match piece.shape {
                        Shape::Text { text, face, .. } => (text, piece.attribute, face),
                        // A leader's note among a block's pieces (docs/adr/0146 §5).
                        Shape::Leader {
                            text: Some(text), ..
                        } => (text, None, Default::default()),
                        _ => return None,
                    };
                    LabelSpot::PieceText {
                        slot,
                        at,
                        rotation: r[4],
                        height: r[5],
                        text,
                        attribute,
                        width_factor: r[7],
                        mask: r[8],
                        face,
                    }
                } else if what == LABEL_LINE {
                    LabelSpot::Line {
                        slot,
                        at,
                        rotation: r[4],
                        height: r[5],
                        width_factor: r[6],
                        start: r[7] as usize,
                        end: r[8] as usize,
                        piece: None,
                        face: None,
                    }
                } else if what == LABEL_PIECE_LINE {
                    let Shape::Text {
                        text,
                        width_factor,
                        runs,
                        face,
                        ..
                    } = self.piece(r[0], r[6])?.shape
                    else {
                        return None;
                    };
                    LabelSpot::Line {
                        slot,
                        at,
                        rotation: r[4],
                        height: r[5],
                        width_factor: width_factor.unwrap_or(1.0),
                        start: r[7] as usize,
                        end: r[8] as usize,
                        piece: Some(std::rc::Rc::new((text, runs.unwrap_or_default()))),
                        face: Some(face),
                    }
                } else if what == LABEL_PARAGRAPH_MASK {
                    // Its text's slant: the object's own, or a block's piece's (its place, d).
                    let lean = match self.store.get(r[0]).map(|it| &it.shape) {
                        Some(Shape::Text { face, .. }) => face.lean(),
                        Some(Shape::Insert { .. }) => match self.piece(r[0], r[7]).map(|p| p.shape)
                        {
                            Some(Shape::Text { face, .. }) => face.lean(),
                            _ => 0.0,
                        },
                        _ => 0.0,
                    };
                    LabelSpot::ParagraphMask {
                        slot,
                        at,
                        rotation: r[4],
                        width: r[5],
                        height: r[6],
                        lean,
                    }
                } else if what == LABEL_CELL {
                    LabelSpot::Cell {
                        slot,
                        at,
                        rotation: r[4],
                        row: r[5] as usize,
                        col: r[6] as usize,
                        bold: r[7] == 1.0,
                    }
                } else if what == LABEL_PIECE_DIMENSION {
                    let Shape::Dimension {
                        text,
                        style,
                        angle,
                        look,
                        ..
                    } = self.piece(r[0], r[6])?.shape
                    else {
                        return None;
                    };
                    let (unit, prefix) = dimension_measure(style.as_deref(), angle);
                    LabelSpot::PieceDimension {
                        slot,
                        at,
                        angle: r[4],
                        value: r[5],
                        height: r[7],
                        text: text.filter(|t| !t.is_empty()),
                        unit,
                        prefix,
                        mask: r[8] == 1.0,
                        look,
                    }
                } else {
                    return None;
                })
            }
        }
    }

    /// Piece `place` of the insert `id`'s block, as its definition holds it.
    fn piece(&self, id: f64, place: f64) -> Option<kentos_geometry_core::block::Piece> {
        let it = self.store.get(id)?;
        if let Shape::Insert { block, .. } = &it.shape {
            let flat = self.store.blocks().get(block)?;
            return flat.pieces.get(place as usize).cloned();
        }
        // Another object drawn by its pieces (a space's label, docs/adr/0342 §2): the piece as the store placed it.
        let shape = it.expanded.as_deref()?.shapes.get(place as usize)?.clone();
        Some(kentos_geometry_core::block::Piece {
            shape,
            color: None,
            line_weight: None,
            by_layer: Vec::new(),
            gates: Vec::new(),
            stroke: Default::default(),
            attribute: None,
        })
    }

    /// How many times every object was read again: once per opened drawing
    /// while the journal keeps up (tests hold the store to that).
    pub fn reloads(&self) -> u64 {
        self.reloads
    }

    /// How many objects the store holds.
    pub fn len(&self) -> usize {
        self.store.len()
    }

    pub fn is_empty(&self) -> bool {
        self.store.is_empty()
    }
}

/// How a label grows in a view (docs/adr/0205 §5): `k` times its own size
/// about `anchor` (world), 1 as it is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Grow {
    pub k: f64,
    pub anchor: Vec2,
}

impl Grow {
    /// As it is.
    pub const NONE: Grow = Grow {
        k: 1.0,
        anchor: Vec2 { x: 0.0, y: 0.0 },
    };
}

/// One thing a view draws as text (the store's label records, typed).
/// Points are in world units; angles in degrees, counter-clockwise.
#[derive(Clone, Debug, PartialEq)]
pub enum LabelSpot {
    /// A dimension's value at its place, turned by `angle`; its `unit`
    /// ("length", "angle", "percent" or "coordinate") and `prefix` ("R ",
    /// "Ø ", "Y=", "X=", "t=", "%" or none), as the core's layout says; `mask`
    /// fills its measured box first (docs/adr/0147).
    Dimension {
        slot: Slot,
        at: Vec2,
        angle: f64,
        value: f64,
        unit: &'static str,
        prefix: &'static str,
        mask: bool,
    },
    /// A text object from where its baseline starts (its point moved by its
    /// alignment, docs/adr/0145), turned by `rotation`, its letters
    /// `width_factor` wide; `mask` the width of the box filled under it
    /// (`TextPlace::mask`), 0 without a mask. A leader's note too (its
    /// object's `text` and `height`, docs/adr/0146 §5).
    Text {
        slot: Slot,
        at: Vec2,
        rotation: f64,
        width_factor: f64,
        mask: f64,
    },
    /// An object's label as the label engine placed it (docs/adr/0212
    /// §3.8): its block's middle, its turn (degrees), its width and height
    /// (px), its class and its state (`PINNED` …); its lines, letters and
    /// callout follow.
    Placed {
        slot: Slot,
        at: Vec2,
        angle: f64,
        width: f64,
        height: f64,
        class: u16,
        state: u32,
    },
    /// A placed label's line: its middle, turn (degrees), size (px) and words.
    PlacedLine {
        slot: Slot,
        at: Vec2,
        angle: f64,
        size: f64,
        text: String,
    },
    /// A curved label's letter: its middle, turn (degrees), size and advance (px).
    PlacedLetter {
        slot: Slot,
        at: Vec2,
        angle: f64,
        size: f64,
        letter: char,
        advance: f64,
    },
    /// A placed label's callout: from by the label to the object.
    PlacedCallout { slot: Slot, from: Vec2, to: Vec2 },
    /// A text among a block's pieces (docs/adr/0144), or a leader's note
    /// (docs/adr/0146): from where its placed baseline starts, turned by
    /// `rotation`, `height` as placed, its width factor and mask as a text's.
    PieceText {
        slot: Slot,
        at: Vec2,
        rotation: f64,
        height: f64,
        text: String,
        /// An attribute's tag (docs/adr/0144 §7): the insert's value under it
        /// is shown, else `text` (the default); `shown_text` says which.
        attribute: Option<String>,
        width_factor: f64,
        mask: f64,
        /// The piece's face (docs/adr/0183 §2); a leader's note has none.
        face: kentos_geometry_core::text::face::Face,
    },
    /// One line of a multi-line text (docs/adr/0182 §3): its letters
    /// `start..end` (Unicode scalar values) from where its baseline starts,
    /// turned by `rotation`, `height` high, its letters `width_factor` wide;
    /// its words and runs its object's, or a block's piece's (`piece`).
    Line {
        slot: Slot,
        at: Vec2,
        rotation: f64,
        height: f64,
        width_factor: f64,
        start: usize,
        end: usize,
        piece: Option<std::rc::Rc<(String, Vec<kentos_geometry_core::text::paragraph::Run>)>>,
        /// A block's piece's face (docs/adr/0183 §2); a text's own is its object's.
        face: Option<kentos_geometry_core::text::face::Face>,
    },
    /// A multi-line text's mask (docs/adr/0182 §3): the box from `at` (its
    /// corner under the first letter's left) `width` along its baseline and
    /// `height` up, turned by `rotation`, filled with the area's colour.
    ParagraphMask {
        slot: Slot,
        at: Vec2,
        rotation: f64,
        width: f64,
        height: f64,
        /// The slant's tangent of the text it is under: the box leans from
        /// its corner (docs/adr/0183 §2); 0 upright.
        lean: f64,
    },
    /// One cell's words of a table (docs/adr/0184 §2): from where their
    /// baseline starts, turned by `rotation`; the words, height and face its
    /// object's, bold for a heading row's.
    Cell {
        slot: Slot,
        at: Vec2,
        rotation: f64,
        row: usize,
        col: usize,
        bold: bool,
    },
    /// A dimension's value among a block's pieces, as `Dimension`, its own
    /// text when it has one and `height` as placed.
    PieceDimension {
        slot: Slot,
        at: Vec2,
        angle: f64,
        value: f64,
        height: f64,
        text: Option<String>,
        unit: &'static str,
        prefix: &'static str,
        mask: bool,
        /// The piece's look: its value's typeface and writing (docs/adr/0183 §3).
        look: kentos_geometry_core::geom::dimension::Look,
    },
}

/// An object's characteristic vertices, as the shared core gives them for
/// grips, snapping and the coordinate list (`entity_vertices`); a polygon's
/// holes follow its outer ring.
pub fn vertices(entity: &Entity) -> Vec<Vec2> {
    entity_vertices(&shape(entity))
}

/// An object's area and length as the shared core measures them
/// (`entity_area`, `entity_length`): arcs of bulged edges followed, holes
/// taken out of the area and counted in the perimeter.
pub fn measures(entity: &Entity) -> (Option<f64>, Option<f64>) {
    let s = shape(entity);
    (entity_area(&s), entity_length(&s))
}

/// The area a closed polyline encloses, an area's own (`enclosed_area`): what
/// Öznitelikler and the card show, while the line has no area of its own (docs/adr/0258 §1).
pub fn enclosed(entity: &Entity) -> Option<f64> {
    kentos_geometry_core::entity::enclosed_area(&shape(entity))
}

/// An arc's sweep from `a0` to `a1`, counter-clockwise, radians (the core's
/// `sweep`, the properties panel's Yay açısı).
pub fn arc_sweep(a0: f64, a1: f64) -> f64 {
    kentos_geometry_core::geom::arc::sweep(a0, a1)
}

/// Whether an ellipse object is whole rather than an elliptical arc (the
/// core's `is_full_ellipse`).
pub fn full_ellipse(e: &kentos_commands::EllipseEntity) -> bool {
    let v = |p: kentos_commands::Vec2| Vec2::new(p.x, p.y);
    kentos_geometry_core::geom::ellipse::is_full_ellipse(
        &kentos_geometry_core::entity::ellipse_geom(v(e.c), v(e.major), e.ratio, e.t0, e.t1),
    )
}

/// A dimension's layout as the shared core lays it out (`layout_dimension`):
/// where its value is written, turned how, and the value; none for another
/// object or a dimension that cannot be laid out.
pub fn dimension_layout(
    entity: &Entity,
) -> Option<kentos_geometry_core::geom::dimension::DimensionLayout> {
    kentos_geometry_core::entity::dimension_geom(&shape(entity))
        .and_then(|d| kentos_geometry_core::geom::dimension::layout_dimension(&d))
}

/// A store id back to the document's slot (ids are the slots, exactly).
/// An object's grips as the store lists them (the web's `GripSet`,
/// `viewport/storeRecords.ts`).
#[derive(Clone, Debug, PartialEq)]
pub struct GripSet {
    pub slot: Slot,
    /// In the order `move_grip` reads an index in: a path's vertices, then a
    /// mid grip per segment, then its holes' vertices.
    pub points: Vec<Vec2>,
    /// The segment a mid grip splits or bends; none for the other grips.
    pub segments: Vec<Option<usize>>,
    /// A path's vertex count (a multi-part area's first part's); 0 for the other kinds.
    pub vertices: usize,
    /// For each mid grip, its ring's first vertex (an index into `points`)
    /// and vertex count; none for the other grips. A multi-part area's later
    /// parts each have a ring of their own (docs/adr/0143).
    pub rings: Vec<Option<(usize, usize)>>,
}

impl GripSet {
    /// The grips as the store lists them, each mid grip's ring found in one
    /// pass: a ring's mid grips run from its segment 0 right after its
    /// vertices; an open path's run is one shorter than its vertices.
    pub fn new(
        slot: Slot,
        points: Vec<Vec2>,
        segments: Vec<Option<usize>>,
        vertices: usize,
    ) -> Self {
        let mut rings = vec![None; segments.len()];
        let mut k = 0;
        while k < segments.len() {
            if segments[k] != Some(0) {
                k += 1;
                continue;
            }
            let run = segments[k..]
                .iter()
                .enumerate()
                .take_while(|(j, s)| **s == Some(*j))
                .count();
            let count = if k == vertices && run + 1 == vertices {
                vertices
            } else {
                run
            };
            if let Some(first) = k.checked_sub(count) {
                rings[k..k + run].fill(Some((first, count)));
            }
            k += run;
        }
        GripSet {
            slot,
            points,
            segments,
            vertices,
            rings,
        }
    }

    /// Whether grip `index` is shown and can be taken: a mid grip only while
    /// its segment is at least 28 px long on the area, to tell it from the
    /// vertices (the web's `midGripVisible`).
    pub fn shown(&self, index: usize, view: &dyn crate::tool::View) -> bool {
        let (Some(Some(segment)), Some(Some((first, count)))) =
            (self.segments.get(index), self.rings.get(index))
        else {
            return true;
        };
        let (Some(&a), Some(&b)) = (
            self.points.get(first + segment),
            self.points.get(first + (segment + 1) % (*count).max(1)),
        ) else {
            return true;
        };
        let (a, b) = (view.to_screen(a), view.to_screen(b));
        kentos_geometry_core::jsmath::js_hypot(b[0] - a[0], b[1] - a[1]) >= 28.0
    }
}

pub(crate) fn slot(id: f64) -> Option<Slot> {
    (id >= 0.0 && id <= f64::from(u32::MAX) && id.fract() == 0.0).then_some(Slot(id as u32))
}

/// An object as the store takes it: its id (the slot), layer, whether it
/// has a label, and its geometry. The one conversion of the desktop's
/// objects into the store's shapes (`kentos_native_application::geometry`,
/// which the transform command shares); `fixtures/store-records/v1` holds
/// it to what the web packs for the same object.
pub fn record(entity: &Entity) -> (f64, &str, bool, Shape) {
    let base = entity.base();
    let label = base.label.as_deref().is_some_and(|l| !l.is_empty());
    (
        f64::from(base.id),
        base.layer_id.as_str(),
        label,
        shape(entity),
    )
}

/// Every node of the layer tree with its flags resolved through the groups
/// above it, in tree order: the web's `layerTable`.
pub fn layer_rows(tree: &LayerTree) -> Vec<(String, LayerFlags)> {
    fn walk(nodes: &[LayerNode], tree: &LayerTree, out: &mut Vec<(String, LayerFlags)>) {
        for node in nodes {
            out.push((
                node.id.clone(),
                LayerFlags {
                    visible: tree.is_visible(&node.id),
                    locked: tree.is_locked(&node.id),
                    pick_interior: node.style.pick_interior != Some(false),
                    snap: node.snap.as_ref().map_or(u32::MAX, layer_snap_mask),
                },
            ));
            walk(&node.children, tree, out);
        }
    }
    let mut out = Vec::new();
    walk(tree.nodes(), tree, &mut out);
    out
}

/// A layer's own snapping as the store's kinds (docs/adr/0163 §4): none
/// when off; Uç nokta brings Çeyrek with it, as the settings do (the web's
/// `layerSnapMask`), and Nokta brings Ekleme noktası (docs/adr/0269 §1: a layer's kinds keep their names).
pub fn layer_snap_mask(snap: &LayerSnap) -> u32 {
    kentos_geometry_core::store::snap::layer_mask(snap.off,snap.kinds.as_deref().unwrap_or_default())
}

/// The kinds with a label style of their own, in the store's order (`set_label_defaults`).
pub const LABELLED_KINDS: [&str; 5] = ["polygon", "circle", "point", "polyline", "line"];

/// The label style of an object whose layer has none: the web's
/// `DEFAULT_LABELS`, in the contract (`kentos_commands::default_label`) so
/// the document's linked texts follow the same (docs/adr/0175 §4).
pub use kentos_commands::default_label;

/// What a block's text piece shows on an insert with these attributes
/// (docs/adr/0144 §7; the web's `pieceText`): an attribute's piece the
/// insert's value under its tag, else its text (the default); any other its
/// text. Empty: nothing is drawn.
pub fn shown_text<'a>(
    text: &'a str,
    attribute: Option<&str>,
    attrs: &'a std::collections::BTreeMap<String, String>,
) -> &'a str {
    attribute
        .and_then(|tag| attrs.get(tag))
        .map(String::as_str)
        .filter(|v| !v.is_empty())
        .unwrap_or(text)
}

#[cfg(test)]
mod attribute_tests {
    use super::shown_text;
    use std::collections::BTreeMap;

    #[test]
    fn an_attribute_shows_the_inserts_value_else_its_default() {
        let attrs = BTreeMap::from([
            ("NO".to_owned(), "R-12".to_owned()),
            ("KOT".to_owned(), String::new()),
        ]);
        assert_eq!(shown_text("R-1", Some("NO"), &attrs), "R-12");
        // An empty value is no value: the default shows.
        assert_eq!(shown_text("?", Some("KOT"), &attrs), "?");
        assert_eq!(shown_text("", Some("KOT"), &attrs), "");
        // A plain text of the block is itself.
        assert_eq!(shown_text("Rögar", None, &attrs), "Rögar");
    }
}
