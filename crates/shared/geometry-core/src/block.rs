//! Blocks (docs/adr/0144): a definition's objects drawn once and placed many
//! times. A definition is flattened once: its nested inserts are expanded by
//! their similarity, and a nested object without its own colour or line
//! weight takes the nested insert's (the DXF's BYBLOCK). Every insert then
//! transforms the flattened pieces by its own similarity: the definition's
//! base point goes to the insertion point, the pieces are mirrored in the
//! definition's x axis when the insert says so, scaled and turned about it.
//!
//! The pieces are kept relative to the base point, so a definition drawn at
//! world coordinates (a block made from a drawing's objects) loses nothing
//! to cancellation, and an exact quarter turn is exact: a 90°, 180° or 270°
//! insert of whole-numbered geometry stays whole-numbered.
//!
//! Attribute definitions (docs/adr/0144 §7) are text pieces that carry their
//! tag: an insert shows its own value under that tag, else the definition's
//! default, which is the piece's text; the host picks the value where it
//! draws, the place, size and turn are the core's. A nested insert's
//! attributes are the definition's own: their values are taken in when the
//! definition is flattened.
//!
//! The documents keep the block rules (known blocks, no cycles, at most 16
//! levels; `kentos_contracts::blocks`); the core stays total all the same:
//! an unknown block expands to nothing and a cycle stops where it closes.
//!
//! An insert's own y factor (docs/adr/0315 §3) makes its placement an affine:
//! its pieces are then warped as Vektör oturtma's affine warps them
//! (`ops::warp`): straight geometry exactly, a circle, an arc and an ellipse
//! the exact ellipse of their image, a path's arc segments straight vertices
//! within 0.1 mm.
//!
//! A piece's look (docs/adr/0315 §2): a part it has is its own; a part in its
//! `byLayer` follows a layer, its own, or for a piece on the block's layer
//! (`""`, the DXF's layer 0) the layer of the insert that draws it; any other
//! part is the insert's (BYBLOCK). A piece on a layer of its own shows only
//! while that layer does (`Piece::gates`), and a nested insert's pieces only
//! while its layer does.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::api::json::Json;
use crate::block_dynamic::Dynamic;
use crate::entity::{Attrs, Entity, Places, Shape, entity_bounds_in};
use crate::geom::affine::{Affine, apply, compose, translation};
use crate::ops::warp::{Warp, warp_shape};
use crate::geometry::{Bounds, empty_bounds, extend_bounds};
use crate::jsmath::{PI, atan2, cos, js_hypot, js_max, js_min, sin};
use crate::ops::curve_cuts::Cut;
use crate::ops::transform::transform_shape;
use crate::text::{Font, TextAlign};
use crate::vec2::Vec2;

/// Nested definitions followed at most this deep (`MAX_BLOCK_DEPTH` in the contracts).
pub const MAX_DEPTH: usize = 16;

/// An attribute definition (docs/adr/0144 §7): the text an insert shows of
/// its value under `tag`, in the definition's coordinates, as a text is.
#[derive(Clone, Debug, PartialEq)]
pub struct Attribute {
    pub tag: String,
    /// The default value, shown when the insert has none ("" : none).
    pub value: String,
    pub p: Vec2,
    pub height: f64,
    /// Degrees, counter-clockwise from east, as a text's.
    pub rotation: f64,
    /// Which point of the text `p` is, and its width factor, as a text's (docs/adr/0145).
    pub align: Option<TextAlign>,
    pub width_factor: Option<f64>,
    /// Not drawn (docs/adr/0317 §1): no piece, nothing on Patlat.
    pub invisible: bool,
    /// Its value is `value` in every insert: the insert's own is not shown.
    pub constant: bool,
    /// An insert may show it elsewhere (its `attributePlaces`).
    pub movable: bool,
    /// Its value's line breaks are lines; a box width wraps them.
    pub multiline: bool,
    pub box_width: Option<f64>,
}

impl Attribute {
    /// Its text showing `text`, in the definition's coordinates.
    pub fn shape(&self, text: String) -> Shape {
        self.shape_at(self.p, text)
    }

    /// The text a value shows (docs/adr/0317 §1): a single-line attribute's line breaks are spaces.
    pub fn shown(&self, value: &str) -> String {
        if self.multiline {
            value.to_owned()
        } else {
            value.replace("\r\n", " ").replace(['\n', '\r'], " ")
        }
    }

    /// Its text showing `text` at `p` of the definition's coordinates (a moved attribute's place).
    pub fn shape_at(&self, p: Vec2, text: String) -> Shape {
        let text = self.shown(&text);
        Shape::Text {
            p,
            text,
            height: self.height,
            rotation: self.rotation,
            align: self.align,
            width_factor: self.width_factor,
            mask: None,
            box_width: if self.multiline { self.box_width } else { None },
            line_spacing: None,
            runs: None,
            face: Default::default(),
            path: None,
        }
    }
}

/// An attribute piece's shape in its definition's coordinates relative to the base point (docs/adr/0317 §1): the
/// value the insert shows (`attrs`; a constant attribute's is the definition's), at the place the insert moved it to
/// (`places`; a movable attribute's only). Any other piece's shape as it is.
fn attribute_piece(flat: &Flat, piece: &Piece, attrs: Option<&Attrs>, places: Option<&Places>) -> Shape {
    let Some(tag) = &piece.attribute else {
        return piece.shape.clone();
    };
    let Some(a) = flat.attributes.iter().find(|a| &a.tag == tag) else {
        return piece.shape.clone();
    };
    let value = attrs.and_then(|v| v.shown(tag)).unwrap_or(&a.value);
    let at = places
        .and_then(|p| p.of(tag))
        .filter(|_| a.movable)
        .unwrap_or(a.p);
    transform_shape(
        &a.shape_at(at, value.to_owned()),
        &translation(-flat.base.x, -flat.base.y),
    )
}

/// A definition as a host gives it: its id, its base point, its objects
/// (each its geometry and every other field, as the drawing holds them) and
/// its attribute definitions.
#[derive(Clone, Debug, PartialEq)]
pub struct Definition {
    pub id: String,
    pub base: Vec2,
    pub entities: Vec<Entity>,
    pub attributes: Vec<Attribute>,
    /// Its parameters and actions (docs/adr/0318): an insert with values draws a variant of it.
    pub dynamic: Option<Arc<Dynamic>>,
}

/// One drawn piece of a block: a shape (never an insert), with the colour
/// and line weight it draws with when it has its own (or inherits a nested
/// insert's); `None` draws with the insert's, then its layer's. An
/// attribute's text carries its tag: the insert's value under it is shown,
/// else the text (the default).
#[derive(Clone, Debug, PartialEq)]
pub struct Piece {
    pub shape: Shape,
    pub color: Option<String>,
    pub line_weight: Option<f64>,
    /// The layers its parts follow rather than the insert (`byLayer`, docs/adr/0260 §1, 0315 §2): each a layer
    /// (`""`: the layer of the insert that draws it) and a mask of [`BY_LAYER_COLOR`] …, at most one entry a layer.
    pub by_layer: Vec<(String, u8)>,
    /// The layers it shows only while they show (docs/adr/0315 §2): its own, and those of the nested inserts it is
    /// drawn through; none on the block's layer.
    pub gates: Vec<String>,
    /// Its line type, its pattern's scale and its transparency: its own, or a nested insert's (BYBLOCK, docs/adr/0312
    /// §4); none of a part draws with the insert's, then its layer's.
    pub stroke: PieceStroke,
    pub attribute: Option<String>,
}

