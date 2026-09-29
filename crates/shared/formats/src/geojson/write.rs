//! GeoJSON out (docs/adr/0046): a FeatureCollection with one feature per
//! object, one feature per line. Coordinates are the shortest decimals that
//! read back to the same float64, never rounded or transformed: a WGS 84
//! project (EPSG:4326) gives RFC 7946 GeoJSON, any other its own
//! coordinates with the 2008 `crs` member naming them (RFC 7946 has no such
//! member; the export window says so before writing).
//!
//! What GeoJSON cannot hold is written as the nearest thing it can, and
//! said: curves (circles, arcs, ellipses, splines, bulged paths) sampled
//! exactly as the app samples its own; hatches as their areas; area rings
//! turned to RFC 7946's right-hand rule. Text, dimensions and infinite
//! lines are left out. A multi-part area is a MultiPolygon (docs/adr/0143).
//! Attributes are written as text properties; layer and
//! label ride in the `kentos` member, which KentOS reads back. A vertex with
//! an elevation is a position of three numbers, one without a position of
//! two (docs/adr/0142): RFC 7946 lets a list mix them; a point sampled along
//! an arc edge has the elevation the edge has there.

use std::collections::HashMap;
use std::fmt::Write as _;

use kentos_contracts::{Entity, ExportReport, GeoJsonLayer, GeoJsonWriteInput, Vec2};
use kentos_geometry_core::Vec2 as CoreVec2;
use kentos_geometry_core::entity::{ellipse_geom, polygon_ring, tessellate_circle};
use kentos_geometry_core::geom::arc::{ArcGeom, DEFAULT_STEP, tessellate_arc};
use kentos_geometry_core::geom::bulge::bulge_path_outline;
use kentos_geometry_core::geom::ellipse::{is_full_ellipse, tessellate_ellipse};
use kentos_geometry_core::geom::spline::catmull_rom;
use serde::{Deserialize, Deserializer};

use crate::dxf::Objects;
use crate::geom::{bulge_path_zs, has_arcs, to_core};
use crate::gis::{Ring, Zs, open_ring, shoelace};
use crate::json::quote;
use crate::num::plain;
use crate::report::Report;

/// Segments per turn of a sampled curve: the app's own (72, the core's `DEFAULT_STEP`).
/// Each curve is sampled by the core function the app's outline uses for it
/// (`entity_outline` with 72 segments), called directly so the formats module
/// does not take in the outline of every other kind (dimensions, text).
const SEGMENTS: f64 = 72.0;
/// Points per turn of an ellipse in that outline (`max(64, 2 × segments)`).
const ELLIPSE_PER_TURN: f64 = 2.0 * SEGMENTS;
/// Points per span of a spline in that outline.
const SPLINE_PER_SPAN: f64 = 16.0;

/// A feature's geometry as it will be written.
enum Geometry {
    Point(Vec2, Option<f64>),
    Line(Ring),
    /// Closed rings, the outline first, turned to the right-hand rule.
    Area(Vec<Ring>),
    /// A multi-part area: each part's rings as [`Geometry::Area`]'s.
    Areas(Vec<Vec<Ring>>),
}

fn from_core(pts: Vec<CoreVec2>) -> Vec<Vec2> {
    pts.into_iter().map(|p| Vec2 { x: p.x, y: p.y }).collect()
}

fn core(p: Vec2) -> CoreVec2 {
    CoreVec2::new(p.x, p.y)
}

/// A ring closed by repeating its first point (and its elevation).
fn closed(mut ring: Ring) -> Ring {
    if let Some(&first) = ring.pts.first()
        && ring.pts.last() != Some(&first)
    {
        ring.pts.push(first);
        if let Some(zs) = ring.zs.as_mut() {
            let z = zs.first().copied().flatten();
            zs.push(z);
        }
    }
    ring
}

/// A ring (without its closing point) turned counter-clockwise (`ccw`) or
/// clockwise, keeping its first vertex (each vertex its elevation); the ring closed.
fn oriented(ring: Ring, ccw: bool, turned: &mut bool) -> Ring {
    let (mut ring, _, _) = open_ring(ring);
    let s = shoelace(&ring.pts);
    if (ccw && s < 0.0) || (!ccw && s > 0.0) {
        if let Some(rest) = ring.pts.get_mut(1..) {
            rest.reverse();
        }
        if let Some(rest) = ring.zs.as_mut().and_then(|zs| zs.get_mut(1..)) {
            rest.reverse();
        }
        *turned = true;
    }
    closed(ring)
}

