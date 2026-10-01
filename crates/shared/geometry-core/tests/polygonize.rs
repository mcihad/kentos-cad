//! Toplu alan (docs/adr/0151) against the independent reference in
//! `fixtures/polygonize/v1/cases.json` (`scripts/fixtures/polygonize_cases.py`,
//! exact rationals, no KentOS code): the operation, called by name as the web
//! calls it through WASM, gives every case's regions (their rings, holes,
//! labels and the input area each repeats), the labels on a boundary and the
//! free ends; points within 1e-8 m. The core's order is its own (smallest
//! region first): both sides are put in the reference's order first.

// Test harness code, not the core: the std float methods are fine here.
#![allow(clippy::disallowed_methods)]

use kentos_geometry_core::api::run_named;
use serde_json::{Value, json};

type Pt = [f64; 2];

fn pt(v: &Value) -> Pt {
    [v["x"].as_f64().unwrap(), v["y"].as_f64().unwrap()]
}

fn pair(v: &Value) -> Pt {
    [v[0].as_f64().unwrap(), v[1].as_f64().unwrap()]
}

/// A ring starting at its lowest vertex (x, then y), its way kept.
fn lowest_first(mut ring: Vec<Pt>) -> Vec<Pt> {
    let k = (0..ring.len())
        .min_by(|&a, &b| ring[a].partial_cmp(&ring[b]).unwrap())
        .unwrap_or(0);
    ring.rotate_left(k);
    ring
}

#[derive(Debug)]
struct Region {
    outer: Vec<Pt>,
    holes: Vec<Vec<Pt>>,
    labels: Vec<u64>,
    existing: Option<u64>,
}

fn sorted(mut regions: Vec<Region>) -> Vec<Region> {
    for r in &mut regions {
        r.outer = lowest_first(std::mem::take(&mut r.outer));
        let mut holes: Vec<Vec<Pt>> = std::mem::take(&mut r.holes)
            .into_iter()
            .map(lowest_first)
            .collect();
        holes.sort_by(|a, b| a[0].partial_cmp(&b[0]).unwrap());
        r.holes = holes;
    }
    regions.sort_by(|a, b| {
        (a.outer[0], a.outer[1])
            .partial_cmp(&(b.outer[0], b.outer[1]))
            .unwrap()
    });
    regions
}

fn close(a: &[Pt], b: &[Pt]) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .all(|(p, q)| (p[0] - q[0]).abs() <= 1e-8 && (p[1] - q[1]).abs() <= 1e-8)
}

fn differ(got: &Value, want: &Value) -> Option<String> {
    let ours = sorted(
        got["regions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| Region {
                outer: r["area"]["outer"]["pts"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(pt)
                    .collect(),
                holes: r["area"]["holes"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|h| h["pts"].as_array().unwrap().iter().map(pt).collect())
                    .collect(),
                labels: r["labels"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_u64().unwrap())
                    .collect(),
                existing: r["existing"].as_u64(),
            })
            .collect(),
    );
    let theirs = sorted(
        want["regions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| Region {
                outer: r["outer"].as_array().unwrap().iter().map(pair).collect(),
                holes: r["holes"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|h| h.as_array().unwrap().iter().map(pair).collect())
                    .collect(),
                labels: r["labels"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_u64().unwrap())
                    .collect(),
                existing: r["existing"].as_u64(),
            })
            .collect(),
    );
    if ours.len() != theirs.len() {
        return Some(format!(
            "{} bölge ≠ {} bölge: {:?}",
            ours.len(),
            theirs.len(),
            ours.iter().map(|r| r.outer[0]).collect::<Vec<_>>()
        ));
    }
    for (i, (a, b)) in ours.iter().zip(&theirs).enumerate() {
        if !close(&a.outer, &b.outer) {
            return Some(format!(
                "bölge {i}: dış halka {:?} ≠ {:?}",
                a.outer, b.outer
            ));
        }
        if a.holes.len() != b.holes.len() || !a.holes.iter().zip(&b.holes).all(|(h, k)| close(h, k))
        {
            return Some(format!("bölge {i}: delikler {:?} ≠ {:?}", a.holes, b.holes));
        }
        if a.labels != b.labels {
            return Some(format!(
                "bölge {i}: etiketler {:?} ≠ {:?}",
                a.labels, b.labels
            ));
        }
        if a.existing != b.existing {
            return Some(format!(
                "bölge {i}: var olan alan {:?} ≠ {:?}",
                a.existing, b.existing
            ));
        }
    }
    if got["onBoundary"] != want["onBoundary"] {
        return Some(format!(
            "sınırdakiler {} ≠ {}",
            got["onBoundary"], want["onBoundary"]
        ));
    }
    let mut ends: Vec<Pt> = got["freeEnds"].as_array().unwrap().iter().map(pt).collect();
    ends.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let want_ends: Vec<Pt> = want["freeEnds"]
        .as_array()
        .unwrap()
        .iter()
        .map(pair)
        .collect();
    if !close(&ends, &want_ends) {
        return Some(format!("boşta uçlar {ends:?} ≠ {want_ends:?}"));
    }
    None
}

