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

use std::sync::Arc;

use kentos_contracts::{BlockDefinition, Entity, LabelPlacement, LabelStyle, LayerNode};
use kentos_domain::{ChangeMark, Changes, Document, LayerTree, Slot};
use kentos_geometry_core::entity::{Shape, entity_area, entity_length, entity_vertices};
use kentos_geometry_core::geometry::Bounds;
use kentos_geometry_core::store::labels::{
    LABEL_ALONG, LABEL_BESIDE, LABEL_CENTER, LABEL_CORNER, LABEL_DIMENSION, LABEL_PIECE_DIMENSION,
    LABEL_PIECE_TEXT, LABEL_STRIDE, LABEL_TEXT, LabelRule, Placement,
};
use kentos_geometry_core::store::snap::SnapHit;
use kentos_geometry_core::store::{LayerFlags, Store};
use kentos_native_application::blocks::{core_blocks, piece_entities};
use kentos_native_application::geometry::{drawing_font, shape};

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
    /// The block definitions sent last.
    blocks: Vec<Arc<BlockDefinition>>,
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
        self.store.set_label_defaults(
            LABELLED_KINDS.map(|kind| default_label(kind).as_ref().map(label_rule)),
        );
        self.store
            .set_font(drawing_font(doc.settings().drawing_font));
        self.blocks = doc.blocks().to_vec();
        self.store.set_blocks(core_blocks(&self.blocks));
        self.store.put_many(doc.entities().map(record));
        self.mark = doc.change_mark();
        self.revision = Some(doc.revision());
        self.layers.clear();
        self.sync_layers(doc.layers());
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
            || blocks.iter().zip(&self.blocks).any(|(a, b)| !Arc::ptr_eq(a, b))
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
        match doc.changes_since(self.mark) {
            Changes::All => return self.reload(doc),
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
        self.store.hit(at, tol).and_then(slot)
    }

    /// The nearest visible object whose edge comes within `tol` world units
    /// of `at` and that `accept` takes (`PickIndex.hitEdge`): what the edge
    /// tools act on. Points and text have no edges; among equal distances
    /// the document's order decides.
    pub fn pick_edge(&self, at: Vec2, tol: f64, accept: impl Fn(Slot) -> bool) -> Option<Slot> {
        self.store
            .hit_edge(at, tol)
            .into_iter()
            .filter_map(|(id, _)| slot(id))
            .find(|s| accept(*s))
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

    /// The object snap near `at` within `tol` world units among `kinds`
    /// (`SnapKind::bit`s); `from` is the running command's last point, for
    /// perpendicular and tangent snaps (`PickIndex.snap`).
    pub fn snap(&self, at: Vec2, tol: f64, kinds: u32, from: Option<Vec2>) -> Option<SnapHit> {
        self.store.snap(at, tol, kinds, from)
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

    /// The box around every object, as zoom to extents takes it (`PickIndex.extent`).
    pub fn extent(&self) -> Option<Bounds> {
        self.store.extent(None)
    }

    /// What a view from `min` to `max` at `scale` pixels per unit draws as
    /// text, in the document's order (`Store::labels`, the web's
    /// `drawLabels`): dimension values, text objects and object labels on
    /// visible layers, near the view, readable at this size and inside
    /// their label style's scale range.
    pub fn labels(&self, min: Vec2, max: Vec2, scale: f64) -> Vec<LabelSpot> {
        let view = Bounds {
            min_x: min.x,
            min_y: min.y,
            max_x: max.x,
            max_y: max.y,
        };
        self.store
            .labels(&view, scale, None)
            .chunks_exact(LABEL_STRIDE)
            .filter_map(|r| {
                let slot = slot(r[0])?;
                let at = Vec2::new(r[2], r[3]);
                let what = r[1];
                Some(if what == LABEL_DIMENSION {
                    LabelSpot::Dimension {
                        slot,
                        at,
                        angle: r[4],
                        value: r[5],
                        angular: r[6] == 1.0,
                        prefix: match r[7] as u8 {
                            1 => "R ",
                            2 => "Ø ",
                            _ => "",
                        },
                    }
                } else if what == LABEL_TEXT {
                    LabelSpot::Text {
                        slot,
                        at,
                        rotation: r[4],
                    }
                } else if what == LABEL_CENTER {
                    LabelSpot::Center { slot, at }
                } else if what == LABEL_CORNER {
                    LabelSpot::Corner { slot, at }
                } else if what == LABEL_BESIDE {
                    LabelSpot::Beside { slot, at }
                } else if what == LABEL_ALONG {
                    LabelSpot::Along {
                        slot,
                        a: at,
                        b: Vec2::new(r[4], r[5]),
                    }
                } else if what == LABEL_PIECE_TEXT {
                    let Shape::Text { text, .. } = self.piece(r[0], r[6])? else {
                        return None;
                    };
                    LabelSpot::PieceText {
                        slot,
                        at,
                        rotation: r[4],
                        height: r[5],
                        text,
                    }
                } else if what == LABEL_PIECE_DIMENSION {
                    let Shape::Dimension { text, style, .. } = self.piece(r[0], r[6])? else {
                        return None;
                    };
                    LabelSpot::PieceDimension {
                        slot,
                        at,
                        angle: r[4],
                        value: r[5],
                        height: r[7],
                        text: text.filter(|t| !t.is_empty()),
                        angular: style.as_deref() == Some("angular"),
                        prefix: match style.as_deref() {
                            Some("radius") => "R ",
                            Some("diameter") => "Ø ",
                            _ => "",
                        },
                    }
                } else {
                    return None;
                })
            })
            .collect()
    }

    /// Piece `place` of the insert `id`'s block, as its definition holds it.
    fn piece(&self, id: f64, place: f64) -> Option<Shape> {
        let Shape::Insert { block, .. } = &self.store.get(id)?.shape else {
            return None;
        };
        let flat = self.store.blocks().get(block)?;
        flat.pieces.get(place as usize).map(|p| p.shape.clone())
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

/// One thing a view draws as text (the store's label records, typed).
/// Points are in world units; angles in degrees, counter-clockwise.
#[derive(Clone, Debug, PartialEq)]
pub enum LabelSpot {
    /// A dimension's value at its place, turned by `angle`; `angular`: an
    /// angle (else a length); `prefix` "R " or "Ø " for a radius or diameter.
    Dimension {
        slot: Slot,
        at: Vec2,
        angle: f64,
        value: f64,
        angular: bool,
        prefix: &'static str,
    },
    /// A text object at its insertion point, turned by `rotation`.
    Text { slot: Slot, at: Vec2, rotation: f64 },
    /// A label centred on its anchor.
    Center { slot: Slot, at: Vec2 },
    /// A label at the top left of the object's box.
    Corner { slot: Slot, at: Vec2 },
    /// A label beside a point.
    Beside { slot: Slot, at: Vec2 },
    /// A label along the edge from `a` to `b`.
    Along { slot: Slot, a: Vec2, b: Vec2 },
    /// A text among a block's pieces (docs/adr/0144): at its placed
    /// insertion point, turned by `rotation`, `height` as placed.
    PieceText {
        slot: Slot,
        at: Vec2,
        rotation: f64,
        height: f64,
        text: String,
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
        angular: bool,
        prefix: &'static str,
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

/// An arc's sweep from `a0` to `a1`, counter-clockwise, radians (the core's
/// `sweep`, the properties panel's Yay açısı).
pub fn arc_sweep(a0: f64, a1: f64) -> f64 {
    kentos_geometry_core::geom::arc::sweep(a0, a1)
}

/// Whether an ellipse object is whole rather than an elliptical arc (the
/// core's `is_full_ellipse`).
pub fn full_ellipse(e: &kentos_contracts::EllipseEntity) -> bool {
    let v = |p: kentos_contracts::Vec2| Vec2::new(p.x, p.y);
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

/// The kinds with a label style of their own, in the store's order (`set_label_defaults`).
pub const LABELLED_KINDS: [&str; 5] = ["polygon", "circle", "point", "polyline", "line"];

/// The label style of an object whose layer has none: the web's
/// `DEFAULT_LABELS` (apps/web/src/viewport/storeRecords.ts); the desktop
/// app's tests hold the two to each other (labels.rs).
pub fn default_label(kind: &str) -> Option<LabelStyle> {
    let style = |placement, size| LabelStyle {
        placement,
        size,
        grow: None,
        max_size: None,
        weight: None,
        template: None,
        min_feature_px: None,
        min_scale: None,
        max_scale: None,
        ink: None,
    };
    Some(match kind {
        "polygon" => LabelStyle {
            grow: Some(1.0),
            max_size: Some(14.0),
            min_feature_px: Some(26.0),
            ..style(LabelPlacement::Center, 10.0)
        },
        "circle" => LabelStyle {
            min_feature_px: Some(26.0),
            ..style(LabelPlacement::Center, 10.0)
        },
        "point" => LabelStyle {
            min_scale: Some(2.0),
            ..style(LabelPlacement::Beside, 10.5)
        },
        "polyline" | "line" => LabelStyle {
            min_scale: Some(1.6),
            ..style(LabelPlacement::Along, 10.0)
        },
        _ => return None,
    })
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
