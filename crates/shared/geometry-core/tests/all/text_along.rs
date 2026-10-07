//! Eğri boyunca yazı (docs/adr/0196) against the independent reference in
//! `fixtures/text/v1/along.json` (`scripts/fixtures/text_along_cases.py`,
//! written from the ADR; no KentOS code): the letters, box, records and
//! direction of a text along its curve, Okunur yap, the transforms, the
//! piece of a curve a tool cuts, Düzleştir and Doğrultuya döndür; the
//! records and pieces through the op table as the web calls them
//! (`apps/web/src/tools/textAlong.wasm.test.ts`).
//!
//! The reference works in the world where the core works in the text's
//! frame, so the two meet within 1e−9 of the coordinates' size.

use kentos_geometry_core::api::json::{FromJson, Json};
use kentos_geometry_core::api::run_named;
use kentos_geometry_core::entity::{Entity, Shape, TextPlace};
use kentos_geometry_core::geom::affine::Affine;
use kentos_geometry_core::ops::transform::transform_shape;
use kentos_geometry_core::text::Font;
use kentos_geometry_core::text::along::{Curve, piece};
use kentos_geometry_core::vec2::Vec2;
use serde_json::{Value, json};

fn cases() -> Value {
    let path = format!(
        "{}/../../../fixtures/text/v1/along.json",
        env!("CARGO_MANIFEST_DIR")
    );
    serde_json::from_str(&std::fs::read_to_string(path).expect("the cases")).expect("JSON")
}

fn run(name: &str, args: Value) -> Value {
    serde_json::from_str(
        &run_named(name, &args.to_string()).unwrap_or_else(|e| panic!("{name}: {e}")),
    )
    .expect("JSON")
}

/// The text as an entity of the drawing.
fn shape(t: &Value) -> Shape {
    let mut v = t.clone();
    v["kind"] = json!("text");
    Entity::from_json(&Json::parse(&v.to_string()).expect("JSON"))
        .expect("a text")
        .shape
}

/// The typeface the case measures in: its own, else the drawing's.
fn font(t: &Value) -> Font {
    Font::from_id(
        t["font"]
            .as_str()
            .or(t["drawingFont"].as_str())
            .unwrap_or("barlow"),
    )
}

fn tolerance(scale: f64) -> f64 {
    1e-9 * kentos_geometry_core::jsmath::js_max(scale.abs(), 1.0)
}

fn near(got: f64, want: f64, what: &str) {
    assert!(
        (got - want).abs() <= tolerance(want),
        "{what}: {got:?} ≠ {want:?}"
    );
}

/// Angles in degrees, equal round the circle.
fn near_turn(got: f64, want: f64, what: &str) {
    let d = ((got - want) % 360.0 + 540.0) % 360.0 - 180.0;
    assert!(d.abs() <= 1e-9, "{what}: {got:?} ≠ {want:?}");
}

fn near_point(got: Vec2, want: &Value, what: &str) {
    near(got.x, want["x"].as_f64().expect(what), &format!("{what} x"));
    near(got.y, want["y"].as_f64().expect(what), &format!("{what} y"));
}

fn curve_of(v: &Value) -> Curve {
    Curve::from_json(&Json::parse(&v.to_string()).expect("JSON")).expect("a curve")
}

fn near_curve(got: &Curve, want: &Value, what: &str) {
    let want = curve_of(want);
    assert_eq!(got.pts.len(), want.pts.len(), "{what}: vertices");
    for (i, (g, w)) in got.pts.iter().zip(&want.pts).enumerate() {
        near(g.x, w.x, &format!("{what} vertex {i} x"));
        near(g.y, w.y, &format!("{what} vertex {i} y"));
    }
    assert_eq!(
        got.bulges.is_some(),
        want.bulges.is_some(),
        "{what}: bulges {:?} ≠ {:?}",
        got.bulges,
        want.bulges
    );
    for i in 0..got.pts.len() {
        near(got.bulge(i), want.bulge(i), &format!("{what} bulge {i}"));
    }
}

