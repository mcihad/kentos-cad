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

/// Architecture data 12 (docs/adr/8085): the parcel's area's use is written with `modules.arch = 12` and read back.
#[test]
fn the_zone_round_trips_under_arch_data_12() {
    let mut doc = sample();
    assert!(!kentos_kcad::uses_zone(&doc));
    doc.settings.arch.get_or_insert_with(Default::default).zone = Some(kentos_contracts::Zone::Mixed);
    assert!(kentos_kcad::uses_zone(&doc));
    assert_eq!(kentos_kcad::arch_version(&doc), kentos_kcad::ARCH_WITH_ZONE);
    let bytes = kentos_kcad::encode(&doc).expect("encodes");
    assert_eq!(kentos_kcad::decode(&bytes).expect("decodes"), doc, "the zone reads back");
}

/// Architecture data 13 (docs/adr/8090): a wall's load-bearing role and a roof's escape route role are written with
/// `modules.arch = 13` and read back.
#[test]
fn the_declared_roles_round_trip_under_arch_data_13() {
    use kentos_contracts::{DeclaredBasis, DeclaredBearing, Entity};
    let mut doc = sample();
    assert!(!kentos_kcad::uses_declared_roles(&doc));
    let wall = doc.entities.iter_mut().find_map(|e| match e {
        Entity::Wall(w) => Some(w),
        _ => None,
    });
    wall.expect("a wall").wall.arch.load_bearing = Some(DeclaredBearing { bearing: true, basis: DeclaredBasis::Author });
    assert!(kentos_kcad::uses_declared_roles(&doc));
    assert_eq!(kentos_kcad::arch_version(&doc), kentos_kcad::ARCH_WITH_DECLARED_ROLES);
    let bytes = kentos_kcad::encode(&doc).expect("encodes");
    assert_eq!(kentos_kcad::decode(&bytes).expect("decodes"), doc, "the role reads back");
}

/// Architecture data 14 (docs/adr/8091): a material's reaction to fire class is written with `modules.arch = 14` and
/// read back.
#[test]
fn a_material_s_reaction_class_round_trips_under_arch_data_14() {
    let mut doc = sample();
    assert!(!kentos_kcad::uses_material_reactions(&doc));
    let arch = doc.settings.arch.get_or_insert_with(Default::default);
    let mut m: kentos_contracts::ArchMaterial =
        serde_json::from_value(serde_json::json!({"id": "alcipan", "name": "Alçıpan"})).expect("a material");
    m.reaction = Some(kentos_contracts::Declared::author("A2-s1,d0"));
    arch.materials.push(m);
    assert!(kentos_kcad::uses_material_reactions(&doc));
    assert_eq!(kentos_kcad::arch_version(&doc), kentos_kcad::ARCH_WITH_MATERIAL_REACTIONS);
    let bytes = kentos_kcad::encode(&doc).expect("encodes");
    assert_eq!(kentos_kcad::decode(&bytes).expect("decodes"), doc, "the class reads back");
}

/// Architecture data 15 (docs/adr/8094): the building's structural system is written with `modules.arch = 15` and read
/// back.
#[test]
fn the_building_s_structure_round_trips_under_arch_data_15() {
    let mut doc = sample();
    assert!(!kentos_kcad::uses_structure(&doc));
    doc.settings.arch.get_or_insert_with(Default::default).structure = Some(kentos_contracts::BuildingStructure {
        system: kentos_contracts::StructuralSystem::CutStoneMasonry,
        traditional: true,
    });
    assert!(kentos_kcad::uses_structure(&doc));
    assert_eq!(kentos_kcad::arch_version(&doc), kentos_kcad::ARCH_WITH_STRUCTURE);
    let bytes = kentos_kcad::encode(&doc).expect("encodes");
    assert_eq!(kentos_kcad::decode(&bytes).expect("decodes"), doc, "the structure reads back");
}

/// Architecture data 16 (docs/adr/8098): a plan note of the plan's building height is written with `modules.arch = 16`
/// and read back.
#[test]
fn the_plan_s_building_height_round_trips_under_arch_data_16() {
    let mut doc = sample();
    assert!(!kentos_kcad::uses_plan_limits(&doc));
    doc.settings.arch.get_or_insert_with(Default::default).plan_notes.push(kentos_contracts::PlanNote {
        key: kentos_contracts::PlanNoteKey::BuildingHeight,
        value: 9.5,
        source: "UİP 12.3.2019, plan notu 3".into(),
    });
    assert!(kentos_kcad::uses_plan_limits(&doc));
    assert_eq!(kentos_kcad::arch_version(&doc), kentos_kcad::ARCH_WITH_PLAN_LIMITS);
    let bytes = kentos_kcad::encode(&doc).expect("encodes");
    assert_eq!(kentos_kcad::decode(&bytes).expect("decodes"), doc, "the note reads back");
}

/// Architecture data 17 (docs/adr/8099): the delivery record is written with `modules.arch = 17` and read back.
#[test]
fn the_delivery_record_round_trips_under_arch_data_17() {
    let mut doc = sample();
    assert!(!kentos_kcad::uses_deliveries(&doc));
    doc.settings.arch.get_or_insert_with(Default::default).deliveries.push(kentos_contracts::Delivery {
        id: "00000000-0000-4000-8000-000000000001".into(),
        number: 1,
        revision: "000".into(),
        package: kentos_contracts::DeliveryPackage { manifest: "0".repeat(64), files: 14, path: None, source: None },
        to: "Sivas Belediyesi".into(),
        channel: kentos_contracts::DeliveryChannel::Epys,
        at: "2026-11-17".into(),
        by: None,
        receipt: Some("2026-0001".into()),
        response: None,
    });
    assert!(kentos_kcad::uses_deliveries(&doc));
    assert_eq!(kentos_kcad::arch_version(&doc), kentos_kcad::ARCH_WITH_DELIVERIES);
    let bytes = kentos_kcad::encode(&doc).expect("encodes");
    assert_eq!(kentos_kcad::decode(&bytes).expect("decodes"), doc, "the record reads back");
}
