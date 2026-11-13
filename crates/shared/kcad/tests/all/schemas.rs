//! The rule every schema change keeps (plan/cad.md `CAD-DATA-23`, docs/adr/0265, docs/specs/kcad-v2.md §3.1):
//! each schema the codec reads has a sample written in it that every reader opens, and each schema after the
//! first has a sample of its newest feature in the schema before, which the readers refuse; the version past the
//! newest is refused as `schema_version`. A schema added without both fails here; one that changes a unit instead of
//! adding a field (191, docs/adr/0977) has its older samples read in the new unit instead.

use std::collections::BTreeSet;

use kentos_kcad::SCHEMAS;
use serde_json::Value;

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../fixtures/kcad/v2/");

/// The document schema a file's payload says (the CBOR text key `version` and its unsigned integer).
fn schema(file: &[u8]) -> u32 {
    let payload = &file[36..file.len() - 32];
    let key = b"\x67version";
    let at = payload
        .windows(key.len())
        .position(|w| w == key)
        .expect("a version key")
        + key.len();
    match payload[at] {
        0x18 => u32::from(payload[at + 1]),
        b => u32::from(b),
    }
}

#[test]
fn every_schema_has_a_sample_and_the_one_before_refuses_its_feature() {
    let expected: Value = serde_json::from_str(
        &std::fs::read_to_string(format!("{DIR}expected.json")).expect("expected.json"),
    )
    .expect("JSON");
    let (mut written, mut refused) = (BTreeSet::new(), BTreeSet::new());
    for f in expected["files"].as_array().expect("files") {
        let name = f["file"].as_str().expect("a name");
        let data = std::fs::read(format!("{DIR}{name}")).expect("the file");
        if f["error"].is_null() && f["sniff"] == "kcad" {
            kentos_kcad::decode(&data).unwrap_or_else(|e| panic!("{name}: {e}"));
            written.insert(schema(&data));
        } else if let Some(n) = name
            .split("in-schema-")
            .nth(1)
            .and_then(|r| r.split(['.', '-']).next())
            .and_then(|n| n.parse::<u32>().ok())
        {
            assert!(kentos_kcad::decode(&data).is_err(), "{name} is read");
            refused.insert(n);
        }
    }
    for (index, &s) in SCHEMAS.iter().enumerate() {
        // Schema 254 (docs/adr/8054 §10) adds a rule, not a field: `modules`, each module's own version of its data.
        // No module has raised its version yet, so this build refuses every `modules` and no valid sample can carry
        // one; its samples are the refused ones (`modules-*.kcad`, read by `fixtures`) and the one in 253 below.
        assert!(
            written.contains(&s) || s == kentos_kcad::SCHEMA_WITH_MODULE_VERSIONS,
            "schema {s} has no sample written in it"
        );
        // Schema 191 (docs/adr/0977) adds no field: it changes the unit of fields older schemas had. Nothing of it can
        // be refused in 186; instead each older road sample, as its own schema's writer wrote it in percent, reads as
        // today's drawing in grades (the `*-percent.kcad` files, read by `fixtures`).
        if s == kentos_kcad::SCHEMA_WITH_ROAD_GRADES {
            continue;
        }
        if let Some(&previous) = index.checked_sub(1).and_then(|i| SCHEMAS.get(i)) {
            assert!(
                refused.contains(&previous),
                "schema {s}'s feature has no sample refused in schema {}",
                previous
            );
        }
    }
    let past = SCHEMAS[SCHEMAS.len() - 1] + 1;
    let file = std::fs::read(format!("{DIR}broken/schema-version-{past}.kcad"))
        .expect("the next schema's sample");
    assert_eq!(
        kentos_kcad::decode(&file).expect_err("refused").code,
        kentos_kcad::Code::SchemaVersion
    );
}
