//! Çizimler arası alışveriş (docs/adr/0193) against the shared cases
//! fixtures/exchange/v1/cases.json (scripts/fixtures/exchange_cases.py, no
//! KentOS code): the selection's drawing, taking from another drawing and a
//! drawing as a block. The web plays the same file (`model/exchange.test.ts`).

use kentos_contracts::{BlockId, DocumentSnapshotV2, EntityId};
use kentos_domain::exchange::{self, Picks, Same};
use serde_json::Value;

fn cases() -> Value {
    let path = format!(
        "{}/../../../fixtures/exchange/v1/cases.json",
        env!("CARGO_MANIFEST_DIR")
    );
    serde_json::from_str(&std::fs::read_to_string(path).expect("the cases")).expect("JSON")
}

fn strings(v: &Value) -> Vec<String> {
    v.as_array()
        .map(|l| {
            l.iter()
                .filter_map(|s| s.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

fn picks(v: &Value) -> Picks {
    Picks {
        layers: strings(&v["layers"]),
        blocks: strings(&v["blocks"]),
        text_styles: strings(&v["textStyles"]),
        dimension_styles: strings(&v["dimensionStyles"]),
        library: strings(&v["library"]),
        layer_states: strings(&v["layerStates"]),
        settings: v["settings"].as_bool().unwrap_or(false),
    }
}

/// The drawing as the contract reads it back to JSON: what the typed entry points give.
fn typed(v: &Value) -> Value {
    let doc: DocumentSnapshotV2 = serde_json::from_value(v.clone()).expect("a drawing");
    serde_json::to_value(doc).expect("JSON")
}

#[test]
fn the_selections_drawing_keeps_what_its_objects_use() {
    let c = cases();
    for case in c["selections"].as_array().expect("selections") {
        let name = case["name"].as_str().expect("a name");
        let from = &c["drawings"][case["from"].as_str().expect("from")];
        let got = exchange::selection_json(from, case["uids"].as_array().expect("uids"), "Seçim");
        assert_eq!(got, case["expect"], "{name}");
        // Through the contract's types, as the app saves it.
        let doc: DocumentSnapshotV2 = serde_json::from_value(from.clone()).expect("a drawing");
        let uids: Vec<EntityId> = serde_json::from_value(case["uids"].clone()).expect("ids");
        let saved = exchange::selection(&doc, &uids, "Seçim").expect("saved");
        assert_eq!(
            serde_json::to_value(saved).expect("JSON"),
            typed(&case["expect"]),
            "{name}: typed"
        );
    }
}

#[test]
fn taking_from_another_drawing_skips_or_replaces_the_same_names() {
    let c = cases();
    for case in c["takes"].as_array().expect("takes") {
        let name = case["name"].as_str().expect("a name");
        let ours = &c["drawings"][case["into"].as_str().expect("into")];
        let theirs = &c["drawings"][case["from"].as_str().expect("from")];
        let same = if case["same"] == "replace" {
            Same::Replace
        } else {
            Same::Skip
        };
        let got = exchange::take_json(ours, theirs, &picks(&case["picks"]), same);
        assert_eq!(got, case["expect"], "{name}");
    }
}

#[test]
fn a_drawing_becomes_one_block() {
    let c = cases();
    for case in c["files"].as_array().expect("files") {
        let name = case["name"].as_str().expect("a name");
        let ours = &c["drawings"][case["into"].as_str().expect("into")];
        let theirs = &c["drawings"][case["from"].as_str().expect("from")];
        let file = case["file"].as_str().expect("file");
        let (got, left) = exchange::file_block_json(ours, theirs, file).expect("a block");
        assert_eq!(got, case["expect"], "{name}");
        assert_eq!(
            (left.0 as u64, left.1 as u64),
            (
                case["left"]["images"].as_u64().expect("images"),
                case["left"]["tables"].as_u64().expect("tables")
            ),
            "{name}: left out"
        );
        // Through the contract's types, the block's id given.
        let o: DocumentSnapshotV2 = serde_json::from_value(ours.clone()).expect("ours");
        let t: DocumentSnapshotV2 = serde_json::from_value(theirs.clone()).expect("theirs");
        let id: BlockId =
            serde_json::from_value(Value::String("0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d9999".into()))
                .expect("an id");
        let made = exchange::file_block(&o, &t, file, id).expect("made");
        assert_eq!(
            (
                made.name.as_str(),
                made.images,
                made.tables,
                made.objects,
                made.layers,
                made.nested
            ),
            ("Kaynak", 1, 1, 8, 2, 2),
            "{name}"
        );
        assert_eq!(made.drawing.blocks.last().map(|b| b.id), Some(id));
    }
    // Nothing a block may hold: refused.
    let ours: DocumentSnapshotV2 =
        serde_json::from_value(c["drawings"]["ours"].clone()).expect("ours");
    let mut empty = ours.clone();
    empty.entities.clear();
    empty.uids.clear();
    let id: BlockId =
        serde_json::from_value(Value::String("0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d9998".into()))
            .expect("an id");
    let refused = exchange::file_block(&ours, &empty, "Boş", id).expect_err("refused");
    assert!(refused.contains("blok yapılacak nesne yok"), "{refused}");
}

#[test]
fn names_fold_as_the_rule_says() {
    assert_eq!(exchange::fold("  ada NO "), exchange::fold("Ada no"));
    assert_eq!(exchange::fold("Yol adı"), "yol adi");
    assert_eq!(exchange::block_key("DİREK"), exchange::block_key("direk"));
    assert_eq!(
        exchange::unique_names(&["DİREK".into()], &["Direk".into(), "Direk".into()]),
        ["Direk (2)", "Direk (3)"]
    );
}
