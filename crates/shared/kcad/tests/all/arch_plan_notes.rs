//! Architecture data 11 (docs/adr/8084): the parcel's plan notes are written with `modules.arch = 11` and read back as
//! they were; a drawing without them stays as an older KentOS reads it.

use kentos_contracts::{PlanNote, PlanNoteKey};

fn sample() -> kentos_contracts::DocumentSnapshotV2 {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../fixtures/arch/v1/section-building.kcad"
    );
    kentos_kcad::decode(&std::fs::read(path).expect("the building")).expect("decodes")
}

#[test]
fn the_plan_notes_round_trip_under_arch_data_11() {
    let plain = sample();
    assert!(!kentos_kcad::uses_plan_notes(&plain));
    assert!(kentos_kcad::arch_version(&plain) < kentos_kcad::ARCH_WITH_PLAN_NOTES);
    let again =
        kentos_kcad::decode(&kentos_kcad::encode(&plain).expect("encodes")).expect("decodes");
    assert_eq!(again, plain, "a drawing without plan notes is as it was");

    let mut doc = plain.clone();
    doc.settings
        .arch
        .get_or_insert_with(Default::default)
        .plan_notes = vec![
        PlanNote {
            key: PlanNoteKey::GroundStoreyHeight,
            value: 4.5,
            source: "UİP 12.3.2019, plan notu 7".into(),
        },
        PlanNote {
            key: PlanNoteKey::StoreyHeight,
            value: 3.2,
            source: "UİP 12.3.2019, plan notu 8".into(),
        },
    ];
    assert!(kentos_kcad::uses_plan_notes(&doc));
    assert_eq!(
        kentos_kcad::arch_version(&doc),
        kentos_kcad::ARCH_WITH_PLAN_NOTES
    );
    let bytes = kentos_kcad::encode(&doc).expect("encodes");
    assert_eq!(
        kentos_kcad::decode(&bytes).expect("decodes"),
        doc,
        "the notes read back"
    );
}

/// Architecture data 12 (docs/adr/8085): the parcel's area's use is written with `modules.arch = 12` and read back.
#[test]
fn the_zone_round_trips_under_arch_data_12() {
    let mut doc = sample();
    assert!(!kentos_kcad::uses_zone(&doc));
    doc.settings.arch.get_or_insert_with(Default::default).zone =
        Some(kentos_contracts::Zone::Mixed);
    assert!(kentos_kcad::uses_zone(&doc));
    assert_eq!(kentos_kcad::arch_version(&doc), kentos_kcad::ARCH_WITH_ZONE);
    let bytes = kentos_kcad::encode(&doc).expect("encodes");
    assert_eq!(
        kentos_kcad::decode(&bytes).expect("decodes"),
        doc,
        "the zone reads back"
    );
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
    wall.expect("a wall").wall.arch.load_bearing = Some(DeclaredBearing {
        bearing: true,
        basis: DeclaredBasis::Author,
    });
    assert!(kentos_kcad::uses_declared_roles(&doc));
    assert_eq!(
        kentos_kcad::arch_version(&doc),
        kentos_kcad::ARCH_WITH_DECLARED_ROLES
    );
    let bytes = kentos_kcad::encode(&doc).expect("encodes");
    assert_eq!(
        kentos_kcad::decode(&bytes).expect("decodes"),
        doc,
        "the role reads back"
    );
}

/// Architecture data 14 (docs/adr/8091): a material's reaction to fire class is written with `modules.arch = 14` and
/// read back.
#[test]
fn a_material_s_reaction_class_round_trips_under_arch_data_14() {
    let mut doc = sample();
    assert!(!kentos_kcad::uses_material_reactions(&doc));
    let arch = doc.settings.arch.get_or_insert_with(Default::default);
    let mut m: kentos_contracts::ArchMaterial =
        serde_json::from_value(serde_json::json!({"id": "alcipan", "name": "Alçıpan"}))
            .expect("a material");
    m.reaction = Some(kentos_contracts::Declared::author("A2-s1,d0"));
    arch.materials.push(m);
    assert!(kentos_kcad::uses_material_reactions(&doc));
    assert_eq!(
        kentos_kcad::arch_version(&doc),
        kentos_kcad::ARCH_WITH_MATERIAL_REACTIONS
    );
    let bytes = kentos_kcad::encode(&doc).expect("encodes");
    assert_eq!(
        kentos_kcad::decode(&bytes).expect("decodes"),
        doc,
        "the class reads back"
    );
}