/// An area from its outline and holes; None when the outline has fewer than three points.
fn area(outline: Ring, holes: Vec<Ring>, rep: &mut Report) -> Option<Geometry> {
    let mut turned = false;
    if open_ring(outline.clone()).0.pts.len() < 3 {
        return None;
    }
    let mut rings = vec![oriented(outline, true, &mut turned)];
    for h in holes {
        if open_ring(h.clone()).0.pts.len() >= 3 {
            rings.push(oriented(h, false, &mut turned));
        } else {
            rep.skip("Delik", "üçten az köşesi var; yazılmadı", 0);
        }
    }
    if turned {
        rep.note(
            "Halka yönü",
            "RFC 7946'nın sağ el kuralına çevrildi (dış sınır saat yönünün tersine, delikler saat yönünde); köşeler aynı",
            0,
        );
    }
    Some(Geometry::Area(rings))
}

fn curve(rep: &mut Report, what: &str, ring: Ring) -> Option<Geometry> {
    rep.note(
        what,
        "GeoJSON'da eğri yok: uygulamanın kendi örneklemesiyle (turda 72 adım) çizgi olarak yazıldı",
        0,
    );
    (ring.pts.len() >= 2).then_some(Geometry::Line(ring))
}

/// A path's elevations when it has as many as vertices (else it has none).
fn heights(zs: &Zs, vertices: usize) -> Option<&[Option<f64>]> {
    zs.as_deref().filter(|z| z.len() == vertices)
}

/// Whether some elevations are not numbers a file can hold.
fn zs_finite(zs: &Zs) -> bool {
    zs.iter().flatten().flatten().all(|z| z.is_finite())
}

/// Whether a geometry has vertices with an elevation and vertices without, in a
/// ring or between its rings (a hole of a 3D area that has none).
fn mixed(rings: &[&Ring]) -> bool {
    let has = |r: &&Ring| r.zs.iter().flatten().any(Option::is_some);
    let lacks = |r: &&Ring| r.zs.as_ref().is_none_or(|z| z.iter().any(Option::is_none));
    rings.iter().any(has) && rings.iter().any(lacks)
}

