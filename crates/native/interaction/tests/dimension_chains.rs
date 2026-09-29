//! Zincir ölçü and Baz ölçü through the session, over the native document
//! (docs/adr/0140, phase 2): where each starts (the dimension the session
//! drew last, or one clicked), what each click writes, the baseline's levels
//! of three text heights, one undo step per dimension, the run following an
//! undo. The text is 2.5 paper mm at 1:1000, so 2.5 m high and a baseline
//! level 7.5 m. Points are east and north differences from (E, N); expected
//! values are worked out by hand.

mod common;

use common::{Bench, rel};
use kentos_contracts::{DimensionEntity, DimensionStyle, Entity};
use kentos_interaction::Level;

fn near(have: f64, want: f64) -> bool {
    (have - want).abs() < 1e-9
}

fn near2(have: [f64; 2], want: [f64; 2]) -> bool {
    near(have[0], want[0]) && near(have[1], want[1])
}

/// The newest object, a dimension.
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

/// The aligned dimension from (0, 0) to (10, 0) with its line 3 m to the left
/// (north), drawn by the Ölçü tool, which then leaves: the last one drawn.
fn with_base(line_at: f64) -> Bench {
    let mut b = Bench::new("dimension");
    b.click(0.0, 0.0);
    b.click(10.0, 0.0);
    b.click(5.0, line_at);
    assert_eq!(dimensions(&b), 1);
    b.confirm();
    assert_eq!(b.session.tool_id(), "select");
    b
}

#[test]
fn a_chain_measures_on_from_the_end_of_the_last_dimension() {
    let mut b = with_base(3.0);
    b.start("dimContinue");
    assert_eq!(
        b.session.prompt().text(),
        "Zincir ölçü: sonraki ölçü noktasını belirtin [Ölçü seç (S)]"
    );
    // Live: the next dimension and its value.
    b.move_to(16.0, 0.0);
    let p = b.session.preview(&Default::default()).expect("a preview");
    assert!(!p.strokes.is_empty());
    assert_eq!(p.tag.expect("a tag").lines, ["6.000"]);
    b.click(16.0, 0.0);
    let d = newest(&b);
    assert_eq!((rel(d.a), rel(d.b)), ([10.0, 0.0], [16.0, 0.0]));
    // Linear along the base's direction (east), on the base's line.
    assert_eq!(
        (d.style, d.angle, d.c, d.text.as_deref()),
        (Some(DimensionStyle::Linear), Some(0.0), None, None)
    );
    assert!(near(d.offset, 3.0) && near(d.height, 2.5), "{d:?}");
    assert_eq!(b.last_level(), Some(Level::Success));
    assert_eq!(b.last_text(), Some("Zincir ölçü eklendi: 6.000"));
    // The next one starts where this ended.
    b.click(21.0, 0.0);
    let d = newest(&b);
    assert_eq!((rel(d.a), rel(d.b)), ([16.0, 0.0], [21.0, 0.0]));
    assert!(near(d.offset, 3.0));
    assert_eq!(dimensions(&b), 3);
    // Each dimension is its own undo step.
    assert_eq!(b.doc.undo().as_deref(), Some("Zincir ölçü"));
    assert_eq!(dimensions(&b), 2);
    // Enter ends.
    b.confirm();
    assert_eq!(b.session.tool_id(), "select");
}

#[test]
fn a_baseline_stacks_lines_three_text_heights_apart() {
    let mut b = with_base(3.0);
    b.start("dimBaseline");
    assert_eq!(
        b.session.prompt().text(),
        "Baz ölçü: sonraki noktayı belirtin [kat 7.500 m; Ölçü seç (S)]"
    );
    b.click(16.0, 0.0);
    let d = newest(&b);
    // From the base's first point, its line one level (7.5 m) beyond the base's 3 m.
    assert_eq!((rel(d.a), rel(d.b)), ([0.0, 0.0], [16.0, 0.0]));
    assert!(near(d.offset, 10.5), "{d:?}");
    assert_eq!(d.style, Some(DimensionStyle::Linear));
    assert_eq!(b.last_text(), Some("Baz ölçü eklendi: 16.000"));
    b.click(21.0, 0.0);
    let d = newest(&b);
    assert_eq!((rel(d.a), rel(d.b)), ([0.0, 0.0], [21.0, 0.0]));
    assert!(near(d.offset, 18.0), "the second level: {d:?}");
    assert_eq!(dimensions(&b), 3);
    assert_eq!(b.doc.undo().as_deref(), Some("Baz ölçü"));
    assert_eq!(dimensions(&b), 2);
}

#[test]
fn a_baseline_grows_away_from_the_measured_points_on_either_side() {
    let mut b = with_base(-3.0);
    b.start("dimBaseline");
    b.click(16.0, 0.0);
    assert!(near(newest(&b).offset, -10.5), "{:?}", newest(&b));
    b.click(21.0, 0.0);
    assert!(near(newest(&b).offset, -18.0));
}

