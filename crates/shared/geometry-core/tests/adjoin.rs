//! Bitişik alan and the overlap control (docs/adr/0162) against the
//! independent reference in `fixtures/adjoin/v1/cases.json`
//! (`scripts/fixtures/adjoin_cases.py`: exact rationals, the arc cases by
//! hand with 50-digit mpmath, no KentOS code). The operations, called by name
//! as the web calls them through WASM, give every case's areas: a vertex
//! that is an input vertex bit for bit, any other within 1e-9 m; bulges
//! within 1e-12; areas within 1e-9 relative; and which neighbours a new area
//! overlaps. The core's order is its own: both sides are put in the
//! reference's order first.

// Test harness code, not the core: the std float methods are fine here.
#![allow(clippy::disallowed_methods)]

use kentos_geometry_core::api::run_named;
use serde_json::{Value, json};

type Pt = [f64; 2];

fn pt(v: &Value) -> Pt {
    if let Some(a) = v.as_array() {
        return [a[0].as_f64().unwrap(), a[1].as_f64().unwrap()];
    }
    [v["x"].as_f64().unwrap(), v["y"].as_f64().unwrap()]
}

#[derive(Debug, Clone)]
struct Ring {
    pts: Vec<Pt>,
    bulges: Vec<f64>,
}

fn ring(v: &Value) -> Ring {
    let pts: Vec<Pt> = v["pts"].as_array().unwrap().iter().map(pt).collect();
    let bulges = match v["bulges"].as_array() {
        Some(b) => b.iter().map(|x| x.as_f64().unwrap()).collect(),
        None => vec![0.0; pts.len()],
    };
    Ring { pts, bulges }
}

/// A ring starting at its lowest vertex (x, then y), its way kept; a
/// bulge stays with the edge leaving its vertex.
fn lowest_first(mut r: Ring) -> Ring {
    let k = (0..r.pts.len())
        .min_by(|&a, &b| r.pts[a].partial_cmp(&r.pts[b]).unwrap())
        .unwrap_or(0);
    r.pts.rotate_left(k);
    r.bulges.rotate_left(k);
    r
}

#[derive(Debug)]
struct Area {
    outer: Ring,
    holes: Vec<Ring>,
}

fn areas(list: &Value) -> Vec<Area> {
    let mut out: Vec<Area> = list
        .as_array()
        .unwrap()
        .iter()
        .map(|a| {
            let mut holes: Vec<Ring> = a["holes"]
                .as_array()
                .unwrap()
                .iter()
                .map(|h| lowest_first(ring(h)))
                .collect();
            holes.sort_by(|a, b| a.pts[0].partial_cmp(&b.pts[0]).unwrap());
            Area {
                outer: lowest_first(ring(&a["outer"])),
                holes,
            }
        })
        .collect();
    out.sort_by(|a, b| {
        (a.outer.pts[0], a.outer.pts[1])
            .partial_cmp(&(b.outer.pts[0], b.outer.pts[1]))
            .unwrap()
    });
    out
}

/// The signed area of a ring of corners and bulges, about its first corner
/// (at TM coordinates the products of whole coordinates lose the area).
fn ring_area(r: &Ring) -> f64 {
    let n = r.pts.len();
    let o = r.pts[0];
    let at = |p: Pt| [p[0] - o[0], p[1] - o[1]];
    let mut s = 0.0;
    for i in 0..n {
        let (a, b) = (at(r.pts[i]), at(r.pts[(i + 1) % n]));
        s += (a[0] * b[1] - b[0] * a[1]) / 2.0;
        let k = r.bulges[i];
        if k != 0.0 {
            let theta = 4.0 * k.atan();
            let chord = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt();
            let rad = chord / (2.0 * (theta / 2.0).sin().abs());
            s += rad * rad / 2.0 * (theta - theta.sin());
        }
    }
    s
}

