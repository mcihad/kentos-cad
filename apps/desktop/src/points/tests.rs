//! Noktalar's rules and the tab over `fixtures/interaction/v1/point-editor.kcad`
//! (docs/adr/0153 §2): the cases of the web's `ui/bottom/PointTable.test.ts`,
//! then the tab as the app runs it, and pictures for the owner.

use iced::Task;
use kentos_domain::Slot;

use super::edit::{Draft, EditColumn, write_cell, write_draft};
use super::{COLUMNS, Event, Walk, click_pick, focus_field, next_cell, next_sort, row_of};
use crate::app::{App, Message};
use crate::bottom::BottomTab;

fn app_with_points() -> App {
    let (mut app, _) = App::boot(None);
    let snapshot = kentos_contracts::DocumentSnapshotV1::from_json(include_str!(
        "../../../../fixtures/interaction/v1/point-editor.kcad"
    ))
    .expect("the drawing reads");
    let doc = crate::document::Document::new(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

/// The names of the rows shown, in their order.
fn names(app: &App) -> Vec<String> {
    let doc = app.document.as_ref().expect("a drawing");
    app.point_rows(doc)
        .shown
        .iter()
        .map(|&s| match doc.model.get(s) {
            Some(kentos_contracts::Entity::Point(p)) => p.base.label.clone().unwrap_or_default(),
            _ => String::new(),
        })
        .collect()
}

fn ev(app: &mut App, e: Event) {
    let _ = app.update(Message::Points(e));
}

#[test]
fn the_columns_are_the_adrs_sira_the_drawings_order() {
    let got: Vec<(&str, Option<&str>)> = COLUMNS.iter().map(|(l, s, _, _)| (*l, *s)).collect();
    assert_eq!(
        got,
        [
            ("Sıra", None),
            ("Ad", Some("name")),
            ("Y (sağa)", Some("east")),
            ("X (yukarı)", Some("north")),
            ("Z (kot)", Some("z")),
            ("Kod", Some("code")),
            ("Katman", Some("layer")),
        ]
    );
}

#[test]
fn a_column_sorts_ascending_then_descending_then_in_the_drawings_order() {
    let (mut sort, mut desc) = (None, false);
    (sort, desc) = next_sort(sort, desc, Some("name"));
    assert_eq!((sort, desc), (Some("name"), false));
    (sort, desc) = next_sort(sort, desc, Some("name"));
    assert_eq!((sort, desc), (Some("name"), true));
    (sort, desc) = next_sort(sort, desc, Some("name"));
    assert_eq!((sort, desc), (None, false));
    assert_eq!(next_sort(Some("name"), true, Some("z")), (Some("z"), false));
    assert_eq!(next_sort(Some("z"), false, None), (None, false));
}

#[test]
fn a_click_selects_the_row_ctrl_turns_one_over_shift_takes_a_run() {
    let s = |v: &[u32]| v.iter().map(|&i| Slot(i)).collect::<Vec<_>>();
    let shown = s(&[11, 12, 13, 14, 15]);
    assert_eq!(
        click_pick(&s(&[99]), &shown, 2, None, false, false),
        s(&[13])
    );
    assert_eq!(
        click_pick(&s(&[99, 13]), &shown, 2, Some(0), true, false),
        s(&[99])
    );
    assert_eq!(
        click_pick(&s(&[99]), &shown, 2, Some(0), true, false),
        s(&[99, 13])
    );
    assert_eq!(
        click_pick(&s(&[12]), &shown, 3, Some(1), false, true),
        s(&[12, 13, 14])
    );
    assert_eq!(
        click_pick(&s(&[15]), &shown, 1, Some(4), false, true),
        s(&[12, 13, 14, 15])
    );
    assert_eq!(click_pick(&[], &shown, 3, None, false, true), s(&[14]));
}

#[test]
fn a_point_reads_its_label_place_elevation_kod_layer_and_selection() {
    let p = kentos_contracts::PointEntity {
        base: kentos_contracts::EntityBase {
            id: 4,
            layer_id: "nokta".into(),
            color: None,
            attrs: [
                ("Kod".to_owned(), "SN".to_owned()),
                ("Tür".to_owned(), "x".to_owned()),
            ]
            .into(),
            label: Some("101".into()),
            symbol: None,
            line_weight: None,
        },
        p: kentos_contracts::Vec2 {
            x: 487001.5,
            y: 4420002.25,
        },
        z: Some(100.5),
    };
    let row = row_of(&p, "Nokta", true);
    assert_eq!(
        (
            row.name.as_deref(),
            row.east,
            row.north,
            row.z,
            row.code.as_deref(),
            row.layer.as_str(),
            row.selected
        ),
        (
            Some("101"),
            487001.5,
            4420002.25,
            Some(100.5),
            Some("SN"),
            "Nokta",
            true
        )
    );
}

#[test]
fn the_tab_lists_every_point_and_follows_its_query() {
    let mut app = app_with_points();
    let _ = app.update(Message::Run("point.editor"));
    assert!(app.command_expanded);
    assert_eq!(app.bottom_tab, BottomTab::Points);
    assert_eq!(names(&app).len(), 36);
    // Sorted by Ad: the natural order, unnamed last.
    ev(&mut app, Event::Sort(1));
    let sorted = names(&app);
    assert_eq!(
        &sorted[..8],
        [
            "101", "101/1", "101/4", "101/9", "101/10", "101/11", "101/12", "101/14"
        ]
    );
    assert_eq!(&sorted[34..], ["", ""]);
    // A search, caseless: the names holding “p1”.
    ev(&mut app, Event::Search("p1".into()));
    assert_eq!(names(&app), ["P13", "P15", "P16", "P18"]);
    // One layer; only the selected (none).
    ev(&mut app, Event::Search(String::new()));
    ev(&mut app, Event::Layer(Some("Kot".into())));
    assert_eq!(names(&app).len(), 17);
    ev(&mut app, Event::OnlySelected(true));
    assert!(names(&app).is_empty());
    ev(&mut app, Event::OnlySelected(false));
    ev(&mut app, Event::Layer(None));
    // Descending, then the drawing's order again.
    ev(&mut app, Event::Sort(1));
    assert_eq!(names(&app)[0], "S9");
    ev(&mut app, Event::Sort(1));
    assert_eq!(&names(&app)[..3], ["101", "102", "103"]);
}

#[test]
fn a_click_selects_the_point_and_a_double_click_on_its_number_zooms_to_it() {
    let mut app = app_with_points();
    let _ = app.update(Message::Run("point.editor"));
    let shown = {
        let doc = app.document.as_ref().expect("a drawing");
        app.point_rows(doc).shown
    };
    ev(&mut app, Event::Press(2));
    assert_eq!(app.selection.ids(), [shown[2]]);
    app.modifiers = iced::keyboard::Modifiers::SHIFT;
    ev(&mut app, Event::Press(4));
    assert_eq!(app.selection.ids(), &shown[2..=4]);
    app.modifiers = iced::keyboard::Modifiers::CTRL;
    ev(&mut app, Event::Press(3));
    assert_eq!(app.selection.ids(), [shown[2], shown[4]]);
    app.modifiers = iced::keyboard::Modifiers::empty();
    // The drawing's selection shows: its first row is the one revealed.
    {
        let doc = app.document.as_ref().expect("a drawing");
        assert_eq!(app.point_rows(doc).first_selected, Some(2));
    }
    let before = app.viewport.camera.center;
    ev(&mut app, Event::Zoom(10));
    assert_eq!(app.selection.ids(), [shown[10]]);
    assert_ne!(app.viewport.camera.center, before, "zoomed to the point");
}

#[test]
fn the_editor_walks_down_right_and_left_as_the_web_does() {
    let ids: Vec<Slot> = [11, 12, 13].into_iter().map(Slot).collect();
    let at = |s: u32, c: EditColumn| Some((Slot(s), c));
    assert_eq!(
        next_cell(&ids, Slot(12), EditColumn::East, Walk::Down),
        at(13, EditColumn::East)
    );
    assert_eq!(
        next_cell(&ids, Slot(13), EditColumn::East, Walk::Down),
        None
    );
    assert_eq!(
        next_cell(&ids, Slot(11), EditColumn::Name, Walk::Right),
        at(11, EditColumn::East)
    );
    assert_eq!(
        next_cell(&ids, Slot(11), EditColumn::Code, Walk::Right),
        at(12, EditColumn::Name)
    );
    assert_eq!(
        next_cell(&ids, Slot(13), EditColumn::Code, Walk::Right),
        None
    );
    assert_eq!(
        next_cell(&ids, Slot(12), EditColumn::Name, Walk::Left),
        at(11, EditColumn::Code)
    );
    assert_eq!(
        next_cell(&ids, Slot(11), EditColumn::Name, Walk::Left),
        None
    );
    assert_eq!(
        next_cell(&ids, Slot(99), EditColumn::Name, Walk::Down),
        None
    );
}

/// Numbers within 1e-9 (a missing field is null), everything else exactly.
pub(super) fn same(a: &serde_json::Value, e: &serde_json::Value, path: &str) -> Result<(), String> {
    use serde_json::Value;
    match (a, e) {
        (Value::Number(x), Value::Number(y)) => {
            let (x, y) = (
                x.as_f64().unwrap_or(f64::NAN),
                y.as_f64().unwrap_or(f64::NAN),
            );
            if (x - y).abs() <= 1e-9 {
                Ok(())
            } else {
                Err(format!("{path}: {x} ≠ {y}"))
            }
        }
        (Value::Array(x), Value::Array(y)) => {
            if x.len() != y.len() {
                return Err(format!("{path}: {} öğe ≠ {} öğe", x.len(), y.len()));
            }
            for (i, (p, q)) in x.iter().zip(y).enumerate() {
                same(p, q, &format!("{path}[{i}]"))?;
            }
            Ok(())
        }
        (Value::Object(x), Value::Object(y)) => {
            let mut keys: Vec<&String> = x.keys().chain(y.keys()).collect();
            keys.sort();
            keys.dedup();
            for k in keys {
                same(
                    x.get(k).unwrap_or(&Value::Null),
                    y.get(k).unwrap_or(&Value::Null),
                    &format!("{path}.{k}"),
                )?;
            }
            Ok(())
        }
        _ if a == e => Ok(()),
        _ => Err(format!("{path}: {a} ≠ {e}")),
    }
}

/// An object as the cases compare it: its kind, label, attributes, and a
/// point's place and elevation or line work's paths.
fn view(e: &kentos_contracts::Entity) -> serde_json::Value {
    use serde_json::json;
    let b = e.base();
    let mut v = json!({ "kind": e.kind(), "label": b.label, "attrs": b.attrs });
    match e {
        kentos_contracts::Entity::Point(p) => {
            v["at"] = json!([p.p.x, p.p.y]);
            v["z"] = json!(p.z);
        }
        _ => {
            v["paths"] = kentos_native_application::elevation::paths(e)
                .iter()
                .map(|p| json!({ "pts": p.pts.iter().map(|q| [q.x, q.y]).collect::<Vec<_>>(), "zs": p.zs }))
                .collect();
        }
    }
    v
}

/// Every case of `fixtures/point-editor/v1/edits.json` on its own drawing,
/// through the editor's writes: the drawing after, the messages said, the
/// undo step and the next draft's name, as the web's `pointEdit.test.ts`.
#[test]
fn every_edit_is_written_as_the_reference_writes_it() {
    use serde_json::{Value, json};
    let file: Value = serde_json::from_str(include_str!(
        "../../../../fixtures/point-editor/v1/edits.json"
    ))
    .expect("edits.json reads");
    assert_eq!(file["format"], "kentos.point-editor-edits");
    let layers: Vec<Value> = file["layers"]
        .as_array()
        .expect("layers")
        .iter()
        .map(|l| {
            json!({ "id": l["id"], "name": l["name"], "type": "layer", "visible": true, "locked": l["locked"], "expanded": true,
                "style": { "color": "fg", "lineType": "continuous", "lineWeight": 0.25 }, "children": [] })
        })
        .collect();
    let cases = file["cases"].as_array().expect("cases");
    assert!(cases.len() >= 30, "{} cases", cases.len());
    let mut off = Vec::new();
    for c in cases {
        let name = c["name"].as_str().unwrap_or_default();
        let snapshot = json!({
            "format": "kentos.document", "version": 1, "name": name,
            "settings": { "srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                "plotScale": 1000, "workspace": "hybrid", "drawingFont": "barlow" },
            "origin": { "x": 0, "y": 0 }, "layers": layers, "activeLayer": c["active"], "entities": c["objects"],
            "styles": { "items": [], "categories": [] }
        });
        let snapshot = kentos_contracts::DocumentSnapshotV1::from_json(&snapshot.to_string())
            .expect("the case's drawing");
        let mut doc = kentos_domain::Document::from_snapshot(snapshot).expect("opens");
        let text = |v: &Value| v.as_str().unwrap_or_default().to_owned();
        let (said, step, next) = if let Some(a) = c.get("cell") {
            let col =
                EditColumn::from_key(a["column"].as_str().unwrap_or_default()).expect("a column");
            let slot = Slot(a["id"].as_u64().unwrap_or_default() as u32);
            let out = write_cell(
                &mut doc,
                slot,
                col,
                &text(&a["text"]),
                a["follow"].as_bool().unwrap_or(false),
            );
            (out.said, out.step, None)
        } else {
            let a = &c["draft"];
            let d = Draft {
                name: text(&a["name"]),
                east: text(&a["east"]),
                north: text(&a["north"]),
                z: text(&a["z"]),
                code: text(&a["code"]),
            };
            let active = text(&c["active"]);
            let out = write_draft(&mut doc, &d, &active, None);
            (out.outcome.said, out.outcome.step, Some(out.next))
        };
        let objects: Vec<Value> = doc.entities().map(view).collect();
        let undone = match step {
            Some(_) => doc.undo(),
            None => doc.can_undo().then(|| "yazıldı".to_owned()),
        };
        let mut seen = json!({ "said": said, "step": undone, "objects": objects });
        if let Some(next) = next {
            seen["next"] = json!(next);
        }
        if let Err(e) = same(&seen, &c["expected"], name) {
            off.push(e);
        }
    }
    assert!(
        off.is_empty(),
        "{} durum farklı:\n{}",
        off.len(),
        off.join("\n")
    );
}

/// The editor in the app: a double click opens a cell with its whole value,
/// Enter writes it and goes down the column, Tab right, a refused value keeps
/// the cell open with what was typed, Esc gives up; Satır ekle writes a row
/// and opens the next with the name one more.
#[test]
fn a_cell_is_edited_in_place_and_satir_ekle_writes_rows() {
    use super::Target;
    let mut app = app_with_points();
    let _ = app.update(Message::Run("point.editor"));
    let shown = {
        let doc = app.document.as_ref().expect("a drawing");
        app.point_rows(doc).shown
    };
    let point = |app: &App, slot: Slot| match app.document.as_ref().and_then(|d| d.model.get(slot))
    {
        Some(kentos_contracts::Entity::Point(p)) => p.clone(),
        _ => panic!("a point"),
    };
    // Y of the first row: its whole value.
    ev(&mut app, Event::Edit(0, 2));
    assert_eq!(
        app.points.editing,
        Some((Target::Point(shown[0]), EditColumn::East))
    );
    assert_eq!(app.points.text, "487000");
    ev(&mut app, Event::Input("487000.5".into()));
    ev(&mut app, Event::Finish(Some(Walk::Down)));
    assert_eq!(point(&app, shown[0]).p.x, 487000.5);
    assert_eq!(
        app.points.editing,
        Some((Target::Point(shown[1]), EditColumn::East))
    );
    // A value refused stays with what was typed.
    ev(&mut app, Event::Input("abc".into()));
    ev(&mut app, Event::Finish(Some(Walk::Right)));
    assert_eq!(
        app.points.editing,
        Some((Target::Point(shown[1]), EditColumn::East))
    );
    assert_eq!(app.points.text, "abc");
    assert_eq!(
        crate::files_testing::last_said(&app),
        "Nokta editörü: Y bir sayı olmalı."
    );
    // Tab writes and goes right.
    ev(&mut app, Event::Input("487024.25".into()));
    ev(&mut app, Event::Finish(Some(Walk::Right)));
    assert_eq!(point(&app, shown[1]).p.x, 487024.25);
    assert_eq!(
        app.points.editing,
        Some((Target::Point(shown[1]), EditColumn::North))
    );
    // Esc gives up.
    ev(&mut app, Event::Input("1".into()));
    ev(&mut app, Event::Cancel);
    assert_eq!(app.points.editing, None);
    assert_eq!(point(&app, shown[1]).p.y, 4420000.0);
    // Satır ekle: Ad first; Enter writes and opens the next, its Y.
    ev(&mut app, Event::AddRow);
    assert_eq!(app.points.editing, Some((Target::Draft, EditColumn::Name)));
    ev(&mut app, Event::Input("201".into()));
    ev(&mut app, Event::Finish(Some(Walk::Right)));
    ev(&mut app, Event::Input("487030".into()));
    ev(&mut app, Event::Finish(Some(Walk::Right)));
    ev(&mut app, Event::Input("4420030".into()));
    let before = app
        .document
        .as_ref()
        .map(|d| d.model.len())
        .unwrap_or_default();
    ev(&mut app, Event::Finish(Some(Walk::Down)));
    assert_eq!(
        app.document.as_ref().map(|d| d.model.len()),
        Some(before + 1)
    );
    assert_eq!(app.points.editing, Some((Target::Draft, EditColumn::East)));
    assert_eq!(
        app.points.draft.as_ref().map(|d| d.name.as_str()),
        Some("202")
    );
    // Esc drops the draft.
    ev(&mut app, Event::Cancel);
    assert_eq!(app.points.draft, None);
}

/// Sil: the selected rows' points go in one step, as the Sil tool takes a
/// selection (the web's `noktalar-sil` scene).
#[test]
fn sil_removes_the_selected_rows_in_one_step() {
    let mut app = app_with_points();
    let _ = app.update(Message::Run("point.editor"));
    let size = |app: &App| app.document.as_ref().map(|d| d.model.len());
    let before = size(&app).expect("a drawing");
    // A click, then Shift and a click: the first two rows.
    ev(&mut app, Event::Press(0));
    app.modifiers = iced::keyboard::Modifiers::SHIFT;
    ev(&mut app, Event::Press(1));
    app.modifiers = iced::keyboard::Modifiers::default();
    assert_eq!(app.selection.len(), 2);
    ev(&mut app, Event::Remove);
    assert_eq!(size(&app), Some(before - 2));
    assert_eq!(names(&app)[..2], ["103", "104"]);
    assert_eq!(crate::files_testing::last_said(&app), "2 nesne silindi.");
    // One step back brings both.
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(size(&app), Some(before));
    assert_eq!(names(&app)[..2], ["101", "102"]);
}

/// Çift noktaları ayıkla over every row (nothing selected): Çiftleri göster
/// shows the two groups, Sıra their numbers; Ayıkla from there, Ortalaması,
/// writes in one step and the table shows its query again (the web's
/// `noktalar-ciftler` and `noktalar-ayiklandi` scenes).
#[test]
fn the_dedupe_window_shows_the_groups_and_writes_them_in_one_step() {
    use kentos_geometry_core::ops::point_editor::Keep;

    use super::batch::Kind;
    use super::batch_view::WindowEvent;
    let mut app = app_with_points();
    let _ = app.update(Message::Run("point.editor"));
    let rows = |app: &App| {
        let doc = app.document.as_ref().expect("a drawing");
        app.point_rows(doc)
    };
    ev(&mut app, Event::Batch(Kind::Dedupe));
    ev(&mut app, Event::Window(WindowEvent::ShowGroups));
    assert!(app.dialog.is_none());
    assert_eq!(rows(&app).group_no, [1, 1, 2, 2]);
    assert_eq!(names(&app), ["103", "103", "105", "S9"]);
    ev(&mut app, Event::Batch(Kind::Dedupe));
    ev(&mut app, Event::Window(WindowEvent::Keep(Keep::Average)));
    ev(&mut app, Event::Window(WindowEvent::Apply));
    assert!(app.dialog.is_none());
    assert_eq!(
        crate::files_testing::last_said(&app),
        "Nokta editörü: 2 grupta 2 nokta silindi, 1 nokta ortalamaya taşındı."
    );
    let after = rows(&app);
    assert!(after.group_no.is_empty());
    assert_eq!(after.points.len(), 34);
    let undone = app.document.as_mut().and_then(|d| d.model.undo());
    assert_eq!(undone.as_deref(), Some("Çift noktaları ayıkla"));
}

/// A press (or a right one) on the lowest `caption` on screen: the bottom
/// panel's, below the ribbon's tab of the same name.
fn press_lowest(
    snapshot: &mut kentos_ui::snapshot::Snapshot,
    app: &mut App,
    caption: &str,
    right: bool,
) {
    use kentos_ui::snapshot::Input;
    let at = crate::files_testing::find_texts(snapshot, app, caption)
        .into_iter()
        .max_by(|a, b| a.y.total_cmp(&b.y))
        .unwrap_or_else(|| panic!("{caption} is on screen"))
        .center();
    let mut update = |app: &mut App, message: Message| {
        let _ = app.update(message);
    };
    let input = if right {
        Input::RightClick(at)
    } else {
        Input::Click(at)
    };
    snapshot.input(app, App::view, &mut update, input);
}

/// The task's widget operations run in the picture, as iced's runtime runs
/// them.
fn operate(snapshot: &mut kentos_ui::snapshot::Snapshot, app: &App, task: Task<Message>) {
    use iced::futures::StreamExt as _;
    if let Some(mut stream) = iced_runtime::task::into_stream(task) {
        while let Some(action) = iced::futures::executor::block_on(stream.next()) {
            if let iced_runtime::Action::Widget(operation) = action {
                snapshot.operate(app.view(), operation);
            }
        }
    }
}

/// The rows the first `n` rows of the table show: a press, then Shift and a press.
fn pick_rows(app: &mut App, last: usize) {
    ev(app, Event::Press(0));
    app.modifiers = iced::keyboard::Modifiers::SHIFT;
    ev(app, Event::Press(last));
    app.modifiers = iced::keyboard::Modifiers::default();
}

/// İşlemler ▾ → Sıralı numara ver over six selected rows: the window opens
/// with the first row's name, writes 201… in one step and closes (the web's
/// `noktalar-sirali-numara` scene).
#[test]
fn the_actions_window_numbers_the_selected_rows_in_one_step() {
    use super::batch::Kind;
    let mut app = app_with_points();
    let _ = app.update(Message::Run("point.editor"));
    pick_rows(&mut app, 5);
    ev(&mut app, Event::Batch(Kind::Number));
    assert!(matches!(app.dialog, Some(crate::app::Dialog::PointBatch)));
    ev(
        &mut app,
        Event::Window(super::batch_view::WindowEvent::Text("201".into())),
    );
    ev(
        &mut app,
        Event::Window(super::batch_view::WindowEvent::Apply),
    );
    assert!(app.dialog.is_none());
    assert_eq!(
        names(&app)[..7],
        ["201", "202", "203", "204", "205", "206", "101/1"]
    );
    assert_eq!(
        crate::files_testing::last_said(&app),
        "Nokta editörü: 6 noktanın adı değişti."
    );
    let undone = app.document.as_mut().and_then(|d| d.model.undo());
    assert_eq!(undone.as_deref(), Some("Sıralı numara ver"));
}

/// A row's menu on a row not selected: it alone is selected, and its
/// Katmana taşı moves it (the web's `noktalar-katmana-tasi` scene); a prefix
/// left empty keeps Uygula off.
#[test]
fn a_rows_menu_takes_that_row_alone_when_it_is_not_selected() {
    use super::RowAction;
    use super::batch::Kind;
    use super::batch_view::WindowEvent;
    let mut app = app_with_points();
    let _ = app.update(Message::Run("point.editor"));
    let shown = {
        let doc = app.document.as_ref().expect("a drawing");
        app.point_rows(doc).shown
    };
    ev(&mut app, Event::Row(4, RowAction::Batch(Kind::Layer)));
    assert_eq!(app.selection.ids(), [shown[4]]);
    ev(&mut app, Event::Window(WindowEvent::Layer("kot".into())));
    ev(&mut app, Event::Window(WindowEvent::Apply));
    let layer = match app.document.as_ref().and_then(|d| d.model.get(shown[4])) {
        Some(kentos_contracts::Entity::Point(p)) => p.base.layer_id.clone(),
        _ => String::new(),
    };
    assert_eq!(layer, "kot");
    assert_eq!(
        crate::files_testing::last_said(&app),
        "Nokta editörü: 1 nokta “Kot” katmanına taşındı."
    );
    // Yeniden adlandır with no prefix writes nothing.
    ev(&mut app, Event::Row(0, RowAction::Batch(Kind::Rename)));
    let before = app.document.as_ref().map(|d| d.model.generation());
    ev(&mut app, Event::Window(WindowEvent::Apply));
    assert_eq!(app.document.as_ref().map(|d| d.model.generation()), before);
    assert!(matches!(app.dialog, Some(crate::app::Dialog::PointBatch)));
    ev(&mut app, Event::Window(WindowEvent::Close));
    assert!(app.dialog.is_none());
}

/// The coordinate list import the table's İçe aktar reads (the web's `IMPORTED`):
/// 101 and 105 measured anew, 201 new.
const IMPORTED: &str =
    "101 487000.02 4419999.99 100.3\r\n105 487024.01 4420018 101.8\r\n201 487050 4420010 103\r\n";

fn exchange(app: &mut App, event: crate::exchange::Event) {
    let task = app.update(Message::Exchange(Box::new(event)));
    crate::files_testing::drive(app, task);
}

/// Dışa aktar with nothing selected, the table sorted by Ad descending: the
/// coordinate list window takes the table's rows, and the file holds them in
/// the table's order, not the drawing's (the web's `noktalar-disa-aktar`).
#[test]
fn the_export_writes_the_tables_rows_in_its_order() {
    let mut app = app_with_points();
    let _ = app.update(Message::Run("point.editor"));
    app.selection.set(Vec::<Slot>::new());
    ev(&mut app, Event::Sort(1));
    ev(&mut app, Event::Sort(1));
    let shown = names(&app);
    let path = crate::files_testing::scratch("points-export").join("noktalar.ncn");
    app.picker = crate::app::Picker::File(path.clone());
    let task = app.update(Message::Points(Event::Export));
    crate::files_testing::drive(&mut app, task);
    assert!(matches!(
        app.exchange,
        Some(crate::exchange::Window::CoordExport(_))
    ));
    exchange(
        &mut app,
        crate::exchange::Event::CoordExport(crate::exchange::coord_export::Event::Run),
    );
    let text = std::fs::read_to_string(&path).expect("written");
    let written: Vec<&str> = text
        .lines()
        .map(|l| l.split(' ').next().unwrap_or_default())
        .collect();
    assert_eq!(written, shown);
    assert_eq!(written[..3], ["S9", "P18", "P16"]);
}

/// İçe aktar: the points go in as one step, then Çift noktaları ayıkla opens
/// by Aynı ad over the points carrying the names the file brought again;
/// Sonuncusu keeps the file's in a step of its own (the web's
/// `noktalar-ice-aktar` and `noktalar-ice-aktarildi`).
#[test]
fn the_import_offers_the_names_brought_again_and_the_last_keeps_the_files() {
    use kentos_geometry_core::ops::point_editor::Keep;

    use super::batch_view::WindowEvent;
    let mut app = app_with_points();
    let _ = app.update(Message::Run("point.editor"));
    let path = crate::files_testing::scratch("points-import").join("olcum.ncn");
    std::fs::write(&path, IMPORTED).expect("the list");
    app.picker = crate::app::Picker::File(path);
    let task = app.update(Message::Points(Event::Import));
    crate::files_testing::drive(&mut app, task);
    assert!(matches!(
        app.exchange,
        Some(crate::exchange::Window::CoordImport(_))
    ));
    exchange(
        &mut app,
        crate::exchange::Event::CoordImport(crate::exchange::coord_import::Event::Run),
    );
    assert!(matches!(app.dialog, Some(crate::app::Dialog::PointBatch)));
    ev(&mut app, Event::Window(WindowEvent::Keep(Keep::Last)));
    ev(&mut app, Event::Window(WindowEvent::Apply));
    assert!(app.dialog.is_none());
    assert_eq!(
        crate::files_testing::last_said(&app),
        "Nokta editörü: 2 grupta 2 nokta silindi."
    );
    let doc = app.document.as_mut().expect("a drawing");
    let named = |doc: &crate::document::Document, n: &str| -> Vec<(f64, f64, Option<f64>)> {
        doc.model
            .entities()
            .filter_map(|e| match e {
                kentos_contracts::Entity::Point(p) if p.base.label.as_deref() == Some(n) => {
                    Some((p.p.x, p.p.y, p.z))
                }
                _ => None,
            })
            .collect()
    };
    assert_eq!(named(doc, "101"), [(487_000.02, 4_419_999.99, Some(100.3))]);
    assert_eq!(named(doc, "105"), [(487_024.01, 4_420_018.0, Some(101.8))]);
    assert_eq!(named(doc, "201"), [(487_050.0, 4_420_010.0, Some(103.0))]);
    assert_eq!(doc.model.undo().as_deref(), Some("Çift noktaları ayıkla"));
    assert_eq!(
        doc.model.undo().as_deref(),
        Some("Koordinat listesi: olcum.ncn")
    );
}

/// An import whose names are new opens no window after it.
#[test]
fn an_import_of_new_names_opens_nothing_after_it() {
    let mut app = app_with_points();
    let _ = app.update(Message::Run("point.editor"));
    let path = crate::files_testing::scratch("points-import-new").join("yeni.ncn");
    std::fs::write(&path, "301 487060 4420010\r\n302 487061 4420011\r\n").expect("the list");
    app.picker = crate::app::Picker::File(path);
    let task = app.update(Message::Points(Event::Import));
    crate::files_testing::drive(&mut app, task);
    exchange(
        &mut app,
        crate::exchange::Event::CoordImport(crate::exchange::coord_import::Event::Run),
    );
    assert!(app.dialog.is_none());
    assert_eq!(names(&app).len(), 38);
}

/// Pictures of Noktalar for the owner, the scenes the web's
/// `node apps/web/scripts/e2e/shots.mjs pointeditor` takes: three points
/// selected in the drawing; sorted by Ad; a search; one layer; a cell
/// written with Enter; two rows added; two rows removed; the batch windows;
/// Dışa aktar and İçe aktar. Dark and light, at 1440×900 and 1100×650;
/// `.run/shots/noktalar-*`:
///
/// ```text
/// cargo test -p kentos-desktop points::tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            for name in [
                "noktalar",
                "noktalar-ad-sirali",
                "noktalar-ara",
                "noktalar-katman",
                "noktalar-duzenle",
                "noktalar-satir-ekle",
                "noktalar-sil",
                "noktalar-islemler",
                "noktalar-yeniden-adlandir",
                "noktalar-sirali-numara",
                "noktalar-sag-tik",
                "noktalar-katmana-tasi",
                "noktalar-cift-ayikla",
                "noktalar-ciftler",
                "noktalar-ayiklandi",
                "noktalar-disa-aktar",
                "noktalar-disa-aktarildi",
                "noktalar-ice-aktar",
                "noktalar-ice-aktarildi",
            ] {
                let mut app = app_with_points();
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                let _ = app.update(Message::Run("point.editor"));
                let _ = app.update(Message::BottomResized(300.0));
                app.selection.set([Slot(4), Slot(5), Slot(6)]);
                // The window laid out before the owner acts, as in the app: the bar knows its
                // width and the table its height when a row is to be shown.
                let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                let mut update = |app: &mut App, message| {
                    let _ = app.update(message);
                };
                snapshot.settle(&mut app, App::view, &mut update);
                match name {
                    "noktalar-ad-sirali" => ev(&mut app, Event::Sort(1)),
                    "noktalar-ara" => {
                        ev(&mut app, Event::Sort(1));
                        ev(&mut app, Event::Search("p1".into()));
                    }
                    "noktalar-katman" => ev(&mut app, Event::Layer(Some("Kot".into()))),
                    // The second row's Y opened by a double click (its first press selects the
                    // row), written with Enter: the row below open (the web's scene).
                    "noktalar-duzenle" => {
                        ev(&mut app, Event::Press(1));
                        ev(&mut app, Event::Edit(1, 2));
                        ev(&mut app, Event::Input("487024.5".into()));
                        ev(&mut app, Event::Finish(Some(Walk::Down)));
                    }
                    // Two rows typed and written, the third open with the name one more.
                    "noktalar-satir-ekle" => {
                        ev(&mut app, Event::AddRow);
                        for (text, walk) in [
                            ("201", Walk::Right),
                            ("487030.25", Walk::Right),
                            ("4420030.5", Walk::Down),
                            ("487031.75", Walk::Right),
                            ("4420031", Walk::Down),
                        ] {
                            ev(&mut app, Event::Input(text.into()));
                            ev(&mut app, Event::Finish(Some(walk)));
                        }
                    }
                    // The first two rows selected and removed (the web's scene).
                    "noktalar-sil" => {
                        ev(&mut app, Event::Press(0));
                        app.modifiers = iced::keyboard::Modifiers::SHIFT;
                        ev(&mut app, Event::Press(1));
                        app.modifiers = iced::keyboard::Modifiers::default();
                        ev(&mut app, Event::Remove);
                    }
                    "noktalar-yeniden-adlandir" => {
                        pick_rows(&mut app, 5);
                        ev(&mut app, Event::Batch(super::batch::Kind::Rename));
                        ev(
                            &mut app,
                            Event::Window(super::batch_view::WindowEvent::Text("P.".into())),
                        );
                    }
                    "noktalar-sirali-numara" => {
                        pick_rows(&mut app, 5);
                        ev(&mut app, Event::Batch(super::batch::Kind::Number));
                        ev(
                            &mut app,
                            Event::Window(super::batch_view::WindowEvent::Text("201".into())),
                        );
                        ev(
                            &mut app,
                            Event::Window(super::batch_view::WindowEvent::Apply),
                        );
                    }
                    "noktalar-katmana-tasi" => {
                        pick_rows(&mut app, 1);
                        ev(
                            &mut app,
                            Event::Row(1, super::RowAction::Batch(super::batch::Kind::Layer)),
                        );
                        ev(
                            &mut app,
                            Event::Window(super::batch_view::WindowEvent::Layer("kot".into())),
                        );
                        ev(
                            &mut app,
                            Event::Window(super::batch_view::WindowEvent::Apply),
                        );
                    }
                    // Çift noktaları ayıkla over every row: Ortalaması chosen (the web's scenes).
                    "noktalar-cift-ayikla" => {
                        app.selection.set(Vec::<Slot>::new());
                        ev(&mut app, Event::Batch(super::batch::Kind::Dedupe));
                        ev(
                            &mut app,
                            Event::Window(super::batch_view::WindowEvent::Keep(
                                kentos_geometry_core::ops::point_editor::Keep::Average,
                            )),
                        );
                    }
                    "noktalar-ciftler" => {
                        app.selection.set(Vec::<Slot>::new());
                        ev(&mut app, Event::Batch(super::batch::Kind::Dedupe));
                        ev(
                            &mut app,
                            Event::Window(super::batch_view::WindowEvent::ShowGroups),
                        );
                    }
                    "noktalar-ayiklandi" => {
                        app.selection.set(Vec::<Slot>::new());
                        ev(&mut app, Event::Batch(super::batch::Kind::Dedupe));
                        ev(
                            &mut app,
                            Event::Window(super::batch_view::WindowEvent::ShowGroups),
                        );
                        ev(&mut app, Event::Batch(super::batch::Kind::Dedupe));
                        ev(
                            &mut app,
                            Event::Window(super::batch_view::WindowEvent::Keep(
                                kentos_geometry_core::ops::point_editor::Keep::Average,
                            )),
                        );
                        ev(
                            &mut app,
                            Event::Window(super::batch_view::WindowEvent::Apply),
                        );
                    }
                    // Dışa aktar with nothing selected, sorted by Ad: the window on the table's
                    // rows (the web's scene).
                    "noktalar-disa-aktar" => {
                        app.selection.set(Vec::<Slot>::new());
                        ev(&mut app, Event::Sort(1));
                        let task = app.update(Message::Points(Event::Export));
                        crate::files_testing::drive(&mut app, task);
                    }
                    // The first two rows (by Ad, descending) written from a row's menu.
                    "noktalar-disa-aktarildi" => {
                        app.selection.set(Vec::<Slot>::new());
                        ev(&mut app, Event::Sort(1));
                        ev(&mut app, Event::Sort(1));
                        pick_rows(&mut app, 1);
                        let path =
                            crate::files_testing::scratch("points-shot").join("noktalar.ncn");
                        app.picker = crate::app::Picker::File(path);
                        let task =
                            app.update(Message::Points(Event::Row(0, super::RowAction::Export)));
                        crate::files_testing::drive(&mut app, task);
                        exchange(
                            &mut app,
                            crate::exchange::Event::CoordExport(
                                crate::exchange::coord_export::Event::Run,
                            ),
                        );
                    }
                    // İçe aktar with 101 and 105 measured anew: in, then Çift noktaları ayıkla by
                    // Aynı ad; Sonuncusu applied in the second.
                    "noktalar-ice-aktar" | "noktalar-ice-aktarildi" => {
                        app.selection.set(Vec::<Slot>::new());
                        let path = crate::files_testing::scratch("points-shot").join("olcum.ncn");
                        std::fs::write(&path, IMPORTED).expect("the list");
                        app.picker = crate::app::Picker::File(path);
                        let task = app.update(Message::Points(Event::Import));
                        crate::files_testing::drive(&mut app, task);
                        exchange(
                            &mut app,
                            crate::exchange::Event::CoordImport(
                                crate::exchange::coord_import::Event::Run,
                            ),
                        );
                        if name == "noktalar-ice-aktarildi" {
                            ev(
                                &mut app,
                                Event::Window(super::batch_view::WindowEvent::Keep(
                                    kentos_geometry_core::ops::point_editor::Keep::Last,
                                )),
                            );
                            ev(
                                &mut app,
                                Event::Window(super::batch_view::WindowEvent::Apply),
                            );
                        }
                    }
                    _ => {}
                }
                snapshot.settle(&mut app, App::view, &mut update);
                match name {
                    // İşlemler ▾ opened, as its click opens it.
                    "noktalar-islemler" => {
                        press_lowest(&mut snapshot, &mut app, "İşlemler", false);
                        snapshot.settle(&mut app, App::view, &mut update);
                    }
                    // A right click on the fifth row (not selected).
                    "noktalar-sag-tik" => {
                        press_lowest(&mut snapshot, &mut app, "105", true);
                        snapshot.settle(&mut app, App::view, &mut update);
                    }
                    // The window's field has the keyboard, as its opening gives it.
                    "noktalar-yeniden-adlandir" => {
                        operate(
                            &mut snapshot,
                            &app,
                            iced::widget::operation::focus(iced::widget::Id::new(
                                super::batch_view::FIELD,
                            )),
                        );
                        snapshot.settle(&mut app, App::view, &mut update);
                    }
                    _ => {}
                }
                if app.points.editing() {
                    // The field takes the keyboard with its text selected, as the app's task
                    // gives it when the edit opens.
                    operate(&mut snapshot, &app, focus_field());
                    snapshot.settle(&mut app, App::view, &mut update);
                }
                let file = out.join(format!("{name}-{width}x{height}{suffix}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
}
