//! A dimension along a cut opts into schema 118 (docs/adr/0711); schema 117 refuses its rule `cross`, and the same
//! drawing without such a dimension stays in the schema before.

use kentos_kcad::contracts::{DimensionRule, DocumentSnapshotV2, Entity};

const MODEL: &str = include_str!("../../../../../fixtures/kcad/v2/dimension-cross.json");
const BYTES: &[u8] = include_bytes!("../../../../../fixtures/kcad/v2/dimension-cross.kcad");

#[test]
fn dimensions_along_a_cut_opt_into_schema_118_and_117_refuses_them() {
    let model: DocumentSnapshotV2 = serde_json::from_str(MODEL).unwrap();
    assert_eq!(kentos_kcad::encode_verified(&model).unwrap(), BYTES);
    let read = kentos_kcad::decode(BYTES).unwrap();
    assert_eq!(read.entities, model.entities);
    assert!(BYTES.windows(10).any(|w| w == b"\x67version\x18\x76"));
    let old =
        include_bytes!("../../../../../fixtures/kcad/v2/broken/dimension-cross-in-schema-117.kcad");
    assert_eq!(
        kentos_kcad::decode(old).expect_err("refused").code,
        kentos_kcad::Code::BadValue
    );
    let mut plain = model;
    for e in &mut plain.entities {
        if let Entity::Dimension(d) = e
            && let Some(refs) = &mut d.refs
        {
            assert_eq!(refs.rule, Some(DimensionRule::Cross));
            refs.rule = None;
        }
    }
    let bytes = kentos_kcad::encode_verified(&plain).unwrap();
    assert!(
        !bytes.windows(10).any(|w| w == b"\x67version\x18\x76"),
        "no cut, no schema 118"
    );
}

#[test]
fn the_typed_columns_carry_the_cut() {
    let model: DocumentSnapshotV2 = serde_json::from_str(MODEL).unwrap();
    let cols = kentos_kcad::columns::pack(model.entities.clone(), model.uids.clone());
    let (entities, uids) = kentos_kcad::columns::unpack(&cols).expect("unpacks");
    assert_eq!((entities, uids), (model.entities, model.uids));
}
