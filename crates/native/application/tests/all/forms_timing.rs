//! Forms over a large layer (plan FORM-QA-15, docs/adr/0424): what a big form costs to prepare, a keystroke to assess, and
//! the layer-wide commands over 100 000 records with subtypes and a value group of 2 000 rows. Each figure has a budget
//! the test holds, so that a slower change is seen.
//!
//! `cargo test --release -p kentos-native-application --test all forms_timing -- --ignored --nocapture --test-threads=1`

use std::time::Instant;

use kentos_domain::Document;
use kentos_commands::{
    CommandResult, DocumentSnapshotV1, EntitiesSetProperties, FormsAudit, FormsRecompute,
    FormsValidate,
};
use kentos_native_application::{
    ExecutionContext, forms_audit, forms_recompute, forms_validate, set,
};
use serde_json::{Value, json};

const RECORDS: u32 = 100_000;
const KINDS: usize = 10;
const MATERIALS: usize = 40;
const ROOFS: usize = 50;

fn fields() -> Value {
    let list = |stem: &str, n: usize| -> Value {
        (0..n)
            .map(|i| json!({"code": format!("{stem}{i}"), "label": format!("{stem} {i}")}))
            .collect()
    };
    let mut out = vec![
        json!({"name": "Tür", "kind": "text", "values": list("t", KINDS)}),
        json!({"name": "Malzeme", "kind": "text", "values": list("m", MATERIALS)}),
        json!({"name": "Çatı", "kind": "text", "values": list("c", ROOFS)}),
        json!({"name": "Kat", "kind": "integer", "min": "0", "max": "200"}),
        json!({"name": "Yükseklik", "kind": "decimal", "scale": 2}),
        json!({"name": "Not", "kind": "text", "length": 500}),
    ];
    for i in 0..24 {
        out.push(json!({"name": format!("A{i}"), "kind": "text", "length": 64}));
    }
    Value::Array(out)
}

/// A form of 200 items: the six fields, twenty-four texts each shown by a rule, notes and checks.
fn body() -> Value {
    let mut items = vec![
        json!({"type": "field", "id": "tur", "field": "Tür", "required": true}),
        json!({"type": "field", "id": "malzeme", "field": "Malzeme"}),
        json!({"type": "field", "id": "cati", "field": "Çatı"}),
        json!({"type": "field", "id": "kat", "field": "Kat", "checks": [{"rule": "Kat < 150", "message": "Çok yüksek."}]}),
        json!({"type": "field", "id": "yukseklik", "field": "Yükseklik", "calculate": "Kat * 3"}),
        json!({"type": "field", "id": "not", "field": "Not", "visibleWhen": "Kat > 2"}),
    ];
    for i in 0..24 {
        items.push(
            json!({"type": "field", "id": format!("a{i}"), "field": format!("A{i}"),
                          "visibleWhen": format!("Kat > {}", i % 7), "requiredWhen": "Tür = 't1'"}),
        );
    }
    let mut group = Vec::new();
    for i in 0..170 {
        group.push(json!({"type": "note", "id": format!("n{i}"), "text": format!("Not {i}: {{Kat * {i}}} kat.")}));
    }
    items.push(json!({"type": "group", "id": "notlar", "label": "Notlar", "items": group}));
    json!({"revision": 1, "items": items})
}

fn subtypes() -> Value {
    let types: Vec<Value> = (0..KINDS)
        .map(|k| {
            let keep: Vec<String> = (0..MATERIALS)
                .filter(|m| m % KINDS != k)
                .map(|m| format!("m{m}"))
                .collect();
            json!({"code": format!("t{k}"), "fields": [{"field": "Malzeme", "values": keep}]})
        })
        .collect();
    json!({"field": "Tür", "types": types})
}

/// 2 000 rows: every material with forty of the fifty roofs, and a material with any roof.
fn group() -> Value {
    let mut rows = Vec::new();
    for m in 0..MATERIALS {
        for c in 0..ROOFS {
            if rows.len() < 1_999 && (m + c) % 5 != 0 {
                rows.push(json!({"cells": [{"kind": "code", "code": format!("m{m}")}, {"kind": "code", "code": format!("c{c}")}]}));
            }
        }
    }
    rows.push(json!({"cells": [{"kind": "code", "code": "m0"}, {"kind": "any"}]}));
    json!([{"name": "Malzeme ve çatı", "fields": ["Malzeme", "Çatı"], "restrictive": true, "rows": rows}])
}

