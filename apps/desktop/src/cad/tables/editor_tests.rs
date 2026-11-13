//! Tabloyu düzenle's line break (docs/adr/1131 §1): Shift+Enter in the bar breaks the cell's line, Enter writes it;
//! the cell keeps its `\n` through Kaydet (one undo), and the table's layout gives it two lines.

use iced::widget::text_editor::{Action, Edit as Typed};
use serde_json::json;

use super::Event as Edit;
use crate::app::{App, Message};
use crate::tools_scenes::{E, N, Objects, open_edited};

/// A drawing of one 2 × 2 table, its heading row and one row: `Kat` / `Alan`, `Zemin` / `120`.
pub(crate) fn table_app() -> (App, kentos_domain::Slot) {
    let (mut app, _) = App::boot(None);
    let mut o = Objects::new();
    let id = o.push(
        "cizim",
        json!({ "kind": "table", "p": { "x": E, "y": N + 4.0 }, "rotation": 0.0, "height": 0.25,
                "rows": [0.6, 0.6], "columns": [3.0, 2.0], "cells": [["Kat", "Alan"], ["Zemin", "120"]],
                "header": true, "font": "arimo" }),
    );
    open_edited(&mut app, o, |_| {});
    (app, kentos_domain::Slot(id))
}

fn send(app: &mut App, e: Edit) {
    let _ = app.update(Message::TableEditor(e));
}

fn cells(app: &App, slot: kentos_domain::Slot) -> Vec<Vec<String>> {
    match app.document.as_ref().and_then(|d| d.model.get(slot)) {
        Some(kentos_commands::Entity::Table(t)) => t.cells.clone(),
        other => panic!("a table: {other:?}"),
    }
}

/// The bar's keys as `text_editor` binds them (`bar_keys`): Shift+Enter a line break, Enter a submit.
fn key(named: iced::keyboard::key::Named, modifiers: iced::keyboard::Modifiers) -> Option<iced::widget::text_editor::Binding<Message>> {
    use iced::keyboard::Key;
    super::bar_keys(iced::widget::text_editor::KeyPress {
        key: Key::Named(named),
        modified_key: Key::Named(named),
        physical_key: iced::keyboard::key::Physical::Unidentified(iced::keyboard::key::NativeCode::Unidentified),
        modifiers,
        text: None,
        status: iced::widget::text_editor::Status::Focused { is_hovered: false },
    })
}

#[test]
fn shift_enter_breaks_the_cells_line_and_kaydet_keeps_it() {
    use iced::keyboard::key::Named;
    let (mut app, slot) = table_app();
    let _ = app.open_table_editor(slot);
    send(&mut app, Edit::Cell(1, 0));
    send(&mut app, Edit::Release);
    send(&mut app, Edit::Write);
    // Typed in the bar: “Zemin”, Shift+Enter, “kat”.
    use iced::keyboard::Modifiers;
    assert!(matches!(key(Named::Enter, Modifiers::SHIFT), Some(iced::widget::text_editor::Binding::Enter)));
    assert!(matches!(key(Named::Enter, Modifiers::ALT), Some(iced::widget::text_editor::Binding::Enter)), "Alt+Enter");
    assert!(matches!(key(Named::Enter, Modifiers::default()), Some(iced::widget::text_editor::Binding::Custom(_))));
    assert!(key(Named::Tab, Modifiers::default()).is_none(), "Tab is the window's");
    send(&mut app, Edit::Bar(Action::Edit(Typed::Enter)));
    for c in "kat".chars() {
        send(&mut app, Edit::Bar(Action::Edit(Typed::Insert(c))));
    }
    send(&mut app, Edit::Commit);
    send(&mut app, Edit::Save);
    assert_eq!(cells(&app, slot)[1][0], "Zemin\nkat", "the cell keeps its line break");
    // The table's layout gives the cell two lines (docs/adr/1030 §7).
    let doc = app.document.as_ref().expect("a drawing");
    let shape = kentos_native_application::geometry::shape(doc.model.get(slot).expect("the table"));
    let t = kentos_geometry_core::geom::table::table_geom(&shape).expect("a table's layout");
    let laid = t.layout(kentos_geometry_core::text::Font::default());
    let lines = laid.cells.iter().filter(|c| c.row == 1 && c.col == 0).count();
    assert_eq!(lines, 2, "two lines laid out");
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(cells(&app, slot)[1][0], "Zemin", "one undo takes the edit back");
}