/// A piece's line type (its name in the contract, `dashed`), its pattern's scale and its transparency, each when it
/// has its own.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PieceStroke {
    pub line_type: Option<String>,
    /// Its own project line type by its name (docs/adr/0313 §1).
    pub pattern: Option<String>,
    pub scale: Option<f64>,
    pub transparency: Option<f64>,
}

impl PieceStroke {
    /// A definition's object's own parts.
    fn of(rest: &[(String, Json)]) -> Self {
        Self {
            line_type: text_field(rest, "lineType"),
            pattern: text_field(rest, "linePattern"),
            scale: number_field(rest, "lineTypeScale"),
            transparency: number_field(rest, "transparency"),
        }
    }
}

/// A definition ready to place: its base point, its own objects and
/// attribute definitions (what Patlat gives back) and its pieces relative
/// to the base point.
#[derive(Debug, PartialEq)]
pub struct Flat {
    pub base: Vec2,
    pub entities: Vec<Entity>,
    pub attributes: Vec<Attribute>,
    pub pieces: Vec<Piece>,
}

/// A drawing's definitions, flattened, by id; and a dynamic block's variants, made when an insert first draws them.
#[derive(Clone, Debug, Default)]
pub struct Blocks {
    by_id: HashMap<String, Arc<Flat>>,
    /// The dynamic definitions as given, for their variants.
    dynamic: Arc<HashMap<String, Arc<Definition>>>,
    /// Their variants by block id and values (`Params::key`); emptied when it grows past `MAX_VARIANTS`.
    variants: Arc<Mutex<HashMap<String, Arc<Flat>>>>,
}

/// The most variants kept at once: a drawing's dynamic inserts draw few distinct values.
const MAX_VARIANTS: usize = 4096;

impl Blocks {
    /// The definitions flattened, each once whatever uses it.
    pub fn new(defs: Vec<Definition>) -> Blocks {
        let sources: HashMap<&str, &Definition> = defs.iter().map(|d| (d.id.as_str(), d)).collect();
        let mut done: HashMap<String, Arc<Flat>> = HashMap::with_capacity(defs.len());
        let mut stack = Vec::new();
        for d in &defs {
            flatten(&d.id, &sources, &mut done, &mut stack);
        }
        let dynamic = defs
            .iter()
            .filter(|d| d.dynamic.is_some())
            .map(|d| (d.id.clone(), Arc::new(d.clone())))
            .collect();
        Blocks {
            by_id: done,
            dynamic: Arc::new(dynamic),
            variants: Arc::default(),
        }
    }

    /// The flat an insert draws: its definition's, or the variant its parameter values make (docs/adr/0318 §2).
    fn flat_for(&self, block: &str, params: Option<&crate::entity::Params>) -> Option<Arc<Flat>> {
        let flat = self.by_id.get(block)?;
        let Some(def) = self.dynamic.get(block) else {
            return Some(flat.clone());
        };
        let dynamic = def.dynamic.as_ref()?;
        if dynamic.is_default(params) {
            return Some(flat.clone());
        }
        let key = format!("{block}|{}", params.map(|p| p.key()).unwrap_or_default());
        if let Ok(cache) = self.variants.lock()
            && let Some(f) = cache.get(&key)
        {
            return Some(f.clone());
        }
        let varied = Definition {
            entities: dynamic.vary(&def.entities, params),
            ..(**def).clone()
        };
        let variant = variant_flat(&varied, &self.by_id, &self.dynamic);
        if let Ok(mut cache) = self.variants.lock() {
            if cache.len() >= MAX_VARIANTS {
                cache.clear();
            }
            cache.insert(key, variant.clone());
        }
        Some(variant)
    }

