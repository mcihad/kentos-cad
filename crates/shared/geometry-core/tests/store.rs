//! The geometry store against the frozen answers of the TypeScript
//! PickIndex it replaced (`fixtures/geometry/v1/store-v1.json`, recorded by
//! `apps/web/scripts/fixtures/record-store.test.ts`; docs/adr/0008, S1): picking,
//! edge picking, snapping, window and crossing selection, enclosing shapes,
//! overlapping objects, boundary edges, labels and grips on a fixed scene,
//! the tool previews and totals (trim, extend, ghosts, stretch ghosts,
//! selection totals), and what the layer builders draw with the
//! expressions' geometry values; and what the processing tools ask
//! (`store-processing.json`, S4): the box test, corner numbering and
//! edge-length labels by id. The WASM build runs the same files
//! (`apps/web/src/wasm/store.wasm.test.ts`).

// Test harness code, not the core: the std float methods are fine here.
#![allow(clippy::disallowed_methods)]

use std::collections::HashMap;

use kentos_geometry_core::Vec2;
use kentos_geometry_core::api::json::{FromJson, Json, ToJson};
use kentos_geometry_core::entity::Entity;
use kentos_geometry_core::geom::affine::Affine;
use kentos_geometry_core::geom::intersect::Edge;
use kentos_geometry_core::geometry::Bounds;
use kentos_geometry_core::processing::numbering::{CornerWalk, StartCorner};
use kentos_geometry_core::store::Store;
use kentos_geometry_core::store::snap::SnapKind;
use serde_json::{Value, json};

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../fixtures/geometry/v1");

const KINDS: [&str; 9] = [
    "endpoint",
    "midpoint",
    "center",
    "node",
    "quadrant",
    "intersection",
    "perpendicular",
    "tangent",
    "nearest",
];

/// Numbers within |a − e| ≤ abs + rel·max(|a|, |e|); everything else exactly.
fn same(actual: &Value, expected: &Value, abs: f64, rel: f64, path: &str) -> Result<(), String> {
    match (actual, expected) {
        (Value::Number(a), Value::Number(e)) => {
            let (a, e) = (a.as_f64().unwrap(), e.as_f64().unwrap());
            let d = (a - e).abs();
            if d <= abs + rel * a.abs().max(e.abs()) {
                Ok(())
            } else {
                Err(format!("{path}: {a} ≠ {e} (fark {d})"))
            }
        }
        (Value::Array(a), Value::Array(e)) => {
            if a.len() != e.len() {
                return Err(format!("{path}: {} öğe ≠ {} öğe", a.len(), e.len()));
            }
            for (i, (x, y)) in a.iter().zip(e).enumerate() {
                same(x, y, abs, rel, &format!("{path}[{i}]"))?;
            }
            Ok(())
        }
        (Value::Object(a), Value::Object(e)) => {
            for k in a.keys().chain(e.keys()) {
                match (a.get(k), e.get(k)) {
                    (Some(x), Some(y)) => same(x, y, abs, rel, &format!("{path}.{k}"))?,
                    (None, _) => return Err(format!("{path}.{k}: eksik")),
                    (_, None) => return Err(format!("{path}.{k}: fazla")),
                }
            }
            Ok(())
        }
        _ if actual == expected => Ok(()),
        _ => Err(format!("{path}: {actual} ≠ {expected}")),
    }
}

fn num(v: &Value) -> f64 {
    v.as_f64().unwrap_or(f64::NAN)
}

fn point(v: &Vec2) -> Value {
    json!({ "x": v.x, "y": v.y })
}

fn rect(v: &Value) -> Bounds {
    Bounds {
        min_x: num(&v["minX"]),
        min_y: num(&v["minY"]),
        max_x: num(&v["maxX"]),
        max_y: num(&v["maxY"]),
    }
}

fn except(v: &Value) -> Option<f64> {
    v.as_f64()
}

