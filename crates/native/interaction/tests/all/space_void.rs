//! Mahal's Boşluk (docs/adr/0708): a space written with it is a void — a shaft drawn by its corners — whose area the
//! quantities sum apart from the rooms'; the preview draws its cross.

use crate::common::Bench;
use kentos_commands::{DrawingUnit, Entity, SpacesQuantities, Workspace};
use kentos_native_application::{ExecutionContext, spaces_quantities};

#[test]
fn a_shaft_drawn_with_bosluk_is_a_void_crossed_and_summed_apart() {
    let mut b = Bench::new("space");
    let mut settings = b.doc.settings().clone();
    settings.workspace = Some(Workspace::Cad);
    settings.srid = 0;
    settings.drawing_unit = Some(DrawingUnit::M);
    b.doc.set_settings(settings);
    b.start("space");
    assert!(b.run(|s, cx| s.configure("Ç", "draw", cx)));
    assert!(b.run(|s, cx| s.configure("V", "yes", cx)));
    assert!(b.run(|s, cx| s.configure("A", "Şaft", cx)));
    for (x, y) in [(0., 0.), (0.8, 0.), (0.8, 0.6)] {
        b.click(x, y);
    }
    b.move_to(0., 0.6);
    let preview = b.session.preview(&Default::default()).expect("the plan drawn");
    let crossed = preview.strokes.iter().filter(|s| s.pts.len() == 2).count();
    assert!(crossed >= 2, "the void's cross in the preview: {crossed}");
    b.click(0., 0.6);
    b.confirm();
    let space = b
        .doc
        .entities()
        .find_map(|e| match e {
            Entity::Space(s) => Some(s.space.clone()),
            _ => None,
        })
        .expect("the shaft");
    assert!(space.void, "written a void");
    assert_eq!(space.name.as_deref(), Some("Şaft"));
    let CommandResult::Completed { output, .. } =
        spaces_quantities::plan(&ExecutionContext::new(&mut b.doc), &SpacesQuantities::default())
    else {
        panic!("quantities");
    };
    assert_eq!(output.totals.space_area, 0.0, "a void is not a room's area");
    assert!((output.totals.void_area - 0.48).abs() < 1e-9, "{:?}", output.totals);
}

use kentos_commands::CommandResult;
