//! A block schedule's further columns opt into schema 122 (docs/adr/0721 §4); schema 121 refuses them, and the same
//! drawing without them stays in the schema before. Blank, eleven or not a list: refused.

use kentos_kcad::contracts::{DocumentSnapshotV2, Entity, TableSource};

const MODEL: &str = include_str!("../../../../../fixtures/kcad/v2/table-schedule-columns.json");
const BYTES: &[u8] = include_bytes!("../../../../../fixtures/kcad/v2/table-schedule-columns.kcad");

#[test]
fn a_block_schedule_with_columns_opts_into_schema_122() {
    let model: DocumentSnapshotV2 = serde_json::from_str(MODEL).unwrap();
    assert_eq!(kentos_kcad::encode_verified(&model).unwrap(), BYTES);
    let read = kentos_kcad::decode(BYTES).unwrap();
    let columns: Vec<Vec<String>> = read
        .entities
        .iter()
        .filter_map(|e| match e {
            Entity::Table(t) => match &t.source {
                Some(TableSource::Blocks { columns, .. }) => Some(columns.clone()),
                _ => None,
            },
            _ => None,
        })
        .collect();
    assert_eq!(columns, [vec!["Tarih".to_owned(), "Açıklama".to_owned()], vec![]]);
    assert!(BYTES.windows(10).any(|w| w == b"\x67version\x18\x7a"));
    let old = include_bytes!("../../../../../fixtures/kcad/v2/broken/table-schedule-columns-in-schema-121.kcad");
    assert_eq!(kentos_kcad::decode(old).expect_err("refused").code, kentos_kcad::Code::UnknownField);
    let mut plain = model;
    for e in &mut plain.entities {
        if let Entity::Table(t) = e
            && let Some(TableSource::Blocks { columns, .. }) = &mut t.source
        {
            columns.clear();
        }
    }
    let bytes = kentos_kcad::encode_verified(&plain).unwrap();
    assert!(!bytes.windows(10).any(|w| w == b"\x67version\x18\x7a"), "no columns, no schema 122");
}

#[test]
fn a_block_schedules_columns_keep_their_rules() {
    for name in ["blank", "eleven", "type"] {
        let bytes = std::fs::read(format!(
            "{}/../../../fixtures/kcad/v2/broken/table-schedule-columns-{name}.kcad",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        assert_eq!(kentos_kcad::decode(&bytes).expect_err(name).code, kentos_kcad::Code::BadValue, "{name}");
    }
}
