//! Noktalar in a Harita project (docs/adr/1540): a survey point's coordinate cells edit one point and survive reopening,
//! the inspector edits the point shown and the grid follows. Harita's profile shows the tab, so these run with the
//! module registered (docs/adr/9199 §4.2); the rest of the points' tests are the core's (gis/points/display_tests.rs).

use crate::app::{App, Message};
use crate::gis;
use crate::points::Event;
use crate::points::edit::EditColumn;
use kentos_commands::Entity;
use kentos_domain::Slot;

fn ev(app: &mut App, e: Event) {
    let _ = app.update(Message::Gis(gis::Event::Points(e)));
}

/// The task's widget operations run in the picture, as iced's runtime runs them.
fn operate(snapshot: &mut kentos_ui::snapshot::Snapshot, app: &App, task: iced::Task<Message>) {
    use iced::futures::StreamExt as _;
    if let Some(mut stream) = iced_runtime::task::into_stream(task) {
        while let Some(action) = iced::futures::executor::block_on(stream.next()) {
            if let iced_runtime::Action::Widget(operation) = action {
                snapshot.operate(app.view(), operation);
            }
        }
    }
}

/// Three survey points, two of them close enough for the cartographic engine to choose different sides.
fn survey_project(workspace: &str, work: &str, style: serde_json::Value) -> App {
    use serde_json::json;
    let mut drawing: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../fixtures/interaction/v1/empty.kcad"
    ))
    .unwrap();
    drawing["settings"]["workspace"] = json!(workspace);
    if workspace == "cad" {
        drawing["settings"]["cadWork"] = json!(work);
    }
    drawing["layers"][0]["style"]["label"] = style;
    drawing["entities"] = json!([
        {"kind":"point","id":1,"layerId":"cizim","label":"P101","attrs":{"Ad":"P101","Kod":"SN"},"p":{"x":487000.0,"y":4420000.0}},
        {"kind":"point","id":2,"layerId":"cizim","label":"P102","attrs":{},"p":{"x":487000.12,"y":4420000.02}},
        {"kind":"point","id":3,"layerId":"cizim","label":"P103","attrs":{"Kod":"BN"},"p":{"x":487010.0,"y":4420003.0}}
    ]);
    let (mut app, _) = App::boot(None);
    let snapshot = kentos_commands::DocumentSnapshotV1::from_json(&drawing.to_string()).unwrap();
    let doc = crate::document::Document::new(snapshot, None).unwrap();
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

