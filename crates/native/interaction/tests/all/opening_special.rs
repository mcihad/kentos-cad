//! Kapı's operation and its special doors' settings (docs/adr/0777 §5): Açılış chooses a revolving, pocket, folding
//! or sliding door over the type's swinging one, Kanat its leaves in the operation's range, and the drum, the pocket's
//! lip, both sides, the overlap and the track are the command's input; a pocket that does not fit is said before the
//! click and not written.

use crate::common::Bench;
use kentos_commands::{Entity, OpeningOperation};

fn set(b: &mut Bench, k: &str, v: &str) -> bool {
    b.run(|s, cx| s.configure(k, v, cx))
}

fn row(b: &Bench, key: &str) -> Option<String> {
    b.session
        .settings(&Default::default())
        .into_iter()
        .find(|r| r.key == key)
        .map(|r| r.value)
}

/// A 12 m wall along x, Kapı started.
fn wall() -> Bench {
    let mut b = Bench::new("wall");
    b.click(0.0, 0.0);
    b.click(12.0, 0.0);
    b.confirm();
    b.confirm();
    b.start("door");
    b
}

fn newest(b: &Bench) -> kentos_commands::OpeningFields {
    let Entity::Opening(o) = b.newest() else {
        panic!("an opening")
    };
    o.opening.clone()
}

#[test]
fn a_revolving_door_takes_its_wings_and_drum() {
    let mut b = wall();
    assert_eq!(row(&b, "J"), None, "no drum for a swinging door");
    assert!(set(&mut b, "Ç", "revolving"));
    assert_eq!(
        row(&b, "K").as_deref(),
        Some("3"),
        "a revolving door's least wings"
    );
    assert!(set(&mut b, "E", "2.4"));
    assert!(set(&mut b, "K", "4"));
    assert!(set(&mut b, "J", "1.8"));
    b.click(3., 0.3);
    let o = newest(&b);
    assert_eq!(o.operation, OpeningOperation::Revolving);
    assert_eq!(o.leaves, 4);
    assert_eq!(o.build.as_ref().and_then(|x| x.drum), Some(1.8));
    assert_eq!(
        b.doc.journal().last().unwrap().command.as_deref(),
        Some("cad.openings.create")
    );
}

#[test]
fn a_pocket_door_says_when_its_pocket_runs_off_the_wall() {
    let mut b = wall();
    assert!(set(&mut b, "Ç", "pocket"));
    assert!(set(&mut b, "P", "3cm"));
    let before = b.doc.entities().count();
    b.move_to(0.6, 0.3);
    b.click(0.6, 0.3);
    assert_eq!(
        b.doc.entities().count(),
        before,
        "its pocket off the wall's start: nothing written"
    );
    b.click(5., 0.3);
    let o = newest(&b);
    assert_eq!(o.operation, OpeningOperation::Pocket);
    assert_eq!(o.build.as_ref().and_then(|x| x.lip), Some(0.03));
}

#[test]
fn a_folding_door_folds_to_both_jambs_and_a_sliding_one_runs_on_its_track() {
    let mut b = wall();
    assert!(set(&mut b, "Ç", "folding"));
    assert!(set(&mut b, "E", "1.6"));
    assert!(set(&mut b, "K", "4"));
    assert!(set(&mut b, "İ", "on"));
    b.click(3., 0.3);
    let o = newest(&b);
    assert_eq!((o.operation, o.leaves), (OpeningOperation::Folding, 4));
    assert_eq!(o.build.as_ref().and_then(|x| x.both_sides), Some(true));
    assert!(set(&mut b, "Ç", "slide"));
    assert_eq!(
        row(&b, "K").as_deref(),
        Some("1"),
        "four panels are no sliding door's: back to its least"
    );
    assert!(set(&mut b, "K", "2"));
    assert!(set(&mut b, "S", "10cm"));
    assert!(set(&mut b, "H", "on"));
    b.click(7., 0.3);
    let o = newest(&b);
    assert_eq!((o.operation, o.leaves), (OpeningOperation::Slide, 2));
    let build = o.build.unwrap();
    assert_eq!((build.overlap, build.track), (Some(0.1), Some(true)));
}

#[test]
fn the_c_key_turns_the_operation_and_the_type_s_comes_back() {
    let mut b = wall();
    assert!(b.type_text("Ç"));
    assert_eq!(row(&b, "Ç").as_deref(), Some("Sürme"));
    assert!(set(&mut b, "Ç", "type"));
    b.click(3., 0.3);
    let o = newest(&b);
    assert_eq!(o.operation, OpeningOperation::Swing);
    assert_eq!(o.build, None);
}