    /// The definitions read from their JSON: the contract's `BlockDefinition`
    /// list (`[{ id, base, entities, attributes?, … }]`; other fields are the documents').
    pub fn from_json(v: &Json) -> Result<Blocks, String> {
        let Json::Arr(list) = v else {
            return Err("blok tanımları bir dizi olmalı".into());
        };
        let mut defs = Vec::with_capacity(list.len());
        for (i, d) in list.iter().enumerate() {
            let Json::Str(id) = d.get("id") else {
                return Err(format!("[{i}].id: metin bekleniyordu"));
            };
            let base = crate::api::json::read_field::<Vec2>(d, "base")
                .map_err(|e| format!("[{i}].base: {e}"))?;
            let Json::Arr(entities) = d.get("entities") else {
                return Err(format!("[{i}].entities: dizi bekleniyordu"));
            };
            let entities = entities
                .iter()
                .enumerate()
                .map(|(k, e)| {
                    crate::api::json::FromJson::from_json(e)
                        .map_err(|m| format!("[{i}].entities[{k}]: {m}"))
                })
                .collect::<Result<Vec<Entity>, String>>()?;
            let attributes = match d.get("attributes") {
                Json::Arr(list) => list
                    .iter()
                    .enumerate()
                    .map(|(k, a)| {
                        attribute_of(a).map_err(|m| format!("[{i}].attributes[{k}]: {m}"))
                    })
                    .collect::<Result<Vec<Attribute>, String>>()?,
                _ => Vec::new(),
            };
            let dynamic = match d.get("dynamic") {
                Json::Obj(_) => Some(Arc::new(Dynamic::from_json(d.get("dynamic")))),
                _ => None,
            };
            defs.push(Definition {
                id: id.clone(),
                base,
                entities,
                attributes,
                dynamic,
            });
        }
        Ok(Blocks::new(defs))
    }

    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }

    /// A definition by id.
    pub fn get(&self, id: &str) -> Option<&Arc<Flat>> {
        self.by_id.get(id)
    }

    /// An insert's pieces in the drawing's coordinates, an attribute's text
    /// the value the insert shows (an empty text when it shows none); none
    /// for an unknown block or a shape that is not an insert.
    pub fn expand(&self, insert: &Shape) -> Vec<Piece> {
        let Some((flat, m, similar)) = self.placed(insert) else {
            return Vec::new();
        };
        let (values, places) = insert_attrs(insert);
        flat.pieces
            .iter()
            .map(|piece| {
                let shape = place(&attribute_piece(&flat, piece, values, places), &m, similar);
                Piece {
                    shape,
                    color: piece.color.clone(),
                    line_weight: piece.line_weight,
                    by_layer: piece.by_layer.clone(),
                    gates: piece.gates.clone(),
                    stroke: piece.stroke.clone(),
                    attribute: piece.attribute.clone(),
                }
            })
            .collect()
    }

    /// The object a block path names inside an insert (docs/adr/0263 §3), placed in the drawing: each number its
    /// object's place in that level's definition (`entities`), every one but the last an insert; none when the path
    /// is empty, deeper than [`MAX_DEPTH`], leaves a definition or ends on an insert.
    pub fn piece(&self, insert: &Shape, path: &[u32]) -> Option<Shape> {
        if path.is_empty() || path.len() > MAX_DEPTH {
            return None;
        }
        let mut cur = insert.clone();
        let mut m: Affine = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
        let mut similar = true;
        for (depth, &i) in path.iter().enumerate() {
            let (flat, placed, alike) = self.placed(&cur)?;
            similar &= alike;
            m = compose(
                &m,
                &compose(&placed, &translation(-flat.base.x, -flat.base.y)),
            );
            let e = flat.entities.get(i as usize)?;
            let inner = matches!(e.shape, Shape::Insert { .. });
            if depth + 1 < path.len() {
                if !inner {
                    return None;
                }
                cur = e.shape.clone();
            } else {
                return (!inner).then(|| place(&e.shape, &m, similar));
            }
        }
        None
    }

    /// A block path through an insert under these definitions, made a path through `after_insert` under `after`
    /// (docs/adr/0315 §1): each step's object found again by its local id, which a definition keeps through its
    /// edits; a step whose id is not once in both definitions keeps its place. None when a step leaves either.
    pub fn path_in(
        &self,
        insert: &Shape,
        after: &Blocks,
        after_insert: &Shape,
        path: &[u32],
    ) -> Option<Vec<u32>> {
        let local = |e: &Entity| number_field(&e.rest, "id");
        let once = |list: &[Entity], id: f64| {
            let mut found = list.iter().enumerate().filter(|(_, e)| local(e) == Some(id));
            match (found.next(), found.next()) {
                (Some((i, _)), None) => Some(i),
                _ => None,
            }
        };
        let (mut was, mut now) = (insert.clone(), after_insert.clone());
        let mut out = Vec::with_capacity(path.len());
        for &i in path {
            let (old, _, _) = self.placed(&was)?;
            let (new, _, _) = after.placed(&now)?;
            let e = old.entities.get(i as usize)?;
            let j = local(e)
                .filter(|&id| once(&old.entities, id) == Some(i as usize))
                .and_then(|id| once(&new.entities, id))
                .unwrap_or(i as usize);
            let next = new.entities.get(j)?;
            out.push(j as u32);
            was = e.shape.clone();
            now = next.shape.clone();
        }
        Some(out)
    }

    /// Every block path of an insert's objects (docs/adr/0263 §3), in the order the flattening places them (a nested
    /// insert's objects where it stands); an unknown block has none.
    pub fn piece_paths(&self, insert: &Shape) -> Vec<Vec<u32>> {
        fn walk(blocks: &Blocks, shape: &Shape, prefix: &mut Vec<u32>, out: &mut Vec<Vec<u32>>) {
            let Some((flat, _, _)) = blocks.placed(shape) else {
                return;
            };
            if prefix.len() >= MAX_DEPTH {
                return;
            }
            for (i, e) in flat.entities.iter().enumerate() {
                prefix.push(i as u32);
                if matches!(e.shape, Shape::Insert { .. }) {
                    walk(blocks, &e.shape, prefix, out);
                } else {
                    out.push(prefix.clone());
                }
                prefix.pop();
            }
        }
        let mut out = Vec::new();
        walk(self, insert, &mut Vec::new(), &mut out);
        out
    }

    /// An insert's box (docs/adr/0186 §4): its pieces' box in its
    /// definition's frame, placed as the insert places them; none for an
    /// unknown block or one without pieces.
    pub fn outline(&self, insert: &Shape, font: Font) -> Option<Vec<Vec2>> {
        let (flat, m, _) = self.placed(insert)?;
        let mut b = empty_bounds();
        for piece in &flat.pieces {
            let pb = entity_bounds_in(&piece.shape, font);
            if pb.min_x <= pb.max_x && pb.min_y <= pb.max_y {
                for q in [Vec2::new(pb.min_x, pb.min_y), Vec2::new(pb.max_x, pb.max_y)] {
                    extend_bounds(&mut b, q, 0.0);
                }
            }
        }
        if !(b.min_x <= b.max_x && b.min_y <= b.max_y) {
            return None;
        }
        Some(
            [
                Vec2::new(b.min_x, b.min_y),
                Vec2::new(b.max_x, b.min_y),
                Vec2::new(b.max_x, b.max_y),
                Vec2::new(b.min_x, b.max_y),
            ]
            .into_iter()
            .map(|q| apply(&m, q))
            .collect(),
        )
    }

    /// An insert's definition, its placement from the base-relative pieces to the drawing, and whether that is a
    /// similarity (no y factor of its own).
    fn placed(&self, insert: &Shape) -> Option<(Arc<Flat>, Affine, bool)> {
        let Shape::Insert {
            block,
            p,
            scale,
            y_scale,
            rotation,
            mirror,
            params,
            ..
        } = insert
        else {
            return None;
        };
        let flat = self.flat_for(block, params.as_ref())?;
        let sy = y_scale.unwrap_or(*scale);
        Some((
            flat,
            placement_xy(*p, *scale, sy, *rotation, mirror.unwrap_or(false)),
            uniform(*scale, *y_scale),
        ))
    }

    /// Patlat (docs/adr/0144 §3): the insert's definition one level open,
    /// its objects placed as the insert places them; a nested insert stays
    /// an insert, its own similarity composed. An object keeps its own
    /// fields; one without a colour or line weight takes the insert's, one
    /// without a layer (`""`, the block's) the insert's layer. The
    /// definition's attributes become texts of the values the insert shows
    /// (§7; an empty one none). The objects come without ids (they are new
    /// objects).
    pub fn explode(&self, insert: &Entity) -> Cut {
        let Some((flat, m, similar)) = self.placed(&insert.shape) else {
            return Cut::Error("Yerleştirilen blok çizimde tanımlı değil; patlatılamadı.".into());
        };
        let (values, places) = insert_attrs(&insert.shape);
        // The attributes that show (docs/adr/0317 §1): an invisible one gives nothing, a constant one its own value,
        // a moved one stands where the insert put it.
        let shown: Vec<(&Attribute, Vec2, String)> = flat
            .attributes
            .iter()
            .filter(|a| !a.invisible)
            .filter_map(|a| {
                let v = if a.constant {
                    &a.value
                } else {
                    values.and_then(|v| v.shown(&a.tag)).unwrap_or(&a.value)
                };
                let at = places
                    .and_then(|p| p.of(&a.tag))
                    .filter(|_| a.movable)
                    .unwrap_or(a.p);
                (!v.is_empty()).then(|| (a, at, v.to_owned()))
            })
            .collect();
        if flat.entities.is_empty() && shown.is_empty() {
            return Cut::Error("Blokta patlatılacak nesne yok.".into());
        }
        // The definition's objects are in its own coordinates: to the base point first.
        let m = compose(&m, &translation(-flat.base.x, -flat.base.y));
        let field = |name: &str| {
            insert
                .rest
                .iter()
                .find(|(k, _)| k == name)
                .map(|(_, v)| v.clone())
        };
        let (color, weight, layer) = (field("color"), field("lineWeight"), field("layerId"));
        // Under a y factor a nested insert stays one only while its axes stay square (docs/adr/0315 §3).
        let mut nested = Vec::with_capacity(flat.entities.len());
        for e in &flat.entities {
            nested.push(if similar || !matches!(e.shape, Shape::Insert { .. }) {
                place(&e.shape, &m, similar)
            } else {
                match insert_through(&e.shape, &m) {
                    Some(s) => s,
                    None => {
                        return Cut::Error(
                            "Eşit olmayan ölçekli bu yerleştirmenin içindeki döndürülmüş blok, eksenleri eğileceği için blok olarak kalamaz; önce içteki bloğu patlatın ya da ölçekleri eşitleyin.".into(),
                        );
                    }
                }
            });
        }
        let pieces = flat
            .entities
            .iter()
            .zip(nested)
            .map(|(e, placed)| {
                let mut rest: Vec<(String, Json)> = e
                    .rest
                    .iter()
                    .filter(|(k, _)| k != "id" && k != "uid")
                    .cloned()
                    .collect();
                let has = |rest: &[(String, Json)], k: &str| rest.iter().any(|(n, _)| n == k);
                for (key, value) in [("color", &color), ("lineWeight", &weight)] {
                    if let Some(v) = value
                        && !has(&rest, key)
                    {
                        rest.push((key.to_owned(), v.clone()));
                    }
                }
                let blank = rest
                    .iter()
                    .find(|(k, _)| k == "layerId")
                    .is_none_or(|(_, v)| matches!(v, Json::Str(s) if s.is_empty()));
                if blank && let Some(l) = &layer {
                    rest.retain(|(k, _)| k != "layerId");
                    rest.push(("layerId".into(), l.clone()));
                }
                // The parts that followed the layer now are their layer's own: an object of the drawing has no
                // `byLayer` (docs/adr/0315 §2).
                rest.retain(|(k, _)| k != "byLayer");
                Entity {
                    shape: placed,
                    rest,
                }
            })
            .chain(shown.into_iter().map(|(a, at, text)| {
                // An attribute's text on the insert's layer, in its colour and line weight.
                let mut rest = vec![("attrs".to_owned(), Json::Obj(Vec::new()))];
                for (key, value) in [
                    ("layerId", &layer),
                    ("color", &color),
                    ("lineWeight", &weight),
                ] {
                    if let Some(v) = value {
                        rest.push((key.to_owned(), v.clone()));
                    }
                }
                Entity {
                    shape: place(&a.shape_at(at, text), &m, similar),
                    rest,
                }
            }))
            .collect();
        Cut::Pieces(pieces)
    }
}