/// Architecture data 24 (docs/adr/8135): a lift shaft room's vent is written with `modules.arch = 24` and read back.
#[test]
fn a_lift_shaft_s_vent_round_trips_under_arch_data_24() {
    let mut doc = sample();
    assert!(!kentos_kcad::uses_shaft_vents(&doc));
    let space = doc
        .entities
        .iter_mut()
        .find_map(|e| match e {
            kentos_contracts::Entity::Space(s) => Some(s),
            _ => None,
        })
        .expect("a room");
    space.space.name = Some("Asansör".into());
    space.space.arch.shaft_vent = Some(kentos_contracts::DeclaredShaftVent {
        area: Some(0.12),
        pressurized: false,
        basis: kentos_contracts::DeclaredBasis::Author,
    });
    assert!(kentos_kcad::uses_shaft_vents(&doc));
    assert_eq!(
        kentos_kcad::arch_version(&doc),
        kentos_kcad::ARCH_WITH_SHAFT_VENTS
    );
    let bytes = kentos_kcad::encode(&doc).expect("encodes");
    assert_eq!(
        kentos_kcad::decode(&bytes).expect("decodes"),
        doc,
        "the vent reads back"
    );
}

/// Architecture data 26 (docs/adr/8137): a wall style's load-bearing role is written with `modules.arch = 26` and read
/// back.
#[test]
fn a_wall_style_s_load_bearing_role_round_trips_under_arch_data_26() {
    let mut doc = sample();
    assert!(!kentos_kcad::uses_style_bearing(&doc));
    let mut style = plain_style();
    style.load_bearing = Some(kentos_contracts::DeclaredBearing {
        bearing: true,
        basis: kentos_contracts::DeclaredBasis::Author,
    });
    doc.settings.wall_styles.push(style);
    assert!(kentos_kcad::uses_style_bearing(&doc));
    assert_eq!(
        kentos_kcad::arch_version(&doc),
        kentos_kcad::ARCH_WITH_STYLE_BEARING
    );
    let bytes = kentos_kcad::encode(&doc).expect("encodes");
    assert_eq!(
        kentos_kcad::decode(&bytes).expect("decodes"),
        doc,
        "the role reads back"
    );
}

/// Architecture data 27 (docs/adr/8140): where İmar durumu's emsal and TAKS are read from is written with
/// `modules.arch = 27` and read back; the same file saying 26 is refused, as an older reader refuses data it has not.
#[test]
fn the_zoning_s_sources_round_trip_under_arch_data_27() {
    let mut doc = sample();
    assert!(!kentos_kcad::uses_zoning_sources(&doc));
    doc.settings
        .arch
        .get_or_insert_with(Default::default)
        .zoning = Some(kentos_contracts::Zoning {
        parcel_area: 900.0,
        far: Some(1.2),
        coverage: Some(0.4),
        far_source: Some(
            "UIP-1 Merkez uygulama imar planı (yururlukte, onay 2024-05-14), karar K1".into(),
        ),
        coverage_source: None,
    });
    assert!(kentos_kcad::uses_zoning_sources(&doc));
    assert_eq!(
        kentos_kcad::arch_version(&doc),
        kentos_kcad::ARCH_WITH_ZONING_SOURCES
    );
    let bytes = kentos_kcad::encode(&doc).expect("encodes");
    assert_eq!(
        kentos_kcad::decode(&bytes).expect("decodes"),
        doc,
        "the sources read back"
    );

    // `modules` says `arch` 26.
    let e = kentos_kcad::decode(&saying_arch(&bytes, 27, 26)).expect_err("refused");
    assert_eq!(e.code, kentos_kcad::Code::UnknownField, "{e}");
    assert!(e.message.contains("sürüm 27'den önce yok"), "{e}");
}