fn edge(e: &Edge) -> Value {
    match *e {
        Edge::Seg { a, b } => json!({ "kind": "seg", "a": point(&a), "b": point(&b) }),
        Edge::Arc { c, r, a0, sweep } => {
            json!({ "kind": "arc", "c": point(&c), "r": r, "a0": a0, "sweep": sweep })
        }
    }
}

fn opt(id: Option<f64>) -> Value {
    id.map_or(Value::Null, |x| json!(x))
}

fn ids(v: &Value) -> Vec<f64> {
    v.as_array().unwrap().iter().map(num).collect()
}

/// A number as the TypeScript's `toJson` writes it.
fn num_json(x: f64) -> Value {
    if x.is_nan() {
        json!("#NaN")
    } else if x == f64::INFINITY {
        json!("#Inf")
    } else if x == f64::NEG_INFINITY {
        json!("#-Inf")
    } else {
        json!(x)
    }
}

/// An object's own points of path or ring `k`, which a drawn record refers to.
fn own_points(e: &Value, k: usize) -> Vec<Value> {
    let arr = |v: &Value| v.as_array().cloned().unwrap_or_default();
    match e["kind"].as_str().unwrap() {
        "line" => vec![e["a"].clone(), e["b"].clone()],
        "polyline" => arr(&e["pts"]),
        "polygon" if k == 0 => arr(&e["pts"]),
        "polygon" => arr(&e["holes"][k - 1]["pts"]),
        "hatch" if k == 0 => arr(&e["ring"]),
        "hatch" => arr(&e["holes"][k - 1]),
        _ => Vec::new(),
    }
}

/// Drawn records read back into the style engine's geometry, as `DrawnReader` reads them.
struct Records<'a> {
    buf: &'a [f64],
    at: usize,
}

impl Records<'_> {
    fn next(&mut self) -> f64 {
        self.at += 1;
        self.buf[self.at - 1]
    }

    fn points(&mut self, e: &Value, k: usize) -> Value {
        let n = self.next();
        if n == -1.0 {
            return Value::Array(own_points(e, k));
        }
        if n == -2.0 {
            let mut p = own_points(e, k);
            p.reverse();
            return Value::Array(p);
        }
        Value::Array(
            (0..n as usize)
                .map(|_| json!({ "x": num_json(self.next()), "y": num_json(self.next()) }))
                .collect(),
        )
    }

    fn read(&mut self, e: &Value) -> Value {
        match self.next() as i64 {
            1 => {
                json!({ "cls": "marker", "point": { "x": num_json(self.next()), "y": num_json(self.next()) } })
            }
            2 => {
                let count = self.next() as usize;
                let paths: Vec<Value> = (0..count)
                    .map(|k| {
                        let closed = self.next() == 1.0;
                        json!({ "pts": self.points(e, k), "closed": closed })
                    })
                    .collect();
                json!({ "cls": "line", "paths": paths })
            }
            3 => {
                let count = self.next() as usize;
                let rings: Vec<Value> = (0..count).map(|k| self.points(e, k)).collect();
                json!({ "cls": "fill", "rings": rings })
            }
            _ => Value::Null,
        }
    }
}

/// `measures` records as `measuredAt` reads them.
fn read_measures(buf: &[f64], n: usize) -> Value {
    let or_null = |has: bool, v: Value| if has { v } else { Value::Null };
    Value::Array(
        (0..n)
            .map(|i| {
                let k = i * 6;
                let f = buf[k] as u32;
                json!({
                    "length": or_null(f & 1 != 0, num_json(buf[k + 1])),
                    "area": or_null(f & 2 != 0, num_json(buf[k + 2])),
                    "anchor": or_null(f & 4 != 0, json!({ "x": num_json(buf[k + 3]), "y": num_json(buf[k + 4]) })),
                })
            })
            .collect(),
    )
}

/// A core answer as the TypeScript's `toJson` writes it (NaN and ±∞ as `"#NaN"`, `"#Inf"`, `"#-Inf"`).
fn core_json<T: ToJson + ?Sized>(v: &T) -> Value {
    let mut out = String::new();
    v.write_json(&mut out);
    serde_json::from_str(&out).expect("core JSON")
}

