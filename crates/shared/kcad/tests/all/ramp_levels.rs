//! A ramp bound to its level opts into schema 124 (docs/adr/0723 §2); schema 123 refuses its `baseOffset`, and the
//! offset keeps the contract's rule (finite, at most 1000 m, the ramp's level given).

use kentos_kcad::contracts::{DocumentSnapshotV2, Entity};

const MODEL: &str = include_str!("../../../../../fixtures/kcad/v2/ramp-levels.json");
const BYTES: &[u8] = include_bytes!("../../../../../fixtures/kcad/v2/ramp-levels.kcad");

#[test]
fn a_ramp_bound_to_its_level_opts_into_schema_124() {
    let model: DocumentSnapshotV2 = serde_json::from_str(MODEL).unwrap();
    assert_eq!(kentos_kcad::encode_verified(&model).unwrap(), BYTES);
    let read = kentos_kcad::decode(BYTES).unwrap();
    let ramps: Vec<_> = read
        .entities
        .iter()
        .filter_map(|e| match e {
            Entity::Ramp(r) => Some(&r.ramp),
            _ => None,
        })
        .collect();
    assert_eq!(ramps.iter().map(|r| (r.base, r.base_offset)).collect::<Vec<_>>(), [
        (Some(0.0), Some(0.0)),
        (Some(3.1), Some(-0.15)),
        (Some(0.45), None),
    ]);
    assert!(BYTES.windows(10).any(|w| w == b"\x67version\x18\x7c"));
    let old = include_bytes!("../../../../../fixtures/kcad/v2/broken/ramp-levels-in-schema-123.kcad");
    assert_eq!(kentos_kcad::decode(old).expect_err("refused").code, kentos_kcad::Code::BadValue);
    // The same drawing without its bound ramps stays in the schema before.
    let mut plain = model;
    for e in &mut plain.entities {
        if let Entity::Ramp(r) = e {
            r.ramp.base_offset = None;
        }
    }
    let bytes = kentos_kcad::encode_verified(&plain).unwrap();
    assert!(!bytes.windows(10).any(|w| w == b"\x67version\x18\x7c"), "no bound ramp, no schema 124");
}

#[test]
fn a_ramps_level_keeps_its_rules() {
    for name in ["without-level", "far-offset"] {
        let bytes = std::fs::read(format!(
            "{}/../../../fixtures/kcad/v2/broken/ramp-level-{name}.kcad",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        assert_eq!(kentos_kcad::decode(&bytes).expect_err(name).code, kentos_kcad::Code::BadValue, "{name}");
    }
}
