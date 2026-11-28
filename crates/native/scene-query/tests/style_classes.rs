//! A layer style's classes as the layer tree lists them (docs/adr/9105 §2): their ids, labels and conditions, which
//! objects each takes and how many, and switching classes on and off in the renderer's JSON.

use kentos_commands::Entity;
use kentos_expression::Unit;
use kentos_scene_query::style_classes::{
    ClassReader, OTHER, Switch, SwitchRefused, set_enabled, style_classes,
};
use serde_json::{Value, json};

/// Points with a use and an area (a missing value left out); a text, which the style engine does not draw, so it
/// counts nowhere.
fn objects() -> Vec<Entity> {
    let rows = [
        ("Konut", "120"),
        ("Konut", "480"),
        ("Ticaret", "900"),
        ("Park", "1500"),
        ("", "60"),
        ("Sanayi", ""),
    ];
    let mut out: Vec<Entity> = rows
        .iter()
        .enumerate()
        .map(|(i, (k, a))| {
            let mut attrs = serde_json::Map::new();
            if !k.is_empty() {
                attrs.insert("kullanim".into(), json!(k));
            }
            if !a.is_empty() {
                attrs.insert("alan".into(), json!(a));
            }
            serde_json::from_value(json!({
                "kind": "point", "id": i + 1, "layerId": "p", "attrs": attrs,
                "p": { "x": 487000.0 + i as f64, "y": 4420000.0 }
            }))
            .expect("a point")
        })
        .collect();
    out.push(
        serde_json::from_value(json!({
            "kind": "text", "id": 99, "layerId": "p", "attrs": { "kullanim": "Konut" },
            "p": { "x": 0.0, "y": 0.0 }, "text": "yazı", "height": 2.5, "rotation": 0
        }))
        .expect("a text"),
    );
    out
}

fn name(_: &str) -> String {
    "Parseller".to_owned()
}

fn counts(renderer: &Value) -> Vec<u64> {
    let list = objects();
    let refs: Vec<&Entity> = list.iter().collect();
    let reader = ClassReader::new(renderer, Unit::METRE).expect("classes");
    reader
        .counts(&refs, &name, &|_| None)
        .into_iter()
        .map(|c| c.expect("a count"))
        .collect()
}

fn members(renderer: &Value, at: usize) -> Vec<bool> {
    let list = objects();
    let refs: Vec<&Entity> = list.iter().collect();
    let reader = ClassReader::new(renderer, Unit::METRE).expect("classes");
    reader
        .members(&refs, &name, &|_| None, at)
        .expect("members")
}

fn categorized() -> Value {
    json!({
        "type": "categorized", "expr": "kullanim",
        "categories": [
            { "value": "Konut", "label": "Konut", "symbols": { "marker": { "ref": "a" } } },
            { "value": "Ticaret", "label": "", "symbols": { "marker": { "ref": "b" } }, "enabled": false },
            { "value": "", "label": "", "symbols": { "marker": { "ref": "c" } } }
        ],
        "other": { "marker": { "ref": "d" } }
    })
}

#[test]
fn a_categorized_renderers_classes_say_their_condition_and_what_they_take() {
    let v = categorized();
    let classes = style_classes(&v);
    let rows: Vec<(&str, &str, bool, Option<&str>)> = classes
        .iter()
        .map(|c| {
            (
                c.id.as_str(),
                c.label.as_str(),
                c.enabled,
                c.expression.as_deref(),
            )
        })
        .collect();
    assert_eq!(
        rows,
        [
            ("c0", "Konut", true, Some("kullanim = 'Konut'")),
            ("c1", "Ticaret", false, Some("kullanim = 'Ticaret'")),
            (
                "c2",
                "(boş)",
                true,
                Some("(kullanim boş veya kullanim = '')")
            ),
            (
                OTHER,
                "Diğer değerler",
                true,
                Some("kullanim değil içinde ('Konut', 'Ticaret', '')")
            ),
        ]
    );
    assert_eq!(
        classes[0].symbols,
        Some(json!({ "marker": { "ref": "a" } }))
    );
    // The text is not drawn: it counts nowhere. A switched-off category still says what it takes.
    assert_eq!(counts(&v), [2, 1, 1, 2]);
    assert_eq!(
        members(&v, 0),
        [true, true, false, false, false, false, false]
    );
    assert_eq!(
        members(&v, 3),
        [false, false, false, true, false, true, false]
    );
}

