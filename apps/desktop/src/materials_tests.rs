use super::*;
use crate::files_testing::{app_with_drawing, make_cad};

const CASES: &str = include_str!("../../../fixtures/commands/v1/cad.materials.edit.json");

/// The shared cases' drawing: a style whose core is bound to Tuğla, its walls, a wall of its own, slabs on Betonarme.
fn app() -> App {
    let mut app = app_with_drawing();
    let file: serde_json::Value = serde_json::from_str(CASES).unwrap();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(file["setup"].clone()).unwrap();
    let doc = crate::document::Document::from_v2(snapshot, None).unwrap();
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    make_cad(&mut app);
    app
}

fn model(app: &App) -> &kentos_domain::Document {
    &app.document.as_ref().unwrap().model
}

#[test]
fn the_window_counts_what_is_bound_and_previews_a_change_before_one_undo_step() {
    let mut app = app();
    let before = model(&app).to_snapshot_v2();
    let _ = app.update(Message::Run("arch.materials"));
    assert_eq!(app.dialog, Some(Dialog::Materials));
    let w = app.materials.as_ref().unwrap();
    let (m, u) = &w.list[0];
    assert_eq!(m.id, "tugla");
    assert_eq!((u.styles.len(), u.walls, u.slabs), (1, 4, 0));
    assert_eq!(w.list[1].1.slabs, 2);
    let _ = app.update(Message::Materials(Event::Pattern("ANSI32".into())));
    let w = app.materials.as_ref().unwrap();
    let Some(Ok(done)) = &w.preview else {
        panic!("{:?}", w.preview);
    };
    // The style, its two walls and the wall of its own follow; the locked one is counted, left.
    assert_eq!((done.report.styles.len(), done.report.walls.len(), done.report.locked.len()), (1, 3, 1));
    assert_eq!(model(&app).to_snapshot_v2(), before, "the preview wrote nothing");
    let _ = app.update(Message::Materials(Event::Apply));
    let style = &model(&app).settings().wall_styles[0];
    assert_eq!(style.components[1].hatch.as_ref().unwrap().pattern, "ANSI32");
    // One undo gives the material and its walls back.
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(model(&app).to_snapshot_v2().settings, before.settings);
}

#[test]
fn a_slab_is_bound_from_its_properties_and_metraj_groups_by_material() {
    let mut app = app();
    let _ = app.update(Message::Properties(crate::properties::Event::Material(
        kentos_commands::MaterialsOperation::Assign {
            material: Some("betonarme".into()),
            targets: vec![kentos_commands::MaterialTarget::Slab {
                uid: "0192f5a1-7850-7000-8000-000000000006".into(),
            }],
        },
    )));
    let Some(Entity::Slab(s)) = model(&app).get(kentos_domain::Slot(6)) else {
        panic!("the slab");
    };
    assert_eq!(s.slab.material_id.as_deref(), Some("betonarme"));
    let q = match kentos_native_application::spaces_quantities::plan_in(model(&app), &Default::default()) {
        kentos_commands::CommandResult::Completed { output, .. } => output,
        other => panic!("{other:?}"),
    };
    let names: Vec<(&str, u32, u32)> = q.materials.iter().map(|m| (m.name.as_str(), m.walls, m.slabs)).collect();
    assert_eq!(names[0], ("Tuğla", 4, 0));
    assert_eq!(names[1], ("Betonarme", 0, 3));
}
