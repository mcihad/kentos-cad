//! A drawing object (`Entity`, the contract) as a stored feature and back
//! (CLAUDE.md §15, docs/adr/0006).
//!
//! - Point, line, straight polyline and straight polygon: the PostGIS
//!   geometry is the source, written and read as EWKB (every bit kept).
//! - Everything else: `cad_definition` (the entity's geometric fields exactly
//!   as in the contract) is the source; the geometry is its linear projection
//!   within `PROJECTION_TOLERANCE`, made by `geometry-core`, and null for
//!   construction lines and rays, which have no finite geometry.
//!
//! Negative zero becomes zero on the way in (PostgreSQL's numeric has none),
//! so every kind round-trips to exactly the value that was accepted.

use kentos_contracts::Entity;
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
const BASE_KEYS: [&str; 7] = ["kind", "id", "layerId", "color", "attrs", "label", "symbol"];

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

fn positive(what: &str, v: f64) -> Result<(), String> {
    if v > 0.0 {
        Ok(())
    } else {
        Err(format!("{what} sıfırdan büyük olmalı"))
    }
}

/// Rules the contract's types cannot say: point counts, positive sizes, text lengths.
fn validate(e: &Entity) -> Result<(), String> {
    use Entity::*;
    let base = match e {
        Point(x) => &x.base,
        Line(x) => &x.base,
        Polyline(x) | Polygon(x) => &x.base,
        Circle(x) => &x.base,
        Arc(x) => &x.base,
        Ellipse(x) => &x.base,
        Spline(x) => &x.base,
        Xline(x) | Ray(x) => &x.base,
        Text(x) => &x.base,
        Dimension(x) => &x.base,
        Hatch(x) => &x.base,
    };
    if base.layer_id.is_empty() || base.layer_id.len() > 200 {
        return Err("katman kimliği boş ya da çok uzun".into());
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
        }
        Polygon(x) => {
            check_len("Alan", x.pts.len(), 3)?;
            check_bulges("Alan", &x.bulges, x.pts.len())?;
            for h in x.holes.iter().flatten() {
                check_len("Ada", h.pts.len(), 3)?;
                check_bulges("Ada", &h.bulges, h.pts.len())?;
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
        }
        Dimension(x) => positive("Ölçü yazısı yüksekliği", x.height)?,
        Hatch(x) => {
            check_len("Tarama sınırı", x.ring.len(), 3)?;
            for h in x.holes.iter().flatten() {
                check_len("Tarama adası", h.len(), 3)?;
            }
            positive("Tarama aralığı", x.pattern.spacing)?;
        }
        Point(_) | Line(_) => {}
    }
    Ok(())
}

/// Whether the geometry alone holds the object (no arcs, no empty hole list).
fn geometry_is_source(e: &Entity) -> bool {
    match e {
        Entity::Point(_) | Entity::Line(_) => true,
        Entity::Polyline(x) => x.bulges.is_none() && x.holes.is_none(),
        Entity::Polygon(x) => {
            x.bulges.is_none()
                && x.holes
                    .as_ref()
                    .is_none_or(|hs| !hs.is_empty() && hs.iter().all(|h| h.bulges.is_none()))
        }
        _ => false,
    }
}

/// The linear PostGIS geometry of an object (`None`: nothing finite to draw).
fn projection(e: &Entity) -> Option<Geometry> {
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
            Geometry::Polygon(rings)
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
    })
}

/// Checks an object and turns it into a feature row's content for SRID `srid`.
pub fn to_stored(entity: &Entity, srid: u32) -> Result<Stored, String> {
    let mut value = serde_json::to_value(entity).map_err(|e| e.to_string())?;
    normalize(&mut value, "nesne")?;
    let entity: Entity = serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
    validate(&entity)?;
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
    let properties = map
        .get("attrs")
        .cloned()
        .unwrap_or_else(|| Value::Object(Map::new()));
    let geom = projection(&entity).map(|g| ewkb::encode(&g, srid));
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
    })
}

fn xy(v: P) -> Value {
    serde_json::json!({ "x": v.x, "y": v.y })
}

fn ring_value(r: &[P]) -> Value {
    Value::Array(r.iter().copied().map(xy).collect())
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
                    m.insert("pts".into(), ring_value(&rings[0]));
                    if rings.len() > 1 {
                        let holes: Vec<Value> = rings[1..]
                            .iter()
                            .map(|r| serde_json::json!({ "pts": ring_value(r) }))
                            .collect();
                        m.insert("holes".into(), Value::Array(holes));
                    }
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
    serde_json::from_value(Value::Object(map)).map_err(|e| format!("nesne okunamadı: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entity(json: Value) -> Entity {
        serde_json::from_value(json).unwrap()
    }

    #[test]
    fn simple_shapes_keep_their_geometry_as_source() {
        let line = entity(
            serde_json::json!({ "kind": "line", "id": 7, "layerId": "cizim", "attrs": { "Ad": "x" },
            "a": { "x": 486512.34, "y": 4420210.5 }, "b": { "x": 486520.0, "y": -0.0 } }),
        );
        let s = to_stored(&line, 5256).unwrap();
        assert_eq!((s.source_kind, s.kind.as_str()), ("geom", "line"));
        assert!(s.cad_definition.is_none());
        let back = from_stored(&s).unwrap();
        let Entity::Line(l) = &back else { panic!() };
        assert_eq!(l.b.y.to_bits(), 0.0f64.to_bits());
        assert_eq!(l.base.id, 0);
        assert_eq!(l.base.attrs["Ad"], "x");
    }

    #[test]
    fn arcs_and_empty_hole_lists_keep_the_definition() {
        let bulged = entity(
            serde_json::json!({ "kind": "polygon", "id": 1, "layerId": "p", "attrs": {},
            "pts": [{ "x": 0, "y": 0 }, { "x": 10, "y": 0 }, { "x": 10, "y": 10 }], "bulges": [0, 0.5, 0] }),
        );
        let s = to_stored(&bulged, 5256).unwrap();
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
        assert_eq!(to_stored(&empty_holes, 5256).unwrap().source_kind, "cad");
    }

    #[test]
    fn construction_lines_have_no_geometry() {
        let x = entity(
            serde_json::json!({ "kind": "xline", "id": 1, "layerId": "y", "attrs": {}, "p": { "x": 1, "y": 2 }, "dir": { "x": 1, "y": 0 } }),
        );
        let s = to_stored(&x, 5256).unwrap();
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
        ];
        for (json, why) in cases {
            let err = to_stored(&entity(json), 5256).unwrap_err();
            assert!(err.contains(why), "{err}");
        }
    }
}