/// Architecture data 28 (docs/adr/8141): the building's layout is written with `modules.arch = 28` and read back; the
/// same file saying 27 is refused.
#[test]
fn the_layout_round_trips_under_arch_data_28() {
    let mut doc = sample();
    assert!(!kentos_kcad::uses_layout(&doc));
    doc.settings
        .arch
        .get_or_insert_with(Default::default)
        .layout = Some(kentos_contracts::BuildingLayout::Attached);
    assert!(kentos_kcad::uses_layout(&doc));
    assert_eq!(
        kentos_kcad::arch_version(&doc),
        kentos_kcad::ARCH_WITH_LAYOUT
    );
    let bytes = kentos_kcad::encode(&doc).expect("encodes");
    assert_eq!(
        kentos_kcad::decode(&bytes).expect("decodes"),
        doc,
        "the layout reads back"
    );
    let e = kentos_kcad::decode(&saying_arch(&bytes, 28, 27)).expect_err("refused");
    assert_eq!(e.code, kentos_kcad::Code::UnknownField, "{e}");
    assert!(e.message.contains("sürüm 28'den önce yok"), "{e}");
}

/// Architecture data 29 (docs/adr/8142): a site plan's party edge is written with `modules.arch = 29` and read back; the
/// same file saying 28 is refused.
#[test]
fn a_party_edge_round_trips_under_arch_data_29() {
    use kentos_contracts::{SiteEdge, SiteEdgeKind, SitePlan};
    let mut doc = sample();
    assert!(!kentos_kcad::uses_party_edges(&doc));
    doc.settings.arch.get_or_insert_with(Default::default).site = Some(SitePlan {
        edges: vec![
            SiteEdge {
                kind: SiteEdgeKind::Front,
                setback: None,
            },
            SiteEdge {
                kind: SiteEdgeKind::Party,
                setback: None,
            },
        ],
        ..Default::default()
    });
    assert!(kentos_kcad::uses_party_edges(&doc));
    assert_eq!(
        kentos_kcad::arch_version(&doc),
        kentos_kcad::ARCH_WITH_PARTY_EDGES
    );
    let bytes = kentos_kcad::encode(&doc).expect("encodes");
    assert_eq!(
        kentos_kcad::decode(&bytes).expect("decodes"),
        doc,
        "the party edge reads back"
    );
    let e = kentos_kcad::decode(&saying_arch(&bytes, 29, 28)).expect_err("refused");
    assert_eq!(e.code, kentos_kcad::Code::UnknownField, "{e}");
    assert!(e.message.contains("sürüm 29'dan önce yok"), "{e}");
}

/// Architecture data 30 (docs/adr/8144): a program building's own Bölge is written with `modules.arch = 30` and read
/// back; the same file saying 29 is refused.
#[test]
fn a_building_s_zone_round_trips_under_arch_data_30() {
    let mut doc = sample();
    assert!(!kentos_kcad::uses_building_zones(&doc));
    let arch = doc.settings.arch.get_or_insert_with(Default::default);
    arch.program
        .buildings
        .push(kentos_contracts::ProgramBuilding {
            id: "a".into(),
            name: "A Blok".into(),
            zone: Some(kentos_contracts::Zone::Commercial),
            ..Default::default()
        });
    assert!(kentos_kcad::uses_building_zones(&doc));
    assert_eq!(
        kentos_kcad::arch_version(&doc),
        kentos_kcad::ARCH_WITH_BUILDING_ZONES
    );
    let bytes = kentos_kcad::encode(&doc).expect("encodes");
    assert_eq!(
        kentos_kcad::decode(&bytes).expect("decodes"),
        doc,
        "the zone reads back"
    );
    let e = kentos_kcad::decode(&saying_arch(&bytes, 30, 29)).expect_err("refused");
    assert_eq!(e.code, kentos_kcad::Code::UnknownField, "{e}");
    assert!(e.message.contains("sürüm 30'dan önce yok"), "{e}");
}

