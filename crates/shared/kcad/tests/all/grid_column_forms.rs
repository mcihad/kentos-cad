//! A gapped grid, an anchored column and an L column opt into schema 126 (docs/adr/0725); schema 125 refuses them, and
//! they keep the contract's rule (ends above 0 and at most 1000 mm, legs an L's only, thinner than its sides, no
//! default anchor written).

use kentos_kcad::contracts::{ColumnAnchor, ColumnShape, DocumentSnapshotV2, Entity};

const MODEL: &str = include_str!("../../../../../fixtures/kcad/v2/grid-column-forms.json");
const BYTES: &[u8] = include_bytes!("../../../../../fixtures/kcad/v2/grid-column-forms.kcad");

#[test]
fn gapped_grids_anchored_and_l_columns_opt_into_schema_126() {
    let model: DocumentSnapshotV2 = serde_json::from_str(MODEL).unwrap();
    assert_eq!(kentos_kcad::encode_verified(&model).unwrap(), BYTES);
    let read = kentos_kcad::decode(BYTES).unwrap();
    let ends: Vec<_> = read
        .entities
        .iter()
        .filter_map(|e| match e {
            Entity::Grid(g) => Some(g.grid.ends),
            _ => None,
        })
        .collect();
    assert_eq!(ends, [Some(12.0), None, Some(8.0)]);
    let columns: Vec<_> = read
        .entities
        .iter()
        .filter_map(|e| match e {
            Entity::Column(c) => Some((c.column.shape, c.column.anchor, c.column.legs)),
            _ => None,
        })
        .collect();
    assert_eq!(
        columns,
        [
            (ColumnShape::Rect, ColumnAnchor::BottomLeft, None),
            (ColumnShape::LShape, ColumnAnchor::Top, Some([0.25, 0.3])),
            (ColumnShape::Rect, ColumnAnchor::Center, None),
        ]
    );
    assert!(BYTES.windows(10).any(|w| w == b"\x67version\x18\x7e"));
    let old = include_bytes!("../../../../../fixtures/kcad/v2/broken/grid-column-forms-in-schema-125.kcad");
    assert_eq!(kentos_kcad::decode(old).expect_err("refused").code, kentos_kcad::Code::BadValue);
    // The same drawing without them stays in the schema before.
    let mut plain = model;
    for e in &mut plain.entities {
        match e {
            Entity::Grid(g) => g.grid.ends = None,
            Entity::Column(c) => {
                c.column.anchor = ColumnAnchor::Center;
                c.column.legs = None;
                c.column.shape = ColumnShape::Rect;
            }
            _ => {}
        }
    }
    let bytes = kentos_kcad::encode_verified(&plain).unwrap();
    assert!(!bytes.windows(10).any(|w| w == b"\x67version\x18\x7e"), "none, no schema 126");
}

#[test]
fn gapped_grids_and_column_forms_keep_their_rules() {
    for name in ["ends-zero", "ends-far", "legs-not-l", "l-without-legs", "legs-thick", "anchor-default", "anchor-unknown"] {
        let bytes = std::fs::read(format!(
            "{}/../../../fixtures/kcad/v2/broken/grid-column-form-{name}.kcad",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        assert_eq!(kentos_kcad::decode(&bytes).expect_err(name).code, kentos_kcad::Code::BadValue, "{name}");
    }
}
