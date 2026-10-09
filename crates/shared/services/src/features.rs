//! A service's objects for the drawing (docs/adr/0208 §10): a GeoJSON
//! answer (WFS's GML through `gml`) read by the formats core as a file is,
//! its vertices moved from the service's system to the project's by the
//! caller's transformation (an object with a vertex that has no place there
//! is left out and counted), axes swapped where a service wrote them north
//! first, objects outside the area asked left out, and the pages that
//! follow: OGC API's `next` link, WFS's `numberMatched`, ArcGIS's
//! `exceededTransferLimit`.

use kentos_contracts::{Entity, GeoJsonReadOptions, ImportResult, PathEntity, Vec2};

/// The objects of a GeoJSON answer on the layer `layer`, at most `max` (0: a million).
pub fn read_geojson(text: &str, layer: &str, max: u32) -> Result<ImportResult, String> {
    kentos_formats::geojson::read(
        text.as_bytes(),
        &GeoJsonReadOptions {
            layer: layer.to_owned(),
            max_entities: max,
        },
    )
}

/// Every vertex of an object moved by `f`; false when one has no place.
fn moved(e: &mut Entity, f: &mut dyn FnMut(Vec2) -> Option<Vec2>) -> bool {
    let mut each = |p: &mut Vec2| match f(*p) {
        Some(q) => {
            *p = q;
            true
        }
        None => false,
    };
    let path = |p: &mut PathEntity, each: &mut dyn FnMut(&mut Vec2) -> bool| {
        p.pts.iter_mut().all(&mut *each)
            && p.holes
                .iter_mut()
                .flatten()
                .all(|h| h.pts.iter_mut().all(&mut *each))
            && p.parts.iter_mut().flatten().all(|part| {
                part.pts.iter_mut().all(&mut *each)
                    && part
                        .holes
                        .iter_mut()
                        .flatten()
                        .all(|h| h.pts.iter_mut().all(&mut *each))
            })
    };
    match e {
        Entity::Point(p) => each(&mut p.p) && p.parts.iter_mut().flatten().all(|q| each(&mut q.p)),
        Entity::Line(l) => each(&mut l.a) && each(&mut l.b),
        Entity::Polyline(p) | Entity::Polygon(p) => path(p, &mut each),
        // A service gives no other kind (GeoJSON reads into these).
        _ => true,
    }
}

/// Moves every object's vertices by `f`; the objects left out (a vertex
/// with no place) are counted and returned. The bounds follow.
pub fn transform(result: &mut ImportResult, mut f: impl FnMut(Vec2) -> Option<Vec2>) -> usize {
    let before = result.entities.len();
    result.entities.retain_mut(|e| moved(e, &mut f));
    result.bounds = bounds(&result.entities);
    result.view = result.bounds;
    before - result.entities.len()
}

/// East and north swapped in every vertex (a service that wrote them north first).
pub fn swap_axes(result: &mut ImportResult) {
    transform(result, |p| Some(Vec2 { x: p.y, y: p.x }));
}

fn vertices(e: &Entity) -> Vec<Vec2> {
    match e {
        Entity::Point(p) => std::iter::once(p.p)
            .chain(p.parts.iter().flatten().map(|q| q.p))
            .collect(),
        Entity::Line(l) => vec![l.a, l.b],
        Entity::Polyline(p) | Entity::Polygon(p) => p
            .pts
            .iter()
            .copied()
            .chain(p.parts.iter().flatten().flat_map(|q| q.pts.iter().copied()))
            .collect(),
        _ => Vec::new(),
    }
}

fn bounds(list: &[Entity]) -> Option<kentos_contracts::Bounds> {
    let mut b: Option<kentos_contracts::Bounds> = None;
    for v in list.iter().flat_map(vertices) {
        let x = b.get_or_insert(kentos_contracts::Bounds {
            min_x: v.x,
            min_y: v.y,
            max_x: v.x,
            max_y: v.y,
        });
        x.min_x = x.min_x.min(v.x);
        x.min_y = x.min_y.min(v.y);
        x.max_x = x.max_x.max(v.x);
        x.max_y = x.max_y.max(v.y);
    }
    b
}

/// Leaves out the objects whose box does not meet `bbox` (a WFS given a
/// CQL filter instead of a box); the number left out.
pub fn within(result: &mut ImportResult, bbox: [f64; 4]) -> usize {
    let before = result.entities.len();
    result.entities.retain(|e| {
        let v = vertices(e);
        if v.is_empty() {
            return true;
        }
        let (mut x1, mut y1, mut x2, mut y2) = (
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        );
        for p in v {
            x1 = x1.min(p.x);
            y1 = y1.min(p.y);
            x2 = x2.max(p.x);
            y2 = y2.max(p.y);
        }
        x1 <= bbox[2] && bbox[0] <= x2 && y1 <= bbox[3] && bbox[1] <= y2
    });
    result.bounds = bounds(&result.entities);
    result.view = result.bounds;
    before - result.entities.len()
}

/// OGC API Features' next page: its answer's `next` link.
pub fn next_link(text: &str, base: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(text).ok()?;
    v["links"].as_array()?.iter().find_map(|l| {
        (l["rel"].as_str() == Some("next"))
            .then(|| l["href"].as_str().map(|h| crate::query::resolve(base, h)))
            .flatten()
    })
}

/// Whether an ArcGIS query's answer stopped at its limit (more pages follow).
pub fn arcgis_more(text: &str) -> bool {
    let v: serde_json::Value = match serde_json::from_str(text) {
        Ok(v) => v,
        Err(_) => return false,
    };
    v["exceededTransferLimit"].as_bool().unwrap_or(false)
        || v["properties"]["exceededTransferLimit"]
            .as_bool()
            .unwrap_or(false)
}

/// How many features a GeoJSON answer holds (to know whether a page was full).
pub fn feature_count(text: &str) -> usize {
    serde_json::from_str::<serde_json::Value>(text)
        .ok()
        .and_then(|v| v["features"].as_array().map(Vec::len))
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_moved_swapped_and_kept_within() {
        let text = r#"{"type":"FeatureCollection","features":[
          {"type":"Feature","properties":{"ada":"104"},"geometry":{"type":"Polygon","coordinates":[[[0,0],[10,0],[10,10],[0,10],[0,0]]]}},
          {"type":"Feature","properties":{"ad":"R1"},"geometry":{"type":"Point","coordinates":[100,100]}}],
          "links":[{"rel":"next","href":"items?offset=2"}]}"#;
        let mut r = read_geojson(text, "WFS", 0).unwrap();
        assert_eq!(r.entities.len(), 2);
        assert_eq!(
            r.entities[0].base().attrs.get("ada").map(String::as_str),
            Some("104")
        );
        let left = transform(&mut r, |p| {
            (p.x < 50.0).then_some(Vec2 {
                x: p.x + 500_000.0,
                y: p.y + 4_400_000.0,
            })
        });
        assert_eq!(left, 1);
        let b = r.bounds.unwrap();
        assert_eq!((b.min_x, b.max_y), (500_000.0, 4_400_010.0));
        swap_axes(&mut r);
        assert_eq!(r.bounds.unwrap().min_x, 4_400_000.0);
        assert_eq!(within(&mut r, [0.0, 0.0, 1.0, 1.0]), 1);
        assert_eq!(
            next_link(text, "https://x/c/items").as_deref(),
            Some("https://x/c/items?offset=2")
        );
        assert!(arcgis_more(
            r#"{"type":"FeatureCollection","features":[],"properties":{"exceededTransferLimit":true}}"#
        ));
        assert_eq!(feature_count(text), 2);
    }
}
