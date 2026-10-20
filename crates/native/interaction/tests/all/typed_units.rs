//! Lengths typed with their own unit and sums in brackets (docs/adr/0266) through the session, in a millimetre
//! project: the points, a radius, a polygon's radius and a split length all read them as the grammar does.

use crate::common;

use common::Bench;
use kentos_commands::{AngleUnit, DrawingUnit, Entity, ProjectSettings, Workspace};

fn millimetres(tool: &str) -> Bench {
    let mut b = Bench::new(tool);
    let settings = ProjectSettings {
        srid: 0,
        drawing_unit: Some(DrawingUnit::Mm),
        workspace: Some(Workspace::Cad),
        angle_unit: AngleUnit::Deg,
        ..b.doc.settings().clone()
    };
    b.doc.set_settings(settings);
    b
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-12
}

#[test]
fn a_line_takes_metres_centimetres_and_sums() {
    let mut b = millimetres("line");
    for text in ["0,0", "@2.5m<0", "@(1.2+0.3)m<90", "@-50cm,(100*2)"] {
        assert!(b.type_text(text), "{text}");
    }
    let mut ends: Vec<(u32, [f64; 4])> = b
        .doc
        .entities()
        .map(|e| match e {
            Entity::Line(l) => (e.base().id, [l.a.x, l.a.y, l.b.x, l.b.y]),
            other => panic!("a line, not {other:?}"),
        })
        .collect();
    ends.sort_by_key(|(id, _)| *id);
    let [first, second, third] = [ends[0].1, ends[1].1, ends[2].1];
    assert_eq!(first, [0.0, 0.0, 2.5, 0.0]);
    assert!(near(second[2], 2.5) && near(second[3], 1.5), "{second:?}");
    // −50 cm east, 200 mm north.
    assert!(near(third[2], 2.0) && near(third[3], 1.7), "{third:?}");
}

#[test]
fn a_radius_takes_a_unit_or_a_sum() {
    let mut b = millimetres("circle");
    assert!(b.type_text("0,0"));
    assert!(b.type_text("0.75m"));
    let Entity::Circle(c) = b.newest() else {
        panic!("a circle, not {:?}", b.newest());
    };
    assert_eq!(c.r, 0.75);
    let mut b = millimetres("circle");
    assert!(b.type_text("0,0"));
    assert!(b.type_text("(10+2.5)"));
    let Entity::Circle(c) = b.newest() else {
        panic!("a circle, not {:?}", b.newest());
    };
    assert_eq!(c.r, 0.0125);
}

#[test]
fn feet_and_inches_are_a_radius_too() {
    let mut b = millimetres("circle");
    assert!(b.type_text("0,0"));
    assert!(b.type_text("1'"));
    let Entity::Circle(c) = b.newest() else {
        panic!("a circle, not {:?}", b.newest());
    };
    assert!(near(c.r, 0.3048), "{}", c.r);
}
