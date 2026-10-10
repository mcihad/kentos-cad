//! The label engine (docs/adr/0212) through the store: each case's objects,
//! layers, texts, pins and drawing's texts placed in its window at its scale
//! must give the labels `fixtures/labels/v1/cases.json` holds (written by
//! scripts/fixtures/label_engine_cases.py from the ADR, not from this code),
//! in the same order, and a click must pick the same label.

use kentos_geometry_core::geometry::Bounds;
use kentos_geometry_core::store::Store;
use kentos_geometry_core::store::labels::LABEL_STRIDE;
use kentos_geometry_core::store::placing::{
    LABEL_PLACED, LABEL_PLACED_CALLOUT, LABEL_PLACED_LETTER, LABEL_PLACED_LINE, ObjectLabels,
    PlaceOptions, Shown, label_at,
};
use kentos_geometry_core::vec2::Vec2;
use serde_json::Value;

fn cases() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../fixtures/labels/v1/cases.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).expect("the cases")).expect("JSON")
}

fn num(v: &Value) -> f64 {
    v.as_f64().unwrap_or_else(|| panic!("a number: {v}"))
}

/// The reference's values are rounded to 1e-9; its sines and roots are the platform's, ours JavaScript's.
fn close(got: f64, want: f64, what: &str) {
    assert!((got - want).abs() <= 1e-6, "{what}: {got} ≠ {want}");
}

/// One placed label as the cases write it, from the records.
#[derive(Debug, Default)]
struct Label {
    id: f64,
    class: f64,
    state: f64,
    at: [f64; 2],
    angle: f64,
    w: f64,
    h: f64,
    lines: Vec<(f64, f64, f64, f64, String, f64)>,
    text: Option<String>,
    letters: Vec<[f64; 6]>,
    callout: Option<[f64; 4]>,
}

fn labels(shown: &Shown) -> Vec<Label> {
    let mut out: Vec<Label> = Vec::new();
    for r in shown.records.chunks_exact(LABEL_STRIDE) {
        if r[1] == LABEL_PLACED {
            out.push(Label {
                id: r[0],
                at: [r[2], r[3]],
                angle: r[4],
                w: r[5],
                h: r[6],
                class: r[7],
                state: r[8],
                ..Label::default()
            });
            continue;
        }
        let l = out.last_mut().expect("a frame before its parts");
        if r[1] == LABEL_PLACED_LINE {
            l.lines.push((
                r[2],
                r[3],
                r[4],
                r[5],
                shown.texts[r[6] as usize].clone(),
                r[7],
            ));
        } else if r[1] == LABEL_PLACED_LETTER {
            l.text = Some(shown.texts[r[6] as usize].clone());
            l.letters.push([r[2], r[3], r[4], r[5], r[7], r[8]]);
        } else if r[1] == LABEL_PLACED_CALLOUT {
            l.callout = Some([r[2], r[3], r[4], r[5]]);
        }
    }
    out
}

fn place(case: &Value) -> Shown {
    let mut s = Store::new();
    s.put_json(&case["objects"].to_string())
        .expect("the objects");
    s.set_label_layers_json(&case["layers"].to_string())
        .expect("the layers");
    if let Some(d) = case.get("defaults") {
        s.set_label_defaults_json(&d.to_string())
            .expect("the defaults");
    }
    for t in case["texts"].as_array().expect("texts") {
        let texts = t["texts"]
            .as_array()
            .expect("an object's texts")
            .iter()
            .map(|e| (num(&e[0]) as u16, e[1].as_str().expect("a text").to_owned()))
            .collect();
        let z = t.get("z").map_or(f64::NAN, num);
        s.set_object_labels(num(&t["id"]), ObjectLabels { texts, z });
    }
    if let Some(p) = case.get("pins") {
        s.set_label_pins_json(&p.to_string()).expect("the pins");
    }
    let fixed: Vec<Vec<Vec2>> = case
        .get("fixed")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .map(|o| {
                    o.as_array()
                        .expect("an outline")
                        .iter()
                        .map(|p| Vec2::new(num(&p[0]), num(&p[1])))
                        .collect()
                })
                .collect()
        })
        .unwrap_or_default();
    let w: Vec<f64> = case["window"]
        .as_array()
        .expect("a window")
        .iter()
        .map(num)
        .collect();
    let options = PlaceOptions {
        unplaced: case["options"]["unplaced"].as_bool().unwrap_or(false),
        hidden: case["options"]["hidden"].as_bool().unwrap_or(false),
    };
    let window = Bounds {
        min_x: w[0],
        min_y: w[1],
        max_x: w[2],
        max_y: w[3],
    };
    s.place_labels(&window, num(&case["scale"]), &fixed, options)
}

