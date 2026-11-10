//! A building section's geometry (docs/adr/8042 §2) against the independent reference's cases
//! (fixtures/arch/v1/sections.json, scripts/fixtures/section_cases.py, exact fractions): each solid's cut rings (the
//! same points in the same turn, from any of them) and its seen pieces (the same segments, either way round, with the
//! same depth), within 1e-7.

use kentos_geometry_core::geom::section::{Flight, Frame, Height, Solid, Void, section};
use kentos_geometry_core::vec2::Vec2;
use serde_json::Value;

const CASES: &str = include_str!("../../../../../fixtures/arch/v1/sections.json");
const TOL: f64 = 1e-7;

fn pt(v: &Value) -> Vec2 {
    Vec2::new(v[0].as_f64().unwrap(), v[1].as_f64().unwrap())
}

fn ring(v: &Value) -> Vec<Vec2> {
    v.as_array().unwrap().iter().map(pt).collect()
}

fn solid(v: &Value) -> Solid {
    let height = match v.get("flight") {
        Some(f) => Height::Flight(Flight {
            start: pt(&f["start"]),
            dir: pt(&f["dir"]),
            steps: f["steps"]
                .as_array()
                .unwrap()
                .iter()
                .map(|s| (s[0].as_f64().unwrap(), s[1].as_f64().unwrap()))
                .collect(),
            soffit: f["soffit"].as_f64().unwrap(),
            slope: f["slope"].as_f64().unwrap(),
            floor: f["floor"].as_f64().unwrap(),
        }),
        None => Height::Prism {
            bottom: v["bottom"].as_f64().unwrap(),
            top: v["top"].as_f64().unwrap(),
        },
    };
    let none = Vec::new();
    Solid {
        rings: v["rings"].as_array().unwrap().iter().map(ring).collect(),
        height,
        voids: v["voids"]
            .as_array()
            .unwrap_or(&none)
            .iter()
            .map(|w| Void {
                ring: ring(&w["ring"]),
                bottom: w["bottom"].as_f64().unwrap(),
                top: w["top"].as_f64().unwrap(),
            })
            .collect(),
        marks: v["marks"]
            .as_array()
            .unwrap_or(&none)
            .iter()
            .map(|m| {
                let end = |e: &Value| {
                    (
                        Vec2::new(e[0].as_f64().unwrap(), e[1].as_f64().unwrap()),
                        e[2].as_f64().unwrap(),
                    )
                };
                [end(&m[0]), end(&m[1])]
            })
            .collect(),
    }
}

fn close(a: Vec2, b: Vec2) -> bool {
    (a.x - b.x).abs() <= TOL && (a.y - b.y).abs() <= TOL
}

/// Whether two rings are the same points in the same turn, from any of them.
fn same_ring(a: &[Vec2], b: &[Vec2]) -> bool {
    a.len() == b.len()
        && (0..a.len()).any(|k| (0..a.len()).all(|i| close(a[(i + k) % a.len()], b[i])))
}

#[test]
fn sections_cut_and_see_as_the_reference_says() {
    let cases: Value = serde_json::from_str(CASES).unwrap();
    let mut seen_cases = 0;
    for c in cases["cases"].as_array().unwrap() {
        let name = c["name"].as_str().unwrap();
        let line = &c["line"];
        let frame = Frame::new(
            pt(&line["a"]),
            pt(&line["b"]),
            line["flip"].as_bool().unwrap_or(false),
            line["depth"].as_f64(),
        )
        .unwrap();
        let solids: Vec<Solid> = c["solids"].as_array().unwrap().iter().map(solid).collect();
        let got = section(&frame, &solids);
        let want_cut = c["expect"]["cut"].as_array().unwrap();
        assert_eq!(got.cut.len(), want_cut.len(), "{name}: cut {:?}", got.cut);
        for w in want_cut {
            let (k, r) = (w["solid"].as_u64().unwrap() as usize, ring(&w["ring"]));
            assert!(
                got.cut
                    .iter()
                    .any(|g| g.solid == k && same_ring(&g.ring, &r)),
                "{name}: no cut of solid {k} as {r:?}: {:?}",
                got.cut
            );
        }
        let want_seen = c["expect"]["seen"].as_array().unwrap();
        assert_eq!(
            got.seen.len(),
            want_seen.len(),
            "{name}: seen {:#?}",
            got.seen
        );
        for w in want_seen {
            let (k, a, b, d) = (
                w["solid"].as_u64().unwrap() as usize,
                pt(&w["a"]),
                pt(&w["b"]),
                w["depth"].as_f64().unwrap(),
            );
            assert!(
                got.seen.iter().any(|g| g.solid == k
                    && ((close(g.a, a) && close(g.b, b)) || (close(g.a, b) && close(g.b, a)))
                    && (g.depth - d).abs() <= TOL),
                "{name}: no seen piece of solid {k} from {a:?} to {b:?} at {d}: {:#?}",
                got.seen
            );
        }
        seen_cases += 1;
    }
    assert_eq!(seen_cases, 17, "every case read");
}
