//! Seçim ekleri (docs/adr/0187) against the independent reference in
//! `fixtures/selection/v1` (`scripts/fixtures/selection_cases.py`, written
//! from the ADR, no KentOS code): every object a click could mean in the
//! order Sıradakini seç steps through, the first of them the click's pick;
//! Çokgenle seç's three modes on a concave ring; the rings that cannot
//! select, in the tools' words. The web runs the same files through WASM
//! (`apps/web/src/viewport/selection.wasm.test.ts`).

use kentos_geometry_core::api::run_named;
use kentos_geometry_core::store::Store;
use kentos_geometry_core::store::polygon::PolygonMode;
use kentos_geometry_core::vec2::Vec2;
use serde_json::Value;

fn file(name: &str) -> Value {
    let path = format!(
        "{}/../../../fixtures/selection/v1/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    serde_json::from_str(&std::fs::read_to_string(path).expect("the cases")).expect("JSON")
}

fn store(objects: &Value) -> Store {
    let mut s = Store::new();
    s.put_json(&objects.to_string()).expect("the objects");
    s
}

fn ids(v: &Value) -> Vec<f64> {
    v.as_array()
        .expect("ids")
        .iter()
        .map(|x| x.as_f64().expect("an id"))
        .collect()
}

fn point(v: &Value) -> Vec2 {
    Vec2::new(v[0].as_f64().expect("x"), v[1].as_f64().expect("y"))
}

#[test]
fn a_click_s_candidates_come_most_specific_first() {
    let f = file("hits.json");
    let s = store(&f["objects"]);
    for c in f["cases"].as_array().expect("cases") {
        let at = point(&c["at"]);
        let tol = c["tol"].as_f64().expect("tol");
        let want = ids(&c["expect"]);
        assert_eq!(s.hits(at, tol), want, "{}", c["name"]);
        assert_eq!(
            s.hit(at, tol),
            want.first().copied(),
            "{}: the pick",
            c["name"]
        );
    }
}

#[test]
fn a_polygon_selects_inside_crossing_and_outside() {
    let f = file("polygon.json");
    let s = store(&f["objects"]);
    for c in f["cases"].as_array().expect("cases") {
        let ring: Vec<Vec2> = c["ring"]
            .as_array()
            .expect("ring")
            .iter()
            .map(point)
            .collect();
        for (mode, key) in [
            (PolygonMode::Inside, "inside"),
            (PolygonMode::Crossing, "crossing"),
            (PolygonMode::Outside, "outside"),
        ] {
            assert_eq!(
                s.in_polygon(&ring, mode),
                ids(&c[key]),
                "{} {key}",
                c["name"]
            );
        }
    }
}

#[test]
fn a_ring_that_cannot_select_is_said_in_the_tools_words() {
    let f = file("polygon.json");
    for r in f["rings"].as_array().expect("rings") {
        let ring: Vec<Value> = r["ring"]
            .as_array()
            .expect("ring")
            .iter()
            .map(|p| serde_json::json!({ "x": p[0], "y": p[1] }))
            .collect();
        let out = run_named(
            "selectionRingProblem",
            &Value::Array(vec![Value::Array(ring)]).to_string(),
        )
        .expect("the call");
        let got: Value = serde_json::from_str(&out).expect("JSON");
        assert_eq!(got, r["problem"], "{}", r["name"]);
    }
}