/// Architecture data 31 (docs/adr/8145): a program building's own structural system is written with `modules.arch = 31`
/// and read back; the same file saying 30 is refused.
#[test]
fn a_building_s_structure_round_trips_under_arch_data_31() {
    let mut doc = sample();
    assert!(!kentos_kcad::uses_building_structures(&doc));
    let arch = doc.settings.arch.get_or_insert_with(Default::default);
    arch.program
        .buildings
        .push(kentos_contracts::ProgramBuilding {
            id: "a".into(),
            name: "A Blok".into(),
            structure: Some(kentos_contracts::BuildingStructure {
                system: kentos_contracts::StructuralSystem::CutStoneMasonry,
                traditional: true,
            }),
            ..Default::default()
        });
    assert!(kentos_kcad::uses_building_structures(&doc));
    assert_eq!(
        kentos_kcad::arch_version(&doc),
        kentos_kcad::ARCH_WITH_BUILDING_STRUCTURES
    );
    let bytes = kentos_kcad::encode(&doc).expect("encodes");
    assert_eq!(
        kentos_kcad::decode(&bytes).expect("decodes"),
        doc,
        "the structure reads back"
    );
    let e = kentos_kcad::decode(&saying_arch(&bytes, 31, 30)).expect_err("refused");
    assert_eq!(e.code, kentos_kcad::Code::UnknownField, "{e}");
    assert!(e.message.contains("sürüm 31'den önce yok"), "{e}");
}

/// Architecture data 32 (docs/adr/8158): a plan note of the subasman is written with `modules.arch = 32` and read back;
/// the same file saying 31 is refused.
#[test]
fn a_plan_s_subasman_round_trips_under_arch_data_32() {
    let mut doc = sample();
    assert!(!kentos_kcad::uses_plan_plinth(&doc));
    let arch = doc.settings.arch.get_or_insert_with(Default::default);
    arch.plan_notes.push(PlanNote {
        key: PlanNoteKey::PlinthMost,
        value: 2.4,
        source: "UİP 12.3.2019, plan notu 56".into(),
    });
    arch.plan_notes.push(PlanNote {
        key: PlanNoteKey::PlinthLeast,
        value: -0.5,
        source: "UİP 12.3.2019, plan notu 56".into(),
    });
    assert!(kentos_kcad::uses_plan_plinth(&doc));
    assert_eq!(
        kentos_kcad::arch_version(&doc),
        kentos_kcad::ARCH_WITH_PLAN_PLINTH
    );
    let bytes = kentos_kcad::encode(&doc).expect("encodes");
    assert_eq!(
        kentos_kcad::decode(&bytes).expect("decodes"),
        doc,
        "the notes read back"
    );
    let e = kentos_kcad::decode(&saying_arch(&bytes, 32, 31)).expect_err("refused");
    assert_eq!(e.code, kentos_kcad::Code::UnknownField, "{e}");
    assert!(e.message.contains("sürüm 32'den önce yok"), "{e}");
}

/// Architecture data 33 (docs/adr/8160): the plan's roof notes are written with `modules.arch = 33` and read back; the
/// same file saying 32 is refused.
#[test]
fn a_plan_s_roof_notes_round_trip_under_arch_data_33() {
    let mut doc = sample();
    assert!(!kentos_kcad::uses_plan_roof(&doc));
    let arch = doc.settings.arch.get_or_insert_with(Default::default);
    let notes = [
        (PlanNoteKey::RidgeMost, 3.5),
        (PlanNoteKey::RoofSlopeMost, 33.0),
        (PlanNoteKey::EavesMost, 0.8),
    ];
    for (key, value) in notes {
        arch.plan_notes.push(PlanNote {
            key,
            value,
            source: "UİP 12.3.2019, plan notu 12".into(),
        });
    }
    assert!(kentos_kcad::uses_plan_roof(&doc));
    assert_eq!(
        kentos_kcad::arch_version(&doc),
        kentos_kcad::ARCH_WITH_PLAN_ROOF
    );
    let bytes = kentos_kcad::encode(&doc).expect("encodes");
    assert_eq!(
        kentos_kcad::decode(&bytes).expect("decodes"),
        doc,
        "the notes read back"
    );
    let e = kentos_kcad::decode(&saying_arch(&bytes, 33, 32)).expect_err("refused");
    assert_eq!(e.code, kentos_kcad::Code::UnknownField, "{e}");
    assert!(e.message.contains("sürüm 33'ten önce yok"), "{e}");
}

