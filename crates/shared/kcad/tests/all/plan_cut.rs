//! The project's plan cut height opts into schema 205 (docs/adr/3012); schema 204 refuses it, and the same drawing
//! without it stays in the schema before.

use kentos_kcad::contracts::DocumentSnapshotV2;

const MODEL: &str = include_str!("../../../../../fixtures/kcad/v2/plan-cut.json");
const BYTES: &[u8] = include_bytes!("../../../../../fixtures/kcad/v2/plan-cut.kcad");

#[test]
fn the_plan_cut_height_opts_into_schema_205() {
    let model: DocumentSnapshotV2 = serde_json::from_str(MODEL).unwrap();
    assert_eq!(kentos_kcad::encode_verified(&model).unwrap(), BYTES);
    let read = kentos_kcad::decode(BYTES).unwrap();
    assert_eq!(read.settings.arch.as_ref().unwrap().cut_height, Some(1.2));
    assert!(BYTES.windows(10).any(|w| w == b"\x67version\x18\xcd"));
    let old = include_bytes!("../../../../../fixtures/kcad/v2/broken/plan-cut-in-schema-204.kcad");
    assert_eq!(kentos_kcad::decode(old).expect_err("refused").code, kentos_kcad::Code::UnknownField);
    let mut plain = model;
    if let Some(a) = plain.settings.arch.as_mut() {
        a.cut_height = None;
    }
    let bytes = kentos_kcad::encode_verified(&plain).unwrap();
    assert!(!bytes.windows(10).any(|w| w == b"\x67version\x18\xcd"), "nothing said, no schema 205");
}
