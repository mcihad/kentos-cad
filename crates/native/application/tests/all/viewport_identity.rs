use kentos_native_application::{ExecutionContext, registry};
use kentos_commands::{CallOp, DocumentSnapshotV1};
use kentos_domain::{Document, Slot};
use serde_json::{Value, json};
fn command(doc: &mut Document, name: &str, input: Value) -> Value {
    let r = registry::run(
        &mut ExecutionContext::new(doc),
        name,
        CallOp::Execute,
        input,
    )
    .unwrap();
    assert_eq!(r["status"], "completed", "{name}: {r}");
    r["output"].clone()
}
#[test]
fn twenty_scoped_queries_use_original_ids_and_choose_visible_snap_before_ranking() {
    let data: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/sheet/v1/viewport-identities.json"
    ))
    .unwrap();
    for c in data["cases"].as_array().unwrap() {
        let snapshot: DocumentSnapshotV1 = serde_json::from_value(c["model"].clone()).unwrap();
        let mut doc = Document::from_snapshot(snapshot).unwrap();
        command(
            &mut doc,
            "cad.sheets.edit",
            json!({"ops":[{"op":"addSheet","sheet":c["sheet"]},c["operation"]]}),
        );
        let before = doc.to_snapshot_v2();
        let scope = json!({"sheet":"page","item":"viewport"});
        let answer = command(
            &mut doc,
            "cad.entities.select",
            json!({"modelScope":scope,"region":{"kind":"all"}}),
        );
        let expected = c["expected"]["selectedSlots"]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| {
                doc.uid(Slot(n.as_u64().unwrap() as u32))
                    .unwrap()
                    .to_string()
            })
            .collect::<Vec<_>>();
        assert_eq!(answer["uids"], json!(expected), "{}", c["name"]);
        let mut input = c["snap"].clone();
        input.as_object_mut().unwrap().remove("expectedSlot");
        input["modelScope"] = scope.clone();
        let answer = command(&mut doc, "cad.snap.find", input);
        assert_eq!(
            answer["snaps"][0]["uid"],
            doc.uid(Slot(5)).unwrap().to_string(),
            "{} {answer}",
            c["name"]
        );
        let picked = command(
            &mut doc,
            "cad.entities.pick",
            json!({"modelScope":scope,"at":answer["snaps"][0]["point"],"tolerance":c["snap"]["aperture"].as_f64().unwrap()/10.}),
        );
        assert_eq!(
            picked["picks"][0]["uid"],
            doc.uid(Slot(5)).unwrap().to_string()
        );
        let inherited = command(
            &mut doc,
            "cad.changeset.apply",
            json!({"modelScope":scope,"steps":[{"command":"cad.entities.select","input":{"region":{"kind":"all"}}}]}),
        );
        assert_eq!(inherited["steps"][0]["output"]["uids"], json!(expected));
        assert_eq!(doc.to_snapshot_v2(), before);
        assert_eq!(doc.entities().count(), 5);
        let uid = doc.uid(Slot(1)).unwrap().to_string();
        let e = doc.get(Slot(1)).unwrap();
        assert_eq!(
            kentos_geometry_core::entity::entity_length(
                &kentos_native_application::geometry::shape(e)
            ),
            Some(5.)
        );
        let normal = command(
            &mut doc,
            "cad.entities.select",
            json!({"region":{"kind":"all"}}),
        );
        assert_eq!(normal["uids"].as_array().unwrap().len(), 5);
        assert!(normal["uids"].as_array().unwrap().contains(&json!(uid)));
    }
}

#[test]
fn invalid_or_conflicting_explicit_viewport_scopes_fail_before_touching_the_model() {
    let data: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/sheet/v1/viewport-identities.json"
    ))
    .unwrap();
    let c = &data["cases"][0];
    let mut doc =
        Document::from_snapshot(serde_json::from_value(c["model"].clone()).unwrap()).unwrap();
    command(
        &mut doc,
        "cad.sheets.edit",
        json!({"ops":[{"op":"addSheet","sheet":c["sheet"]},c["operation"]]}),
    );
    let before = doc.to_snapshot_v2();
    for (name, input) in [
        ("cad.snap.find", json!({"at":{"x":0.,"y":0.},"aperture":1.})),
        ("cad.entities.select", json!({"region":{"kind":"all"}})),
        (
            "cad.entities.pick",
            json!({"at":{"x":0.,"y":0.},"tolerance":1.}),
        ),
    ] {
        for op in [CallOp::Validate, CallOp::Plan, CallOp::Execute] {
            let mut input = input.clone();
            input["modelScope"] = json!({"sheet":"page","item":"missing"});
            let r = registry::run(&mut ExecutionContext::new(&mut doc), name, op, input).unwrap();
            assert_eq!(r["error"]["code"], "scope_mismatch", "{r}");
        }
    }
    let r=registry::run(&mut ExecutionContext::new(&mut doc),"cad.changeset.apply",CallOp::Execute,json!({"modelScope":{"sheet":"page","item":"viewport"},"steps":[{"command":"cad.entities.select","input":{"region":{"kind":"all"},"modelScope":{"sheet":"other","item":"viewport"}}}]})).unwrap();
    assert_eq!(r["error"]["code"], "scope_mismatch");
    assert_eq!(doc.to_snapshot_v2(), before);
}

#[test]
fn a_layers_own_snap_switch_is_respected_by_viewport_and_model_queries() {
    let data:Value=serde_json::from_str(include_str!("../../../../../fixtures/sheet/v1/viewport-identities.json")).unwrap();let mut c=data["cases"][12].clone();c["model"]["layers"][0]["snap"]=json!({"off":true});
    let mut doc=Document::from_snapshot(serde_json::from_value(c["model"].clone()).unwrap()).unwrap();command(&mut doc,"cad.sheets.edit",json!({"ops":[{"op":"addSheet","sheet":c["sheet"]},c["operation"]]}));
    for scope in [Value::Null,json!({"sheet":"page","item":"viewport"})] {
        let result=command(&mut doc,"cad.snap.find",json!({"modelScope":scope,"at":c["snap"]["at"],"aperture":c["snap"]["aperture"],"kinds":["node"]}));assert!(result["snaps"].as_array().unwrap().is_empty());
    }
}
