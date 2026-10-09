//! A WFS's GML, 2, 3.1 and 3.2 (docs/adr/0208 §10), read into a GeoJSON
//! feature collection the formats core then reads as any GeoJSON file
//! (`kentos_formats::geojson`), so a service's objects get the same
//! geometry and attribute rules as a file's. Geometries: Point,
//! LineString, LinearRing, Polygon (exterior and interior, or GML 2's outer
//! and inner boundaries), MultiPoint, MultiLineString, MultiCurve,
//! MultiPolygon, MultiSurface, Curve of LineStringSegments, Surface of
//! PolygonPatches; coordinates as `pos`, `posList` (with `srsDimension`) or
//! GML 2's `coordinates`. A geometry's `srsName` says its axis order: the
//! URN and URI forms follow EPSG's (TUREF's TM zones north first, EPSG:4326
//! latitude first), `EPSG:n` is east first. Coordinates come out east and
//! north.

use roxmltree::Node;
use serde_json::{Map, Value as Json, json};

use crate::caps::{attr, text_of, xml};
use crate::crs::{Axis, axis_order, crs_name};

/// How a geometry's numbers are read: its dimension and whether its axes swap.
#[derive(Clone, Copy)]
struct Reading {
    dim: usize,
    swap: bool,
}

fn reading_of(n: Node<'_, '_>, parent: Reading) -> Reading {
    let mut r = parent;
    if let Some(name) = attr(n, "srsName") {
        let upper = name.to_ascii_uppercase();
        let ordered = upper.starts_with("URN:") || upper.starts_with("HTTP");
        r.swap = ordered
            && crs_name(name).is_some_and(|c| {
                !c.lon_lat && matches!(axis_order(c.srid), Axis::Ne | Axis::LatLon)
            });
    }
    if let Some(d) = attr(n, "srsDimension").and_then(|d| d.parse::<usize>().ok()) {
        r.dim = d.clamp(2, 3);
    }
    r
}

fn numbers(text: &str) -> Vec<f64> {
    text.split(|c: char| c.is_whitespace() || c == ',')
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse::<f64>().ok())
        .collect()
}

fn position(nums: &[f64], r: Reading) -> Json {
    let (a, b) = (nums[0], nums[1]);
    let (x, y) = if r.swap { (b, a) } else { (a, b) };
    match nums.get(2).filter(|_| r.dim == 3) {
        Some(z) => json!([x, y, z]),
        None => json!([x, y]),
    }
}

/// A list of positions under `n` (`posList`, `pos`s, `coordinates`, `coord`s).
fn positions(n: Node<'_, '_>, r: Reading) -> Vec<Json> {
    let r = reading_of(n, r);
    let mut out = Vec::new();
    for c in n.children().filter(Node::is_element) {
        let cr = reading_of(c, r);
        match c.tag_name().name() {
            "posList" => {
                let nums = numbers(&text_of(c).unwrap_or_default());
                for p in nums.chunks_exact(cr.dim) {
                    out.push(position(p, cr));
                }
            }
            "pos" => {
                let nums = numbers(&text_of(c).unwrap_or_default());
                if nums.len() >= 2 {
                    out.push(position(
                        &nums,
                        Reading {
                            dim: nums.len().min(3),
                            ..cr
                        },
                    ));
                }
            }
            "coordinates" => {
                // GML 2: tuples by spaces, numbers by commas, x first.
                let t = text_of(c).unwrap_or_default();
                let cs = attr(c, "cs").unwrap_or(",");
                let ts = attr(c, "ts").unwrap_or(" ");
                for tuple in t
                    .split(|ch: char| ts.contains(ch) || (ts == " " && ch.is_whitespace()))
                    .filter(|s| !s.is_empty())
                {
                    let nums: Vec<f64> = tuple
                        .split(cs)
                        .filter_map(|s| s.trim().parse().ok())
                        .collect();
                    if nums.len() >= 2 {
                        out.push(position(
                            &nums,
                            Reading {
                                dim: nums.len().min(3),
                                swap: false,
                            },
                        ));
                    }
                }
            }
            "coord" => {
                let v = |k: &str| crate::caps::child_text(c, k).and_then(|t| t.parse::<f64>().ok());
                if let (Some(x), Some(y)) = (v("X"), v("Y")) {
                    out.push(match v("Z") {
                        Some(z) => json!([x, y, z]),
                        None => json!([x, y]),
                    });
                }
            }
            "Point" => out.extend(positions(c, cr)),
            _ => {}
        }
    }
    out
}

