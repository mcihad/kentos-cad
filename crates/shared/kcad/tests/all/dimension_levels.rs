//! A Kot bound to a level opts into schema 121 (docs/adr/0721 §2); schema 120 refuses it, and the same drawing without
//! it stays in the schema before. The level's JSON is the contract's; only an elevation mark takes one.

use kentos_kcad::contracts::{DocumentSnapshotV2, Entity};

const MODEL: &str = include_str!("../../../../../fixtures/kcad/v2/dimension-levels.json");
const BYTES: &[u8] = include_bytes!("../../../../../fixtures/kcad/v2/dimension-levels.kcad");

#[test]
fn a_kot_bound_to_a_level_opts_into_schema_121() {
    let model: DocumentSnapshotV2 = serde_json::from_str(MODEL).unwrap();
    assert_eq!(kentos_kcad::encode_verified(&model).unwrap(), BYTES);
    let read = kentos_kcad::decode(BYTES).unwrap();
    let levels: Vec<_> = read
        .entities
        .iter()
        .filter_map(|e| match e {
            Entity::Dimension(d) => Some(d.level.as_deref().map(|l| (l.id.clone(), l.offset))),
            _ => None,
        })
        .collect();
    assert_eq!(
        levels,
        [Some(("kat-1".into(), None)), Some(("kat-1".into(), Some(-0.02))), Some(("zemin".into(), None)), None]
    );
    assert!(BYTES.windows(10).any(|w| w == b"\x67version\x18\x79"));
    let old = include_bytes!("../../../../../fixtures/kcad/v2/broken/dimension-levels-in-schema-120.kcad");
    assert_eq!(kentos_kcad::decode(old).expect_err("refused").code, kentos_kcad::Code::UnknownField);
    let mut plain = model;
    for e in &mut plain.entities {
        if let Entity::Dimension(d) = e {
            d.level = None;
        }
    }
    let bytes = kentos_kcad::encode_verified(&plain).unwrap();
    assert!(!bytes.windows(10).any(|w| w == b"\x67version\x18\x79"), "no level, no schema 121");
}

#[test]
fn a_level_only_an_elevation_mark_takes_and_its_rules_hold() {
    for (name, code) in [
        ("not-a-kot", kentos_kcad::Code::BadValue),
        ("blank-id", kentos_kcad::Code::BadValue),
        ("far-offset", kentos_kcad::Code::BadValue),
        ("unknown-field", kentos_kcad::Code::UnknownField),
        ("type", kentos_kcad::Code::BadValue),
    ] {
        let bytes = std::fs::read(format!(
            "{}/../../../fixtures/kcad/v2/broken/dimension-level-{name}.kcad",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        assert_eq!(kentos_kcad::decode(&bytes).expect_err(name).code, code, "{name}");
    }
}
