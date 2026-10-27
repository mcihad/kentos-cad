//! The level and view a drawing opens with opt into schema 120 (docs/adr/0720); schema 119 refuses them, and the same
//! drawing without them stays in the schema before. A level's id and Katsız (null) are both written.

use kentos_kcad::contracts::{DocumentSnapshotV2, LevelView};

const MODEL: &str = include_str!("../../../../../fixtures/kcad/v2/level-state.json");
const BYTES: &[u8] = include_bytes!("../../../../../fixtures/kcad/v2/level-state.kcad");
const UNASSIGNED: &str = include_str!("../../../../../fixtures/kcad/v2/level-state-unassigned.json");

#[test]
fn the_level_and_view_a_drawing_opens_with_opt_into_schema_120() {
    let model: DocumentSnapshotV2 = serde_json::from_str(MODEL).unwrap();
    assert_eq!(kentos_kcad::encode_verified(&model).unwrap(), BYTES);
    let read = kentos_kcad::decode(BYTES).unwrap();
    let arch = read.settings.arch.as_ref().unwrap();
    assert_eq!((arch.active_level.clone(), arch.level_view), (Some(Some("kat-1".into())), Some(LevelView::Below)));
    assert!(BYTES.windows(10).any(|w| w == b"\x67version\x18\x78"));
    let old = include_bytes!("../../../../../fixtures/kcad/v2/broken/level-state-in-schema-119.kcad");
    assert_eq!(kentos_kcad::decode(old).expect_err("refused").code, kentos_kcad::Code::UnknownField);
    let mut plain = model;
    if let Some(a) = plain.settings.arch.as_mut() {
        (a.active_level, a.level_view) = (None, None);
    }
    let bytes = kentos_kcad::encode_verified(&plain).unwrap();
    assert!(!bytes.windows(10).any(|w| w == b"\x67version\x18\x78"), "nothing said, no schema 120");
    // Katsız is a null, read back as such.
    let katsiz: DocumentSnapshotV2 = serde_json::from_str(UNASSIGNED).unwrap();
    let back = kentos_kcad::decode(&kentos_kcad::encode_verified(&katsiz).unwrap()).unwrap();
    assert_eq!(back.settings.arch.unwrap().active_level, Some(None));
}
