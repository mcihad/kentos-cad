//! The desktop's objects reach the geometry store as the web's do
//! (docs/adr/0029): `spatial::record` turns each object of the shared
//! drawing in `fixtures/store-records/v1/cases.json` into the store's shape,
//! and packed it gives the numbers and strings the web's `packEntities`
//! gives for the same object (`apps/web/src/viewport/packedRecords.test.ts`).
//! The packed layout reads back to the very shape the store puts
//! (geometry-core's own pack tests), so both hosts put the same objects.

use kentos_contracts::DocumentSnapshotV1;
use kentos_geometry_core::api::json::{self, Json};
use kentos_geometry_core::store::Packer;
use kentos_interaction::spatial::record;

const CASES: &str = include_str!("../../../../fixtures/store-records/v1/cases.json");

/// A record's number: a JSON number, or "NaN".
fn number(v: &Json) -> f64 {
    match v {
        Json::Num(x) => *x,
        Json::Str(s) if s == "NaN" => f64::NAN,
        other => panic!("a number or \"NaN\", not {other:?}"),
    }
}

#[test]
fn every_object_becomes_the_record_the_web_packs() {
    let fixture = Json::parse(CASES).expect("the cases read");
    assert_eq!(
        fixture.get("format"),
        &Json::Str("kentos.store-records".into())
    );
    assert_eq!(fixture.get("version"), &Json::Num(1.0));
    let snapshot = DocumentSnapshotV1::from_json(&json::to_string(fixture.get("document")))
        .expect("the drawing reads");
    let Json::Arr(records) = fixture.get("records") else {
        panic!("records");
    };
    assert_eq!(
        records.len(),
        snapshot.entities.len(),
        "a record per object"
    );
    assert!(records.len() >= 20);
    let mut problems = Vec::new();
    for (entity, want) in snapshot.entities.iter().zip(records) {
        let (id, layer, label, shape) = record(entity);
        let mut packer = Packer::default();
        packer.object(id, layer, label, &shape);
        let Json::Arr(nums) = want.get("nums") else {
            panic!("nums");
        };
        let want_nums: Vec<f64> = nums.iter().map(number).collect();
        let same_nums = packer.nums.len() == want_nums.len()
            && packer
                .nums
                .iter()
                .zip(&want_nums)
                .all(|(a, b)| a.to_bits() == b.to_bits() || (a.is_nan() && b.is_nan()));
        let Json::Arr(strings) = want.get("strings") else {
            panic!("strings");
        };
        let want_strings: Vec<String> = strings
            .iter()
            .map(|s| match s {
                Json::Str(s) => s.clone(),
                other => panic!("a string, not {other:?}"),
            })
            .collect();
        if !same_nums || packer.strings != want_strings {
            problems.push(format!(
                "{} {}: {:?} {:?}, beklenen {want_nums:?} {want_strings:?}",
                entity.kind(),
                entity.base().id,
                packer.nums,
                packer.strings
            ));
        }
    }
    assert!(problems.is_empty(), "\n{}", problems.join("\n"));
}
