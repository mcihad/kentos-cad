//! An insert's own y factor (docs/adr/0315 §3) against the independent reference's cases
//! (fixtures/blocks/v1/scales.json, scripts/fixtures/block_scale_cases.py): every placed piece compared as
//! geometry: an ellipse by its centre, semi-axes and axis, an arc by its ends, its sense and its middle, a text by
//! its point, height, width factor and turn, a path's arc edge by its vertices on the exact image within 0.1 mm;
//! and Patlat's nested inserts, kept while square, refused when they shear.

use kentos_geometry_core::api::json::{FromJson, Json};
use kentos_geometry_core::block::{Blocks, shows_nothing};
use kentos_geometry_core::entity::{Entity, Shape};
use kentos_geometry_core::ops::curve_cuts::Cut;
use kentos_geometry_core::jsmath::{atan2, cos, js_hypot, js_max, js_min, sin};
use kentos_geometry_core::vec2::Vec2;
use serde_json::Value;

const CASES: &str = include_str!("../../../../../fixtures/blocks/v1/scales.json");
const TOL: f64 = 1e-9;

fn parse(v: &Value) -> Json {
    Json::parse(&v.to_string()).expect("JSON")
}

fn pt(v: &Value) -> Vec2 {
    Vec2::new(v[0].as_f64().unwrap(), v[1].as_f64().unwrap())
}

fn near(name: &str, what: &str, got: Vec2, want: Vec2, tol: f64) {
    assert!(
        (got.x - want.x).abs() <= tol && (got.y - want.y).abs() <= tol,
        "{name}: {what} {got:?}, başvuru {want:?}"
    );
}

fn num(name: &str, what: &str, got: f64, want: f64) {
    assert!(
        (got - want).abs() <= TOL * js_max(want.abs(), 1.0),
        "{name}: {what} {got}, başvuru {want}"
    );
}

/// An angle compared modulo `period`.
fn angle(name: &str, what: &str, got: f64, want: f64, period: f64) {
    let d = (got - want).rem_euclid(period);
    assert!(
        js_min(d, period - d) <= 1e-9,
        "{name}: {what} {got}, başvuru {want}"
    );
}

/// An ellipse's or a circle's centre, semi-axes (major first) and the major's direction modulo π; and its point at
/// a parameter.
struct Curve {
    c: Vec2,
    semi: [f64; 2],
    axis: f64,
    at: Box<dyn Fn(f64) -> Vec2>,
    t0: f64,
    t1: f64,
}

fn curve(s: &Shape) -> Option<Curve> {
    match s {
        Shape::Circle { c, r } => {
            let (c, r) = (*c, *r);
            Some(Curve {
                c,
                semi: [r, r],
                axis: 0.0,
                at: Box::new(move |t| Vec2::new(c.x + r * cos(t), c.y + r * sin(t))),
                t0: 0.0,
                t1: std::f64::consts::TAU,
            })
        }
        Shape::Arc { c, r, a0, a1 } => {
            let (c, r) = (*c, *r);
            let a1 = if a1 < a0 { a1 + std::f64::consts::TAU } else { *a1 };
            Some(Curve {
                c,
                semi: [r, r],
                axis: 0.0,
                at: Box::new(move |t| Vec2::new(c.x + r * cos(t), c.y + r * sin(t))),
                t0: *a0,
                t1: a1,
            })
        }
        Shape::Ellipse {
            c,
            major,
            ratio,
            t0,
            t1,
        } => {
            let (c, mj, ratio) = (*c, *major, *ratio);
            let a = js_hypot(mj.x, mj.y);
            let mn = Vec2::new(-mj.y * ratio, mj.x * ratio);
            let t1 = if t1 < t0 { t1 + std::f64::consts::TAU } else { *t1 };
            Some(Curve {
                c,
                semi: [a, a * ratio],
                axis: atan2(mj.y, mj.x).rem_euclid(std::f64::consts::PI),
                at: Box::new(move |t| {
                    Vec2::new(
                        c.x + mj.x * cos(t) + mn.x * sin(t),
                        c.y + mj.y * cos(t) + mn.y * sin(t),
                    )
                }),
                t0: *t0,
                t1,
            })
        }
        _ => None,
    }
}

