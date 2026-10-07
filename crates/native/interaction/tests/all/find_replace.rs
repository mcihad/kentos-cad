//! Bul ve değiştir's matches and write (docs/adr/0145 §6) over a document:
//! the core's rule decides (fixtures/text/v1/pattern.json holds it); here the
//! scope, the locked and emptied texts, the trimming and the one step. The
//! web's are apps/web/src/ui/text/findReplace.test.ts.

use crate::common;

use common::{Bench, E, N, base};
use kentos_contracts::{Entity, TextEntity};
use kentos_domain::Slot;
use kentos_geometry_core::text::edit::Find;
use kentos_interaction::find_replace::{Blocked, Query, matches, write};

const TEXTS: &str = include_str!("../../../../../fixtures/interaction/v1/texts.kcad");

/// The trace's drawing (Ada 101, Yol 12, Park; Kilitli on a locked layer)
/// with Ada 102, Ada 9 on the locked layer and a bare Ada: slots 5 to 7.
fn scene() -> Bench {
    let mut b = Bench::on(TEXTS);
    for (layer, words, y) in [
        ("cizim", "Ada 102", 20.0),
        ("kilitli", "Ada 9", 24.0),
        ("cizim", "Ada", 28.0),
    ] {
        b.doc
            .add(Entity::Text(TextEntity {
                base: base(layer),
                p: kentos_contracts::Vec2 { x: E, y: N + y },
                text: words.to_owned(),
                height: 2.0,
                rotation: 0.0,
                align: None,
                width_factor: None,
                mask: false,
                label_of: None,
                label_scale: None,
                paragraph: Default::default(),
                face: Default::default(),
                path: None,
            }))
            .expect("a slot");
    }
    b
}

fn query(find: &str, replace: &str, how: Find) -> Query {
    Query {
        find: find.to_owned(),
        replace: replace.to_owned(),
        how,
    }
}

const CASELESS: Find = Find {
    wildcard: false,
    caseless: true,
    whole_word: false,
};

fn words(b: &Bench, slot: u32) -> String {
    match b.doc.get(Slot(slot)) {
        Some(Entity::Text(t)) => t.text.clone(),
        other => panic!("a text at {slot}: {other:?}"),
    }
}

#[test]
fn every_text_the_rule_changes_is_found_with_its_layer_a_locked_one_blocked() {
    let b = scene();
    let found = matches(&b.doc, None, &query("Ada", "Parsel", CASELESS));
    let rows: Vec<(u32, &str, &str, &str, Option<Blocked>)> = found
        .iter()
        .map(|m| {
            (
                m.slot.0,
                m.layer.as_str(),
                m.old.as_str(),
                m.new.as_str(),
                m.blocked,
            )
        })
        .collect();
    assert_eq!(
        rows,
        [
            (1, "Çizim", "Ada 101", "Parsel 101", None),
            (5, "Çizim", "Ada 102", "Parsel 102", None),
            (6, "Kilitli", "Ada 9", "Parsel 9", Some(Blocked::Locked)),
            (7, "Çizim", "Ada", "Parsel", None),
        ]
    );
}

#[test]
fn wildcards_match_the_whole_text_and_carry_what_each_star_took() {
    let b = scene();
    let how = Find {
        wildcard: true,
        ..CASELESS
    };
    let found = matches(&b.doc, None, &query("Ada *", "Parsel *", how));
    let rows: Vec<(u32, &str)> = found.iter().map(|m| (m.slot.0, m.new.as_str())).collect();
    assert_eq!(
        rows,
        [(1, "Parsel 101"), (5, "Parsel 102"), (6, "Parsel 9")]
    );
}

#[test]
fn what_becomes_empty_is_blocked_and_what_is_left_trimmed() {
    let b = scene();
    let found = matches(&b.doc, None, &query("Ada", "", CASELESS));
    let at = |slot: u32| found.iter().find(|m| m.slot.0 == slot).expect("a match");
    assert_eq!(at(1).new, "101");
    assert_eq!(
        (at(7).new.as_str(), at(7).blocked),
        ("", Some(Blocked::Empty))
    );
}

#[test]
fn case_and_the_selection_narrow_it() {
    let b = scene();
    let exact = Find {
        caseless: false,
        ..CASELESS
    };
    assert!(matches(&b.doc, None, &query("ada", "X", exact)).is_empty());
    assert_eq!(matches(&b.doc, None, &query("ada", "X", CASELESS)).len(), 4);
    let only = matches(
        &b.doc,
        Some(&[Slot(1), Slot(2)]),
        &query("Ada", "X", CASELESS),
    );
    assert_eq!(only.iter().map(|m| m.slot.0).collect::<Vec<_>>(), [1]);
}

#[test]
fn what_may_be_written_goes_in_one_step_the_rest_stays() {
    let mut b = scene();
    let found = matches(&b.doc, None, &query("Ada", "Parsel", CASELESS));
    let (n, said) = write(&mut b.doc, &found);
    assert_eq!((n, said.len()), (3, 0));
    let texts: Vec<String> = [1, 5, 6, 7, 2].iter().map(|&s| words(&b, s)).collect();
    assert_eq!(
        texts,
        ["Parsel 101", "Parsel 102", "Ada 9", "Parsel", "Yol 12"]
    );
    // The text's alignment stays: Ada 101 is centred.
    let Some(Entity::Text(t)) = b.doc.get(Slot(1)) else {
        panic!("a text");
    };
    assert_eq!(t.align, Some(kentos_contracts::TextAlign::MiddleCenter));
    assert_eq!(b.doc.undo().as_deref(), Some("Bul ve değiştir"));
    assert_eq!(words(&b, 1), "Ada 101");
    assert_eq!(write(&mut b.doc, &[]), (0, Vec::new()));
}
