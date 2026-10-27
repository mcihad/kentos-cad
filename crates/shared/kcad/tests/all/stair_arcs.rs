//! A curved stair opts into schema 117 (docs/adr/0709); schema 116 refuses its `bulges`, and the same drawing with
//! straight stairs only stays in the schema before.

use kentos_kcad::contracts::{DocumentSnapshotV2, Entity};

const MODEL: &str = include_str!("../../../../../fixtures/kcad/v2/stair-arcs.json");
const BYTES: &[u8] = include_bytes!("../../../../../fixtures/kcad/v2/stair-arcs.kcad");

#[test]
fn curved_stairs_opt_into_schema_117_and_116_refuses_them() {
    let model: DocumentSnapshotV2 = serde_json::from_str(MODEL).unwrap();
    assert_eq!(kentos_kcad::encode_verified(&model).unwrap(), BYTES);
    let read = kentos_kcad::decode(BYTES).unwrap();
    assert_eq!(read.entities, model.entities);
    assert!(BYTES.windows(10).any(|w| w == b"\x67version\x18\x75"));
    let old =
        include_bytes!("../../../../../fixtures/kcad/v2/broken/stair-arcs-in-schema-116.kcad");
    assert_eq!(
        kentos_kcad::decode(old).expect_err("refused").code,
        kentos_kcad::Code::BadValue
    );
    let mut straight = model;
    let (entities, uids): (Vec<_>, Vec<_>) = straight
        .entities
        .into_iter()
        .zip(straight.uids)
        .filter(|(e, _)| !matches!(e, Entity::Stair(s) if s.stair.bulges.is_some()))
        .unzip();
    (straight.entities, straight.uids) = (entities, uids);
    let bytes = kentos_kcad::encode_verified(&straight).unwrap();
    assert!(
        !bytes.windows(10).any(|w| w == b"\x67version\x18\x75"),
        "no arc, no schema 117"
    );
}
