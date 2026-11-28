//! GIS-005 (docs/adr/1071): a curve polygon's arcs read as bulges at map coordinates. The park buffer of the usage
//! report (a TM33 CURVEPOLYGON with a few-micrometre arc of three nearly straight points), turned copies
//! and a real arc past 180° both ways. The area and the perimeter are checked against an independent
//! 60-digit computation (mpmath, `scripts/fixtures/gpkg_arc_area.py`: each arc's circle from its three points as
//! written, a three points on one line straight); the extent against an independent reader's envelope of the report's GeoPackage.
//! That reader's own curve area for this ring (38 956.275 m²) is 0.244 m² off the exact one; its linear copy agrees
//! with it. Which reader and version: the ADR of the arcs' reading.

use std::collections::BTreeSet;

use kentos_geometry_core::geom::arrangement::{Area, Ring};
use kentos_geometry_core::geom::region::net_area;
use kentos_geometry_core::vec2::Vec2;
use kentos_geopackage::wkt::read_wkt;

use super::{arc_bulge, geometry_json};
use kentos_geopackage::Coord;

/// The park buffer as the report's GeoPackage holds it (synthetic municipal data, TUREF / TM33).
const PARK: &str = "CURVEPOLYGON (COMPOUNDCURVE (CIRCULARSTRING (488066.28988128 4422216.23320277,488063.360949092 \
4422223.30427058,488056.28988128 4422226.23320277),(488056.28988128 4422226.23320277,487834.180347811 \
4422226.23320277),CIRCULARSTRING (487834.180347811 4422226.23320277,487827.154898782 4422223.34959694,\
487824.181174913 4422216.36181596),CIRCULARSTRING (487824.181174913 4422216.36181596,487824.181174845 \
4422216.36181067,487824.181174777 4422216.36180538),(487824.181174777 4422216.36180538,487822.411944988 \
4422078.81110132),CIRCULARSTRING (487822.411944988 4422078.81110132,487825.197968795 4422071.75641564,\
487832.13375469 4422068.68633539),(487832.13375469 4422068.68633539,488056.012496449 4422062.47437315),\
CIRCULARSTRING (488056.012496449 4422062.47437315,488056.012507266 4422062.47437285,488056.012518083 \
4422062.47437255),CIRCULARSTRING (488056.012518083 4422062.47437255,488063.262196822 4422065.30206548,\
488066.28988128 4422072.47052529),(488066.28988128 4422072.47052529,488066.28988128 4422216.23320277)))";

/// `PARK`'s area and perimeter at 60 digits (`scripts/fixtures/gpkg_arc_area.py`).
const AREA: f64 = 38_956.519_339_831;
const PERIMETER: f64 = 790.231_084_908_664;
/// `PARK`'s envelope as an independent reader gives it: min x, max x, min y, max y (the arcs' bulge past the vertices
/// in).
const REFERENCE_EXTENT: [f64; 4] = [
    487_822.411_117_886_25,
    488_066.289_881_279_9,
    4_422_062.470_525_288,
    4_422_226.233_202_772,
];

/// The area object's outer ring read from a WKT text.
fn ring_of(wkt: &str) -> Ring {
    let g = read_wkt(wkt).expect("the WKT reads");
    let (kind, m) = geometry_json(&g, &mut BTreeSet::new()).expect("an object");
    assert_eq!(kind, "polygon");
    let pts: Vec<Vec2> = m["pts"]
        .as_array()
        .expect("pts")
        .iter()
        .map(|p| Vec2::new(p["x"].as_f64().unwrap(), p["y"].as_f64().unwrap()))
        .collect();
    let bulges = m.get("bulges").map(|b| {
        b.as_array()
            .expect("bulges")
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect::<Vec<f64>>()
    });
    Ring { pts, bulges }
}

/// The ring's length, each bulged edge its arc: chord · (θ/2) / sin(θ/2), θ = 4·atan(bulge).
fn perimeter(r: &Ring) -> f64 {
    let n = r.pts.len();
    (0..n)
        .map(|i| {
            let (a, b) = (r.pts[i], r.pts[(i + 1) % n]);
            let chord = kentos_num::hypot(b.x - a.x, b.y - a.y);
            let k = r.bulges.as_ref().map_or(0.0, |v| v[i]);
            let half = 2.0 * kentos_num::atan(k);
            if half == 0.0 {
                chord
            } else {
                chord * half / kentos_num::sin(half)
            }
        })
        .sum()
}

fn area(r: &Ring) -> f64 {
    net_area(&Area {
        outer: r.clone(),
        holes: Vec::new(),
    })
    .abs()
}

