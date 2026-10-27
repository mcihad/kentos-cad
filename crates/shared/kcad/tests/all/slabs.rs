//! A slab opts into schema 123 (docs/adr/0722 §1); schema 122 does not know the kind, a block definition holds none, and
//! a slab's fields keep the contract's rule (a default is not written, a thickness, an edge's width, a ring, its runs).

use kentos_kcad::contracts::{DocumentSnapshotV2, Entity, SlabEdgeKind, SlabRole};

const MODEL: &str = include_str!("../../../../../fixtures/kcad/v2/slabs.json");
const BYTES: &[u8] = include_bytes!("../../../../../fixtures/kcad/v2/slabs.kcad");

#[test]
fn a_slab_opts_into_schema_123() {
    let model: DocumentSnapshotV2 = serde_json::from_str(MODEL).unwrap();
    assert_eq!(kentos_kcad::encode_verified(&model).unwrap(), BYTES);
    let read = kentos_kcad::decode(BYTES).unwrap();
    let slabs: Vec<_> = read
        .entities
        .iter()
        .filter_map(|e| match e {
            Entity::Slab(s) => Some(&s.slab),
            _ => None,
        })
        .collect();
    assert_eq!(slabs.len(), 4);
    assert_eq!(slabs[0].parts[0].holes.len(), 1);
    assert_eq!(slabs[1].role, SlabRole::Balcony);
    assert_eq!(slabs[1].edge.as_ref().map(|e| (e.kind, e.runs.len())), Some((SlabEdgeKind::Railing, 1)));
    assert!(slabs[2].excluded && slabs[2].offset == -0.05);
    assert_eq!(slabs[3].parts.len(), 2);
    assert!(BYTES.windows(10).any(|w| w == b"\x67version\x18\x7b"));
    let old = include_bytes!("../../../../../fixtures/kcad/v2/broken/slabs-in-schema-122.kcad");
    assert_eq!(kentos_kcad::decode(old).expect_err("refused").code, kentos_kcad::Code::UnknownKind);
    // The same drawing without its slabs stays in the schema before.
    let mut plain = model;
    plain.entities.retain(|e| !matches!(e, Entity::Slab(_)));
    plain.uids.clear();
    let bytes = kentos_kcad::encode_verified(&plain).unwrap();
    assert!(!bytes.windows(10).any(|w| w == b"\x67version\x18\x7b"), "no slab, no schema 123");
}

#[test]
fn a_slabs_fields_keep_their_rules() {
    for (name, code) in [
        ("thin", kentos_kcad::Code::BadValue),
        ("default-role", kentos_kcad::Code::BadValue),
        ("zero-offset", kentos_kcad::Code::BadValue),
        ("unknown-field", kentos_kcad::Code::UnknownField),
        ("wide-edge", kentos_kcad::Code::BadValue),
        ("run-bulges", kentos_kcad::Code::BadValue),
        ("blank-material", kentos_kcad::Code::BadValue),
        ("two-point-ring", kentos_kcad::Code::BadValue),
        ("false-excluded", kentos_kcad::Code::BadValue),
        ("in-block", kentos_kcad::Code::BadValue),
    ] {
        let bytes = std::fs::read(format!("{}/../../../fixtures/kcad/v2/broken/slab-{name}.kcad", env!("CARGO_MANIFEST_DIR")))
            .unwrap();
        assert_eq!(kentos_kcad::decode(&bytes).expect_err(name).code, code, "{name}");
    }
}