#[test]
fn survey_coordinate_cells_edit_one_point_and_survive_reopening() {
    use crate::points::{Target, Walk};
    use iced::{Point, Size, keyboard::key::Named};
    use kentos_ui::snapshot::{Input, Snapshot};
    let mut app = survey_project("cad", "map", serde_json::Value::Null);
    app.selection.set([Slot(1), Slot(2)]);
    let _ = app.update(Message::Run("view.coords"));
    let _ = app.update(Message::BottomResized(270.));
    let mut snap = Snapshot::new(Size::new(1440., 900.)).unwrap();
    let edit_tasks = std::cell::RefCell::new(Vec::new());
    let mut update = |app: &mut App, message| {
        let opening = matches!(
            message,
            Message::Gis(crate::gis::Event::Points(
                Event::Edit(..) | Event::EditCoordinate(..)
            ))
        );
        let task = app.update(message);
        if opening {
            edit_tasks.borrow_mut().push(task);
        }
    };
    let apply_edit_tasks = |snapshot: &mut Snapshot, app: &App| {
        let tasks = edit_tasks.take();
        assert!(!tasks.is_empty(), "click opens a field through the widget");
        for task in tasks {
            operate(snapshot, app, task);
        }
    };
    snap.settle(&mut app, App::view, &mut update);
    let find = |s: &mut Snapshot, a: &App, text| {
        crate::files_testing::find_texts(s, a, text)
            .into_iter()
            .max_by(|a, b| a.y.total_cmp(&b.y))
            .unwrap()
            .center()
    };
    let name = find(&mut snap, &app, "P101");
    let second = find(&mut snap, &app, "P102");
    let code = find(&mut snap, &app, "SN");
    // Click the empty part of the name cell, then the wholly empty code cell on another row.
    snap.input(
        &mut app,
        App::view,
        &mut update,
        Input::Click(Point::new(name.x + 32., name.y)),
    );
    assert_eq!(
        app.gis.points.editing,
        Some((Target::Point(Slot(1)), EditColumn::Name))
    );
    apply_edit_tasks(&mut snap, &app);
    snap.input(&mut app, App::view, &mut update, Input::Type("S101".into()));
    snap.input(
        &mut app,
        App::view,
        &mut update,
        Input::Click(Point::new(code.x + 30., second.y)),
    );
    assert_eq!(
        app.gis.points.editing,
        Some((Target::Point(Slot(2)), EditColumn::Code))
    );
    apply_edit_tasks(&mut snap, &app);
    snap.input(&mut app, App::view, &mut update, Input::Type("RS".into()));
    snap.input(&mut app, App::view, &mut update, Input::Key(Named::Enter));
    let doc = &app.document.as_ref().unwrap().model;
    let first = doc.get(Slot(1)).unwrap().base();
    assert_eq!(
        (
            first.label.as_deref(),
            first.attrs["Ad"].as_str(),
            first.attrs["Kod"].as_str()
        ),
        (Some("S101"), "S101", "SN")
    );
    let other = doc.get(Slot(2)).unwrap().base();
    assert_eq!(
        (other.label.as_deref(), other.attrs["Kod"].as_str()),
        (Some("P102"), "RS")
    );
    assert_eq!(app.selection.ids(), &[Slot(1), Slot(2)]);
    if let Some(path) = std::env::var_os("KENTOS_POINT_GRID_SHOT") {
        snap.render(app.view(), &app.theme()).save(path).unwrap();
    }
    let first_uid = doc.uid(Slot(1)).unwrap();
    let second_uid = doc.uid(Slot(2)).unwrap();
    let bytes = kentos_kcad::encode_verified(&doc.to_snapshot_v2()).unwrap();
    let reopened =
        crate::document::Document::from_v2(kentos_kcad::decode(&bytes).unwrap(), None).unwrap();
    let saved = |uid| {
        reopened
            .model
            .get(reopened.model.slot_of(uid).unwrap())
            .unwrap()
            .base()
    };
    let first = saved(first_uid);
    let second = saved(second_uid);
    assert_eq!(first.label.as_deref(), Some("S101"));
    assert_eq!(first.attrs["Ad"], "S101");
    assert_eq!(first.attrs["Kod"], "SN");
    assert_eq!(second.label.as_deref(), Some("P102"));
    assert_eq!(second.attrs["Kod"], "RS");
    let _ = app.update(Message::Run("edit.undo"));
    assert!(
        !app.document
            .as_ref()
            .unwrap()
            .model
            .get(Slot(2))
            .unwrap()
            .base()
            .attrs
            .contains_key("Kod")
    );
    let _ = app.update(Message::Run("edit.redo"));
    let doc = &app.document.as_ref().unwrap().model;
    assert_eq!(doc.get(Slot(2)).unwrap().base().attrs["Kod"], "RS");
    ev(&mut app, Event::EditCoordinate(Slot(1), EditColumn::Name));
    ev(&mut app, Event::Finish(Some(Walk::Right)));
    assert_eq!(
        app.gis.points.editing,
        Some((Target::Point(Slot(1)), EditColumn::Code))
    );
    ev(&mut app, Event::Input("discard".into()));
    ev(&mut app, Event::Cancel);
    assert_eq!(
        app.document
            .as_ref()
            .unwrap()
            .model
            .get(Slot(1))
            .unwrap()
            .base()
            .attrs["Kod"],
        "SN"
    );

    // The coordinate table edits one measured elevation, including zero and an explicit clear.
    let header = find(&mut snap, &app, "Z (kot)");
    let row = find(&mut snap, &app, "P102");
    let kot = Point::new(header.x - 36., row.y);
    for (typed, expected) in [("0", Some(0.)), ("-4,125", Some(-4.125)), ("", None)] {
        snap.input(&mut app, App::view, &mut update, Input::Click(kot));
        assert_eq!(
            app.gis.points.editing,
            Some((Target::Point(Slot(2)), EditColumn::Z))
        );
        apply_edit_tasks(&mut snap, &app);
        if typed.is_empty() {
            snap.input(
                &mut app,
                App::view,
                &mut update,
                Input::Key(Named::Backspace),
            );
        } else {
            snap.input(&mut app, App::view, &mut update, Input::Type(typed.into()));
        }
        snap.input(&mut app, App::view, &mut update, Input::Key(Named::Enter));
        let doc = &app.document.as_ref().unwrap().model;
        let Some(Entity::Point(second)) = doc.get(Slot(2)) else {
            unreachable!()
        };
        assert_eq!(second.z, expected);
        assert_eq!(second.base.attrs["Kod"], "RS");
    }

    // The full editor keeps the identity of the clicked row after sorting.
    let _ = app.update(Message::Run("point.editor"));
    ev(&mut app, Event::Sort(1));
    snap.settle(&mut app, App::view, &mut update);
    let row = find(&mut snap, &app, "P103");
    let elevation_header = find(&mut snap, &app, "Z (kot)");
    snap.input(
        &mut app,
        App::view,
        &mut update,
        Input::Click(Point::new(elevation_header.x, row.y)),
    );
    assert_eq!(
        app.gis.points.editing,
        Some((Target::Point(Slot(3)), EditColumn::Z))
    );
    apply_edit_tasks(&mut snap, &app);
    snap.input(&mut app, App::view, &mut update, Input::Type("12,5".into()));
    snap.input(&mut app, App::view, &mut update, Input::Key(Named::Enter));
    let doc = &app.document.as_ref().unwrap().model;
    let Some(Entity::Point(third)) = doc.get(Slot(3)) else {
        unreachable!()
    };
    assert_eq!(third.z, Some(12.5));
    assert_eq!(third.base.attrs["Kod"], "BN");
    let column = find(&mut snap, &app, "BN");
    snap.input(
        &mut app,
        App::view,
        &mut update,
        Input::Click(Point::new(column.x + 15., row.y)),
    );
    assert_eq!(
        app.gis.points.editing,
        Some((Target::Point(Slot(3)), EditColumn::Code))
    );
    apply_edit_tasks(&mut snap, &app);
    snap.input(
        &mut app,
        App::view,
        &mut update,
        Input::Type("gecici".into()),
    );
    let mut other = app.document.as_ref().unwrap().model.to_snapshot();
    other.layers[0].locked = true;
    app.show_document(crate::document::Document::new(other, None).unwrap());
    assert!(!app.gis.points.editing());
    app.selection.set([Slot(3)]);
    let _ = app.update(Message::Run("view.coords"));
    ev(&mut app, Event::EditCoordinate(Slot(3), EditColumn::Code));
    assert!(!app.gis.points.editing());
    assert_eq!(
        app.document
            .as_ref()
            .unwrap()
            .model
            .get(Slot(3))
            .unwrap()
            .base()
            .attrs["Kod"],
        "BN"
    );
}

