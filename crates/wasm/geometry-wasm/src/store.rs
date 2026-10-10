//! The geometry store across the boundary (docs/adr/0008, S1): objects go
//! in packed or as JSON (puts, removals, the layer table), queries take
//! numbers and give back flat arrays of numbers; moved, copied and pasted
//! objects come back packed (`PackedObjects`). `apps/web/src/viewport/picking.ts`
//! holds one per view; the clipboard has its own.

use std::collections::HashMap;

use kentos_geometry_core::Vec2;
use kentos_geometry_core::api::json::{self, FromJson, Json};
use kentos_geometry_core::entity::{Entity, Shape};
use kentos_geometry_core::geom::affine::similarity;
use kentos_geometry_core::geom::intersect::Edge;
use kentos_geometry_core::geometry::Bounds;
use kentos_geometry_core::ops::spatial_query::Relation;
use kentos_geometry_core::processing::numbering::{CornerWalk, StartCorner};
use kentos_geometry_core::store::overview::OverviewRequest;
use kentos_geometry_core::store::proximity::Measure;
use kentos_geometry_core::store::snap::{Extension, SnapExtras};
use kentos_geometry_core::store::{Store, array_packed_objects, transform_packed_objects};
use kentos_geometry_core::text::Font;
use kentos_geometry_core::tools::editing::array_transforms;
use kentos_style_core::style::build::{LayerObjects, Program, View, ViewFrame, build_layer_in};
use wasm_bindgen::prelude::*;

fn read_entity(text: &str) -> Result<Entity, JsError> {
    Json::parse(text)
        .and_then(|v| Entity::from_json(&v))
        .map_err(|e| JsError::new(&format!("Geometri deposu nesneyi okuyamadı: {e}")))
}

