//! Açıklığın numarası yerleştirirken (docs/adr/0714): Kapı, Pencere and Boşluk number each opening they place after
//! the last numbered one of its form (none yet: K1, P1, B1), a number typed is the next one's and they go on from it,
//! a number in use is passed over, Yok leaves it unnumbered; the number is in the command's input.

use crate::common::Bench;
use kentos_commands::Entity;

fn set(b: &mut Bench, k: &str, v: &str) -> bool {
    b.run(|s, cx| s.configure(k, v, cx))
}

/// A 30 m wall along x, `tool` started.
fn wall(tool: &str) -> Bench {
    let mut b = Bench::new("wall");
    b.click(0.0, 0.0);
    b.click(30.0, 0.0);
    b.confirm();
    b.confirm();
    b.start(tool);
    b
}

fn number(b: &Bench) -> Option<String> {
    let Entity::Opening(o) = b.newest() else {
        panic!("an opening")
    };
    o.opening.number.clone()
}

fn shown(b: &Bench) -> String {
    b.session
        .settings(&Default::default())
        .into_iter()
        .find(|r| r.key == "N")
        .expect("Numara")
        .value
}

#[test]
fn doors_count_on_from_k1_and_windows_from_p1() {
    let mut b = wall("door");
    b.move_to(2., 0.3);
    assert_eq!(shown(&b), "K1 · otomatik", "said before it is placed");
    b.click(2., 0.3);
    assert_eq!(number(&b).as_deref(), Some("K1"));
    b.click(5., 0.3);
    assert_eq!(number(&b).as_deref(), Some("K2"));
    assert_eq!(b.last_text(), Some("Kapı K2 eklendi."));
    b.session.exit();
    b.start("window");
    b.click(8., 0.3);
    assert_eq!(number(&b).as_deref(), Some("P1"), "a window's own series");
    assert_eq!(
        b.doc.journal().last().unwrap().command.as_deref(),
        Some("cad.openings.create")
    );
}

#[test]
fn a_number_typed_starts_there_keeps_its_width_and_passes_over_one_in_use() {
    let mut b = wall("door");
    assert!(set(&mut b, "N", "K-07"));
    assert_eq!(shown(&b), "K-07");
    b.click(2., 0.3);
    assert_eq!(number(&b).as_deref(), Some("K-07"));
    b.click(5., 0.3);
    assert_eq!(number(&b).as_deref(), Some("K-08"), "on from it, as wide");
    // K-09 is taken: typed again, the next free one.
    assert!(set(&mut b, "N", "K-09"));
    b.click(8., 0.3);
    assert!(set(&mut b, "N", "K-09"));
    b.click(11., 0.3);
    assert_eq!(number(&b).as_deref(), Some("K-10"));
    assert!(set(&mut b, "N", "K9"));
    b.click(14., 0.3);
    b.click(17., 0.3);
    assert_eq!(number(&b).as_deref(), Some("K10"), "K9 → K10");
}

#[test]
fn yok_leaves_it_unnumbered_and_otomatik_comes_back() {
    let mut b = wall("door");
    assert!(set(&mut b, "N", "yok"));
    assert_eq!(shown(&b), "Yok");
    b.click(2., 0.3);
    assert_eq!(number(&b), None);
    assert!(set(&mut b, "N", ""));
    b.click(5., 0.3);
    assert_eq!(number(&b).as_deref(), Some("K1"), "the unnumbered one does not count");
    assert!(!set(&mut b, "N", &"K".repeat(129)), "too long");
}