fn check_piece(name: &str, got: &Shape, want: &Value) {
    let kind = want["kind"].as_str().unwrap();
    match kind {
        "line" => {
            let Shape::Line { a, b } = got else {
                panic!("{name}: çizgi değil: {got:?}")
            };
            near(name, "baş", *a, pt(&want["ends"][0]), TOL);
            near(name, "son", *b, pt(&want["ends"][1]), TOL);
        }
        "polygon" => {
            let Shape::Polygon { pts, .. } = got else {
                panic!("{name}: alan değil: {got:?}")
            };
            let corners = want["corners"].as_array().unwrap();
            assert_eq!(pts.len(), corners.len(), "{name}");
            for (p, q) in pts.iter().zip(corners) {
                near(name, "köşe", *p, pt(q), TOL);
            }
        }
        "circle" | "ellipse" | "arc" => {
            if kind == "circle" {
                assert!(matches!(got, Shape::Circle { .. }), "{name}: daire değil: {got:?}");
            } else if kind == "ellipse" {
                assert!(matches!(got, Shape::Ellipse { .. }), "{name}: elips değil: {got:?}");
            }
            let c = curve(got).unwrap_or_else(|| panic!("{name}: eğri değil: {got:?}"));
            near(name, "merkez", c.c, pt(&want["center"]), TOL);
            num(name, "büyük yarı eksen", c.semi[0], want["semi"][0].as_f64().unwrap());
            num(name, "küçük yarı eksen", c.semi[1], want["semi"][1].as_f64().unwrap());
            if (c.semi[0] - c.semi[1]).abs() > 1e-9 {
                angle(name, "eksen", c.axis, want["axis"].as_f64().unwrap(), std::f64::consts::PI);
            }
            if kind == "arc" {
                near(name, "yayın başı", (c.at)(c.t0), pt(&want["start"]), 1e-9);
                near(name, "yayın sonu", (c.at)(c.t1), pt(&want["end"]), 1e-9);
                near(name, "yayın ortası", (c.at)((c.t0 + c.t1) / 2.0), pt(&want["mid"]), 1e-9);
            }
        }
        "text" => {
            let Shape::Text {
                p,
                height,
                rotation,
                width_factor,
                ..
            } = got
            else {
                panic!("{name}: yazı değil: {got:?}")
            };
            near(name, "yazının yeri", *p, pt(&want["p"]), TOL);
            num(name, "yükseklik", *height, want["height"].as_f64().unwrap());
            num(
                name,
                "genişlik çarpanı",
                width_factor.unwrap_or(1.0),
                want["widthFactor"].as_f64().unwrap(),
            );
            angle(name, "dönüş", *rotation, want["rotation"].as_f64().unwrap(), 360.0);
        }
        "polyline" => {
            let Shape::Polyline { pts, .. } = got else {
                panic!("{name}: çoklu çizgi değil: {got:?}")
            };
            near(name, "ilk köşe", pts[0], pt(&want["first"]), TOL);
            near(name, "son köşe", *pts.last().unwrap(), pt(&want["last"]), TOL);
            let joint = pt(&want["joint"]);
            let k = pts
                .iter()
                .position(|p| (p.x - joint.x).abs() <= TOL && (p.y - joint.y).abs() <= TOL)
                .unwrap_or_else(|| panic!("{name}: yay ile düz kenarın birleştiği köşe yok: {pts:?}"));
            assert!(k > 1, "{name}: yay kenarı köşelere bölünmedi");
            // Every vertex of the arc edge on its exact image: c + u cos t + v sin t.
            let arc = &want["arc"];
            let (c, u, v) = (pt(&arc["center"]), pt(&arc["u"]), pt(&arc["v"]));
            let d = u.x * v.y - u.y * v.x;
            for q in &pts[..=k] {
                let (x, y) = (q.x - c.x, q.y - c.y);
                let (cs, sn) = ((x * v.y - y * v.x) / d, (u.x * y - u.y * x) / d);
                let off = (js_hypot(cs, sn) - 1.0).abs() * js_max(js_hypot(u.x, u.y), js_hypot(v.x, v.y));
                assert!(off <= 1e-4, "{name}: köşe {q:?} görüntüden {off} m uzak");
            }
        }
        other => panic!("{name}: bilinmeyen tür {other}"),
    }
}

