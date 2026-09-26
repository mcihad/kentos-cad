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
//! Queries answer by slot. What they decide is the store's, rule for rule
//! the web's: points and edges before interiors, the smallest area, window
//! and crossing boxes, the snap kinds' weights.

use std::collections::HashSet;

use kentos_contracts::{DrawingFont, Entity, LabelPlacement, LabelStyle, LayerNode};
use kentos_domain::{ChangeMark, Changes, Document, LayerTree, Slot};
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geometry::Bounds;
use kentos_geometry_core::store::labels::{LabelRule, Placement};
use kentos_geometry_core::store::snap::SnapHit;
use kentos_geometry_core::store::{LayerFlags, Store};
use kentos_geometry_core::text::Font;
use kentos_native_application::geometry::shape;

use crate::Vec2;

/// The geometry store and where it stands in the document's changes.
#[derive(Default)]
pub struct Spatial {
    store: Store,
    /// The document's changes the store has taken.
    mark: ChangeMark,
    /// The document's revision at the last sync: nothing to do while it holds.
    revision: Option<u64>,
    /// The layer table sent last.
    layers: Vec<(String, LayerFlags)>,
    /// How many times every object was read (a drawing opened, or the journal fell behind).
    reloads: u64,
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

    /// Reads every object of `doc` again: a drawing was opened, or the
    /// document's journal no longer reaches back to the store's mark.
    pub fn reload(&mut self, doc: &Document) {
        self.reloads += 1;
        self.store.clear();
        self.store
            .set_font(Font::from_id(font_id(doc.settings().drawing_font)));
        self.store.put_many(doc.entities().map(record));
        self.mark = doc.change_mark();
        self.revision = Some(doc.revision());
        self.layers.clear();
        self.sync_layers(doc.layers());
    }

    /// Brings the store up to date with `doc`: the objects touched since the
    /// last sync and the layer table, when it changed. Nothing is read while
    /// the document's revision and journal are where they were.
    pub fn sync(&mut self, doc: &Document) {
        let mark = doc.change_mark();
        if mark == self.mark && self.revision == Some(doc.revision()) {
            return;
        }
        self.store
            .set_font(Font::from_id(font_id(doc.settings().drawing_font)));
        match doc.changes_since(self.mark) {
            Changes::All => return self.reload(doc),
            Changes::Slots(slots) => {
                let mut seen = HashSet::with_capacity(slots.len());
                let mut gone = Vec::new();
                let mut changed = Vec::new();
                for &slot in slots {
                    if !seen.insert(slot) {
                        continue;
                    }
                    match doc.get(slot) {
                        Some(entity) => changed.push(entity),
                        None => gone.push(f64::from(slot.0)),
                    }
                }
                self.store.remove(&gone);
                self.store.put_many(changed.into_iter().map(record));
            }
        }
        self.mark = mark;
        self.revision = Some(doc.revision());
        self.sync_layers(doc.layers());
    }

    /// Sends the layer table when it differs from the one sent last.
    fn sync_layers(&mut self, tree: &LayerTree) {
        let rows = layer_rows(tree);
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

    /// The most specific visible object within `tol` world units of `at`
    /// (`PickIndex.hit`): points and edges first, then the smallest area
    /// around it; layers without interior picking by their edges only.
    pub fn pick(&self, at: Vec2, tol: f64) -> Option<Slot> {
        self.store.hit(at, tol).and_then(slot)
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

    /// The object snap near `at` within `tol` world units among `kinds`
    /// (`SnapKind::bit`s); `from` is the running command's last point, for
    /// perpendicular and tangent snaps (`PickIndex.snap`).
    pub fn snap(&self, at: Vec2, tol: f64, kinds: u32, from: Option<Vec2>) -> Option<SnapHit> {
        self.store.snap(at, tol, kinds, from)
    }

    /// The box around every object, as zoom to extents takes it (`PickIndex.extent`).
    pub fn extent(&self) -> Option<Bounds> {
        self.store.extent(None)
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

/// A store id back to the document's slot (ids are the slots, exactly).
fn slot(id: f64) -> Option<Slot> {
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

/// The drawing typeface by its id, as the store measures text in it.
fn font_id(font: Option<DrawingFont>) -> &'static str {
    match font {
        None | Some(DrawingFont::Barlow) => "barlow",
        Some(DrawingFont::Arimo) => "arimo",
        Some(DrawingFont::Overpass) => "overpass",
        Some(DrawingFont::Quicksand) => "quicksand",
        Some(DrawingFont::ArchitectsDaughter) => "architects-daughter",
        Some(DrawingFont::CourierPrime) => "courier-prime",
        Some(DrawingFont::PlexMono) => "plex-mono",
    }
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
                    label: node.style.label.as_ref().map(label_rule),
                },
            ));
            walk(&node.children, tree, out);
        }
    }
    let mut out = Vec::new();
    walk(tree.nodes(), tree, &mut out);
    out
}

/// What of a label style decides whether and where a label is drawn (the web's `labelRule`).
fn label_rule(style: &LabelStyle) -> LabelRule {
    LabelRule {
        placement: match style.placement {
            LabelPlacement::Center => Placement::Center,
            LabelPlacement::Corner => Placement::Corner,
            LabelPlacement::Beside => Placement::Beside,
            LabelPlacement::Along => Placement::Along,
        },
        min_scale: style.min_scale,
        max_scale: style.max_scale,
        min_feature_px: style.min_feature_px,
    }
}
