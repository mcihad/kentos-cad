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

/// A block's rings out and back in by `d` (a closing, as Alanları kümele makes it): its area and its parts (the
/// reference's rule: none below 10⁻⁶ m²).
fn closing_of(rings: &[Vec<Vec2>], d: f64) -> (f64, usize) {
    use kentos_geometry_core::entity::Shape;
    use kentos_geometry_core::geom::region::net_area;
    use kentos_geometry_core::ops::geoprocess::Class;
    use kentos_geometry_core::ops::geoprocess::buffer::{Side, buffer};
    let areas: Vec<_> = rings
        .iter()
        .flat_map(|pts| {
            areas_of_entity(&Shape::Polygon {
                pts: pts.clone(),
                bulges: None,
                holes: None,
                parts: None,
            })
        })
        .collect();
    let out = buffer(&Class::Areas(areas), d, Side::Both);
    let closing = buffer(&Class::Areas(out), -d, Side::Both);
    let parts: Vec<f64> = closing.iter().map(net_area).filter(|a| *a > 1e-6).collect();
    (parts.iter().sum(), parts.len())
}

/// The closings of near blocks (docs/adr/0221 §10): four turned buildings out and back in by the same distance,
/// against the exact closing of `scripts/fixtures/closing_cases.py`, among them the blocks whose faces the overlay
/// once lost where circles met within 10⁻⁶ m.
#[test]
fn closings_of_near_blocks_are_exact() {
    let path = format!(
        "{}/../../../fixtures/geoprocess/v1/closings.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let file: Value = serde_json::from_str(&std::fs::read_to_string(path).expect("fixture file"))
        .expect("fixture JSON");
    assert_eq!(file["format"], "kentos.closing-cases");
    let tol = file["tolerance"]["area"].as_f64().expect("a tolerance");
    let cases = file["cases"].as_array().expect("cases");
    assert!(cases.len() >= 45, "{} durum", cases.len());
    for c in cases {
        let name = c["name"].as_str().expect("a name");
        let rings: Vec<Vec<Vec2>> = c["rings"]
            .as_array()
            .expect("rings")
            .iter()
            .map(|r| {
                r.as_array()
                    .expect("a ring")
                    .iter()
                    .map(|p| Vec2::new(p[0].as_f64().expect("x"), p[1].as_f64().expect("y")))
                    .collect()
            })
            .collect();
        let (area, parts) = closing_of(&rings, c["distance"].as_f64().expect("a distance"));
        assert_eq!(Some(parts as u64), c["parts"].as_u64(), "{name}: parça");
        let want = c["area"].as_f64().expect("an area");
        assert!(
            (area - want).abs() <= tol,
            "{name}: alan {area}, beklenen {want}"
        );
    }
}

/// Random blocks' closings for `closing_cases.py --stress FILE` (`--ignored`; KENTOS_CLOSINGS_OUT=FILE, and
/// KENTOS_BLOCKS, KENTOS_SEED, KENTOS_DISTANCE): four stepped buildings a block, gaps of 2 to 4.5 m, each turned up to
/// 3° either way and its corners moved up to 0.2 m.
#[test]
#[ignore = "zorlama: KENTOS_CLOSINGS_OUT=… cargo test -p kentos-geometry-core --test all closings_of_random_blocks -- --ignored"]
fn closings_of_random_blocks() {
    use kentos_geometry_core::jsmath::{cos, sin};
    let var = |k: &str, d: f64| {
        std::env::var(k)
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(d)
    };
    let (n, distance) = (
        var("KENTOS_BLOCKS", 300.0) as usize,
        var("KENTOS_DISTANCE", 2.5),
    );
    let mut seed = var("KENTOS_SEED", 221.0) as u64 | 1;
    let mut rnd = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed >> 11) as f64 / (1u64 << 53) as f64
    };
    let shape = [
        (0.0, 0.0),
        (12.0, 0.0),
        (12.0, 8.0),
        (6.0, 8.0),
        (6.0, 8.8),
        (0.0, 8.8),
    ];
    let mut out = Vec::new();
    for b in 0..n {
        let (bx, by) = (
            487_000.0 + (b % 20) as f64 * 70.0 + rnd(),
            4_420_000.0 + (b / 20) as f64 * 60.0 + rnd(),
        );
        let (gx, gy) = (2.0 + 2.5 * rnd(), 2.0 + 2.5 * rnd());
        let mut rings = Vec::new();
        for (ox, oy) in [
            (0.0, 0.0),
            (12.0 + gx, 0.0),
            (0.0, 8.8 + gy),
            (12.0 + gx, 8.8 + gy),
        ] {
            let turn = (rnd() - 0.5) * 6.0 * std::f64::consts::PI / 180.0;
            let (s, c) = (sin(turn), cos(turn));
            let ring: Vec<Vec2> = shape
                .iter()
                .map(|&(x, y)| {
                    let (x, y) = (x - 6.0 + (rnd() - 0.5) * 0.4, y - 4.4 + (rnd() - 0.5) * 0.4);
                    Vec2::new(bx + ox + 6.0 + x * c - y * s, by + oy + 4.4 + x * s + y * c)
                })
                .collect();
            rings.push(ring);
        }
        let (area, count) = closing_of(&rings, distance);
        out.push(json!({
            "rings": rings.iter().map(|r| r.iter().map(|p| [p.x, p.y]).collect::<Vec<_>>()).collect::<Vec<_>>(),
            "distance": distance,
            "area": area,
            "count": count,
        }));
    }
    let path = std::env::var("KENTOS_CLOSINGS_OUT").expect("KENTOS_CLOSINGS_OUT");
    std::fs::write(path, serde_json::to_string(&out).expect("JSON")).expect("written");
}
