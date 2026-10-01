//! A drawing object (`Entity`, the contract) as a stored feature and back
//! (CLAUDE.md §15, docs/adr/0006).
//!
//! - Point, line, straight polyline and straight polygon (a multi-part area
//!   a MultiPolygon, docs/adr/0143) without vertex elevations: the PostGIS
//!   geometry is the source, written and read as EWKB (every bit kept). The
//!   geometry is 2D, so an object with elevations keeps its definition
//!   (docs/adr/0142).
//! - Everything else: `cad_definition` (the entity's geometric fields exactly
//!   as in the contract) is the source; the geometry is its linear projection
//!   within `PROJECTION_TOLERANCE`, made by `geometry-core`, and null for
//!   construction lines and rays, which have no finite geometry. A block
//!   insert's is the collection of its block's objects placed, each
//!   projected as that object is, flat (docs/adr/0144 §5).
//!
//! Negative zero becomes zero on the way in (PostgreSQL's numeric has none),
//! so every kind round-trips to exactly the value that was accepted.

use kentos_contracts::{BlockDefinition, Entity};
use kentos_formats::blocks::Placing;
use kentos_geometry_core::Vec2 as P;
use kentos_geometry_core::ewkb::{self, Geometry};
use kentos_geometry_core::tessellate::{
    arc_points, bulge_path, catmull_rom, circle_ring, ellipse_points,
};
use serde_json::{Map, Value};

/// Version of the projection rules below, stored with each feature.
pub const PROJECTION_VERSION: i32 = 1;
/// Largest chord deviation of a projected curve, metres.
pub const PROJECTION_TOLERANCE: f64 = 0.001;
/// Coordinates and sizes beyond this are refused as garbage (TM coordinates reach 4.5·10⁶ m).
const MAX_ABS: f64 = 1e9;
const MAX_POINTS: usize = 1_000_000;
const BASE_KEYS: [&str; 8] = [
    "kind",
    "id",
    "layerId",
    "color",
    "attrs",
    "label",
    "symbol",
    "lineWeight",
];

/// A feature row's content (without ids, versions and audit columns).
#[derive(Clone, Debug, PartialEq)]
pub struct Stored {
    pub layer_id: String,
    pub kind: String,
    pub source_kind: &'static str,
    pub geom: Option<Vec<u8>>,
    pub cad_definition: Option<Value>,
    pub properties: Value,
    pub label: Option<String>,
    pub color: Option<String>,
    pub symbol: Option<String>,
    /// Its own line weight, mm (docs/adr/0139); none: its layer's.
    pub line_weight: Option<f64>,
}

fn p(v: kentos_contracts::Vec2) -> P {
    P::new(v.x, v.y)
}

fn pts(list: &[kentos_contracts::Vec2]) -> Vec<P> {
    list.iter().copied().map(p).collect()
}

/// Makes every number sign-free at zero and checks its size; counts points.
fn normalize(v: &mut Value, where_: &str) -> Result<(), String> {
    match v {
        Value::Number(n) => {
            let f = n
                .as_f64()
                .ok_or_else(|| format!("{where_}: sayı okunamadı"))?;
            if f.abs() > MAX_ABS {
                return Err(format!("{where_}: {f} çok büyük (en çok ±10⁹)"));
            }
            if f == 0.0 && f.is_sign_negative() {
                *v = Value::from(0.0);
            }
        }
        Value::Array(items) => items.iter_mut().try_for_each(|x| normalize(x, where_))?,
        Value::Object(map) => map.values_mut().try_for_each(|x| normalize(x, where_))?,
        _ => {}
    }
    Ok(())
}

fn check_len(what: &str, n: usize, min: usize) -> Result<(), String> {
    if n < min {
        return Err(format!("{what} en az {min} köşe ister ({n} verildi)"));
    }
    if n > MAX_POINTS {
        return Err(format!("{what} en çok {MAX_POINTS} köşe olabilir"));
    }
    Ok(())
}

fn check_bulges(what: &str, bulges: &Option<Vec<f64>>, points: usize) -> Result<(), String> {
    match bulges {
        Some(b) if b.len() > points => Err(format!(
            "{what}: yay sayısı ({}) köşe sayısından ({points}) fazla",
            b.len()
        )),
        _ => Ok(()),
    }
}

/// A text's or an attribute definition's width factor, when it has one (docs/adr/0145).
fn width_factor(w: Option<f64>) -> Result<(), String> {
    match w {
        Some(w) if !kentos_contracts::width_factor_ok(w) => Err(format!(
            "Yazının genişlik çarpanı {w}; 0'dan büyük, en çok {} olmalı",
            kentos_contracts::MAX_WIDTH_FACTOR
        )),
        _ => Ok(()),
    }
}

fn positive(what: &str, v: f64) -> Result<(), String> {
    if v > 0.0 {
        Ok(())
    } else {
        Err(format!("{what} sıfırdan büyük olmalı"))
    }
}

