//! A styled layer's way to the GPU as `fixtures/style/v1/batches.json` holds
//! it (the web records it, `apps/web/scripts/fixtures/record-batches.test.ts`,
//! and checks it in `render/styledFixture.test.ts`): the scale symbols are
//! compiled at, how each object is drawn, what the page sends the style core,
//! and the GPU batches it makes of the answer with the file's palette. The
//! desktop builds the same layer through its own page and must get the same.

use std::path::PathBuf;

use kentos_contracts::{Entity, LayerStyle};
use kentos_geometry_core::Vec2;
use kentos_geometry_core::store::Store;
use kentos_native_application::geometry::shape;
use kentos_native_style::StylePalette;
use kentos_native_style::batches::{DecodeOptions, decode};
use kentos_native_style::library::{Source, StyleLibrary};
use kentos_native_style::program::{BuildOptions, build_layer, symbol_scale_of};
use serde_json::{Value, json};

fn fixture() -> Value {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/style/v1/batches.json");
    let text = std::fs::read_to_string(&path).expect("batches.json");
    serde_json::from_str(&text).expect("batches.json is JSON")
}

/// A number as the fixture writes it: NaN, ±∞ and −0 as text.
fn num(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => match s.as_str() {
            "NaN" => Some(f64::NAN),
            "Infinity" => Some(f64::INFINITY),
            "-Infinity" => Some(f64::NEG_INFINITY),
            "-0" => Some(-0.0),
            _ => None,
        },
        _ => None,
    }
}

/// Keys whose numbers are float32 on the GPU (written in the fewest digits that read back).
const FLOAT32: [&str; 3] = ["positions", "segments", "instances"];

/// Whether `got` is `want`: numbers as numbers (1 and 1.0 alike), float32 arrays as float32.
fn same(got: &Value, want: &Value, at: &str, f32s: bool, out: &mut Vec<String>) {
    match (got, want) {
        (Value::Object(g), Value::Object(w)) => {
            for k in g.keys().chain(w.keys()) {
                let (gv, wv) = (
                    g.get(k).unwrap_or(&Value::Null),
                    w.get(k).unwrap_or(&Value::Null),
                );
                if g.contains_key(k) != w.contains_key(k) {
                    out.push(format!("{at}.{k}: {} ≠ {}", gv, wv));
                    continue;
                }
                same(
                    gv,
                    wv,
                    &format!("{at}.{k}"),
                    FLOAT32.contains(&k.as_str()),
                    out,
                );
            }
        }
        (Value::Array(g), Value::Array(w)) => {
            if g.len() != w.len() {
                out.push(format!("{at}: {} items ≠ {}", g.len(), w.len()));
                return;
            }
            for (i, (gv, wv)) in g.iter().zip(w).enumerate() {
                same(gv, wv, &format!("{at}[{i}]"), f32s, out);
            }
        }
        _ => match (num(got), num(want)) {
            (Some(a), Some(b)) => {
                let equal = if f32s {
                    (a as f32) == (b as f32) || ((a as f32).is_nan() && (b as f32).is_nan())
                } else {
                    a == b || (a.is_nan() && b.is_nan())
                };
                if !equal {
                    out.push(format!("{at}: {a} ≠ {b}"));
                }
            }
            _ => {
                if got != want {
                    out.push(format!("{at}: {got} ≠ {want}"));
                }
            }
        },
    }
}

fn check(got: &Value, want: &Value, what: &str) -> Vec<String> {
    let mut out = Vec::new();
    same(got, want, what, false, &mut out);
    out
}

const MODES: [&str; 5] = ["skip", "dimension", "set", "own", "renderer"];