#[test]
fn the_store_places_the_labels_the_reference_places() {
    let all = cases();
    let list = all["cases"].as_array().expect("cases");
    assert!(list.len() >= 40, "the cases are there");
    for case in list {
        let name = case["name"].as_str().expect("a name");
        let shown = place(case);
        let got = labels(&shown);
        let want = case["expect"].as_array().expect("the expected labels");
        let say = |l: &Label| format!("{} {} st {} at {:?}", l.id, l.class, l.state, l.at);
        assert_eq!(
            got.len(),
            want.len(),
            "{name}: {} labels, the reference has {}: {:?}",
            got.len(),
            want.len(),
            got.iter().map(say).collect::<Vec<_>>()
        );
        for (k, (g, e)) in got.iter().zip(want).enumerate() {
            let what = format!("{name}, label {k}");
            assert_eq!(
                (g.id, g.class, g.state),
                (num(&e["id"]), num(&e["class"]), num(&e["state"])),
                "{what}: {}",
                say(g)
            );
            close(g.at[0], num(&e["at"][0]), &format!("{what} x"));
            close(g.at[1], num(&e["at"][1]), &format!("{what} y"));
            close(g.angle, num(&e["angle"]), &format!("{what} angle"));
            close(g.w, num(&e["w"]), &format!("{what} width"));
            close(g.h, num(&e["h"]), &format!("{what} height"));
            let lines = e
                .get("lines")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            assert_eq!(g.lines.len(), lines.len(), "{what}: lines");
            for (gl, el) in g.lines.iter().zip(&lines) {
                close(gl.0, num(&el[0]), &format!("{what} line x"));
                close(gl.1, num(&el[1]), &format!("{what} line y"));
                close(gl.2, num(&el[2]), &format!("{what} line angle"));
                close(gl.3, num(&el[3]), &format!("{what} line size"));
                assert_eq!(
                    gl.4,
                    el[4].as_str().expect("a line's text"),
                    "{what}: line text"
                );
                close(gl.5, num(&el[5]), &format!("{what} line width"));
            }
            assert_eq!(
                g.text.as_deref(),
                e.get("text").and_then(Value::as_str),
                "{what}: curved text"
            );
            let letters = e
                .get("letters")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            assert_eq!(g.letters.len(), letters.len(), "{what}: letters");
            for (i, (gl, el)) in g.letters.iter().zip(&letters).enumerate() {
                for (j, v) in gl.iter().enumerate() {
                    close(*v, num(&el[j]), &format!("{what} letter {i} [{j}]"));
                }
            }
            match (g.callout, e.get("callout")) {
                (None, None) => {}
                (Some(c), Some(ec)) => {
                    let ec = [
                        num(&ec[0][0]),
                        num(&ec[0][1]),
                        num(&ec[1][0]),
                        num(&ec[1][1]),
                    ];
                    for j in 0..4 {
                        close(c[j], ec[j], &format!("{what} callout [{j}]"));
                    }
                }
                (g, e) => panic!("{what}: callout {g:?} ≠ {e:?}"),
            }
        }
        for h in case
            .get("hits")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let at = Vec2::new(num(&h["at"][0]), num(&h["at"][1]));
            let all = h.get("all").and_then(Value::as_bool).unwrap_or(false);
            let hit = label_at(
                &shown.records,
                LABEL_STRIDE,
                num(&case["scale"]),
                at,
                num(&h["tol"]),
                all,
            );
            let want = &h["hit"];
            match hit {
                None => assert!(
                    want.is_null(),
                    "{name}: nothing under {at:?}, the reference picks {want}"
                ),
                Some(hit) => assert_eq!(
                    (hit.id, f64::from(hit.class), f64::from(hit.state)),
                    (num(&want["id"]), num(&want["class"]), num(&want["state"])),
                    "{name}: the label under {at:?}"
                ),
            }
        }
    }
}