fn same_ring(got: &Ring, want: &Ring, inputs: &[Pt]) -> Result<(), String> {
    if got.pts.len() != want.pts.len() {
        return Err(format!(
            "{} köşe ≠ {} köşe: {:?}",
            got.pts.len(),
            want.pts.len(),
            got.pts
        ));
    }
    for (i, (g, w)) in got.pts.iter().zip(&want.pts).enumerate() {
        let exact = inputs.contains(w);
        let ok = if exact {
            g == w
        } else {
            (g[0] - w[0]).abs() <= 1e-9 && (g[1] - w[1]).abs() <= 1e-9
        };
        if !ok {
            let how = if exact {
                "girdi köşesi, bit bit"
            } else {
                "1e-9 m"
            };
            return Err(format!("köşe {i}: {g:?} ≠ {w:?} ({how})"));
        }
    }
    for (i, (g, w)) in got.bulges.iter().zip(&want.bulges).enumerate() {
        if (g - w).abs() > 1e-12 {
            return Err(format!("kabarıklık {i}: {g} ≠ {w}"));
        }
    }
    Ok(())
}

fn differ(got: &Value, want: &Value, inputs: &[Pt]) -> Option<String> {
    let ours = areas(got);
    let theirs = areas(&want["areas"]);
    if ours.len() != theirs.len() {
        return Some(format!(
            "{} alan ≠ {} alan: {:?}",
            ours.len(),
            theirs.len(),
            ours.iter().map(|a| a.outer.pts.clone()).collect::<Vec<_>>()
        ));
    }
    for (i, (a, b)) in ours.iter().zip(&theirs).enumerate() {
        if let Err(e) = same_ring(&a.outer, &b.outer, inputs) {
            return Some(format!("alan {i}, dış halka: {e}"));
        }
        if a.holes.len() != b.holes.len() {
            return Some(format!(
                "alan {i}: {} delik ≠ {}",
                a.holes.len(),
                b.holes.len()
            ));
        }
        for (k, (h, g)) in a.holes.iter().zip(&b.holes).enumerate() {
            if let Err(e) = same_ring(h, g, inputs) {
                return Some(format!("alan {i}, delik {k}: {e}"));
            }
        }
        let net = ring_area(&a.outer) + a.holes.iter().map(ring_area).sum::<f64>();
        let expected = want["areas"]
            .as_array()
            .unwrap()
            .iter()
            .map(|w| w["area"].as_f64().unwrap())
            .find(|w| (w - net).abs() <= 1e-9 * w.abs().max(1.0));
        if expected.is_none() {
            return Some(format!("alan {i}: {net} m² başvurunun alanlarında yok"));
        }
    }
    None
}

fn inputs_of(case: &Value, own: &str) -> Vec<Pt> {
    let mut out = Vec::new();
    let mut take = |r: &Value| out.extend(r["pts"].as_array().unwrap().iter().map(pt));
    match own {
        "area" => {
            take(&case["area"]["outer"]);
            for h in case["area"]["holes"].as_array().unwrap() {
                take(h);
            }
        }
        _ => take(&case["path"]),
    }
    for obj in case["neighbours"].as_array().unwrap() {
        for part in obj.as_array().unwrap() {
            take(&part["outer"]);
            for h in part["holes"].as_array().unwrap() {
                take(h);
            }
        }
    }
    out
}