/// What an object becomes, or None (said in the report).
fn geometry(e: &Entity, rep: &mut Report) -> Option<Geometry> {
    let g = match e {
        Entity::Point(p) => Some(Geometry::Point(p.p, p.z)),
        Entity::Line(l) => Some(Geometry::Line(Ring::new(vec![l.a, l.b], [l.za, l.zb]))),
        Entity::Polyline(p) => {
            if p.holes.as_ref().is_some_and(|h| !h.is_empty()) {
                rep.note(
                    "Çoklu çizginin delikleri",
                    "açık çizginin delikleri GeoJSON'da gösterilemez; yazılmadı",
                    0,
                );
            }
            let zs = heights(&p.zs, p.pts.len());
            match p.bulges.as_deref() {
                Some(b) if has_arcs(b) => {
                    let pts = from_core(bulge_path_outline(
                        &to_core(&p.pts),
                        Some(b),
                        false,
                        DEFAULT_STEP,
                    ));
                    let zs = zs.map(|z| bulge_path_zs(&p.pts, b, z, false));
                    curve(
                        rep,
                        "Yaylı çoklu çizgi",
                        Ring::new(pts, zs.into_iter().flatten()),
                    )
                }
                _ => (p.pts.len() >= 2).then(|| {
                    Geometry::Line(Ring::new(p.pts.clone(), zs.into_iter().flatten().copied()))
                }),
            }
        }
        Entity::Polygon(p) => {
            let mut arcs = false;
            let mut ring = |pts: &[Vec2], bulges: Option<&[f64]>, zs: &Zs| {
                arcs |= bulges.is_some_and(has_arcs);
                let outline = from_core(polygon_ring(&to_core(pts), bulges));
                let zs = heights(zs, pts.len())
                    .map(|z| bulge_path_zs(pts, bulges.unwrap_or(&[]), z, true));
                Ring::new(outline, zs.into_iter().flatten())
            };
            // The area's own fields are its first part (docs/adr/0143).
            let mut parts = vec![(
                ring(&p.pts, p.bulges.as_deref(), &p.zs),
                p.holes
                    .iter()
                    .flatten()
                    .map(|h| ring(&h.pts, h.bulges.as_deref(), &h.zs))
                    .collect::<Vec<_>>(),
            )];
            for part in p.parts.iter().flatten() {
                let holes = part.holes.iter().flatten();
                let holes = holes.map(|h| ring(&h.pts, h.bulges.as_deref(), &h.zs));
                let holes: Vec<Ring> = holes.collect();
                parts.push((ring(&part.pts, part.bulges.as_deref(), &part.zs), holes));
            }
            if arcs {
                rep.note(
                    "Yaylı alan",
                    "GeoJSON'da eğri yok: yayları uygulamanın kendi örneklemesiyle (turda 72 adım) yazıldı",
                    0,
                );
            }
            let several = parts.len() > 1;
            let mut areas: Vec<Vec<Ring>> = Vec::new();
            for (outline, holes) in parts {
                match area(outline, holes, rep) {
                    Some(Geometry::Area(rings)) => areas.push(rings),
                    _ if several => rep.skip("Parça", "üçten az köşesi var; yazılmadı", 0),
                    _ => {}
                }
            }
            match areas.len() {
                0 => None,
                1 => areas.pop().map(Geometry::Area),
                _ => Some(Geometry::Areas(areas)),
            }
        }
        Entity::Circle(c) => {
            let pts = from_core(tessellate_circle(core(c.c), c.r, SEGMENTS));
            curve(rep, "Daire", closed(Ring::flat(pts)))
        }
        Entity::Arc(a) => curve(
            rep,
            "Yay",
            Ring::flat(from_core(tessellate_arc(
                &ArcGeom {
                    c: core(a.c),
                    r: a.r,
                    a0: a.a0,
                    a1: a.a1,
                },
                DEFAULT_STEP,
            ))),
        ),
        Entity::Ellipse(el) => {
            let g = ellipse_geom(core(el.c), core(el.major), el.ratio, el.t0, el.t1);
            let pts = from_core(tessellate_ellipse(&g, ELLIPSE_PER_TURN.max(64.0)));
            let ring = Ring::flat(pts);
            curve(
                rep,
                "Elips",
                if is_full_ellipse(&g) {
                    closed(ring)
                } else {
                    ring
                },
            )
        }
        Entity::Spline(s) => {
            let mut pts = from_core(catmull_rom(&to_core(&s.pts), s.closed, SPLINE_PER_SPAN));
            // The curve passes through its fit points: its ends are the source's own
            // coordinates, not a sample off by the last bit.
            if let (Some(first), Some(&start)) = (pts.first_mut(), s.pts.first()) {
                *first = start;
            }
            if s.closed {
                if let Some(first) = pts.first().copied() {
                    pts.pop();
                    pts.push(first);
                }
            } else if let (Some(last), Some(&end)) = (pts.last_mut(), s.pts.last()) {
                *last = end;
            }
            curve(rep, "Eğri (spline)", Ring::flat(pts))
        }
        Entity::Hatch(h) => {
            rep.note(
                "Tarama",
                "deseni GeoJSON'da gösterilemez; sınırı alan (Polygon) olarak yazıldı",
                0,
            );
            let holes = h.holes.iter().flatten().cloned().map(Ring::flat);
            area(Ring::flat(h.ring.clone()), holes.collect(), rep)
        }
        Entity::Text(_) => {
            rep.skip("Yazı", "GeoJSON'da yazı nesnesi yok; yazılmadı", 0);
            return None;
        }
        Entity::Dimension(_) => {
            rep.skip("Ölçü", "GeoJSON'da ölçü nesnesi yok; yazılmadı", 0);
            return None;
        }
        Entity::Xline(_) | Entity::Ray(_) => {
            rep.skip(
                "Sonsuz doğru",
                "uçsuz doğru ve ışın GeoJSON'da gösterilemez; yazılmadı",
                0,
            );
            return None;
        }
    };
    let Some(g) = g else {
        rep.skip("Geçersiz nesne", "yeterli köşesi yok; yazılmadı", 0);
        return None;
    };
    let finite = |p: &Vec2| p.x.is_finite() && p.y.is_finite();
    let ok = match &g {
        Geometry::Point(p, z) => finite(p) && z.is_none_or(f64::is_finite),
        Geometry::Line(r) => r.pts.iter().all(finite) && zs_finite(&r.zs),
        Geometry::Area(rings) => rings
            .iter()
            .all(|r| r.pts.iter().all(finite) && zs_finite(&r.zs)),
        Geometry::Areas(parts) => parts
            .iter()
            .flatten()
            .all(|r| r.pts.iter().all(finite) && zs_finite(&r.zs)),
    };
    if !ok {
        rep.skip(
            "Sonlu olmayan koordinat",
            "koordinatı sayı değil ya da sonsuz; yazılmadı",
            0,
        );
        return None;
    }
    let mixes = match &g {
        Geometry::Line(r) => mixed(&[r]),
        Geometry::Area(rings) => mixed(&rings.iter().collect::<Vec<_>>()),
        Geometry::Areas(parts) => mixed(&parts.iter().flatten().collect::<Vec<_>>()),
        Geometry::Point(..) => false,
    };
    if mixes {
        rep.note(
            "Kotsuz köşe",
            "kotu olmayan köşeler iki sayıyla yazıldı (RFC 7946 iki ve üç sayılı konumların bir dizide karışmasına izin verir; bazı programlar onları 0 kotlu okur)",
            0,
        );
    }
    Some(g)
}

