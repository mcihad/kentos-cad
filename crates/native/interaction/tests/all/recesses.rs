//! Niş and Baca (docs/adr/0719): a niche takes its face from the side of the wall the pointer is on and its depth from
//! its type (Derinlik over it); a flue is centred until Yüzden says; one too deep for its wall is refused and said
//! beside the pointer; the tag says whether plan cuts it. Each is written by `cad.openings.create`.

use crate::common::Bench;
use kentos_commands::{Entity, OpeningFields, OpeningForm, OpeningSide};

fn set(b: &mut Bench, k: &str, v: &str) -> bool {
    b.run(|s, cx| s.configure(k, v, cx))
}

/// A 30 m wall along x, 0.20 m thick, `tool` started.
fn wall(tool: &str) -> Bench {
    let mut b = Bench::new("wall");
    b.click(0.0, 0.0);
    b.click(30.0, 0.0);
    b.confirm();
    b.confirm();
    b.start(tool);
    b
}

fn newest(b: &Bench) -> OpeningFields {
    let Entity::Opening(o) = b.newest() else {
        panic!("an opening")
    };
    o.opening.clone()
}

fn row(b: &Bench, key: &str) -> Option<String> {
    b.session
        .settings(&Default::default())
        .into_iter()
        .find(|r| r.key == key)
        .map(|r| r.value)
}

fn tag(b: &Bench) -> Vec<String> {
    b.session
        .preview(&Default::default())
        .and_then(|p| p.tag)
        .map(|t| t.lines)
        .unwrap_or_default()
}

#[test]
fn a_niche_is_cut_from_the_face_the_pointer_is_on() {
    let mut b = wall("niche");
    assert!(row(&b, "D").is_some_and(|v| v.contains("türün")), "its type's depth");
    b.click(5., -0.3);
    let o = newest(&b);
    assert_eq!(o.form, OpeningForm::Niche);
    assert_eq!(o.side, OpeningSide::Right, "the pointer's side");
    assert_eq!(o.depth, Some(0.1));
    assert_eq!(o.number, None, "a niche is unnumbered at first");
    assert_eq!(b.last_text(), Some("Niş eklendi."));
    assert_eq!(
        b.doc.journal().last().unwrap().command.as_deref(),
        Some("cad.openings.create")
    );
    b.click(10., 0.3);
    assert_eq!(newest(&b).side, OpeningSide::Left);
    assert!(set(&mut b, "Y", "Sağ"));
    b.click(15., 0.3);
    assert_eq!(newest(&b).side, OpeningSide::Right, "Yüz chosen beats the pointer");
}

#[test]
fn a_flue_is_centred_until_yuzden_says_and_takes_the_depth_set() {
    let mut b = wall("flue");
    assert_eq!(row(&b, "Z").as_deref(), Some("Ortada"));
    assert!(set(&mut b, "D", "0.1"));
    b.click(5., 0.3);
    let o = newest(&b);
    assert_eq!((o.form, o.depth, o.inset), (OpeningForm::Flue, Some(0.1), None));
    assert!(set(&mut b, "Z", "0.03"));
    b.click(10., 0.3);
    assert_eq!(newest(&b).inset, Some(0.03));
    assert!(set(&mut b, "Z", "Ortada"));
    assert_eq!(row(&b, "Z").as_deref(), Some("Ortada"));
    b.click(15., 0.3);
    assert_eq!(newest(&b).inset, None);
}

#[test]
fn one_too_deep_for_its_wall_is_refused_and_said_beside_the_pointer() {
    let mut b = wall("niche");
    assert!(set(&mut b, "D", "0.25"));
    b.move_to(5., 0.3);
    let lines = tag(&b).join(" ");
    assert!(lines.contains("Nişin derinliği"), "{lines}");
    let n = b.doc.entities().count();
    b.click(5., 0.3);
    assert_eq!(b.doc.entities().count(), n, "nothing written");
    assert!(!set(&mut b, "D", "0"), "a depth above 0");
}

#[test]
fn the_tag_says_whether_plan_cuts_it() {
    let mut b = wall("niche");
    b.move_to(5., 0.3);
    let lines = tag(&b).join(" ");
    assert!(lines.contains("Planda kesilir"), "Niş 60 crosses the cut: {lines}");
    assert!(lines.contains("derinlik"), "{lines}");
    assert!(set(&mut b, "L", "1.2"));
    assert!(set(&mut b, "Ü", "1.8"));
    b.move_to(5.1, 0.3);
    let lines = tag(&b).join(" ");
    assert!(lines.contains("Planda kesik çizgi"), "above the cut: {lines}");
    b.click(5., 0.3);
    let o = newest(&b);
    assert_eq!((o.sill, o.head), (Some(1.2), Some(1.8)));
    assert!(!o.cut_in_plan(kentos_commands::PLAN_CUT_HEIGHT));
}