/// Architecture data 34 (docs/adr/8162): the plan's projection and basement notes are written with `modules.arch = 34`
/// and read back; the same file saying 33 is refused.
#[test]
fn a_plan_s_projection_and_basement_notes_round_trip_under_arch_data_34() {
    let mut doc = sample();
    assert!(!kentos_kcad::uses_plan_mass(&doc));
    let arch = doc.settings.arch.get_or_insert_with(Default::default);
    for (key, value) in [
        (PlanNoteKey::ProjectionMost, 1.0),
        (PlanNoteKey::BasementsMost, 2.0),
    ] {
        arch.plan_notes.push(PlanNote {
            key,
            value,
            source: "UİP 12.3.2019, plan notu 9".into(),
        });
    }
    assert!(kentos_kcad::uses_plan_mass(&doc));
    assert_eq!(
        kentos_kcad::arch_version(&doc),
        kentos_kcad::ARCH_WITH_PLAN_MASS
    );
    let bytes = kentos_kcad::encode(&doc).expect("encodes");
    assert_eq!(
        kentos_kcad::decode(&bytes).expect("decodes"),
        doc,
        "the notes read back"
    );
    let e = kentos_kcad::decode(&saying_arch(&bytes, 34, 33)).expect_err("refused");
    assert_eq!(e.code, kentos_kcad::Code::UnknownField, "{e}");
    assert!(e.message.contains("sürüm 34'ten önce yok"), "{e}");
}

/// Architecture data 35 (docs/adr/8163): a program building's own plan notes are written with `modules.arch = 35` and
/// read back; the same file saying 34 is refused.
#[test]
fn a_building_s_plan_notes_round_trip_under_arch_data_35() {
    let mut doc = sample();
    assert!(!kentos_kcad::uses_building_plan_notes(&doc));
    let arch = doc.settings.arch.get_or_insert_with(Default::default);
    arch.program
        .buildings
        .push(kentos_contracts::ProgramBuilding {
            id: "a".into(),
            name: "A Blok".into(),
            plan_notes: vec![PlanNote {
                key: PlanNoteKey::BuildingHeight,
                value: 12.5,
                source: "UİP 12.3.2019, kademe hattı A alanı".into(),
            }],
            ..Default::default()
        });
    assert!(kentos_kcad::uses_building_plan_notes(&doc));
    assert_eq!(
        kentos_kcad::arch_version(&doc),
        kentos_kcad::ARCH_WITH_BUILDING_PLAN_NOTES
    );
    let bytes = kentos_kcad::encode(&doc).expect("encodes");
    assert_eq!(
        kentos_kcad::decode(&bytes).expect("decodes"),
        doc,
        "the notes read back"
    );
    let e = kentos_kcad::decode(&saying_arch(&bytes, 35, 34)).expect_err("refused");
    assert_eq!(e.code, kentos_kcad::Code::UnknownField, "{e}");
    assert!(e.message.contains("sürüm 35'ten önce yok"), "{e}");
}

/// `bytes` with `modules.arch` saying `to` instead of `from` (CBOR: the text "arch", then 0x18 and the version), the
/// SHA-256 trailer made again: a file an older KentOS would have written, holding newer data.
fn saying_arch(bytes: &[u8], from: u8, to: u8) -> Vec<u8> {
    let mut older = bytes.to_vec();
    let at = older
        .windows(7)
        .position(|w| w == [0x64, b'a', b'r', b'c', b'h', 0x18, from])
        .expect("modules.arch");
    older[at + 6] = to;
    let body = older.len() - 32;
    let hash = <sha2::Sha256 as sha2::Digest>::digest(&older[..body]);
    older[body..].copy_from_slice(&hash);
    older
}

/// A plain wall style of its own (one 20 cm brick component).
fn plain_style() -> kentos_contracts::WallStyleDef {
    serde_json::from_value(serde_json::json!({
        "id": "tasiyici-tugla", "name": "Taşıyıcı tuğla",
        "components": [{"name": "Tuğla", "material": "Tuğla", "thickness": 0.2}]
    }))
    .expect("a style")
}

