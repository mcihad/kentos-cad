//! A niche and a flue opt into schema 119 (docs/adr/0719); schema 118 refuses them, and the same drawing with its
//! service hole only (a bare opening with a sill and a head) stays in the schema before.

use kentos_kcad::contracts::{DocumentSnapshotV2, Entity};

const MODEL: &str = include_str!("../../../../../fixtures/kcad/v2/recesses.json");
const BYTES: &[u8] = include_bytes!("../../../../../fixtures/kcad/v2/recesses.kcad");

#[test]
fn niches_and_flues_opt_into_schema_119_and_118_refuses_them() {
    let model: DocumentSnapshotV2 = serde_json::from_str(MODEL).unwrap();
    assert_eq!(kentos_kcad::encode_verified(&model).unwrap(), BYTES);
    let read = kentos_kcad::decode(BYTES).unwrap();
    assert_eq!(read.entities, model.entities);
    assert!(BYTES.windows(10).any(|w| w == b"\x67version\x18\x77"));
    let old = include_bytes!("../../../../../fixtures/kcad/v2/broken/recesses-in-schema-118.kcad");
    assert_eq!(
        kentos_kcad::decode(old).expect_err("refused").code,
        kentos_kcad::Code::BadValue
    );
    let mut through = model;
    let (entities, uids): (Vec<_>, Vec<_>) = through
        .entities
        .into_iter()
        .zip(through.uids)
        .filter(|(e, _)| !matches!(e, Entity::Opening(o) if o.opening.form.is_recess()))
        .unzip();
    (through.entities, through.uids) = (entities, uids);
    let bytes = kentos_kcad::encode_verified(&through).unwrap();
    assert!(
        !bytes.windows(10).any(|w| w == b"\x67version\x18\x77"),
        "no niche, no flue, no schema 119"
    );
}
