//! The values an approval kept (docs/adr/3680): one text per approved object, in its list's order, each the very text
//! the object's digest is the SHA-256 of, at most `MAX_APPROVED_VALUES_BYTES` together. Every other list is refused at
//! `items[i].approvedValues`.

use kentos_contracts::encoding::sha256_hex;
use kentos_contracts::{EntityId, MAX_APPROVED_VALUES_BYTES, MapWork, WorkItem, WorkKind, WorkStatus};

fn uid(n: u32) -> EntityId {
    let mut b = [0u8; 16];
    b[12..].copy_from_slice(&n.to_be_bytes());
    EntityId(b)
}

/// A design approved with these objects' values: each object's digest the SHA-256 of its value.
fn approved(values: &[&str], kept: Vec<String>) -> MapWork {
    let pairs: Vec<(EntityId, String)> =
        values.iter().enumerate().map(|(k, v)| (uid(k as u32 + 1), sha256_hex(v.as_bytes()))).collect();
    let stamp = sha256_hex(&serde_json::to_vec(&pairs).expect("a list"));
    MapWork {
        items: vec![WorkItem {
            id: "is-1".into(),
            kind: WorkKind::Design,
            name: "Tasarım yüzeyi".into(),
            status: WorkStatus::Approved,
            layers: vec!["tasarim".into()],
            file: None,
            approved: Some(stamp),
            approved_objects: pairs,
            approved_parts: Vec::new(),
            approved_values: kept,
        }],
        derivations: Vec::new(),
    }
}

fn refused(w: &MapWork) -> Option<String> {
    w.problem().map(|(at, why)| {
        assert_eq!(at, "items[0].approvedValues", "{why}");
        why
    })
}

#[test]
fn the_values_an_approval_kept_are_its_objects_digests() {
    let values = [
        r#"{"attrs":{"Ad":"P1"},"kind":"point","layerId":"tasarim","p":{"x":1.0,"y":2.0}}"#,
        r#"{"attrs":{},"kind":"point","layerId":"tasarim","p":{"x":3.0,"y":4.0}}"#,
    ];
    let kept: Vec<String> = values.iter().map(|v| (*v).to_owned()).collect();
    let w = approved(&values, kept.clone());
    assert_eq!(w.problem(), None);
    assert!(w.has_approved_values());
    // None kept: nothing written, nothing judged.
    let w = approved(&values, Vec::new());
    assert!(!w.has_approved_values() && w.problem().is_none());
    assert!(serde_json::to_value(&w.items[0]).expect("writes").get("approvedValues").is_none());
    // Not one to one; a value that is not its object's digest's text.
    assert!(refused(&approved(&values, kept[..1].to_vec())).is_some_and(|w| w.contains("bire bir")));
    let mut other = kept.clone();
    other[1] = other[1].replace("3.0", "3.5");
    assert!(refused(&approved(&values, other)).is_some_and(|w| w.contains("SHA-256")));
}

#[test]
fn values_past_the_limit_are_refused() {
    let big = format!(r#"{{"attrs":{{"Not":"{}"}},"kind":"point","layerId":"tasarim","p":{{"x":0.0,"y":0.0}}}}"#, "x".repeat(MAX_APPROVED_VALUES_BYTES));
    let w = approved(&[big.as_str()], vec![big.clone()]);
    assert!(refused(&w).is_some_and(|w| w.contains("bayt")));
}