/// Rules the contract's types cannot say: point counts, positive sizes, text
/// lengths. A block's object may have no layer of its own (`""`, the block's,
/// docs/adr/0144).
fn validate(e: &Entity, in_block: bool) -> Result<(), String> {
    use Entity::*;
    let base = e.base();
    if (base.layer_id.is_empty() && !in_block) || base.layer_id.len() > 200 {
        return Err("katman kimliği boş ya da çok uzun".into());
    }
    if let Some(w) = base.line_weight
        && !(0.0..=kentos_contracts::MAX_LINE_WEIGHT).contains(&w)
    {
        return Err(format!(
            "çizgi kalınlığı {w} mm; 0 ile {} arasında olmalı",
            kentos_contracts::MAX_LINE_WEIGHT
        ));
    }
    if base.attrs.len() > 500
        || base
            .attrs
            .iter()
            .any(|(k, v)| k.is_empty() || k.len() > 200 || v.len() > 10_000)
    {
        return Err(
            "öznitelikler sınırı aşıyor (en çok 500 alan, ad 200, değer 10 000 karakter)".into(),
        );
    }
    if base.label.as_ref().is_some_and(|l| l.len() > 1000)
        || base.color.as_ref().is_some_and(|c| c.len() > 64)
        || base.symbol.as_ref().is_some_and(|s| s.len() > 200)
    {
        return Err("etiket, renk ya da sembol çok uzun".into());
    }
    match e {
        Polyline(x) => {
            check_len("Çoklu çizgi", x.pts.len(), 2)?;
            check_bulges("Çoklu çizgi", &x.bulges, x.pts.len())?;
            if x.holes.is_some() {
                return Err("Çoklu çizginin adası olamaz".into());
            }
            if x.parts.is_some() {
                return Err(
                    "Çoklu çizginin parçası olamaz; yalnız kapalı alan çok parçalı olur".into(),
                );
            }
        }
        Polygon(x) => {
            check_len("Alan", x.pts.len(), 3)?;
            check_bulges("Alan", &x.bulges, x.pts.len())?;
            for h in x.holes.iter().flatten() {
                check_len("Ada", h.pts.len(), 3)?;
                check_bulges("Ada", &h.bulges, h.pts.len())?;
            }
            // A multi-part area's other parts, each as the area's own ring (docs/adr/0143).
            for p in x.parts.iter().flatten() {
                check_len("Alanın parçası", p.pts.len(), 3)?;
                check_bulges("Alanın parçası", &p.bulges, p.pts.len())?;
                for h in p.holes.iter().flatten() {
                    check_len("Ada", h.pts.len(), 3)?;
                    check_bulges("Ada", &h.bulges, h.pts.len())?;
                }
            }
        }
        Circle(x) => positive("Yarıçap", x.r)?,
        Arc(x) => positive("Yarıçap", x.r)?,
        Ellipse(x) => {
            if !(x.ratio > 0.0 && x.ratio <= 1.0) {
                return Err("Elipsin eksen oranı 0 ile 1 arasında olmalı".into());
            }
            positive("Büyük eksen", x.major.x.hypot(x.major.y))?;
        }
        Spline(x) => check_len("Eğri", x.pts.len(), 2)?,
        Xline(x) | Ray(x) => positive("Doğrultu", x.dir.x.hypot(x.dir.y))?,
        Text(x) => {
            positive("Yazı yüksekliği", x.height)?;
            if x.text.len() > 10_000 {
                return Err("Yazı en çok 10 000 karakter olabilir".into());
            }
            width_factor(x.width_factor)?;
        }
        Dimension(x) => positive("Ölçü yazısı yüksekliği", x.height)?,
        Hatch(x) => {
            check_len("Tarama sınırı", x.ring.len(), 3)?;
            for h in x.holes.iter().flatten() {
                check_len("Tarama adası", h.len(), 3)?;
            }
            positive("Tarama aralığı", x.pattern.spacing)?;
        }
        // Its block is the project's: the commit checks it is there (docs/adr/0144 §5).
        Insert(x) => {
            if !kentos_contracts::blocks::scale_ok(x.scale) {
                return Err("Blok ölçeği pozitif bir sayı olmalı".into());
            }
        }
        // The file's rules (docs/adr/0146 §3): two vertices, a height, a note that is not empty.
        Leader(x) => {
            check_len("Kılavuz", x.pts.len(), 2)?;
            positive("Kılavuz yüksekliği", x.height)?;
            match &x.text {
                Some(t) if t.is_empty() => {
                    return Err(
                        "Kılavuzun notu boş olamaz; notsuz kılavuzda not alanı yazılmaz".into(),
                    );
                }
                Some(t) if t.len() > 10_000 => {
                    return Err("Kılavuzun notu en çok 10 000 karakter olabilir".into());
                }
                _ => {}
            }
        }
        Point(_) | Line(_) => {}
    }
    Ok(())
}

