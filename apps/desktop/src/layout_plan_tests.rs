//! The desktop keeps its layout as the web does: every section of
//! fixtures/shell/v1/layout.json (format in fixtures/shell/README.md), whose
//! answers scripts/fixtures/layout_cases.py wrote apart from either program.

use serde_json::{Value, json};

use crate::layout_plan::{self as plan, Rule};

fn fixture() -> Value {
    serde_json::from_str(include_str!("../../../fixtures/shell/v1/layout.json"))
        .expect("layout.json reads")
}

fn f64_of(v: &Value) -> f64 {
    v.as_f64().expect("a number")
}

#[test]
fn the_fields_their_defaults_rules_and_limits_are_the_webs() {
    let f = fixture();
    assert_eq!(f["key"], plan::LAYOUT_KEY);
    assert_eq!(f["saveMs"], plan::SAVE_MS);
    assert_eq!(Value::Object(plan::defaults()), f["defaults"]);
    let defaults = plan::defaults();
    let keys: Vec<&str> = defaults.keys().map(String::as_str).collect();
    let fields = f["fields"].as_object().expect("fields");
    assert_eq!(fields.len(), plan::FIELDS.len());
    for (key, rule) in plan::FIELDS {
        assert!(keys.contains(&key), "{key} has a default");
        let written = match rule {
            Rule::Enum(values) => json!({ "kind": "enum", "values": values }),
            Rule::Boolean => json!({ "kind": "boolean" }),
            Rule::Number { min, max } => {
                let mut o = json!({ "kind": "number" });
                if let Some(min) = min {
                    o["min"] = json!(min);
                }
                if let Some(max) = max {
                    o["max"] = json!(max);
                }
                o
            }
            Rule::Columns => json!({ "kind": "columns" }),
            Rule::Text => json!({ "kind": "text" }),
            Rule::Texts => json!({ "kind": "texts" }),
            Rule::TextMap => json!({ "kind": "textMap" }),
        };
        let expected = &fields[key];
        // Numbers compare by value (240 and 240.0 are one limit).
        assert_eq!(written["kind"], expected["kind"], "{key}");
        assert_eq!(written["values"], expected["values"], "{key}");
        for bound in ["min", "max"] {
            assert_eq!(
                written[bound].as_f64(),
                expected[bound].as_f64(),
                "{key} {bound}"
            );
        }
    }
    let limits = &f["limits"];
    for (name, ours) in [
        ("dockWidth", plan::DOCK_WIDTH),
        ("layersFraction", plan::LAYERS_FRACTION),
        ("bottomHeight", plan::BOTTOM_HEIGHT),
    ] {
        let l = &limits[name];
        assert_eq!(Some(ours.min), l["min"].as_f64(), "{name}");
        assert_eq!(ours.max, l["max"].as_f64(), "{name}");
        assert_eq!(ours.max_share, l["maxShare"].as_f64(), "{name}");
        assert_eq!(Some(ours.reset), l["reset"].as_f64(), "{name}");
    }
}

#[test]
fn a_stored_layout_is_read_as_the_web_reads_it() {
    let f = fixture();
    for case in f["reads"].as_array().expect("reads") {
        let read = plan::read_layout(case["stored"].as_str());
        assert_eq!(Value::Object(read), case["layout"], "{}", case["title"]);
    }
}

#[test]
fn the_sizes_shown_are_the_webs() {
    let f = fixture();
    for c in f["dockWidths"].as_array().expect("widths") {
        assert_eq!(
            plan::dock_width_on(f64_of(&c["kept"]), f64_of(&c["window"])),
            f64_of(&c["shown"]),
            "{c}"
        );
    }
    for c in f["bottomHeights"].as_array().expect("heights") {
        assert_eq!(
            plan::bottom_height_on(f64_of(&c["kept"]), f64_of(&c["window"])),
            f64_of(&c["shown"]),
            "{c}"
        );
    }
    for c in f["layersDrags"].as_array().expect("drags") {
        assert_eq!(
            plan::dragged_layers_fraction(
                f64_of(&c["start"]),
                f64_of(&c["dy"]),
                f64_of(&c["height"])
            ),
            f64_of(&c["fraction"]),
            "{c}"
        );
    }
}

#[test]
fn the_ribbons_kept_parts_are_the_webs() {
    let f = fixture();
    let r = &f["ribbon"];
    let texts = |v: &Value| -> Vec<String> {
        v.as_array()
            .expect("a list")
            .iter()
            .map(|x| x.as_str().expect("a text").to_owned())
            .collect()
    };
    assert_eq!(texts(&r["quickAccessFixed"]), plan::QUICK_ACCESS);
    for c in r["quickAccess"].as_array().expect("bars") {
        let exists = texts(&c["exists"]);
        let bar = plan::quick_access_of(&texts(&c["kept"]), |id| exists.iter().any(|e| e == id));
        assert_eq!(bar, texts(&c["bar"]), "{c}");
    }
    for c in r["splits"].as_array().expect("splits") {
        let entries: Vec<(&str, Option<&str>)> = c["entries"]
            .as_array()
            .expect("entries")
            .iter()
            .map(|e| {
                (
                    e["command"].as_str().expect("a command"),
                    e["option"].as_str(),
                )
            })
            .collect();
        assert_eq!(
            plan::split_current(&entries, c["kept"].as_str()) as u64,
            c["current"].as_u64().expect("current"),
            "{c}"
        );
    }
    for c in r["startTabs"].as_array().expect("tabs") {
        let tabs: Vec<(&str, bool)> = c["tabs"]
            .as_array()
            .expect("tabs")
            .iter()
            .map(|t| (t["id"].as_str().expect("an id"), !t["contextual"].is_null()))
            .collect();
        assert_eq!(
            plan::start_tab(c["kept"].as_str().expect("kept"), &tabs),
            c["tab"].as_str().expect("tab"),
            "{c}"
        );
    }
}