fn position(p: Vec2, z: Option<f64>, out: &mut String) {
    out.push('[');
    out.push_str(&plain(p.x));
    out.push(',');
    out.push_str(&plain(p.y));
    if let Some(z) = z {
        out.push(',');
        out.push_str(&plain(z));
    }
    out.push(']');
}

fn positions(ring: &Ring, out: &mut String) {
    out.push('[');
    for (i, p) in ring.pts.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        let z = ring.zs.as_ref().and_then(|zs| zs.get(i).copied().flatten());
        position(*p, z, out);
    }
    out.push(']');
}

fn write_geometry(g: &Geometry, out: &mut String) {
    match g {
        Geometry::Point(p, z) => {
            out.push_str("{\"type\":\"Point\",\"coordinates\":");
            position(*p, *z, out);
        }
        Geometry::Line(ring) => {
            out.push_str("{\"type\":\"LineString\",\"coordinates\":");
            positions(ring, out);
        }
        Geometry::Area(rings) => {
            out.push_str("{\"type\":\"Polygon\",\"coordinates\":");
            polygon(rings, out);
        }
        Geometry::Areas(parts) => {
            out.push_str("{\"type\":\"MultiPolygon\",\"coordinates\":[");
            for (k, rings) in parts.iter().enumerate() {
                if k > 0 {
                    out.push(',');
                }
                polygon(rings, out);
            }
            out.push(']');
        }
    }
    out.push('}');
}

/// A Polygon's coordinates: its rings, the outline first.
fn polygon(rings: &[Ring], out: &mut String) {
    out.push('[');
    for (i, r) in rings.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        positions(r, out);
    }
    out.push(']');
}

fn kind(e: &Entity) -> &'static str {
    match e {
        Entity::Point(_) => "point",
        Entity::Line(_) => "line",
        Entity::Polyline(_) => "polyline",
        Entity::Polygon(_) => "polygon",
        Entity::Circle(_) => "circle",
        Entity::Arc(_) => "arc",
        Entity::Ellipse(_) => "ellipse",
        Entity::Spline(_) => "spline",
        Entity::Xline(_) => "xline",
        Entity::Ray(_) => "ray",
        Entity::Text(_) => "text",
        Entity::Dimension(_) => "dimension",
        Entity::Hatch(_) => "hatch",
    }
}

fn base(e: &Entity) -> &kentos_contracts::EntityBase {
    match e {
        Entity::Point(x) => &x.base,
        Entity::Line(x) => &x.base,
        Entity::Polyline(x) | Entity::Polygon(x) => &x.base,
        Entity::Circle(x) => &x.base,
        Entity::Arc(x) => &x.base,
        Entity::Ellipse(x) => &x.base,
        Entity::Spline(x) => &x.base,
        Entity::Xline(x) | Entity::Ray(x) => &x.base,
        Entity::Text(x) => &x.base,
        Entity::Dimension(x) => &x.base,
        Entity::Hatch(x) => &x.base,
    }
}