/// Whether the geometry alone holds the object: no arcs, no empty hole or
/// part list, and no vertex elevation (the geometry is 2D, docs/adr/0142);
/// a multi-part area's parts likewise (docs/adr/0143).
fn geometry_is_source(e: &Entity) -> bool {
    let flat =
        |bulges: &Option<Vec<f64>>, zs: &Option<Vec<Option<f64>>>| bulges.is_none() && zs.is_none();
    let area = |bulges, zs, holes: &Option<Vec<kentos_contracts::RingGeometry>>| {
        flat(bulges, zs)
            && holes
                .as_ref()
                .is_none_or(|hs| !hs.is_empty() && hs.iter().all(|h| flat(&h.bulges, &h.zs)))
    };
    match e {
        Entity::Point(_) => true,
        Entity::Line(x) => x.za.is_none() && x.zb.is_none(),
        Entity::Polyline(x) => flat(&x.bulges, &x.zs) && x.holes.is_none(),
        Entity::Polygon(x) => {
            area(&x.bulges, &x.zs, &x.holes)
                && x.parts.as_ref().is_none_or(|ps| {
                    !ps.is_empty() && ps.iter().all(|p| area(&p.bulges, &p.zs, &p.holes))
                })
        }
        _ => false,
    }
}

/// A placed block object's projection in a collection, which is flat: a point's elevation stays in the source.
fn flat(g: Geometry) -> Geometry {
    match g {
        Geometry::Point { p, .. } => Geometry::Point { p, z: None },
        other => other,
    }
}

/// The linear PostGIS geometry of an object (`None`: nothing finite to
/// draw); a block insert's from the project's blocks.
fn projection(e: &Entity, blocks: &Placing) -> Option<Geometry> {
    let tol = PROJECTION_TOLERANCE;
    let ring = |r: &[kentos_contracts::Vec2], b: &Option<Vec<f64>>| {
        bulge_path(&pts(r), b.as_deref(), true, tol)
    };
    Some(match e {
        Entity::Point(x) => Geometry::Point { p: p(x.p), z: x.z },
        Entity::Line(x) => Geometry::LineString(vec![p(x.a), p(x.b)]),
        Entity::Polyline(x) => {
            Geometry::LineString(bulge_path(&pts(&x.pts), x.bulges.as_deref(), false, tol))
        }
        Entity::Polygon(x) => {
            let mut rings = vec![ring(&x.pts, &x.bulges)];
            rings.extend(x.holes.iter().flatten().map(|h| ring(&h.pts, &h.bulges)));
            match x.parts.as_deref() {
                // A multi-part area is a MultiPolygon, its own rings the first member (docs/adr/0143).
                Some(parts) if !parts.is_empty() => {
                    let mut polygons = vec![rings];
                    polygons.extend(parts.iter().map(|p| {
                        let mut rings = vec![ring(&p.pts, &p.bulges)];
                        rings.extend(p.holes.iter().flatten().map(|h| ring(&h.pts, &h.bulges)));
                        rings
                    }));
                    Geometry::MultiPolygon(polygons)
                }
                _ => Geometry::Polygon(rings),
            }
        }
        Entity::Circle(x) => Geometry::Polygon(vec![circle_ring(p(x.c), x.r, tol)]),
        Entity::Arc(x) => {
            let sweep = (x.a1 - x.a0).rem_euclid(std::f64::consts::TAU);
            let sweep = if sweep == 0.0 {
                std::f64::consts::TAU
            } else {
                sweep
            };
            Geometry::LineString(arc_points(p(x.c), x.r, x.a0, sweep, tol))
        }
        Entity::Ellipse(x) => match ellipse_points(p(x.c), p(x.major), x.ratio, x.t0, x.t1, tol) {
            (points, true) => Geometry::Polygon(vec![points]),
            (points, false) => Geometry::LineString(points),
        },
        Entity::Spline(x) => {
            let points = catmull_rom(&pts(&x.pts), x.closed, tol);
            if x.closed && x.pts.len() >= 3 {
                Geometry::Polygon(vec![points])
            } else {
                Geometry::LineString(points)
            }
        }
        Entity::Xline(_) | Entity::Ray(_) => return None,
        Entity::Text(x) => Geometry::Point { p: p(x.p), z: None },
        // A dimension's defining points (the measured ones and, for an angle, its vertex).
        Entity::Dimension(x) => Geometry::LineString(match x.c {
            Some(c) => vec![p(x.a), p(c), p(x.b)],
            None => vec![p(x.a), p(x.b)],
        }),
        Entity::Hatch(x) => {
            let mut rings = vec![pts(&x.ring)];
            rings.extend(x.holes.iter().flatten().map(|h| pts(h)));
            Geometry::Polygon(rings)
        }
        // Its line; the arrowhead, the landing and the note are not projected (docs/adr/0146 §3).
        Entity::Leader(x) => Geometry::LineString(pts(&x.pts)),
        // Its block's objects placed (the core's expansion, nested blocks opened), each as it is projected.
        Entity::Insert(i) => Geometry::Collection(
            blocks
                .placed(i)
                .iter()
                .filter_map(|piece| projection(piece, blocks).map(flat))
                .collect(),
        ),
    })
}

/// A block insert's geometry for SRID `srid` as the project's blocks place
/// it now (a definition it places changed, docs/adr/0144 §5).
pub fn insert_geometry(entity: &Entity, srid: u32, blocks: &Placing) -> Option<Vec<u8>> {
    projection(entity, blocks).map(|g| ewkb::encode(&g, srid))
}