#[test]
fn a_linear_base_gives_its_own_direction() {
    // A linear dimension measured along north (its angle 90), drawn by the Ölçü tool.
    let mut b = Bench::new("dimension");
    assert!(b.type_text("D"));
    b.click(0.0, 0.0);
    b.click(0.0, 10.0);
    assert!(b.type_text("X"));
    b.click(4.0, 5.0);
    let base = newest(&b).clone();
    assert_eq!(
        (base.style, base.angle),
        (Some(DimensionStyle::Linear), Some(90.0))
    );
    b.confirm();
    b.start("dimContinue");
    b.click(0.0, 17.0);
    let d = newest(&b);
    assert_eq!((rel(d.a), rel(d.b)), ([0.0, 10.0], [0.0, 17.0]));
    assert_eq!(d.angle, Some(90.0));
    assert!(near(d.offset, base.offset), "{d:?} {base:?}");
}

#[test]
fn with_no_dimension_known_a_click_on_one_chooses_it() {
    let mut b = with_base(3.0);
    b.memory.last_dimension = None;
    b.start("dimContinue");
    assert_eq!(
        b.session.prompt().text(),
        "Zincir ölçü: ölçüsü sürdürülecek hizalı ya da doğrusal ölçüye tıklayın"
    );
    // A click that is on no dimension is said.
    b.click(5.0, -8.0);
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert!(
        b.last_text()
            .expect("said")
            .starts_with("Hizalı ya da doğrusal bir ölçüye")
    );
    // On its dimension line.
    b.click(5.0, 3.0);
    assert_eq!(
        b.session.prompt().text(),
        "Zincir ölçü: sonraki ölçü noktasını belirtin [Ölçü seç (S)]"
    );
    b.click(14.0, 0.0);
    let d = newest(&b);
    assert_eq!((rel(d.a), rel(d.b)), ([10.0, 0.0], [14.0, 0.0]));
}

#[test]
fn a_dimension_that_is_gone_is_asked_for_again() {
    let mut b = with_base(3.0);
    assert!(b.doc.undo().is_some());
    assert_eq!(dimensions(&b), 0);
    b.start("dimBaseline");
    assert!(b.session.prompt().text().contains("tıklayın"));
    // Nothing to chain with: a click on empty ground warns, Enter leaves.
    b.click(2.0, 2.0);
    assert_eq!(b.last_level(), Some(Level::Warn));
    b.confirm();
    assert_eq!(b.session.tool_id(), "select");
    assert_eq!(dimensions(&b), 0);
}

#[test]
fn an_angle_dimension_cannot_be_chained() {
    let mut b = Bench::new("dimension");
    assert!(b.type_text("A"));
    assert!(b.type_text("K"));
    b.click(0.0, 0.0);
    b.click(6.0, 0.0);
    b.click(0.0, 6.0);
    b.click(4.0, 4.0);
    assert_eq!(dimensions(&b), 1);
    b.confirm();
    // Not aligned or linear: the tool asks for a click, and refuses this one.
    b.start("dimContinue");
    assert!(b.session.prompt().text().contains("tıklayın"));
    b.click(4.243, 4.243);
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert_eq!(dimensions(&b), 1);
}

#[test]
fn an_undone_dimension_takes_the_run_back_to_where_it_was() {
    let mut b = with_base(3.0);
    b.start("dimContinue");
    b.click(16.0, 0.0);
    b.click(21.0, 0.0);
    assert_eq!(dimensions(&b), 3);
    // The drawing's undo takes the newest back; the next click measures on from 16.
    assert!(b.doc.undo().is_some());
    b.click(19.0, 0.0);
    let d = newest(&b);
    assert_eq!((rel(d.a), rel(d.b)), ([16.0, 0.0], [19.0, 0.0]));
    assert_eq!(dimensions(&b), 3);
}

#[test]
fn a_point_on_the_last_end_makes_no_dimension_and_s_chooses_another_base() {
    let mut b = with_base(3.0);
    b.start("dimContinue");
    b.click(10.0, 0.0);
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert!(
        b.last_text()
            .expect("said")
            .starts_with("Bu yerde ölçü oluşmuyor")
    );
    assert_eq!(dimensions(&b), 1);
    // S: a base is asked for; Esc goes back to the one the run had.
    assert!(b.type_text("S"));
    assert!(b.session.prompt().text().contains("tıklayın"));
    assert!(b.run(|s, cx| s.cancel(cx)));
    assert!(b.session.prompt().text().contains("sonraki ölçü noktasını"));
    // Typed points work as clicks: 4 m along from the end.
    assert!(b.type_text("@4,0"));
    assert_eq!(dimensions(&b), 2);
    assert_eq!(rel(newest(&b).b)[0], 14.0);
    assert!(near2(rel(newest(&b).a), [10.0, 0.0]));
}
