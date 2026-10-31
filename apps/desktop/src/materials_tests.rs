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

#[test]
fn kentos_library_materials_are_taken_in_one_undo_step_the_projects_kept() {
    let mut app = app();
    let before = model(&app).to_snapshot_v2();
    let _ = app.update(Message::Run("arch.materials"));
    let _ = app.update(Message::Materials(Event::Tab(Tab::Library)));
    let w = app.materials.as_ref().unwrap();
    assert!(matches!(w.entries[0].place, crate::material_library::Place::Kentos("kentos-temel")));
    // KentOS's 21, the project's Tuğla and Betonarme not ticked (they differ): 19 new ones previewed.
    assert_eq!(w.picked.iter().filter(|p| **p).count(), 19);
    let Some(Ok(plan)) = &w.take_preview else { panic!("{:?}", w.take_preview) };
    assert_eq!(plan.report.taken.len(), 19);
    assert_eq!(model(&app).to_snapshot_v2(), before, "the preview wrote nothing");
    let _ = app.update(Message::Materials(Event::Take));
    assert_eq!(model(&app).materials().len(), 21);
    assert_eq!(model(&app).materials()[0].id, "tugla", "the project's stays first, as it was");
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(model(&app).materials().len(), 2);
}

#[test]
fn replacing_from_the_library_counts_what_follows_and_export_writes_a_file_read_back() {
    let mut app = app();
    let _ = app.update(Message::Run("arch.materials"));
    let _ = app.update(Message::Materials(Event::Tab(Tab::Library)));
    let _ = app.update(Message::Materials(Event::PickAll(true)));
    let _ = app.update(Message::Materials(Event::Conflict(Conflict::Replace)));
    let Some(Ok(plan)) = &app.materials.as_ref().unwrap().take_preview else { panic!() };
    assert_eq!(plan.report.replaced, ["betonarme", "tugla"]);
    assert_eq!((plan.report.walls.len(), plan.report.slabs.len(), plan.report.locked.len()), (3, 0, 1), "the slabs keep their text: the name is the same");
    let file = std::env::temp_dir().join(format!("kentos-malzeme-yaz-{}.kmalzeme", std::process::id()));
    app.export_materials(&file);
    let back = crate::material_library::read(&file).expect("the written library reads");
    assert_eq!(back.materials, model(&app).materials());
    assert_eq!(back.library, file.file_stem().unwrap().to_string_lossy());
    let _ = std::fs::remove_file(&file);
}

#[test]
fn a_wall_style_component_is_bound_in_duvar_stilleri_and_saved_in_one_step() {
    let mut app = app();
    let _ = app.update(Message::Run("arch.wallStyles"));
    let _ = app.update(Message::WallStyles(crate::wall_styles::Event::Material(2, Some("betonarme".into()))));
    let _ = app.update(Message::WallStyles(crate::wall_styles::Event::Save));
    let style = &model(&app).settings().wall_styles[0];
    let c = &style.components[2];
    assert_eq!((c.material_id.as_deref(), c.material.as_deref()), (Some("betonarme"), Some("Betonarme")));
    assert_eq!(c.hatch, model(&app).materials()[1].hatch, "the material's hatch, read-only in the window");
    // Left again: its text and look stay, unbound.
    let _ = app.update(Message::Run("arch.wallStyles"));
    let _ = app.update(Message::WallStyles(crate::wall_styles::Event::Material(2, None)));
    let _ = app.update(Message::WallStyles(crate::wall_styles::Event::Save));
    let c = &model(&app).settings().wall_styles[0].components[2];
    assert_eq!((c.material_id.as_deref(), c.material.as_deref()), (None, Some("Betonarme")));
    let _ = app.update(Message::Run("edit.undo"));
    let c = &model(&app).settings().wall_styles[0].components[2];
    assert_eq!(c.material_id.as_deref(), Some("betonarme"), "one undo step a save");
}

#[test]
fn a_slab_is_bound_to_a_space_of_its_level_from_its_sinir_row() {
    let mut app = app_with_drawing();
    crate::material_library_scenes::slab_row(&mut app);
    let m = model(&app);
    let (slot, slab) = m
        .entities()
        .find_map(|e| match e {
            Entity::Slab(s) => Some((kentos_domain::Slot(e.base().id), s.slab.clone())),
            _ => None,
        })
        .unwrap();
    let b = slab.bound.as_ref().expect("bound");
    assert_eq!(b.source, kentos_commands::SlabSourceKind::Space);
    let spaces = crate::properties::wall_parts::space_choices(m, &slab);
    // Both spaces of the level, the salon (where the slab now is) first.
    let words: Vec<&str> = spaces.iter().map(|(_, w)| w.as_str()).collect();
    assert_eq!(words, ["101 Salon", "102 Mutfak"]);
    assert_eq!(crate::properties::wall_parts::bound_text(&slab, &spaces), "Mahale bağlı · 101 Salon");
    let choices = crate::properties::wall_parts::bound_choices(&slab, &spaces);
    let chosen: Vec<&str> = choices.iter().filter(|c| c.1).map(|c| c.0.as_str()).collect();
    assert_eq!(chosen, ["Mahale bağlı · 101 Salon"]);
    // One undo: bound to the kitchen's walls again.
    let _ = app.update(Message::Run("edit.undo"));
    let Some(Entity::Slab(s)) = model(&app).get(slot) else { panic!() };
    assert_eq!(s.slab.bound.as_ref().unwrap().source, kentos_commands::SlabSourceKind::Walls);
}