#[test]
fn inspector_edits_the_displayed_survey_point_and_updates_the_grid() {
    use crate::properties::Editor;
    use iced::{Point, Size, keyboard::key::Named};
    use kentos_ui::snapshot::{Input, Snapshot};

    let mut app = survey_project("cad", "map", serde_json::Value::Null);
    let mut survey = app.document.as_ref().unwrap().model.to_snapshot();
    let Entity::Point(point) = &mut survey.entities[1] else {
        unreachable!()
    };
    point.z = Some(128.75);
    // The point layer's labels turned on, as its Etiketler does: a layer labels nothing until then (docs/adr/9311).
    let layer = point.base.layer_id.clone();
    fn turn_on(nodes: &mut [kentos_commands::LayerNode], id: &str) {
        for n in nodes {
            if n.id == id {
                n.style.labels = Some(kentos_commands::LayerLabels {
                    mode: kentos_commands::LabelsMode::Single,
                    classes: Vec::new(),
                    obstacle: None,
                });
            }
            turn_on(&mut n.children, id);
        }
    }
    turn_on(&mut survey.layers, &layer);
    app.show_document(crate::document::Document::new(survey, None).unwrap());
    app.selection.set([Slot(1)]);
    let _ = app.update(Message::Run("view.coords"));
    let _ = app.update(Message::BottomResized(200.));
    let mut snap = Snapshot::new(Size::new(1440., 900.)).unwrap();
    fn update(app: &mut App, message: Message) {
        let _ = app.update(message);
    }
    snap.settle(&mut app, App::view, &mut update);
    let inspector_text = |snap: &mut Snapshot, app: &App, text| {
        crate::files_testing::find_texts(snap, app, text)
            .into_iter()
            .max_by(|a, b| a.x.total_cmp(&b.x))
            .unwrap_or_else(|| panic!("missing inspector text {text}"))
    };
    let click_field = |snap: &mut Snapshot, app: &mut App, text| {
        let r = inspector_text(snap, app, text);
        snap.input(
            app,
            App::view,
            &mut update,
            Input::Click(Point::new(r.x + 214., r.center().y)),
        );
    };
    click_field(&mut snap, &mut app, "Nokta adı");
    snap.input(&mut app, App::view, &mut update, Input::Key(Named::End));
    for _ in 0..4 {
        snap.input(
            &mut app,
            App::view,
            &mut update,
            Input::Key(Named::Backspace),
        );
    }
    snap.input(
        &mut app,
        App::view,
        &mut update,
        Input::Type("ÖLÇ-01".into()),
    );
    snap.input(&mut app, App::view, &mut update, Input::Key(Named::Enter));
    let point = app
        .document
        .as_ref()
        .unwrap()
        .model
        .get(Slot(1))
        .unwrap()
        .base();
    assert_eq!(point.label.as_deref(), Some("ÖLÇ-01"));
    assert_eq!(point.attrs["Ad"], "ÖLÇ-01");
    assert_eq!(point.attrs["Kod"], "SN");
    assert!(
        app.spatial
            .label_texts(Slot(1))
            .iter()
            .any(|label| label.contains("ÖLÇ-01"))
    );
    assert!(
        crate::files_testing::find_texts(&mut snap, &app, "ÖLÇ-01")
            .iter()
            .any(|r| r.x < 300.)
    );

    app.selection.set([Slot(1), Slot(2)]);
    snap.settle(&mut app, App::view, &mut update);
    let nav = inspector_text(&mut snap, &app, "1 / 2");
    snap.input(
        &mut app,
        App::view,
        &mut update,
        Input::Click(Point::new(nav.x + nav.width + 17., nav.center().y)),
    );
    inspector_text(&mut snap, &app, "2 / 2");
    click_field(&mut snap, &mut app, "Nokta kodu");
    snap.input(&mut app, App::view, &mut update, Input::Type("POL".into()));
    snap.input(&mut app, App::view, &mut update, Input::Key(Named::Enter));
    assert_eq!(app.selection.ids(), &[Slot(1), Slot(2)]);
    let doc = &app.document.as_ref().unwrap().model;
    assert_eq!(doc.get(Slot(1)).unwrap().base().attrs["Kod"], "SN");
    let Some(Entity::Point(second)) = doc.get(Slot(2)) else {
        unreachable!()
    };
    assert_eq!(second.base.attrs["Kod"], "POL");
    assert_eq!(second.z, Some(128.75));
    click_field(&mut snap, &mut app, "Kot (Z)");
    snap.input(&mut app, App::view, &mut update, Input::Key(Named::End));
    for _ in 0.."128.75".len() {
        snap.input(
            &mut app,
            App::view,
            &mut update,
            Input::Key(Named::Backspace),
        );
    }
    snap.input(
        &mut app,
        App::view,
        &mut update,
        Input::Type("129,25".into()),
    );
    snap.input(&mut app, App::view, &mut update, Input::Key(Named::Enter));
    let doc = &app.document.as_ref().unwrap().model;
    let Some(Entity::Point(first)) = doc.get(Slot(1)) else {
        unreachable!()
    };
    let Some(Entity::Point(second)) = doc.get(Slot(2)) else {
        unreachable!()
    };
    assert_eq!(first.z, None);
    assert_eq!(second.z, Some(129.25));
    assert_eq!(second.base.attrs["Kod"], "POL");
    assert!(
        crate::files_testing::find_texts(&mut snap, &app, "129.250")
            .iter()
            .any(|r| r.x < 1100.)
    );
    let uid = doc.uid(Slot(2)).unwrap();
    let bytes = kentos_kcad::encode_verified(&doc.to_snapshot_v2()).unwrap();
    let reopened =
        crate::document::Document::from_v2(kentos_kcad::decode(&bytes).unwrap(), None).unwrap();
    let Some(Entity::Point(saved)) = reopened.model.get(reopened.model.slot_of(uid).unwrap())
    else {
        unreachable!()
    };
    assert_eq!(saved.base.attrs["Kod"], "POL");
    assert_eq!(saved.z, Some(129.25));
    assert!(
        crate::files_testing::find_texts(&mut snap, &app, "POL")
            .iter()
            .any(|r| r.x < 380.)
    );
    if let Some(path) = std::env::var_os("KENTOS_POINT_INSPECTOR_SHOT") {
        snap.render(app.view(), &app.theme()).save(path).unwrap();
    }
    let _ = app.update(Message::Run("edit.undo"));
    let Some(Entity::Point(before)) = app.document.as_ref().unwrap().model.get(Slot(2)) else {
        unreachable!()
    };
    assert_eq!(before.z, Some(128.75));
    assert_eq!(before.base.attrs["Kod"], "POL");
    let _ = app.update(Message::Run("edit.undo"));
    assert!(
        !app.document
            .as_ref()
            .unwrap()
            .model
            .get(Slot(2))
            .unwrap()
            .base()
            .attrs
            .contains_key("Kod")
    );
    let _ = app.update(Message::Run("edit.redo"));
    assert_eq!(
        app.document
            .as_ref()
            .unwrap()
            .model
            .get(Slot(2))
            .unwrap()
            .base()
            .attrs["Kod"],
        "POL"
    );

    // Leaving an editor for the previous point must commit on the old row and navigate in one click.
    click_field(&mut snap, &mut app, "Nokta kodu");
    snap.input(&mut app, App::view, &mut update, Input::Key(Named::End));
    snap.input(&mut app, App::view, &mut update, Input::Type("-2".into()));
    let nav = inspector_text(&mut snap, &app, "2 / 2");
    snap.input(
        &mut app,
        App::view,
        &mut update,
        Input::Click(Point::new(nav.x - 17., nav.center().y)),
    );
    inspector_text(&mut snap, &app, "1 / 2");
    let doc = &app.document.as_ref().unwrap().model;
    let Some(Entity::Point(second)) = doc.get(Slot(2)) else {
        unreachable!()
    };
    assert_eq!(second.base.attrs["Kod"], "POL-2");
    assert_eq!(second.z, Some(128.75));
    assert_eq!(doc.get(Slot(1)).unwrap().base().attrs["Kod"], "SN");

    let mut locked = app.document.as_ref().unwrap().model.to_snapshot();
    locked.layers[0].locked = true;
    app.show_document(crate::document::Document::new(locked, None).unwrap());
    app.selection.set([Slot(2)]);
    let panel = app.properties_panel(app.document.as_ref().unwrap());
    let section = panel
        .sections
        .iter()
        .find(|s| s.id == "surveyIdentity")
        .unwrap();
    assert!(
        section
            .rows
            .iter()
            .all(|row| !matches!(row.editor, Some(Editor::Text(_) | Editor::Number(_))))
    );
    for (workspace, work) in [("gis", ""), ("cad", "architecture")] {
        let mut app = survey_project(workspace, work, serde_json::Value::Null);
        app.selection.set([Slot(1)]);
        let panel = app.properties_panel(app.document.as_ref().unwrap());
        assert!(panel.sections.iter().all(|s| s.id != "surveyIdentity"));
    }
}
