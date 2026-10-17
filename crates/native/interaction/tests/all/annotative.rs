//! Ölçekteki yeri through the session (docs/adr/0323 §4): a click on an annotative text, then where it goes at its
//! other scale, written by `cad.annotations.scales`'s `set`; Ö chooses the scale; a text with no other scale is not
//! taken. Points are east and north differences from (E, N).

use crate::common;

use common::{Bench, E, N, rel};
use kentos_contracts::{Entity, annotative_of};

const EMPTY: &str = include_str!("../../../../../fixtures/interaction/v1/empty.kcad");

fn bench_with(scales: serde_json::Value) -> (Bench, kentos_domain::Slot) {
    let mut b = Bench::on(EMPTY);
    b.draft.snap = false;
    let text: Entity = serde_json::from_value(serde_json::json!({
        "kind": "text", "id": 0, "layerId": "cizim", "attrs": {},
        "p": { "x": E + 10.0, "y": N + 10.0 }, "text": "SALON", "height": 2.5, "rotation": 0.0,
        "annotative": { "at": 1000.0, "scales": scales },
    }))
    .expect("a text");
    let slot = b.doc.add(text).expect("a slot");
    (b, slot)
}

#[test]
fn a_click_sets_the_texts_place_at_its_other_scale() {
    let (mut b, slot) = bench_with(serde_json::json!([{ "scale": 500.0 }, { "scale": 2000.0 }]));
    b.start("annotativePlace");
    assert_eq!(
        b.session.prompt().text(),
        "Ölçekteki yeri: ölçekli yazıya, ölçüye ya da kılavuza tıklayın"
    );
    b.click(11.0, 10.5);
    assert!(
        b.session.prompt().text().contains("Ölçek (Ö)"),
        "{}",
        b.session.prompt().text()
    );
    b.move_to(14.0, 12.0);
    let p = b.session.preview(&Default::default()).expect("a preview");
    assert_eq!(
        p.tag.expect("a tag").lines,
        vec!["SALON".to_owned(), "1:500".to_owned()]
    );
    // Ö: the next scale, 1:2000.
    assert!(b.type_text("Ö"));
    b.click(14.0, 12.0);
    let Some(e) = b.doc.get(slot) else {
        panic!("the text");
    };
    let a = annotative_of(e).expect("annotative");
    assert_eq!(a.entry(500.0).and_then(|s| s.point), None);
    assert_eq!(
        a.entry(2000.0).and_then(|s| s.point).map(rel),
        Some([14.0, 12.0])
    );
    let Entity::Text(t) = e else {
        panic!("a text");
    };
    assert_eq!(rel(t.p), [10.0, 10.0], "its own place stays");
    assert_eq!(b.last_text(), Some("1:2000 yeri yazıldı"));
}

#[test]
fn a_text_with_no_other_scale_is_not_taken() {
    let (mut b, _) = bench_with(serde_json::json!([]));
    b.start("annotativePlace");
    b.click(11.0, 10.5);
    assert!(b.session.prompt().text().ends_with("tıklayın"));
}