#[test]
fn an_insert_s_own_y_factor_places_its_pieces_as_the_reference_says() {
    let doc: Value = serde_json::from_str(CASES).expect("the cases");
    let blocks = Blocks::from_json(&parse(&doc["definitions"])).expect("the definitions");
    for c in doc["cases"].as_array().expect("cases") {
        let name = c["name"].as_str().unwrap();
        let insert = Entity::from_json(&parse(&c["insert"])).expect("an insert");
        let got: Vec<Shape> = blocks
            .expand(&insert.shape)
            .into_iter()
            .map(|p| p.shape)
            .filter(|s| !shows_nothing(s))
            .collect();
        let want = c["pieces"].as_array().unwrap();
        assert_eq!(got.len(), want.len(), "{name}: {got:?}");
        for (g, w) in got.iter().zip(want) {
            check_piece(name, g, w);
        }
    }
    for c in doc["explodes"].as_array().expect("explodes") {
        let name = c["name"].as_str().unwrap();
        let insert = Entity::from_json(&parse(&c["insert"])).expect("an insert");
        let want = &c["expect"];
        match blocks.explode(&insert) {
            Cut::Error(m) => assert!(
                want["refused"].as_bool() == Some(true),
                "{name}: reddedildi: {m}"
            ),
            Cut::Pieces(pieces) => {
                assert!(want["refused"].is_null(), "{name}: reddedilmeliydi");
                let want = want["pieces"].as_array().unwrap();
                assert_eq!(pieces.len(), want.len(), "{name}");
                for (p, w) in pieces.iter().zip(want) {
                    if w["kind"] == "insert" {
                        let Shape::Insert {
                            p: at,
                            scale,
                            y_scale,
                            rotation,
                            mirror,
                            ..
                        } = &p.shape
                        else {
                            panic!("{name}: yerleştirme değil: {:?}", p.shape)
                        };
                        near(name, "yerleştirme noktası", *at, pt(&w["p"]), TOL);
                        num(name, "x ölçeği", *scale, w["scale"].as_f64().unwrap());
                        match w["yScale"].as_f64() {
                            Some(y) => num(name, "y ölçeği", y_scale.expect("a y factor"), y),
                            None => assert!(y_scale.is_none(), "{name}: {y_scale:?}"),
                        }
                        angle(
                            name,
                            "dönüş",
                            *rotation,
                            w["rotation"].as_f64().unwrap(),
                            std::f64::consts::TAU,
                        );
                        assert_eq!(
                            mirror.unwrap_or(false),
                            w["mirror"].as_bool().unwrap(),
                            "{name}"
                        );
                    } else {
                        check_piece(name, &p.shape, w);
                    }
                }
            }
        }
    }
}

/// How long 10 000 inserts of the room (a polygon, a line, an ellipse, a text) take to place, uniform and with an
/// own y factor (docs/adr/0315 §3): the affine's warp against the similarity's move.
#[test]
#[ignore]
fn timing() {
    let doc: Value = serde_json::from_str(CASES).expect("the cases");
    let blocks = Blocks::from_json(&parse(&doc["definitions"])).expect("the definitions");
    let shape = |sy: Option<f64>, i: usize| Shape::Insert {
        block: "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4103".into(),
        p: Vec2::new((i % 100) as f64 * 10.0, (i / 100) as f64 * 10.0),
        scale: 1.5,
        y_scale: sy,
        rotation: 0.3,
        mirror: None,
        attrs: None,
    };
    for (what, sy) in [("eşit ölçek", None), ("x 1,5 y 0,5", Some(0.5))] {
        let t = std::time::Instant::now();
        let mut n = 0;
        for i in 0..10_000 {
            n += blocks.expand(&shape(sy, i)).len();
        }
        println!("{what}: 10 000 yerleştirme, {n} parça: {:.1} ms", t.elapsed().as_secs_f64() * 1e3);
    }
}
