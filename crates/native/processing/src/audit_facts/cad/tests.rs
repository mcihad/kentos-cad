use std::path::PathBuf;

use kentos_commands::DocumentSnapshotV2;

use super::*;

/// The architecture drawing of the audit facts (scripts/fixtures/audit_facts_mimari.py made it with KentOS's commands):
/// two storeys, rooms numbered Z01 … (not EK-3's), everything on Çizim.
fn plan() -> Document {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/audit-facts/v1/mimari.kcad");
    let snapshot: DocumentSnapshotV2 = kentos_kcad::decode(&std::fs::read(path).unwrap()).unwrap();
    Document::from_snapshot_v2(snapshot).unwrap()
}

/// Every layer with its objects and line type, the rooms with their storey's code as the sheets write it and their
/// centres, in the section's order.
#[test]
fn the_layers_say_their_objects_and_line_type_and_the_rooms_their_storey_code() {
    let (facts, cut) = measure(&plan()).normalized();
    assert!(!cut);
    let names: Vec<_> = facts.layers.iter().map(|l| (l.name.as_str(), l.objects, l.line_type.as_str())).collect();
    assert_eq!(names[0], ("Çizim", 48, "continuous"));
    assert!(names.contains(&("Eksen", 0, "dashdot")), "{names:?}");
    let cizim = &facts.layers[0];
    assert_eq!(cizim.first.len(), 48);
    assert!(cizim.first.windows(2).all(|w| w[0] < w[1]), "by persistent id");
    assert!(facts.rooms.windows(2).all(|w| w[0].uid < w[1].uid));
    let codes: std::collections::BTreeSet<_> = facts.rooms.iter().filter_map(|r| r.level_code.as_deref()).collect();
    assert_eq!(codes.into_iter().collect::<Vec<_>>(), ["K00", "K01"]);
    assert!(facts.rooms.iter().all(|r| r.level_code.is_some()));
    let salon = facts
        .rooms
        .iter()
        .find(|r| r.name.as_deref() == Some("Salon") && r.level_code.as_deref() == Some("K00"))
        .expect("the ground storey's living room");
    assert_eq!((salon.number.as_deref(), salon.at), (Some("Z02"), [15.0, 2.025]));
    assert!(facts.blocks.is_empty() && facts.dimension_fonts.is_empty() && facts.grids.is_empty());
}