/// An attribute definition from its JSON (the contract's `AttributeDefinition`).
fn attribute_of(v: &Json) -> Result<Attribute, String> {
    let Json::Str(tag) = v.get("tag") else {
        return Err("tag: metin bekleniyordu".into());
    };
    let value = match v.get("value") {
        Json::Str(s) => s.clone(),
        _ => String::new(),
    };
    let p = crate::api::json::read_field::<Vec2>(v, "p").map_err(|e| format!("p: {e}"))?;
    let Json::Num(height) = v.get("height") else {
        return Err("height: sayı bekleniyordu".into());
    };
    let Json::Num(rotation) = v.get("rotation") else {
        return Err("rotation: sayı bekleniyordu".into());
    };
    Ok(Attribute {
        tag: tag.clone(),
        value,
        p,
        height: *height,
        rotation: *rotation,
        align: crate::api::json::read_field(v, "align")?,
        width_factor: crate::api::json::read_field(v, "widthFactor")?,
        invisible: matches!(v.get("invisible"), Json::Bool(true)),
        constant: matches!(v.get("constant"), Json::Bool(true)),
        movable: matches!(v.get("movable"), Json::Bool(true)),
        multiline: matches!(v.get("multiline"), Json::Bool(true)),
        box_width: crate::api::json::read_field(v, "boxWidth")?,
    })
}

/// Whether a piece is an attribute's text that shows nothing (§7: neither
/// the insert's value nor a default): it keeps its place among the pieces,
/// but draws, picks, snaps and measures nothing.
pub fn shows_nothing(piece: &Shape) -> bool {
    matches!(piece, Shape::Text { text, .. } if text.is_empty())
}

/// An insert's box with its placed pieces, as the geometry store keeps it:
/// `own` (the insertion point's box) widened by the box of every piece that
/// shows something, in the drawing's typeface (Hizala ve dağıt measures
/// with it too, docs/adr/0194 §2).
pub fn pieces_bounds(own: Bounds, pieces: &[Piece], font: Font) -> Bounds {
    let mut b = own;
    for piece in pieces.iter().filter(|p| !shows_nothing(&p.shape)) {
        let pb = entity_bounds_in(&piece.shape, font);
        b.min_x = js_min(b.min_x, pb.min_x);
        b.min_y = js_min(b.min_y, pb.min_y);
        b.max_x = js_max(b.max_x, pb.max_x);
        b.max_y = js_max(b.max_y, pb.max_y);
    }
    b
}

/// An insert's attributes (docs/adr/0144 §7) and where it moved them (docs/adr/0317 §1).
fn insert_attrs(shape: &Shape) -> (Option<&Attrs>, Option<&Places>) {
    match shape {
        Shape::Insert { attrs, places, .. } => (attrs.as_ref(), places.as_ref()),
        _ => (None, None),
    }
}

/// The similarity an insert places its base-relative pieces with:
/// x' = p + scale·R(rotation)·M·x, where M mirrors in the x axis when
/// `mirror`. A rotation that is exactly the double of a quarter turn
/// (0, π/2, π, 3π/2 and their negatives) has an exact sine and cosine.
pub fn placement(p: Vec2, scale: f64, rotation: f64, mirror: bool) -> Affine {
    placement_xy(p, scale, scale, rotation, mirror)
}

/// [`placement`] with its own y factor (docs/adr/0315 §3): x' = p + R(rotation)·S·M·x, S = diag(sx, sy). With
/// `sx == sy` it is [`placement`]'s similarity, bit for bit.
pub fn placement_xy(p: Vec2, sx: f64, sy: f64, rotation: f64, mirror: bool) -> Affine {
    let (s, c) = quarter_turn(rotation).unwrap_or_else(|| (sin(rotation), cos(rotation)));
    let k = if mirror { -1.0 } else { 1.0 };
    [sx * c, sx * s, -sy * k * s, sy * k * c, p.x, p.y]
}

/// Whether an insert's factors are one (no y factor of its own, or the same): its placement a similarity.
pub fn uniform(scale: f64, y_scale: Option<f64>) -> bool {
    y_scale.is_none_or(|y| y == scale)
}

/// A shape placed by `m`: moved, turned and scaled when `m` is a similarity (`similar`), else warped as an affine
/// (docs/adr/0315 §3): curves the exact ellipse of their image, a path's arcs straight vertices within 0.1 mm.
pub fn place(shape: &Shape, m: &Affine, similar: bool) -> Shape {
    if similar {
        return transform_shape(shape, m);
    }
    let warp = Warp::Affine {
        from: Vec2::new(0.0, 0.0),
        to: Vec2::new(m[4], m[5]),
        m: [m[0], m[1], m[2], m[3]],
    };
    match warp_shape(shape, &[], &warp) {
        Ok(w) => w.shape,
        // An affine has no horizon; the similarity's own way all the same.
        Err(_) => transform_shape(shape, m),
    }
}