#[test]
fn builds_the_web_s_batches() {
    let f = fixture();
    assert_eq!(f["format"], "kentos.style-batches");
    assert_eq!(f["version"], 1);
    let p = &f["palette"];
    let palette = StylePalette {
        fg: p["fg"].as_str().unwrap().into(),
        fg_dim: p["fgDim"].as_str().unwrap().into(),
        ink: p["ink"].as_str().unwrap().into(),
        paper: p["paper"].as_str().unwrap().into(),
    };
    let mut library = StyleLibrary::default();
    let mut items: Vec<Value> = f["assets"].as_array().unwrap().clone();
    for (id, symbol) in f["library"].as_object().unwrap() {
        items.push(json!({ "kind": "symbol", "id": id, "name": id, "path": [], "symbol": symbol }));
    }
    library.load(Source::Project, &items, &[]);
    let origin = Vec2::new(
        f["origin"]["x"].as_f64().unwrap(),
        f["origin"]["y"].as_f64().unwrap(),
    );
    let layer_id = f["layer"]["id"].as_str().unwrap();
    let layer_name = f["layer"]["name"].as_str().unwrap().to_owned();
    let mut problems = Vec::new();
    for c in f["cases"].as_array().unwrap() {
        let id = c["id"].as_str().unwrap();
        let style: LayerStyle = serde_json::from_value(c["style"].clone()).expect("style");
        let entities: Vec<Entity> = c["entities"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| {
                let mut e = e.clone();
                e["layerId"] = json!(layer_id);
                serde_json::from_value(e).expect("entity")
            })
            .collect();
        let list: Vec<&Entity> = entities.iter().collect();
        let mut store = Store::new();
        store.put_many(entities.iter().map(|e| {
            let b = e.base();
            (
                f64::from(b.id),
                b.layer_id.as_str(),
                b.label.as_deref().is_some_and(|l| !l.is_empty()),
                shape(e),
            )
        }));
        let view = &c["view"];
        let screen = view["symbolSize"] == "screen";
        let scale = symbol_scale_of(
            screen,
            c["plotScale"].as_f64().unwrap(),
            view["pxPerM"].as_f64().unwrap(),
        );
        let e = &c["expect"];
        problems.extend(check(
            &json!(scale),
            &e["symbolScale"],
            &format!("{id}.symbolScale"),
        ));
        let name = layer_name.clone();
        let names = move |_: &str| name.clone();
        let opts = BuildOptions {
            origin,
            plot_scale: scale,
            screen,
            hairlines: view["lineWeights"] == false,
            clip: None,
            library: &library,
            layer_name: &names,
        };
        let (call, batches) = build_layer(&store, &style, &list, &opts).expect("build");
        let decisions: Vec<Value> = list
            .iter()
            .enumerate()
            .map(|(i, e)| {
                json!({
                    "id": e.base().id,
                    "mode": MODES[call.objects[4 * i] as usize],
                    "a": call.objects[4 * i + 1],
                    "simple": call.objects[4 * i + 2],
                    "color": call.objects[4 * i + 3],
                })
            })
            .collect();
        problems.extend(check(
            &json!(decisions),
            &e["decisions"],
            &format!("{id}.decisions"),
        ));
        let program: Value = serde_json::from_str(&call.program).unwrap();
        problems.extend(check(&program, &e["program"], &format!("{id}.program")));
        problems.extend(check(
            &json!(call.objects),
            &e["objects"],
            &format!("{id}.objects"),
        ));
        let table = json!({
            "texts": call.table.texts,
            "lens": call.table.lens,
            "numbers": call.table.numbers.iter().map(|x| if x.is_finite() { json!(x) } else { json!(x.to_string()) }).collect::<Vec<_>>(),
        });
        problems.extend(check(&table, &e["table"], &format!("{id}.table")));
        let layer = decode(
            batches,
            &DecodeOptions {
                palette: &palette,
                plot_scale: scale,
                library: &library,
            },
        )
        .expect("decode");
        problems.extend(check(
            &layer.to_json(),
            &e["batches"],
            &format!("{id}.batches"),
        ));
    }
    assert!(
        problems.is_empty(),
        "{} differences:\n{}",
        problems.len(),
        problems.join("\n")
    );
}

