//! Persistent recipes use opt-in schema 113; legacy bytes and every identity survive.

use kentos_kcad::contracts::DocumentSnapshotV2;

const MODEL: &str = include_str!("../../../../../fixtures/kcad/v2/dimension-sets.json");
const BYTES: &[u8] = include_bytes!("../../../../../fixtures/kcad/v2/dimension-sets.kcad");

#[test]
fn recipes_members_suppression_and_layout_survive_all_boundaries() {
    let model: DocumentSnapshotV2 = serde_json::from_str(MODEL).unwrap();
    assert_eq!(kentos_kcad::encode_verified(&model).unwrap(), BYTES);
    let read = kentos_kcad::decode(BYTES).unwrap();
    assert_eq!(read.dimension_sets, model.dimension_sets);
    let (head, columns) = kentos_kcad::split(read).unwrap();
    let (bytes, back) =
        kentos_kcad::encode_columns(&head, &columns, &mut kentos_kcad::Quiet).unwrap();
    let back: DocumentSnapshotV2 = serde_json::from_str(&back).expect("document head JSON");
    assert_eq!(back.dimension_sets, model.dimension_sets);
    assert_eq!(bytes, BYTES);
}

#[test]
fn absent_recipes_keep_the_previous_bytes_and_explicit_empty_is_omitted() {
    let mut model = kentos_kcad::decode(include_bytes!(
        "../../../../../fixtures/kcad/v2/minimal.kcad"
    ))
    .unwrap();
    assert!(model.dimension_sets.is_empty());
    model.dimension_sets.clear();
    assert!(
        serde_json::to_value(&model)
            .unwrap()
            .get("dimensionSets")
            .is_none()
    );
    assert_eq!(
        kentos_kcad::encode_verified(&model).unwrap(),
        include_bytes!("../../../../../fixtures/kcad/v2/minimal.kcad")
    );
}
