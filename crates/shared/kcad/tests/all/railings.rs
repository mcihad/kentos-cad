//! A stair's and a ramp's railings opt into schema 115 (docs/adr/0705 §2); schema 114 refuses them, and the same
//! drawing without railings stays in the schema before.

use kentos_kcad::contracts::{DocumentSnapshotV2, Entity};

const MODEL: &str = include_str!("../../../../../fixtures/kcad/v2/railings.json");
const BYTES: &[u8] = include_bytes!("../../../../../fixtures/kcad/v2/railings.kcad");

#[test]
fn railings_opt_into_schema_115_and_114_refuses_them() {
    let model: DocumentSnapshotV2 = serde_json::from_str(MODEL).unwrap();
    assert_eq!(kentos_kcad::encode_verified(&model).unwrap(), BYTES);
    let read = kentos_kcad::decode(BYTES).unwrap();
    assert_eq!(read.entities, model.entities);
    assert!(BYTES.windows(10).any(|w| w == b"\x67version\x18\x73"));
    let old = include_bytes!("../../../../../fixtures/kcad/v2/broken/railings-in-schema-114.kcad");
    assert_eq!(
        kentos_kcad::decode(old).expect_err("refused").code,
        kentos_kcad::Code::BadValue
    );
    let mut plain = model;
    for e in &mut plain.entities {
        match e {
            Entity::Stair(s) => s.stair.railings = None,
            Entity::Ramp(r) => r.ramp.railings = None,
            _ => {}
        }
    }
    let bytes = kentos_kcad::encode_verified(&plain).unwrap();
    assert!(!bytes.windows(10).any(|w| w == b"\x67version\x18\x73"), "no railings, no schema 115");
}
