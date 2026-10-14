//! Koordinat oku says what a snapped click snapped to (docs/adr/0272 §3): the kind, the object's kind and the
//! start of its persistent id, and its place on the object.

use crate::common;

use common::Bench;

#[test]
fn a_snapped_click_says_its_object_and_place() {
    let mut b = Bench::new("crsQuery");
    b.draft.snap = true;
    let slot = b.add_line("cizim", [0.0, 0.0], [10.0, 0.0]);
    let uid = b.doc.uid(slot).expect("a uid").to_string();
    let before = b.log.len();
    b.click(10.05, 0.05);
    let said: Vec<&str> = b.said(before).into_iter().map(|(_, t)| t).collect();
    let want = format!("Kenet: Uç nokta · Çizgi {} · bitiş", &uid[..8]);
    assert!(said.contains(&want.as_str()), "{said:?}");
    // Off any snap: no such line.
    let before = b.log.len();
    b.click(30.0, 30.0);
    assert!(
        !b.said(before).iter().any(|(_, t)| t.starts_with("Kenet:")),
        "no snap, no line"
    );
}
