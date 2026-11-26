//! Genel bakış and Büyüteç (docs/adr/0181): the cards' places, the overview's
//! fit and its picture, against `fixtures/navigation/v1/cases.json`
//! (written by scripts/fixtures/navigation_cases.py from the ADR).

use std::collections::HashMap;

use kentos_geometry_core::geometry::Bounds;
use kentos_geometry_core::store::Store;
use kentos_geometry_core::store::overview::{OverviewLayer, OverviewRequest, overview_picture_of};
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

/// docs/adr/9327: the overview of several stores over the extent the host gives: a data layer's outline (rows the drawing
/// does not hold) drawn first, the drawing's objects over it; one store over its own extent is the store's own picture.
#[test]
fn the_overview_draws_the_data_layers_outlines_under_the_drawing() {
    let mut drawing = Store::new();
    drawing
        .put_json(
            &json!([{ "id": 1, "layerId": "a", "kind": "line", "attrs": {}, "a": { "x": 0, "y": 0 }, "b": { "x": 10, "y": 0 } }])
                .to_string(),
        )
        .expect("entities");
    let mut data = Store::new();
    data.put_json(
        &json!([
            { "id": 1, "layerId": "veri", "kind": "point", "attrs": {}, "p": { "x": 100, "y": 50 } },
            { "id": 2, "layerId": "veri", "kind": "line", "attrs": {}, "a": { "x": 0, "y": 0 }, "b": { "x": 10, "y": 0 } }
        ])
        .to_string(),
    )
    .expect("rows");
    let colors: HashMap<String, [u8; 3]> =
        [("a".to_owned(), [200, 0, 0]), ("veri".to_owned(), [0, 0, 200])].into_iter().collect();
    let request = OverviewRequest { width: 240.0, height: 160.0, dpr: 1.0, colors: &colors };
    // The drawing alone, over its own extent: the store's own picture, bit for bit.
    let own = drawing.overview_picture(&request).expect("a picture");
    let extent = drawing.overview_extent().expect("an extent");
    assert_eq!(overview_picture_of(&[OverviewLayer { store: &drawing, dot: 1.0 }], &extent, &request), own);
    // The data layer's box with the drawing's: the point a dot of three pixels about its place, the line the drawing's
    // colour over the outline's.
    let both = Bounds { min_x: 0.0, min_y: 0.0, max_x: 100.0, max_y: 50.0 };
    let picture = overview_picture_of(
        &[OverviewLayer { store: &data, dot: 3.0 }, OverviewLayer { store: &drawing, dot: 1.0 }],
        &both,
        &request,
    );
    let (u, v) = picture.fit.to_card((240.0, 160.0), 100.0, 50.0);
    let pixel = |i: i64, j: i64| {
        let at = (j as usize * picture.width + i as usize) * 4;
        picture.pixels[at..at + 4].to_vec()
    };
    let (i, j) = (u.floor() as i64, v.floor() as i64);
    for (di, dj) in [(-1, -1), (0, 0), (1, 1), (1, -1)] {
        assert_eq!(pixel(i + di, j + dj)[..3], [0, 0, 200], "the data point's dot at {di} {dj}");
    }
    assert_eq!(pixel(i + 2, j)[3], 0, "three pixels across");
    let (u, v) = picture.fit.to_card((240.0, 160.0), 5.0, 0.0);
    assert_eq!(pixel(u.floor() as i64, v.floor() as i64)[..3], [200, 0, 0], "the drawing over the outline");
    assert_eq!(picture.fit, fit(&both, (240.0, 160.0)));
}