/// ADR 0212 §6's budgets, the engine alone (`place_labels`; the store holds the objects, the
/// layers' labelling and the texts): 2 000 survey points named around them, 10 000 parcels
/// numbered inside them, 500 contours round 25 hills read uphill every 400 px and an overview of 100 000
/// parcels too small to label. The scenes are `label_scenes` below; the web's
/// `apps/web/scripts/perf/labels.test.ts` builds the same. Release, by hand:
/// `cargo test --release -p kentos-geometry-core --test all label_engine::timing -- --ignored --nocapture`.
#[test]
#[ignore = "timings, run by hand in release"]
fn timing() {
    use std::time::Instant;
    println!();
    for scene in label_scenes() {
        let mut s = Store::new();
        s.put_json(&scene.objects).expect("the objects");
        s.set_label_layers_json(&scene.layers).expect("the layers");
        for (id, text, z) in &scene.texts {
            s.set_object_labels(
                *id,
                ObjectLabels {
                    texts: vec![(0, text.clone())],
                    z: *z,
                },
            );
        }
        let window = Bounds {
            min_x: scene.window[0],
            min_y: scene.window[1],
            max_x: scene.window[2],
            max_y: scene.window[3],
        };
        let mut placed = 0;
        let mut runs: Vec<f64> = (0..7)
            .map(|_| {
                let t = Instant::now();
                let shown = s.place_labels(&window, scene.scale, &[], PlaceOptions::default());
                let ms = t.elapsed().as_secs_f64() * 1000.0;
                placed = shown
                    .records
                    .chunks_exact(LABEL_STRIDE)
                    .filter(|r| r[1] == LABEL_PLACED)
                    .count();
                ms
            })
            .collect();
        runs.sort_by(f64::total_cmp);
        let (p50, best) = (runs[runs.len() / 2], runs[0]);
        println!(
            "{:<42} {placed:>6} etiket  en iyi {best:>8.3} ms  p50 {p50:>8.3} ms   bütçe {:>4} ms  {}",
            scene.name,
            scene.budget,
            if p50 <= scene.budget { "✓" } else { "✗" }
        );
    }
}

/// A timing scene: its objects and layers as the store reads them, each object's one text (and
/// height), the window (metres) and scale (px/m), and its budget (ms).
struct LabelScene {
    name: &'static str,
    objects: String,
    layers: String,
    texts: Vec<(f64, String, f64)>,
    window: [f64; 4],
    scale: f64,
    budget: f64,
}

