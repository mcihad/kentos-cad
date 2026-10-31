use super::*;
use crate::files_testing::{app_with_drawing, make_cad};

const CASES: &str = include_str!("../../../fixtures/commands/v1/cad.modules.edit.json");

/// The shared cases' bathroom (objects 1–9), a corridor, a line and two more walls.
fn app() -> App {
    let mut app = app_with_drawing();
    let file: serde_json::Value = serde_json::from_str(CASES).unwrap();
    let snapshot: kentos_commands::DocumentSnapshotV2 =
        serde_json::from_value(file["setup"].clone()).unwrap();
    let doc = crate::document::Document::from_v2(snapshot, None).unwrap();
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    make_cad(&mut app);
    app
}

fn select_bathroom(app: &mut App) {
    app.selection.set((1..=9).map(kentos_domain::Slot));
}

fn modules(app: &App) -> Vec<kentos_commands::ArchModule> {
    app.document.as_ref().unwrap().model.arch_modules().to_vec()
}

fn door_width(app: &App, slot: u32) -> f64 {
    match app
        .document
        .as_ref()
        .unwrap()
        .model
        .get(kentos_domain::Slot(slot))
    {
        Some(Entity::Opening(o)) => o.opening.width,
        other => panic!("{other:?}"),
    }
}

#[test]
fn the_window_defines_the_selection_and_closing_writes_nothing() {
    let mut app = app();
    let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
    let _ = app.update(Message::Run("arch.modules"));
    assert_eq!(app.dialog, Some(Dialog::RoomModules));
    let _ = app.update(Message::RoomModules(Event::Close));
    assert_eq!(
        app.document
            .as_ref()
            .unwrap()
            .model
            .to_snapshot_v2()
            .settings,
        before.settings
    );
    select_bathroom(&mut app);
    let _ = app.update(Message::Run("arch.modules"));
    let _ = app.update(Message::RoomModules(Event::Name("Banyo".into())));
    let _ = app.update(Message::RoomModules(Event::Define));
    let m = modules(&app);
    assert_eq!(m.len(), 1);
    assert_eq!(m[0].members.len(), 9);
    assert_eq!(m[0].instances[0].level.as_deref(), Some("zemin"));
}

#[test]
fn an_update_from_the_window_reaches_the_linked_instances_in_one_step() {
    let mut app = app();
    select_bathroom(&mut app);
    let _ = app.update(Message::Run("arch.modules"));
    let _ = app.update(Message::RoomModules(Event::Name("Banyo".into())));
    let _ = app.update(Message::RoomModules(Event::Define));
    let _ = app.update(Message::RoomModules(Event::Close));
    let doc = &mut app.document.as_mut().unwrap().model;
    for (x, mirror) in [(10.0, false), (20.0, true)] {
        let input: kentos_commands::ModulesEdit = serde_json::from_value(serde_json::json!({
            "operation": {"kind": "place", "module": "Banyo", "at": {"x": x, "y": 0}, "mirror": mirror, "level": "zemin"}
        }))
        .unwrap();
        assert!(matches!(
            kentos_native_application::modules_edit::execute(
                &mut kentos_native_application::ExecutionContext::new(doc),
                input
            ),
            CommandResult::Completed { .. }
        ));
        let _ = doc;
    }
    let doc = &mut app.document.as_mut().unwrap().model;
    let door: kentos_commands::OpeningsEdit = serde_json::from_value(serde_json::json!({
        "uids": ["0192f5a1-7750-7000-8000-000000000005"], "operation": {"kind": "set", "width": 0.9}
    }))
    .unwrap();
    let _ = kentos_native_application::openings_edit::execute(
        &mut kentos_native_application::ExecutionContext::new(doc),
        door,
    );
    app.selection.set([kentos_domain::Slot(5)]);
    let _ = app.update(Message::Run("arch.modules"));
    let w = app.room_modules.as_ref().unwrap();
    let Some(Ok(plan)) = &w.preview else {
        panic!("{:?}", w.preview)
    };
    assert_eq!(plan.report.updated, ["banyo-1", "banyo-2", "banyo-3"]);
    let _ = app.update(Message::RoomModules(Event::Update));
    assert!((door_width(&app, 19) - 0.9).abs() < 1e-12);
    assert!((door_width(&app, 28) - 0.9).abs() < 1e-12);
    let _ = app.update(Message::Run("edit.undo"));
    assert!((door_width(&app, 19) - 0.8).abs() < 1e-12);
    assert_eq!(modules(&app)[0].version, 1);
}

#[test]
fn an_object_is_made_a_local_exception_from_the_selection() {
    let mut app = app();
    select_bathroom(&mut app);
    let _ = app.update(Message::Run("arch.modules"));
    let _ = app.update(Message::RoomModules(Event::Define));
    let _ = app.update(Message::RoomModules(Event::Close));
    app.selection.set([kentos_domain::Slot(5)]);
    let _ = app.update(Message::Run("arch.moduleExcept"));
    assert_eq!(
        modules(&app)[0].instances[0].exceptions,
        ["0192f5a1-7750-7000-8000-000000000005"]
    );
    let _ = app.update(Message::Run("arch.moduleRestore"));
    assert!(modules(&app)[0].instances[0].exceptions.is_empty());
}