#[test]
fn other_values_take_objects_without_a_value_when_no_category_holds_the_empty_one() {
    let mut v = categorized();
    v["categories"].as_array_mut().expect("a list").truncate(2);
    let other = style_classes(&v).pop().expect("other");
    assert_eq!(
        other.expression.as_deref(),
        Some("(kullanim değil içinde ('Konut', 'Ticaret') veya kullanim boş)")
    );
    assert_eq!(counts(&v), [2, 1, 3]);
    // Without “Diğer değerler”, the objects of no category are in no class.
    v.as_object_mut().expect("an object").remove("other");
    assert_eq!(counts(&v), [2, 1]);
}

#[test]
fn graduated_classes_bound_their_numbers_and_the_last_takes_its_maximum() {
    let v = json!({
        "type": "graduated", "expr": "alan",
        "classes": [
            { "min": 0, "max": 500, "label": "0 - 500", "symbols": {} },
            { "min": 500, "max": 1500, "label": "500 - 1500", "symbols": {}, "enabled": false }
        ]
    });
    let classes = style_classes(&v);
    assert_eq!(
        classes[0].expression.as_deref(),
        Some("alan >= 0 ve alan < 500")
    );
    assert_eq!(
        classes[1].expression.as_deref(),
        Some("alan >= 500 ve alan <= 1500")
    );
    assert!(!classes[1].enabled);
    // 120, 480, 60 below 500; 900 and 1500 (the last's maximum) in the second; the one without a value in none.
    assert_eq!(counts(&v), [3, 2]);
    assert_eq!(
        members(&v, 1),
        [false, false, true, true, false, false, false]
    );
}

#[test]
fn rules_list_their_children_after_them_and_the_else_rule_takes_the_rest() {
    let v = json!({
        "type": "rules",
        "rules": [
            { "id": "1", "label": "Büyük", "filter": "alan >= 500", "symbols": {},
              "children": [{ "id": "1a", "label": "Park", "filter": "kullanim = 'Park'", "symbols": {} }] },
            { "id": "2", "label": "Konut", "filter": "kullanim = 'Konut'", "symbols": {}, "enabled": false },
            { "id": "3", "label": "", "isElse": true, "symbols": {} }
        ]
    });
    let classes = style_classes(&v);
    let rows: Vec<(&str, &str, usize, Option<&str>)> = classes
        .iter()
        .map(|c| {
            (
                c.id.as_str(),
                c.label.as_str(),
                c.depth,
                c.expression.as_deref(),
            )
        })
        .collect();
    assert_eq!(
        rows,
        [
            ("r0", "Büyük", 0, Some("alan >= 500")),
            (
                "r0.0",
                "Park",
                1,
                Some("(alan >= 500) ve (kullanim = 'Park')")
            ),
            ("r1", "Konut", 0, Some("kullanim = 'Konut'")),
            // A switched-off sibling does not keep its objects from the else rule.
            ("r2", "(adsız kural)", 0, Some("değil ((alan >= 500))")),
        ]
    );
    assert_eq!(counts(&v), [2, 1, 2, 4]);
    assert_eq!(members(&v, 1).iter().filter(|m| **m).count(), 1);
}

#[test]
fn a_condition_that_does_not_compile_says_why_and_counts_nothing() {
    let v = json!({ "type": "categorized", "expr": "kullanim = (", "categories": [
        { "value": "Konut", "label": "Konut", "symbols": {} }
    ]});
    let list = objects();
    let refs: Vec<&Entity> = list.iter().collect();
    let reader = ClassReader::new(&v, Unit::METRE).expect("classes");
    let got = reader.counts(&refs, &name, &|_| None);
    assert_eq!(got.len(), 1);
    assert!(got[0].is_err(), "{got:?}");
}

#[test]
fn the_fields_a_scan_reads_are_the_ones_the_classes_name() {
    let v = json!({ "type": "rules", "rules": [
        { "id": "1", "label": "A", "filter": "alan > 5 ve kullanim = 'Konut'", "symbols": {} },
        { "id": "2", "label": "B", "filter": "$alan > 10", "symbols": {} }
    ]});
    let reader = ClassReader::new(&v, Unit::METRE).expect("classes");
    let mut fields = reader.fields().to_vec();
    fields.sort();
    assert_eq!(fields, ["alan", "kullanim"]);
    assert!(reader.reads_geometry(), "$alan reads the shape");
    assert!(ClassReader::new(&json!({ "type": "single", "symbols": {} }), Unit::METRE).is_none());
}

