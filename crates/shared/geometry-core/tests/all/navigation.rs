//! Genel bakış and Büyüteç (docs/adr/0181): the cards' places, the overview's
//! fit and its picture, against `fixtures/navigation/v1/cases.json`
//! (written by scripts/fixtures/navigation_cases.py from the ADR).

use std::collections::HashMap;

use kentos_geometry_core::geometry::Bounds;
use kentos_geometry_core::store::Store;
use kentos_geometry_core::store::overview::OverviewRequest;
use kentos_geometry_core::tools::navigation::{Card, Side, cards, fit, next_side};
use serde_json::{Value, json};

fn cases() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../fixtures/navigation/v1/cases.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).expect("the cases")).expect("JSON")
}

fn num(v: &Value) -> f64 {
    v.as_f64().expect("a number")
}

fn nums(v: &Value) -> Vec<f64> {
    v.as_array().expect("a list").iter().map(num).collect()
}

/// Exactly: the reference takes the same IEEE 754 steps in the same order.
fn same(got: &[f64], want: &[f64], what: &str) {
    assert_eq!(got, want, "{what}");
}

fn card_is(got: &Card, want: &Value, what: &str) {
    same(&got.card, &nums(&want["card"]), what);
    same(&got.content, &nums(&want["content"]), what);
}

#[test]
fn the_cards_stand_where_the_reference_puts_them() {
    for c in cases()["cards"].as_array().expect("cards") {
        let name = c["name"].as_str().unwrap_or_default();
        let area = nums(&c["area"]);
        let laid = cards(
            (area[0], area[1]),
            num(&c["header"]),
            c["cbs"].as_bool().unwrap_or_default(),
            c["overview"].as_bool().unwrap_or_default(),
        );
        let want = &c["layout"];
        match (&laid.overview, want["overview"].is_null()) {
            (None, true) => {}
            (Some(o), false) => card_is(o, &want["overview"], name),
            _ => panic!("{name}: the overview's card"),
        }
        card_is(&laid.right, &want["right"], name);
        card_is(&laid.left, &want["left"], name);
        let side = if c["side"] == "left" {
            Side::Left
        } else {
            Side::Right
        };
        let pointer = nums(&c["pointer"]);
        let next = next_side(&laid, side, (pointer[0], pointer[1]));
        assert_eq!(
            next.word(),
            c["next"].as_str().unwrap_or_default(),
            "{name}"
        );
    }
}

#[test]
fn the_overview_fits_frames_and_answers_presses_as_the_reference() {
    for c in cases()["fits"].as_array().expect("fits") {
        let name = c["name"].as_str().unwrap_or_default();
        let e = nums(&c["extent"]);
        let size = nums(&c["size"]);
        let size = (size[0], size[1]);
        let f = fit(
            &Bounds {
                min_x: e[0],
                min_y: e[1],
                max_x: e[2],
                max_y: e[3],
            },
            size,
        );
        same(&[f.cx, f.cy, f.k], &nums(&c["fit"]), name);
        let center = nums(&c["center"]);
        let view = nums(&c["viewPx"]);
        let (frame, cross) = f.view_frame(
            size,
            (center[0], center[1]),
            num(&c["metresPerPixel"]),
            (view[0], view[1]),
        );
        same(&frame, &nums(&c["frame"]), name);
        assert_eq!(cross, c["cross"].as_bool().unwrap_or_default(), "{name}");
        for p in c["presses"].as_array().expect("presses") {
            let at = nums(&p["at"]);
            let (x, y) = f.to_world(size, at[0], at[1]);
            same(&[x, y], &nums(&p["world"]), name);
        }
    }
}

/// A case's objects as the document's entities.
fn entity(i: usize, o: &Value) -> Value {
    let s = &o["shape"];
    let p = |v: &Value| json!({ "x": num(&v[0]), "y": num(&v[1]) });
    let mut e = json!({ "id": i + 1, "layerId": o["layer"], "kind": s["kind"], "attrs": {} });
    let pts = |v: &Value| Value::Array(v.as_array().expect("points").iter().map(p).collect());
    match s["kind"].as_str().unwrap_or_default() {
        "point" => e["p"] = p(&s["p"]),
        "line" => {
            e["a"] = p(&s["a"]);
            e["b"] = p(&s["b"]);
        }
        "polyline" => e["pts"] = pts(&s["points"]),
        "polygon" => {
            let rings = s["rings"].as_array().expect("rings");
            e["pts"] = pts(&rings[0]);
            if rings.len() > 1 {
                e["holes"] = Value::Array(
                    rings[1..]
                        .iter()
                        .map(|r| json!({ "pts": pts(r) }))
                        .collect(),
                );
            }
        }
        "circle" => {
            e["c"] = p(&s["c"]);
            e["r"] = s["r"].clone();
        }
        "arc" => {
            e["c"] = p(&s["c"]);
            e["r"] = s["r"].clone();
            e["a0"] = s["a0"].clone();
            e["a1"] = s["a1"].clone();
        }
        other => panic!("a case's kind: {other}"),
    }
    e
}