#[test]
fn every_case_is_found_as_the_reference_finds_it() {
    let path = format!(
        "{}/../../../fixtures/polygonize/v1/cases.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let file: Value = serde_json::from_str(&std::fs::read_to_string(path).expect("fixture file"))
        .expect("fixture JSON");
    assert_eq!(file["format"], "kentos.polygonize-fixtures");
    assert_eq!(file["version"], 1);
    let cases = file["cases"].as_array().expect("cases");
    assert!(cases.len() >= 50, "{} cases", cases.len());
    let mut failures = Vec::new();
    for c in cases {
        let name = c["name"].as_str().unwrap();
        let args = json!([c["lines"], c["labels"], c["islands"]]).to_string();
        let got: Value = match run_named("polygonize", &args) {
            Ok(text) => serde_json::from_str(&text).expect("answer JSON"),
            Err(e) => {
                failures.push(format!("{name}: {e}"));
                continue;
            }
        };
        if let Some(e) = differ(&got, &c["expected"]) {
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

/// A street grid of 100 × 100 parcels drawn as 202 long lines at TM
/// coordinates, a quarter of them areas already, each parcel's number
/// inside it: 10 000 regions and labels in one call (docs/adr/0151 §6).
/// Run by hand in release:
///
/// ```text
/// cargo test --release -p kentos-geometry-core --test polygonize -- --ignored --nocapture
/// ```
#[test]
#[ignore = "timings, run by hand in release"]
fn ten_thousand_parcels_are_found_in_one_call() {
    use kentos_geometry_core::Vec2;
    use kentos_geometry_core::entity::{Entity, Shape};
    use kentos_geometry_core::ops::polygonize::{PolyLabel, polygonize};
    let (e, n, side, cell) = (487_000.0, 4_420_000.0, 100usize, 20.0);
    let span = side as f64 * cell;
    let line = |a: Vec2, b: Vec2| Entity::new(Shape::Line { a, b });
    let mut lines = Vec::new();
    for i in 0..=side {
        let x = e + i as f64 * cell;
        lines.push(line(Vec2::new(x, n), Vec2::new(x, n + span)));
        let y = n + i as f64 * cell;
        lines.push(line(Vec2::new(e, y), Vec2::new(e + span, y)));
    }
    // A quarter of the parcels are areas already.
    for k in (0..side * side).step_by(4) {
        let (x, y) = (e + (k % side) as f64 * cell, n + (k / side) as f64 * cell);
        lines.push(Entity::new(Shape::Polygon {
            pts: vec![
                Vec2::new(x, y),
                Vec2::new(x + cell, y),
                Vec2::new(x + cell, y + cell),
                Vec2::new(x, y + cell),
            ],
            bulges: None,
            holes: None,
            parts: None,
        }));
    }
    let labels: Vec<PolyLabel> = (0..side * side)
        .map(|k| PolyLabel {
            at: Vec2::new(
                e + (k % side) as f64 * cell + 7.0,
                n + (k / side) as f64 * cell + 9.0,
            ),
            value: format!("{}", 1000 + k),
        })
        .collect();
    let start = std::time::Instant::now();
    let r = polygonize(&lines, &labels, true);
    let took = start.elapsed().as_secs_f64();
    println!(
        "{} çizgi, {} etiket: {:.3} sn, {} bölge",
        lines.len(),
        labels.len(),
        took,
        r.regions.len()
    );
    assert_eq!(r.regions.len(), side * side);
    assert!(r.regions.iter().all(|g| g.labels.len() == 1));
    assert_eq!(
        r.regions.iter().filter(|g| g.existing.is_some()).count(),
        side * side / 4
    );
    assert!(r.free_ends.is_empty());
}
