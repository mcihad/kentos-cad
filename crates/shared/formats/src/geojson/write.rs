//! GeoJSON out (docs/adr/0046): a FeatureCollection with one feature per
//! object, one feature per line. Coordinates are the shortest decimals that
//! read back to the same float64, never rounded or transformed: a WGS 84
//! project (EPSG:4326) gives RFC 7946 GeoJSON, any other its own
//! coordinates with the 2008 `crs` member naming them (RFC 7946 has no such
//! member; the export window says so before writing). Asked to
//! ([`write_wgs84`], docs/adr/0222 §7), the writer moves them into WGS 84
//! longitude and latitude as Koordinat dönüştür moves a point (the
//! project's datum choices taken), rounded once to ten decimals (about
//! 0.01 mm), and writes RFC 7946 GeoJSON without the `crs` member; an object
//! one of whose vertices the move does not reach is left out and said;
//! elevations are written as they are.
//!
//! What GeoJSON cannot hold is written as the nearest thing it can, and
//! said: curves (circles, arcs, ellipses, splines, bulged paths) sampled
//! exactly as the app samples its own; hatches as their areas; area rings
//! turned to RFC 7946's right-hand rule. Text, dimensions and infinite
//! lines are left out. A multi-part area is a MultiPolygon (docs/adr/0143), a
//! multi-part polyline a MultiLineString and a multi-point object a
//! MultiPoint (docs/adr/0174).
//! GeoJSON has no blocks: an insert is its block's objects placed (the
//! core's expansion, nested blocks opened), one GeometryCollection with the
//! insert's attributes (docs/adr/0144 §5); it reads back as those objects.
//! Attributes are written as text properties, but on a layer with fields
//! (docs/adr/0199 §6) a number field's value in its canonical text is a JSON
//! number when its nearest float64 reads back to the same value (else text),
//! a yes or no field's a JSON boolean; layer and
//! label ride in the `kentos` member, which KentOS reads back. A vertex with
//! an elevation is a position of three numbers, one without a position of
//! two (docs/adr/0142): RFC 7946 lets a list mix them; a point sampled along
//! an arc edge has the elevation the edge has there.

use std::collections::HashMap;
use std::fmt::Write as _;

use kentos_contracts::{
    ExportReport, GeoJsonLayer, GeoJsonWriteInput, LayerField, LayerFieldKind, Vec2, check_value,
    compare_decimals,
};
use kentos_geometry_core::crs::{Choice, System, Unreached, transform_in};
use serde::{Deserialize, Deserializer};

