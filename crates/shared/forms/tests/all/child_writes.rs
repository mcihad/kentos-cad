//! What a Veri girişi may do with a record's children and write of a record holds to the shared cases the
//! independent reference writes (scripts/fixtures/child_writes_cases.py, fixtures/forms/v1/child-writes.json;
//! docs/adr/0764): each offered action's refusal, and each change's refusal.

use std::collections::BTreeMap;

use kentos_contracts::{LayerField, ProjectForm, RelatedAction};
use kentos_expression::Unit;
use kentos_forms::reach::{Caller, Reach};
use kentos_forms::{
    Attrs, ChildEntry, ChildPermits, EntryRefusal, entry_write_refusal, related_offer,
};
use serde_json::Value;

const CASES: &str = include_str!("../../../../../fixtures/forms/v1/child-writes.json");

struct World {
    fields: BTreeMap<String, Vec<LayerField>>,
    forms: Vec<ProjectForm>,
}

impl World {
    fn read(doc: &Value) -> World {
        World {
            fields: serde_json::from_value(doc["fields"].clone()).expect("the fields"),
            forms: serde_json::from_value(doc["forms"].clone()).expect("the forms"),
        }
    }

    fn reach(&self) -> Reach {
        Reach::new(
            &self.forms,
            |l| self.fields.get(l).map_or(&[][..], Vec::as_slice),
            &[],
            Unit::METRE,
            Caller::through_forms(),
        )
    }
}

fn attrs(v: &Value) -> Attrs {
    serde_json::from_value(v.clone()).expect("attributes")
}

/// The kind of refusal a message is: what the reference names.
fn kind(message: &str, record: Option<&str>) -> &'static str {
    if message.starts_with("Önce kaydı kaydedin") {
        "unsaved"
    } else if Some(message) == record {
        "record"
    } else if message.contains("katmanı kilitli") {
        "childLocked"
    } else if message.contains("rolünüz nesne yazmaya") {
        "noWrites"
    } else if message.contains("oluşturamaz ya da silemez") {
        "attributes"
    } else if message.contains("yayımlanmış formu yok") {
        "noForm"
    } else if message.contains("formunda yok") {
        "fieldHidden"
    } else {
        panic!("an unknown refusal: {message}")
    }
}

#[test]
fn child_writes_hold_to_the_shared_cases() {
    let doc: Value = serde_json::from_str(CASES).expect("the cases");
    let world = World::read(&doc);
    let reach = world.reach();
    let mut offers = 0;
    for case in doc["offers"].as_array().expect("offers") {
        let name = case["name"].as_str().unwrap();
        let p = &case["permits"];
        let text = |k: &str| p.get(k).and_then(Value::as_str).map(str::to_owned);
        // The child as the reference saw it, and as the rule makes it from the same forms.
        let child = p.get("child").map(|c| {
            let given = ChildEntry {
                layer: c["layer"].as_str().unwrap().to_owned(),
                field: c["field"].as_str().unwrap().to_owned(),
                form: c["form"].as_bool().unwrap(),
                shows_field: c["showsField"].as_bool().unwrap(),
            };
            assert_eq!(
                ChildEntry::of(&reach, &given.layer, &given.layer, &given.field),
                given,
                "{name}: the child entry"
            );
            given
        });
        let permits = ChildPermits {
            saved: p["saved"].as_bool().unwrap(),
            record_locked: text("recordLocked"),
            child_locked: text("childLocked"),
            no_writes: p.get("noWrites").and_then(Value::as_bool).unwrap_or(false),
            attributes_only: p
                .get("attributesOnly")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            child,
        };
        let actions: Option<Vec<RelatedAction>> =
            serde_json::from_value(case["actions"].clone()).expect("actions");
        let got: Vec<(RelatedAction, Option<&'static str>)> =
            related_offer(actions.as_deref(), &permits)
                .into_iter()
                .map(|o| {
                    let k = o
                        .refused
                        .as_deref()
                        .map(|m| kind(m, permits.record_locked.as_deref()));
                    (o.action, k)
                })
                .collect();
        let want: Vec<(RelatedAction, Option<&'static str>)> = case["expect"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| {
                let a: RelatedAction = serde_json::from_value(e["action"].clone()).unwrap();
                let k = e["refused"].as_str().map(|s| match s {
                    "unsaved" => "unsaved",
                    "record" => "record",
                    "childLocked" => "childLocked",
                    "noWrites" => "noWrites",
                    "attributes" => "attributes",
                    "noForm" => "noForm",
                    "fieldHidden" => "fieldHidden",
                    other => panic!("{other}"),
                });
                (a, k)
            })
            .collect();
        assert_eq!(got, want, "{name}");
        offers += 1;
    }
    let mut writes = 0;
    for case in doc["writes"].as_array().expect("writes") {
        let name = case["name"].as_str().unwrap();
        let layer = case["layer"].as_str().unwrap();
        let got = entry_write_refusal(
            &reach,
            layer,
            &attrs(&case["before"]),
            &attrs(&case["after"]),
        );
        let want = match case["expect"].get("problem").and_then(Value::as_str) {
            None => None,
            Some("noForm") => Some(EntryRefusal::NoForm),
            Some("hidden") => Some(EntryRefusal::Hidden(
                case["expect"]["field"].as_str().unwrap().to_owned(),
            )),
            Some(other) => panic!("{other}"),
        };
        assert_eq!(got, want, "{name}");
        if let Some(r) = got {
            assert!(r.message(layer).contains(layer), "{name}: names the layer");
        }
        writes += 1;
    }
    assert!(
        offers >= 20 && writes >= 20,
        "{offers} offers, {writes} writes"
    );
}