/// Writes the objects as a GeoJSON FeatureCollection.
pub fn write(input: &GeoJsonWriteInput) -> (Vec<u8>, ExportReport) {
    let names: HashMap<&str, &str> = input
        .layers
        .iter()
        .map(|l: &GeoJsonLayer| (l.id.as_str(), l.name.as_str()))
        .collect();
    let mut rep = Report::default();
    let mut out = String::with_capacity(64 + input.entities.len() * 160);
    out.push_str("{\"type\":\"FeatureCollection\",\"name\":");
    quote(&input.name, &mut out);
    if input.srid != 4326 {
        let _ = write!(
            out,
            ",\"crs\":{{\"type\":\"name\",\"properties\":{{\"name\":\"urn:ogc:def:crs:EPSG::{}\"}}}}",
            input.srid
        );
        rep.note(
            "Koordinat sistemi",
            &format!(
                "EPSG:{}; RFC 7946 WGS 84 boylam, enlem ister: koordinatlar dönüştürülmeden projenin sisteminde yazıldı ve eski (2008) crs üyesiyle adlandırıldı",
                input.srid
            ),
            0,
        );
    }
    out.push_str(",\"features\":[\n");
    let mut first = true;
    for e in &input.entities {
        let Some(g) = geometry(e, &mut rep) else {
            continue;
        };
        let b = base(e);
        if b.color.is_some() || b.symbol.is_some() {
            rep.note(
                "Renk ve sembol",
                "nesnenin kendi rengi ve sembolü GeoJSON'a yazılmadı",
                0,
            );
        }
        if !first {
            out.push_str(",\n");
        }
        first = false;
        out.push_str("{\"type\":\"Feature\",\"properties\":{");
        for (i, (k, v)) in b.attrs.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            quote(k, &mut out);
            out.push(':');
            quote(v, &mut out);
        }
        out.push_str("},\"geometry\":");
        write_geometry(&g, &mut out);
        let layer = names.get(b.layer_id.as_str());
        let label = b.label.as_deref().filter(|l| !l.is_empty());
        if layer.is_some() || label.is_some() {
            out.push_str(",\"kentos\":{");
            if let Some(l) = layer {
                out.push_str("\"layer\":");
                quote(l, &mut out);
            }
            if let Some(l) = label {
                if layer.is_some() {
                    out.push(',');
                }
                out.push_str("\"label\":");
                quote(l, &mut out);
            }
            out.push('}');
        }
        out.push('}');
        rep.count(kind(e));
    }
    out.push_str("\n]}\n");
    (out.into_bytes(), rep.export())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Input {
    entities: Objects,
    layers: Vec<GeoJsonLayer>,
    srid: u32,
    name: String,
}

/// A `GeoJsonWriteInput` read with the DXF writer's visitor for the objects
/// (no derived code for the tagged object list in the formats module).
pub struct WriteInput(pub GeoJsonWriteInput);

impl<'de> Deserialize<'de> for WriteInput {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<WriteInput, D::Error> {
        let i = Input::deserialize(d)?;
        Ok(WriteInput(GeoJsonWriteInput {
            entities: i.entities.0,
            layers: i.layers,
            srid: i.srid,
            name: i.name,
        }))
    }
}