/// A block definition as it is stored (docs/adr/0144 §5): its numbers
/// without the sign of zero and in range, each of its objects one the
/// server keeps; the definition and its JSON. The rules over the project's
/// definitions (names, known blocks, cycles, depth) are the commit's.
pub fn to_stored_block(block: &BlockDefinition) -> Result<(BlockDefinition, Value), String> {
    let mut value = serde_json::to_value(block).map_err(|e| e.to_string())?;
    normalize(&mut value, "blok")?;
    let block: BlockDefinition =
        serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
    if block.name.chars().count() > 255 {
        return Err("Blok adı en çok 255 karakter olabilir".into());
    }
    for (i, e) in block.entities.iter().enumerate() {
        validate(e, true).map_err(|why| format!("{}. nesnesi: {why}", i + 1))?;
    }
    for (i, a) in block.attributes.iter().enumerate() {
        width_factor(a.width_factor).map_err(|why| format!("{}. özniteliği: {why}", i + 1))?;
    }
    Ok((block, value))
}

/// Checks an object and turns it into a feature row's content for SRID
/// `srid`; a block insert's geometry from the project's `blocks` (whether its
/// block is there is the commit's check).
pub fn to_stored(entity: &Entity, srid: u32, blocks: &Placing) -> Result<Stored, String> {
    let mut value = serde_json::to_value(entity).map_err(|e| e.to_string())?;
    normalize(&mut value, "nesne")?;
    let entity: Entity = serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
    validate(&entity, false)?;
    let Value::Object(mut map) = value else {
        unreachable!("an entity serializes to an object")
    };
    let kind = map
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let text =
        |m: &Map<String, Value>, k: &str| m.get(k).and_then(Value::as_str).map(str::to_string);
    let layer_id = text(&map, "layerId").unwrap_or_default();
    let (label, color, symbol) = (
        text(&map, "label"),
        text(&map, "color"),
        text(&map, "symbol"),
    );
    let line_weight = map.get("lineWeight").and_then(Value::as_f64);
    let properties = map
        .get("attrs")
        .cloned()
        .unwrap_or_else(|| Value::Object(Map::new()));
    let geom = projection(&entity, blocks).map(|g| ewkb::encode(&g, srid));
    let (source_kind, cad_definition) = if geometry_is_source(&entity) {
        ("geom", None)
    } else {
        for k in BASE_KEYS {
            map.remove(k);
        }
        ("cad", Some(Value::Object(map)))
    };
    Ok(Stored {
        layer_id,
        kind,
        source_kind,
        geom,
        cad_definition,
        properties,
        label,
        color,
        symbol,
        line_weight,
    })
}

fn xy(v: P) -> Value {
    serde_json::json!({ "x": v.x, "y": v.y })
}

fn ring_value(r: &[P]) -> Value {
    Value::Array(r.iter().copied().map(xy).collect())
}

/// An area's (or a part's) `pts` and `holes` from its rings, the outline first.
fn area_value(m: &mut Map<String, Value>, rings: &[Vec<P>]) {
    m.insert("pts".into(), ring_value(&rings[0]));
    if rings.len() > 1 {
        let holes: Vec<Value> = rings[1..]
            .iter()
            .map(|r| serde_json::json!({ "pts": ring_value(r) }))
            .collect();
        m.insert("holes".into(), Value::Array(holes));
    }
}