/// A nested insert placed by an affine `m` (docs/adr/0315 §3): its point moved, its own placement composed with
/// `m` and read back as a turn, two factors and a mirror; none when the composition shears (its axes are no
/// longer square) and no insert can hold it.
pub fn insert_through(shape: &Shape, m: &Affine) -> Option<Shape> {
    let Shape::Insert {
        params,
        block,
        p,
        scale,
        y_scale,
        rotation,
        mirror,
        attrs,
        places,
    } = shape
    else {
        return None;
    };
    let own = placement_xy(
        Vec2::new(0.0, 0.0),
        *scale,
        y_scale.unwrap_or(*scale),
        *rotation,
        mirror.unwrap_or(false),
    );
    let l = compose(m, &own);
    let (u, v) = (Vec2::new(l[0], l[1]), Vec2::new(l[2], l[3]));
    let (su, sv) = (js_hypot(u.x, u.y), js_hypot(v.x, v.y));
    if !(su > 0.0 && sv > 0.0) || (u.x * v.x + u.y * v.y).abs() > 1e-12 * su * sv {
        return None;
    }
    let flip = u.x * v.y - u.y * v.x < 0.0;
    let sy = if (su - sv).abs() <= 1e-15 * su { su } else { sv };
    Some(Shape::Insert {
        params: params.clone(),
        block: block.clone(),
        p: apply(m, *p),
        scale: su,
        y_scale: (sy != su).then_some(sy),
        rotation: atan2(u.y, u.x),
        mirror: flip.then_some(true),
        attrs: attrs.clone(),
        places: places.clone(),
    })
}

/// The whole matrix of an insert of a definition whose base point is `base`
/// (`placement` after moving the base point to the origin).
pub fn insert_matrix(base: Vec2, p: Vec2, scale: f64, rotation: f64, mirror: bool) -> Affine {
    compose(
        &placement(p, scale, rotation, mirror),
        &translation(-base.x, -base.y),
    )
}

/// [`insert_matrix`] with an own y factor (docs/adr/0315 §3).
pub fn insert_matrix_xy(
    base: Vec2,
    p: Vec2,
    (sx, sy): (f64, f64),
    rotation: f64,
    mirror: bool,
) -> Affine {
    compose(
        &placement_xy(p, sx, sy, rotation, mirror),
        &translation(-base.x, -base.y),
    )
}

/// Where a drawing point falls in a definition's own coordinates for an
/// insert of it: `insert_matrix` undone. The Bloklar panel's new base point
/// is shown on an insert and taken back into the definition. A scale of
/// zero (or one not finite) has no inverse: `None`. Quarter turns are exact,
/// as `placement` makes them.
pub fn insert_local(
    base: Vec2,
    p: Vec2,
    scale: f64,
    rotation: f64,
    mirror: bool,
    at: Vec2,
) -> Option<Vec2> {
    insert_local_xy(base, p, (scale, scale), rotation, mirror, at)
}

/// [`insert_local`] with an own y factor (docs/adr/0315 §3).
pub fn insert_local_xy(
    base: Vec2,
    p: Vec2,
    (sx, sy): (f64, f64),
    rotation: f64,
    mirror: bool,
    at: Vec2,
) -> Option<Vec2> {
    if !(sx.is_finite() && sx != 0.0 && sy.is_finite() && sy != 0.0) {
        return None;
    }
    let (s, c) = quarter_turn(rotation).unwrap_or_else(|| (sin(rotation), cos(rotation)));
    // R⁻¹ is R's transpose, then S⁻¹; M is its own inverse. One factor divides first, as it always did.
    let (x, y) = if sx == sy {
        let (dx, dy) = ((at.x - p.x) / sx, (at.y - p.y) / sx);
        (c * dx + s * dy, -s * dx + c * dy)
    } else {
        let (dx, dy) = (at.x - p.x, at.y - p.y);
        ((c * dx + s * dy) / sx, (-s * dx + c * dy) / sy)
    };
    let k = if mirror { -1.0 } else { 1.0 };
    Some(Vec2::new(base.x + x, base.y + k * y))
}

/// `(sin, cos)` of an exact quarter turn, as its double is written.
fn quarter_turn(a: f64) -> Option<(f64, f64)> {
    const HALF: f64 = PI / 2.0;
    const THREE_HALVES: f64 = 3.0 * (PI / 2.0);
    if a == 0.0 {
        Some((0.0, 1.0))
    } else if a == HALF || a == -THREE_HALVES {
        Some((1.0, 0.0))
    } else if a == PI || a == -PI {
        Some((0.0, -1.0))
    } else if a == THREE_HALVES || a == -HALF {
        Some((-1.0, 0.0))
    } else {
        None
    }
}

/// Flattens `id` into `done` (and every definition it uses first).
fn flatten(
    id: &str,
    sources: &HashMap<&str, &Definition>,
    done: &mut HashMap<String, Arc<Flat>>,
    stack: &mut Vec<String>,
) {
    if done.contains_key(id) {
        return;
    }
    let Some(def) = sources.get(id) else { return };
    if let Some(flat) = flat_of(def, sources, done, stack) {
        done.insert(id.to_owned(), flat);
    }
}

/// A dynamic definition's variant, its objects already varied (docs/adr/0318 §2): flattened over the drawing's
/// flats, nested dynamic inserts drawing their own variants.
fn variant_flat(
    def: &Definition,
    flats: &HashMap<String, Arc<Flat>>,
    dynamic: &HashMap<String, Arc<Definition>>,
) -> Arc<Flat> {
    let sources: HashMap<&str, &Definition> =
        dynamic.iter().map(|(k, d)| (k.as_str(), &**d)).collect();
    let mut done = flats.clone();
    flat_of(def, &sources, &mut done, &mut Vec::new()).unwrap_or_else(|| {
        Arc::new(Flat {
            base: def.base,
            entities: Vec::new(),
            attributes: Vec::new(),
            pieces: Vec::new(),
        })
    })
}

