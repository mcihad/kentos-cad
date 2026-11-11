//! Asansör bloğunun mevzuat denetimi (docs/adr/8045 §5) on the section building (fixtures/arch/v1/section-building.kcad):
//! KentOS's library block Asansör placed on its storey 1. kat is an elevator; PAİY 34/2 measures its cabin (narrow side
//! 1.20 m, 1.80 m²) and its door (0.90 m), each fixed by `cad.blocks.parameters`. A shaft (a space named Asansör) is
//! read by its net size and the block standing in it by its cabin. In ten storeys, 34/5 asks one elevator of a
//! stretcher's size (1.20 m, 2.52 m², a 1.10 m door).

use kentos_commands::{CallOp, DocumentSnapshotV2};
use kentos_domain::Document;
use kentos_native_application::{ExecutionContext, registry};
use serde_json::{Value, json};

fn building() -> Document {
    let snap: DocumentSnapshotV2 = kentos_kcad::decode(include_bytes!(
        "../../../../../fixtures/arch/v1/section-building.kcad"
    ))
    .unwrap();
    Document::from_snapshot_v2(snap).unwrap()
}

fn run(doc: &mut Document, name: &str, input: Value) -> Value {
    let answer = registry::run(
        &mut ExecutionContext::new(doc),
        name,
        CallOp::Execute,
        input,
    )
    .unwrap();
    assert_eq!(answer["status"], "completed", "{name}: {answer}");
    answer
}

/// The library's Asansör block taken in alone.
fn take_elevator(doc: &mut Document) {
    let mut library: Value = serde_json::from_str(include_str!(
        "../../../../../catalog/v1/blocks/kentos-temel.kblok"
    ))
    .unwrap();
    library["blocks"]
        .as_array_mut()
        .unwrap()
        .retain(|b| b["name"] == "Asansör");
    assert_eq!(library["blocks"].as_array().map(Vec::len), Some(1));
    run(
        doc,
        "cad.blocks.edit",
        json!({"operation": "import", "library": library.to_string()}),
    );
}

/// An Asansör placed with its base point at (x, y) on `level`: its uid.
fn place(doc: &mut Document, x: f64, y: f64, level: &str, values: Value) -> String {
    let mut input = json!({"name": "Asansör", "at": {"x": x, "y": y}, "layerId": "0", "arch": {"level": level}});
    if !values.is_null() {
        input["parameters"] = values;
    }
    let made = run(doc, "cad.blocks.insert", input);
    made["output"]["uid"]
        .as_str()
        .or_else(|| made["output"]["uids"][0].as_str())
        .unwrap_or_else(|| panic!("an insert's uid: {made}"))
        .to_owned()
}

/// Mevzuat's elevator findings.
fn elevators(doc: &mut Document) -> Vec<Value> {
    let answer = run(doc, "cad.arch.audit", json!({"checks": ["regulation"]}));
    answer["output"]["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| {
            f["code"]
                .as_str()
                .is_some_and(|c| c.starts_with("paiy_elevator"))
        })
        .cloned()
        .collect()
}

fn said(found: &[Value]) -> Vec<String> {
    found
        .iter()
        .map(|f| f["message"].as_str().unwrap_or_default().to_owned())
        .collect()
}

fn apply(doc: &mut Document, f: &Value) {
    let step = &f["fix"]["step"];
    run(
        doc,
        step["command"].as_str().unwrap(),
        step["input"].clone(),
    );
}

#[test]
fn an_elevator_block_s_cabin_and_door_are_said_and_fixed() {
    let mut doc = building();
    take_elevator(&mut doc);
    // Its defaults are PAİY 34/2's least: a cabin 1.20 × 1.50 m and a 0.90 m door.
    let uid = place(&mut doc, 9.0, 1.0, "k1", Value::Null);
    assert!(
        elevators(&mut doc).is_empty(),
        "{:?}",
        said(&elevators(&mut doc))
    );
    // TS EN 81-70's type 2 (1.10 × 1.40 m; its row's door 0.90 m), then a 0.80 m door: each measure said, each fixed.
    for values in [
        json!({"Tip": "TSE Tip 2 110 × 140"}),
        json!({"Kapı genişliği": 0.8}),
    ] {
        run(
            &mut doc,
            "cad.blocks.parameters",
            json!({"operation": "set", "uids": [uid], "values": values}),
        );
    }
    let found = elevators(&mut doc);
    assert_eq!(
        said(&found),
        [
            "Asansör kabininin dar kenarı 1.10 m < 1.20 m — PAİY 34/2",
            "Asansör kabininin alanı 1.54 m² < 1.80 m² — PAİY 34/2",
            "Asansör kapısının net geçiş genişliği 0.80 m < 0.90 m — PAİY 34/2",
        ]
    );
    assert!(
        found
            .iter()
            .all(|f| f["code"] == "paiy_elevator_small"
                && f["rule"] == "PAİY 34/2 · RG 3.7.2017/30113"),
        "{found:#?}"
    );
    assert_eq!(found[0]["uids"], json!([uid]));
    assert_eq!(found[0]["fix"]["label"], "Kabini 1,20 × 1,50 m yap");
    assert_eq!(found[1]["fix"], found[0]["fix"]);
    assert_eq!(found[2]["fix"]["label"], "Kapıyı 0,90 m yap");
    let step = &found[0]["fix"]["step"];
    assert_eq!(step["command"], "cad.blocks.parameters");
    assert_eq!(
        step["input"],
        json!({"operation": "set", "uids": [uid], "values": {"Kabin eni": 1.2, "Kabin derinliği": 1.5}})
    );
    apply(&mut doc, &found[0]);
    let found = elevators(&mut doc);
    assert_eq!(
        said(&found),
        ["Asansör kapısının net geçiş genişliği 0.80 m < 0.90 m — PAİY 34/2"]
    );
    apply(&mut doc, &found[0]);
    assert!(
        elevators(&mut doc).is_empty(),
        "{:?}",
        said(&elevators(&mut doc))
    );
    // A wide cabin (1.60 × 1.10 m): its narrow side is its depth, raised; its width keeps the area.
    run(
        &mut doc,
        "cad.blocks.parameters",
        json!({"operation": "set", "uids": [uid], "values": {"Kabin eni": 1.6, "Kabin derinliği": 1.1}}),
    );
    let found = elevators(&mut doc);
    assert_eq!(
        said(&found),
        [
            "Asansör kabininin dar kenarı 1.10 m < 1.20 m — PAİY 34/2",
            "Asansör kabininin alanı 1.76 m² < 1.80 m² — PAİY 34/2",
        ]
    );
    assert_eq!(found[0]["fix"]["label"], "Kabini 1,60 × 1,20 m yap");
    apply(&mut doc, &found[0]);
    assert!(elevators(&mut doc).is_empty());
    // A second elevator on the storey, too small: of two, one keeps the cabin's size (34/2: half, rounded down).
    place(
        &mut doc,
        12.0,
        1.0,
        "k1",
        json!({"Kabin eni": 1.0, "Kabin derinliği": 1.25, "Kapı genişliği": 0.8}),
    );
    assert!(
        elevators(&mut doc).is_empty(),
        "{:?}",
        said(&elevators(&mut doc))
    );
}

