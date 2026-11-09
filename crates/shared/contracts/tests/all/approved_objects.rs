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
            approved_parts: Vec::new(),
            approved_values: Vec::new(),
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

/// docs/adr/3280: the parts kept beside the list, one to one with it, four digests of 16 lower-case hex digits each;
/// else refused at `items[i].approvedParts`. An object's parts are its snapshot form's fields split four ways.
#[test]
fn the_parts_an_approval_kept_are_one_to_one_with_its_objects() {
    use kentos_contracts::{OBJECT_PARTS, object_parts};
    let pairs = vec![(uid(3), "c".repeat(64)), (uid(4), "d".repeat(64))];
    let stamp = stamp_of(&pairs);
    let four = || ["0123456789abcdef".to_owned(), "1".repeat(16), "2".repeat(16), "3".repeat(16)];
    let mut w = approved(pairs.clone(), Some(stamp.clone()));
    w.items[0].approved_parts = vec![four(), four()];
    assert_eq!(w.problem(), None);
    assert!(w.has_approved_parts());
    let at = |w: &MapWork| w.problem().map(|(at, why)| (at, why));
    for parts in [vec![four()], vec![four(), four(), four()]] {
        let mut w = approved(pairs.clone(), Some(stamp.clone()));
        w.items[0].approved_parts = parts;
        let (path, why) = at(&w).expect("refused");
        assert_eq!(path, "items[0].approvedParts");
        assert!(why.contains("bire bir"), "{why}");
    }
    for bad in ["0123456789ABCDEF", "0123456789abcde", "0123456789abcdefg"] {
        let mut w = approved(pairs.clone(), Some(stamp.clone()));
        let mut one = four();
        one[2] = bad.to_owned();
        w.items[0].approved_parts = vec![four(), one];
        assert!(at(&w).is_some_and(|(p, why)| p == "items[0].approvedParts" && why.contains("16")), "{bad}");
    }
    // Parts without the list.
    let mut w = approved(Vec::new(), Some(stamp));
    w.items[0].approved_parts = vec![four()];
    assert!(at(&w).is_some_and(|(p, _)| p == "items[0].approvedParts"));
    // The split: a colour is the look's, a name the attributes', the layer its own; the corners the geometry's.
    assert_eq!(OBJECT_PARTS, ["geometry", "attributes", "layer", "look"]);
    let line = serde_json::json!({"kind": "line", "id": 7, "layerId": "yapi", "attrs": {"Ad": "D1"},
                                  "a": {"x": 1.0, "y": 2.0}, "b": {"x": 3.0, "y": 4.0}});
    let base = object_parts(&line);
    let changed = |f: &dyn Fn(&mut serde_json::Value)| {
        let mut v = line.clone();
        f(&mut v);
        let now = object_parts(&v);
        (0..4).filter(|k| now[*k] != base[*k]).map(|k| OBJECT_PARTS[k]).collect::<Vec<_>>()
    };
    assert_eq!(changed(&|v| v["id"] = 9.into()), Vec::<&str>::new(), "the slot is not a part");
    assert_eq!(changed(&|v| v["b"]["x"] = 3.5.into()), ["geometry"]);
    assert_eq!(changed(&|v| v["attrs"]["Ad"] = "D2".into()), ["attributes"]);
    assert_eq!(changed(&|v| v["layerId"] = "yol".into()), ["layer"]);
    assert_eq!(changed(&|v| v["color"] = "#ff0000".into()), ["look"]);
    // The geometry's digest by hand: SHA-256 of {"a":…,"b":…,"kind":"line"}, compact with sorted keys, 16 digits.
    let text = r#"{"a":{"x":1.0,"y":2.0},"b":{"x":3.0,"y":4.0},"kind":"line"}"#;
    assert_eq!(base[0], sha256_hex(text.as_bytes())[..16]);
}
