//! Merkez işareti, Eksen çizgisi and Kot through the session (docs/adr/0322): what each click writes through
//! `cad.dimensions.create`, the rule each binds with, the preview, the level asking for an elevation its point lacks.
//! A Kot in a project with levels (docs/adr/0721 §2): a point with no elevation takes Etkin kat's and follows it;
//! Kat (K) binds it to a level chosen or to none, Fark (F) is its offset.
//! The value height is 2.5 paper mm at 1:1000, 2.5 m; points are east and north differences from (E, N).

use crate::common;

use common::{Bench, rel};
use kentos_commands::{DimensionEntity, DimensionRule, DimensionStyle, Entity};

const EMPTY: &str = include_str!("../../../../../fixtures/interaction/v1/empty.kcad");

fn empty() -> Bench {
    let mut b = Bench::on(EMPTY);
    b.draft.snap = false;
    b
}

fn newest(b: &Bench) -> &DimensionEntity {
    let Entity::Dimension(d) = b.newest() else {
        panic!("a dimension: {:?}", b.newest());
    };
    d
}

fn dimensions(b: &Bench) -> usize {
    b.doc
        .entities()
        .filter(|e| matches!(e, Entity::Dimension(_)))
        .count()
}

#[test]
fn a_centre_mark_goes_on_the_clicked_circle_and_binds_to_it() {
    let mut b = empty();
    b.add_circle("cizim", [10.0, 10.0], 5.0);
    b.start("centerMark");
    assert_eq!(
        b.session.prompt().text(),
        "Merkez işareti: daireye, yaya ya da yay parçasına tıklayın"
    );
    b.move_to(15.0, 10.0);
    let p = b.session.preview(&Default::default()).expect("a preview");
    assert_eq!(
        p.strokes.len(),
        6,
        "the cross's two arms and four axis lines"
    );
    b.click(15.0, 10.0);
    let d = newest(&b);
    assert_eq!(
        (d.style, rel(d.a), rel(d.b)),
        (Some(DimensionStyle::CenterMark), [10.0, 10.0], [15.0, 10.0])
    );
    assert_eq!(d.offset, 2.5);
    let refs = d.refs.as_ref().expect("bound");
    assert_eq!(
        (refs.rule, refs.a.is_none(), refs.b.is_some()),
        (Some(DimensionRule::Curve), true, true)
    );
    assert_eq!(b.last_text(), Some("Merkez işareti eklendi"));
    // The tool stays; a click on nothing writes nothing.
    b.click(40.0, 40.0);
    assert_eq!(dimensions(&b), 1);
    assert_eq!(b.session.tool_id(), "centerMark");
}

#[test]
fn an_axis_goes_between_two_edges_the_second_turned() {
    let mut b = empty();
    b.add_line("cizim", [0.0, 0.0], [20.0, 0.0]);
    b.add_line("cizim", [20.0, 6.0], [0.0, 6.0]);
    b.start("dimCenterLine");
    b.click(5.0, 0.0);
    assert_eq!(
        b.session.prompt().text(),
        "Eksen çizgisi: ikinci düz kenara tıklayın"
    );
    assert_eq!(dimensions(&b), 0);
    b.click(15.0, 6.0);
    let d = newest(&b);
    assert_eq!(
        (d.style, rel(d.a), rel(d.b)),
        (Some(DimensionStyle::CenterLine), [0.0, 3.0], [20.0, 3.0])
    );
    assert_eq!(
        d.refs.as_ref().and_then(|r| r.rule),
        Some(DimensionRule::Pair)
    );
    assert_eq!(
        b.session.prompt().text(),
        "Eksen çizgisi: birinci düz kenara tıklayın"
    );
}

#[test]
fn a_level_asks_for_the_elevation_its_point_lacks() {
    let mut b = empty();
    b.add_point_z("cizim", [0.0, 0.0], 12.5);
    b.start("dimElevation");
    b.draft.snap = true;
    // On the point: its own elevation, bound.
    b.click(0.0, 0.0);
    b.click(3.0, 2.0);
    let d = newest(&b);
    assert_eq!(
        (d.style, d.za, rel(d.a), rel(d.b)),
        (
            Some(DimensionStyle::Elevation),
            Some(12.5),
            [0.0, 0.0],
            [3.0, 2.0]
        )
    );
    assert!(d.refs.as_ref().is_some_and(|r| r.a.is_some()));
    // Away from everything: the elevation is asked for and typed.
    b.click(30.0, 30.0);
    assert_eq!(b.session.prompt().text(), "Kot: noktanın kotunu yazın");
    assert!(b.type_text("-1.5"));
    b.click(33.0, 31.0);
    let d = newest(&b);
    assert_eq!((d.za, d.refs.is_none()), (Some(-1.5), true));
    assert_eq!(dimensions(&b), 2);
}

