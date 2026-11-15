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

/// Rooms of one storey that share ground are a pair, with what they share (docs/adr/8067, EK-9 Tablo 9.1/13): a metre
/// square drawn from the ground storey's living room's centre shares a square metre with it; the same square on the
/// storey above is not compared with it, and the drawing as made has no overlap.
#[test]
fn rooms_of_one_storey_that_share_ground_are_paired_with_what_they_share() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/audit-facts/v1/mimari.kcad");
    let mut snapshot: DocumentSnapshotV2 = kentos_kcad::decode(&std::fs::read(path).unwrap()).unwrap();
    let (facts, _) = measure(&Document::from_snapshot_v2(snapshot.clone()).unwrap()).normalized();
    assert!(facts.room_overlaps.is_empty(), "{:?}", facts.room_overlaps);
    let salon = facts
        .rooms
        .iter()
        .find(|r| r.name.as_deref() == Some("Salon") && r.level_code.as_deref() == Some("K00"))
        .expect("the ground storey's living room")
        .clone();
    let upper = facts.rooms.iter().find(|r| r.level_code.as_deref() == Some("K01")).and_then(|r| r.level.clone()).expect("K01");
    let at = snapshot.uids.iter().position(|u| kentos_domain::Uuid::from_bytes(u.0).to_string() == salon.uid).expect("its object");
    let [x, y] = salon.at;
    let square = kentos_commands::SpacePart {
        outer: kentos_commands::SpaceRing {
            pts: vec![
                kentos_commands::Vec2 { x, y },
                kentos_commands::Vec2 { x: x + 1.0, y },
                kentos_commands::Vec2 { x: x + 1.0, y: y + 1.0 },
                kentos_commands::Vec2 { x, y: y + 1.0 },
            ],
            bulges: None,
        },
        holes: Vec::new(),
    };
    let last = snapshot.entities.iter().map(|e| e.base().id).max().unwrap_or(0);
    for (n, level) in [(1u128, salon.level.clone()), (2u128, Some(upper))] {
        let mut e = snapshot.entities[at].clone();
        let Entity::Space(s) = &mut e else { panic!("a room") };
        s.base.id = last + n as u32;
        s.space.parts = vec![square.clone()];
        s.space.arch.level = level;
        s.space.name = Some(format!("Kare {n}"));
        snapshot.entities.push(e);
        snapshot.uids.push(kentos_commands::EntityId(kentos_domain::Uuid::from_u128(0xC0FFEE_0000 + n).into_bytes()));
    }
    let (facts, _) = measure(&Document::from_snapshot_v2(snapshot).unwrap()).normalized();
    let ground = kentos_domain::Uuid::from_u128(0xC0FFEE_0001).to_string();
    let pairs: Vec<_> = facts.room_overlaps.iter().map(|o| (o.a.as_str(), o.b.as_str(), o.area)).collect();
    let (a, b) = if ground < salon.uid { (ground.as_str(), salon.uid.as_str()) } else { (salon.uid.as_str(), ground.as_str()) };
    assert!(pairs.contains(&(a, b, 1.0)), "{pairs:?}");
    let above = kentos_domain::Uuid::from_u128(0xC0FFEE_0002).to_string();
    assert!(!pairs.iter().any(|p| (p.0 == ground || p.1 == ground) && (p.0 == above || p.1 == above)), "{pairs:?}");
    let o = facts.room_overlaps.iter().find(|o| o.area == 1.0).expect("the pair");
    assert_eq!(o.at, [x + 0.5, y + 0.5]);
}