/// A large layer drawn in parts (the desktop's parts of a layer, docs/adr/0121):
/// every case built whole and built in parts of one, two and three objects
/// one after another. In the parts' merged order the batches meet in the
/// whole layer's order, and a batch's parts carry its objects' numbers in
/// the whole batch's order: the GPU draws the same things in the same order.
#[test]
fn a_layer_built_in_parts_draws_as_the_layer_built_whole() {
    use kentos_native_style::batches::{StyledLayer, merged_order};

    let f = fixture();
    let p = &f["palette"];
    let palette = StylePalette {
        fg: p["fg"].as_str().unwrap().into(),
        fg_dim: p["fgDim"].as_str().unwrap().into(),
        ink: p["ink"].as_str().unwrap().into(),
        paper: p["paper"].as_str().unwrap().into(),
    };
    let mut library = StyleLibrary::default();
    let mut items: Vec<Value> = f["assets"].as_array().unwrap().clone();
    for (id, symbol) in f["library"].as_object().unwrap() {
        items.push(json!({ "kind": "symbol", "id": id, "name": id, "path": [], "symbol": symbol }));
    }
    library.load(Source::Project, &items, &[]);
    let origin = Vec2::new(
        f["origin"]["x"].as_f64().unwrap(),
        f["origin"]["y"].as_f64().unwrap(),
    );
    let layer_id = f["layer"]["id"].as_str().unwrap();
    let layer_name = f["layer"]["name"].as_str().unwrap().to_owned();
    let mut checked = 0;
    for c in f["cases"].as_array().unwrap() {
        let id = c["id"].as_str().unwrap();
        let style: LayerStyle = serde_json::from_value(c["style"].clone()).expect("style");
        let entities: Vec<Entity> = c["entities"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| {
                let mut e = e.clone();
                e["layerId"] = json!(layer_id);
                serde_json::from_value(e).expect("entity")
            })
            .collect();
        let mut store = Store::new();
        store.put_many(entities.iter().map(|e| {
            let b = e.base();
            (
                f64::from(b.id),
                b.layer_id.as_str(),
                b.label.as_deref().is_some_and(|l| !l.is_empty()),
                shape(e),
            )
        }));
        let view = &c["view"];
        let screen = view["symbolSize"] == "screen";
        let scale = symbol_scale_of(
            screen,
            c["plotScale"].as_f64().unwrap(),
            view["pxPerM"].as_f64().unwrap(),
        );
        let name = layer_name.clone();
        let names = move |_: &str| name.clone();
        let opts = BuildOptions {
            origin,
            plot_scale: scale,
            screen,
            hairlines: view["lineWeights"] == false,
            clip: None,
            library: &library,
            layer_name: &names,
        };
        let build = |list: &[&Entity]| -> StyledLayer {
            let (call, batches) = build_layer(&store, &style, list, &opts).expect("build");
            assert!(!call.reads_index, "{id}: no case reads $sıra");
            decode(
                batches,
                &DecodeOptions {
                    palette: &palette,
                    plot_scale: scale,
                    library: &library,
                },
            )
            .expect("decode")
        };
        let list: Vec<&Entity> = entities.iter().collect();
        let whole = build(&list);
        // What the whole layer draws: each batch's key and its numbers, in order.
        let want: Vec<(u64, &[f32])> = whole
            .batches
            .iter()
            .map(|b| (b.key, &whole.data[b.range.clone()]))
            .collect();
        for size in 1..=3 {
            let parts: Vec<StyledLayer> = list.chunks(size).map(build).collect();
            let refs: Vec<&StyledLayer> = parts.iter().collect();
            let mut got: Vec<(u64, Vec<f32>)> = Vec::new();
            for (p, b) in merged_order(&refs) {
                let batch = &parts[p].batches[b];
                let data = &parts[p].data[batch.range.clone()];
                match got.last_mut() {
                    Some((key, numbers)) if *key == batch.key => numbers.extend_from_slice(data),
                    _ => got.push((batch.key, data.to_vec())),
                }
            }
            assert_eq!(
                got.len(),
                want.len(),
                "{id}, parts of {size}: the whole layer's batches"
            );
            for (i, ((gk, gd), (wk, wd))) in got.iter().zip(&want).enumerate() {
                assert_eq!(gk, wk, "{id}, parts of {size}: batch {i} in its place");
                let same = gd.len() == wd.len()
                    && gd
                        .iter()
                        .zip(wd.iter())
                        .all(|(a, b)| a.to_bits() == b.to_bits());
                assert!(same, "{id}, parts of {size}: batch {i}'s numbers");
            }
            checked += 1;
        }
    }
    assert_eq!(checked, 33, "eleven cases, three part sizes");
}
