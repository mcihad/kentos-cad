//! Architecture data 1 (docs/adr/8068): the fire resistance classes, exit roles, strengths and the building's fire
//! protection class a drawing declares are written with `modules.arch = 1` and read back as they were; a drawing
//! without them stays as an older KentOS reads it.

use kentos_contracts::{BuildingPerformance, Declared, DeclaredBasis, DeclaredExit, Entity};

fn sample() -> kentos_contracts::DocumentSnapshotV2 {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../fixtures/arch/v1/section-building.kcad"
    );
    kentos_kcad::decode(&std::fs::read(path).expect("the building")).expect("decodes")
}

#[test]
fn declarations_round_trip_under_arch_data_1() {
    let plain = sample();
    assert!(!kentos_kcad::uses_performance(&plain));
    let again =
        kentos_kcad::decode(&kentos_kcad::encode(&plain).expect("encodes")).expect("decodes");
    assert_eq!(again, plain, "a drawing without declarations is as it was");

    let mut doc = plain.clone();
    let document = DeclaredBasis::Document {
        name: "Test raporu 2026/41".into(),
        date: Some("2026-11-16".into()),
    };
    for e in &mut doc.entities {
        match e {
            Entity::Wall(w) => w.wall.arch.fire_rating = Some(Declared::author("REI 120")),
            Entity::Opening(o) => {
                o.opening.fire_rating = Some(Declared {
                    value: "EI 60".into(),
                    basis: document.clone(),
                });
                o.opening.exit = Some(DeclaredExit {
                    exit: true,
                    basis: DeclaredBasis::Author,
                });
            }
            Entity::Stair(s) => {
                s.stair.arch.exit = Some(DeclaredExit {
                    exit: false,
                    basis: DeclaredBasis::Author,
                })
            }
            _ => {}
        }
    }
    let arch = doc.settings.arch.get_or_insert_with(Default::default);
    arch.performance = Some(BuildingPerformance {
        fire_protection_class: Some(Declared::author("Konut")),
    });
    assert!(kentos_kcad::uses_performance(&doc));
    let bytes = kentos_kcad::encode(&doc).expect("encodes");
    assert_eq!(
        kentos_kcad::decode(&bytes).expect("decodes"),
        doc,
        "every declaration read back"
    );
}