/// A ring element's positions (LinearRing, or a Ring of curve members).
fn ring(n: Node<'_, '_>, r: Reading) -> Vec<Json> {
    let r = reading_of(n, r);
    let mut out = positions(n, r);
    if out.is_empty() {
        for c in n.descendants().filter(|d| {
            d.is_element()
                && matches!(
                    d.tag_name().name(),
                    "LinearRing" | "LineStringSegment" | "LineString"
                )
        }) {
            out.extend(positions(c, reading_of(c, r)));
        }
    }
    out
}

fn polygon(n: Node<'_, '_>, r: Reading) -> Option<Vec<Vec<Json>>> {
    let r = reading_of(n, r);
    let mut rings = Vec::new();
    for c in n.children().filter(Node::is_element) {
        match c.tag_name().name() {
            "exterior" | "outerBoundaryIs" => {
                let ring_node = c.children().find(|x| x.is_element())?;
                rings.insert(0, ring(ring_node, r));
            }
            "interior" | "innerBoundaryIs" => {
                if let Some(rn) = c.children().find(|x| x.is_element()) {
                    rings.push(ring(rn, r));
                }
            }
            "patches" => {
                // A Surface: its PolygonPatches.
                for p in c.children().filter(|x| x.is_element()) {
                    if let Some(mut more) = polygon(p, r) {
                        rings.append(&mut more);
                    }
                }
            }
            _ => {}
        }
    }
    (!rings.is_empty()).then_some(rings)
}

/// A GML geometry element as GeoJSON's geometry.
fn geometry(n: Node<'_, '_>, parent: Reading) -> Option<Json> {
    let r = reading_of(n, parent);
    let members = |names: &[&str]| -> Vec<Node<'_, '_>> {
        n.descendants()
            .filter(|d| {
                d.is_element() && d.parent() != Some(n) && names.contains(&d.tag_name().name())
            })
            .filter(|d| {
                // Only the outermost of the names: a member's own parts are not members.
                d.ancestors()
                    .skip(1)
                    .take_while(|a| *a != n)
                    .all(|a| !names.contains(&a.tag_name().name()))
            })
            .collect()
    };
    match n.tag_name().name() {
        "Point" => {
            let p = positions(n, r);
            Some(json!({"type": "Point", "coordinates": p.first()?}))
        }
        "LineString" | "LinearRing" => {
            Some(json!({"type": "LineString", "coordinates": positions(n, r)}))
        }
        "Curve" | "CompositeCurve" => {
            let mut pts = Vec::new();
            for s in members(&["LineStringSegment", "LineString"]) {
                let mut more = positions(s, reading_of(s, r));
                if !pts.is_empty() && !more.is_empty() && pts.last() == more.first() {
                    more.remove(0);
                }
                pts.extend(more);
            }
            Some(json!({"type": "LineString", "coordinates": pts}))
        }
        "Polygon" | "Surface" | "PolygonPatch" => {
            Some(json!({"type": "Polygon", "coordinates": polygon(n, r)?}))
        }
        "MultiPoint" => {
            let pts: Vec<Json> = members(&["Point"])
                .into_iter()
                .filter_map(|p| positions(p, reading_of(p, r)).into_iter().next())
                .collect();
            Some(json!({"type": "MultiPoint", "coordinates": pts}))
        }
        "MultiLineString" | "MultiCurve" => {
            let lines: Vec<Json> = members(&["LineString", "Curve", "CompositeCurve"])
                .into_iter()
                .filter_map(|l| geometry(l, r))
                .map(|g| g["coordinates"].clone())
                .collect();
            Some(json!({"type": "MultiLineString", "coordinates": lines}))
        }
        "MultiPolygon" | "MultiSurface" | "CompositeSurface" => {
            let polys: Vec<Json> = members(&["Polygon", "Surface"])
                .into_iter()
                .filter_map(|p| polygon(p, reading_of(p, r)))
                .map(|rings| json!(rings))
                .collect();
            Some(json!({"type": "MultiPolygon", "coordinates": polys}))
        }
        _ => None,
    }
}

const GEOMETRIES: [&str; 14] = [
    "Point",
    "LineString",
    "LinearRing",
    "Curve",
    "CompositeCurve",
    "Polygon",
    "Surface",
    "MultiPoint",
    "MultiLineString",
    "MultiCurve",
    "MultiPolygon",
    "MultiSurface",
    "CompositeSurface",
    "PolygonPatch",
];