fn fixture() -> Value {
    let path = format!(
        "{}/../../../fixtures/adjoin/v1/cases.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let file: Value = serde_json::from_str(&std::fs::read_to_string(path).expect("fixture file"))
        .expect("fixture JSON");
    assert_eq!(file["format"], "kentos.adjoin");
    assert_eq!(file["version"], 1);
    file
}

#[test]
fn every_new_area_is_cut_as_the_reference_cuts_it() {
    let file = fixture();
    let cases = file["avoid"].as_array().expect("avoid");
    assert!(cases.len() >= 40, "{} cases", cases.len());
    let mut failures = Vec::new();
    for c in cases {
        let name = c["name"].as_str().unwrap();
        let args = json!([c["area"], c["neighbours"]]).to_string();
        let got: Value = match run_named("adjoinAvoidAreas", &args) {
            Ok(text) => serde_json::from_str(&text).expect("answer JSON"),
            Err(e) => {
                failures.push(format!("{name}: {e}"));
                continue;
            }
        };
        if got["overlapped"] != c["expect"]["overlapped"] {
            failures.push(format!(
                "{name}: örtüşenler {} ≠ {}",
                got["overlapped"], c["expect"]["overlapped"]
            ));
            continue;
        }
        if let Some(e) = differ(&got["areas"], &c["expect"], &inputs_of(c, "area")) {
            failures.push(format!("{name}: {e}"));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} cases differ:\n{}",
        failures.len(),
        cases.len(),
        failures.join("\n")
    );
}

#[test]
fn every_path_fills_what_the_reference_fills() {
    let file = fixture();
    let cases = file["fill"].as_array().expect("fill");
    assert!(cases.len() >= 40, "{} cases", cases.len());
    let mut failures = Vec::new();
    for c in cases {
        let name = c["name"].as_str().unwrap();
        let args = json!([c["path"]["pts"], c["path"]["bulges"], c["neighbours"]]).to_string();
        let got: Value = match run_named("adjoinFillAreas", &args) {
            Ok(text) => serde_json::from_str(&text).expect("answer JSON"),
            Err(e) => {
                failures.push(format!("{name}: {e}"));
                continue;
            }
        };
        if let Some(e) = differ(&got, &c["expect"], &inputs_of(c, "path")) {
            failures.push(format!("{name}: {e}"));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} cases differ:\n{}",
        failures.len(),
        cases.len(),
        failures.join("\n")
    );
}

/// A view's worth of parcels (a grid at TM coordinates, one parcel missing)
/// and a path closing the gap over its top: how long one `fill` takes, for
/// the preview's budget (docs/adr/0162 §5: under 2 000 edges every pointer
/// move). Run by hand in release:
///
/// ```text
/// cargo test --release -p kentos-geometry-core --test adjoin -- --ignored --nocapture
/// ```
#[test]
#[ignore = "timings, run by hand in release"]
fn a_view_of_parcels_fills_in_time() {
    use kentos_geometry_core::Vec2;
    use kentos_geometry_core::geom::arrangement::{Area, Ring};
    use kentos_geometry_core::ops::adjoin::Neighbours;
    use std::time::Instant;

    for n in [10usize, 20, 32, 45] {
        let (e0, n0, w) = (487000.0, 4420000.0, 20.0);
        let gap = (n / 2, n - 1);
        let mut sets = Vec::new();
        for i in 0..n {
            for j in 0..n {
                if (i, j) == gap {
                    continue;
                }
                let (x, y) = (e0 + i as f64 * w, n0 + j as f64 * w);
                sets.push(vec![Area {
                    outer: Ring {
                        pts: vec![
                            Vec2::new(x, y),
                            Vec2::new(x + w, y),
                            Vec2::new(x + w, y + w),
                            Vec2::new(x, y + w),
                        ],
                        bulges: None,
                    },
                    holes: Vec::new(),
                }]);
            }
        }
        let neighbours = Neighbours::new(sets);
        let (gx, gy) = (e0 + gap.0 as f64 * w, n0 + gap.1 as f64 * w);
        let path = [
            Vec2::new(gx - 5.0, gy + 10.0),
            Vec2::new(gx + 3.0, gy + w + 6.0),
            Vec2::new(gx + w - 3.0, gy + w + 6.0),
            Vec2::new(gx + w + 5.0, gy + 10.0),
        ];
        let start = Instant::now();
        let mut found = 0;
        let runs = 20;
        for _ in 0..runs {
            found = neighbours.fill(&path, None).len();
        }
        let each = start.elapsed().as_secs_f64() * 1e3 / f64::from(runs);
        println!(
            "{} komşu, {} kenar: {each:.2} ms ({found} bölge)",
            neighbours.len(),
            neighbours.edge_count()
        );
        assert_eq!(found, 1);
    }
}
