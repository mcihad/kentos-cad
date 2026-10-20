//! Merkez işareti, Eksen çizgisi and Kot through the session (docs/adr/0322): what each click writes through
//! `cad.dimensions.create`, the rule each binds with, the preview, the level asking for an elevation its point lacks.
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