#[test]
fn a_shaft_is_read_by_its_net_size_and_the_block_in_it_by_its_cabin() {
    let mut doc = building();
    take_elevator(&mut doc);
    let p = |x: f64, y: f64| json!({"x": x, "y": y});
    // A shaft of 1.60 × 2.10 m beside the building, a block of 1.10 × 1.40 m in it: the shaft holds a cabin of 34/2's
    // size, the block is none.
    run(
        &mut doc,
        "cad.spaces.create",
        json!({"layerId": "0", "spaces": [{
            "parts": [{"outer": {"pts": [p(20.0, 0.0), p(21.6, 0.0), p(21.6, 2.1), p(20.0, 2.1)]}}],
            "method": "net", "name": "Asansör", "void": true, "arch": {"level": "k1"}
        }]}),
    );
    assert!(
        elevators(&mut doc).is_empty(),
        "{:?}",
        said(&elevators(&mut doc))
    );
    let uid = place(
        &mut doc,
        20.0,
        0.0,
        "k1",
        json!({"Kabin eni": 1.1, "Kabin derinliği": 1.4}),
    );
    let found = elevators(&mut doc);
    assert_eq!(
        said(&found),
        [
            "Asansör kabininin dar kenarı 1.10 m < 1.20 m — PAİY 34/2",
            "Asansör kabininin alanı 1.54 m² < 1.80 m² — PAİY 34/2",
        ]
    );
    assert!(found.iter().all(|f| f["uids"] == json!([uid])));
    apply(&mut doc, &found[0]);
    assert!(elevators(&mut doc).is_empty());
}

#[test]
fn ten_storeys_ask_one_elevator_of_a_stretcher_s_size() {
    let mut doc = building();
    take_elevator(&mut doc);
    let mut arch = serde_json::to_value(doc.settings().arch.clone().unwrap()).unwrap();
    let levels = arch["levels"].as_array_mut().unwrap();
    let top = levels
        .iter()
        .map(|l| l["elevation"].as_f64().unwrap())
        .fold(f64::MIN, f64::max);
    for k in 1..=8 {
        levels.push(
            json!({"id": format!("ust{k}"), "name": format!("Üst kat {k}"),
                           "elevation": top + 3.0 * f64::from(k), "height": 3.0}),
        );
    }
    run(&mut doc, "cad.settings.set", json!({"arch": arch}));
    let uid = place(&mut doc, 9.0, 1.0, "k1", Value::Null);
    let found = elevators(&mut doc);
    assert_eq!(
        said(&found),
        [
            "Sedye asansörü yok (10 kat): en az bir asansörün kabini 1.20 m dar kenarlı ve 2.52 m², kapısı 1.10 m olmalı \
          — PAİY 34/5"
        ]
    );
    assert_eq!(found[0]["code"], "paiy_elevator_stretcher");
    assert_eq!(found[0]["rule"], "PAİY 34/5 · RG 3.7.2017/30113");
    assert_eq!(found[0]["uids"], json!([uid]));
    assert_eq!(
        found[0]["fix"]["label"],
        "Kabini 1,20 × 2,10 m, kapıyı 1,10 m yap"
    );
    apply(&mut doc, &found[0]);
    assert!(
        elevators(&mut doc).is_empty(),
        "{:?}",
        said(&elevators(&mut doc))
    );
    // Its lookup row Sedye 120 × 210 is the same.
    let other = place(&mut doc, 12.0, 1.0, "k1", json!({"Tip": "Sedye 120 × 210"}));
    run(
        &mut doc,
        "cad.blocks.parameters",
        json!({"operation": "set", "uids": [uid], "values": {"Tip": "Konut 120 × 150"}}),
    );
    assert!(
        elevators(&mut doc).is_empty(),
        "{other}: {:?}",
        said(&elevators(&mut doc))
    );
}
