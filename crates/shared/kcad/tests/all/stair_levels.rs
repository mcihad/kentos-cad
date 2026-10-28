//! A stair whose rise is its level's height, or a curved stair with a centre post, opts into schema 125 (docs/adr/0724);
//! schema 124 refuses them, and they keep the contract's rule (the level given, only true written, a post on a curved
//! stair narrower than its inner side).

use kentos_kcad::contracts::{DocumentSnapshotV2, Entity};

const MODEL: &str = include_str!("../../../../../fixtures/kcad/v2/stair-levels.json");
const BYTES: &[u8] = include_bytes!("../../../../../fixtures/kcad/v2/stair-levels.kcad");

#[test]
fn a_stair_bound_to_its_level_or_with_a_post_opts_into_schema_125() {
    let model: DocumentSnapshotV2 = serde_json::from_str(MODEL).unwrap();
    assert_eq!(kentos_kcad::encode_verified(&model).unwrap(), BYTES);
    let read = kentos_kcad::decode(BYTES).unwrap();
    let stairs: Vec<_> = read
        .entities
        .iter()
        .filter_map(|e| match e {
            Entity::Stair(s) => Some(&s.stair),
            _ => None,
        })
        .collect();
    assert_eq!(
        stairs.iter().map(|s| (s.height_from_level, s.post)).collect::<Vec<_>>(),
        [(true, None), (true, Some(0.3)), (false, None)]
    );
    assert!(BYTES.windows(10).any(|w| w == b"\x67version\x18\x7d"));
    let old = include_bytes!("../../../../../fixtures/kcad/v2/broken/stair-levels-in-schema-124.kcad");
    assert_eq!(kentos_kcad::decode(old).expect_err("refused").code, kentos_kcad::Code::BadValue);
    // The same drawing without them stays in the schema before.
    let mut plain = model;
    for e in &mut plain.entities {
        if let Entity::Stair(s) = e {
            s.stair.height_from_level = false;
            s.stair.post = None;
        }
    }
    let bytes = kentos_kcad::encode_verified(&plain).unwrap();
    assert!(!bytes.windows(10).any(|w| w == b"\x67version\x18\x7d"), "none, no schema 125");
}

#[test]
fn a_stairs_level_and_post_keep_their_rules() {
    for name in ["without-level", "false", "post-straight", "post-wide"] {
        let bytes = std::fs::read(format!(
            "{}/../../../fixtures/kcad/v2/broken/stair-level-{name}.kcad",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        assert_eq!(kentos_kcad::decode(&bytes).expect_err(name).code, kentos_kcad::Code::BadValue, "{name}");
    }
}