/// The empty drawing with two levels: Zemin kat at 0, 1. kat at 3.25.
fn levelled() -> Bench {
    let mut b = empty();
    let mut s = b.doc.settings().clone();
    s.arch = Some(
        serde_json::from_value(serde_json::json!({
            "levels": [
                {"id": "zemin", "name": "Zemin kat", "elevation": 0.0},
                {"id": "kat-1", "name": "1. kat", "elevation": 3.25}
            ]
        }))
        .unwrap(),
    );
    b.doc.set_settings(s);
    b
}

fn level_of(d: &DimensionEntity) -> Option<(&str, Option<f64>)> {
    d.level.as_deref().map(|l| (l.id.as_str(), l.offset))
}

#[test]
fn a_kot_away_from_everything_takes_the_active_level_and_follows_it() {
    let mut b = levelled();
    b.level = Some("kat-1".into());
    b.start("dimElevation");
    assert_eq!(b.options(), ["K", "F"], "Kat and Fark while the project has levels");
    // Away from everything: not asked, Etkin kat's elevation.
    b.click(30.0, 30.0);
    assert_eq!(b.session.prompt().text().split(" [").next(), Some("Kot: değerin yerini belirtin"));
    b.click(33.0, 31.0);
    let d = newest(&b);
    assert_eq!((d.za, level_of(d)), (Some(3.25), Some(("kat-1", None))));
    // The level raised in the project's settings: the Kot follows, no undo step of its own.
    let mut s = b.doc.settings().clone();
    s.arch.as_mut().unwrap().levels[1].elevation = 6.5;
    b.doc.set_settings(s);
    assert_eq!(newest(&b).za, Some(6.5));
    assert_eq!(b.doc.undo().as_deref(), Some("Ekle"));
    assert_eq!(dimensions(&b), 0);
}

#[test]
fn kat_and_fark_choose_the_level_and_the_offset() {
    let mut b = levelled();
    b.level = Some("zemin".into());
    b.add_point_z("cizim", [0.0, 0.0], 12.5);
    b.start("dimElevation");
    // Kat: 1. kat chosen; Fark −0.02 typed after F.
    assert!(b.run(|s, cx| s.configure("K", "id:kat-1", cx)));
    assert!(b.type_text("F"));
    assert!(b.session.prompt().text().starts_with("Kot: kata göre farkı yazın"));
    assert!(b.type_text("-0.02"));
    b.draft.snap = true;
    // On the point with its own elevation: a level chosen still wins.
    b.click(0.0, 0.0);
    b.click(2.0, 2.0);
    let d = newest(&b);
    assert_eq!((d.za, level_of(d)), (Some(3.25 + -0.02), Some(("kat-1", Some(-0.02)))));
    assert!(d.refs.as_ref().is_some_and(|r| r.a.is_some()), "bound to the point for its place");
    // Etkin kat again: the point's own elevation is read, no level.
    assert!(b.run(|s, cx| s.configure("K", "etkin", cx)));
    b.click(0.0, 0.0);
    b.click(-2.0, 2.0);
    let d = newest(&b);
    assert_eq!((d.za, level_of(d)), (Some(12.5), None));
    // Yok: a point with no elevation asks for one, as without levels.
    assert!(b.run(|s, cx| s.configure("K", "yok", cx)));
    b.draft.snap = false;
    b.click(40.0, 40.0);
    assert_eq!(b.session.prompt().text().split(" [").next(), Some("Kot: noktanın kotunu yazın"));
    assert!(b.type_text("1.5"));
    b.click(42.0, 41.0);
    let d = newest(&b);
    assert_eq!((d.za, level_of(d)), (Some(1.5), None));
    // A level the project lacks is refused and said.
    assert!(!b.run(|s, cx| s.configure("K", "id:kat-9", cx)));
    assert_eq!(b.last_text(), Some("Projede böyle bir kat yok."));
}

#[test]
fn a_kot_waiting_reads_the_new_active_level_at_once() {
    let mut b = levelled();
    b.level = Some("zemin".into());
    b.start("dimElevation");
    b.click(30.0, 30.0);
    b.move_to(32.0, 31.0);
    // The status bar's level changes while the value waits: the preview and the mark take kat-1.
    b.level = Some("kat-1".into());
    b.run(|s, cx| s.level_changed(cx));
    b.click(32.0, 31.0);
    let d = newest(&b);
    assert_eq!((d.za, level_of(d)), (Some(3.25), Some(("kat-1", None))));
    // Katsız: no level to take, the elevation is asked for.
    b.level = None;
    b.run(|s, cx| s.level_changed(cx));
    b.click(50.0, 50.0);
    assert_eq!(b.session.prompt().text().split(" [").next(), Some("Kot: noktanın kotunu yazın"));
}