/// `#RRGGBB` as its three bytes.
fn hex_color(text: &str) -> Option<[u8; 3]> {
    let hex = text.strip_prefix('#')?;
    if hex.len() != 6 {
        return None;
    }
    let v = u32::from_str_radix(hex, 16).ok()?;
    Some([(v >> 16) as u8, (v >> 8) as u8, v as u8])
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

/// Packed objects (`apps/web/src/wasm/pack.ts`) moved by one similarity,
/// packed again, with no store (`transform_packed_objects`, docs/adr/0037):
/// the web's `cad.entities.transform` handler. `kind` and `params` are the
/// command's transform (`similarity`: `move` dx, dy; `rotate` cx, cy,
/// angle; `scale` cx, cy, factor; `mirror` ax, ay, bx, by); the matrix is
/// built here, so none of its numbers crosses as JSON either and −0 stays −0.
#[wasm_bindgen(js_name = transformObjects)]
pub fn transform_objects(
    nums: &[f64],
    strings: &str,
    kind: &str,
    params: &[f64],
) -> Result<PackedObjects, JsError> {
    let m = similarity(kind, params).ok_or_else(|| {
        JsError::new(&format!(
            "Bilinmeyen dönüşüm: {kind} ({} sayı).",
            params.len()
        ))
    })?;
    let strings = Json::parse(strings)
        .and_then(|v| Vec::<String>::from_json(&v))
        .map_err(|e| {
            JsError::new(&format!(
                "Dönüştürülecek nesnelerin metinleri okunamadı: {e}"
            ))
        })?;
    let p = transform_packed_objects(nums, &strings, &[m])
        .map_err(|e| JsError::new(&format!("Dönüştürülecek nesneler okunamadı: {e}")))?;
    Ok(PackedObjects {
        strings: json::to_string(&p.strings),
        nums: p.nums,
    })
}

/// Packed objects (`apps/web/src/wasm/pack.ts`) copied into an array,
/// packed again, with no store (`array_packed_objects`, docs/adr/0047): the
/// web's `cad.entities.array` handler. `kind` and `params` are the array
/// (`array_transforms`: `grid` rows, cols, dx, dy; `polar` cx, cy, count,
/// fill, rotate 1 or 0); `font` is the drawing's typeface id, which measures
/// text for a polar array's middle. The copies come affine after affine,
/// each run in the order the objects were packed; no number crosses as JSON.
#[wasm_bindgen(js_name = arrayObjects)]
pub fn array_objects(
    nums: &[f64],
    strings: &str,
    kind: &str,
    params: &[f64],
    font: &str,
) -> Result<PackedObjects, JsError> {
    let strings = Json::parse(strings)
        .and_then(|v| Vec::<String>::from_json(&v))
        .map_err(|e| {
            JsError::new(&format!(
                "Diziye alınacak nesnelerin metinleri okunamadı: {e}"
            ))
        })?;
    let font = Font::from_id(font);
    let p = array_packed_objects(nums, &strings, |shapes| {
        array_transforms(kind, params, shapes, font)
    })
    .map_err(|e| JsError::new(&format!("Diziye alınacak nesneler okunamadı: {e}")))?
    .ok_or_else(|| JsError::new(&format!("Dizi kurulamadı: {kind} ({} sayı).", params.len())))?;
    Ok(PackedObjects {
        strings: json::to_string(&p.strings),
        nums: p.nums,
    })
}

/// A layer's symbols and expressions for one styled build
/// (`kentos_style_core::style::build::Program`): the page reads which values
/// its expressions need, builds the table, and hands both to `buildStyled`.
#[wasm_bindgen]
pub struct StyleProgram {
    inner: Program,
}

#[wasm_bindgen]
impl StyleProgram {
    #[wasm_bindgen(constructor)]
    pub fn new(json: &str) -> Result<StyleProgram, JsError> {
        Program::read(json)
            .map(|inner| StyleProgram { inner })
            .map_err(|e| JsError::new(&format!("Stil okunamadı: {e}")))
    }

    /// The attribute names the expressions read (a JSON array), the table's text slots in order.
    #[wasm_bindgen(getter)]
    pub fn fields(&self) -> String {
        json::to_string(&self.inner.fields)
    }

    /// The variables they read, as bits: 1 geometry values, 2 corners, 4 kind,
    /// 8 layer, 16 label, 32 position, 64 id, 128 scale.
    #[wasm_bindgen(getter)]
    pub fn needs(&self) -> u32 {
        let n = self.inner.needs;
        [
            n.measured, n.vertices, n.kind, n.layer, n.label, n.index, n.id, n.scale,
        ]
        .iter()
        .enumerate()
        .fold(0, |bits, (i, &on)| bits | (u32::from(on) << i))
    }

    /// What a build of it depends on beyond its objects (docs/adr/0213 §3),
    /// as bits: 1 the view's scale, 2 the heat map's box, 4 the construction
    /// lines' box, 8 built whole.
    #[wasm_bindgen(getter, js_name = viewNeeds)]
    pub fn view_needs(&self) -> u32 {
        use kentos_style_core::style::model::Frame;
        let n = self.inner.view_needs();
        u32::from(n.scale)
            | match n.frame {
                Some(Frame::Heat) => 2,
                Some(Frame::Construction) => 4,
                None => 0,
            }
            | if self.inner.whole() { 8 } else { 0 }
    }
}

/// Why a renderer (its JSON) cannot be a layer's (docs/adr/0213 §5), or
/// an empty text.
#[wasm_bindgen(js_name = rendererProblem)]
pub fn renderer_problem(json: &str) -> String {
    kentos_style_core::style::rules::renderer_problem_text(json).unwrap_or_default()
}

/// A styled layer's batches (`style::batch`): their descriptions as JSON and
/// their numbers one after another (float32, origin-relative); the
/// pictures the build made (a heat map's) and the dots left out.
#[wasm_bindgen]
pub struct StyledBatches {
    json: String,
    data: Vec<f32>,
    pictures: Vec<kentos_style_core::style::batch::Picture>,
    dropped: u64,
}

#[wasm_bindgen]
impl StyledBatches {
    #[wasm_bindgen(getter)]
    pub fn json(&self) -> String {
        self.json.clone()
    }

    /// The pictures' keys and sizes, `[{key, width, height}]`.
    #[wasm_bindgen(getter)]
    pub fn pictures(&self) -> String {
        let mut out = String::from("[");
        for (i, p) in self.pictures.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push_str(&format!(
                "{{\"key\":{},\"width\":{},\"height\":{}}}",
                json::to_string(&Json::Str(p.key.clone())),
                p.width,
                p.height
            ));
        }
        out.push(']');
        out
    }

    /// A picture's pixels (RGBA, straight alpha, rows from the top).
    #[wasm_bindgen(js_name = pictureData)]
    pub fn picture_data(&self, i: usize) -> Vec<u8> {
        self.pictures
            .get(i)
            .map(|p| p.rgba.clone())
            .unwrap_or_default()
    }

    /// Dots left out over the layer's limit (docs/adr/0213 §2.4).
    #[wasm_bindgen(getter)]
    pub fn dropped(&self) -> f64 {
        self.dropped as f64
    }

    /// The numbers; the object is used up.
    #[wasm_bindgen(js_name = intoData)]
    pub fn into_data(self) -> Vec<f32> {
        self.data
    }
}

#[wasm_bindgen]
pub struct GeometryStore {
    inner: Store,
    /// The texts of the last labels asked (`placedTexts`), a line each.
    texts: String,
    /// The main view's last labels (`labelAt`): its records, their stride and scale.
    kept: (Vec<f64>, usize, f64),
}

impl Default for GeometryStore {
    fn default() -> Self {
        GeometryStore::new()
    }
}

/// A placing's options from `labels`' flags.
fn place_options(flags: u32) -> kentos_geometry_core::store::placing::PlaceOptions {
    kentos_geometry_core::store::placing::PlaceOptions {
        unplaced: flags & 1 != 0,
        hidden: flags & 2 != 0,
    }
}

