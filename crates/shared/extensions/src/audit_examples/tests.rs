//! The examples file read as gate 1 reads it: a good one whole, each mistake with its place.

use serde_json::{Value, json};

use super::*;

fn square(x: f64) -> Value {
    json!({ "type": "Polygon", "coordinates": [[[x, 0.0], [x + 10.0, 0.0], [x + 10.0, 10.0], [x, 10.0], [x, 0.0]]] })
}

fn file(examples: Value) -> Vec<u8> {
    serde_json::to_vec(&json!({ "format": FORMAT, "schemaVersion": 1, "examples": examples }))
        .unwrap()
}

fn places(found: Result<AuditExamples, Vec<Problem>>) -> Vec<String> {
    found
        .expect_err("refused")
        .into_iter()
        .map(|p| {
            assert_eq!(p.code, "audit_examples");
            assert_eq!(p.file.as_deref(), Some("audits/r.examples.json"));
            p.at.unwrap_or_default()
        })
        .collect()
}

/// A rule's examples: objects as data and the findings expected, read whole.
#[test]
fn a_rules_examples_are_read_whole() {
    let read = read(
        "audits/r.examples.json",
        "r",
        &file(json!([
            { "title": "Çakışan iki parsel",
              "features": [{ "layer": "parsel", "geometry": square(0.0), "attrs": { "no": 1 } },
                           { "layer": "parsel", "geometry": square(5.0) }],
              "expect": { "count": 1, "has": [{ "severity": "error", "message": "çakışıyor", "feature": 1,
                                                "fix": "ext.tr.ornek.r.duzelt" }] } },
            { "title": "Boş çizim", "expect": { "count": 0 } }
        ])),
    )
    .expect("read");
    assert_eq!(read.examples.len(), 2);
    let want = &read.examples[0].expect.has[0];
    assert_eq!(want.severity, Some(Severity::Error));
    assert_eq!(want.feature, Some(1));
    assert_eq!(read.examples[1].expect.count, Some(0));
    assert!(read.examples[1].features.is_empty() && read.examples[1].architecture.is_none());
}

/// Each mistake is said where it is: the format, the count of examples, an object's layer and shape, an empty
/// expectation, more findings wanted than counted, an empty one, an object that is not in the example.
#[test]
fn each_mistake_is_said_with_its_place() {
    let bad = |examples: Value| places(read("audits/r.examples.json", "r", &file(examples)));
    let one = |expect: Value| {
        json!([{ "title": "Bir", "features": [{ "layer": "a", "geometry": square(0.0) }],
                                        "expect": expect }])
    };
    assert_eq!(bad(json!([])), ["/examples"]);
    let eleven: Vec<Value> = (0..11)
        .map(|i| json!({ "title": format!("Ö{i}"), "expect": { "count": 0 } }))
        .collect();
    assert_eq!(bad(json!(eleven)), ["/examples"]);
    assert_eq!(
        bad(
            json!([{ "title": " ", "features": [{ "geometry": square(0.0) },
                                                  { "layer": "a", "geometry": { "type": "Circle" } },
                                                  { "layer": "a", "geometry": square(0.0), "renk": 1 }],
                     "expect": { "count": 0 } }])
        ),
        [
            "/examples/0/title",
            "/examples/0/features/0",
            "/examples/0/features/1",
            "/examples/0/features/2"
        ]
    );
    assert_eq!(bad(one(json!({}))), ["/examples/0/expect"]);
    assert_eq!(
        bad(one(json!({ "count": 0, "has": [{ "severity": "error" }] }))),
        ["/examples/0/expect/has"]
    );
    assert_eq!(
        bad(one(json!({ "has": [{}] }))),
        ["/examples/0/expect/has/0"]
    );
    assert_eq!(
        bad(one(json!({ "has": [{ "feature": 1 }] }))),
        ["/examples/0/expect/has/0/feature"]
    );
    // Another format, and a key the format has not (a typo): said, the second by its line.
    let other = serde_json::to_vec(&json!({ "format": "x", "schemaVersion": 1,
                                             "examples": [{ "title": "Bir", "expect": { "count": 0 } }] }))
    .unwrap();
    assert_eq!(
        places(read("audits/r.examples.json", "r", &other)),
        ["/format"]
    );
    let typo = b"{\"format\": \"kentos.auditExamples\", \"schemaVersion\": 1,\n\"examples\": [{\"title\": \"Bir\", \
                 \"expect\": {\"cuont\": 1}}]}";
    let said = read("audits/r.examples.json", "r", typo).expect_err("refused");
    assert_eq!(said[0].at.as_deref(), Some("satır 2"));
    assert!(said[0].message.contains("cuont"), "{}", said[0].message);
}

/// The committed `auditExamplesSchema` is the one the file's types make: written by
/// `KENTOS_WRITE_SDK=1 cargo test -p kentos-extensions --lib -- sdk_pack` with the SDK's other parts.
#[cfg(feature = "schema")]
#[test]
fn the_sdk_pack_audit_examples_schema_is_the_types() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let made = serde_json::to_string_pretty(&json_schema()).unwrap() + "\n";
    if std::env::var_os("KENTOS_WRITE_SDK").is_some() {
        std::fs::write(root.join("sdk/kentos-audit-examples.schema.json"), &made)
            .expect("the schema");
        return;
    }
    assert!(
        crate::sdk_pack::AUDIT_EXAMPLES_SCHEMA == made,
        "the examples' types changed: KENTOS_WRITE_SDK=1 cargo test -p kentos-extensions --lib -- sdk_pack"
    );
    let schema = crate::sdk_pack::sdk_part("auditExamplesSchema").expect("the part");
    assert_eq!(schema["properties"]["format"]["const"], FORMAT);
    assert_eq!(schema["properties"]["examples"]["maxItems"], MOST_EXAMPLES);
}
