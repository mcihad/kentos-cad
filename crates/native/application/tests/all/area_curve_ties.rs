//! CAD-ADV-04: exact curved areas and the identities of their split pieces.
use kentos_commands::{CallOp, DocumentSnapshotV1, Entity};
use kentos_domain::{Document, Slot, Uuid};
use kentos_native_application::{ExecutionContext, registry};
use serde_json::{Value, json};
use std::f64::consts::PI;

fn run(doc: &mut Document, name: &str, op: CallOp, input: Value) -> Value {
    let r = registry::run(&mut ExecutionContext::new(doc), name, op, input).unwrap();
    assert_eq!(r["status"], "completed", "{r}");
    r["output"].clone()
}
fn done(doc: &mut Document, name: &str, input: Value) -> Value {
    run(doc, name, CallOp::Execute, input)
}
fn capsule(id: u32, x: f64) -> Value {
    json!({"kind":"polygon","id":id,"layerId":"cizim","attrs":{"QA":"capsule"},
        "pts":[{"x":x,"y":0},{"x":x+4.,"y":0},{"x":x+4.,"y":2},{"x":x,"y":2}],
        "bulges":[0,1,0,1],"holes":[{"pts":[{"x":x+0.5,"y":0.5},{"x":x+1.5,"y":0.5},{"x":x+1.5,"y":1.5},{"x":x+0.5,"y":1.5}]}]})
}
fn drawing(two: bool) -> Document {
    let mut raw: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/interaction/v1/objects.kcad"
    ))
    .unwrap();
    raw["origin"] = json!({"x":0,"y":0});
    raw["settings"]["workspace"] = json!("cad");
    raw["entities"] = if two {
        json!([capsule(1, 0.), capsule(2, 10.)])
    } else {
        json!([capsule(1, 0.)])
    };
    Document::from_snapshot(DocumentSnapshotV1::from_json(&raw.to_string()).unwrap()).unwrap()
}
fn hatch(doc: &mut Document, x: f64) -> Uuid {
    let out = done(
        doc,
        "cad.entities.hatch",
        json!({"region":{"kind":"point","at":{"x":x,"y":1.75},"associative":true},"pattern":{"name":"ANSI31","angle":30.,"scale":2.,"origin":{"kind":"point","at":{"x":0.123,"y":0.456}}},"layerId":"cizim"}),
    );
    Uuid::parse_str(out["created"][0].as_str().unwrap()).unwrap()
}
fn get(doc: &Document, uid: Uuid) -> &Entity {
    doc.get(doc.slot_of(uid).unwrap()).unwrap()
}
fn area(e: &Entity) -> f64 {
    use kentos_geometry_core::api::json::{FromJson, Json};
    let c = kentos_geometry_core::entity::Entity::from_json(
        &Json::parse(&serde_json::to_string(e).unwrap()).unwrap(),
    )
    .unwrap();
    kentos_geometry_core::entity::entity_area(&c.shape).unwrap()
}
#[test]
fn curved_holed_split_retains_analytic_area_hatches_and_point_identity_in_one_undo() {
    let mut doc = drawing(false);
    let source = doc.uid(Slot(1)).unwrap();
    let left = hatch(&mut doc, 1.);
    let right = hatch(&mut doc, 3.);
    let dim = done(
        &mut doc,
        "cad.entities.create",
        json!({"layerId":"cizim","objects":[{"geometry":{"kind":"dimension","a":{"x":4,"y":0},"b":{"x":4,"y":2},"height":0.1,"offset":0.2}}]}),
    );
    let dim = dim["created"][0].as_str().unwrap().to_owned();
    done(
        &mut doc,
        "cad.dimensions.reassociate",
        json!({"uid":dim,"points":[
        {"point":"a","to":{"uid":source,"place":{"kind":"vertex","part":0,"ring":0,"index":1}}},
        {"point":"b","to":{"uid":source,"place":{"kind":"vertex","part":0,"ring":0,"index":2}}}]}),
    );
    let patterns: Vec<_> = [left, right].iter().map(|u| match get(&doc, *u) {
        Entity::Hatch(h) => h.pattern.clone(), _ => unreachable!(),
    }).collect();
    let before = doc.to_snapshot_v2();
    let input =
        json!({"work":{"kind":"split","ids":[source],"line":[{"x":2,"y":-1},{"x":2,"y":3}]}});
    let plan = run(&mut doc, "cad.areas.edit", CallOp::Plan, input.clone());
    assert_eq!(doc.to_snapshot_v2(), before);
    assert_eq!(plan["changed"].as_array().unwrap().len(), 1);
    let out = done(&mut doc, "cad.areas.edit", input);
    let new = Uuid::parse_str(out["created"][0].as_str().unwrap()).unwrap();
    assert!((out["size"].as_f64().unwrap() - (7. + PI)).abs() < 1e-12);
    assert!((area(get(&doc, source)) + area(get(&doc, new)) - (7. + PI)).abs() < 1e-12);
    for (uid, outer, holes) in [(left, source, 1), (right, new, 0)] {
        let Entity::Hatch(h) = get(&doc, uid) else {
            panic!("hatch")
        };
        assert_eq!(
            h.assoc.as_ref().unwrap().outer.unwrap().0,
            *outer.as_bytes()
        );
        assert_eq!(h.holes.as_ref().map_or(0, Vec::len), holes);
        let index = usize::from(uid == right);
        assert_eq!(h.pattern, patterns[index], "splitting the source must not reset pattern origin or angle");
        let expected = if index == 0 { 3. + PI / 2. } else { 4. + PI / 2. };
        assert!((area(get(&doc, uid)) - expected).abs() < 1e-9);
        let b = h.boundary.as_ref().unwrap();
        assert!(serde_json::to_string(b).unwrap().contains("bulges"));
    }
    let Entity::Dimension(d) = get(&doc, Uuid::parse_str(&dim).unwrap()) else {
        panic!("dimension")
    };
    for r in [
        d.refs.as_ref().unwrap().a.as_ref().unwrap(),
        d.refs.as_ref().unwrap().b.as_ref().unwrap(),
    ] {
        assert!(r.live());
        assert_eq!(r.uid.0, *new.as_bytes());
    }
    assert_eq!(d.a, kentos_commands::Vec2 { x: 4., y: 0. });
    assert_eq!(d.b, kentos_commands::Vec2 { x: 4., y: 2. });
    assert_eq!(get(&doc, new).base().attrs, get(&doc, source).base().attrs);
    assert_eq!(doc.undo().as_deref(), Some("Alan böl"));
    assert_eq!(doc.to_snapshot_v2(), before);
}
#[test]
fn splitting_two_sources_never_sends_a_hatch_to_another_sources_piece() {
    let mut doc = drawing(true);
    let sources = [doc.uid(Slot(1)).unwrap(), doc.uid(Slot(2)).unwrap()];
    let hatches = [hatch(&mut doc, 3.), hatch(&mut doc, 13.)];
    let before = doc.to_snapshot_v2();
    let out = done(
        &mut doc,
        "cad.areas.edit",
        json!({"work":{"kind":"split","ids":sources,"line":[{"x":-2,"y":1.75},{"x":16,"y":1.75}]}}),
    );
    assert_eq!(out["pieces"], 4);
    for (i, h) in hatches.into_iter().enumerate() {
        let Entity::Hatch(h) = get(&doc, h) else {
            panic!("hatch")
        };
        let outer = Uuid::from_bytes(h.assoc.as_ref().unwrap().outer.unwrap().0);
        let own_new = Uuid::parse_str(out["created"][i].as_str().unwrap()).unwrap();
        assert!([sources[i], own_new].contains(&outer));
    }
    assert_eq!(doc.undo().as_deref(), Some("Alan böl"));
    assert_eq!(doc.to_snapshot_v2(), before);
}

