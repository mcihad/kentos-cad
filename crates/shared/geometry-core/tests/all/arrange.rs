//! Hizala ve dağıt (docs/adr/0194 §2) against the independent reference in
//! `fixtures/arrange/v1/cases.json` (`scripts/fixtures/arrange_cases.py`,
//! IEEE doubles in the ADR's order, checked against exact fractions; no
//! KentOS code), through the op table as the web calls it
//! (`apps/web/src/tools/arrange.wasm.test.ts`); and an insert's box as the
//! geometry store keeps it.

use kentos_geometry_core::api::json::{FromJson, Json};
use kentos_geometry_core::api::run_named;
use kentos_geometry_core::block::Blocks;
use kentos_geometry_core::entity::Entity;
use kentos_geometry_core::ops::arrange::object_bounds;
use kentos_geometry_core::store::Store;
use kentos_geometry_core::text::Font;
use serde_json::{Value, json};

fn cases() -> Value {
    let path = format!(
        "{}/../../../fixtures/arrange/v1/cases.json",
        env!("CARGO_MANIFEST_DIR")
    );
    serde_json::from_str(&std::fs::read_to_string(path).expect("the cases")).expect("JSON")
}

fn bounds(b: &Value) -> Value {
    json!({ "minX": b[0], "minY": b[1], "maxX": b[2], "maxY": b[3] })
}

fn run(name: &str, args: Value) -> Value {
    serde_json::from_str(
        &run_named(name, &args.to_string()).unwrap_or_else(|e| panic!("{name}: {e}")),
    )
    .expect("JSON")
}

/// Bit for bit: the reference's doubles in the same order of operations.
fn same(got: &Value, want: &Value, what: &str) {
    let (g, w) = (got.as_f64().expect(what), want.as_f64().expect(what));
    assert!(
        g.to_bits() == w.to_bits() || (g == 0.0 && w == 0.0),
        "{what}: {g:?} ≠ {w:?}"
    );
}

#[test]
fn the_cases_move_as_the_reference_says() {
    let data = cases();
    for c in data["cases"].as_array().expect("cases") {
        let name = c["name"].as_str().expect("a name");
        let boxes: Vec<Value> = c["boxes"]
            .as_array()
            .expect("boxes")
            .iter()
            .map(bounds)
            .collect();
        let got = run("arrangeMoves", json!([boxes, c["mode"], c["at"]]));
        let want = c["moves"].as_array().expect("moves");
        let got = got.as_array().expect("moves");
        assert_eq!(got.len(), want.len(), "{name}");
        for (i, (g, w)) in got.iter().zip(want).enumerate() {
            same(&g["x"], &w[0], &format!("{name} #{i} east"));
            same(&g["y"], &w[1], &format!("{name} #{i} north"));
        }
    }
}

#[test]
fn a_reference_box_gives_its_side_or_middle() {
    let data = cases();
    for r in data["references"].as_array().expect("references") {
        let got = run("arrangeAt", json!([bounds(&r["box"]), r["mode"]]));
        same(&got, &r["at"], &format!("{} {}", r["mode"], r["box"]));
    }
    for u in data["unions"].as_array().expect("unions") {
        let boxes: Vec<Value> = u["boxes"]
            .as_array()
            .expect("boxes")
            .iter()
            .map(bounds)
            .collect();
        let got = run("arrangeUnion", json!([boxes]));
        let want = bounds(&u["box"]);
        for k in ["minX", "minY", "maxX", "maxY"] {
            same(&got[k], &want[k], &format!("union {k}"));
        }
    }
    assert!(run_named("arrangeMoves", &json!([[], "diagonal", null]).to_string()).is_err());
}

#[test]
fn an_insert_is_measured_as_the_store_measures_it() {
    let blocks = json!([{
        "id": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d6001", "name": "Direk", "base": { "x": 0, "y": 0 },
        "entities": [
            { "kind": "circle", "id": 1, "layerId": "0", "attrs": {}, "c": { "x": 0, "y": 0 }, "r": 0.4 },
            { "kind": "line", "id": 2, "layerId": "0", "attrs": {}, "a": { "x": 0.4, "y": 0 }, "b": { "x": 3, "y": 1.5 } },
            { "kind": "text", "id": 3, "layerId": "0", "attrs": {}, "p": { "x": 0, "y": -2 }, "text": "D-1", "height": 0.5, "rotation": 0 }
        ]
    }]);
    let insert = json!({
        "kind": "insert", "id": 7, "layerId": "0", "attrs": {}, "block": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d6001",
        "p": { "x": 487010, "y": 4420005 }, "scale": 2, "rotation": 0.5
    });
    let font = Font::from_id("overpass");
    let mut store = Store::new();
    store.set_font(font);
    store.set_blocks_json(&blocks.to_string()).expect("blocks");
    store.put_json(&format!("[{insert}]")).expect("the insert");
    let held = store.extent(Some(&[7.0])).expect("its box");
    let core_blocks =
        Blocks::from_json(&Json::parse(&blocks.to_string()).expect("JSON")).expect("blocks");
    let e = Entity::from_json(&Json::parse(&insert.to_string()).expect("JSON")).expect("an entity");
    assert_eq!(object_bounds(&e.shape, &core_blocks, font), held);
    // The op gives the same box.
    let boxes = run("arrangeBoxes", json!([[insert], blocks, "overpass"]));
    assert_eq!(boxes[0]["minX"].as_f64(), Some(held.min_x));
    assert_eq!(boxes[0]["maxY"].as_f64(), Some(held.max_y));
}