impl GeometryStore {
    /// A window's labels' texts kept for `placedTexts`, its records for `labelAt` when the flags say so.
    fn keep(
        &mut self,
        shown: kentos_geometry_core::store::placing::Shown,
        stride: usize,
        scale: f64,
        flags: u32,
    ) -> Vec<f64> {
        self.texts = shown.texts.join("\n");
        if flags & 4 != 0 {
            self.kept = (shown.records.clone(), stride, scale);
        }
        shown.records
    }

    /// The store itself, for the crate's other classes (a network built from its objects).
    pub(crate) fn store(&self) -> &Store {
        &self.inner
    }

    /// The store to change, for the crate's other bindings (the objects' times, docs/adr/0210 §6).
    pub(crate) fn store_mut(&mut self) -> &mut Store {
        &mut self.inner
    }
}

#[wasm_bindgen]
impl GeometryStore {
    #[wasm_bindgen(constructor)]
    pub fn new() -> GeometryStore {
        GeometryStore {
            inner: Store::new(),
            texts: String::new(),
            kept: (Vec::new(), 0, 1.0),
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

    /// The drawing typeface (a `DrawingFont` id): text boxes follow its measured letters.
    #[wasm_bindgen(js_name = setFont)]
    pub fn set_font(&mut self, id: &str) {
        self.inner
            .set_font(kentos_geometry_core::text::Font::from_id(id));
    }

    /// `[{ id, visible, locked, pickInterior }]` for every layer node, ancestors resolved.
    #[wasm_bindgen(js_name = setLayers)]
    pub fn set_layers(&mut self, layers: &str) -> Result<(), JsError> {
        self.inner
            .set_layers_json(layers)
            .map_err(|e| JsError::new(&format!("Geometri deposu katmanları okuyamadı: {e}")))
    }

    /// The drawing's block definitions (docs/adr/0144), the contract's JSON
    /// (`[{ id, base, entities, … }]`): every insert is placed again.
    #[wasm_bindgen(js_name = setBlocks)]
    pub fn set_blocks(&mut self, blocks: &str) -> Result<(), JsError> {
        self.inner
            .set_blocks_json(blocks)
            .map_err(|e| JsError::new(&format!("Geometri deposu blok tanımlarını okuyamadı: {e}")))
    }

    /// A block's pieces as the drawn `GROUP` records and the `LABEL_PIECE_*`
    /// labels number them (their geometry relative to the base point, their
    /// own colour and line weight), as JSON; nothing for an unknown block.
    #[wasm_bindgen(js_name = blockPieces)]
    pub fn block_pieces(&self, block: &str) -> Option<String> {
        self.inner.block_pieces_json(block)
    }

    /// An insert's pieces as placed (`blockPieces`' layout, in the drawing's
    /// coordinates); nothing for any other object.
    #[wasm_bindgen(js_name = insertPieces)]
    pub fn insert_pieces(&self, id: f64) -> Option<String> {
        self.inner.insert_pieces_json(id)
    }

    /// Patlat of an insert (docs/adr/0144 §3), the entity as JSON: its
    /// definition one level open, as `explodeEntity`'s answer (`{ pieces }`
    /// or `{ error }`).
    #[wasm_bindgen(js_name = explodeInsert)]
    pub fn explode_insert(&self, entity: &str) -> Result<String, JsError> {
        let e = Json::parse(entity)
            .and_then(|v| Entity::from_json(&v))
            .map_err(|e| JsError::new(&format!("Patlatılacak blok okunamadı: {e}")))?;
        let mut out = String::new();
        json::ToJson::write_json(&self.inner.blocks().explode(&e), &mut out);
        Ok(out)
    }

    /// The label styles of objects whose layer has none, by kind: `{ polygon, circle, point, polyline, line }`
    /// (`LabelStyle`s, docs/adr/0212 §2).
    #[wasm_bindgen(js_name = setLabelDefaults)]
    pub fn set_label_defaults(&mut self, defaults: &str) -> Result<(), JsError> {
        self.inner.set_label_defaults_json(defaults).map_err(|e| {
            JsError::new(&format!(
                "Geometri deposu etiket varsayılanlarını okuyamadı: {e}"
            ))
        })
    }

    /// The label engine's layers (docs/adr/0212 §3.1): `[{ id, rank, point?, label?, labels? }]`.
    #[wasm_bindgen(js_name = setLabelLayers)]
    pub fn set_label_layers(&mut self, layers: &str) -> Result<(), JsError> {
        self.inner.set_label_layers_json(layers).map_err(|e| {
            JsError::new(&format!(
                "Geometri deposu katmanların etiketlemesini okuyamadı: {e}"
            ))
        })
    }

    /// Objects' labels' texts (docs/adr/0212 §3.1): `ids[i]`'s are the entries `from[i]..from[i + 1]`
    /// of `classes` and `texts` (`lens` their UTF-16 lengths), its height `zs[i]`.
    #[wasm_bindgen(js_name = setObjectLabels)]
    pub fn set_object_labels(
        &mut self,
        ids: &[f64],
        from: &[u32],
        classes: &[u16],
        texts: &str,
        lens: &[u32],
        zs: &[f64],
    ) -> Result<(), JsError> {
        self.inner
            .set_object_labels_packed(ids, from, classes, texts, lens, zs)
            .map_err(|e| {
                JsError::new(&format!(
                    "Geometri deposu etiket metinlerini okuyamadı: {e}"
                ))
            })
    }

    /// Objects' pins (docs/adr/0212 §3.7): `[[id, [LabelPin …]] …]`; an empty list forgets an object's.
    #[wasm_bindgen(js_name = setLabelPins)]
    pub fn set_label_pins(&mut self, pins: &str) -> Result<(), JsError> {
        self.inner
            .set_label_pins_json(pins)
            .map_err(|e| JsError::new(&format!("Geometri deposu etiket iğnelerini okuyamadı: {e}")))
    }

    /// The label engine alone (docs/adr/0212 §3; the shared cases'
    /// `fixtures/labels/v1`): the labels of the window (x0, y0)–(x1, y1) at
    /// `scale` px per metre around the drawing's texts' outlines `fixed`
    /// (`[[[x, y] …] …]` as JSON, world), as `Store::place_labels` gives
    /// them; `flags` as `labels`'. Their texts are `placedTexts`'.
    #[wasm_bindgen(js_name = placeLabels)]
    #[allow(clippy::too_many_arguments)]
    pub fn place_labels(
        &mut self,
        x0: f64,
        y0: f64,
        x1: f64,
        y1: f64,
        scale: f64,
        fixed: &str,
        flags: u32,
    ) -> Result<Vec<f64>, JsError> {
        let outlines = Json::parse(fixed)
            .and_then(|v| Vec::<Vec<[f64; 2]>>::from_json(&v))
            .map_err(|e| JsError::new(&format!("Yazıların çerçeveleri okunamadı: {e}")))?;
        let fixed: Vec<Vec<Vec2>> = outlines
            .iter()
            .map(|o| o.iter().map(|p| Vec2::new(p[0], p[1])).collect())
            .collect();
        let window = Bounds {
            min_x: x0,
            min_y: y0,
            max_x: x1,
            max_y: y1,
        };
        let shown = self
            .inner
            .place_labels(&window, scale, &fixed, place_options(flags));
        self.texts = shown.texts.join("\n");
        Ok(shown.records)
    }

    /// The texts of the labels last asked (`labels`, `labelsShown`), a line each.
    #[wasm_bindgen(js_name = placedTexts)]
    pub fn placed_texts(&self) -> String {
        self.texts.clone()
    }

    /// The label under (x, y) among the main view's last labels, within `tol` px: `[id, class, state, x, y,
    /// angle, w, h]`, empty for none; `all` counts the unplaced and the hidden (docs/adr/0212 §4).
    #[wasm_bindgen(js_name = labelAt)]
    pub fn label_at(&self, x: f64, y: f64, tol: f64, all: bool) -> Vec<f64> {
        let (records, stride, scale) = &self.kept;
        kentos_geometry_core::store::placing::label_at(
            records,
            *stride,
            *scale,
            Vec2::new(x, y),
            tol,
            all,
        )
        .map_or_else(Vec::new, |h| {
            vec![
                h.id,
                f64::from(h.class),
                f64::from(h.state),
                h.at.x,
                h.at.y,
                h.angle,
                h.w,
                h.h,
            ]
        })
    }

    /// The labels whose middle is in the box (x0, y0)–(x1, y1) among the main view's last labels: 8 numbers each, as
    /// `labelAt` gives one; `all` counts the unplaced and the hidden (Etiketi sabitle's window, docs/adr/0212 §4).
    #[wasm_bindgen(js_name = labelsIn)]
    pub fn labels_in(&self, x0: f64, y0: f64, x1: f64, y1: f64, all: bool) -> Vec<f64> {
        let (records, stride, _) = &self.kept;
        kentos_geometry_core::store::placing::labels_in(
            records,
            *stride,
            Vec2::new(x0, y0),
            Vec2::new(x1, y1),
            all,
        )
        .iter()
        .flat_map(|h| {
            [
                h.id,
                f64::from(h.class),
                f64::from(h.state),
                h.at.x,
                h.at.y,
                h.angle,
                h.w,
                h.h,
            ]
        })
        .collect()
    }

    /// Where an object's labels are pinned from: its anchor `[x, y]` (world), empty for none (docs/adr/0212 §2).
    #[wasm_bindgen(js_name = labelAnchor)]
    pub fn label_anchor(&self, id: f64) -> Vec<f64> {
        self.inner
            .label_anchor(id)
            .map_or_else(Vec::new, |p| vec![p.x, p.y])
    }

    /// The objects whose label a text writes (docs/adr/0175 §4): their own labels are left out.
    #[wasm_bindgen(js_name = setTextLabelled)]
    pub fn set_text_labelled(&mut self, ids: &[f64]) {
        self.inner.set_text_labelled(ids);
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

    /// Every object a click at the point could mean, the most specific
    /// first (Sıradakini seç, docs/adr/0187 §1); the first is `hit`'s.
    pub fn hits(&self, x: f64, y: f64, tol: f64) -> Vec<f64> {
        self.inner.hits(Vec2::new(x, y), tol)
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

    /// `snap` with what the drawing does not hold (docs/adr/0163): acquired
    /// extensions as records (`[0, endX, endY, dirX, dirY]` a line,
    /// `[1, cx, cy, r, a0, sweep]` an arc), parallel directions (`[ux, uy]`
    /// each), the object being drawn as an open path (`draftXy`, its bulges or
    /// none), Karelaj's spacings (0: none).
    #[wasm_bindgen(js_name = snapEx)]
    #[allow(clippy::too_many_arguments)]
    pub fn snap_ex(
        &self,
        x: f64,
        y: f64,
        tol: f64,
        kinds: u32,
        has_from: bool,
        fx: f64,
        fy: f64,
        extensions: &[f64],
        parallels: &[f64],
        draft_xy: &[f64],
        draft_bulges: &[f64],
        grid_x: f64,
        grid_y: f64,
    ) -> Vec<f64> {
        let from = has_from.then(|| Vec2::new(fx, fy));
        let extras = SnapExtras {
            extensions: read_extensions(extensions),
            parallels: parallels
                .chunks_exact(2)
                .map(|u| Vec2::new(u[0], u[1]))
                .collect(),
            draft: draft_path(draft_xy, draft_bulges),
            grid: (grid_x > 0.0 && grid_y > 0.0).then_some([grid_x, grid_y]),
        };
        self.inner
            .snap_ex(Vec2::new(x, y), tol, kinds, from, &extras)
            .map_or_else(Vec::new, |h| {
                vec![f64::from(h.kind as u32), h.point.x, h.point.y, h.id]
            })
    }

    /// The extensions of object `id`'s edges ending at the point, as
    /// `snapEx` takes them (docs/adr/0163 §2).
    #[wasm_bindgen(js_name = extensionsAt)]
    pub fn extensions_at(&self, id: f64, x: f64, y: f64) -> Vec<f64> {
        let mut out = Vec::new();
        for e in self.inner.extensions_at(id, Vec2::new(x, y)) {
            match e {
                Extension::Line { end, dir } => out.extend([0.0, end.x, end.y, dir.x, dir.y]),
                Extension::Arc { c, r, a0, sweep } => {
                    out.extend([1.0, c.x, c.y, r, a0, sweep]);
                }
            }
        }
        out
    }

    /// The direction (`[ux, uy]`) of the straight edge nearest the point
    /// within `tol`, or nothing (Paralel's acquisition, docs/adr/0163 §2).
    #[wasm_bindgen(js_name = directionAt)]
    pub fn direction_at(&self, x: f64, y: f64, tol: f64) -> Vec<f64> {
        self.inner
            .direction_at(Vec2::new(x, y), tol)
            .map_or_else(Vec::new, |u| vec![u.x, u.y])
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

    /// `[id, area, id, area, …]`: the visible closed shapes around the
    /// point, smallest first (İçeren alanı seç, docs/adr/0141).
    pub fn containing(&self, x: f64, y: f64) -> Vec<f64> {
        self.inner
            .containing(Vec2::new(x, y))
            .into_iter()
            .flat_map(|(id, area)| [id, area])
            .collect()
    }

    /// `[id, part, hole, id, part, hole, …]`: the visible areas with a hole
    /// around the point, the smallest hole first (Deliği sil, Deliği doldur;
    /// docs/adr/0173 §5).
    #[wasm_bindgen(js_name = holesAt)]
    pub fn holes_at(&self, x: f64, y: f64) -> Vec<f64> {
        self.inner
            .holes_at(Vec2::new(x, y))
            .into_iter()
            .flat_map(|(id, at)| [id, at.part as f64, at.hole as f64])
            .collect()
    }

    /// Ids of visible objects the fence `[x0, y0, x1, y1, …]` crosses; a
    /// point within `tol` counts (Çitle seç).
    #[wasm_bindgen(js_name = inFence)]
    pub fn in_fence(&self, fence: &[f64], tol: f64) -> Vec<f64> {
        let pts: Vec<Vec2> = fence
            .chunks_exact(2)
            .map(|c| Vec2::new(c[0], c[1]))
            .collect();
        self.inner.in_fence(&pts, tol)
    }

    /// Ids of visible objects wholly inside the circle, or also those it
    /// touches when `crossing` (Daireyle seç).
    #[wasm_bindgen(js_name = inCircle)]
    pub fn in_circle(&self, x: f64, y: f64, r: f64, crossing: bool) -> Vec<f64> {
        self.inner.in_circle(Vec2::new(x, y), r, crossing)
    }

    /// Ids of visible objects wholly inside the ring `[x0, y0, x1, y1, …]`
    /// (`mode` 0), touching it too (1), or not touching it (2): Çokgenle seç
    /// (docs/adr/0187 §2). A ring that cannot select gives none.
    #[wasm_bindgen(js_name = inPolygon)]
    pub fn in_polygon(&self, ring: &[f64], mode: u8) -> Vec<f64> {
        let pts: Vec<Vec2> = ring
            .chunks_exact(2)
            .map(|c| Vec2::new(c[0], c[1]))
            .collect();
        self.inner.in_polygon(
            &pts,
            kentos_geometry_core::store::polygon::PolygonMode::of(mode),
        )
    }

    /// Ids of visible objects lying far from the rest of the drawing
    /// (Kapsam denetimi).
    #[wasm_bindgen(js_name = extentOutliers)]
    pub fn extent_outliers(&self) -> Vec<f64> {
        self.inner.extent_outliers()
    }

    /// Genel bakış (docs/adr/0181 §3): the extent of what the visible layers
    /// hold, `minX, minY, maxX, maxY`; empty when they hold nothing.
    #[wasm_bindgen(js_name = overviewExtent)]
    pub fn overview_extent(&self) -> Vec<f64> {
        self.inner
            .overview_extent()
            .map_or_else(Vec::new, |b| vec![b.min_x, b.min_y, b.max_x, b.max_y])
    }

    /// The overview's picture at `width` × `height` CSS px and `dpr`: RGBA
    /// with straight alpha, rows top down, ⌊width·dpr + 0.5⌋ pixels wide;
    /// `colors` is `{ "<layer id>": "#RRGGBB" }` (a layer without one is not
    /// drawn). Empty for a drawing with nothing to show.
    #[wasm_bindgen(js_name = overviewPicture)]
    pub fn overview_picture(
        &self,
        width: f64,
        height: f64,
        dpr: f64,
        colors: &str,
    ) -> Result<Vec<u8>, JsError> {
        let read = Json::parse(colors)
            .map_err(|e| JsError::new(&format!("Katman renkleri okunamadı: {e}")))?;
        let mut table = HashMap::new();
        if let Json::Obj(entries) = read {
            for (id, value) in entries {
                if let Json::Str(hex) = value
                    && let Some(c) = hex_color(&hex)
                {
                    table.insert(id, c);
                }
            }
        }
        Ok(self
            .inner
            .overview_picture(&OverviewRequest {
                width,
                height,
                dpr,
                colors: &table,
            })
            .map_or_else(Vec::new, |p| p.pixels))
    }

    /// What the overlay draws in the view at `scale` px/m (nine numbers per
    /// record, see `Store::labels`); `editing` is left out when `has_editing`.
    /// `flags`: 1 the unplaced labels too, 2 the hidden ones, 4 kept for `labelAt`
    /// (docs/adr/0212 §4). The labels' texts are then `placedTexts`.
    #[allow(clippy::too_many_arguments)]
    pub fn labels(
        &mut self,
        min_x: f64,
        min_y: f64,
        max_x: f64,
        max_y: f64,
        scale: f64,
        has_editing: bool,
        editing: f64,
        flags: u32,
    ) -> Vec<f64> {
        let shown = self.inner.labels(
            &rect(min_x, min_y, max_x, max_y),
            scale,
            has_editing.then_some(editing),
            place_options(flags),
        );
        self.keep(
            shown,
            kentos_geometry_core::store::labels::LABEL_STRIDE,
            scale,
            flags,
        )
    }

    /// The same as the view shows them (docs/adr/0205 §5): each record and
    /// its factor and anchor, `LABEL_SHOWN_STRIDE` numbers (see
    /// `Store::labels_shown`); `size` is `graphics.annotationSize`'s value
    /// (`legible`, `true`, `screen`), `plot_scale` the project's; `flags` as `labels`'.
    #[allow(clippy::too_many_arguments)]
    #[wasm_bindgen(js_name = labelsShown)]
    pub fn labels_shown(
        &mut self,
        min_x: f64,
        min_y: f64,
        max_x: f64,
        max_y: f64,
        scale: f64,
        has_editing: bool,
        editing: f64,
        size: &str,
        plot_scale: f64,
        flags: u32,
    ) -> Vec<f64> {
        let shown = self.inner.labels_shown(
            &rect(min_x, min_y, max_x, max_y),
            scale,
            has_editing.then_some(editing),
            kentos_geometry_core::store::legible::LabelSize::of(size, plot_scale),
            place_options(flags),
        );
        self.keep(
            shown,
            kentos_geometry_core::store::legible::LABEL_SHOWN_STRIDE,
            scale,
            flags,
        )
    }

    /// Etiketleri yazıya çevir (docs/adr/0212 §4, `Store::label_texts`): the
    /// labels of the objects `ids` as the engine places their window at
    /// 1:`scale`, written as objects; `every` writes the unplaced too.
    /// `{ texts, callouts, outOfScale, small, overlapping }` as JSON.
    #[wasm_bindgen(js_name = labelTexts)]
    pub fn label_texts(&self, ids: &[f64], scale: f64, every: bool) -> String {
        let mut out = String::new();
        json::ToJson::write_json(&self.inner.label_texts(ids, scale, every), &mut out);
        out
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

    /// Outlines of a block placed as an insert would place it (docs/adr/0144):
    /// Blok ekle's ghost, and the Bloklar panel's picture at the origin; as
    /// `transformOutlines`, nothing for a block the drawing does not define.
    #[wasm_bindgen(js_name = insertOutlines)]
    pub fn insert_outlines(
        &self,
        block: &str,
        x: f64,
        y: f64,
        scale: f64,
        rotation: f64,
        mirror: bool,
    ) -> Vec<f64> {
        self.inner
            .insert_outlines(block, Vec2::new(x, y), scale, rotation, mirror)
    }

    /// A drawing point in the own coordinates of the definition the insert
    /// `id` places (docs/adr/0144): the Bloklar panel's new base point shown
    /// on that insert. `[x, y]`, or empty for an object that is not an
    /// insert of a known block.
    #[wasm_bindgen(js_name = insertLocal)]
    pub fn insert_local(&self, id: f64, x: f64, y: f64) -> Vec<f64> {
        self.inner
            .insert_local(id, Vec2::new(x, y))
            .map_or_else(Vec::new, |p| vec![p.x, p.y])
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

    /// One expression's values for these objects (docs/adr/0100 §3): what it
    /// reads of their attributes and names crosses as the expression table
    /// (`kentos_expression::rows`, without `measures`); the geometry values
    /// (`$alan`, `$merkez_y`, `$genişlik` …) are read here from the store's
    /// shapes, only those the expression reads. `want` as `exprEvaluate`'s.
    #[allow(clippy::too_many_arguments)]
    #[wasm_bindgen(js_name = evaluateExpression)]
    pub fn evaluate_expression(
        &self,
        source: &str,
        ids: &[f64],
        texts: &str,
        text_lens: &[i32],
        numbers: &[f64],
        scale: f64,
        want: u8,
    ) -> Result<crate::ExprColumn, JsError> {
        use kentos_expression::{compile, rows};
        let e = compile(source).map_err(|e| JsError::new(&e.text()))?;
        let input = rows::RowsInput {
            n: ids.len(),
            texts,
            text_lens,
            numbers,
            measures: &[],
            scale,
        };
        let shape = |i: usize| {
            ids.get(i)
                .and_then(|&id| self.inner.get(id))
                .map(|it| &it.shape)
        };
        let c = rows::evaluate_rows_on(&e, &input, rows::As::from_code(want), shape)
            .map_err(|e| JsError::new(&e))?;
        Ok(crate::ExprColumn::from(c))
    }

    /// `evaluateExpression` in a context (docs/adr/0214): the schema as JSON
    /// (`{ variables, world }`), and the layers its calls to other objects
    /// look at, whose objects are in this store too.
    #[wasm_bindgen(js_name = evaluateExpressionIn)]
    #[allow(clippy::too_many_arguments)]
    pub fn evaluate_expression_in(
        &self,
        source: &str,
        context: &str,
        ids: &[f64],
        texts: &str,
        text_lens: &[i32],
        numbers: &[f64],
        scale: f64,
        want: u8,
        world: Option<crate::ExprWorld>,
    ) -> Result<crate::ExprColumn, JsError> {
        use kentos_expression::world::{World, WorldLayer};
        use kentos_expression::{api::schema_of, compile_with, rows};
        let schema = schema_of(context).map_err(|e| JsError::new(&e))?;
        let e = compile_with(source, &schema).map_err(|e| JsError::new(&e.text()))?;
        let input = rows::RowsInput {
            n: ids.len(),
            texts,
            text_lens,
            numbers,
            measures: &[],
            scale,
        };
        let shape = |i: usize| {
            ids.get(i)
                .and_then(|&id| self.inner.get(id))
                .map(|it| &it.shape)
        };
        let want = rows::As::from_code(want);
        let c = match world.filter(|_| e.looks_around()) {
            Some(w) => {
                let needs = e.world_needs();
                let table = rows::WorldTable::new(
                    &needs,
                    w.names,
                    &w.counts,
                    &w.ids,
                    &w.texts,
                    &w.text_lens,
                    &w.numbers,
                )
                .map_err(|e| JsError::new(&e))?;
                let layers = table.layers(Some(&self.inner));
                let world = World {
                    layers: layers
                        .iter()
                        .map(|(name, ids, objects)| WorldLayer {
                            name: name.clone(),
                            ids: ids.clone(),
                            objects,
                        })
                        .collect(),
                    store: Some(&self.inner),
                };
                rows::evaluate_rows_on_in(&e, &input, want, shape, Some(&world))
            }
            None => rows::evaluate_rows_on(&e, &input, want, shape),
        }
        .map_err(|e| JsError::new(&e))?;
        Ok(crate::ExprColumn::from(c))
    }

    /// A layer through the style engine (`kentos_style_core::style::build`):
    /// `objects` four numbers per id (how it is drawn, its set or symbol, the
    /// set of its simple look, its colour), `pieces` the sets of every
    /// insert's pieces in turn (docs/adr/0144), the program's table of values
    /// (`texts`, `text_lens`, `numbers`), the box construction lines are
    /// clipped to, the origin the batches are relative to, the plot scale,
    /// whether symbol sizes are on the screen (paper mm drawn as px), and
    /// what Görünüm kipleri show: the fills and the areas' edges (docs/adr/0195).
    #[wasm_bindgen(js_name = buildStyled)]
    #[allow(clippy::too_many_arguments)]
    pub fn build_styled(
        &self,
        program: &StyleProgram,
        ids: &[f64],
        objects: &[i32],
        pieces: &[i32],
        texts: &str,
        text_lens: &[i32],
        numbers: &[f64],
        has_clip: bool,
        min_x: f64,
        min_y: f64,
        max_x: f64,
        max_y: f64,
        origin_x: f64,
        origin_y: f64,
        plot_scale: f64,
        screen: bool,
        fills: bool,
        area_edges: bool,
        px_per_m: f64,
        picture: &str,
    ) -> Result<StyledBatches, JsError> {
        let clip = has_clip.then(|| rect(min_x, min_y, max_x, max_y));
        // A view-dependent renderer's view (docs/adr/0213 §3): its scale (0: none) and the key of its picture.
        let frame = (px_per_m > 0.0).then_some(ViewFrame { px_per_m, picture });
        let b = build_layer_in(
            &self.inner,
            &program.inner,
            &LayerObjects {
                ids,
                objects,
                texts,
                text_lens,
                numbers,
                pieces,
            },
            clip.as_ref(),
            Vec2::new(origin_x, origin_y),
            plot_scale,
            screen,
            View { fills, area_edges },
            frame,
        )
        .map_err(|e| JsError::new(&e))?;
        Ok(StyledBatches {
            json: b.json,
            data: b.data,
            pictures: b.pictures,
            dropped: b.dropped,
        })
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

    /// Pairs of input and reference objects that satisfy the relation
    /// (`Store::relate_pairs`; relation code is `Relation::from_code`'s):
    /// flat (input position, reference position) pairs, inputs first.
    #[wasm_bindgen(js_name = relatePairs)]
    pub fn relate_pairs(
        &self,
        inputs: &[f64],
        references: &[f64],
        relation: u32,
        within: f64,
    ) -> Vec<f64> {
        match Relation::from_code(relation) {
            Some(r) => self.inner.relate_pairs(inputs, references, r, within),
            None => Vec::new(),
        }
    }

    /// Each input's nearest targets (docs/adr/0215 §2.1; `Store::nearest`):
    /// `k` of them (0: all) within `max` (infinite: no bound), measured edge to
    /// edge (0) or centre to centre (1); eight numbers each: the input's place,
    /// the target's, the distance, the two nearest points and the bearing.
    pub fn nearest(
        &self,
        inputs: &[f64],
        targets: &[f64],
        k: u32,
        max: f64,
        measure: u32,
    ) -> Vec<f64> {
        match Measure::from_code(measure) {
            Some(m) => self.inner.nearest(inputs, targets, k as usize, max, m),
            None => Vec::new(),
        }
    }

    /// The areas' neighbours (docs/adr/0215 §2.2; `Store::neighbors`): five
    /// numbers each: the area's place, its neighbour's, the kind (0 edge,
    /// 1 corner, 2 overlap), the shared length and the overlapping area.
    pub fn neighbors(
        &self,
        ids: &[f64],
        tolerance: f64,
        corners: bool,
        overlaps: bool,
    ) -> Vec<f64> {
        self.inner.neighbors(ids, tolerance, corners, overlaps)
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

/// Acquired extensions from `snapEx`'s records.
fn read_extensions(f: &[f64]) -> Vec<Extension> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < f.len() {
        match f[i] {
            0.0 if i + 5 <= f.len() => {
                out.push(Extension::Line {
                    end: Vec2::new(f[i + 1], f[i + 2]),
                    dir: Vec2::new(f[i + 3], f[i + 4]),
                });
                i += 5;
            }
            1.0 if i + 6 <= f.len() => {
                out.push(Extension::Arc {
                    c: Vec2::new(f[i + 1], f[i + 2]),
                    r: f[i + 3],
                    a0: f[i + 4],
                    sweep: f[i + 5],
                });
                i += 6;
            }
            _ => break,
        }
    }
    out
}

/// The object being drawn as an open path (none without a point).
fn draft_path(xy: &[f64], bulges: &[f64]) -> Vec<Shape> {
    let pts: Vec<Vec2> = xy.chunks_exact(2).map(|p| Vec2::new(p[0], p[1])).collect();
    if pts.is_empty() {
        return Vec::new();
    }
    let bulges = (!bulges.is_empty()).then(|| bulges.to_vec());
    vec![Shape::Polyline {
        pts,
        bulges,
        holes: None,
        parts: None,
    }]
}