/// The timing scenes, the web's (`labels.test.ts`) built the same way.
fn label_scenes() -> Vec<LabelScene> {
    use kentos_geometry_core::jsmath;
    use std::fmt::Write as _;
    let (x0, y0) = (487_000.0, 4_420_000.0);
    let mut out = Vec::new();

    // 2 000 survey points 5 m apart, a little off the grid, named around them at 4 px/m.
    let mut objects = String::from("[");
    let mut texts = Vec::new();
    for i in 0..2000 {
        let x = x0 + (i % 50) as f64 * 5.0 + ((i * 7) % 3) as f64 * 0.7;
        let y = y0 + (i / 50) as f64 * 5.0 + ((i * 11) % 5) as f64 * 0.4;
        let _ = write!(
            objects,
            r#"{}{{"id":{},"layerId":"nokta","kind":"point","p":{{"x":{x},"y":{y}}}}}"#,
            if i > 0 { "," } else { "" },
            i + 1
        );
        texts.push(((i + 1) as f64, format!("P.{}", 1000 + i), f64::NAN));
    }
    objects.push(']');
    out.push(LabelScene {
        name: "2 000 nokta adı (yoğun, 20 px)",
        objects,
        layers: r#"[{"id":"nokta","rank":0,"point":6,"label":{"placement":"beside","size":10,"point":"around"}}]"#.into(),
        texts,
        window: [x0 - 10.0, y0 - 10.0, x0 + 260.0, y0 + 210.0],
        scale: 4.0,
        budget: 8.0,
    });

    // The same 2 000 points 10 m apart (40 px): a survey's names at a zoom they are read at.
    let mut objects = String::from("[");
    let mut texts = Vec::new();
    for i in 0..2000 {
        let x = x0 + (i % 50) as f64 * 10.0 + ((i * 7) % 3) as f64 * 1.4;
        let y = y0 + (i / 50) as f64 * 10.0 + ((i * 11) % 5) as f64 * 0.8;
        let _ = write!(
            objects,
            r#"{}{{"id":{},"layerId":"nokta","kind":"point","p":{{"x":{x},"y":{y}}}}}"#,
            if i > 0 { "," } else { "" },
            i + 1
        );
        texts.push(((i + 1) as f64, format!("P.{}", 1000 + i), f64::NAN));
    }
    objects.push(']');
    out.push(LabelScene {
        name: "2 000 nokta adı (40 px)",
        objects,
        layers: r#"[{"id":"nokta","rank":0,"point":6,"label":{"placement":"beside","size":10,"point":"around"}}]"#.into(),
        texts,
        window: [x0 - 10.0, y0 - 10.0, x0 + 510.0, y0 + 410.0],
        scale: 4.0,
        budget: 5.0,
    });

    // 10 000 parcels 20 m apart, numbered inside them at 1.6 px/m (32 px a parcel).
    let parcels = |side: usize| {
        let mut objects = String::from("[");
        let mut texts = Vec::new();
        for i in 0..side * side {
            let (x, y) = (x0 + (i % side) as f64 * 20.0, y0 + (i / side) as f64 * 20.0);
            let _ = write!(
                objects,
                r#"{}{{"id":{},"layerId":"parsel","kind":"polygon","pts":[{{"x":{x},"y":{y}}},{{"x":{},"y":{y}}},{{"x":{},"y":{}}},{{"x":{x},"y":{}}}]}}"#,
                if i > 0 { "," } else { "" },
                i + 1,
                x + 18.0,
                x + 18.0,
                y + 18.0,
                y + 18.0
            );
            texts.push(((i + 1) as f64, format!("{}", i + 1), f64::NAN));
        }
        objects.push(']');
        (objects, texts)
    };
    let parcel_layer = r#"[{"id":"parsel","rank":0,"label":{"placement":"center","size":10,"area":"parcel","inside":true,"minFeaturePx":26}}]"#;
    let (objects, texts) = parcels(100);
    out.push(LabelScene {
        name: "10 000 parsel (parsel kipi)",
        objects,
        layers: parcel_layer.into(),
        texts,
        window: [x0 - 10.0, y0 - 10.0, x0 + 2010.0, y0 + 2010.0],
        scale: 1.6,
        budget: 30.0,
    });

    // 500 contours: 25 hills 450 m apart, 20 rings each 10 m apart (8 px at 0.8 px/m), 73 vertices a ring,
    // heights rising to the tops; read uphill every 400 px.
    let mut objects = String::from("[");
    let mut texts = Vec::new();
    for hill in 0..25 {
        let (cx, cy) = (
            x0 + 225.0 + (hill % 5) as f64 * 450.0,
            y0 + 225.0 + (hill / 5) as f64 * 450.0,
        );
        for ring in 0..20 {
            let id = hill * 20 + ring + 1;
            let r = 210.0 - ring as f64 * 10.0;
            let mut pts = String::new();
            for k in 0..=72 {
                let a = std::f64::consts::TAU * f64::from(k % 72) / 72.0;
                let wobble = 1.0 + 0.06 * jsmath::sin(3.0 * a + hill as f64);
                let (x, y) = (
                    cx + r * wobble * jsmath::cos(a),
                    cy + r * wobble * jsmath::sin(a),
                );
                let _ = write!(
                    pts,
                    r#"{}{{"x":{x},"y":{y}}}"#,
                    if k > 0 { "," } else { "" }
                );
            }
            let _ = write!(
                objects,
                r#"{}{{"id":{id},"layerId":"esyukselti","kind":"polyline","pts":[{pts}]}}"#,
                if id > 1 { "," } else { "" }
            );
            let z = 1000.0 + ring as f64 * 5.0;
            texts.push((id as f64, format!("{z}"), z));
        }
    }
    objects.push(']');
    out.push(LabelScene {
        name: "500 eş yükselti (25 tepe, 400 px'te bir)",
        objects,
        layers: r#"[{"id":"esyukselti","rank":0,"label":{"placement":"along","size":9,"line":"contour","repeat":400}}]"#.into(),
        texts,
        window: [x0 - 10.0, y0 - 10.0, x0 + 2260.0, y0 + 2260.0],
        scale: 0.8,
        budget: 15.0,
    });

    // An overview of 100 000 parcels at 0.15 px/m: 3 px a parcel, none labelled.
    let (objects, texts) = parcels(316);
    out.push(LabelScene {
        name: "100 000 parsellik genel bakış",
        objects,
        layers: parcel_layer.into(),
        texts,
        window: [x0 - 50.0, y0 - 50.0, x0 + 6370.0, y0 + 6370.0],
        scale: 0.15,
        budget: 10.0,
    });
    out
}
