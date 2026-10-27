//! A space that is a void opts into schema 116 (docs/adr/0708); schema 115 refuses its `void`, and the same drawing
//! with rooms only stays in the schema before.

use kentos_kcad::contracts::{DocumentSnapshotV2, Entity};

const MODEL: &str = include_str!("../../../../../fixtures/kcad/v2/space-voids.json");
const BYTES: &[u8] = include_bytes!("../../../../../fixtures/kcad/v2/space-voids.kcad");

#[test]
fn voids_opt_into_schema_116_and_115_refuses_them() {
    let model: DocumentSnapshotV2 = serde_json::from_str(MODEL).unwrap();
    assert_eq!(kentos_kcad::encode_verified(&model).unwrap(), BYTES);
    let read = kentos_kcad::decode(BYTES).unwrap();
    assert_eq!(read.entities, model.entities);
    assert!(BYTES.windows(10).any(|w| w == b"\x67version\x18\x74"));
    let old = include_bytes!("../../../../../fixtures/kcad/v2/broken/space-voids-in-schema-115.kcad");
    assert_eq!(
        kentos_kcad::decode(old).expect_err("refused").code,
        kentos_kcad::Code::UnknownField
    );
    let mut rooms = model;
    for e in &mut rooms.entities {
        if let Entity::Space(s) = e {
            s.space.void = false;
        }
    }
    let bytes = kentos_kcad::encode_verified(&rooms).unwrap();
    assert!(!bytes.windows(10).any(|w| w == b"\x67version\x18\x74"), "no void, no schema 116");
}
