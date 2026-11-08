//! The objects an approval kept (docs/adr/2880 §2): on a map work item without a file, approved and stamped, each a
//! non-nil persistent id with a lower-hex SHA-256, sorted by id and written once, at most `MAX_APPROVED_OBJECTS`, the
//! list whose compact JSON's SHA-256 is the stamp. Every other list is refused at `items[i].approvedObjects`.

use kentos_contracts::encoding::sha256_hex;
use kentos_contracts::{EntityId, MAX_APPROVED_OBJECTS, MapWork, WorkFile, WorkItem, WorkKind, WorkStatus};

fn uid(n: u32) -> EntityId {
    let mut b = [0u8; 16];
    b[12..].copy_from_slice(&n.to_be_bytes());
    EntityId(b)
}

fn stamp_of(pairs: &[(EntityId, String)]) -> String {
    sha256_hex(&serde_json::to_vec(pairs).expect("a list"))
}

/// A design approved with `pairs`, stamped `stamp`.
fn approved(pairs: Vec<(EntityId, String)>, stamp: Option<String>) -> MapWork {
    MapWork {
        items: vec![WorkItem {
            id: "is-1".into(),
            kind: WorkKind::Design,
            name: "Tasarım yüzeyi".into(),
            status: WorkStatus::Approved,
            layers: vec!["tasarim".into()],
            file: None,
            approved: stamp,
            approved_objects: pairs,
        }],
        derivations: Vec::new(),
    }
}

fn refused(w: &MapWork) -> Option<String> {
    w.problem().map(|(at, why)| {
        assert_eq!(at, "items[0].approvedObjects", "{why}");
        why
    })
}

#[test]
fn the_objects_an_approval_kept_are_its_stamp() {
    let pairs = vec![(uid(3), "c".repeat(64)), (uid(4), "d".repeat(64))];
    let stamp = stamp_of(&pairs);
    let w = approved(pairs.clone(), Some(stamp.clone()));
    assert_eq!(w.problem(), None);
    assert!(w.has_approved_objects());
    // The same list as JSON: pairs of the id's text and the digest.
    let json = serde_json::to_value(&w.items[0]).expect("writes");
    assert_eq!(json["approvedObjects"][0][0], "00000000-0000-0000-0000-000000000003");
    // None kept: nothing written, nothing judged.
    let w = approved(Vec::new(), Some(stamp.clone()));
    assert!(!w.has_approved_objects() && w.problem().is_none());
    assert!(serde_json::to_value(&w.items[0]).expect("writes").get("approvedObjects").is_none());
    // Without the stamp; on an item with a file.
    assert!(refused(&approved(pairs.clone(), None)).is_some_and(|w| w.contains("onay damgası")));
    let mut w = approved(pairs.clone(), Some(stamp.clone()));
    w.items[0].file = Some(WorkFile { path: "/veri/tasarim.dxf".into(), revision: "e".repeat(64), relative: None });
    assert!(refused(&w).is_some_and(|w| w.contains("dosyası olan")));
    // Out of order, written twice, a nil id, a digest that is not one.
    let reversed: Vec<_> = pairs.iter().rev().cloned().collect();
    let twice = vec![pairs[0].clone(), pairs[0].clone()];
    for list in [reversed, twice] {
        let s = stamp_of(&list);
        assert!(refused(&approved(list, Some(s))).is_some_and(|w| w.contains("sıralı")));
    }
    for list in [vec![(EntityId([0; 16]), "c".repeat(64))], vec![(uid(3), "C".repeat(64))], vec![(uid(3), "c".repeat(63))]] {
        let s = stamp_of(&list);
        assert!(refused(&approved(list, Some(s))).is_some_and(|w| w.contains("boş olmayan")));
    }
    // A list that is not the stamp's.
    let one = vec![pairs[0].clone()];
    assert!(refused(&approved(one, Some(stamp))).is_some_and(|w| w.contains("damgasını vermiyor")));
    // One more than the most.
    let many: Vec<_> = (1..=MAX_APPROVED_OBJECTS as u32 + 1).map(|k| (uid(k), "c".repeat(64))).collect();
    let s = stamp_of(&many);
    assert!(refused(&approved(many, Some(s))).is_some_and(|w| w.contains("en çok")));
}