#[test]
fn switching_classes_writes_only_false_and_keeps_what_it_does_not_know() {
    let mut v = categorized();
    v["note"] = json!("kept");
    // Off: written as false; on: the flag goes; the other way: each turned over.
    assert_eq!(
        set_enabled(&mut v, Some(&["c0".into(), OTHER.into()]), Switch::Off),
        Ok(true)
    );
    assert_eq!(v["categories"][0]["enabled"], json!(false));
    assert_eq!(v["otherEnabled"], json!(false));
    assert_eq!(
        set_enabled(&mut v, Some(&["c0".into()]), Switch::Off),
        Ok(false),
        "already off"
    );
    assert_eq!(set_enabled(&mut v, None, Switch::Invert), Ok(true));
    assert!(v["categories"][0].get("enabled").is_none());
    assert!(v["categories"][1].get("enabled").is_none());
    assert_eq!(v["categories"][2]["enabled"], json!(false));
    assert!(v.get("otherEnabled").is_none());
    assert_eq!(set_enabled(&mut v, None, Switch::On), Ok(true));
    assert_eq!(
        v["categories"]
            .as_array()
            .map(|a| a.iter().all(|k| k.get("enabled").is_none())),
        Some(true)
    );
    assert_eq!(v["note"], json!("kept"));
    assert_eq!(
        set_enabled(&mut v, Some(&["c0".into(), "c7".into()]), Switch::Off),
        Err(SwitchRefused::UnknownClass(1))
    );
    assert!(
        v["categories"][0].get("enabled").is_none(),
        "nothing written on a refusal"
    );

    let mut rules = json!({ "type": "rules", "rules": [
        { "id": "1", "label": "A", "children": [{ "id": "1a", "label": "B" }] }
    ]});
    assert_eq!(
        set_enabled(&mut rules, Some(&["r0.0".into()]), Switch::Off),
        Ok(true)
    );
    assert_eq!(rules["rules"][0]["children"][0]["enabled"], json!(false));

    let mut graduated = json!({ "type": "graduated", "expr": "alan", "classes": [
        { "min": 0, "max": 1, "label": "a", "symbols": {} }
    ]});
    assert_eq!(
        set_enabled(&mut graduated, Some(&["g0".into()]), Switch::Off),
        Ok(true)
    );
    assert_eq!(graduated["classes"][0]["enabled"], json!(false));

    let mut single = json!({ "type": "single", "symbols": {} });
    assert_eq!(
        set_enabled(&mut single, None, Switch::Off),
        Err(SwitchRefused::NoClasses)
    );
}

#[test]
fn an_object_is_drawn_when_its_class_is_on() {
    let list = objects();
    let refs: Vec<&Entity> = list.iter().collect();
    // Ticaret is off; Diğer değerler on: Park and Sanayi drawn by it; the text never.
    let v = categorized();
    let reader = ClassReader::new(&v, Unit::METRE).expect("classes");
    assert_eq!(
        reader.drawn(&refs, &name, &|_| None),
        [true, true, false, true, true, true, false]
    );
    // Without Diğer değerler, a value no category holds is drawn by nothing.
    let mut no_other = categorized();
    no_other.as_object_mut().expect("an object").remove("other");
    let reader = ClassReader::new(&no_other, Unit::METRE).expect("classes");
    assert_eq!(
        reader.drawn(&refs, &name, &|_| None),
        [true, true, false, false, true, false, false]
    );
    // A rule's own symbols draw while it and the rules above it are on; a switched-off parent hides its children.
    let rules = json!({ "type": "rules", "rules": [
        { "id": "1", "label": "Büyük", "filter": "alan >= 500", "enabled": false,
          "children": [{ "id": "1a", "label": "Park", "filter": "kullanim = 'Park'", "symbols": {} }] },
        { "id": "2", "label": "Konut", "filter": "kullanim = 'Konut'", "symbols": {} }
    ]});
    let reader = ClassReader::new(&rules, Unit::METRE).expect("classes");
    assert_eq!(
        reader.drawn(&refs, &name, &|_| None),
        [true, true, false, false, false, false, false]
    );
}