/// Rebuilds the object from a feature row (`entity.id` is 0: the browser numbers objects itself).
pub fn from_stored(s: &Stored) -> Result<Entity, String> {
    let mut map = match (s.source_kind, &s.cad_definition) {
        ("cad", Some(Value::Object(def))) => def.clone(),
        ("geom", _) => {
            let bytes = s.geom.as_deref().ok_or("kaynak geometri yok")?;
            let (g, _) = ewkb::decode(bytes)?;
            let mut m = Map::new();
            match (s.kind.as_str(), g) {
                ("point", Geometry::Point { p, z }) => {
                    m.insert("p".into(), xy(p));
                    if let Some(z) = z {
                        m.insert("z".into(), z.into());
                    }
                }
                ("line", Geometry::LineString(l)) if l.len() == 2 => {
                    m.insert("a".into(), xy(l[0]));
                    m.insert("b".into(), xy(l[1]));
                }
                ("polyline", Geometry::LineString(l)) => {
                    m.insert("pts".into(), ring_value(&l));
                }
                ("polygon", Geometry::Polygon(rings)) if !rings.is_empty() => {
                    area_value(&mut m, &rings);
                }
                // The first member is the area's own rings, the others its parts (docs/adr/0143).
                ("polygon", Geometry::MultiPolygon(polygons))
                    if polygons.len() > 1 && polygons.iter().all(|p| !p.is_empty()) =>
                {
                    area_value(&mut m, &polygons[0]);
                    let parts = polygons[1..].iter().map(|rings| {
                        let mut part = Map::new();
                        area_value(&mut part, rings);
                        Value::Object(part)
                    });
                    m.insert("parts".into(), Value::Array(parts.collect()));
                }
                (kind, g) => {
                    return Err(format!("{kind} nesnesinin geometrisi beklenmedik: {g:?}"));
                }
            }
            m
        }
        _ => return Err("nesne tanımı bozuk".into()),
    };
    map.insert("kind".into(), s.kind.clone().into());
    map.insert("id".into(), 0.into());
    map.insert("layerId".into(), s.layer_id.clone().into());
    map.insert("attrs".into(), s.properties.clone());
    for (k, v) in [
        ("label", &s.label),
        ("color", &s.color),
        ("symbol", &s.symbol),
    ] {
        if let Some(v) = v {
            map.insert(k.into(), v.clone().into());
        }
    }
    if let Some(w) = s.line_weight {
        map.insert("lineWeight".into(), w.into());
    }
    serde_json::from_value(Value::Object(map)).map_err(|e| format!("nesne okunamadı: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entity(json: Value) -> Entity {
        serde_json::from_value(json).unwrap()
    }

    /// A project without block definitions.
    fn none() -> Placing {
        Placing::new(&[])
    }

    /// A block insert (docs/adr/0144 §5): its source is its definition, its
    /// geometry its block's objects placed (a quarter turn, exact), flat;
    /// an unknown block's is an empty collection (the commit refuses it).
    #[test]
    fn an_insert_is_its_blocks_objects_placed() {
        let block: BlockDefinition = serde_json::from_value(serde_json::json!({
            "id": "018f3a2b-0000-7000-8000-000000000001", "name": "Direk", "base": { "x": 1, "y": 2 },
            "entities": [
                { "kind": "point", "id": 1, "layerId": "", "attrs": {}, "p": { "x": 1, "y": 3 }, "z": 5 },
                { "kind": "line", "id": 2, "layerId": "", "attrs": {}, "a": { "x": 1, "y": 2 }, "b": { "x": 3, "y": 2 } }
            ]
        }))
        .unwrap();
        let (block, _) = to_stored_block(&block).unwrap();
        let placing = Placing::new(std::slice::from_ref(&block));
        let insert = entity(
            serde_json::json!({ "kind": "insert", "id": 9, "layerId": "cizim", "attrs": { "No": "7" },
            "block": "018f3a2b-0000-7000-8000-000000000001", "p": { "x": 100, "y": 200 }, "scale": 2, "rotation": std::f64::consts::FRAC_PI_2 }),
        );
        let s = to_stored(&insert, 5256, &placing).unwrap();
        assert_eq!((s.kind.as_str(), s.source_kind), ("insert", "cad"));
        assert_eq!(s.properties, serde_json::json!({ "No": "7" }));
        let (g, srid) = ewkb::decode(s.geom.as_deref().unwrap()).unwrap();
        assert_eq!(srid, 5256);
        // (1, 3) is (0, 1) from the base: scaled (0, 2), turned (−2, 0); the line (0, 0)–(2, 0): (0, 0)–(0, 4).
        assert_eq!(
            g,
            Geometry::Collection(vec![
                Geometry::Point {
                    p: P::new(98.0, 200.0),
                    z: None
                },
                Geometry::LineString(vec![P::new(100.0, 200.0), P::new(100.0, 204.0)]),
            ])
        );
        assert_eq!(back(&s, 9), insert);
        let (unknown, _) = ewkb::decode(
            to_stored(&insert, 5256, &none())
                .unwrap()
                .geom
                .as_deref()
                .unwrap(),
        )
        .unwrap();
        assert_eq!(unknown, Geometry::Collection(Vec::new()));
        let zero = entity(
            serde_json::json!({ "kind": "insert", "id": 9, "layerId": "cizim", "attrs": {},
            "block": "018f3a2b-0000-7000-8000-000000000001", "p": { "x": 0, "y": 0 }, "scale": 0, "rotation": 0 }),
        );
        assert!(
            to_stored(&zero, 5256, &placing)
                .unwrap_err()
                .contains("ölçeği")
        );
    }

    /// A definition keeps its objects as a file does (the block's own layer
    /// `""` too), its numbers without the sign of zero; a bad object is refused by its place.
    #[test]
    fn a_block_definition_is_stored_as_its_objects_are() {
        let block: BlockDefinition = serde_json::from_value(serde_json::json!({
            "id": "018f3a2b-0000-7000-8000-000000000002", "name": "Ağaç", "base": { "x": -0.0, "y": 0 },
            "entities": [ { "kind": "circle", "id": 1, "layerId": "", "attrs": {}, "c": { "x": 0, "y": 0 }, "r": 1 } ]
        }))
        .unwrap();
        let (stored, value) = to_stored_block(&block).unwrap();
        assert_eq!(stored.name, "Ağaç");
        assert!(value["base"]["x"].as_f64().unwrap().is_sign_positive());
        let bad: BlockDefinition = serde_json::from_value(serde_json::json!({
            "id": "018f3a2b-0000-7000-8000-000000000003", "name": "Bozuk", "base": { "x": 0, "y": 0 },
            "entities": [
                { "kind": "circle", "id": 1, "layerId": "", "attrs": {}, "c": { "x": 0, "y": 0 }, "r": 1 },
                { "kind": "circle", "id": 2, "layerId": "", "attrs": {}, "c": { "x": 0, "y": 0 }, "r": 0 }
            ]
        }))
        .unwrap();
        assert!(
            to_stored_block(&bad)
                .unwrap_err()
                .starts_with("2. nesnesi: ")
        );
    }

    #[test]
    fn simple_shapes_keep_their_geometry_as_source() {
        let line = entity(
            serde_json::json!({ "kind": "line", "id": 7, "layerId": "cizim", "attrs": { "Ad": "x" },
            "a": { "x": 486512.34, "y": 4420210.5 }, "b": { "x": 486520.0, "y": -0.0 } }),
        );
        let s = to_stored(&line, 5256, &none()).unwrap();
        assert_eq!((s.source_kind, s.kind.as_str()), ("geom", "line"));
        assert!(s.cad_definition.is_none());
        let back = from_stored(&s).unwrap();
        let Entity::Line(l) = &back else { panic!() };
        assert_eq!(l.b.y.to_bits(), 0.0f64.to_bits());
        assert_eq!(l.base.id, 0);
        assert_eq!(l.base.attrs["Ad"], "x");
    }

    #[test]
    fn an_objects_own_line_weight_is_a_column_of_its_own() {
        // docs/adr/0139: a line keeps its geometry as source and has no definition to hold it.
        let line = entity(
            serde_json::json!({ "kind": "line", "id": 1, "layerId": "p", "attrs": {}, "lineWeight": 0.35,
            "a": { "x": 0, "y": 0 }, "b": { "x": 10, "y": 0 } }),
        );
        let s = to_stored(&line, 5256, &none()).unwrap();
        assert_eq!((s.source_kind, s.line_weight), ("geom", Some(0.35)));
        assert_eq!(from_stored(&s).unwrap().base().line_weight, Some(0.35));
        // A circle's definition does not hold it twice.
        let circle = entity(
            serde_json::json!({ "kind": "circle", "id": 1, "layerId": "p", "attrs": {}, "lineWeight": 0.0,
            "c": { "x": 0, "y": 0 }, "r": 2 }),
        );
        let s = to_stored(&circle, 5256, &none()).unwrap();
        assert_eq!(s.line_weight, Some(0.0));
        assert!(
            s.cad_definition
                .as_ref()
                .unwrap()
                .get("lineWeight")
                .is_none()
        );
        assert_eq!(from_stored(&s).unwrap().base().line_weight, Some(0.0));
        // Without one, none; past 100 mm, refused.
        let plain = entity(
            serde_json::json!({ "kind": "point", "id": 1, "layerId": "p", "attrs": {}, "p": { "x": 0, "y": 0 } }),
        );
        assert_eq!(to_stored(&plain, 5256, &none()).unwrap().line_weight, None);
        let heavy = entity(
            serde_json::json!({ "kind": "point", "id": 1, "layerId": "p", "attrs": {}, "lineWeight": 101.0, "p": { "x": 0, "y": 0 } }),
        );
        assert!(
            to_stored(&heavy, 5256, &none())
                .unwrap_err()
                .contains("çizgi kalınlığı")
        );
    }

    /// The object read back, its id as it went in (the browser numbers objects itself).
    fn back(s: &Stored, id: u32) -> Entity {
        let mut again = from_stored(s).unwrap();
        again.base_mut().id = id;
        again
    }

    #[test]
    fn a_multi_part_area_is_a_multi_polygon() {
        // docs/adr/0143: straight, its MultiPolygon is the source, every part and hole kept.
        let two = entity(
            serde_json::json!({ "kind": "polygon", "id": 1, "layerId": "p", "attrs": { "Ada": "101" },
            "pts": [{ "x": 0, "y": 0 }, { "x": 10, "y": 0 }, { "x": 10, "y": 10 }],
            "parts": [{ "pts": [{ "x": 20, "y": 0 }, { "x": 30, "y": 0 }, { "x": 30, "y": 10 }],
                "holes": [{ "pts": [{ "x": 25, "y": 1 }, { "x": 28, "y": 1 }, { "x": 28, "y": 4 }] }] }] }),
        );
        let s = to_stored(&two, 5256, &none()).unwrap();
        assert_eq!(s.source_kind, "geom");
        let (g, _) = ewkb::decode(s.geom.as_deref().unwrap()).unwrap();
        assert!(matches!(&g, Geometry::MultiPolygon(p) if p.len() == 2 && p[1].len() == 2));
        assert_eq!(back(&s, 1), two);
        // With an elevation or an arc, the definition is the source and the projection still every part.
        let arced = entity(
            serde_json::json!({ "kind": "polygon", "id": 1, "layerId": "p", "attrs": {},
            "pts": [{ "x": 0, "y": 0 }, { "x": 10, "y": 0 }, { "x": 10, "y": 10 }],
            "parts": [{ "pts": [{ "x": 20, "y": 0 }, { "x": 30, "y": 0 }, { "x": 30, "y": 10 }], "bulges": [0, 0.5, 0] }] }),
        );
        let s = to_stored(&arced, 5256, &none()).unwrap();
        assert_eq!(s.source_kind, "cad");
        let (g, _) = ewkb::decode(s.geom.as_deref().unwrap()).unwrap();
        assert!(matches!(&g, Geometry::MultiPolygon(p) if p.len() == 2));
        assert_eq!(back(&s, 1), arced);
        let high = entity(
            serde_json::json!({ "kind": "polygon", "id": 1, "layerId": "p", "attrs": {},
            "pts": [{ "x": 0, "y": 0 }, { "x": 10, "y": 0 }, { "x": 10, "y": 10 }],
            "parts": [{ "pts": [{ "x": 20, "y": 0 }, { "x": 30, "y": 0 }, { "x": 30, "y": 10 }], "zs": [1.5, null, 2.0] }] }),
        );
        let s = to_stored(&high, 5256, &none()).unwrap();
        assert_eq!(s.source_kind, "cad");
        assert_eq!(back(&s, 1), high);
        // An empty part list is kept as it is.
        let empty = entity(
            serde_json::json!({ "kind": "polygon", "id": 1, "layerId": "p", "attrs": {},
            "pts": [{ "x": 0, "y": 0 }, { "x": 10, "y": 0 }, { "x": 10, "y": 10 }], "parts": [] }),
        );
        assert_eq!(back(&to_stored(&empty, 5256, &none()).unwrap(), 1), empty);
        // A part is refused as the area's own ring would be; a polyline has none.
        let thin = entity(
            serde_json::json!({ "kind": "polygon", "id": 1, "layerId": "p", "attrs": {},
            "pts": [{ "x": 0, "y": 0 }, { "x": 10, "y": 0 }, { "x": 10, "y": 10 }],
            "parts": [{ "pts": [{ "x": 20, "y": 0 }, { "x": 30, "y": 0 }] }] }),
        );
        assert!(
            to_stored(&thin, 5256, &none())
                .unwrap_err()
                .contains("Alanın parçası")
        );
        let path = entity(
            serde_json::json!({ "kind": "polyline", "id": 1, "layerId": "p", "attrs": {},
            "pts": [{ "x": 0, "y": 0 }, { "x": 10, "y": 0 }], "parts": [] }),
        );
        assert!(
            to_stored(&path, 5256, &none())
                .unwrap_err()
                .contains("parçası olamaz")
        );
    }

    #[test]
    fn vertex_elevations_keep_the_definition() {
        // docs/adr/0142: the geometry is 2D; a line's ends, a path's or an area's vertices keep theirs.
        for json in [
            serde_json::json!({ "kind": "line", "id": 1, "layerId": "p", "attrs": {},
                "a": { "x": 0, "y": 0 }, "b": { "x": 10, "y": 0 }, "za": 12.5 }),
            serde_json::json!({ "kind": "polyline", "id": 1, "layerId": "p", "attrs": {},
                "pts": [{ "x": 0, "y": 0 }, { "x": 10, "y": 0 }], "zs": [null, -0.5] }),
            serde_json::json!({ "kind": "polygon", "id": 1, "layerId": "p", "attrs": {},
                "pts": [{ "x": 0, "y": 0 }, { "x": 10, "y": 0 }, { "x": 10, "y": 10 }],
                "holes": [{ "pts": [{ "x": 5, "y": 1 }, { "x": 8, "y": 1 }, { "x": 8, "y": 4 }], "zs": [1, 2, 3] }] }),
        ] {
            let e = entity(json);
            let s = to_stored(&e, 5256, &none()).unwrap();
            assert_eq!(s.source_kind, "cad", "{e:?}");
            assert_eq!(back(&s, 1), e);
        }
    }

    #[test]
    fn arcs_and_empty_hole_lists_keep_the_definition() {
        let bulged = entity(
            serde_json::json!({ "kind": "polygon", "id": 1, "layerId": "p", "attrs": {},
            "pts": [{ "x": 0, "y": 0 }, { "x": 10, "y": 0 }, { "x": 10, "y": 10 }], "bulges": [0, 0.5, 0] }),
        );
        let s = to_stored(&bulged, 5256, &none()).unwrap();
        assert_eq!(s.source_kind, "cad");
        assert!(s.geom.is_some());
        let def = s.cad_definition.as_ref().unwrap();
        assert!(def.get("layerId").is_none() && def.get("bulges").is_some());
        let mut again = from_stored(&s).unwrap();
        if let Entity::Polygon(x) = &mut again {
            x.base.id = 1;
        }
        assert_eq!(again, bulged);
        let empty_holes = entity(
            serde_json::json!({ "kind": "polygon", "id": 1, "layerId": "p", "attrs": {},
            "pts": [{ "x": 0, "y": 0 }, { "x": 1, "y": 0 }, { "x": 1, "y": 1 }], "holes": [] }),
        );
        assert_eq!(
            to_stored(&empty_holes, 5256, &none()).unwrap().source_kind,
            "cad"
        );
    }

    #[test]
    fn construction_lines_have_no_geometry() {
        let x = entity(
            serde_json::json!({ "kind": "xline", "id": 1, "layerId": "y", "attrs": {}, "p": { "x": 1, "y": 2 }, "dir": { "x": 1, "y": 0 } }),
        );
        let s = to_stored(&x, 5256, &none()).unwrap();
        assert!(s.geom.is_none() && s.source_kind == "cad");
    }

    #[test]
    fn broken_objects_are_refused_with_a_reason() {
        let cases = [
            (
                serde_json::json!({ "kind": "polygon", "id": 1, "layerId": "p", "attrs": {}, "pts": [{ "x": 0, "y": 0 }, { "x": 1, "y": 0 }] }),
                "en az 3",
            ),
            (
                serde_json::json!({ "kind": "circle", "id": 1, "layerId": "p", "attrs": {}, "c": { "x": 0, "y": 0 }, "r": 0 }),
                "Yarıçap",
            ),
            (
                serde_json::json!({ "kind": "polyline", "id": 1, "layerId": "p", "attrs": {}, "pts": [{ "x": 0, "y": 0 }, { "x": 1, "y": 0 }], "bulges": [0, 0, 1] }),
                "yay sayısı",
            ),
            (
                serde_json::json!({ "kind": "point", "id": 1, "layerId": "", "attrs": {}, "p": { "x": 0, "y": 0 } }),
                "katman",
            ),
            (
                serde_json::json!({ "kind": "point", "id": 1, "layerId": "p", "attrs": {}, "p": { "x": 2e9, "y": 0 } }),
                "çok büyük",
            ),
            (
                serde_json::json!({ "kind": "ellipse", "id": 1, "layerId": "p", "attrs": {}, "c": { "x": 0, "y": 0 }, "major": { "x": 1, "y": 0 }, "ratio": 1.5, "t0": 0, "t1": 0 }),
                "oranı",
            ),
            (
                serde_json::json!({ "kind": "text", "id": 1, "layerId": "p", "attrs": {}, "p": { "x": 0, "y": 0 }, "text": "A", "height": 2.5, "rotation": 0, "widthFactor": 0 }),
                "genişlik çarpanı",
            ),
        ];
        for (json, why) in cases {
            let err = to_stored(&entity(json), 5256, &none()).unwrap_err();
            assert!(err.contains(why), "{err}");
        }
    }

    /// A text's alignment, width factor and mask are kept with it (docs/adr/0145);
    /// an attribute definition's width factor is checked as a text's.
    #[test]
    fn a_texts_extras_are_kept_and_checked() {
        let t = entity(
            serde_json::json!({ "kind": "text", "id": 1, "layerId": "p", "attrs": {}, "p": { "x": 3, "y": 4 }, "text": "Ada 1284",
            "height": 2.5, "rotation": 30, "align": "middleCenter", "widthFactor": 0.8, "mask": true }),
        );
        let s = to_stored(&t, 5256, &none()).unwrap();
        assert_eq!(s.source_kind, "cad");
        let mut again = from_stored(&s).unwrap();
        if let Entity::Text(x) = &mut again {
            x.base.id = 1;
        }
        assert_eq!(again, t);
        let block: BlockDefinition = serde_json::from_value(serde_json::json!({
            "id": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0003", "name": "Rögar", "base": { "x": 0, "y": 0 }, "entities": [],
            "attributes": [{ "tag": "NO", "p": { "x": 0, "y": 0 }, "height": 0.5, "rotation": 0, "align": "topLeft", "widthFactor": 150 }]
        }))
        .unwrap();
        let err = to_stored_block(&block).unwrap_err();
        assert!(
            err.contains("1. özniteliği") && err.contains("genişlik çarpanı"),
            "{err}"
        );
    }

    /// A leader (docs/adr/0146): its definition is the source, every field
    /// kept; its geometry the line through its vertices; the file's rules.
    #[test]
    fn a_leader_is_kept_and_projected_as_its_line() {
        let l = entity(
            serde_json::json!({ "kind": "leader", "id": 1, "layerId": "p", "attrs": {},
            "pts": [{ "x": 0, "y": 0 }, { "x": 4, "y": 3 }, { "x": 9, "y": 3 }], "text": "Ø150 PVC",
            "height": 2, "rotation": 30, "arrow": "open", "mask": true }),
        );
        let s = to_stored(&l, 5256, &none()).unwrap();
        assert_eq!(s.source_kind, "cad");
        let (g, srid) = ewkb::decode(s.geom.as_deref().unwrap()).unwrap();
        assert_eq!(srid, 5256);
        assert!(
            matches!(&g, Geometry::LineString(p) if p.len() == 3 && p[2].x == 9.0 && p[2].y == 3.0),
            "{g:?}"
        );
        assert_eq!(back(&s, 1), l);
        for (json, words) in [
            (
                serde_json::json!({ "kind": "leader", "id": 1, "layerId": "p", "attrs": {},
                "pts": [{ "x": 0, "y": 0 }], "height": 2, "rotation": 0 }),
                "Kılavuz",
            ),
            (
                serde_json::json!({ "kind": "leader", "id": 1, "layerId": "p", "attrs": {},
                "pts": [{ "x": 0, "y": 0 }, { "x": 4, "y": 3 }], "height": 0, "rotation": 0 }),
                "Kılavuz yüksekliği",
            ),
            (
                serde_json::json!({ "kind": "leader", "id": 1, "layerId": "p", "attrs": {},
                "pts": [{ "x": 0, "y": 0 }, { "x": 4, "y": 3 }], "text": "", "height": 2, "rotation": 0 }),
                "notu boş",
            ),
        ] {
            let err = to_stored(&entity(json), 5256, &none()).unwrap_err();
            assert!(err.contains(words), "{words}: {err}");
        }
    }
}
