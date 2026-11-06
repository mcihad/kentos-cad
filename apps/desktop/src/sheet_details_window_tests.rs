//! Detay paftaları (sheet_details_window.rs, docs/adr/1112): the window previews the plan of `cad.sheets.details`
//! without writing, builds the whole set in one step that one undo takes back, Esc (Kapat) writes nothing; a table's
//! parts come as details of their own; an exact ratio and a frozen layer reach the viewports.

use crate::app::{App, Dialog, Message};
use crate::sheet_details_window::{Event as D, ratios};

/// The details' drawing of `cad.sheets.details`'s shared cases, opened.
fn app() -> App {
    let (mut app, _) = App::boot(None);
    let fx: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/cad.sheets.details.json")).expect("cases");
    let model = kentos_domain::Document::from_snapshot(serde_json::from_value(fx["setup"].clone()).expect("a drawing"))
        .expect("opens");
    let doc = crate::document::Document::from_v2(model.to_snapshot_v2(), None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn send(app: &mut App, e: D) {
    let _ = app.update(Message::SheetDetails(e));
}

fn select(app: &mut App, slot: u32) {
    app.selection.set(vec![kentos_domain::Slot(slot)]);
}

fn book(app: &App) -> serde_json::Value {
    let doc = app.document.as_ref().expect("a drawing");
    let sheets = doc.model.sheets().expect("a book");
    serde_json::to_value(kentos_sheet::project::read_project_sheets(sheets).expect("reads").0).expect("json")
}

#[test]
fn the_window_previews_builds_the_set_in_one_step_and_one_undo_takes_it_back() {
    let mut app = app();
    select(&mut app, 1);
    let _ = app.update(Message::Run("sheet.details"));
    assert_eq!(app.dialog, Some(Dialog::SheetDetails));
    let one = ratios().iter().position(|r| *r == (1.0, 1.0)).expect("1:1");
    let five = ratios().iter().position(|r| *r == (1.0, 5.0)).expect("1:5");
    send(&mut app, D::Ratio(0, one));
    send(&mut app, D::Name(0, "Bulon".into()));
    select(&mut app, 3);
    send(&mut app, D::FromSelection);
    send(&mut app, D::Ratio(1, five));
    // Ölçü frozen in the second detail's viewport only.
    let olcu = {
        let doc = app.document.as_ref().expect("a drawing");
        let mut ids = Vec::new();
        fn walk(nodes: &[kentos_commands::LayerNode], out: &mut Vec<String>) {
            for n in nodes {
                if n.kind == kentos_commands::LayerNodeType::Layer {
                    out.push(n.id.clone());
                }
                walk(&n.children, out);
            }
        }
        walk(doc.model.layers().nodes(), &mut ids);
        ids.iter().position(|l| l == "olcu").expect("Ölçü")
    };
    send(&mut app, D::Freeze(olcu));
    // The table's parts, each a detail of its own.
    select(&mut app, 4);
    send(&mut app, D::FromTable);
    let set = app.sheet_details.as_ref().expect("open");
    let parts = set.details.len() - 2;
    assert!(parts >= 2, "the parts list is broken in parts: {parts}");
    let planned = set.result.clone().expect("previewed").expect("plans");
    assert_eq!(planned.details.len(), 2 + parts);
    let first = &planned.details[0];
    assert_eq!(first["denominator"], 1.0, "{first}");
    // The plate's line is 0,37 m long: with the 5% margin each side, at 1:5 its viewport is 0,37 × 1,1 × 1e6 / 5 µm
    // on the paper (docs/adr/1090 §2), rounded to the µm; from the model's length, not from KentOS's answer.
    let plate = &planned.details[1];
    let want = (0.37 * (1.0 + 2.0 * crate::sheet_details_window::MARGIN) * 1e6 / 5.0).round();
    assert_eq!(plate["frame"]["width"].as_f64(), Some(want), "{plate}");
    // The preview wrote nothing.
    assert!(app.document.as_ref().expect("a drawing").model.sheets().is_none());
    let can_undo = app.document.as_ref().expect("a drawing").model.can_undo();
    assert!(!can_undo);
    send(&mut app, D::Build);
    assert!(app.dialog.is_none(), "the window closes");
    let built = book(&app);
    let sheets = built["sheets"].as_array().expect("sheets");
    assert_eq!(sheets.len(), planned.made.len(), "{built}");
    let items: Vec<&serde_json::Value> = sheets.iter().flat_map(|s| s["items"].as_array().expect("items")).collect();
    let levha = items.iter().find(|i| i["id"] == plate["item"]).expect("the plate's viewport");
    assert_eq!(levha["kind"]["layerOverrides"], serde_json::json!([{ "layer": "olcu", "frozen": true }]), "{levha}");
    assert_eq!(levha["kind"]["viewLocked"], true, "{levha}");
    let _ = app.update(Message::Run("edit.undo"));
    let after = app.document.as_ref().expect("a drawing").model.sheets().map(|s| {
        kentos_sheet::project::read_project_sheets(s).map(|(b, _)| b.sheets.len()).unwrap_or(0)
    });
    assert!(after.unwrap_or(0) == 0, "one undo takes the set back: {after:?}");
}

#[test]
fn closing_writes_nothing_and_an_empty_selection_is_said() {
    let mut app = app();
    let _ = app.update(Message::Run("sheet.details"));
    assert_eq!(app.dialog, Some(Dialog::SheetDetails));
    send(&mut app, D::FromSelection);
    assert!(app.sheet_details.as_ref().is_some_and(|s| s.details.is_empty()));
    assert!(app.log.lines().last().is_some_and(|l| l.text.contains("seçili nesne yok")), "said");
    select(&mut app, 2);
    send(&mut app, D::FromSelection);
    send(&mut app, D::Close);
    assert!(app.dialog.is_none());
    assert!(app.document.as_ref().expect("a drawing").model.sheets().is_none(), "Kapat wrote nothing");
}
