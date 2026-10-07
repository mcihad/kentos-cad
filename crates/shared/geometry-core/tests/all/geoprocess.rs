//! Geometri işlemleri (docs/adr/0201) against the independent reference in
//! `fixtures/geoprocess/v1/cases.json` (`scripts/fixtures/geoprocess_cases.py`,
//! no KentOS code): Tampon's pieces (closed-form areas, places inside and
//! outside), Kes, the overlays, Birleştir, the problems Geçerliliği denetle
//! finds and where, Onar, Sadeleştir, Koordinat sistemine dönüştür (PROJ's
//! vertices) and Alan oranıyla paylaştır; by name, as the web calls them
//! through WASM (`ops::geoprocess::calls`, `ops::statistics::apportion`).

use kentos_geometry_core::api::json::{FromJson, Json};
use kentos_geometry_core::api::run_named;
use kentos_geometry_core::entity::Entity;
use kentos_geometry_core::geom::region::inside_area;
use kentos_geometry_core::ops::areas::areas_of_entity;
use kentos_geometry_core::vec2::Vec2;
use serde_json::{Value, json};

fn cases() -> Value {
    let path = format!(
        "{}/../../../fixtures/geoprocess/v1/cases.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let file: Value = serde_json::from_str(&std::fs::read_to_string(path).expect("fixture file"))
        .expect("fixture JSON");
    assert_eq!(file["format"], "kentos.geoprocess-cases");
    file
}

fn call(name: &str, args: Value) -> Value {
    serde_json::from_str(&run_named(name, &args.to_string()).expect(name)).expect("JSON")
}

fn tol(f: &Value, what: &str) -> f64 {
    f["tolerance"][what].as_f64().expect("a tolerance")
}

fn pt(v: &Value) -> Vec2 {
    Vec2::new(v["x"].as_f64().expect("x"), v["y"].as_f64().expect("y"))
}

fn entity(v: &Value) -> Entity {
    Entity::from_json(&Json::parse(&v.to_string()).expect("JSON")).expect("an entity")
}

/// An object's kind, parts, holes and area, length or points against the reference's.
fn same_measures(name: &str, f: &Value, got: &Value, want: &Value) {
    assert_eq!(got["kind"], want["kind"], "{name}: tür");
    let m = &call("geoMeasure", json!([[got]]))[0];
    assert_eq!(m["parts"], want["parts"], "{name}: parça");
    assert_eq!(m["holes"], want["holes"], "{name}: delik");
    for (key, t) in [("area", "area"), ("length", "length")] {
        if let Some(w) = want[key].as_f64() {
            let g = m[key].as_f64().unwrap_or(f64::NAN);
            assert!(
                (g - w).abs() <= tol(f, t),
                "{name}: {key} {g}, beklenen {w}"
            );
        }
    }
    if let Some(w) = want["points"].as_array() {
        let g = m["points"].as_array().expect("points");
        assert_eq!(g.len(), w.len(), "{name}: nokta sayısı");
        for (a, b) in g.iter().zip(w) {
            let (a, b) = (pt(a), pt(b));
            assert!(
                (a.x - b.x).abs() <= tol(f, "point") && (a.y - b.y).abs() <= tol(f, "point"),
                "{name}: {a:?}, beklenen {b:?}"
            );
        }
    }
}

#[test]
fn buffers_are_the_closed_forms() {
    let f = cases();
    for c in f["buffer"].as_array().expect("buffer") {
        let name = format!("tampon {}", c["id"]);
        let got = call(
            "geoBuffer",
            json!([
                c["shapes"],
                c["distances"],
                c["side"],
                c["rings"],
                c["dissolve"]
            ]),
        );
        let want = &c["expect"];
        for key in ["unread", "inward", "empty"] {
            assert_eq!(got[key], want[key], "{name}: {key}");
        }
        let (pieces, wanted) = (
            got["pieces"].as_array().expect("pieces"),
            want["pieces"].as_array().expect("pieces"),
        );
        assert_eq!(
            pieces.len(),
            wanted.len(),
            "{name}: parça sayısı {pieces:?}"
        );
        for (p, w) in pieces.iter().zip(wanted) {
            let at = format!("{name}, halka {}", w["ring"]);
            for key in ["source", "ring", "distance"] {
                assert_eq!(p[key], w[key], "{at}: {key}");
            }
            let mut m = w.clone();
            m["kind"] = json!("polygon");
            same_measures(&at, &f, &p["shape"], &m);
            let areas = areas_of_entity(&entity(&p["shape"]).shape);
            let inside = |q: Vec2| areas.iter().any(|a| inside_area(a, q));
            for q in w["inside"].as_array().expect("inside") {
                assert!(inside(pt(q)), "{at}: {q} içeride olmalı");
            }
            for q in w["outside"].as_array().expect("outside") {
                assert!(!inside(pt(q)), "{at}: {q} dışarıda olmalı");
            }
        }
    }
}

#[test]
fn clipping_keeps_what_is_inside() {
    let f = cases();
    for c in f["clip"].as_array().expect("clip") {
        let name = format!("kes {}", c["id"]);
        let got = call("geoClip", json!([c["shapes"], c["cut"]]));
        let (got, want) = (
            got.as_array().expect("list"),
            c["expect"].as_array().expect("list"),
        );
        assert_eq!(got.len(), want.len(), "{name}");
        for (g, w) in got.iter().zip(want) {
            if w.is_null() {
                assert!(g.is_null(), "{name}: boş olmalı, {g}");
            } else {
                same_measures(&name, &f, g, w);
            }
        }
    }
}

#[test]
fn overlays_give_their_pieces_in_order() {
    let f = cases();
    for c in f["overlay"].as_array().expect("overlay") {
        let name = format!("bindirme {}", c["id"]);
        let got = call("geoOverlay", json!([c["a"], c["b"], c["mode"]]));
        let (got, want) = (
            got.as_array().expect("list"),
            c["expect"].as_array().expect("list"),
        );
        assert_eq!(got.len(), want.len(), "{name}: {got:?}");
        for (k, (g, w)) in got.iter().zip(want).enumerate() {
            let at = format!("{name}[{k}]");
            assert_eq!(g["a"], w["a"], "{at}: a");
            assert_eq!(g["b"], w["b"], "{at}: b");
            match w["share"].as_f64() {
                Some(s) => {
                    let gs = g["share"].as_f64().expect("a share");
                    assert!((gs - s).abs() <= 1e-12, "{at}: pay {gs}, beklenen {s}");
                }
                None => assert!(g["share"].is_null(), "{at}: pay olmamalı"),
            }
            same_measures(&at, &f, &g["shape"], w);
        }
    }
}

#[test]
fn dissolving_joins_each_group() {
    let f = cases();
    for c in f["dissolve"].as_array().expect("dissolve") {
        let name = format!("birleştir {}", c["id"]);
        let got = call("geoDissolve", json!([c["shapes"], c["groups"], c["multi"]]));
        let (got, want) = (
            got.as_array().expect("list"),
            c["expect"].as_array().expect("list"),
        );
        assert_eq!(got.len(), want.len(), "{name}: {got:?}");
        for (g, w) in got.iter().zip(want) {
            assert_eq!(g["group"], w["group"], "{name}: grup");
            same_measures(&name, &f, &g["shape"], w);
        }
    }
}

#[test]
fn problems_are_found_where_they_first_show() {
    let f = cases();
    for c in f["validity"].as_array().expect("validity") {
        let name = format!("geçerlilik {}", c["id"]);
        let got = call("geoValidity", json!([[c["shape"]]]));
        let (got, want) = (
            got.as_array().expect("list"),
            c["expect"].as_array().expect("list"),
        );
        assert_eq!(got.len(), want.len(), "{name}: {got:?}");
        for (g, w) in got.iter().zip(want) {
            assert_eq!(g["problem"], w["problem"], "{name}");
            let (a, b) = (pt(&g["at"]), pt(&w["at"]));
            assert!(
                (a.x - b.x).abs() <= 1e-6 && (a.y - b.y).abs() <= 1e-6,
                "{name}: yer {a:?}, beklenen {b:?}"
            );
        }
    }
}

#[test]
fn repair_rebuilds_from_the_rings() {
    let f = cases();
    for c in f["repair"].as_array().expect("repair") {
        let name = format!("onar {}", c["id"]);
        let got = &call("geoRepair", json!([[c["shape"]]]))[0];
        let want = &c["expect"];
        for key in ["parts", "holes", "vertices", "problems"] {
            assert_eq!(got[key], want[key], "{name}: {key}");
        }
        match want["area"].as_array() {
            Some(w) => {
                let g = got["area"].as_array().expect("areas");
                for (a, b) in g.iter().zip(w) {
                    let (a, b) = (a.as_f64().expect("a"), b.as_f64().expect("b"));
                    assert!(
                        (a - b).abs() <= tol(&f, "area"),
                        "{name}: alan {a}, beklenen {b}"
                    );
                }
            }
            None => assert!(got["area"].is_null(), "{name}: alan olmamalı"),
        }
        if want["after"].is_null() {
            assert!(got["shape"].is_null(), "{name}: bir şey kalmamalı");
        } else {
            same_measures(&name, &f, &got["shape"], &want["after"]);
        }
    }
}

/// An object's vertices in order: an area's ring then its holes, a path's, a point's.
fn vertices(shape: &Value) -> Vec<Vec2> {
    let list = |v: &Value| -> Vec<Vec2> {
        v.as_array()
            .map(|a| a.iter().map(pt).collect())
            .unwrap_or_default()
    };
    match shape["kind"].as_str() {
        Some("polygon") => {
            let mut out = list(&shape["pts"]);
            for h in shape["holes"].as_array().into_iter().flatten() {
                out.extend(list(&h["pts"]));
            }
            out
        }
        Some("polyline") => list(&shape["pts"]),
        Some("point") => {
            let mut out = vec![pt(&shape["p"])];
            for q in shape["parts"].as_array().into_iter().flatten() {
                out.push(pt(&q["p"]));
            }
            out
        }
        _ => Vec::new(),
    }
}

fn same_points(name: &str, got: &[Vec2], want: &Value, tolerance: f64) {
    let want: Vec<Vec2> = want.as_array().expect("points").iter().map(pt).collect();
    assert_eq!(got.len(), want.len(), "{name}: köşe sayısı");
    for (a, b) in got.iter().zip(&want) {
        assert!(
            (a.x - b.x).abs() <= tolerance && (a.y - b.y).abs() <= tolerance,
            "{name}: {a:?}, beklenen {b:?}"
        );
    }
}

#[test]
fn simplifying_keeps_the_far_vertices() {
    let f = cases();
    for c in f["simplify"].as_array().expect("simplify") {
        let name = format!("sadeleştir {}", c["id"]);
        let got = &call("geoSimplify", json!([[c["shape"]], c["tolerance"]]))[0];
        let want = &c["expect"];
        assert_eq!(got["vertices"], want["vertices"], "{name}: köşeler");
        assert_eq!(got["changed"], want["changed"], "{name}: değişti mi");
        let dev = got["deviation"].as_f64().expect("deviation");
        let want_dev = want["deviation"].as_f64().expect("deviation");
        assert!(
            (dev - want_dev).abs() <= 1e-9,
            "{name}: sapma {dev}, beklenen {want_dev}"
        );
        match want["area"].as_array() {
            Some(w) => {
                for (a, b) in got["area"].as_array().expect("areas").iter().zip(w) {
                    let (a, b) = (a.as_f64().expect("a"), b.as_f64().expect("b"));
                    assert!(
                        (a - b).abs() <= tol(&f, "area"),
                        "{name}: alan {a}, beklenen {b}"
                    );
                }
            }
            None => assert!(got["area"].is_null(), "{name}: alan olmamalı"),
        }
        if !want["after"].is_null() {
            same_points(
                &name,
                &vertices(&got["shape"]),
                &want["after"],
                tol(&f, "point"),
            );
        }
    }
}

#[test]
fn reprojection_moves_every_vertex_as_proj() {
    let f = cases();
    for c in f["reproject"].as_array().expect("reproject") {
        let name = format!("dönüştür {}", c["id"]);
        let got = &call(
            "geoReproject",
            json!([[c["shape"]], c["from"], c["to"], null]),
        )[0];
        let want = &c["expect"];
        if let Some(e) = want["error"].as_str() {
            assert_eq!(got["error"], json!(e), "{name}: neden");
            assert!(got["shape"].is_null(), "{name}: şekil olmamalı");
            continue;
        }
        assert!(got["error"].is_null(), "{name}: {}", got["error"]);
        assert_eq!(
            got["chorded"], want["chorded"],
            "{name}: doğru parçalarına çevrilen yay"
        );
        same_points(
            &name,
            &vertices(&got["shape"]),
            &want["points"],
            tol(&f, "point"),
        );
    }
}

#[test]
fn shares_are_exact_and_rounded_half_to_even() {
    let f = cases();
    for c in f["apportion"].as_array().expect("apportion") {
        let got = &call("apportion", json!([[c["value"]], c["share"]]))[0];
        assert_eq!(got, &c["expect"], "{} × {}", c["value"], c["share"]);
    }
}

/// How long Tampon, Birleştir and Kesişim take on a parcel layer (`--ignored --nocapture`): 400 parcels of 24
/// vertices each (a 20 × 20 grid of irregular 24-gons), buffered 2 m one by one and joined, dissolved, and
/// intersected with a second grid shifted half a parcel.
#[test]
#[ignore = "süre ölçümü: cargo test --release -p kentos-geometry-core --test all geoprocess::timing -- --ignored --nocapture"]
fn timing() {
    use kentos_geometry_core::entity::Shape;
    use kentos_geometry_core::ops::geoprocess::buffer::Side;
    use kentos_geometry_core::ops::geoprocess::calls::{
        Mode, buffer_run, dissolve_run, overlay_run,
    };
    let parcel = |i: usize, j: usize, dx: f64| {
        let (cx, cy) = (
            487000.0 + i as f64 * 40.0 + dx,
            4420000.0 + j as f64 * 40.0 + dx,
        );
        let pts: Vec<Vec2> = (0..24)
            .map(|k| {
                let a = k as f64 / 24.0 * std::f64::consts::TAU;
                let r = 19.5 + if k % 2 == 0 { 0.4 } else { -0.3 };
                Vec2::new(
                    cx + r * kentos_geometry_core::jsmath::cos(a),
                    cy + r * kentos_geometry_core::jsmath::sin(a),
                )
            })
            .collect();
        Shape::Polygon {
            pts,
            bulges: None,
            holes: None,
            parts: None,
        }
    };
    let grid = |dx: f64| -> Vec<Shape> {
        (0..20)
            .flat_map(|i| (0..20).map(move |j| parcel(i, j, dx)))
            .collect()
    };
    let (a, b) = (grid(0.0), grid(20.0));
    let distances = vec![Some("2".to_owned()); a.len()];
    let t = std::time::Instant::now();
    let one = buffer_run(&a, &distances, Side::Both, 1, false);
    println!(
        "Tampon, 400 parsel tek tek: {:?} ({} parça)",
        t.elapsed(),
        one.pieces.len()
    );
    let t = std::time::Instant::now();
    let joined = buffer_run(&a, &distances, Side::Both, 1, true);
    println!(
        "Tampon, birleştirerek: {:?} ({} parça)",
        t.elapsed(),
        joined.pieces.len()
    );
    let t = std::time::Instant::now();
    let d = dissolve_run(&a, &vec![0; a.len()], true);
    println!("Birleştir, tek grup: {:?} ({} nesne)", t.elapsed(), d.len());
    let t = std::time::Instant::now();
    let o = overlay_run(&a, &b, Mode::Intersection);
    println!("Kesişim, 400 × 400: {:?} ({} parça)", t.elapsed(), o.len());
}
