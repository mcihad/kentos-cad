//! How the project's plan lines are weighted opts into schema 218 (docs/adr/8013); schema 217 refuses it, an unknown
//! group is a bad value, and the same drawing without it stays in the schema before.

use kentos_kcad::contracts::{DocumentSnapshotV2, LineGroups};

const MODEL: &str = include_str!("../../../../../fixtures/kcad/v2/line-groups.json");
const BYTES: &[u8] = include_bytes!("../../../../../fixtures/kcad/v2/line-groups.kcad");

#[test]
fn the_line_groups_opt_into_schema_206() {
    let model: DocumentSnapshotV2 = serde_json::from_str(MODEL).unwrap();
    assert_eq!(kentos_kcad::encode_verified(&model).unwrap(), BYTES);
    let read = kentos_kcad::decode(BYTES).unwrap();
    assert_eq!(
        read.settings.arch.as_ref().unwrap().line_groups,
        Some(LineGroups::ByScale)
    );
    assert!(BYTES.windows(10).any(|w| w == b"\x67version\x18\xda"));
    let old =
        include_bytes!("../../../../../fixtures/kcad/v2/broken/line-groups-in-schema-217.kcad");
    assert_eq!(
        kentos_kcad::decode(old).expect_err("refused").code,
        kentos_kcad::Code::UnknownField
    );
    let odd = include_bytes!("../../../../../fixtures/kcad/v2/broken/line-groups-unknown.kcad");
    assert_eq!(
        kentos_kcad::decode(odd).expect_err("refused").code,
        kentos_kcad::Code::BadValue
    );
    let mut plain = model;
    if let Some(a) = plain.settings.arch.as_mut() {
        a.line_groups = None;
    }
    let bytes = kentos_kcad::encode_verified(&plain).unwrap();
    assert!(
        !bytes.windows(10).any(|w| w == b"\x67version\x18\xda"),
        "nothing said, no schema 218"
    );
}