#[test]
fn the_overview_picture_is_the_references_pixel_for_pixel() {
    const COLORS: [[u8; 3]; 3] = [[229, 72, 77], [95, 191, 119], [77, 150, 255]];
    for c in cases()["pictures"].as_array().expect("pictures") {
        let name = c["name"].as_str().unwrap_or_default();
        let layers: Vec<&str> = c["layers"]
            .as_array()
            .expect("layers")
            .iter()
            .map(|l| l.as_str().unwrap_or_default())
            .collect();
        let entities: Vec<Value> = c["objects"]
            .as_array()
            .expect("objects")
            .iter()
            .enumerate()
            .map(|(i, o)| entity(i, o))
            .collect();
        let mut store = Store::new();
        store
            .put_json(&Value::Array(entities).to_string())
            .expect("entities");
        let colors: HashMap<String, [u8; 3]> = layers
            .iter()
            .zip(COLORS)
            .map(|(l, c)| ((*l).to_owned(), c))
            .collect();
        let size = nums(&c["size"]);
        let picture = store
            .overview_picture(&OverviewRequest {
                width: size[0],
                height: size[1],
                dpr: num(&c["dpr"]),
                colors: &colors,
            })
            .expect("a picture");
        same(
            &[picture.fit.cx, picture.fit.cy, picture.fit.k],
            &nums(&c["fit"]),
            name,
        );
        let extent = store.overview_extent().expect("an extent");
        same(
            &[extent.min_x, extent.min_y, extent.max_x, extent.max_y],
            &nums(&c["extent"]),
            name,
        );
        assert_eq!(picture.width, num(&c["width"]) as usize, "{name}");
        assert_eq!(picture.height, num(&c["height"]) as usize, "{name}");
        let rows: Vec<String> = (0..picture.height)
            .map(|j| {
                (0..picture.width)
                    .map(|i| {
                        let at = (j * picture.width + i) * 4;
                        let px = &picture.pixels[at..at + 4];
                        if px[3] == 0 {
                            return '.';
                        }
                        let layer = COLORS
                            .iter()
                            .position(|c| c[..] == px[..3])
                            .expect("a layer's colour");
                        let letter = b"ABCDEFGH"[layer] as char;
                        match px[3] {
                            255 => letter,
                            64 => letter.to_ascii_lowercase(),
                            other => panic!("{name}: opacity {other}"),
                        }
                    })
                    .collect()
            })
            .collect();
        let want: Vec<&str> = c["rows"]
            .as_array()
            .expect("rows")
            .iter()
            .map(|r| r.as_str().unwrap_or_default())
            .collect();
        assert_eq!(rows, want, "{name}");
    }
}

#[test]
fn hidden_layers_and_construction_lines_stay_out_of_the_overview() {
    let mut store = Store::new();
    store
        .put_json(
            &json!([
                { "id": 1, "layerId": "a", "kind": "line", "attrs": {}, "a": { "x": 0, "y": 0 }, "b": { "x": 10, "y": 0 } },
                { "id": 2, "layerId": "b", "kind": "line", "attrs": {}, "a": { "x": 100, "y": 0 }, "b": { "x": 200, "y": 0 } },
                { "id": 3, "layerId": "a", "kind": "xline", "attrs": {}, "p": { "x": 500, "y": 500 }, "dir": { "x": 1, "y": 0 } }
            ])
            .to_string(),
        )
        .expect("entities");
    store
        .set_layers_json(
            r#"[{"id":"a","visible":true,"locked":false,"pickInterior":true},{"id":"b","visible":false,"locked":false,"pickInterior":true}]"#,
        )
        .expect("layers");
    let e = store.overview_extent().expect("an extent");
    assert_eq!([e.min_x, e.min_y, e.max_x, e.max_y], [0.0, 0.0, 10.0, 0.0]);
    let empty = Store::new();
    assert!(empty.overview_extent().is_none());
    assert!(
        empty
            .overview_picture(&OverviewRequest {
                width: 240.0,
                height: 160.0,
                dpr: 1.0,
                colors: &HashMap::new(),
            })
            .is_none()
    );
}
