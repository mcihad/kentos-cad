//! The geometry store across the boundary (docs/adr/0008, S1): objects go
//! in packed or as JSON (puts, removals, the layer table), queries take
//! numbers and give back flat arrays of numbers; moved, copied and pasted
//! objects come back packed (`PackedObjects`). `apps/web/src/viewport/picking.ts`
//! holds one per view; the clipboard has its own.

use kentos_geometry_core::Vec2;
use kentos_geometry_core::api::json::{self, FromJson, Json};
use kentos_geometry_core::entity::Entity;
use kentos_geometry_core::geom::intersect::Edge;
use kentos_geometry_core::geometry::Bounds;
use kentos_geometry_core::processing::numbering::{CornerWalk, StartCorner};
use kentos_geometry_core::store::Store;
use wasm_bindgen::prelude::*;

fn read_entity(text: &str) -> Result<Entity, JsError> {
    Json::parse(text)
        .and_then(|v| Entity::from_json(&v))
        .map_err(|e| JsError::new(&format!("Geometri deposu nesneyi okuyamadı: {e}")))
}

fn rect(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Bounds {
    Bounds {
        min_x,
        min_y,
        max_x,
        max_y,
    }
}

/// Edges as numbers: `0, ax, ay, bx, by` for a segment, `1, cx, cy, r, a0, sweep` for an arc.
pub fn pack_edges(edges: &[Edge]) -> Vec<f64> {
    let mut out = Vec::with_capacity(edges.len() * 5);
    for e in edges {
        match *e {
            Edge::Seg { a, b } => out.extend([0.0, a.x, a.y, b.x, b.y]),
            Edge::Arc { c, r, a0, sweep } => out.extend([1.0, c.x, c.y, r, a0, sweep]),
        }
    }
    out
}

/// Affines as the page sends them: six numbers each.
fn affines_of(flat: &[f64]) -> Vec<[f64; 6]> {
    flat.chunks_exact(6)
        .map(|m| [m[0], m[1], m[2], m[3], m[4], m[5]])
        .collect()
}

/// Objects packed as `putPacked` reads them (`apps/web/src/wasm/pack.ts`
/// `unpackEntities` reads them back): the store's answer to a move, copy,
/// array or paste. `intoNums` hands the numbers over and frees the object.
#[wasm_bindgen]
pub struct PackedObjects {
    nums: Vec<f64>,
    strings: String,
}

#[wasm_bindgen]
impl PackedObjects {
    /// A JSON array of the strings the numbers point at.
    #[wasm_bindgen(getter)]
    pub fn strings(&self) -> String {
        self.strings.clone()
    }

    /// The numbers; the object is used up (no second copy of a large answer).
    #[wasm_bindgen(js_name = intoNums)]
    pub fn into_nums(self) -> Vec<f64> {
        self.nums
    }
}

#[wasm_bindgen]
pub struct GeometryStore {
    inner: Store,
}

impl Default for GeometryStore {
    fn default() -> Self {
        GeometryStore::new()
    }
}

#[wasm_bindgen]
impl GeometryStore {
    #[wasm_bindgen(constructor)]
    pub fn new() -> GeometryStore {
        GeometryStore {
            inner: Store::new(),
        }
    }

    /// Adds or replaces objects (a JSON array of entities): new ids go last,
    /// known ones keep their place, as in the document's `Map`.
    pub fn put(&mut self, entities: &str) -> Result<u32, JsError> {
        self.inner
            .put_json(entities)
            .map(|n| n as u32)
            .map_err(|e| JsError::new(&format!("Geometri deposu nesneleri okuyamadı: {e}")))
    }

    /// Adds or replaces packed objects (`apps/web/src/wasm/pack.ts`): the numbers and
    /// a JSON array of the strings they point at.
    #[wasm_bindgen(js_name = putPacked)]
    pub fn put_packed(&mut self, nums: &[f64], strings: &str) -> Result<u32, JsError> {
        let strings = Json::parse(strings)
            .and_then(|v| Vec::<String>::from_json(&v))
            .map_err(|e| JsError::new(&format!("Geometri deposu metinleri okuyamadı: {e}")))?;
        self.inner
            .put_packed(nums, &strings)
            .map(|n| n as u32)
            .map_err(|e| JsError::new(&format!("Geometri deposu nesneleri okuyamadı: {e}")))
    }

    pub fn remove(&mut self, ids: &[f64]) {
        self.inner.remove(ids);
    }

    pub fn clear(&mut self) {
        self.inner.clear();
    }

    /// `[{ id, visible, locked, pickInterior }]` for every layer node, ancestors resolved.
    #[wasm_bindgen(js_name = setLayers)]
    pub fn set_layers(&mut self, layers: &str) -> Result<(), JsError> {
        self.inner
            .set_layers_json(layers)
            .map_err(|e| JsError::new(&format!("Geometri deposu katmanları okuyamadı: {e}")))
    }

    /// Label rules by kind for layers without a label style: `{ polygon, circle, point, polyline, line }`.
    #[wasm_bindgen(js_name = setLabelDefaults)]
    pub fn set_label_defaults(&mut self, defaults: &str) -> Result<(), JsError> {
        self.inner.set_label_defaults_json(defaults).map_err(|e| {
            JsError::new(&format!(
                "Geometri deposu etiket varsayılanlarını okuyamadı: {e}"
            ))
        })
    }

    #[wasm_bindgen(getter)]
    pub fn size(&self) -> u32 {
        self.inner.len() as u32
    }

    /// Ids in the document's order.
    pub fn ids(&self) -> Vec<f64> {
        self.inner.ids()
    }

    /// An object as the store holds it (id, layer, label flag, geometry), as JSON; for tests.
    #[wasm_bindgen(js_name = itemJson)]
    pub fn item_json(&self, id: f64) -> Option<String> {
        self.inner.item_json(id)
    }

    /// `[minX, minY, maxX, maxY]` of an object, or nothing.
    pub fn bounds(&self, id: f64) -> Vec<f64> {
        self.inner.get(id).map_or_else(Vec::new, |it| {
            let b = it.bounds;
            vec![b.min_x, b.min_y, b.max_x, b.max_y]
        })
    }

    /// `[minX, minY, maxX, maxY]` around these objects (all of them without
    /// `ids`), or nothing when there are none.
    pub fn extent(&self, ids: Option<Box<[f64]>>) -> Vec<f64> {
        self.inner
            .extent(ids.as_deref())
            .map_or_else(Vec::new, |b| vec![b.min_x, b.min_y, b.max_x, b.max_y])
    }

    /// The object picked at a point, if any.
    pub fn hit(&self, x: f64, y: f64, tol: f64) -> Option<f64> {
        self.inner.hit(Vec2::new(x, y), tol)
    }

    /// `id, distance` pairs of objects whose edges are within `tol`, nearest first.
    #[wasm_bindgen(js_name = hitEdge)]
    pub fn hit_edge(&self, x: f64, y: f64, tol: f64) -> Vec<f64> {
        self.inner
            .hit_edge(Vec2::new(x, y), tol)
            .into_iter()
            .flat_map(|(id, d)| [id, d])
            .collect()
    }

    /// `[kind, x, y, id]` of the snap point (kind: the bit number), or nothing.
    /// `from` counts only when `has_from` (the command's last point).
    #[allow(clippy::too_many_arguments)]
    pub fn snap(
        &self,
        x: f64,
        y: f64,
        tol: f64,
        kinds: u32,
        has_from: bool,
        fx: f64,
        fy: f64,
    ) -> Vec<f64> {
        let from = has_from.then(|| Vec2::new(fx, fy));
        self.inner
            .snap(Vec2::new(x, y), tol, kinds, from)
            .map_or_else(Vec::new, |h| {
                vec![f64::from(h.kind as u32), h.point.x, h.point.y, h.id]
            })
    }

    /// Ids of visible objects whose boxes come within `tol` of the point.
    pub fn near(&self, x: f64, y: f64, tol: f64) -> Vec<f64> {
        self.inner
            .near(Vec2::new(x, y), tol)
            .iter()
            .map(|it| it.id)
            .collect()
    }

    /// Ids of visible objects whose boxes overlap the rectangle, `except` left out when `has_except`.
    #[allow(clippy::too_many_arguments)]
    pub fn overlapping(
        &self,
        min_x: f64,
        min_y: f64,
        max_x: f64,
        max_y: f64,
        has_except: bool,
        except: f64,
    ) -> Vec<f64> {
        self.inner
            .overlapping(
                &rect(min_x, min_y, max_x, max_y),
                has_except.then_some(except),
            )
            .iter()
            .map(|it| it.id)
            .collect()
    }

    /// Window (fully inside) or crossing selection.
    #[wasm_bindgen(js_name = inRect)]
    pub fn in_rect(
        &self,
        min_x: f64,
        min_y: f64,
        max_x: f64,
        max_y: f64,
        crossing: bool,
    ) -> Vec<f64> {
        self.inner
            .in_rect(&rect(min_x, min_y, max_x, max_y), crossing)
    }

    /// `[id, x0, y0, x1, y1, …]`: the smallest closed shape around the point and its ring, or nothing.
    pub fn enclosing(&self, x: f64, y: f64) -> Vec<f64> {
        self.inner
            .enclosing(Vec2::new(x, y))
            .map_or_else(Vec::new, |(id, ring)| {
                let mut out = Vec::with_capacity(1 + 2 * ring.len());
                out.push(id);
                for p in ring {
                    out.extend([p.x, p.y]);
                }
                out
            })
    }

    /// What the overlay draws in the view at `scale` px/m (eight numbers per
    /// record, see `Store::labels`); `editing` is left out when `has_editing`.
    #[allow(clippy::too_many_arguments)]
    pub fn labels(
        &self,
        min_x: f64,
        min_y: f64,
        max_x: f64,
        max_y: f64,
        scale: f64,
        has_editing: bool,
        editing: f64,
    ) -> Vec<f64> {
        self.inner.labels(
            &rect(min_x, min_y, max_x, max_y),
            scale,
            has_editing.then_some(editing),
        )
    }

    /// Grips of these objects (see `Store::grips`).
    pub fn grips(&self, ids: &[f64]) -> Vec<f64> {
        self.inner.grips(ids)
    }

    /// `trimEntity(target, …)` against the boundaries the trim tool would
    /// pass: the `chosen` objects, or the visible edges in the view (the
    /// target, `except`, left out). `{ pieces }` or `{ error }` as JSON.
    #[wasm_bindgen(js_name = trimPreview)]
    #[allow(clippy::too_many_arguments)]
    pub fn trim_preview(
        &self,
        target: &str,
        x: f64,
        y: f64,
        has_except: bool,
        except: f64,
        chosen: Option<Box<[f64]>>,
        min_x: f64,
        min_y: f64,
        max_x: f64,
        max_y: f64,
    ) -> Result<String, JsError> {
        let target = read_entity(target)?;
        let cut = self.inner.trim_preview(
            &target,
            Vec2::new(x, y),
            has_except.then_some(except),
            chosen.as_deref(),
            &rect(min_x, min_y, max_x, max_y),
        );
        Ok(json::to_string(&cut))
    }

    /// `extendEntity(target, …)` against the boundaries the extend tool
    /// would pass (as `trimPreview`). `{ geometry }` or `{ error }` as JSON.
    #[wasm_bindgen(js_name = extendPreview)]
    #[allow(clippy::too_many_arguments)]
    pub fn extend_preview(
        &self,
        target: &str,
        x: f64,
        y: f64,
        has_except: bool,
        except: f64,
        chosen: Option<Box<[f64]>>,
        min_x: f64,
        min_y: f64,
        max_x: f64,
        max_y: f64,
    ) -> Result<String, JsError> {
        let target = read_entity(target)?;
        let g = self.inner.extend_preview(
            &target,
            Vec2::new(x, y),
            has_except.then_some(except),
            chosen.as_deref(),
            &rect(min_x, min_y, max_x, max_y),
        );
        Ok(json::to_string(&g))
    }

    /// Outlines of these objects moved by each affine (six numbers each), at
    /// most `limit` + 1 objects: `flags, n, x0, y0, …` per path (flags 0 open,
    /// 1 closed, 2 a marker).
    #[wasm_bindgen(js_name = transformOutlines)]
    pub fn transform_outlines(&self, ids: &[f64], affines: &[f64], limit: u32) -> Vec<f64> {
        self.inner
            .transform_outlines(ids, &affines_of(affines), limit as usize)
    }

    /// These objects moved by each affine (six numbers each), affine after
    /// affine, as `transformEntities` gives them, packed as `putPacked`
    /// reads them (`Store::transform_packed`): move, copy, arrays and paste
    /// without the objects crossing as JSON. Unknown ids are skipped.
    #[wasm_bindgen(js_name = transformPacked)]
    pub fn transform_packed(&self, ids: &[f64], affines: &[f64]) -> PackedObjects {
        let p = self.inner.transform_packed(ids, &affines_of(affines));
        PackedObjects {
            strings: json::to_string(&p.strings),
            nums: p.nums,
        }
    }

    /// Outlines of these objects stretched by the window and (dx, dy), as `transformOutlines`.
    #[wasm_bindgen(js_name = stretchOutlines)]
    #[allow(clippy::too_many_arguments)]
    pub fn stretch_outlines(
        &self,
        ids: &[f64],
        min_x: f64,
        min_y: f64,
        max_x: f64,
        max_y: f64,
        dx: f64,
        dy: f64,
    ) -> Vec<f64> {
        self.inner
            .stretch_outlines(ids, &rect(min_x, min_y, max_x, max_y), dx, dy)
    }

    /// `[length, area]`: the objects' total length (polygons left out) and area.
    pub fn measure(&self, ids: &[f64]) -> Vec<f64> {
        let (length, area) = self.inner.measure(ids);
        vec![length, area]
    }

    /// What is drawn of these objects, one record each
    /// (`geometry-core::store::draw`); `oriented`: rings turned for the style
    /// engine. Construction lines are clipped to the box, or left out
    /// without one.
    #[allow(clippy::too_many_arguments)]
    pub fn drawn(
        &self,
        ids: &[f64],
        oriented: bool,
        has_clip: bool,
        min_x: f64,
        min_y: f64,
        max_x: f64,
        max_y: f64,
    ) -> Vec<f64> {
        let clip = has_clip.then(|| rect(min_x, min_y, max_x, max_y));
        self.inner.drawn(ids, oriented, clip.as_ref())
    }

    /// Geometry values of these objects for expressions: `flags, length,
    /// area, anchor x, anchor y, 0` each (`store::draw::measure_record`).
    pub fn measures(&self, ids: &[f64]) -> Vec<f64> {
        self.inner.measures(ids)
    }

    /// Ids of objects on every layer whose box overlaps the rectangle, in the
    /// document's order (the processing tools' "visible" scope, `Store::in_box`).
    #[wasm_bindgen(js_name = inBox)]
    pub fn in_box(&self, min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Vec<f64> {
        self.inner.in_box(&rect(min_x, min_y, max_x, max_y))
    }

    /// Corner numbering of these objects (`Store::number_corners`): walked
    /// counter-clockwise or clockwise from the start (0 north-west, 1 north,
    /// 2 first vertex, 3 nearest the point, which counts only when
    /// `has_point`); `existing` points (x, y pairs) keep their numbers when
    /// `shared`. Five numbers per corner.
    #[wasm_bindgen(js_name = numberCorners)]
    #[allow(clippy::too_many_arguments)]
    pub fn number_corners(
        &self,
        ids: &[f64],
        ccw: bool,
        start: u32,
        has_point: bool,
        px: f64,
        py: f64,
        tolerance: f64,
        shared: bool,
        existing: &[f64],
    ) -> Vec<f64> {
        let walk = CornerWalk {
            ccw,
            start: StartCorner::from_code(start),
            point: has_point.then(|| Vec2::new(px, py)),
            tolerance,
            shared,
        };
        let existing: Vec<Vec2> = existing
            .chunks_exact(2)
            .map(|c| Vec2::new(c[0], c[1]))
            .collect();
        self.inner.number_corners(ids, &walk, &existing)
    }

    /// Edge-length labels of these objects (`Store::edge_lengths`): the
    /// number of shared edges skipped, then five numbers per label.
    #[wasm_bindgen(js_name = edgeLengths)]
    pub fn edge_lengths(
        &self,
        ids: &[f64],
        height: f64,
        min_length: f64,
        inside: bool,
        shared: bool,
    ) -> Vec<f64> {
        self.inner
            .edge_lengths(ids, height, min_length, inside, shared)
    }

    /// Edges of visible objects overlapping the rectangle (see `pack_edges`).
    #[wasm_bindgen(js_name = edgesIn)]
    #[allow(clippy::too_many_arguments)]
    pub fn edges_in(
        &self,
        min_x: f64,
        min_y: f64,
        max_x: f64,
        max_y: f64,
        has_except: bool,
        except: f64,
    ) -> Vec<f64> {
        pack_edges(&self.inner.edges_in(
            &rect(min_x, min_y, max_x, max_y),
            has_except.then_some(except),
        ))
    }
}