/// Architecture data 25 (docs/adr/8136): a shaft room's protected role and a wall's fire stop role are written with
/// `modules.arch = 25` and read back.
#[test]
fn a_protected_shaft_and_a_fire_stop_round_trip_under_arch_data_25() {
    let mut doc = sample();
    assert!(!kentos_kcad::uses_fire_stops(&doc));
    for e in doc.entities.iter_mut() {
        match e {
            kentos_contracts::Entity::Space(s) if s.space.arch.protected.is_none() => {
                s.space.name = Some("Tesisat şaftı".into());
                s.space.void = true;
                s.space.arch.protected = Some(kentos_contracts::DeclaredProtected {
                    protected: true,
                    basis: kentos_contracts::DeclaredBasis::Author,
                });
                break;
            }
            _ => {}
        }
    }
    let wall = doc
        .entities
        .iter_mut()
        .find_map(|e| match e {
            kentos_contracts::Entity::Wall(w) => Some(w),
            _ => None,
        })
        .expect("a wall");
    wall.wall.arch.fire_stop = Some(kentos_contracts::DeclaredFireStop {
        fire_stop: true,
        basis: kentos_contracts::DeclaredBasis::Document {
            name: "Yangın raporu".into(),
            date: None,
        },
    });
    assert!(kentos_kcad::uses_fire_stops(&doc));
    assert_eq!(
        kentos_kcad::arch_version(&doc),
        kentos_kcad::ARCH_WITH_FIRE_STOPS
    );
    let bytes = kentos_kcad::encode(&doc).expect("encodes");
    assert_eq!(
        kentos_kcad::decode(&bytes).expect("decodes"),
        doc,
        "the roles read back"
    );
}

/// Architecture data 23 (docs/adr/8134): a material's roof covering class is written with `modules.arch = 23` and read
/// back.
#[test]
fn a_material_s_roof_class_round_trips_under_arch_data_23() {
    let mut doc = sample();
    assert!(!kentos_kcad::uses_roof_classes(&doc));
    let arch = doc.settings.arch.get_or_insert_with(Default::default);
    let mut m: kentos_contracts::ArchMaterial =
        serde_json::from_value(serde_json::json!({"id": "kil-kiremit", "name": "Kil kiremit"}))
            .expect("a material");
    m.roof_class = Some(kentos_contracts::Declared::author("BROOF(t1)"));
    arch.materials.push(m);
    assert!(kentos_kcad::uses_roof_classes(&doc));
    assert_eq!(
        kentos_kcad::arch_version(&doc),
        kentos_kcad::ARCH_WITH_ROOF_CLASSES
    );
    let bytes = kentos_kcad::encode(&doc).expect("encodes");
    assert_eq!(
        kentos_kcad::decode(&bytes).expect("decodes"),
        doc,
        "the class reads back"
    );
}

/// Architecture data 15 (docs/adr/8094): the building's structural system is written with `modules.arch = 15` and read
/// back.
#[test]
fn the_building_s_structure_round_trips_under_arch_data_15() {
    let mut doc = sample();
    assert!(!kentos_kcad::uses_structure(&doc));
    doc.settings
        .arch
        .get_or_insert_with(Default::default)
        .structure = Some(kentos_contracts::BuildingStructure {
        system: kentos_contracts::StructuralSystem::CutStoneMasonry,
        traditional: true,
    });
    assert!(kentos_kcad::uses_structure(&doc));
    assert_eq!(
        kentos_kcad::arch_version(&doc),
        kentos_kcad::ARCH_WITH_STRUCTURE
    );
    let bytes = kentos_kcad::encode(&doc).expect("encodes");
    assert_eq!(
        kentos_kcad::decode(&bytes).expect("decodes"),
        doc,
        "the structure reads back"
    );
}

