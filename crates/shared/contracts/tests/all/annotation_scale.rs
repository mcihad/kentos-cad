//! The contract's annotation heights and following rule (docs/adr/0205 §1,
//! §3) hold to the shared cases the independent reference writes
//! (scripts/fixtures/annotation_scale_cases.py, fixtures/text/v1/scale.json):
//! the heights checked and kept, a change that is none, and each object after
//! the scale or a height changed, exactly.

use kentos_contracts::{
    AnnotationHeights, DimensionStyleDef, Entity, ScaleChange, TextStyleDef,
    follow_annotation_scale,
};
use serde_json::{Value, json};

const CASES: &str = include_str!("../../../../../fixtures/text/v1/scale.json");

fn heights(v: &Value) -> AnnotationHeights {
    serde_json::from_value(v.clone()).expect("annotation heights")
}

fn change(v: &Value) -> ScaleChange {
    ScaleChange {
        from_scale: v["fromScale"].as_f64().expect("a scale"),
        to_scale: v["toScale"].as_f64().expect("a scale"),
        from: heights(&v["from"]),
        to: heights(&v["to"]),
    }
}

#[test]
fn the_rules_hold_to_the_shared_cases() {
    let doc: Value = serde_json::from_str(CASES).expect("the cases read");
    let cases = doc["cases"].as_array().expect("a list of cases");
    assert!(cases.len() >= 40, "{} cases", cases.len());
    for c in cases {
        let name = c["name"].as_str().unwrap_or("?");
        let got = match c["op"].as_str() {
            Some("problem") => match heights(&c["heights"]).problem() {
                Some((key, words)) => json!([key, words]),
                None => Value::Null,
            },
            Some("sanitize") => match heights(&c["heights"]).sanitized() {
                Some(h) => serde_json::to_value(h).expect("heights"),
                None => Value::Null,
            },
            Some("isNone") => json!(change(&c["change"]).is_none()),
            Some("follow") => {
                let entity: Entity = serde_json::from_value(c["entity"].clone())
                    .unwrap_or_else(|e| panic!("{name}: the object reads: {e}"));
                let texts: Vec<TextStyleDef> =
                    serde_json::from_value(c["textStyles"].clone()).expect("text styles");
                let dims: Vec<DimensionStyleDef> =
                    serde_json::from_value(c["dimensionStyles"].clone()).expect("dimension styles");
                match follow_annotation_scale(&entity, &change(&c["change"]), &texts, &dims) {
                    Some(e) => serde_json::to_value(e).expect("an object"),
                    None => Value::Null,
                }
            }
            other => panic!("{name}: unknown op {other:?}"),
        };
        assert_eq!(got, c["expect"], "{name}");
    }
}
