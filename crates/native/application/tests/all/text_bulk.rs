//! CAD-ADV-10: atomic bulk edits preserve physical rows and explicit profiles.
use kentos_commands::{CallOp, DocumentSnapshotV1, Entity};
use kentos_domain::Document;
use kentos_native_application::{ExecutionContext, registry};
use serde_json::{Value, json};

fn drawing(mut texts: Vec<Value>) -> Document {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/commands/v1/cad.blocks.find.json"
    ))
    .unwrap();
    let mut raw = fixture["setup"].clone();
    for (i, t) in texts.iter_mut().enumerate() {
        t["kind"] = json!("text");
        t["id"] = json!(i + 1);
        t["layerId"] = json!("a");
        t["attrs"] = json!({});
        if t.get("p").is_none() {
            t["p"] = json!({"x":0.,"y":-(i as f64)});
        }
        if t.get("height").is_none() {
            t["height"] = json!(0.3);
        }
        if t.get("rotation").is_none() {
            t["rotation"] = json!(0.);
        }
    }
    raw["entities"] = json!(texts);
    Document::from_snapshot(DocumentSnapshotV1::from_json(&raw.to_string()).unwrap()).unwrap()
}
fn ids(doc: &Document) -> Vec<String> {
    doc.entities_with_uids()
        .map(|(u, _)| u.to_string())
        .collect()
}
fn call(doc: &mut Document, op: CallOp, edit: Value) -> Value {
    registry::run(
        &mut ExecutionContext::new(doc),
        "cad.texts.edit",
        op,
        json!({"edit":edit}),
    )
    .unwrap()
}
fn refused(doc: &mut Document, edit: Value) {
    let before = doc.to_snapshot_v2();
    let undo = doc.undo_labels();
    for op in [CallOp::Validate, CallOp::Plan, CallOp::Execute] {
        let r = call(doc, op, edit.clone());
        assert_eq!(r["status"], "failed", "{r}");
        assert_eq!(r["error"]["code"], "invalid_input", "{r}");
        assert_eq!(doc.to_snapshot_v2(), before);
        assert_eq!(doc.undo_labels(), undo);
    }
}
#[test]
fn partial_combining_character_match_refuses_the_whole_batch() {
    let mut doc = drawing(vec![
        json!({"text":"g plain"}),
        json!({"text":"Ög\u{306}ç","layout":{"engine":"unicode-v1"}}),
    ]);
    let u = ids(&doc);
    refused(
        &mut doc,
        json!({"kind":"replace","uids":u,"find":"g","with":"h","matchCase":true,"wholeWord":false}),
    );
}
#[test]
fn unicode_bulk_rejects_unrenderable_proposed_content() {
    for kind in ["replace", "number"] {
        let mut doc = drawing(vec![
            json!({"text":"Ölçü","layout":{"engine":"unicode-v1"}}),
        ]);
        let u = ids(&doc);
        let edit = if kind == "replace" {
            json!({"kind":kind,"uids":u,"find":"Ölçü","with":"🛸","matchCase":true,"wholeWord":false})
        } else {
            json!({"kind":kind,"uids":u,"start":1,"step":1,"prefix":"🛸","suffix":"","place":"replace"})
        };
        refused(&mut doc, edit);
    }
}
#[test]
fn unicode_combining_profiles_or_base_formats_is_an_explicit_refusal() {
    for patch in [
        json!({"layout":null}),
        json!({"font":"barlow"}),
        json!({"height":0.5}),
        json!({"color":"#FF0000"}),
        json!({"boxWidth":3.}),
        json!({"rotation":30.}),
    ] {
        let a = json!({"text":"Ög\u{306}ç","layout":{"engine":"unicode-v1"}});
        let mut b = json!({"text":"AV","layout":{"engine":"unicode-v1"}});
        for (k, v) in patch.as_object().unwrap() {
            if v.is_null() {
                b.as_object_mut().unwrap().remove(k);
            } else {
                b[k] = v.clone();
            }
        }
        let mut doc = drawing(vec![a, b]);
        let u = ids(&doc);
        refused(&mut doc, json!({"kind":"combine","uids":u}));
    }
}
#[test]
fn compatible_unicode_combining_keeps_runs_and_is_one_undo() {
    let mut doc = drawing(vec![
        json!({"text":"Ög\u{306}ç","layout":{"engine":"unicode-v1"},"runs":[{"start":0,"end":4,"bold":true}]}),
        json!({"text":"AV","layout":{"engine":"unicode-v1"},"runs":[{"start":0,"end":2,"italic":true}]}),
    ]);
    let before = doc.to_snapshot_v2();
    let u = ids(&doc);
    let edit = json!({"kind":"combine","uids":u});
    let plan = call(&mut doc, CallOp::Plan, edit.clone());
    assert_eq!(plan["status"], "completed", "{plan}");
    assert_eq!(doc.to_snapshot_v2(), before);
    let r = call(&mut doc, CallOp::Execute, edit);
    assert_eq!(r["status"], "completed", "{r}");
    let Entity::Text(t) = doc.entities().next().unwrap() else {
        panic!()
    };
    assert_eq!(t.text, "Ög\u{306}ç\nAV");
    assert_eq!(t.paragraph.runs[1].start, 5);
    assert_eq!(t.paragraph.runs[1].end, 7);
    assert!(t.paragraph.runs[1].italic);
    assert_eq!(doc.entities().count(), 1);
    assert_eq!(doc.undo().as_deref(), Some("Yazıları birleştir"));
    assert_eq!(doc.to_snapshot_v2(), before);
}
#[test]
fn split_uses_physical_baselines_not_the_number_of_tab_pieces() {
    for engine in [None, Some("unicode-v1")] {
        let mut text = json!({"text":"A\tB\nC\tD\nE\tF","height":0.3,"rotation":30.,"align":"bottomLeft",
            "boxWidth":4.,"p":{"x":5000000.,"y":4400000.},"layout":{"tabs":[1.5]}});
        if let Some(e) = engine {
            text["layout"]["engine"] = json!(e);
        }
        let mut doc = drawing(vec![text]);
        let before = doc.to_snapshot_v2();
        let u = ids(&doc);
        let r = call(
            &mut doc,
            CallOp::Execute,
            json!({"kind":"split","uid":u[0]}),
        );
        assert_eq!(r["status"], "completed", "{r}");
        let texts: Vec<_> = doc
            .entities()
            .filter_map(|e| {
                if let Entity::Text(t) = e {
                    Some(t)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(texts.len(), 3);
        // bottom alignment means original first baseline is 1.06 m above p;
        // each new top point is another 0.3 m above its baseline.
        for (i, t) in texts.iter().enumerate() {
            let d = 1.36 - 0.5 * i as f64;
            assert!(
                (t.p.x - (5000000. - 0.5 * d)).abs() < 2e-9,
                "{engine:?}: {t:?}"
            );
            assert!(
                (t.p.y - (4400000. + 3f64.sqrt() / 2. * d)).abs() < 2e-9,
                "{engine:?}: {t:?}"
            );
            assert_eq!(t.text, ["A\tB", "C\tD", "E\tF"][i]);
        }
        assert_eq!(doc.undo().as_deref(), Some("Satırlara ayır"));
        assert_eq!(doc.to_snapshot_v2(), before);
    }
}
#[test]
fn font_copy_keeps_the_target_profile_and_refuses_a_new_invalid_cluster_paint() {
    let mut doc = drawing(vec![
        json!({"text":"Source","font":"barlow"}),
        json!({"text":"office","font":"arimo","layout":{"engine":"unicode-v1"},"runs":[{"start":2,"end":3,"color":"#FF0000"}]}),
    ]);
    let u = ids(&doc);
    let before = doc.to_snapshot_v2();
    for op in [CallOp::Validate, CallOp::Plan, CallOp::Execute] {
        let r = registry::run(
            &mut ExecutionContext::new(&mut doc),
            "cad.entities.match",
            op,
            json!({"source":u[0],"uids":[u[1]],"properties":["text"]}),
        )
        .unwrap();
        assert_eq!(r["status"], "failed", "{r}");
        assert_eq!(doc.to_snapshot_v2(), before);
    }
}

#[test]
fn positional_bulk_does_not_refresh_stale_fields_while_measuring() {
    for kind in ["height", "split", "combine"] {
        let mut doc = drawing(vec![
            json!({"text":"Eski\nAV","layout":{"engine":"unicode-v1"},
            "fields":[{"start":0,"end":4,"source":{"kind":"project","value":"name"}}]}),
            json!({"text":"Ölçü","layout":{"engine":"unicode-v1"}}),
        ]);
        let u = ids(&doc);
        let edit = match kind {
            "height" => json!({"kind":kind,"uids":u,"height":0.5}),
            "split" => json!({"kind":kind,"uid":u[0]}),
            _ => json!({"kind":kind,"uids":u}),
        };
        refused(&mut doc, edit);
    }
}
#[test]
fn valid_unicode_replace_is_previewed_without_writing_and_undoes_once() {
    let mut doc = drawing(vec![
        json!({"text":"Ög\u{306}ç — AV","layout":{"engine":"unicode-v1"},
        "runs":[{"start":0,"end":4,"bold":true}]}),
    ]);
    let before = doc.to_snapshot_v2();
    let u = ids(&doc);
    let edit =
        json!({"kind":"replace","uids":u,"find":"Ög\u{306}ç","with":"İstanbul","matchCase":true});
    let plan = call(&mut doc, CallOp::Plan, edit.clone());
    assert_eq!(plan["status"], "completed", "{plan}");
    assert_eq!(doc.to_snapshot_v2(), before);
    let result = call(&mut doc, CallOp::Execute, edit);
    assert_eq!(result["status"], "completed", "{result}");
    let t = doc.entities().next().unwrap();
    assert_eq!(serde_json::to_value(t).unwrap(), plan["output"]["texts"][0]);
    let Entity::Text(t) = t else { panic!() };
    assert_eq!(t.text, "İstanbul — AV");
    assert_eq!(t.paragraph.runs[0].end, 8);
    assert!(t.paragraph.runs[0].bold);
    assert_eq!(doc.undo().as_deref(), Some("Bul ve değiştir"));
    assert_eq!(doc.to_snapshot_v2(), before);
}
#[test]
fn wildcard_and_property_commands_cannot_bypass_unicode_result_validation() {
    for operation in ["replaceText", "properties"] {
        let mut doc = drawing(vec![
            json!({"text":"Ölçü","layout":{"engine":"unicode-v1"}}),
        ]);
        let u = ids(&doc);
        let before = doc.to_snapshot_v2();
        for op in [CallOp::Validate, CallOp::Plan, CallOp::Execute] {
            let r=registry::run(&mut ExecutionContext::new(&mut doc),"cad.entities.edit",op,
                json!({"operation":operation,"changes":[{"kind":"update","uid":u[0],"geometry":{
                    "kind":"text","p":{"x":0.,"y":0.},"text":"🛸","height":0.3,"rotation":0.,"layout":{"engine":"unicode-v1"}}}]})).unwrap();
            assert_eq!(r["status"], "failed", "{r}");
            assert_eq!(r["error"]["code"], "invalid_input", "{r}");
            assert_eq!(doc.to_snapshot_v2(), before);
        }
    }
}