fn drawing() -> Document {
    let mut entities = Vec::with_capacity(RECORDS as usize);
    for i in 0..RECORDS {
        let (x, y) = (f64::from(i % 300) * 30.0, f64::from(i / 300) * 30.0);
        let kind = i as usize % KINDS;
        let material = (0..MATERIALS)
            .find(|m| m % KINDS != kind)
            .map_or(1, |m| (m + i as usize) % MATERIALS);
        let material = if material % KINDS == kind {
            (material + 1) % MATERIALS
        } else {
            material
        };
        let roof = (0..ROOFS).find(|c| (material + c) % 5 != 0).unwrap_or(1);
        entities.push(json!({"kind": "polygon", "id": i + 1, "layerId": "bina",
            "pts": [{"x": x, "y": y}, {"x": x + 20.0, "y": y}, {"x": x + 20.0, "y": y + 20.0}, {"x": x, "y": y + 20.0}],
            "attrs": {"Tür": format!("t{kind}"), "Malzeme": format!("m{material}"), "Çatı": format!("c{roof}"),
                      "Kat": (i % 9).to_string(), "A1": "x"}}));
    }
    let b = body();
    let typed: kentos_commands::FormBody =
        serde_json::from_value(b.clone()).expect("a body");
    let form = json!({"id": "bina", "name": "Bina", "layer": "bina", "draft": b.clone(),
        "releases": [{"version": 1, "digest": kentos_commands::body_digest(&typed), "body": b}]});
    let snapshot = json!({"format":"kentos.document","version":1,"name":"Büyük form","settings":{"srid":0,
        "lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"deg","plotScale":100,"forms":[form]},
        "origin":{"x":0,"y":0},"layers":[{"id":"bina","name":"Bina","type":"layer","visible":true,"locked":false,
        "expanded":true,"style":{"color":"fg","lineType":"continuous","lineWeight":0.18},"children":[],
        "fields": fields(), "subtypes": subtypes(), "contingents": group()}],
        "activeLayer":"bina","entities":entities,"styles":{"items":[],"categories":[]}});
    Document::from_snapshot(DocumentSnapshotV1::from_json(&snapshot.to_string()).expect("reads"))
        .expect("opens")
}

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1e3
}

#[test]
#[ignore = "a timing, run on purpose"]
fn forms_timing() {
    let t = Instant::now();
    let mut doc = drawing();
    println!("{RECORDS} kayıtlı çizim {:.0} ms'de kuruldu", ms(t));
    let uid = doc
        .uid(kentos_domain::Slot(1))
        .expect("an object")
        .to_string();

    // One record through the form, as the editor's first look does: the form prepared, the record assessed.
    let ask = |values: &[(&str, &str)]| FormsValidate {
        form: "bina".into(),
        draft: false,
        uid: Some(uid.clone()),
        values: values
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect(),
        defaults: false,
        moment: None,
        user: None,
    };
    let t = Instant::now();
    let rounds = 50;
    for i in 0..rounds {
        let kat = (i % 9).to_string();
        let CommandResult::Completed { .. } =
            forms_validate::execute(&mut ExecutionContext::new(&mut doc), ask(&[("Kat", &kat)]))
        else {
            panic!("not completed")
        };
    }
    let validate = ms(t) / f64::from(rounds);

    // A keystroke in the editor: the prepared form assessing one record.
    let prepared = kentos_native_application::forms::prepared_in(&doc, "bina").expect("prepared");
    let before = doc
        .get(kentos_domain::Slot(1))
        .expect("an object")
        .base()
        .attrs
        .clone();
    let object = kentos_forms::Object::default();
    let t = Instant::now();
    let keys = 2_000;
    for i in 0..keys {
        let kat = (i % 9).to_string();
        let _ = prepared.assess(&before, &[("Kat".to_owned(), kat)], &object, false);
        let _ = prepared.described(&before, &object);
    }
    let keystroke = ms(t) / f64::from(keys);

    let t = Instant::now();
    let CommandResult::Completed {
        output: audited, ..
    } = forms_audit::execute(
        &mut ExecutionContext::new(&mut doc),
        FormsAudit {
            form: "bina".into(),
            draft: false,
            warnings: false,
            limit: None,
            moment: None,
            user: None,
        },
    )
    else {
        panic!("not completed")
    };
    let audit = ms(t);

    let t = Instant::now();
    let CommandResult::Completed {
        output: planned, ..
    } = forms_recompute::plan(
        &ExecutionContext::new(&mut doc),
        &FormsRecompute {
            form: "bina".into(),
            draft: false,
            moment: None,
            user: None,
            expected_revision: None,
        },
    )
    else {
        panic!("not completed")
    };
    let recompute = ms(t);

    // 10 000 records written at once: the subtypes' and the value group's rules for each.
    let uids: Vec<String> = (1..=10_000u32)
        .filter_map(|s| doc.uid(kentos_domain::Slot(s)).map(|u| u.to_string()))
        .collect();
    let input: EntitiesSetProperties = serde_json::from_value(json!({
        "uids": uids, "operation": "attributes", "attrs": {"Not": "Toplu not"}
    }))
    .expect("an input");
    let t = Instant::now();
    let result = set::execute(&mut ExecutionContext::new(&mut doc), input);
    let written = ms(t);
    assert!(
        matches!(result, CommandResult::Completed { .. }),
        "{result:?}"
    );

    println!(
        "kayıt formla {validate:.2} ms; tuş {keystroke:.3} ms; Kayıtları denetle {audit:.0} ms ({} kayıt); \
         Hesapları yenile planı {recompute:.0} ms ({} değişecek); 10 000 kayda yazma {written:.0} ms",
        audited.checked,
        planned.changed.len()
    );
    // Budgets about twice the figures measured on the owner's machine (17 October): a slower change shows.
    assert!(keystroke < 0.5, "bir tuş {keystroke:.3} ms; bütçe 0,5 ms");
    assert!(validate < 5.0, "kayıt formla {validate:.2} ms; bütçe 5 ms");
    assert!(audit < 1_800.0, "denetim {audit:.0} ms; bütçe 1,8 s");
    assert!(recompute < 800.0, "yenileme {recompute:.0} ms; bütçe 0,8 s");
    assert!(
        written < 400.0,
        "10 000 kayda yazma {written:.0} ms; bütçe 0,4 s"
    );
}
