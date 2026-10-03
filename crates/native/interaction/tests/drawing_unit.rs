//! A local project's drawing unit (docs/adr/0165 §2) through the session:
//! what is typed is in the unit, what the drawing keeps is metres. The web's
//! are `apps/web/src/tools/drawingUnit.test.ts`.

mod common;

use common::Bench;
use kentos_contracts::{DrawingUnit, Entity, ProjectSettings};

/// `tool` running on the empty drawing made local, in `unit`.
fn local(tool: &str, unit: DrawingUnit) -> Bench {
    let mut b = Bench::new(tool);
    let settings = ProjectSettings {
        srid: 0,
        drawing_unit: Some(unit),
        ..b.doc.settings().clone()
    };
    b.doc.set_settings(settings);
    b
}

fn ends(e: &Entity) -> [[f64; 2]; 2] {
    let Entity::Line(line) = e else {
        panic!("a line, not {e:?}");
    };
    [[line.a.x, line.a.y], [line.b.x, line.b.y]]
}

#[test]
fn typed_points_are_in_millimetres() {
    let mut b = local("line", DrawingUnit::Mm);
    for text in ["100,250", "@500,0", "@100<90"] {
        assert!(b.type_text(text), "{text}");
    }
    let mut lines: Vec<(u32, [[f64; 2]; 2])> =
        b.doc.entities().map(|e| (e.base().id, ends(e))).collect();
    lines.sort_by_key(|(id, _)| *id);
    let lines: Vec<[[f64; 2]; 2]> = lines.into_iter().map(|(_, l)| l).collect();
    assert_eq!(lines[0], [[0.1, 0.25], [0.6, 0.25]]);
    let [a, end] = lines[1];
    assert_eq!(a, [0.6, 0.25]);
    assert!(
        (end[0] - 0.6).abs() < 1e-12 && (end[1] - 0.35).abs() < 1e-12,
        "{end:?}"
    );
}

#[test]
fn a_typed_radius_is_in_millimetres() {
    let mut b = local("circle", DrawingUnit::Mm);
    assert!(b.type_text("0,0"));
    assert!(b.type_text("12.5"));
    let Entity::Circle(circle) = b.newest() else {
        panic!("a circle, not {:?}", b.newest());
    };
    assert_eq!(circle.r, 0.0125);
}

#[test]
fn a_project_with_a_coordinate_system_is_in_metres() {
    let mut b = local("line", DrawingUnit::Mm);
    let settings = ProjectSettings {
        srid: 5254,
        ..b.doc.settings().clone()
    };
    b.doc.set_settings(settings);
    for text in ["100,250", "@5,0"] {
        assert!(b.type_text(text), "{text}");
    }
    assert_eq!(ends(b.newest()), [[100.0, 250.0], [105.0, 250.0]]);
}
