//! One rule, every way in (docs/adr/0443 §4, FORM-QA-09): each record the shared cases of
//! `cad.entities.set` for subtypes, value combinations and record rules judge (their expectations written
//! without KentOS code) is written again, in place of the record it edits, as a new area
//! (`cad.polygon.create`) and as a record without geometry (`cad.records.create`): a value the edit
//! refuses (a subtype's value it does not offer, a combination, a broken rule) is refused with the same
//! code. What an edit alone asks is left out: emptying a field the subtype requires (a new record is not
//! refused for it, docs/adr/0422) and what an edit lets through untouched (it judges the fields it
//! writes). The codes are the language's; the sentences may change with the language, the codes may not.

use std::collections::BTreeMap;

use kentos_domain::Document;
use kentos_commands::{CallOp, DocumentSnapshotV1};
use kentos_native_application::{ExecutionContext, registry};
use serde_json::{Value, json};

const FILES: [&str; 3] = [
    include_str!("../../../../../fixtures/commands/v1/cad.entities.set.subtypes.json"),
    include_str!("../../../../../fixtures/commands/v1/cad.entities.set.contingents.json"),
    include_str!("../../../../../fixtures/commands/v1/cad.entities.set.rules.json"),
];

fn outcome(answer: &Value) -> (String, Option<String>) {
    (
        answer["status"].as_str().unwrap_or("?").to_owned(),
        answer
            .pointer("/error/code")
            .and_then(Value::as_str)
            .map(str::to_owned),
    )
}

/// Reads a record's geometry ($alan …): only the area's way in can write it alike.
fn reads_geometry(setup: &Value) -> bool {
    setup.to_string().contains("$alan") || setup.to_string().contains("$uzunluk")
}

#[test]
fn every_way_in_refuses_a_record_with_the_same_code() {
    let mut checked = 0;
    let mut wrong = Vec::new();
    for text in FILES {
        let file: Value = serde_json::from_str(text).expect("the file");
        for case in file["cases"].as_array().expect("cases") {
            let setup = case.get("setup").unwrap_or(&file["setup"]);
            let step = &case["steps"][0];
            let input = &step["input"];
            let Some(uid) = input["uids"]
                .as_array()
                .filter(|u| u.len() == 1)
                .and_then(|u| u[0].as_str())
            else {
                continue;
            };
            if step["op"] != "execute"
                || input["operation"] != "attributes"
                || case["steps"].as_array().map_or(0, Vec::len) != 1
            {
                continue;
            }
            let Some(id) = uid
                .strip_prefix("$uidOf:")
                .and_then(|s| s.parse::<u64>().ok())
            else {
                continue;
            };
            let Some(entity) = setup["entities"]
                .as_array()
                .and_then(|es| es.iter().find(|e| e["id"] == id))
            else {
                continue;
            };
            if entity["kind"] != "polygon" {
                continue;
            }
            let mut attrs: BTreeMap<String, String> =
                serde_json::from_value(entity["attrs"].clone()).unwrap_or_default();
            for (k, v) in input["attrs"].as_object().into_iter().flatten() {
                match v.as_str() {
                    Some(s) => attrs.insert(k.clone(), s.to_owned()),
                    None => attrs.remove(k),
                };
            }
            let want = outcome(&step["result"]);
            if want.0 != "failed" || want.1.as_deref() == Some("attribute_required") {
                continue;
            }
            // The new record stands in place of the one the case edits.
            let mut setup = setup.clone();
            if let Some(es) = setup["entities"].as_array_mut() {
                es.retain(|e| e["id"] != id);
            }
            let setup = &setup;
            let layer = entity["layerId"].clone();
            let name = case["name"].as_str().unwrap_or("?");
            let mut entries = vec![(
                "cad.polygon.create",
                json!({ "layerId": layer, "pts": entity["pts"], "attrs": attrs }),
            )];
            if !reads_geometry(setup) {
                entries.push((
                    "cad.records.create",
                    json!({ "layerId": layer, "rows": [{ "attrs": attrs }] }),
                ));
            }
            for (command, input) in entries {
                let snapshot =
                    DocumentSnapshotV1::from_json(&setup.to_string()).expect("the setup");
                let mut doc = Document::from_snapshot(snapshot).expect("the setup");
                let mut cx = ExecutionContext::new(&mut doc);
                let got = registry::run(&mut cx, command, CallOp::Execute, input).expect("runs");
                checked += 1;
                if outcome(&got) != want {
                    wrong.push(format!(
                        "{name} › {command}: {:?}, the set case says {want:?}",
                        outcome(&got)
                    ));
                }
            }
        }
    }
    assert!(checked >= 20, "only {checked} checked");
    assert!(
        wrong.is_empty(),
        "{} of {checked}:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}