/// A feature element: its first geometry and its simple properties.
fn feature(f: Node<'_, '_>) -> Json {
    let mut props = Map::new();
    let mut geom = Json::Null;
    for c in f.children().filter(Node::is_element) {
        let name = c.tag_name().name();
        if matches!(name, "boundedBy" | "name" | "description")
            && c.tag_name()
                .namespace()
                .is_some_and(|ns| ns.contains("gml"))
        {
            continue;
        }
        let inner = c.children().find(|x| x.is_element());
        match inner {
            Some(g) if GEOMETRIES.contains(&g.tag_name().name()) => {
                if geom.is_null() {
                    geom = geometry(
                        g,
                        Reading {
                            dim: 2,
                            swap: false,
                        },
                    )
                    .unwrap_or(Json::Null);
                }
            }
            Some(_) => {}
            None => {
                let v = text_of(c).unwrap_or_default();
                let nil = attr(c, "nil") == Some("true");
                props.insert(
                    name.to_owned(),
                    if nil { Json::Null } else { Json::String(v) },
                );
            }
        }
    }
    json!({"type": "Feature", "geometry": geom, "properties": props})
}

/// A GML feature collection (a WFS's GetFeature answer) as GeoJSON text;
/// the number of features the server says it matched, when it says.
pub fn to_geojson(text: &str) -> Result<(String, Option<u64>), String> {
    let doc = xml(text)?;
    let root = doc.root_element();
    let matched = attr(root, "numberMatched")
        .or_else(|| attr(root, "numberOfFeatures"))
        .and_then(|n| n.parse::<u64>().ok());
    let mut features = Vec::new();
    for m in root.descendants().filter(|n| {
        n.is_element()
            && matches!(
                n.tag_name().name(),
                "featureMember" | "member" | "featureMembers"
            )
    }) {
        for f in m.children().filter(Node::is_element) {
            // WFS 2.0 may wrap a member in an additional collection.
            if f.tag_name().name() == "FeatureCollection" {
                continue;
            }
            features.push(feature(f));
        }
    }
    let out = json!({"type": "FeatureCollection", "features": features});
    Ok((out.to_string(), matched))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gml_32_with_axes_in_the_systems_order() {
        let gml = r#"<?xml version="1.0"?>
<wfs:FeatureCollection xmlns:wfs="http://www.opengis.net/wfs/2.0" xmlns:gml="http://www.opengis.net/gml/3.2" xmlns:k="urn:k" numberMatched="2">
 <wfs:member><k:parsel gml:id="p.1">
   <k:ada>104</k:ada><k:parsel>7</k:parsel>
   <k:geom><gml:Polygon srsName="urn:ogc:def:crs:EPSG::5254"><gml:exterior><gml:LinearRing>
     <gml:posList>4400000 500000 4400000 500010 4400010 500010 4400000 500000</gml:posList>
   </gml:LinearRing></gml:exterior></gml:Polygon></k:geom>
 </k:parsel></wfs:member>
 <wfs:member><k:nokta><k:geom><gml:Point srsName="EPSG:5254"><gml:pos>500001 4400001</gml:pos></gml:Point></k:geom><k:ad xsi:nil="true" xmlns:xsi="x"/></k:nokta></wfs:member>
</wfs:FeatureCollection>"#;
        let (text, matched) = to_geojson(gml).unwrap();
        assert_eq!(matched, Some(2));
        let v: Json = serde_json::from_str(&text).unwrap();
        let f = &v["features"][0];
        assert_eq!(f["properties"]["ada"], "104");
        // North first in the URN form: east first here.
        assert_eq!(
            f["geometry"]["coordinates"][0][1],
            json!([500010.0, 4400000.0])
        );
        let p = &v["features"][1];
        assert_eq!(p["geometry"]["coordinates"], json!([500001.0, 4400001.0]));
        assert_eq!(p["properties"]["ad"], Json::Null);
    }

    #[test]
    fn gml_2_and_curves() {
        let gml = r#"<wfs:FeatureCollection xmlns:wfs="w" xmlns:gml="http://www.opengis.net/gml" xmlns:k="k">
 <gml:featureMember><k:yol><k:geom><gml:MultiLineString srsName="EPSG:32636"><gml:lineStringMember><gml:LineString>
   <gml:coordinates>1,2 3,4</gml:coordinates></gml:LineString></gml:lineStringMember></gml:MultiLineString></k:geom></k:yol></gml:featureMember>
 <gml:featureMember><k:dere><k:geom><gml:Curve><gml:segments>
   <gml:LineStringSegment><gml:posList>0 0 1 1</gml:posList></gml:LineStringSegment>
   <gml:LineStringSegment><gml:posList>1 1 2 0</gml:posList></gml:LineStringSegment></gml:segments></gml:Curve></k:geom></k:dere></gml:featureMember>
</wfs:FeatureCollection>"#;
        let (text, _) = to_geojson(gml).unwrap();
        let v: Json = serde_json::from_str(&text).unwrap();
        assert_eq!(
            v["features"][0]["geometry"]["coordinates"],
            json!([[[1.0, 2.0], [3.0, 4.0]]])
        );
        assert_eq!(
            v["features"][1]["geometry"]["coordinates"],
            json!([[0.0, 0.0], [1.0, 1.0], [2.0, 0.0]])
        );
    }
}
