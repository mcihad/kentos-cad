//! İç ölçü zinciri (docs/adr/0711 §5): a cut drawn across walls, its chain previewed from the command's plan, placed by
//! a third click (or Enter on the cut) in one undo step; Esc steps back; the tool stays for the next chain.

use crate::common::{Bench, E, N};
use kentos_commands::{DimensionRule, DrawingUnit, Entity, Workspace};
use kentos_domain::Slot;

fn bench() -> Bench {
    let mut b = Bench::new("wall");
    let mut settings = b.doc.settings().clone();
    settings.workspace = Some(Workspace::Cad);
    settings.srid = 0;
    settings.drawing_unit = Some(DrawingUnit::M);
    b.doc.set_settings(settings);
    b
}

fn wall(b: &mut Bench, pts: &[(f64, f64)]) -> Slot {
    b.start("wall");
    for &(x, y) in pts {
        b.click(x, y);
    }
    b.confirm();
    b.confirm();
    Slot(b.newest().base().id)
}

/// Three walls across, 4 m apart, 20 cm thick.
fn rooms() -> Bench {
    let mut b = bench();
    for x in [0., 4., 8.] {
        wall(&mut b, &[(x, 0.), (x, 4.)]);
    }
    b.selection.clear();
    b
}

/// A dimension's two points (local) and whether both follow the cut.
type Measured = ((f64, f64), (f64, f64), bool);

/// The drawing's dimensions.
fn dimensions(b: &Bench) -> Vec<Measured> {
    b.doc
        .entities()
        .filter_map(|e| match e {
            Entity::Dimension(d) => Some((
                (d.a.x - E, d.a.y - N),
                (d.b.x - E, d.b.y - N),
                d.refs.as_ref().is_some_and(|r| {
                    r.rule == Some(DimensionRule::Cross) && r.a.is_some() && r.b.is_some()
                }),
            )),
            _ => None,
        })
        .collect()
}

fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} ≠ {b}");
}

#[test]
fn three_clicks_write_the_chain_once_and_the_tool_stays() {
    let mut b = rooms();
    b.start("interiorDimensions");
    b.click(-1., 2.);
    b.move_to(9., 2.);
    let before = b.doc.to_snapshot_v2();
    let preview = b.session.preview(&Default::default()).expect("the chain");
    assert_eq!(preview.texts.len(), 5, "six faces, five dimensions");
    assert_eq!(b.doc.to_snapshot_v2(), before, "the preview writes nothing");
    b.click(9., 2.);
    assert_eq!(b.points(), 2);
    b.move_to(1., 3.);
    assert_eq!(b.doc.to_snapshot_v2(), before);
    b.click(1., 3.);
    let made = dimensions(&b);
    assert_eq!(made.len(), 5);
    assert!(
        made.iter().all(|(_, _, cross)| *cross),
        "both ends bound, the cut's rule"
    );
    let xs: Vec<f64> = made.iter().map(|(a, _, _)| a.0).collect();
    for (x, want) in xs.iter().zip([-0.1, 0.1, 3.9, 4.1, 7.9]) {
        close(*x, want);
    }
    assert_eq!(b.selection.len(), 5, "the chain selected");
    assert!(b.session.is_running(), "the tool stays for the next chain");
    assert_eq!(b.points(), 0);
    b.doc.undo().unwrap();
    assert!(dimensions(&b).is_empty(), "one undo step");
}

#[test]
fn enter_puts_the_chain_on_the_cut_and_esc_steps_back() {
    let mut b = rooms();
    b.start("interiorDimensions");
    b.click(-1., 2.);
    b.click(5., 2.);
    assert_eq!(b.points(), 2);
    assert!(
        b.run(|session, cx| session.cancel(cx)),
        "back to the cut's end"
    );
    assert_eq!(b.points(), 1);
    assert!(dimensions(&b).is_empty());
    b.click(5., 2.);
    b.confirm();
    let made = dimensions(&b);
    assert_eq!(made.len(), 3, "faces at −0.1, 0.1, 3.9, 4.1");
    let offsets: Vec<f64> = b
        .doc
        .entities()
        .filter_map(|e| match e {
            Entity::Dimension(d) => Some(d.offset),
            _ => None,
        })
        .collect();
    assert!(
        offsets.iter().all(|o| o.abs() < 1e-9),
        "on the cut: {offsets:?}"
    );
    b.click(-1., 3.);
    assert!(b.run(|session, cx| session.cancel(cx)));
    assert_eq!(b.points(), 0);
    assert!(
        !b.run(|session, cx| session.cancel(cx)),
        "nothing left to step back"
    );
}

#[test]
fn a_cut_crossing_one_face_says_why_and_keeps_drawing() {
    let mut b = rooms();
    b.start("interiorDimensions");
    b.click(-1., 2.);
    b.click(0., 2.);
    assert_eq!(b.points(), 1, "the cut is drawn again");
    assert!(dimensions(&b).is_empty());
    let settings = b.session.settings(&Default::default());
    assert!(settings.iter().any(|s| s.key == "E"), "why it is said");
}

#[test]
fn the_settings_change_the_preview_and_are_remembered() {
    let mut b = bench();
    wall(&mut b, &[(0., 0.), (0., 4.)]);
    wall(&mut b, &[(4., 0.), (4., 4.)]);
    b.selection.clear();
    b.start("interiorDimensions");
    let settings = b.session.settings(&Default::default());
    for key in ["C", "L", "S", "F", "K", "T", "V", "H"] {
        assert!(settings.iter().any(|s| s.key == key), "{key}");
    }
    assert!(b.run(|session, cx| session.configure("T", "yes", cx)));
    b.click(-1., 2.);
    b.move_to(5., 2.);
    let texts = b.session.preview(&Default::default()).unwrap().texts.len();
    assert_eq!(texts, 4, "three dimensions and the total");
    assert!(!b.run(|session, cx| session.configure("K", "everything", cx)));
    b.run(|session, cx| session.cancel(cx));
    b.run(|session, cx| session.cancel(cx));
    b.run(|session, cx| session.cancel(cx));
    b.start("interiorDimensions");
    let settings = b.session.settings(&Default::default());
    let total = settings.iter().find(|s| s.key == "T").unwrap();
    assert_eq!(total.value, "Açık", "remembered");
}
