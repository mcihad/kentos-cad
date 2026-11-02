//! The constraints' marks over the drawing (docs/adr/0338 §1): a mark by each part a constraint names, one where two
//! parts meet, a dimension's name and value on its first mark, a constraint the drawing does not meet marked so, and
//! the parts one constraint names for the pointer's highlight.

use crate::common;

use common::Bench;
use kentos_commands::{
    CadElement, CommandResult, ConstraintEdit, ConstraintRule, ConstraintSense, ConstraintsEdit,
    DistanceAxis, ElementRef, EntityId, NewConstraint, SubPlace,
};
use kentos_domain::Slot;
use kentos_interaction::constraint_marks::{bound, glyphs};
use kentos_native_application::{ExecutionContext, constraints_edit};

const EMPTY: &str = include_str!("../../../../../fixtures/interaction/v1/empty.kcad");

fn part(b: &Bench, slot: Slot, element: CadElement) -> ElementRef {
    ElementRef {
        uid: EntityId(b.doc.uid(slot).expect("an object").into_bytes()),
        element,
    }
}

fn point(b: &Bench, slot: Slot, place: SubPlace) -> ElementRef {
    part(b, slot, CadElement::Point { place })
}

fn segment(b: &Bench, slot: Slot) -> ElementRef {
    part(
        b,
        slot,
        CadElement::Segment {
            part: 0,
            ring: 0,
            index: 0,
        },
    )
}

fn add(b: &mut Bench, rules: Vec<(ConstraintRule, Option<&str>)>) {
    let constraints = rules
        .into_iter()
        .map(|(rule, value)| NewConstraint {
            rule,
            sense: ConstraintSense::Drawing,
            value: value.map(str::to_owned),
            reference: false,
        })
        .collect();
    let input = ConstraintsEdit {
        edit: ConstraintEdit::Add { constraints },
        hold: Vec::new(),
        allow_flip: false,
        expected_revision: None,
    };
    match constraints_edit::execute(&mut ExecutionContext::new(&mut b.doc), input) {
        CommandResult::Completed { .. } => {}
        other => panic!("not written: {other:?}"),
    }
}

/// An L: a bottom line 10 long and a side line up from its end.
fn ell() -> (Bench, Slot, Slot) {
    let mut b = Bench::on(EMPTY);
    let bottom = b.add_line("cizim", [0.0, 0.0], [10.0, 0.0]);
    let side = b.add_line("cizim", [10.0, 0.0], [10.0, 6.0]);
    (b, bottom, side)
}

#[test]
fn a_drawing_without_constraints_has_no_marks() {
    let (b, _, _) = ell();
    assert!(glyphs(&b.doc).is_empty());
}

#[test]
fn each_part_has_its_mark_and_a_meeting_point_one() {
    let (mut b, bottom, side) = ell();
    let rules = vec![
        (
            ConstraintRule::Coincident {
                a: point(&b, bottom, SubPlace::End),
                b: point(&b, side, SubPlace::Start),
            },
            None,
        ),
        (
            ConstraintRule::Perpendicular {
                a: segment(&b, bottom),
                b: segment(&b, side),
            },
            None,
        ),
        (
            ConstraintRule::Distance {
                a: point(&b, bottom, SubPlace::Start),
                b: point(&b, bottom, SubPlace::End),
                axis: DistanceAxis::Aligned,
                left: false,
                parameter: "genislik".into(),
            },
            Some("10"),
        ),
    ];
    add(&mut b, rules);
    let marks = glyphs(&b.doc);
    let by = |index: usize| {
        marks
            .iter()
            .filter(|g| g.index == index)
            .collect::<Vec<_>>()
    };
    // The two ends meet: one mark.
    assert_eq!(by(0).len(), 1);
    assert_eq!(by(0)[0].icon, "constraintCoincident");
    // Two edges: a mark at each one's middle.
    let square = by(1);
    assert_eq!(square.len(), 2);
    assert_eq!(square[0].title, "Dik");
    assert!((square[0].at.x - (common::E + 5.0)).abs() < 1e-9);
    // A dimension: its name and value on its first mark only.
    let width = by(2);
    assert_eq!(width.len(), 2);
    assert_eq!(width[0].label.as_deref(), Some("genislik = 10 m"));
    assert_eq!(width[1].label, None);
    assert!(marks.iter().all(|g| !g.unmet && !g.reference));
    // The pointer on the perpendicular's mark shows its two edges.
    let parts = bound(&b.doc, 1);
    assert_eq!(parts.len(), 2);
    assert!(parts.iter().all(|p| p.edges.len() == 1));
    assert!(bound(&b.doc, 9).is_empty());
}

#[test]
fn parameter_marks_share_project_units_and_keep_small_nonzero_values_visible() {
    use kentos_commands::{AngleUnit, DrawingUnit, ParameterKind};
    use kentos_interaction::constraint_marks::parameter_value_words as words;
    let (mut b, bottom, _) = ell();
    let mut settings = b.doc.settings().clone();
    settings.srid = 0;
    settings.drawing_unit = Some(DrawingUnit::Mm);
    settings.angle_unit = AngleUnit::Grad;
    b.doc.set_settings(settings);
    let rules = vec![(
        ConstraintRule::Distance {
            a: point(&b, bottom, SubPlace::Start),
            b: point(&b, bottom, SubPlace::End),
            axis: DistanceAxis::Aligned,
            left: false,
            parameter: "boy".into(),
        },
        Some("10000mm"),
    )];
    add(&mut b, rules);
    assert_eq!(glyphs(&b.doc)[0].label.as_deref(), Some("boy = 10000 mm"));
    let s = b.doc.settings();
    assert_eq!(words(100.0, ParameterKind::Angle, s), "100 gon");
    assert_eq!(words(0.00000002, ParameterKind::Length, s), "2e-8 mm");
    assert_eq!(words(-0.0, ParameterKind::Number, s), "0 · sayı");
    let mut deg = s.clone();
    deg.angle_unit = AngleUnit::Deg;
    assert_eq!(words(90.0, ParameterKind::Angle, &deg), "90 °");
    assert_eq!(words(f64::NAN, ParameterKind::Length, s), "— mm");
}
