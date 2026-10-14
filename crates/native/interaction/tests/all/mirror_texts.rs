//! Aynala's Yazılar (docs/adr/0292, AutoCAD's MIRRTEXT): a text mirrored across a level axis stays readable (its up
//! turned back: upright, top for baseline) or, aynalı, turns with the drawing (half a turn, right for left); the
//! choice stays for the session and the grip mode's Aynala shares it. Values worked out by hand.

use crate::common;

use common::{Bench, rel};
use kentos_contracts::Entity;
use kentos_domain::Slot;

/// A text at (2, 3) on the traces' empty drawing, selected.
fn bench() -> Bench {
    let mut b = Bench::new("line");
    let text: Entity = serde_json::from_value(serde_json::json!({
        "kind": "text", "id": 0, "layerId": "cizim", "attrs": {},
        "p": {"x": common::E + 2.0, "y": common::N + 3.0}, "text": "Kapı 12", "height": 0.5, "rotation": 0.0
    }))
    .expect("a text");
    let slot = b.doc.add(text).expect("a slot");
    b.selection.set([slot]);
    b
}

fn text(b: &Bench, slot: Slot) -> ([f64; 2], f64, Option<String>, String) {
    let Some(Entity::Text(t)) = b.doc.get(slot) else {
        panic!("a text");
    };
    let align = serde_json::to_value(t.align).ok().and_then(|v| v.as_str().map(str::to_owned));
    (rel(t.p), t.rotation, align, t.text.clone())
}

#[test]
fn a_level_mirror_keeps_a_text_readable_or_turns_it_with_aynali() {
    let mut b = bench();
    let slot = b.selection.ids()[0];
    b.start("mirror");
    assert!(b.session.prompt().text().ends_with("[Kaynağı sil (S): hayır / Yazılar (Y): okunur]"));
    assert!(b.type_text("S"), "in place");
    b.click(0.0, 0.0);
    b.click(10.0, 0.0);
    let (p, rotation, align, words) = text(&b, slot);
    assert_eq!(p, [2.0, -3.0]);
    assert_eq!(rotation, 0.0, "upright");
    assert_eq!(align.as_deref(), Some("topLeft"), "hanging from its point: it fills its mirrored place");
    assert_eq!(words, "Kapı 12", "the words never change");

    // Aynalı: half a turn more, right for left; the session keeps it.
    b.doc.undo();
    b.start("mirror");
    assert!(b.type_text("S"));
    assert!(b.type_text("Y"));
    assert!(b.session.prompt().text().ends_with("Yazılar (Y): aynalı]"));
    b.click(0.0, 0.0);
    b.click(10.0, 0.0);
    let (p, rotation, align, _) = text(&b, slot);
    assert_eq!(p, [2.0, -3.0]);
    assert_eq!(rotation, 180.0);
    assert_eq!(align.as_deref(), Some("baselineRight"));
    assert!(b.memory.mirror_text_mirrored, "the session keeps it");
    b.start("mirror");
    assert!(b.session.prompt().text().ends_with("Yazılar (Y): aynalı]"));
}

#[test]
fn a_vertical_mirror_keeps_the_text_upright_with_its_side_swapped() {
    let mut b = bench();
    let slot = b.selection.ids()[0];
    b.start("mirror");
    assert!(b.type_text("S"));
    b.click(0.0, 0.0);
    b.click(0.0, 10.0);
    let (p, rotation, align, _) = text(&b, slot);
    assert_eq!(p, [-2.0, 3.0]);
    assert!(rotation.abs() < 1e-12 || (rotation - 360.0).abs() < 1e-12, "{rotation}");
    assert_eq!(align.as_deref(), Some("baselineRight"), "ends where it began");
}
