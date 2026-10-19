//! The chords of arcs, circles and bulged edges (docs/adr/0530 §2) against the independent reference in
//! `fixtures/gis-derivation/v1/chords.json` (`scripts/fixtures/cad_gis_derivation_cases.py`: the count and the
//! sagitta at 50 digits with mpmath, no KentOS code).

use kentos_geometry_core::jsmath::{PI, TAU, js_hypot, js_max};
use kentos_geometry_core::ops::derive::{
    MAX_CHORDS, arc_chord_count, arc_chords, path_chords, sagitta,
};
use kentos_geometry_core::vec2::Vec2;
use serde_json::Value;

fn cases() -> Value {
    let path = format!(
        "{}/../../../fixtures/gis-derivation/v1/chords.json",
        env!("CARGO_MANIFEST_DIR")
    );
    serde_json::from_str(&std::fs::read_to_string(path).expect("the cases")).expect("JSON")
}

fn f(v: &Value) -> f64 {
    v.as_f64().expect("a number")
}

fn near(got: f64, want: f64, what: &str) {
    assert!(
        (got - want).abs() <= 1e-15 + 1e-12 * want.abs(),
        "{what}: {got} ≠ {want}"
    );
}

fn point(v: &Value) -> Vec2 {
    let a = v.as_array().expect("a point");
    Vec2::new(f(&a[0]), f(&a[1]))
}

#[test]
fn an_arc_is_cut_into_the_least_chords_that_hold_the_tolerance() {
    let c = cases();
    let arcs = c["arcs"].as_array().expect("arcs");
    assert!(arcs.len() >= 20, "{} arcs", arcs.len());
    for a in arcs {
        let name = a["name"].as_str().expect("a name");
        let (r, sweep, tol) = (f(&a["r"]), f(&a["sweep"]), f(&a["tolerance"]));
        let want = a["chords"].as_u64().expect("a count") as usize;
        assert_eq!(arc_chord_count(r, sweep, tol), want, "{name}: chords");
        let run = arc_chords(Vec2::new(10.5, -3.25), r, 0.3, sweep, tol);
        assert_eq!(run.pts.len(), want + 1, "{name}: points");
        near(run.deviation, f(&a["deviation"]), name);
        let capped = a["capped"].as_bool().expect("capped");
        assert_eq!(capped, want == MAX_CHORDS, "{name}: the cap");
        // Over the tolerance only where the count was capped, and then the deviation says so.
        assert_eq!(
            run.deviation > tol,
            capped,
            "{name}: {} against {tol}",
            run.deviation
        );
        // Every point on the circle, one angle between them.
        for p in &run.pts {
            let d = js_hypot(p.x - 10.5, p.y + 3.25);
            assert!(
                (d - r).abs() <= 1e-12 * js_max(r, 1.0),
                "{name}: {d} off the circle {r}"
            );
        }
        // A chord spans at most a quarter turn.
        assert!(
            sweep.abs() / want as f64 <= PI / 2.0 + 1e-12,
            "{name}: a chord over a quarter turn"
        );
        near(sagitta(r, sweep / want as f64), f(&a["deviation"]), name);
    }
}

#[test]
fn a_bulged_edge_has_the_arc_the_reference_works_out() {
    let c = cases();
    let edges = c["bulges"].as_array().expect("bulges");
    assert!(edges.len() >= 8, "{} bulges", edges.len());
    for e in edges {
        let name = e["name"].as_str().expect("a name");
        let (a, b) = (point(&e["a"]), point(&e["b"]));
        let run = path_chords(&[a, b], Some(&[f(&e["bulge"])]), false, f(&e["tolerance"]));
        let want = e["chords"].as_u64().expect("a count") as usize;
        // The chords of one edge: its start, the arc's n - 1 inside points and the end.
        assert_eq!(run.pts.len(), want + 1, "{name}: points");
        near(run.deviation, f(&e["deviation"]), name);
        assert_eq!(run.pts.first(), Some(&a), "{name}: start");
        assert_eq!(run.pts.last(), Some(&b), "{name}: end");
        let centre = point(&e["center"]);
        let r = f(&e["r"]);
        for p in &run.pts {
            let d = js_hypot(p.x - centre.x, p.y - centre.y);
            assert!(
                (d - r).abs() <= 1e-8 * js_max(r, 1.0),
                "{name}: {d} off the circle {r}"
            );
        }
        // The fractions are k/n along the edge, 0 at its start.
        assert_eq!(run.at.len(), run.pts.len());
        for (k, (edge, fr)) in run.at.iter().enumerate().take(want) {
            assert_eq!(*edge, 0, "{name}");
            near(*fr, k as f64 / want as f64, name);
        }
    }
}

#[test]
fn a_path_keeps_its_vertices_and_cuts_only_its_arcs() {
    let c = cases();
    let paths = c["paths"].as_array().expect("paths");
    assert!(paths.len() >= 5, "{} paths", paths.len());
    for p in paths {
        let name = p["name"].as_str().expect("a name");
        let pts: Vec<Vec2> = p["pts"]
            .as_array()
            .expect("pts")
            .iter()
            .map(point)
            .collect();
        let bulges: Option<Vec<f64>> = p["bulges"].as_array().map(|b| b.iter().map(f).collect());
        let closed = p["closed"].as_bool().expect("closed");
        let run = path_chords(&pts, bulges.as_deref(), closed, f(&p["tolerance"]));
        assert_eq!(
            run.pts.len(),
            p["count"].as_u64().expect("a count") as usize,
            "{name}: points"
        );
        near(run.deviation, f(&p["deviation"]), name);
        for v in &pts {
            assert!(run.pts.contains(v), "{name}: a vertex was moved");
        }
        // A closed ring does not repeat its first point.
        if closed {
            assert_ne!(run.pts.first(), run.pts.last(), "{name}");
        }
    }
}

#[test]
fn the_count_is_capped_and_a_degenerate_arc_is_one_chord() {
    assert_eq!(
        arc_chord_count(1.0e6, TAU, 1e-6).min(MAX_CHORDS),
        MAX_CHORDS
    );
    assert_eq!(arc_chord_count(0.0, TAU, 1e-4), 1);
    assert_eq!(arc_chord_count(5.0, 0.0, 1e-4), 1);
    assert_eq!(arc_chord_count(f64::NAN, 1.0, 1e-4), 1);
}