// ── docs/adr/0779: members added and taken out, drift, repeated levels ─────────────────────────────────────────────

const FLATS: &str = include_str!("../../../fixtures/commands/v1/cad.modules.edit.flats.json");

fn flat_uid(n: u32) -> String {
    format!("0192f5a1-7790-7000-8000-{n:012x}")
}

/// The shared cases' two-room flat defined as a module (objects 1–13), a second fixture (14) and door (15) beside it.
fn flat_app() -> App {
    let mut app = app_with_drawing();
    let file: serde_json::Value = serde_json::from_str(FLATS).unwrap();
    let snapshot: kentos_commands::DocumentSnapshotV2 =
        serde_json::from_value(file["setup"].clone()).unwrap();
    let doc = crate::document::Document::from_v2(snapshot, None).unwrap();
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    make_cad(&mut app);
    app.selection.set((1..=13).map(kentos_domain::Slot));
    let _ = app.update(Message::Run("arch.modules"));
    let _ = app.update(Message::RoomModules(Event::Name("Daire".into())));
    let _ = app.update(Message::RoomModules(Event::Define));
    let _ = app.update(Message::RoomModules(Event::Close));
    app
}

#[test]
fn the_selections_free_objects_join_the_instance_they_are_selected_with() {
    let mut app = flat_app();
    // The bed (14) with one of the flat's walls: Örneğe ekle takes the bed into the definition.
    app.selection
        .set([kentos_domain::Slot(1), kentos_domain::Slot(14)]);
    let _ = app.update(Message::Run("arch.moduleAddMembers"));
    let m = modules(&app);
    assert_eq!(m[0].version, 2);
    assert_eq!(m[0].members.len(), 14);
    assert!(m[0].member(&flat_uid(14)).is_some());
    // Modülden çıkar on it gives it back to the drawing as an ordinary object.
    app.selection.set([kentos_domain::Slot(14)]);
    let _ = app.update(Message::Run("arch.moduleRemoveMembers"));
    let m = modules(&app);
    assert_eq!(m[0].version, 3);
    assert!(m[0].member(&flat_uid(14)).is_none());
    assert!(
        app.document
            .as_ref()
            .unwrap()
            .model
            .get(kentos_domain::Slot(14))
            .is_some()
    );
}

#[test]
fn the_window_repeats_the_ground_floor_with_its_prefixes_in_one_step() {
    let mut app = flat_app();
    let _ = app.update(Message::Run("arch.modules"));
    let w = app.room_modules.as_ref().unwrap();
    assert_eq!(w.repeat_from.as_deref(), Some("zemin"));
    assert_eq!(
        w.repeat_to
            .iter()
            .map(|t| t.level.as_str())
            .collect::<Vec<_>>(),
        ["kat1", "kat2", "kat3"]
    );
    let before = app.document.as_ref().unwrap().model.entities().count();
    let _ = app.update(Message::RoomModules(Event::RepeatPrefix(0, "1-".into())));
    let _ = app.update(Message::RoomModules(Event::RepeatChoose(1, true)));
    let Some(Ok(plan)) = &app.room_modules.as_ref().unwrap().repeat_preview else {
        panic!("{:?}", app.room_modules.as_ref().unwrap().repeat_preview)
    };
    assert_eq!(plan.placed.len(), 2);
    assert_eq!(
        app.document.as_ref().unwrap().model.entities().count(),
        before,
        "the plan writes nothing"
    );
    let _ = app.update(Message::RoomModules(Event::Repeat));
    assert_eq!(modules(&app)[0].instances.len(), 3);
    assert_eq!(
        modules(&app)[0].instances[1].numbering.prefix.as_deref(),
        Some("1-")
    );
    assert_eq!(
        app.document.as_ref().unwrap().model.entities().count(),
        before + 26
    );
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(modules(&app)[0].instances.len(), 1);
}

#[test]
fn an_object_moved_by_hand_is_said_drifted_and_restore_gives_it_back() {
    let mut app = flat_app();
    let doc = &mut app.document.as_mut().unwrap().model;
    let mut bed = doc.get(kentos_domain::Slot(10)).unwrap().clone();
    if let Entity::Insert(b) = &mut bed {
        b.p.x += 0.5;
    }
    doc.update(kentos_domain::Slot(10), bed);
    let _ = app.update(Message::Run("arch.modules"));
    let w = app.room_modules.as_ref().unwrap();
    assert_eq!(w.rows[0].instances[0].drifted, [flat_uid(10)]);
    let _ = app.update(Message::RoomModules(Event::Close));
    app.selection.set([kentos_domain::Slot(10)]);
    let _ = app.update(Message::Run("arch.moduleRestore"));
    let Some(Entity::Insert(b)) = app
        .document
        .as_ref()
        .unwrap()
        .model
        .get(kentos_domain::Slot(10))
    else {
        panic!()
    };
    assert!((b.p.x - 4.5).abs() < 1e-12);
}