/// Reads a `GeoJsonWriteInput` from its JSON (as the contract writes it).
pub fn input_from_json(text: &str) -> Result<GeoJsonWriteInput, String> {
    serde_json::from_str::<WriteInput>(text)
        .map(|w| w.0)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use kentos_contracts::GeoJsonReadOptions;

    const INPUT: &str = r#"{"entities":[
      {"kind":"point","id":1,"layerId":"k","attrs":{"Ad":"P1","Z":"105.20"},"label":"P1","p":{"x":452345.123,"y":4412345.678},"z":105.2},
      {"kind":"line","id":2,"layerId":"k","attrs":{},"a":{"x":0,"y":0},"b":{"x":1,"y":0.1}},
      {"kind":"polygon","id":3,"layerId":"p","attrs":{"ada":"12"},"pts":[{"x":0,"y":0},{"x":0,"y":10},{"x":10,"y":10},{"x":10,"y":0}],
       "holes":[{"pts":[{"x":2,"y":2},{"x":4,"y":2},{"x":4,"y":4}]}]},
      {"kind":"text","id":4,"layerId":"k","attrs":{},"p":{"x":1,"y":1},"text":"Ağaç","height":2.5,"rotation":0},
      {"kind":"circle","id":5,"layerId":"p","attrs":{},"c":{"x":5,"y":5},"r":2}
    ],"layers":[{"id":"k","name":"Kotlar"},{"id":"p","name":"Parseller"}],"srid":5256,"name":"Pafta \"A\""}"#;

    #[test]
    fn writes_what_its_reader_reads_back() {
        let input = input_from_json(INPUT).expect("input");
        let (bytes, report) = write(&input);
        let text = String::from_utf8(bytes.clone()).expect("utf8");
        assert!(text.starts_with("{\"type\":\"FeatureCollection\",\"name\":\"Pafta \\\"A\\\"\",\"crs\":{\"type\":\"name\",\"properties\":{\"name\":\"urn:ogc:def:crs:EPSG::5256\"}},\"features\":[\n"), "{text}");
        assert!(
            text.contains("\"coordinates\":[452345.123,4412345.678,105.2]"),
            "{text}"
        );
        let back = crate::geojson::read(
            &bytes,
            &GeoJsonReadOptions {
                layer: "x".into(),
                max_entities: 0,
            },
        )
        .expect("read back");
        assert_eq!(back.declared_crs.and_then(|d| d.srid), Some(5256));
        assert_eq!(back.entities.len(), 4);
        let Entity::Point(p) = &back.entities[0] else {
            panic!()
        };
        assert_eq!((p.p.x, p.p.y, p.z), (452345.123, 4412345.678, Some(105.2)));
        assert_eq!(
            (p.base.layer_id.as_str(), p.base.label.as_deref()),
            ("Kotlar", Some("P1"))
        );
        assert_eq!(p.base.attrs.get("Z").map(String::as_str), Some("105.20"));
        // The outline was clockwise: turned, its first vertex kept; the hole the other way.
        let Entity::Polygon(g) = &back.entities[2] else {
            panic!()
        };
        assert_eq!(g.pts[0], Vec2 { x: 0.0, y: 0.0 });
        assert!(shoelace(&g.pts) > 0.0);
        assert!(shoelace(&g.holes.as_ref().expect("holes")[0].pts) < 0.0);
        // The circle: 72 sides, closed.
        let Entity::Polyline(c) = &back.entities[3] else {
            panic!()
        };
        assert_eq!(c.pts.len(), 73);
        assert_eq!(c.pts.first(), c.pts.last());
        let what: Vec<&str> = report.notes.iter().map(|i| i.what.as_str()).collect();
        assert!(
            what.contains(&"Halka yönü")
                && what.contains(&"Daire")
                && what.contains(&"Koordinat sistemi"),
            "{what:?}"
        );
        assert_eq!(
            report
                .skipped
                .iter()
                .map(|i| i.what.as_str())
                .collect::<Vec<_>>(),
            vec!["Yazı"]
        );
        assert_eq!(report.counts.values().sum::<u32>(), 4);
    }

    /// Objects with elevations: a line with one end's, a path with a vertex without, an area whose outline
    /// turns (clockwise) with heights that turn with it and a hole with none, a path with an arc.
    const ELEVATED: &str = r#"{"entities":[
      {"kind":"line","id":1,"layerId":"k","attrs":{},"a":{"x":1,"y":2},"b":{"x":3,"y":4},"za":10.5,"zb":null},
      {"kind":"polyline","id":2,"layerId":"k","attrs":{},"pts":[{"x":0,"y":0},{"x":1,"y":0},{"x":2,"y":0}],"zs":[1,null,-2.5]},
      {"kind":"polygon","id":3,"layerId":"k","attrs":{},"pts":[{"x":0,"y":0},{"x":0,"y":10},{"x":10,"y":10},{"x":10,"y":0}],"zs":[100,101,102,103],
       "holes":[{"pts":[{"x":2,"y":2},{"x":4,"y":2},{"x":4,"y":4}]}]},
      {"kind":"polyline","id":4,"layerId":"k","attrs":{},"pts":[{"x":0,"y":0},{"x":2,"y":0},{"x":2,"y":5}],"bulges":[1,0],"zs":[100,200,300]},
      {"kind":"polyline","id":5,"layerId":"k","attrs":{},"pts":[{"x":0,"y":0},{"x":1,"y":0}],"zs":[7]}
    ],"layers":[{"id":"k","name":"Kotlar"}],"srid":5256,"name":"kotlu"}"#;

    #[test]
    fn a_vertex_with_an_elevation_has_three_numbers_and_one_without_has_two() {
        let (bytes, report) = write(&input_from_json(ELEVATED).expect("input"));
        let text = String::from_utf8(bytes.clone()).expect("utf8");
        // A line and a path mix the two; a list of elevations that does not match the vertices is none.
        assert!(
            text.contains("\"coordinates\":[[1,2,10.5],[3,4]]"),
            "{text}"
        );
        assert!(
            text.contains("\"coordinates\":[[0,0,1],[1,0],[2,0,-2.5]]"),
            "{text}"
        );
        assert!(text.contains("\"coordinates\":[[0,0],[1,0]]"), "{text}");
        // The clockwise outline turned counter-clockwise keeping its first vertex, each height with its vertex.
        assert!(
            text.contains("[[[0,0,100],[10,0,103],[10,10,102],[0,10,101],[0,0,100]],"),
            "{text}"
        );
        // A point sampled along the arc is at the blend of the heights of the edge's ends by arc length: 36 segments a half turn.
        assert!(text.contains(",102.77777777777777],"), "{text}");
        let back = crate::geojson::read(
            &bytes,
            &GeoJsonReadOptions {
                layer: "x".into(),
                max_entities: 0,
            },
        )
        .expect("read back");
        let Entity::Line(l) = &back.entities[0] else {
            panic!("{:?}", back.entities[0])
        };
        assert_eq!((l.za, l.zb), (Some(10.5), None));
        let Entity::Polyline(p) = &back.entities[1] else {
            panic!("{:?}", back.entities[1])
        };
        assert_eq!(p.zs, Some(vec![Some(1.0), None, Some(-2.5)]));
        let Entity::Polygon(g) = &back.entities[2] else {
            panic!("{:?}", back.entities[2])
        };
        assert_eq!(
            g.zs,
            Some(vec![Some(100.0), Some(103.0), Some(102.0), Some(101.0)])
        );
        assert!(g.holes.as_ref().is_some_and(|h| h[0].zs.is_none()));
        let Entity::Polyline(arc) = &back.entities[3] else {
            panic!("{:?}", back.entities[3])
        };
        let zs = arc.zs.as_ref().expect("heights");
        assert_eq!(
            (zs.len(), zs[0], zs[18], zs[36], zs[37]),
            (
                arc.pts.len(),
                Some(100.0),
                Some(150.0),
                Some(200.0),
                Some(300.0)
            )
        );
        assert!(matches!(&back.entities[4], Entity::Line(l) if (l.za, l.zb) == (None, None)));
        // What mixes is said, once for each object that does.
        let mixed = report
            .notes
            .iter()
            .find(|n| n.what == "Kotsuz köşe")
            .expect("the note");
        assert_eq!(mixed.count, 3, "{:?}", report.notes);
        assert!(report.skipped.is_empty());
    }

    #[test]
    fn a_non_finite_elevation_leaves_the_object_out() {
        let mut input = input_from_json(ELEVATED).expect("input");
        if let Entity::Line(l) = &mut input.entities[0] {
            l.zb = Some(f64::INFINITY);
        }
        let (_, report) = write(&input);
        assert_eq!(
            report
                .skipped
                .iter()
                .map(|i| i.what.as_str())
                .collect::<Vec<_>>(),
            ["Sonlu olmayan koordinat"]
        );
        assert_eq!(report.counts.values().sum::<u32>(), 4);
    }

    #[test]
    fn a_wgs84_project_is_written_as_rfc_7946_has_it() {
        let mut input = input_from_json(INPUT).expect("input");
        input.srid = 4326;
        let (bytes, report) = write(&input);
        let text = String::from_utf8(bytes).expect("utf8");
        assert!(!text.contains("\"crs\""));
        assert!(!report.notes.iter().any(|i| i.what == "Koordinat sistemi"));
    }

    #[test]
    fn reads_what_the_contracts_own_deserializer_reads() {
        let ours = input_from_json(INPUT).expect("input");
        let derived: GeoJsonWriteInput = serde_json::from_str(INPUT).expect("contract");
        assert_eq!(ours, derived);
    }
}
