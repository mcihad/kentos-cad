//! Korkuluğun mevzuat denetimi (docs/adr/8045 §4) on the section building (fixtures/arch/v1/section-building.kcad): the
//! floor of its storey 1. kat has the stair's void, and PAİY 38/1 asks a railing along its sides that no wall runs
//! along, but where the stair from Zemin kat arrives; Boşluktan's railings answer it. A low railing (38/1) and a wide
//! gap (38/4) are said and fixed by cad.arch.edit; a balcony's free sides by its edge (cad.slabs.edit).

use kentos_commands::{CallOp, DocumentSnapshotV2, Entity};
use kentos_domain::Document;
use kentos_native_application::{ExecutionContext, railing_voids, registry};
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

/// The railings' findings of Mevzuat: the void's, a balcony's and a railing's own (the stairs' are left out).
fn railings(doc: &mut Document) -> Vec<Value> {
    let answer = run(doc, "cad.arch.audit", json!({"checks": ["regulation"]}));
    answer["output"]["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| {
            let m = f["message"].as_str().unwrap_or_default();
            ["Döşeme boşluğunun", "Balkonun", "Korkulu"]
                .iter()
                .any(|w| m.starts_with(w))
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
fn a_void_s_open_sides_a_low_railing_a_wide_gap_and_a_balcony_s_sides_are_said_and_fixed() {
    let mut doc = building();
    // The void (2.85 × 3.94 m): its west and east sides along the stair hall's walls, its south side open where the
    // stair's second flight (6.20–7.40) arrives: 2.85 m north and 1.625 m south want a railing.
    let found = railings(&mut doc);
    assert_eq!(
        said(&found),
        ["Döşeme boşluğunun 4.47 m'lik kenarında korkuluk yok — PAİY 38/1"]
    );
    assert_eq!(found[0]["code"], "paiy_railing_missing");
    assert_eq!(found[0]["rule"], "PAİY 38/1 · RG 3.7.2017/30113");
    assert!(found[0]["fix"].is_null(), "drawn with Korkuluk's Boşluktan");
    // Boşluktan's railings: the void's finding goes.
    let (slab, hole) = doc
        .entities()
        .find_map(|e| match e {
            Entity::Slab(s) => s
                .slab
                .parts
                .iter()
                .flat_map(|p| &p.holes)
                .next()
                .map(|h| (s.slab.clone(), h.clone())),
            _ => None,
        })
        .expect("the floor with the void");
    let pieces = railing_voids::void_railings(&doc, &slab, &hole, Some("k1"), 0.05);
    let lengths: Vec<f64> = pieces
        .iter()
        .map(|(p, _)| {
            p.windows(2)
                .map(|w| (w[1].x - w[0].x).hypot(w[1].y - w[0].y))
                .sum()
        })
        .collect();
    assert_eq!(lengths.len(), 2, "{lengths:?}");
    assert!(
        (lengths.iter().sum::<f64>() - 4.475).abs() < 1e-9,
        "{lengths:?}"
    );
    let railings_in: Vec<Value> = pieces
        .iter()
        .map(|(p, closed)| {
            json!({"pts": p.iter().map(|q| json!({"x": q.x, "y": q.y})).collect::<Vec<_>>(), "closed": closed,
                   "arch": {"level": "k1"}})
        })
        .collect();
    let made = run(
        &mut doc,
        "cad.railings.create",
        json!({"layerId": "0", "railings": railings_in}),
    );
    assert!(
        railings(&mut doc).is_empty(),
        "{:?}",
        said(&railings(&mut doc))
    );
    // One of them 0.90 m high with a 0.12 m gap: both said, each fixed by cad.arch.edit.
    let uid = made["output"]["uids"][0].as_str().unwrap().to_owned();
    run(
        &mut doc,
        "cad.arch.edit",
        json!({"uids": [uid], "operation": {"kind": "set", "height": 0.9, "gap": 0.12}}),
    );
    let found = railings(&mut doc);
    assert_eq!(
        said(&found),
        [
            "Korkuluk 0.90 m < 1.10 m — PAİY 38/1",
            "Korkuluğun çubuk aralığı 0.12 m > 0.10 m — PAİY 38/4",
        ]
    );
    assert_eq!(found[1]["rule"], "PAİY 38/4 · RG 3.7.2017/30113");
    for f in &found {
        assert_eq!(f["fix"]["step"]["command"], "cad.arch.edit");
        apply(&mut doc, f);
    }
    assert!(railings(&mut doc).is_empty());
    // A 3.00 × 1.50 m balcony off the south wall, its north side along the wall's face: 6.00 m free, no edge.
    let p = |x: f64, y: f64| json!({"x": x, "y": y});
    run(
        &mut doc,
        "cad.slabs.create",
        json!({"layerId": "0", "slabs": [{
            "parts": [{"outer": {"pts": [p(2.0, -1.625), p(5.0, -1.625), p(5.0, -0.125), p(2.0, -0.125)]}}],
            "role": "balcony", "arch": {"level": "k1"}
        }]}),
    );
    let found = railings(&mut doc);
    assert_eq!(
        said(&found),
        ["Balkonun 6.00 m'lik serbest kenarında korkuluk yok — PAİY 38/1"]
    );
    assert_eq!(found[0]["fix"]["label"], "Serbest kenarlarına korkuluk koy");
    assert_eq!(found[0]["fix"]["step"]["command"], "cad.slabs.edit");
    apply(&mut doc, &found[0]);
    assert!(railings(&mut doc).is_empty());
}