/// A definition flattened (every definition it uses flattened into `done` first); none for a cycle or past the depth.
fn flat_of(
    def: &Definition,
    sources: &HashMap<&str, &Definition>,
    done: &mut HashMap<String, Arc<Flat>>,
    stack: &mut Vec<String>,
) -> Option<Arc<Flat>> {
    let id = def.id.as_str();
    // A cycle, or deeper than the documents allow: nothing more below here.
    if stack.iter().any(|s| s == id) || stack.len() >= MAX_DEPTH {
        return None;
    }
    stack.push(id.to_owned());
    let to_base = translation(-def.base.x, -def.base.y);
    let mut pieces = Vec::new();
    for e in &def.entities {
        let own_color = text_field(&e.rest, "color");
        let own_weight = number_field(&e.rest, "lineWeight");
        let own_stroke = PieceStroke::of(&e.rest);
        let own_layer = text_field(&e.rest, "layerId").unwrap_or_default();
        let own_follows = by_layer(&e.rest);
        if let Shape::Insert {
            params,
            block,
            p,
            scale,
            y_scale,
            rotation,
            mirror,
            attrs,
            places,
        } = &e.shape
        {
            flatten(block, sources, done, stack);
            // A nested dynamic insert draws its own variant (docs/adr/0318 §2).
            let varied = sources
                .get(block.as_str())
                .and_then(|d| d.dynamic.as_ref().map(|dy| (d, dy)))
                .filter(|(_, dy)| !dy.is_default(params.as_ref()))
                .and_then(|(d, dy)| {
                    let varied = Definition {
                        entities: dy.vary(&d.entities, params.as_ref()),
                        ..(**d).clone()
                    };
                    flat_of(&varied, sources, done, stack)
                });
            let Some(inner) = varied.or_else(|| done.get(block.as_str()).cloned()) else {
                continue;
            };
            // The nested pieces: placed in this definition, then relative to its base point.
            let m = compose(
                &to_base,
                &placement_xy(
                    *p,
                    *scale,
                    y_scale.unwrap_or(*scale),
                    *rotation,
                    mirror.unwrap_or(false),
                ),
            );
            let similar = uniform(*scale, *y_scale);
            // What the nested insert's own `byLayer` parts follow: its layer (docs/adr/0315 §2).
            let insert_mask = own_follows.as_ref().map_or(0, |(_, m)| *m);
            for piece in &inner.pieces {
                // A nested insert's attribute shows its value where it put it, fixed in this definition.
                let shape = place(
                    &attribute_piece(&inner, piece, attrs.as_ref(), places.as_ref()),
                    &m,
                    similar,
                );
                if matches!(&shape, Shape::Text { text, .. } if text.is_empty()) {
                    continue;
                }
                // A part that follows a layer takes nothing from the nested insert.
                let follows =
                    |bit: u8| piece.by_layer.iter().any(|(_, m)| m & bit != 0);
                // A part the piece takes from the nested insert (BYBLOCK) and the insert has none of its own:
                // the insert's own `byLayer` part follows the insert's layer.
                let mut via = 0u8;
                let mut from_insert = |bit: u8, has: bool| {
                    if !follows(bit) && !has && insert_mask & bit != 0 {
                        via |= bit;
                    }
                };
                from_insert(BY_LAYER_COLOR, piece.color.is_some() || own_color.is_some());
                from_insert(
                    BY_LAYER_WEIGHT,
                    piece.line_weight.is_some() || own_weight.is_some(),
                );
                from_insert(
                    BY_LAYER_LINE_TYPE,
                    piece.stroke.line_type.is_some()
                        || piece.stroke.pattern.is_some()
                        || own_stroke.line_type.is_some()
                        || own_stroke.pattern.is_some(),
                );
                from_insert(
                    BY_LAYER_TRANSPARENCY,
                    piece.stroke.transparency.is_some() || own_stroke.transparency.is_some(),
                );
                // The block's layer of the nested definition is the nested insert's layer here.
                let mut layers: Vec<(String, u8)> = Vec::with_capacity(2);
                for (layer, mask) in piece.by_layer.iter().cloned().chain(
                    (via != 0).then(|| (String::new(), via)),
                ) {
                    let layer = if layer.is_empty() { own_layer.clone() } else { layer };
                    match layers.iter_mut().find(|(l, _)| *l == layer) {
                        Some((_, m)) => *m |= mask,
                        None => layers.push((layer, mask)),
                    }
                }
                let mut gates = piece.gates.clone();
                if !own_layer.is_empty() && !gates.contains(&own_layer) {
                    gates.push(own_layer.clone());
                }
                pieces.push(Piece {
                    shape,
                    color: piece.color.clone().or_else(|| {
                        (!follows(BY_LAYER_COLOR))
                            .then(|| own_color.clone())
                            .flatten()
                    }),
                    line_weight: piece
                        .line_weight
                        .or_else(|| (!follows(BY_LAYER_WEIGHT)).then_some(own_weight).flatten()),
                    by_layer: layers,
                    gates,
                    // The nested insert's line type, scale and transparency, as its colour (BYBLOCK).
                    stroke: PieceStroke {
                        line_type: piece.stroke.line_type.clone().or_else(|| {
                            (!follows(BY_LAYER_LINE_TYPE))
                                .then(|| own_stroke.line_type.clone())
                                .flatten()
                        }),
                        // A pattern as its line type: the piece's own, else the nested insert's when the piece has
                        // no type of its own.
                        pattern: piece.stroke.pattern.clone().or_else(|| {
                            (!follows(BY_LAYER_LINE_TYPE) && piece.stroke.line_type.is_none())
                                .then(|| own_stroke.pattern.clone())
                                .flatten()
                        }),
                        scale: piece.stroke.scale.or(own_stroke.scale),
                        transparency: piece.stroke.transparency.or_else(|| {
                            (!follows(BY_LAYER_TRANSPARENCY))
                                .then_some(own_stroke.transparency)
                                .flatten()
                        }),
                    },
                    attribute: None,
                });
            }
        } else {
            pieces.push(Piece {
                shape: transform_shape(&e.shape, &to_base),
                color: own_color,
                line_weight: own_weight,
                gates: if own_layer.is_empty() {
                    Vec::new()
                } else {
                    vec![own_layer]
                },
                by_layer: own_follows.into_iter().collect(),
                stroke: own_stroke,
                attribute: None,
            });
        }
    }
    // Its own attributes: texts of the default, the tag telling the host to show the insert's value; an invisible
    // one draws nothing, a constant one is its definition's text alone (docs/adr/0317 §1).
    for a in def.attributes.iter().filter(|a| !a.invisible) {
        pieces.push(Piece {
            shape: transform_shape(&a.shape(a.value.clone()), &to_base),
            color: None,
            line_weight: None,
            by_layer: Vec::new(),
            gates: Vec::new(),
            stroke: PieceStroke::default(),
            attribute: (!a.constant).then(|| a.tag.clone()),
        });
    }
    stack.pop();
    Some(Arc::new(Flat {
        base: def.base,
        entities: def.entities.clone(),
        attributes: def.attributes.clone(),
        pieces,
    }))
}

/// A piece's colour follows its own layer (docs/adr/0260 §1).
pub const BY_LAYER_COLOR: u8 = 1;
/// Its line type does.
pub const BY_LAYER_LINE_TYPE: u8 = 2;
/// Its line weight does.
pub const BY_LAYER_WEIGHT: u8 = 4;
/// Its transparency does.
pub const BY_LAYER_TRANSPARENCY: u8 = 8;

/// A definition's object's own layer and the mask of its `byLayer` parts; none without them.
fn by_layer(rest: &[(String, Json)]) -> Option<(String, u8)> {
    let parts = rest.iter().find_map(|(k, v)| match v {
        Json::Arr(list) if k == "byLayer" => Some(list),
        _ => None,
    })?;
    let mask = parts.iter().fold(0, |m, p| {
        m | match p {
            Json::Str(s) if s == "color" => BY_LAYER_COLOR,
            Json::Str(s) if s == "lineType" => BY_LAYER_LINE_TYPE,
            Json::Str(s) if s == "lineWeight" => BY_LAYER_WEIGHT,
            Json::Str(s) if s == "transparency" => BY_LAYER_TRANSPARENCY,
            _ => 0,
        }
    });
    (mask != 0).then(|| (text_field(rest, "layerId").unwrap_or_default(), mask))
}

fn text_field(rest: &[(String, Json)], name: &str) -> Option<String> {
    rest.iter().find_map(|(k, v)| match v {
        Json::Str(s) if k == name => Some(s.clone()),
        _ => None,
    })
}