/// Architecture data 16 (docs/adr/8098): a plan note of the plan's building height is written with `modules.arch = 16`
/// and read back.
#[test]
fn the_plan_s_building_height_round_trips_under_arch_data_16() {
    let mut doc = sample();
    assert!(!kentos_kcad::uses_plan_limits(&doc));
    doc.settings
        .arch
        .get_or_insert_with(Default::default)
        .plan_notes
        .push(kentos_contracts::PlanNote {
            key: kentos_contracts::PlanNoteKey::BuildingHeight,
            value: 9.5,
            source: "UİP 12.3.2019, plan notu 3".into(),
        });
    assert!(kentos_kcad::uses_plan_limits(&doc));
    assert_eq!(
        kentos_kcad::arch_version(&doc),
        kentos_kcad::ARCH_WITH_PLAN_LIMITS
    );
    let bytes = kentos_kcad::encode(&doc).expect("encodes");
    assert_eq!(
        kentos_kcad::decode(&bytes).expect("decodes"),
        doc,
        "the note reads back"
    );
}

/// Architecture data 17 (docs/adr/8099): the delivery record is written with `modules.arch = 17` and read back.
#[test]
fn the_delivery_record_round_trips_under_arch_data_17() {
    let mut doc = sample();
    assert!(!kentos_kcad::uses_deliveries(&doc));
    doc.settings
        .arch
        .get_or_insert_with(Default::default)
        .deliveries
        .push(kentos_contracts::Delivery {
            id: "00000000-0000-4000-8000-000000000001".into(),
            number: 1,
            revision: "000".into(),
            package: kentos_contracts::DeliveryPackage {
                manifest: "0".repeat(64),
                files: 14,
                path: None,
                source: None,
            },
            to: "Sivas Belediyesi".into(),
            channel: kentos_contracts::DeliveryChannel::Epys,
            at: "2026-11-17".into(),
            by: None,
            receipt: Some("2026-0001".into()),
            response: None,
        });
    assert!(kentos_kcad::uses_deliveries(&doc));
    assert_eq!(
        kentos_kcad::arch_version(&doc),
        kentos_kcad::ARCH_WITH_DELIVERIES
    );
    let bytes = kentos_kcad::encode(&doc).expect("encodes");
    assert_eq!(
        kentos_kcad::decode(&bytes).expect("decodes"),
        doc,
        "the record reads back"
    );
}

/// Architecture data 18 (docs/adr/8101): an underlay model is written with `modules.arch = 18` and read back.
#[test]
fn an_underlay_model_round_trips_under_arch_data_18() {
    let mut doc = sample();
    assert!(!kentos_kcad::uses_underlays(&doc));
    doc.settings
        .arch
        .get_or_insert_with(Default::default)
        .underlays
        .push(kentos_contracts::ModelUnderlay {
            id: "00000000-0000-4000-8000-000000000001".into(),
            name: "Statik model".into(),
            file: "/proje/statik.ifc".into(),
            sha256: "0".repeat(64),
            bytes: 141_965,
            schema: "IFC4X3_ADD2".into(),
            discipline: Some("Statik".into()),
            revision: Some("R2".into()),
            placement: kentos_contracts::UnderlayPlacement::Origin,
            shown: false,
        });
    assert!(kentos_kcad::uses_underlays(&doc));
    assert_eq!(
        kentos_kcad::arch_version(&doc),
        kentos_kcad::ARCH_WITH_UNDERLAYS
    );
    let bytes = kentos_kcad::encode(&doc).expect("encodes");
    assert_eq!(
        kentos_kcad::decode(&bytes).expect("decodes"),
        doc,
        "the underlay reads back"
    );
}