#[test]
fn breaking_a_curved_multipart_path_keeps_other_part_and_rebinds_its_index() {
    let mut raw: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/interaction/v1/objects.kcad"
    ))
    .unwrap();
    raw["origin"] = json!({"x":0,"y":0});
    raw["settings"]["workspace"] = json!("cad");
    raw["entities"] = json!([
        {"kind":"polyline","id":1,"layerId":"cizim","attrs":{"QA":"multi"},"pts":[{"x":0,"y":0},{"x":2,"y":0}],"bulges":[1,0],"parts":[{"pts":[{"x":0,"y":10},{"x":4,"y":10}]}]},
        {"kind":"dimension","id":2,"layerId":"cizim","attrs":{},"a":{"x":0,"y":10},"b":{"x":4,"y":10},"height":0.1,"offset":0.2}
    ]);
    let mut doc =
        Document::from_snapshot(DocumentSnapshotV1::from_json(&raw.to_string()).unwrap()).unwrap();
    let source = doc.uid(Slot(1)).unwrap();
    let dim = doc.uid(Slot(2)).unwrap();
    done(
        &mut doc,
        "cad.dimensions.reassociate",
        json!({"uid":dim,"points":[
        {"point":"a","to":{"uid":source,"place":{"kind":"vertex","part":1,"ring":0,"index":0}}},
        {"point":"b","to":{"uid":source,"place":{"kind":"vertex","part":1,"ring":0,"index":1}}}]}),
    );
    let before = doc.to_snapshot_v2();
    let out = done(
        &mut doc,
        "cad.entities.break",
        json!({"uid":source,"first":{"x":1,"y":-1}}),
    );
    assert!(out["created"].as_array().unwrap().is_empty());
    let Entity::Polyline(p) = get(&doc, source) else {
        panic!("multipart path")
    };
    let parts = p.parts.as_ref().unwrap();
    assert_eq!(parts.len(), 2);
    assert_eq!(
        parts[1].pts,
        vec![
            kentos_commands::Vec2 { x: 0., y: 10. },
            kentos_commands::Vec2 { x: 4., y: 10. }
        ]
    );
    let length = area_length(get(&doc, source));
    assert!((length - (PI + 4.)).abs() < 1e-12);
    let Entity::Dimension(d) = get(&doc, dim) else {
        panic!("dimension")
    };
    for r in [
        d.refs.as_ref().unwrap().a.as_ref().unwrap(),
        d.refs.as_ref().unwrap().b.as_ref().unwrap(),
    ] {
        assert!(r.live());
        assert!(matches!(
            r.place,
            kentos_commands::SubPlace::Vertex { part: 2, .. }
        ));
    }
    assert_eq!(doc.undo().as_deref(), Some("Kır"));
    assert_eq!(doc.to_snapshot_v2(), before);
}
fn area_length(e: &Entity) -> f64 {
    use kentos_geometry_core::api::json::{FromJson, Json};
    let c = kentos_geometry_core::entity::Entity::from_json(
        &Json::parse(&serde_json::to_string(e).unwrap()).unwrap(),
    )
    .unwrap();
    kentos_geometry_core::entity::entity_length(&c.shape).unwrap()
}
