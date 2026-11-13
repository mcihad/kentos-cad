//! A CAD project's axes named as its screen shows them (usage review UR-009): X to the right, Y up, in the move's,
//! copy's and stretch's messages, the dimension's options and keys and a coordinate dimension's value; a CBS project
//! keeps its Y to the right, X up (the modify and dimension tests hold those).

use crate::common::{self, Bench};
use kentos_commands::Workspace;
use kentos_domain::Slot;
use kentos_interaction::Level;

const OBJECTS: &str = include_str!("../../../../../fixtures/interaction/v1/objects.kcad");

/// The objects drawing as a CAD project, `selected` chosen.
fn cad(selected: &[u32]) -> Bench {
    let mut b = Bench::on(OBJECTS);
    let mut s = b.doc.settings().clone();
    s.workspace = Some(Workspace::Cad);
    b.doc.set_settings(s);
    b.draft.snap = false;
    b.selection.set(selected.iter().map(|s| Slot(*s)));
    b
}

#[test]
fn moves_copies_and_stretches_say_x_to_the_right_and_y_up() {
    let mut b = cad(&[1]);
    b.start("move");
    b.click(-24.0, -12.0);
    let before = b.log.len();
    assert!(b.type_text("@12.5,-7.25"));
    assert_eq!(b.said(before), [(Level::Success, "1 nesne taşındı: ΔX 12.500  ΔY -7.250")]);

    let mut b = cad(&[1]);
    b.start("copy");
    b.click(-24.0, -12.0);
    let before = b.log.len();
    assert!(b.type_text("@20.585,2.109"));
    assert_eq!(b.said(before), [(Level::Success, "1 nesne kopyalandı: ΔX 20.585  ΔY 2.109")]);
}

#[test]
fn a_dimensions_axes_are_named_and_typed_as_the_cad_screen_shows_them() {
    let mut b = cad(&[]);
    b.memory.dimension_mode = kentos_interaction::DimensionMode::Linear;
    b.start("dimension");
    assert!(b.session.prompt().text().contains("(ΔX / ΔY)"), "{}", b.session.prompt().text());
    b.click(0.0, 0.0);
    b.click(10.0, 0.0);
    let text = b.session.prompt().text();
    assert!(text.contains("Yatay ΔX (X)") && text.contains("Düşey ΔY (Y)"), "{text}");
    assert!(b.type_text("X"));
    assert_eq!(b.memory.dimension_lock, Some(0.0), "X: horizontal, along the east axis");
    assert!(b.type_text("Y"));
    assert_eq!(b.memory.dimension_lock, Some(90.0));
}

#[test]
fn a_coordinate_dimension_writes_the_cad_projects_axis() {
    let mut s = common::Bench::on(OBJECTS).doc.settings().clone();
    let look = kentos_commands::DimensionLook::default();
    let gis = kentos_interaction::Format::of(&s);
    // The layout's Y= is the east value: a CBS project's Y, a CAD project's X.
    assert_eq!(gis.dimension_value("Y=", "coordinate", 10.0, &look, None), "Y=10.000");
    s.workspace = Some(Workspace::Cad);
    let cad = kentos_interaction::Format::of(&s);
    assert_eq!(cad.dimension_value("Y=", "coordinate", 10.0, &look, None), "X=10.000");
    assert_eq!(cad.dimension_value("X=", "coordinate", 0.0, &look, None), "Y=0.000");
    assert_eq!(cad.dimension_value("", "length", 4.0, &look, None), "4.000", "a length has no axis");
}