use crate::blocks::Placing;
use crate::dxf::{Definitions, Objects};
use crate::export_geom::{GEOJSON, Geometry, geometry};
use crate::gis::Ring;
use crate::json::quote;
use crate::kml::write::{DEGREE_DECIMALS, WGS84, say_unreached, tally};
use crate::num::{plain, rounded};
use crate::report::Report;

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
        Geometry::Points(ps) => {
            out.push_str("{\"type\":\"MultiPoint\",\"coordinates\":[");
            for (k, (p, z)) in ps.iter().enumerate() {
                if k > 0 {
                    out.push(',');
                }
                position(*p, *z, out);
            }
            out.push(']');
        }
        Geometry::Line(ring) => {
            out.push_str("{\"type\":\"LineString\",\"coordinates\":");
            positions(ring, out);
        }
        Geometry::Lines(paths) => {
            out.push_str("{\"type\":\"MultiLineString\",\"coordinates\":[");
            for (k, ring) in paths.iter().enumerate() {
                if k > 0 {
                    out.push(',');
                }
                positions(ring, out);
            }
            out.push(']');
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
        Geometry::Collection(members) => {
            out.push_str("{\"type\":\"GeometryCollection\",\"geometries\":[");
            for (k, g) in members.iter().enumerate() {
                if k > 0 {
                    out.push(',');
                }
                write_geometry(g, out);
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

/// How an attribute is written on a layer with fields (docs/adr/0199 §6): a
/// number field's value in its canonical text as a JSON number when its
/// nearest float64's shortest text is the same value (a wider one stays
/// text, so nothing is lost), a yes or no field's as a JSON boolean, any
/// other as text.
fn typed(field: Option<&LayerField>, value: &str, out: &mut String) {
    let Some(f) = field else {
        return quote(value, out);
    };
    let Ok(c) = check_value(f, value) else {
        return quote(value, out);
    };
    if f.kind.is_number() && !c.is_empty() {
        let exact = c
            .parse::<f64>()
            .ok()
            .filter(|x| x.is_finite())
            .is_some_and(|x| {
                compare_decimals(&c, &x.to_string()) == Some(std::cmp::Ordering::Equal)
            });
        if exact {
            return out.push_str(&c);
        }
    }
    if f.kind == LayerFieldKind::Boolean && (c == "true" || c == "false") {
        return out.push_str(&c);
    }
    quote(value, out);
}

/// Writes the objects as a GeoJSON FeatureCollection in the project's coordinates.
pub fn write(input: &GeoJsonWriteInput) -> (Vec<u8>, ExportReport) {
    write_with(input, None)
}

/// Writes the objects as RFC 7946 GeoJSON: moved from the project's system
/// `from` into WGS 84 longitude and latitude, `choices` the project's datum
/// choices (docs/adr/0222 §7).
pub fn write_wgs84(
    input: &GeoJsonWriteInput,
    from: &System,
    choices: &[Choice],
) -> (Vec<u8>, ExportReport) {
    write_with(input, Some((from, choices)))
}

/// A vertex moved into WGS 84 and rounded to ten decimals; why not when the move does not reach it.
fn to_wgs84(from: &System, choices: &[Choice], p: Vec2) -> Result<Vec2, Unreached> {
    transform_in(
        from,
        &WGS84,
        kentos_geometry_core::Vec2::new(p.x, p.y),
        choices,
    )
    .map(|t| Vec2 {
        x: rounded(t.point.x, DEGREE_DECIMALS),
        y: rounded(t.point.y, DEGREE_DECIMALS),
    })
}

fn write_with(
    input: &GeoJsonWriteInput,
    wgs84: Option<(&System, &[Choice])>,
) -> (Vec<u8>, ExportReport) {
    let fields: HashMap<&str, &[LayerField]> = input
        .layers
        .iter()
        .map(|l| (l.id.as_str(), l.fields.as_slice()))
        .collect();
    let names: HashMap<&str, &str> = input
        .layers
        .iter()
        .map(|l: &GeoJsonLayer| (l.id.as_str(), l.name.as_str()))
        .collect();
    let mut rep = Report::default();
    let mut out = String::with_capacity(64 + input.entities.len() * 160);
    out.push_str("{\"type\":\"FeatureCollection\",\"name\":");
    quote(&input.name, &mut out);
    if wgs84.is_some() {
        rep.note(
            "Koordinat sistemi",
            &format!(
                "EPSG:{} koordinatları WGS 84 boylam ve enlemine çevrilip on basamağa yuvarlandı (RFC 7946); kotlar olduğu gibi yazıldı",
                input.srid
            ),
            0,
        );
    } else if input.srid == 0 {
        // A local project's coordinates are bound to no place (docs/adr/0165 §2): no crs member.
        rep.note(
            "Koordinat sistemi",
            "yerel proje (koordinat sistemi yok): koordinatlar olduğu gibi yazıldı, dosya bir koordinat sistemi adlandırmıyor; okuyan program onları WGS 84 boylam, enlem sayabilir",
            0,
        );
    } else if input.srid != 4326 {
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
    let blocks = Placing::new(&input.blocks);
    let mut first = true;
    let mut unreached: Vec<(Unreached, u32)> = Vec::new();
    for e in &input.entities {
        // A record has no geometry: a feature with a null one (docs/adr/0410 §4).
        let record = matches!(e, kentos_contracts::Entity::Record(_));
        let mut g = if record {
            None
        } else {
            match geometry(e, &blocks, &mut rep, &GEOJSON) {
                Some(g) => Some(g),
                None => continue,
            }
        };
        if let (Some((from, choices)), Some(g)) = (wgs84, g.as_mut()) {
            let mut why = None;
            let moved = g.map(&mut |p| match to_wgs84(from, choices, p) {
                Ok(q) => Some(q),
                Err(u) => {
                    why = Some(u);
                    None
                }
            });
            if !moved {
                if let Some(u) = why {
                    tally(&mut unreached, u);
                }
                continue;
            }
        }
        let b = e.base();
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
        let own = fields.get(b.layer_id.as_str()).copied().unwrap_or_default();
        for (i, (k, v)) in b.attrs.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            quote(k, &mut out);
            out.push(':');
            typed(own.iter().find(|f| f.name == *k), v, &mut out);
        }
        out.push_str("},\"geometry\":");
        match &g {
            Some(g) => write_geometry(g, &mut out),
            None => out.push_str("null"),
        }
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
        rep.count(e.kind());
    }
    out.push_str("\n]}\n");
    say_unreached(&mut rep, unreached);
    (out.into_bytes(), rep.export())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Input {
    entities: Objects,
    layers: Vec<GeoJsonLayer>,
    srid: u32,
    name: String,
    #[serde(default)]
    blocks: Definitions,
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
            blocks: i.blocks.0,
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
    use crate::gis::shoelace;
    use kentos_contracts::{Entity, GeoJsonReadOptions};

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

    /// A local project (SRID 0, docs/adr/0165 §2) names no system: no crs member, and the
    /// report says why.
    #[test]
    fn a_local_project_names_no_coordinate_system() {
        let mut input = input_from_json(INPUT).expect("input");
        input.srid = 0;
        let (bytes, report) = write(&input);
        let text = String::from_utf8(bytes).expect("utf8");
        assert!(!text.contains("\"crs\""));
        assert!(
            report
                .notes
                .iter()
                .any(|i| i.what == "Koordinat sistemi" && i.reason.starts_with("yerel proje"))
        );
    }

    #[test]
    fn reads_what_the_contracts_own_deserializer_reads() {
        let ours = input_from_json(INPUT).expect("input");
        let derived: GeoJsonWriteInput = serde_json::from_str(INPUT).expect("contract");
        assert_eq!(ours, derived);
    }
}