/// Corner numbering records as `ObjectStore.numberCorners` reads them.
fn read_corners(buf: &[f64]) -> Value {
    Value::Array(
        buf.chunks(5)
            .map(|r| {
                json!({
                    "p": { "x": num_json(r[0]), "y": num_json(r[1]) },
                    "out": { "x": num_json(r[2]), "y": num_json(r[3]) },
                    "ref": r[4],
                })
            })
            .collect(),
    )
}

/// Edge-length labels as `ObjectStore.edgeLengths` reads them.
fn read_edge_labels(buf: &[f64]) -> Value {
    let labels: Vec<Value> = buf[1..]
        .chunks(5)
        .map(|r| {
            json!({
                "id": r[0],
                "p": { "x": num_json(r[1]), "y": num_json(r[2]) },
                "rotation": num_json(r[3]),
                "length": num_json(r[4]),
            })
        })
        .collect();
    json!({ "labels": labels, "skipped": buf[0] })
}

#[test]
fn the_store_gives_the_typescript_answers() {
    let mut files: Vec<std::path::PathBuf> = std::fs::read_dir(DIR)
        .expect("fixture directory")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("store-") && n.ends_with(".json"))
        })
        .collect();
    files.sort();
    assert!(files.len() >= 2, "store fixtures: {files:?}");
    for path in &files {
        check(path);
    }
}