/// The shared cases' drawing (cad.schedules.edit) with its four openings and the door and window schedule (object
/// 17): rows B1, K01, P01, K01 and the Kapı 80; column 3 the width.
fn schedule_app() -> (App, kentos_domain::Slot) {
    use kentos_native_application::ExecutionContext;
    let mut app = crate::files_testing::app_with_drawing();
    let file: serde_json::Value =
        serde_json::from_str(include_str!("../../../../../fixtures/commands/v1/cad.schedules.edit.json")).unwrap();
    let snapshot: kentos_commands::DocumentSnapshotV1 = serde_json::from_value(file["setup"].clone()).unwrap();
    let doc = crate::document::Document::new(snapshot, None).unwrap();
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    let model = &mut app.document.as_mut().unwrap().model;
    let wall = |n: u32| model.uid(kentos_domain::Slot(n)).unwrap().to_string();
    let (w1, w2) = (wall(1), wall(2));
    let openings: kentos_commands::OpeningsCreate = serde_json::from_value(json!({"layerId": "kapi", "openings": [
        {"host": w1, "at": 2.0, "type": "Kapı 90", "number": "K01"},
        {"host": w1, "at": 6.0, "type": "Pencere 120", "number": "P01"},
        {"host": w2, "at": 3.0, "type": "Kapı 90", "number": "K01"},
        {"host": w2, "at": 8.0, "type": "Kapı 80"}]}))
    .unwrap();
    assert!(matches!(
        kentos_native_application::openings_create::execute(&mut ExecutionContext::new(model), openings),
        kentos_commands::CommandResult::Completed { .. }
    ));
    let create: kentos_commands::SchedulesCreate =
        serde_json::from_value(json!({"layerId": "cizelge", "at": {"x": 0.0, "y": -10.0}, "of": "openings"})).unwrap();
    let kentos_commands::CommandResult::Completed { output, .. } =
        kentos_native_application::schedules_create::execute(&mut ExecutionContext::new(model), create)
    else {
        panic!("a schedule");
    };
    (app, kentos_domain::Slot(output.id))
}

fn width_of(app: &App, number: &str, form: &str) -> f64 {
    let doc = &app.document.as_ref().unwrap().model;
    doc.entities()
        .find_map(|e| match e {
            kentos_commands::Entity::Opening(o)
                if o.opening.number.as_deref().unwrap_or("") == number
                    && format!("{:?}", o.opening.form).to_lowercase().contains(form) =>
            {
                Some(o.opening.width)
            }
            _ => None,
        })
        .expect("the opening")
}

/// UR-058: a schedule's width cell written in Tabloyu düzenle goes into the draft (no refusal), a value the objects
/// refuse is said at once and leaves the draft, and Kaydet writes the window's and the niche's widths in one undo step.
#[test]
fn a_schedules_cell_is_drafted_and_kaydet_writes_its_objects() {
    let (mut app, slot) = schedule_app();
    let _ = app.open_table_editor(slot);
    let draft = |app: &App| app.table_editor.as_ref().unwrap().draft.cells.clone();
    let problem = |app: &App| app.table_editor.as_ref().unwrap().problem.clone();
    assert_eq!(draft(&app)[3][3], "1.200");
    send(&mut app, Edit::Cell(3, 3));
    send(&mut app, Edit::Words("1.1".into()));
    send(&mut app, Edit::Commit);
    assert_eq!(problem(&app), None);
    assert_eq!(draft(&app)[3][3], "1.100");
    assert!((width_of(&app, "P01", "window") - 1.2).abs() < 1e-12, "nothing written before Kaydet");
    send(&mut app, Edit::Cell(1, 3));
    send(&mut app, Edit::Words("0,8".into()));
    send(&mut app, Edit::Commit);
    assert_eq!(problem(&app), None);
    assert_eq!(draft(&app)[1][3], "0.800");
    // A width that is no number: said, the draft as it was.
    send(&mut app, Edit::Cell(2, 3));
    send(&mut app, Edit::Words("geniş".into()));
    send(&mut app, Edit::Commit);
    assert!(problem(&app).is_some());
    assert_eq!(draft(&app)[2][3], "0.900");
    send(&mut app, Edit::Save);
    assert!(app.table_editor.is_none(), "{:?}", app.table_editor.as_ref().and_then(|e| e.problem.clone()));
    assert!((width_of(&app, "P01", "window") - 1.1).abs() < 1e-12);
    assert_eq!(cells(&app, slot)[3][3], "1.100");
    assert_eq!(cells(&app, slot)[1][3], "0.800");
    let _ = app.update(Message::Run("edit.undo"));
    assert!((width_of(&app, "P01", "window") - 1.2).abs() < 1e-12, "one undo takes both back");
    assert_eq!(cells(&app, slot)[1][3], "0.600");
}

/// UR-058's picture: the window's width written in the draft, `.run/gui-evidence/kullanim-058/`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn schedule_screens() {
    crate::files_testing::usage_shots("058", "cizelge-hucresi-taslakta", || {
        let (mut app, slot) = schedule_app();
        let _ = app.open_table_editor(slot);
        send(&mut app, Edit::Cell(3, 3));
        send(&mut app, Edit::Words("1.1".into()));
        send(&mut app, Edit::Commit);
        app
    });
}