/// The text's own fields as `placed` gives them, against the reference's.
fn near_text(got: &Shape, want: &Value, what: &str) {
    let Shape::Text {
        p,
        rotation,
        align,
        path,
        height,
        ..
    } = got
    else {
        panic!("{what}: not a text");
    };
    near_point(*p, &want["p"], &format!("{what} p"));
    near_turn(
        *rotation,
        want["rotation"].as_f64().expect("rotation"),
        what,
    );
    assert_eq!(
        align.map(|a| a.name()),
        want["align"].as_str(),
        "{what}: alignment"
    );
    if let Some(h) = want["height"].as_f64() {
        near(*height, h, &format!("{what} height"));
    }
    near_curve(path.as_ref().expect("a curve"), &want["path"], what);
}

#[test]
fn the_letters_stand_and_turn_as_the_reference_says() {
    let data = cases();
    for c in data["layout"].as_array().expect("layout") {
        let name = c["name"].as_str().expect("a name");
        let (t, want) = (&c["text"], &c["want"]);
        let s = shape(t);
        let place = TextPlace::of(&s).expect("a text place");
        let along = place.along(font(t)).expect("along its curve");
        near(
            along.length(),
            want["pathLength"].as_f64().expect("length"),
            &format!("{name} length"),
        );
        let letters = along.letters();
        let wanted = want["letters"].as_array().expect("letters");
        assert_eq!(letters.len(), wanted.len(), "{name}: letters");
        let total: f64 = letters.iter().map(|l| l.advance).sum();
        near(
            total,
            want["textLength"].as_f64().expect("text length"),
            &format!("{name} text length"),
        );
        for (i, (l, w)) in letters.iter().zip(wanted).enumerate() {
            near_point(l.at, &w["at"], &format!("{name} letter {i}"));
            near_turn(
                l.turn,
                w["turn"].as_f64().expect("turn"),
                &format!("{name} letter {i} turn"),
            );
            near(
                l.advance,
                w["advance"].as_f64().expect("advance"),
                &format!("{name} letter {i} advance"),
            );
        }
        near_turn(
            along.direction(&letters),
            want["direction"].as_f64().expect("direction"),
            &format!("{name} direction"),
        );
        for (key, margin) in [
            ("outline", 0.0),
            ("grown", t["height"].as_f64().expect("h") * 0.25),
        ] {
            let ring = place.outline_grown(font(t), margin);
            let wanted = want[key].as_array().expect(key);
            assert_eq!(ring.len(), wanted.len(), "{name}: {key}");
            for (i, (g, w)) in ring.iter().zip(wanted).enumerate() {
                near_point(*g, w, &format!("{name} {key} {i}"));
            }
        }
        // The records, as the web asks for them (`textLines`, the drawing's typeface as `font`).
        let mut ask = t.clone();
        ask["font"] = json!(
            t["font"]
                .as_str()
                .or(t["drawingFont"].as_str())
                .unwrap_or("barlow")
        );
        let got = run("textLines", json!([ask]));
        let got = got.as_array().expect("records");
        let wanted: Vec<f64> = want["records"]
            .as_array()
            .expect("records")
            .iter()
            .flat_map(|r| {
                r.as_array()
                    .expect("a record")
                    .iter()
                    .map(|x| x.as_f64().expect("a number"))
            })
            .collect();
        assert_eq!(got.len(), wanted.len(), "{name}: records");
        for (i, (g, w)) in got.iter().zip(&wanted).enumerate() {
            let g = g.as_f64().expect("a number");
            if i % 9 == 4 {
                near_turn(g, *w, &format!("{name} record number {i}"));
            } else {
                near(g, *w, &format!("{name} record number {i}"));
            }
        }
        // Its box is its letters' (the store picks and selects by it).
        let ring = run("textBox", json!([ask]));
        assert_eq!(
            ring.as_array().expect("a ring").len(),
            want["outline"].as_array().expect("outline").len(),
            "{name}: textBox"
        );
    }
}

#[test]
fn okunur_yap_turns_the_curve_the_other_way() {
    let data = cases();
    for c in data["readable"].as_array().expect("readable") {
        let name = c["name"].as_str().expect("a name");
        let t = &c["text"];
        let s = shape(t);
        let along = TextPlace::of(&s)
            .and_then(|p| p.along(font(t)))
            .expect("along its curve");
        match (along.readable(), c["want"].is_null()) {
            (None, true) => {}
            (Some(placed), false) => {
                let mut turned = s.clone();
                if let Shape::Text {
                    p,
                    rotation,
                    align,
                    path,
                    ..
                } = &mut turned
                {
                    *p = placed.p;
                    *rotation = placed.rotation;
                    *align = placed.align;
                    *path = Some(placed.curve);
                }
                near_text(&turned, &c["want"], name);
            }
            (got, _) => panic!("{name}: {got:?} ≠ {}", c["want"]),
        }
        // The web's call says the same.
        let mut ask = t.clone();
        ask["font"] = json!(
            t["font"]
                .as_str()
                .or(t["drawingFont"].as_str())
                .unwrap_or("barlow")
        );
        let got = run("textAlongReadable", json!([ask]));
        assert_eq!(
            got.is_null(),
            c["want"].is_null(),
            "{name}: textAlongReadable"
        );
    }
}

