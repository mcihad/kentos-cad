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
    assert!(matches!(key(Named::Enter, Modifiers::ALT), Some(iced::widget::text_editor::Binding::Enter)), "AutoCAD's Alt+Enter");
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
