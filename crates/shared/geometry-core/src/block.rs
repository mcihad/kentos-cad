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

use std::collections::HashMap;
use std::sync::Arc;

use crate::api::json::Json;
use crate::entity::{Attrs, Entity, Shape};
use crate::geom::affine::{Affine, compose, translation};
use crate::jsmath::{PI, cos, sin};
use crate::ops::curve_cuts::Cut;
use crate::ops::transform::transform_shape;
use crate::text::TextAlign;
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
}

impl Attribute {
    /// Its text showing `text`, in the definition's coordinates.
    pub fn shape(&self, text: String) -> Shape {
        Shape::Text {
            p: self.p,
            text,
            height: self.height,
            rotation: self.rotation,
            align: self.align,
            width_factor: self.width_factor,
            mask: None,
            box_width: None,
            line_spacing: None,
            runs: None,
        }
    }
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
    pub attribute: Option<String>,
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

/// A drawing's definitions, flattened, by id.
#[derive(Clone, Debug, Default)]
pub struct Blocks {
    by_id: HashMap<String, Arc<Flat>>,
}

impl Blocks {
    /// The definitions flattened, each once whatever uses it.
    pub fn new(defs: Vec<Definition>) -> Blocks {
        let sources: HashMap<&str, &Definition> = defs.iter().map(|d| (d.id.as_str(), d)).collect();
        let mut done: HashMap<String, Arc<Flat>> = HashMap::with_capacity(defs.len());
        let mut stack = Vec::new();
        for d in &defs {
            flatten(&d.id, &sources, &mut done, &mut stack);
        }
        Blocks { by_id: done }
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
            defs.push(Definition {
                id: id.clone(),
                base,
                entities,
                attributes,
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
        let Some((flat, m)) = self.placed(insert) else {
            return Vec::new();
        };
        let values = insert_attrs(insert);
        flat.pieces
            .iter()
            .map(|piece| {
                let mut shape = transform_shape(&piece.shape, &m);
                if let (Some(tag), Some(values), Shape::Text { text, .. }) =
                    (&piece.attribute, values, &mut shape)
                    && let Some(v) = values.shown(tag)
                {
                    v.clone_into(text);
                }
                Piece {
                    shape,
                    color: piece.color.clone(),
                    line_weight: piece.line_weight,
                    attribute: piece.attribute.clone(),
                }
            })
            .collect()
    }

    /// An insert's definition and its similarity from the base-relative pieces to the drawing.
    fn placed(&self, insert: &Shape) -> Option<(&Arc<Flat>, Affine)> {
        let Shape::Insert {
            block,
            p,
            scale,
            rotation,
            mirror,
            ..
        } = insert
        else {
            return None;
        };
        let flat = self.by_id.get(block)?;
        Some((
            flat,
            placement(*p, *scale, *rotation, mirror.unwrap_or(false)),
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
        let Some((flat, m)) = self.placed(&insert.shape) else {
            return Cut::Error("Yerleştirilen blok çizimde tanımlı değil; patlatılamadı.".into());
        };
        let values = insert_attrs(&insert.shape);
        let shown: Vec<(&Attribute, String)> = flat
            .attributes
            .iter()
            .filter_map(|a| {
                let v = values.and_then(|v| v.shown(&a.tag)).unwrap_or(&a.value);
                (!v.is_empty()).then(|| (a, v.to_owned()))
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
        let pieces = flat
            .entities
            .iter()
            .map(|e| {
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
                Entity {
                    shape: transform_shape(&e.shape, &m),
                    rest,
                }
            })
            .chain(shown.into_iter().map(|(a, text)| {
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
                    shape: transform_shape(&a.shape(text), &m),
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
    })
}

/// Whether a piece is an attribute's text that shows nothing (§7: neither
/// the insert's value nor a default): it keeps its place among the pieces,
/// but draws, picks, snaps and measures nothing.
pub fn shows_nothing(piece: &Shape) -> bool {
    matches!(piece, Shape::Text { text, .. } if text.is_empty())
}

/// An insert's attributes (docs/adr/0144 §7).
fn insert_attrs(shape: &Shape) -> Option<&Attrs> {
    match shape {
        Shape::Insert { attrs, .. } => attrs.as_ref(),
        _ => None,
    }
}

/// The similarity an insert places its base-relative pieces with:
/// x' = p + scale·R(rotation)·M·x, where M mirrors in the x axis when
/// `mirror`. A rotation that is exactly the double of a quarter turn
/// (0, π/2, π, 3π/2 and their negatives) has an exact sine and cosine.
pub fn placement(p: Vec2, scale: f64, rotation: f64, mirror: bool) -> Affine {
    let (s, c) = quarter_turn(rotation).unwrap_or_else(|| (sin(rotation), cos(rotation)));
    let k = if mirror { -1.0 } else { 1.0 };
    [
        scale * c,
        scale * s,
        -scale * k * s,
        scale * k * c,
        p.x,
        p.y,
    ]
}

/// The whole matrix of an insert of a definition whose base point is `base`
/// (`placement` after moving the base point to the origin).
pub fn insert_matrix(base: Vec2, p: Vec2, scale: f64, rotation: f64, mirror: bool) -> Affine {
    compose(
        &placement(p, scale, rotation, mirror),
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
    if !(scale.is_finite() && scale != 0.0) {
        return None;
    }
    let (s, c) = quarter_turn(rotation).unwrap_or_else(|| (sin(rotation), cos(rotation)));
    let (dx, dy) = ((at.x - p.x) / scale, (at.y - p.y) / scale);
    // R⁻¹ is R's transpose; M is its own inverse.
    let (x, y) = (c * dx + s * dy, -s * dx + c * dy);
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
    // A cycle, or deeper than the documents allow: nothing more below here.
    if stack.iter().any(|s| s == id) || stack.len() >= MAX_DEPTH {
        return;
    }
    stack.push(id.to_owned());
    let to_base = translation(-def.base.x, -def.base.y);
    let mut pieces = Vec::new();
    for e in &def.entities {
        let own_color = text_field(&e.rest, "color");
        let own_weight = number_field(&e.rest, "lineWeight");
        if let Shape::Insert {
            block,
            p,
            scale,
            rotation,
            mirror,
            attrs,
        } = &e.shape
        {
            flatten(block, sources, done, stack);
            let Some(inner) = done.get(block.as_str()).cloned() else {
                continue;
            };
            // The nested pieces: placed in this definition, then relative to its base point.
            let m = compose(
                &to_base,
                &placement(*p, *scale, *rotation, mirror.unwrap_or(false)),
            );
            for piece in &inner.pieces {
                // A nested insert's attribute shows its value, fixed in this definition.
                let mut shape = transform_shape(&piece.shape, &m);
                if let (Some(tag), Some(values), Shape::Text { text, .. }) =
                    (&piece.attribute, attrs, &mut shape)
                    && let Some(v) = values.shown(tag)
                {
                    v.clone_into(text);
                }
                if matches!(&shape, Shape::Text { text, .. } if text.is_empty()) {
                    continue;
                }
                pieces.push(Piece {
                    shape,
                    color: piece.color.clone().or_else(|| own_color.clone()),
                    line_weight: piece.line_weight.or(own_weight),
                    attribute: None,
                });
            }
        } else {
            pieces.push(Piece {
                shape: transform_shape(&e.shape, &to_base),
                color: own_color,
                line_weight: own_weight,
                attribute: None,
            });
        }
    }
    // Its own attributes: texts of the default, the tag telling the host to show the insert's value.
    for a in &def.attributes {
        pieces.push(Piece {
            shape: transform_shape(&a.shape(a.value.clone()), &to_base),
            color: None,
            line_weight: None,
            attribute: Some(a.tag.clone()),
        });
    }
    stack.pop();
    done.insert(
        id.to_owned(),
        Arc::new(Flat {
            base: def.base,
            entities: def.entities.clone(),
            attributes: def.attributes.clone(),
            pieces,
        }),
    );
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
            block: block.into(),
            p: Vec2::new(p.0, p.1),
            scale,
            rotation,
            mirror: mirror.then_some(true),
            attrs: None,
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
                id: "a".into(),
                base: Vec2::new(0.0, 0.0),
                entities: vec![entity(
                    r#"{"kind":"insert","id":1,"layerId":"","attrs":{},"block":"b","p":{"x":0,"y":0},"scale":1,"rotation":0}"#,
                )],
                attributes: Vec::new(),
            },
            Definition {
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
