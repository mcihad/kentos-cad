//! Pictures of CAD-ADV-13's tails (docs/adr/1131) for the owner, in tools_screens' harness: a table cell broken over
//! two lines in Tabloyu düzenle, Veri çıkarımı over the dynamic Sıra rows (what the visibility state hides, said), and
//! the preview of a large drawing running on its own thread with Durdur.
//!
//! ```text
//! KENTOS_SHOTS_ONLY=tablo-satir-sonu,veri-cikarimi-gizli,veri-cikarimi-buyuk cargo test -p kentos-desktop --lib tools_screens::screens -- --ignored --nocapture
//! ```

use serde_json::json;

use crate::app::{App, Message};
use crate::tables::editor::Event as Edit;
use crate::tools_scenes::{E, N, Objects, open_edited};
use crate::tools_screens::Scene;

/// Tabloyu düzenle: Zemin's cell written “Zemin” Shift+Enter “kat”, then chosen again, its two lines in the bar.
fn line_break(app: &mut App) {
    use iced::widget::text_editor::{Action, Edit as Typed};
    let mut o = Objects::new();
    let id = o.push(
        "cizim",
        json!({ "kind": "table", "p": { "x": E, "y": N + 4.0 }, "rotation": 0.0, "height": 0.25,
                "rows": [0.6, 0.6, 0.6], "columns": [3.0, 2.0], "cells": [["Kat", "Alan"], ["Zemin", "120"], ["Bodrum", "80"]],
                "header": true, "font": "arimo" }),
    );
    open_edited(app, o, |_| {});
    let slot = kentos_domain::Slot(id);
    let _ = app.open_table_editor(slot);
    let send = |app: &mut App, e: Edit| {
        let _ = app.update(Message::TableEditor(e));
    };
    send(app, Edit::Cell(1, 0));
    send(app, Edit::Release);
    send(app, Edit::Write);
    send(app, Edit::Bar(Action::Edit(Typed::Enter)));
    for c in "kat".chars() {
        send(app, Edit::Bar(Action::Edit(Typed::Insert(c))));
    }
    send(app, Edit::Commit);
    send(app, Edit::Cell(1, 0));
    send(app, Edit::Release);
    send(app, Edit::Write);
}

/// Veri çıkarımı over `Kat 3` of the shared cases: two Sıra at Boy 2 m, one Boş, one on a hidden layer.
fn hidden(app: &mut App) {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.extract.run.json")).expect("cases");
    let setup = cases["cases"][7]["setup"].clone();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(setup).expect("a drawing");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    crate::files_testing::make_cad(app);
    app.extract = None;
    let _ = app.update(Message::Run("table.extract"));
    let _ = app.update(Message::Extract(crate::tables::extract::Event::Name("Sıralar".into())));
}

/// A drawing of 60 000 lines: the preview runs on its own thread, how far it is and Durdur shown.
fn large(app: &mut App) {
    let mut o = Objects::new();
    for k in 0..60_000usize {
        let (x, y) = (E + (k % 300) as f64 * 3.0, N + (k / 300) as f64 * 2.0);
        o.push("cizim", json!({ "kind": "line", "a": { "x": x, "y": y }, "b": { "x": x + 2.5, "y": y } }));
    }
    open_edited(app, o, |_| {});
    app.extract = None;
    let _ = app.update(Message::Run("table.extract"));
    let _ = app.update(Message::Extract(crate::tables::extract::Event::Kind("line", true)));
    // Let the thread get on before the picture; it is a picture of a preview still running.
    std::thread::sleep(std::time::Duration::from_millis(15));
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![("tablo-satir-sonu", line_break), ("veri-cikarimi-gizli", hidden), ("veri-cikarimi-buyuk", large)]
}
