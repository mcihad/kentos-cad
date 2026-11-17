//! Architecture data 11 (docs/adr/8084): the parcel's plan notes are written with `modules.arch = 11` and read back as
//! they were; a drawing without them stays as an older KentOS reads it.

use kentos_contracts::{PlanNote, PlanNoteKey};

fn sample() -> kentos_contracts::DocumentSnapshotV2 {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../fixtures/arch/v1/section-building.kcad");
    kentos_kcad::decode(&std::fs::read(path).expect("the building")).expect("decodes")
}

#[test]
fn the_plan_notes_round_trip_under_arch_data_11() {
    let plain = sample();
    assert!(!kentos_kcad::uses_plan_notes(&plain));
    assert!(kentos_kcad::arch_version(&plain) < kentos_kcad::ARCH_WITH_PLAN_NOTES);
    let again = kentos_kcad::decode(&kentos_kcad::encode(&plain).expect("encodes")).expect("decodes");
    assert_eq!(again, plain, "a drawing without plan notes is as it was");

    let mut doc = plain.clone();
    doc.settings.arch.get_or_insert_with(Default::default).plan_notes = vec![
        PlanNote { key: PlanNoteKey::GroundStoreyHeight, value: 4.5, source: "UİP 12.3.2019, plan notu 7".into() },
        PlanNote { key: PlanNoteKey::StoreyHeight, value: 3.2, source: "UİP 12.3.2019, plan notu 8".into() },
    ];
    assert!(kentos_kcad::uses_plan_notes(&doc));
    assert_eq!(kentos_kcad::arch_version(&doc), kentos_kcad::ARCH_WITH_PLAN_NOTES);
    let bytes = kentos_kcad::encode(&doc).expect("encodes");
    assert_eq!(kentos_kcad::decode(&bytes).expect("decodes"), doc, "the notes read back");
}