/// Architecture data 19 (docs/adr/8106): a project's element type and a column following it, with an exception, are
/// written with `modules.arch = 19` and read back.
#[test]
fn an_element_type_and_its_column_round_trip_under_arch_data_19() {
    let mut doc = sample();
    assert!(!kentos_kcad::uses_element_types(&doc));
    let values = serde_json::json!({ "width": 0.3, "depth": 0.5, "fill": true });
    doc.settings
        .arch
        .get_or_insert_with(Default::default)
        .element_types
        .push(kentos_contracts::ElementType {
            id: "s1".into(),
            name: "S1".into(),
            kind: kentos_contracts::ElementTypeKind::Column,
            version: 2,
            values: values.as_object().cloned().unwrap_or_default(),
        });
    let column = doc
        .entities
        .iter_mut()
        .find(|e| matches!(e, kentos_contracts::Entity::Column(_)))
        .expect("the building has a column");
    column.arch_info_mut().expect("a column's").element_type = Some(kentos_contracts::TypeLink {
        id: "s1".into(),
        version: 2,
        overrides: vec!["width".into()],
    });
    assert!(kentos_kcad::uses_element_types(&doc));
    assert_eq!(
        kentos_kcad::arch_version(&doc),
        kentos_kcad::ARCH_WITH_ELEMENT_TYPES
    );
    let bytes = kentos_kcad::encode(&doc).expect("encodes");
    assert_eq!(
        kentos_kcad::decode(&bytes).expect("decodes"),
        doc,
        "the type and the link read back"
    );
}

/// Architecture data 20 (docs/adr/8124): a wall's and a slab's declared exterior role and a column's declared
/// load-bearing role are written with `modules.arch = 20` and read back.
#[test]
fn the_declared_readings_round_trip_under_arch_data_20() {
    use kentos_contracts::{DeclaredBasis, DeclaredBearing, DeclaredExternal, Entity};
    let mut doc = sample();
    assert!(!kentos_kcad::uses_declared_readings(&doc));
    // A wall's own load-bearing role is architecture data 13's, not 20's.
    let wall = doc.entities.iter_mut().find_map(|e| match e {
        Entity::Wall(w) => Some(w),
        _ => None,
    });
    wall.expect("a wall").wall.arch.load_bearing = Some(DeclaredBearing {
        bearing: true,
        basis: DeclaredBasis::Author,
    });
    assert!(!kentos_kcad::uses_declared_readings(&doc));
    let column = doc
        .entities
        .iter_mut()
        .find(|e| matches!(e, Entity::Column(_)))
        .expect("the building has a column");
    column.arch_info_mut().expect("a column's").load_bearing = Some(DeclaredBearing {
        bearing: false,
        basis: DeclaredBasis::Author,
    });
    assert!(kentos_kcad::uses_declared_readings(&doc));
    let slab = doc
        .entities
        .iter_mut()
        .find(|e| matches!(e, Entity::Slab(_)))
        .expect("the building has a slab");
    slab.arch_info_mut().expect("a slab's").external = Some(DeclaredExternal {
        external: true,
        basis: DeclaredBasis::Document {
            name: "Isı yalıtım projesi".into(),
            date: Some("2026-11-18".into()),
        },
    });
    assert_eq!(
        kentos_kcad::arch_version(&doc),
        kentos_kcad::ARCH_WITH_DECLARED_READINGS
    );
    let bytes = kentos_kcad::encode(&doc).expect("encodes");
    assert_eq!(
        kentos_kcad::decode(&bytes).expect("decodes"),
        doc,
        "the roles read back"
    );
}

/// Architecture data 21 (docs/adr/8125): a room's own emsal state, an emsal increase with its basis, is written with
/// `modules.arch = 21` and read back.
#[test]
fn a_room_s_own_emsal_state_round_trips_under_arch_data_21() {
    use kentos_contracts::{EmsalCount, Entity, SpaceEmsal};
    let mut doc = sample();
    assert!(!kentos_kcad::uses_room_emsal(&doc));
    let room = doc.entities.iter_mut().find_map(|e| match e {
        Entity::Space(s) => Some(s),
        _ => None,
    });
    room.expect("a room").space.emsal = Some(SpaceEmsal {
        count: EmsalCount::Bonus,
        basis: Some("Sivas İmar Yönetmeliği 22/k".into()),
    });
    assert!(kentos_kcad::uses_room_emsal(&doc));
    assert_eq!(
        kentos_kcad::arch_version(&doc),
        kentos_kcad::ARCH_WITH_ROOM_EMSAL
    );
    let bytes = kentos_kcad::encode(&doc).expect("encodes");
    assert_eq!(
        kentos_kcad::decode(&bytes).expect("decodes"),
        doc,
        "the state reads back"
    );
}