#[test]
fn a_transform_carries_the_curve() {
    let data = cases();
    for c in data["transform"].as_array().expect("transform") {
        let name = c["name"].as_str().expect("a name");
        let m: Vec<f64> = c["affine"]
            .as_array()
            .expect("an affine")
            .iter()
            .map(|x| x.as_f64().expect("a number"))
            .collect();
        let m: Affine = [m[0], m[1], m[2], m[3], m[4], m[5]];
        let got = transform_shape(&shape(&c["text"]), &m);
        near_text(&got, &c["want"], name);
    }
}

#[test]
fn a_curve_gives_the_piece_the_reference_says() {
    let data = cases();
    for c in data["piece"].as_array().expect("piece") {
        let name = c["name"].as_str().expect("a name");
        let curve = Entity::from_json(&Json::parse(&c["curve"].to_string()).expect("JSON"))
            .expect("a curve")
            .shape;
        let click = Vec2::new(
            c["click"]["x"].as_f64().expect("x"),
            c["click"]["y"].as_f64().expect("y"),
        );
        let (length, share) = (
            c["length"].as_f64().expect("length"),
            c["share"].as_f64().expect("share"),
        );
        let (p, rotation, path) = piece(&curve, click, length, share).expect("a piece");
        let want = &c["want"];
        near_point(p, &want["p"], &format!("{name} p"));
        near_turn(rotation, want["rotation"].as_f64().expect("rotation"), name);
        near_curve(&path, &want["path"], name);
        // The web's call gives the same.
        let got = run(
            "textAlongPiece",
            json!([c["curve"], c["click"], length, share]),
        );
        near(
            got["rotation"].as_f64().expect("rotation"),
            rotation,
            &format!("{name} textAlongPiece"),
        );
    }
}

#[test]
fn duzlestir_and_dogrultuya_dondur() {
    let data = cases();
    for c in data["straight"].as_array().expect("straight") {
        let name = c["name"].as_str().expect("a name");
        let t = &c["text"];
        let s = shape(t);
        let (p, rotation) = TextPlace::of(&s)
            .and_then(|p| p.along(font(t)))
            .expect("along its curve")
            .straight();
        near_point(p, &c["want"]["p"], name);
        near_turn(
            rotation,
            c["want"]["rotation"].as_f64().expect("rotation"),
            name,
        );
    }
    for c in data["turn"].as_array().expect("turn") {
        let name = c["name"].as_str().expect("a name");
        let got = run("textAlongTurn", json!([c["direction"]]));
        near_turn(
            got.as_f64().expect("a turn"),
            c["want"].as_f64().expect("a turn"),
            name,
        );
    }
}

#[test]
fn a_curved_text_survives_the_packed_store() {
    let data = cases();
    let t = &data["layout"][3]["text"];
    let mut store = kentos_geometry_core::store::Store::new();
    let mut v = t.clone();
    v["kind"] = json!("text");
    v["id"] = json!(1);
    v["layerId"] = json!("yazi");
    store.put_json(&json!([v]).to_string()).expect("the text");
    // Moved 5 m east, packed and read back: the curve rides in its frame, unchanged.
    let packed = store.transform_packed(&[1.0], &[[1.0, 0.0, 0.0, 1.0, 5.0, 0.0]]);
    let mut back = kentos_geometry_core::store::Store::new();
    back.put_packed(&packed.nums, &packed.strings)
        .expect("the pack");
    let Some(Shape::Text { p, path, .. }) = back.get(1.0).map(|it| it.shape.clone()) else {
        panic!("the text came back");
    };
    assert_eq!(p, Vec2::new(5.0, 0.0));
    let Shape::Text { path: was, .. } = shape(t) else {
        unreachable!()
    };
    assert_eq!(path, was);
}