fn check(path: &std::path::Path) {
    let file: Value = serde_json::from_str(&std::fs::read_to_string(path).expect("store fixture"))
        .expect("fixture JSON");
    assert_eq!(file["format"], "kentos.geometry-store");
    assert_eq!(file["version"], 1);
    let abs = num(&file["tolerance"]["abs"]);
    let rel = num(&file["tolerance"]["rel"]);
    let mut store = Store::new();
    store
        .put_json(&serde_json::to_string(&file["entities"]).unwrap())
        .expect("entities");
    store
        .set_layers_json(&serde_json::to_string(&file["layers"]).unwrap())
        .expect("layers");
    store
        .set_label_defaults_json(&serde_json::to_string(&file["labelDefaults"]).unwrap())
        .expect("label defaults");
    let entity_json: HashMap<u64, &Value> = file["entities"]
        .as_array()
        .expect("entities")
        .iter()
        .map(|e| (num(&e["id"]).to_bits(), e))
        .collect();
    let entities: HashMap<u64, Entity> = file["entities"]
        .as_array()
        .expect("entities")
        .iter()
        .map(|e| {
            let json = Json::parse(&e.to_string()).expect("entity JSON");
            (
                num(&e["id"]).to_bits(),
                Entity::from_json(&json).expect("entity"),
            )
        })
        .collect();
    let cases = file["cases"].as_array().expect("cases");
    assert!(cases.len() > 500, "{} cases", cases.len());
    let mut failures = Vec::new();
    for c in cases {
        let a = c["args"].as_array().expect("args");
        let label = format!(
            "{} {}",
            c["op"].as_str().unwrap(),
            c["name"].as_str().unwrap()
        );
        let p = || Vec2::new(num(&a[0]), num(&a[1]));
        let got = match c["op"].as_str().unwrap() {
            "hit" => opt(store.hit(p(), num(&a[2]))),
            "hitEdge" => opt(store.hit_edge(p(), num(&a[2])).first().map(|h| h.0)),
            "snap" => {
                let mut mask = 0;
                for k in a[3].as_array().unwrap() {
                    let i = KINDS.iter().position(|n| k == *n).expect("snap kind");
                    mask |= SnapKind::ALL[i].bit();
                }
                let from = (!a[4].is_null()).then(|| Vec2::new(num(&a[4]["x"]), num(&a[4]["y"])));
                store.snap(p(), num(&a[2]), mask, from).map_or(Value::Null, |h| {
                    let kind = KINDS[SnapKind::ALL.iter().position(|k| *k == h.kind).unwrap()];
                    json!({ "kind": kind, "point": point(&h.point), "entityId": h.id })
                })
            }
            "enclosing" => store.enclosing(p()).map_or(Value::Null, |(id, ring)| {
                json!({ "id": id, "ring": ring.iter().map(point).collect::<Vec<_>>() })
            }),
            "inRect" => json!(store.in_rect(&rect(&a[0]), a[1].as_bool().unwrap())),
            "overlapping" => json!(store.overlapping(&rect(&a[0]), except(&a[1])).iter().map(|it| it.id).collect::<Vec<_>>()),
            "edgesIn" => json!(store.edges_in(&rect(&a[0]), except(&a[1])).iter().map(edge).collect::<Vec<_>>()),
            "labels" => json!(store.labels(&rect(&a[0]), num(&a[1]), a[2].as_f64())),
            "grips" => json!(store.grips(&ids(&a[0]))),
            "trim" | "extend" => {
                let id = num(&a[0]);
                let target = &entities[&id.to_bits()];
                let pick = Vec2::new(num(&a[1]["x"]), num(&a[1]["y"]));
                let chosen = (!a[3].is_null()).then(|| ids(&a[3]));
                let view = rect(&a[2]);
                if c["op"] == "trim" {
                    core_json(&store.trim_preview(target, pick, Some(id), chosen.as_deref(), &view))
                } else {
                    core_json(&store.extend_preview(target, pick, Some(id), chosen.as_deref(), &view))
                }
            }
            "ghosts" => {
                let affines: Vec<Affine> = a[1]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|m| std::array::from_fn(|i| num(&m[i])))
                    .collect();
                core_json(&store.transform_outlines(&ids(&a[0]), &affines, num(&a[2]) as usize))
            }
            "stretchGhosts" => core_json(&store.stretch_outlines(
                &ids(&a[0]),
                &rect(&a[1]),
                num(&a[2]),
                num(&a[3]),
            )),
            "measure" => {
                let (length, area) = store.measure(&ids(&a[0]));
                core_json(&[length, area])
            }
            "drawn" => {
                let ids = ids(&a[0]);
                let clip = (!a[2].is_null()).then(|| rect(&a[2]));
                let buf = store.drawn(&ids, a[1].as_bool().unwrap(), clip.as_ref());
                let mut records = Records { buf: &buf, at: 0 };
                Value::Array(
                    ids.iter()
                        .map(|id| records.read(entity_json[&id.to_bits()]))
                        .collect(),
                )
            }
            "measures" => {
                let ids = ids(&a[0]);
                read_measures(&store.measures(&ids), ids.len())
            }
            "inBox" => json!(store.in_box(&rect(&a[0]))),
            "numberCorners" => {
                let w = &a[1];
                let walk = CornerWalk {
                    ccw: w["dir"] == "ccw",
                    start: StartCorner::parse(w["start"].as_str().unwrap()).unwrap(),
                    point: (!w["point"].is_null())
                        .then(|| Vec2::new(num(&w["point"]["x"]), num(&w["point"]["y"]))),
                    tolerance: num(&w["tolerance"]),
                    shared: w["shared"].as_bool().unwrap(),
                };
                let existing: Vec<Vec2> = a[2]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|p| Vec2::new(num(&p["x"]), num(&p["y"])))
                    .collect();
                read_corners(&store.number_corners(&ids(&a[0]), &walk, &existing))
            }
            "edgeLengths" => read_edge_labels(&store.edge_lengths(
                &ids(&a[0]),
                num(&a[1]),
                num(&a[2]),
                a[3] == "inside",
                a[4].as_bool().unwrap(),
            )),
            op => panic!("unknown op {op}"),
        };
        if let Err(e) = same(&got, &c["expect"], abs, rel, &label) {
            failures.push(e);
        }
    }
    assert!(
        failures.is_empty(),
        "{}: {} of {} case(s) differ:\n{}",
        path.display(),
        failures.len(),
        cases.len(),
        failures
            .iter()
            .take(20)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
