//! The drawing's constraints in the document (docs/adr/0334 §1): a step that deletes a constrained object takes its
//! constraints with it, a dimension's parameter with its constraint unless another parameter reads it, and undo
//! brings both back; the drawing stays as the file format can keep it.

use kentos_domain::contracts::{DocumentSnapshotV2, EntityId, constraints_problem};
use kentos_domain::{Document, Uuid};

const SAMPLE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../fixtures/kcad/v2/constraints.json"
);

fn sample() -> Document {
    let text = std::fs::read_to_string(SAMPLE).expect("the sample");
    let snapshot: DocumentSnapshotV2 = serde_json::from_str(&text).expect("reads");
    Document::from_snapshot_v2(snapshot).expect("opens")
}

fn slot(doc: &Document, last: u8) -> kentos_domain::Slot {
    let uid = Uuid::parse_str(&format!(
        "0192f5a6-1000-7000-8000-000000000{}",
        100 + u32::from(last)
    ))
    .expect("uid");
    doc.slot_of(uid).expect("in the drawing")
}

fn names(doc: &Document) -> Vec<String> {
    doc.constraints()
        .parameters
        .iter()
        .map(|p| p.name.clone())
        .collect()
}

fn keeps_the_rules(doc: &Document) {
    let snapshot = doc.to_snapshot_v2();
    let uids = snapshot.uids.iter().copied().collect();
    assert_eq!(
        constraints_problem(&snapshot.constraints, Some(&uids)),
        None
    );
}

#[test]
fn a_deleted_object_takes_its_constraints_and_their_parameters() {
    let mut doc = sample();
    assert_eq!(doc.constraints().constraints.len(), 16);
    // The diagonal: its corner, its angle and its reference length go; nobody reads `a1` nor `d3`.
    let diagonal = slot(&doc, 6);
    doc.remove(&[diagonal]);
    assert_eq!(doc.constraints().constraints.len(), 13);
    assert_eq!(names(&doc), ["genislik", "d1", "d2", "r1"]);
    keeps_the_rules(&doc);
    doc.undo();
    assert_eq!(doc.constraints().constraints.len(), 16);
    assert_eq!(names(&doc), ["genislik", "d1", "d2", "r1", "a1", "d3"]);
}

#[test]
fn a_parameter_another_reads_stays() {
    let mut doc = sample();
    // The right edge: its corners, its upright and its height `d2` go; `r1` and `a1` read `d2`, so it stays.
    let right = slot(&doc, 2);
    doc.remove(&[right]);
    let set = doc.constraints();
    assert!(
        set.constraints
            .iter()
            .all(|c| c.rule.elements().iter().all(|e| e.uid
                != EntityId(
                    Uuid::parse_str("0192f5a6-1000-7000-8000-000000000102")
                        .expect("uid")
                        .into_bytes()
                )))
    );
    assert_eq!(
        set.parameter("d2").map(|p| p.expression.as_str()),
        Some("4")
    );
    keeps_the_rules(&doc);
}
