//! The contract's style rules (docs/adr/0183) hold to the shared cases the
//! independent reference writes (scripts/fixtures/annotation_style_cases.py,
//! fixtures/text/v1/styles.json): the tables' rule, a text's face and a
//! dimension's look checked, a style applied, an object following its style.

use kentos_contracts::{
    DimensionLook, DimensionStyleDef, TextFace, TextLook, TextStyleDef, apply_dimension_style,
    apply_text_style, dimension_styles_problem, follow_dimension_style, follow_text_style,
    text_styles_problem,
};
use serde_json::{Map, Value, json};

const CASES: &str = include_str!("../../../../../fixtures/text/v1/styles.json");

/// A text's height, width factor and face from the cases' flat object.
fn text_look(v: &Value) -> TextLook {
    TextLook {
        face: serde_json::from_value(v.clone()).expect("a face"),
        width_factor: v.get("widthFactor").and_then(Value::as_f64),
        height: v["height"].as_f64().expect("a height"),
    }
}

/// The cases' flat object of a text's look: its height, width factor and face.
fn text_value(look: &TextLook) -> Value {
    let mut out = match serde_json::to_value(&look.face).expect("a face") {
        Value::Object(m) => m,
        _ => Map::new(),
    };
    out.insert("height".into(), json!(look.height));
    if let Some(w) = look.width_factor {
        out.insert("widthFactor".into(), json!(w));
    }
    Value::Object(out)
}

fn dimension_value(look: &DimensionLook, height: f64) -> Value {
    let mut out = match serde_json::to_value(look).expect("a look") {
        Value::Object(m) => m,
        _ => Map::new(),
    };
    out.insert("height".into(), json!(height));
    Value::Object(out)
}

/// Objects compared as sets of fields (the cases write them in the reference's order).
fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len() && x.iter().all(|(k, v)| y.get(k).is_some_and(|w| same(v, w)))
        }
        (Value::Number(x), Value::Number(y)) => {
            x.as_f64().map(f64::to_bits) == y.as_f64().map(f64::to_bits)
        }
        _ => a == b,
    }
}

#[test]
fn every_style_rule_holds_to_the_shared_cases() {
    let doc: Value = serde_json::from_str(CASES).expect("the cases read");
    assert_eq!(doc["format"], "kentos.annotation-style-cases");
    let cases = doc["cases"].as_array().expect("cases");
    assert!(cases.len() >= 60);
    let mut problems = Vec::new();
    for c in cases {
        let name = c["name"].as_str().unwrap_or("?");
        let scale = c["plotScale"].as_f64().unwrap_or(1000.0);
        let got: Value = match c["op"].as_str().unwrap_or("") {
            "textStylesProblem" => {
                let styles: Vec<TextStyleDef> =
                    serde_json::from_value(c["styles"].clone()).expect("text styles");
                json!(text_styles_problem(&styles))
            }
            "dimensionStylesProblem" => {
                let styles: Vec<DimensionStyleDef> =
                    serde_json::from_value(c["styles"].clone()).expect("dimension styles");
                json!(dimension_styles_problem(&styles))
            }
            "faceProblem" => {
                let face: TextFace = serde_json::from_value(c["face"].clone()).expect("a face");
                json!(face.problem())
            }
            "lookProblem" => {
                let look: DimensionLook =
                    serde_json::from_value(c["look"].clone()).expect("a look");
                json!(look.problem())
            }
            "applyText" => {
                let styles: Vec<TextStyleDef> =
                    serde_json::from_value(c["styles"].clone()).expect("text styles");
                let style = c["style"]
                    .as_str()
                    .and_then(|id| styles.iter().find(|s| s.id == id));
                text_value(&apply_text_style(style, &text_look(&c["text"]), scale))
            }
            "followText" => {
                let old: TextStyleDef = serde_json::from_value(c["old"].clone()).expect("old");
                let new: TextStyleDef = serde_json::from_value(c["new"].clone()).expect("new");
                text_value(&follow_text_style(
                    &old,
                    &new,
                    &text_look(&c["text"]),
                    scale,
                ))
            }
            "applyDimension" => {
                let styles: Vec<DimensionStyleDef> =
                    serde_json::from_value(c["styles"].clone()).expect("dimension styles");
                let style = c["style"]
                    .as_str()
                    .and_then(|id| styles.iter().find(|s| s.id == id));
                let (look, height) = apply_dimension_style(style, scale);
                dimension_value(&look, height)
            }
            "followDimension" => {
                let old: DimensionStyleDef = serde_json::from_value(c["old"].clone()).expect("old");
                let new: DimensionStyleDef = serde_json::from_value(c["new"].clone()).expect("new");
                let d = &c["dimension"];
                let look: DimensionLook = serde_json::from_value(d.clone()).expect("a look");
                let height = d["height"].as_f64().expect("a height");
                let (look, height) = follow_dimension_style(&old, &new, &look, height, scale);
                dimension_value(&look, height)
            }
            // A dimension's value as written is the formatters' (kentos_interaction's
            // `Format::dimension_in`, the web's `dimensionValue`); they hold to these cases.
            "dimensionValue" => continue,
            other => panic!("bilinmeyen işlem {other}"),
        };
        if !same(&got, &c["expect"]) {
            problems.push(format!("{name}: {got}, beklenen {}", c["expect"]));
        }
    }
    assert!(problems.is_empty(), "\n{}", problems.join("\n"));
}