/// `PARK` with every point turned by `deg` about its first vertex.
fn turned(deg: f64) -> String {
    let (s, c) = kentos_num::sin_cos(deg.to_radians());
    let (x0, y0) = (488_066.289_881_28, 4_422_216.233_202_77);
    let mut out = String::new();
    let mut number = String::new();
    let mut pending: Option<f64> = None;
    let flush = |number: &mut String, pending: &mut Option<f64>, out: &mut String| {
        if number.is_empty() {
            return;
        }
        let v: f64 = number.parse().expect("a number");
        number.clear();
        match pending.take() {
            None => *pending = Some(v),
            Some(x) => {
                let (dx, dy) = (x - x0, v - y0);
                out.push_str(&format!(
                    "{} {}",
                    x0 + c * dx - s * dy,
                    y0 + s * dx + c * dy
                ));
            }
        }
    };
    for ch in PARK.chars() {
        if ch.is_ascii_digit() || ch == '.' {
            number.push(ch);
        } else {
            flush(&mut number, &mut pending, &mut out);
            if pending.is_none() {
                out.push(ch);
            }
        }
    }
    out
}

#[test]
fn the_parks_buffer_keeps_its_few_micrometre_arc_near_straight() {
    let r = ring_of(PARK);
    let bulges = r.bulges.as_ref().expect("the buffer has arcs");
    // Every bulge about a quarter circle's or a near-straight one's: none of the far centre's millions.
    assert!(bulges.iter().all(|b| b.abs() < 0.5), "{bulges:?}");
    let a = area(&r);
    let p = perimeter(&r);
    assert!((a - AREA).abs() < 1e-5, "{a} m² for {AREA}");
    assert!((p - PERIMETER).abs() < 1e-6, "{p} m for {PERIMETER}");
    let (xs, ys): (Vec<f64>, Vec<f64>) = r.pts.iter().map(|q| (q.x, q.y)).unzip();
    let min = |v: &[f64]| v.iter().copied().fold(f64::INFINITY, f64::min);
    let max = |v: &[f64]| v.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    // The vertices lie within the arcs' extent, at most a few millimetres in from it.
    let got = [min(&xs), max(&xs), min(&ys), max(&ys)];
    for (g, w) in got.iter().zip(REFERENCE_EXTENT) {
        assert!((g - w).abs() < 0.01, "{got:?} for {REFERENCE_EXTENT:?}");
    }
}

#[test]
fn turned_copies_keep_area_and_perimeter() {
    for deg in [37.0, 90.0, 213.5] {
        let r = ring_of(&turned(deg));
        assert!((area(&r) - AREA).abs() < 1e-3, "{deg}°: {}", area(&r));
        assert!(
            (perimeter(&r) - PERIMETER).abs() < 1e-5,
            "{deg}°: {}",
            perimeter(&r)
        );
    }
}

#[test]
fn a_real_arc_past_half_a_circle_keeps_its_sweep_and_direction() {
    // A 270° arc of radius 50 m at TM33 coordinates, counter-clockwise from east to south, and the same arc back.
    let (cx, cy, r) = (487_900.0, 4_422_100.0, 50.0);
    let at = |deg: f64| {
        let t: f64 = deg.to_radians();
        Coord {
            x: cx + r * kentos_num::cos(t),
            y: cy + r * kentos_num::sin(t),
            z: None,
            m: None,
        }
    };
    let want = kentos_num::tan(270f64.to_radians() / 4.0);
    let ccw = arc_bulge(&at(0.0), &at(135.0), &at(270.0)).expect("an arc");
    assert!((ccw - want).abs() < 1e-9 * want, "{ccw} for {want}");
    let cw = arc_bulge(&at(270.0), &at(135.0), &at(0.0)).expect("an arc");
    assert!((cw + want).abs() < 1e-9 * want, "{cw} for −{want}");
    // A semicircle is 1, a quarter tan(22.5°).
    assert!((arc_bulge(&at(0.0), &at(90.0), &at(180.0)).unwrap() - 1.0).abs() < 1e-12);
    let quarter = arc_bulge(&at(0.0), &at(45.0), &at(90.0)).unwrap();
    assert!((quarter - kentos_num::tan(22.5f64.to_radians())).abs() < 1e-9);
    // Three points on one line: straight; a and b one point: a whole circle, not one bulge.
    assert_eq!(arc_bulge(&at(0.0), &at(0.0), &at(90.0)), None);
    assert_eq!(arc_bulge(&at(0.0), &at(180.0), &at(0.0)), None);
}