fn number_field(rest: &[(String, Json)], name: &str) -> Option<f64> {
    rest.iter().find_map(|(k, v)| match v {
        Json::Num(x) if k == name => Some(*x),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::json::FromJson;

    fn entity(text: &str) -> Entity {
        Entity::from_json(&Json::parse(text).expect("JSON")).expect("an entity")
    }

    const ROGAR: &str = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0002";
    const DIREK: &str = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0001";

    /// Rögar: a 2×1 rectangle and a red line, base (1, 0); Direk: a point and
    /// Rögar turned a quarter, base (0, 0).
    fn blocks() -> Blocks {
        Blocks::new(vec![
            Definition {
                dynamic: None,
                id: DIREK.into(),
                base: Vec2::new(0.0, 0.0),
                entities: vec![
                    entity(r#"{"kind":"point","id":1,"layerId":"","attrs":{},"p":{"x":0,"y":5}}"#),
                    entity(&format!(
                        r##"{{"kind":"insert","id":2,"layerId":"","attrs":{{}},"color":"#00FF00","block":"{ROGAR}","p":{{"x":10,"y":0}},"scale":1,"rotation":{}}}"##,
                        PI / 2.0
                    )),
                ],
                attributes: Vec::new(),
            },
            Definition {
                dynamic: None,
                id: ROGAR.into(),
                base: Vec2::new(1.0, 0.0),
                entities: vec![
                    entity(
                        r#"{"kind":"polygon","id":1,"layerId":"0","attrs":{},"pts":[{"x":1,"y":0},{"x":3,"y":0},{"x":3,"y":1},{"x":1,"y":1}]}"#,
                    ),
                    entity(
                        r##"{"kind":"line","id":2,"layerId":"","color":"#FF0000","attrs":{},"a":{"x":1,"y":0},"b":{"x":3,"y":1}}"##,
                    ),
                ],
                attributes: Vec::new(),
            },
        ])
    }

    fn insert(block: &str, p: (f64, f64), scale: f64, rotation: f64, mirror: bool) -> Shape {
        Shape::Insert {
            params: None,
            block: block.into(),
            p: Vec2::new(p.0, p.1),
            scale,
            y_scale: None,
            rotation,
            mirror: mirror.then_some(true),
            attrs: None,
            places: None,
        }
    }

    fn corners(s: &Shape) -> Vec<(f64, f64)> {
        match s {
            Shape::Polygon { pts, .. } => pts.iter().map(|q| (q.x, q.y)).collect(),
            Shape::Line { a, b } => vec![(a.x, a.y), (b.x, b.y)],
            Shape::Point { p, .. } => vec![(p.x, p.y)],
            other => panic!("{other:?}"),
        }
    }

    /// Worked by hand: Rögar at (100, 200), a quarter turn, twice its size.
    /// Relative to the base (1, 0) the rectangle is (0,0) (2,0) (2,1) (0,1);
    /// doubled (0,0) (4,0) (4,2) (0,2); turned (x, y) → (−y, x): (0,0) (0,4)
    /// (−2,4) (−2,0); moved to (100, 200). Whole numbers stay whole.
    #[test]
    fn a_quarter_turn_keeps_whole_numbers() {
        let b = blocks();
        let pieces = b.expand(&insert(ROGAR, (100.0, 200.0), 2.0, PI / 2.0, false));
        assert_eq!(pieces.len(), 2);
        assert_eq!(
            corners(&pieces[0].shape),
            [(100.0, 200.0), (100.0, 204.0), (98.0, 204.0), (98.0, 200.0)]
        );
        assert_eq!(corners(&pieces[1].shape), [(100.0, 200.0), (98.0, 204.0)]);
        assert_eq!(pieces[0].color, None);
        assert_eq!(pieces[1].color.as_deref(), Some("#FF0000"));
    }

    /// A door: its leaf, its swing arc and its number attribute, base (0, 0). Mirrored in the definition's x
    /// axis the swing turns over; the number reads upright (docs/adr/0292): its up turned back, top for
    /// baseline, its value the insert's. Mirrored and turned half a turn (a mirror in the y axis), its baseline
    /// turns back: right for left, still at 0°.
    #[test]
    fn a_mirrored_door_keeps_its_number_readable() {
        const DOOR: &str = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0003";
        let blocks = Blocks::new(vec![Definition {
            dynamic: None,
            id: DOOR.into(),
            base: Vec2::new(0.0, 0.0),
            entities: vec![
                entity(r#"{"kind":"line","id":1,"layerId":"","attrs":{},"a":{"x":0,"y":0},"b":{"x":0,"y":0.9}}"#),
                entity(&format!(
                    r#"{{"kind":"arc","id":2,"layerId":"","attrs":{{}},"c":{{"x":0,"y":0}},"r":0.9,"a0":0,"a1":{}}}"#,
                    PI / 2.0
                )),
            ],
            attributes: vec![Attribute {
                tag: "NO".into(),
                value: "?".into(),
                p: Vec2::new(0.45, 0.3),
                height: 0.15,
                rotation: 0.0,
                align: None,
                width_factor: None,
                invisible: false,
                constant: false,
                movable: false,
                multiline: false,
                box_width: None,
            }],
        }]);
        let mut attrs = Attrs::default();
        attrs.0.push(("NO".to_owned(), Json::Str("K-12".to_owned())));
        let door = |rotation: f64| Shape::Insert {
            params: None,
            block: DOOR.into(),
            p: Vec2::new(10.0, 20.0),
            scale: 1.0,
            y_scale: None,
            rotation,
            mirror: Some(true),
            attrs: Some(attrs.clone()),
            places: None,
        };
        let texts = |pieces: &[Piece]| -> Vec<(f64, f64, f64, Option<TextAlign>, String)> {
            pieces
                .iter()
                .filter_map(|q| match &q.shape {
                    Shape::Text { p, rotation, align, text, .. } => {
                        let shown = q
                            .attribute
                            .as_ref()
                            .and_then(|tag| attrs.shown(tag).map(str::to_owned))
                            .unwrap_or_else(|| text.clone());
                        Some((p.x, p.y, *rotation, *align, shown))
                    }
                    _ => None,
                })
                .collect()
        };
        let pieces = blocks.expand(&door(0.0));
        // The swing arc mirrored: from east clockwise to south, stored counter-clockwise from 270° to 360°.
        let Some(Shape::Arc { a0, a1, .. }) = pieces.iter().map(|q| &q.shape).find(|s| matches!(s, Shape::Arc { .. }))
        else {
            panic!("an arc");
        };
        assert!(((a0 - 3.0 * PI / 2.0).abs() < 1e-12) && a1.abs() < 1e-12, "{a0} {a1}");
        assert_eq!(texts(&pieces), [(10.45, 19.7, 0.0, Some(TextAlign::TopLeft), "K-12".to_owned())]);
        let pieces = blocks.expand(&door(PI));
        let shown = texts(&pieces);
        let [(x, y, rotation, align, ref value)] = shown[..] else {
            panic!("one text");
        };
        assert!((x - 9.55).abs() < 1e-12 && (y - 20.3).abs() < 1e-12, "{x} {y}");
        assert!(rotation.abs() < 1e-9 || (rotation - 360.0).abs() < 1e-9, "{rotation}");
        assert_eq!((align, value.as_str()), (Some(TextAlign::BaselineRight), "K-12"));
    }

    /// Mirrored in the definition's x axis, then a half turn: (x, y) → (x, −y)
    /// → (−x, y). The rectangle (0,0) (2,0) (2,1) (0,1) becomes (0,0) (−2,0)
    /// (−2,1) (0,1), moved to (5, 5).
    #[test]
    fn a_mirror_then_a_half_turn() {
        let pieces = blocks().expand(&insert(ROGAR, (5.0, 5.0), 1.0, PI, true));
        assert_eq!(
            corners(&pieces[0].shape),
            [(5.0, 5.0), (3.0, 5.0), (3.0, 6.0), (5.0, 6.0)]
        );
        // Mirrored and three quarter turns: M (x, y) → (x, −y), R(270°) (u, v)
        // → (v, −u); together (x, y) → (−y, −x).
        let pieces = blocks().expand(&insert(ROGAR, (0.0, 0.0), 1.0, 3.0 * (PI / 2.0), true));
        assert_eq!(
            corners(&pieces[0].shape),
            [(0.0, 0.0), (0.0, -2.0), (-1.0, -2.0), (-1.0, 0.0)]
        );
    }

    /// A drawing point on an insert back in the definition's coordinates,
    /// worked by hand: base (1.5, 1), placed at (−4, −7) twice as large, a
    /// quarter turn, mirrored. The definition's (2.5, 1) is (1, 0) from the
    /// base: mirrored (1, 0), turned (0, 1), doubled (0, 2): at (−4, −5).
    /// Its (1.5, 0) is (0, −1): mirrored (0, 1), turned (−1, 0), doubled
    /// (−2, 0): at (−6, −7).
    #[test]
    fn a_point_on_an_insert_goes_back_into_the_definition() {
        let base = Vec2::new(1.5, 1.0);
        let p = Vec2::new(-4.0, -7.0);
        let quarter = PI / 2.0;
        // The insertion point is the base point, however it is placed.
        for (rotation, mirror) in [(0.0, false), (quarter, true), (PI, false), (-quarter, true)] {
            assert_eq!(insert_local(base, p, 2.0, rotation, mirror, p), Some(base));
        }
        let back = |x, y| insert_local(base, p, 2.0, quarter, true, Vec2::new(x, y));
        assert_eq!(back(-4.0, -5.0), Some(Vec2::new(2.5, 1.0)));
        assert_eq!(back(-6.0, -7.0), Some(Vec2::new(1.5, 0.0)));
        // Any turn: there and back through `insert_matrix`.
        let local = Vec2::new(3.25, -0.5);
        let at = crate::geom::affine::apply(&insert_matrix(base, p, 0.8, 0.3, true), local);
        let back = insert_local(base, p, 0.8, 0.3, true, at).expect("a scale");
        assert!((back.x - local.x).abs() < 1e-12 && (back.y - local.y).abs() < 1e-12);
        assert_eq!(insert_local(base, p, 0.0, 0.0, false, p), None);
        assert_eq!(insert_local(base, p, f64::NAN, 0.0, false, p), None);
    }

    /// Two levels: Direk holds Rögar turned a quarter at (10, 0), coloured
    /// green; Direk placed at (1000, 2000). Rögar's rectangle relative to its
    /// base turned: (0,0) (0,2) (−1,2) (−1,0), at (10, 0), then at Direk's
    /// place. Its uncoloured rectangle takes the nested insert's green; the
    /// red line keeps its red.
    #[test]
    fn nested_blocks_expand_through_both_levels() {
        let b = blocks();
        let pieces = b.expand(&insert(DIREK, (1000.0, 2000.0), 1.0, 0.0, false));
        assert_eq!(pieces.len(), 3);
        assert_eq!(corners(&pieces[0].shape), [(1000.0, 2005.0)]);
        assert_eq!(
            corners(&pieces[1].shape),
            [
                (1010.0, 2000.0),
                (1010.0, 2002.0),
                (1009.0, 2002.0),
                (1009.0, 2000.0)
            ]
        );
        assert_eq!(pieces[1].color.as_deref(), Some("#00FF00"));
        assert_eq!(pieces[2].color.as_deref(), Some("#FF0000"));
        // Flattened once: the definition's pieces are relative to its base.
        assert_eq!(b.get(DIREK).map(|f| f.pieces.len()), Some(3));
    }

    #[test]
    fn unknown_blocks_and_cycles_expand_to_nothing_more() {
        let b = blocks();
        assert!(
            b.expand(&insert("yok", (0.0, 0.0), 1.0, 0.0, false))
                .is_empty()
        );
        // A holds B, B holds A: each stops where the cycle closes.
        let ring = Blocks::new(vec![
            Definition {
                dynamic: None,
                id: "a".into(),
                base: Vec2::new(0.0, 0.0),
                entities: vec![entity(
                    r#"{"kind":"insert","id":1,"layerId":"","attrs":{},"block":"b","p":{"x":0,"y":0},"scale":1,"rotation":0}"#,
                )],
                attributes: Vec::new(),
            },
            Definition {
                dynamic: None,
                id: "b".into(),
                base: Vec2::new(0.0, 0.0),
                entities: vec![
                    entity(r#"{"kind":"point","id":1,"layerId":"","attrs":{},"p":{"x":1,"y":1}}"#),
                    entity(
                        r#"{"kind":"insert","id":2,"layerId":"","attrs":{},"block":"a","p":{"x":0,"y":0},"scale":1,"rotation":0}"#,
                    ),
                ],
                attributes: Vec::new(),
            },
        ]);
        assert_eq!(ring.get("a").map(|f| f.pieces.len()), Some(1));
        assert!(ring.get("b").is_some());
    }

    /// Patlat opens one level: Direk's point and Rögar's insert, the insert
    /// with its own similarity composed; the point takes the insert's layer
    /// (it has none of its own) and colour.
    #[test]
    fn explode_opens_one_level() {
        let b = blocks();
        let placed = entity(&format!(
            r##"{{"kind":"insert","id":7,"layerId":"altyapi","color":"#123456","attrs":{{}},"block":"{DIREK}","p":{{"x":1000,"y":2000}},"scale":2,"rotation":0}}"##
        ));
        let Cut::Pieces(parts) = b.explode(&placed) else {
            panic!("pieces");
        };
        assert_eq!(parts.len(), 2);
        assert_eq!(corners(&parts[0].shape), [(1000.0, 2010.0)]);
        let field =
            |e: &Entity, k: &str| e.rest.iter().find(|(n, _)| n == k).map(|(_, v)| v.clone());
        assert_eq!(
            field(&parts[0], "layerId"),
            Some(Json::Str("altyapi".into()))
        );
        assert_eq!(field(&parts[0], "color"), Some(Json::Str("#123456".into())));
        assert_eq!(field(&parts[0], "id"), None);
        // The nested insert: at (1020, 2000), twice its size, still a quarter turn, its own green.
        match &parts[1].shape {
            Shape::Insert {
                block,
                p,
                scale,
                rotation,
                mirror,
                ..
            } => {
                assert_eq!(block, ROGAR);
                assert_eq!(
                    (p.x, p.y, *scale, *rotation, *mirror),
                    (1020.0, 2000.0, 2.0, PI / 2.0, None)
                );
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(field(&parts[1], "color"), Some(Json::Str("#00FF00".into())));
        // Exploding the nested insert then gives the rectangle where the expansion drew it.
        let Cut::Pieces(inner) = b.explode(&parts[1]) else {
            panic!("pieces");
        };
        let expanded = b.expand(&placed.shape);
        assert_eq!(corners(&inner[0].shape), corners(&expanded[1].shape));
        assert!(matches!(
            b.explode(&entity(
                r#"{"kind":"point","id":1,"layerId":"a","attrs":{},"p":{"x":0,"y":0}}"#
            )),
            Cut::Error(_)
        ));
    }
}
