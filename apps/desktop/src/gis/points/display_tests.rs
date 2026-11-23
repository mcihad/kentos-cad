//! Noktaların görünüşü ve düzenlenişi (docs/adr/1540): a point drawn as a solid dot when its layer has no symbol,
//! Nokta adları and Nokta kodları in Görünüm (settings, the label engine's texts), Ad and Kod of the one selected row
//! edited by a click, and Çift noktaları ayıkla setting the points left over aside on a layer.

use crate::gis;
use kentos_commands::Entity;
use kentos_domain::Slot;

use super::Event;
use super::batch::Kind;
use super::batch_view::{ASIDE_LAYER, WindowEvent};
use super::edit::EditColumn;
use crate::app::{App, Message};

fn app_with_points() -> App {
    let (mut app, _) = App::boot(None);
    let snapshot = kentos_commands::DocumentSnapshotV1::from_json(include_str!(
        "../../../../../fixtures/interaction/v1/point-editor.kcad"
    ))
    .expect("the drawing reads");
    let doc = crate::document::Document::new(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn ev(app: &mut App, e: Event) {
    let _ = app.update(Message::Gis(gis::Event::Points(e)));
}

/// The first point with both a name and a code, and the two.
fn named_and_coded(app: &App) -> (Slot, String, String) {
    let doc = app.document.as_ref().expect("a drawing");
    doc.model
        .entities()
        .find_map(|e| match e {
            Entity::Point(p) => {
                let name = p.base.label.clone().filter(|n| !n.trim().is_empty())?;
                let code = p.base.attrs.get("Kod").cloned().filter(|c| !c.trim().is_empty())?;
                Some((Slot(p.base.id), name.trim().to_owned(), code.trim().to_owned()))
            }
            _ => None,
        })
        .expect("a point with a name and a code")
}

#[test]
fn a_layer_without_a_point_symbol_draws_a_solid_dot() {
    let style = kentos_domain::default_style();
    assert!(style.point.is_none());
    let symbols =
        kentos_native_style::simple::symbols_of_layer_style(&style, "#123456", 0.25, false);
    let mark = &symbols["marker"]["layers"][0];
    assert_eq!(mark["shape"], "circle", "{mark}");
    assert_eq!(mark["fill"], "#123456");
    assert!(mark["stroke"].is_null());
    assert_eq!(mark["size"], kentos_native_style::simple::DOT_SIZE);
    // A symbol chosen for the layer is still drawn as its outline.
    let mut crossed = style.clone();
    crossed.point = Some(kentos_commands::PointStyle {
        symbol: kentos_commands::PointSymbol::Cross,
        size: 8.0,
    });
    let symbols =
        kentos_native_style::simple::symbols_of_layer_style(&crossed, "#123456", 0.25, false);
    assert_eq!(symbols["marker"]["layers"][0]["shape"], "cross");
    assert!(symbols["marker"]["layers"][0]["fill"].is_null());
}

#[test]
fn nokta_adlari_and_kodlari_are_settings_and_change_only_the_labels() {
    let mut app = app_with_points();
    let (slot, name, code) = named_and_coded(&app);
    let revision = app.document.as_ref().map(|d| d.model.revision());
    // As the drawing labels it: its name.
    assert!(app.settings.bool("graphics.pointNames"));
    assert!(!app.settings.bool("graphics.pointCodes"));
    let before = app.spatial.label_texts(slot);
    assert!(
        before.is_empty() || before == [name.clone()],
        "{before:?}"
    );
    let labelled = !before.is_empty();
    let _ = app.update(Message::Run("view.pointCodes"));
    assert!(app.settings.bool("graphics.pointCodes"));
    let expect = |t: String| if labelled { vec![t] } else { Vec::new() };
    assert_eq!(app.spatial.label_texts(slot), expect(format!("{name}  {code}")));
    let _ = app.update(Message::Run("view.pointNames"));
    assert!(!app.settings.bool("graphics.pointNames"));
    assert_eq!(app.spatial.label_texts(slot), expect(code.clone()));
    let _ = app.update(Message::Run("view.pointCodes"));
    assert!(app.spatial.label_texts(slot).is_empty());
    let _ = app.update(Message::Run("view.pointNames"));
    assert_eq!(app.spatial.label_texts(slot), before);
    // A view preference: the drawing did not change.
    assert_eq!(app.document.as_ref().map(|d| d.model.revision()), revision);
    // Uygulama ayarları › Görünüm lists both.
    for key in ["graphics.pointNames", "graphics.pointCodes"] {
        assert!(crate::settings_view::KEYS.contains(&key), "{key}");
    }
}

#[test]
fn a_labelled_point_layer_writes_the_name_and_the_code() {
    // Every layer labels its objects beside them, as a map's point layers do.
    let mut drawing: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/interaction/v1/point-editor.kcad"
    ))
    .expect("the drawing reads");
    fn label_all(nodes: &mut serde_json::Value) {
        for node in nodes.as_array_mut().into_iter().flatten() {
            node["style"]["label"] = serde_json::json!({ "placement": "beside", "size": 10.5 });
            label_all(&mut node["children"]);
        }
    }
    label_all(&mut drawing["layers"]);
    let (mut app, _) = App::boot(None);
    let snapshot = kentos_commands::DocumentSnapshotV1::from_json(&drawing.to_string())
        .expect("the drawing reads");
    let doc = crate::document::Document::new(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    let (slot, name, code) = named_and_coded(&app);
    assert_eq!(app.spatial.label_texts(slot), std::slice::from_ref(&name));
    let _ = app.update(Message::Run("view.pointCodes"));
    assert_eq!(app.spatial.label_texts(slot), [format!("{name}  {code}")]);
    let _ = app.update(Message::Run("view.pointNames"));
    assert_eq!(app.spatial.label_texts(slot), [code]);
}

#[test]
fn a_click_on_ad_or_kod_of_the_one_selected_row_edits_it() {
    use iced::Size;
    use kentos_ui::snapshot::{Input, Snapshot};
    let mut app = app_with_points();
    let _ = app.update(Message::Run("point.editor"));
    let _ = app.update(Message::BottomResized(300.0));
    let (slot, name, code) = named_and_coded(&app);
    app.selection.set([slot]);
    let mut snapshot = Snapshot::new(Size::new(1440.0, 900.0)).expect("a renderer");
    let mut update = |app: &mut App, message: Message| {
        let _ = app.update(message);
    };
    snapshot.settle(&mut app, App::view, &mut update);
    // The row's name: the lowest one on screen (the table's, below the drawing); its code on the same row (other
    // rows may have the same code).
    let row_y = crate::files_testing::find_texts(&mut snapshot, &app, &name)
        .into_iter()
        .max_by(|a, b| a.y.total_cmp(&b.y))
        .expect("the name is in the table")
        .center()
        .y;
    for (text, column) in [(name, EditColumn::Name), (code, EditColumn::Code)] {
        let at = crate::files_testing::find_texts(&mut snapshot, &app, &text)
            .into_iter()
            .min_by(|a, b| (a.center().y - row_y).abs().total_cmp(&(b.center().y - row_y).abs()))
            .unwrap_or_else(|| panic!("{text} is in the table"))
            .center();
        snapshot.input(&mut app, App::view, &mut update, Input::Click(at));
        assert_eq!(
            app.gis.points.editing.map(|(_, c)| c),
            Some(column),
            "a click on {text}"
        );
        ev(&mut app, Event::Cancel);
        snapshot.settle(&mut app, App::view, &mut update);
    }
}

#[test]
fn the_points_left_over_go_to_their_own_layer_instead_of_being_deleted() {
    let mut app = app_with_points();
    let _ = app.update(Message::Run("point.editor"));
    let count = |app: &App| {
        let doc = app.document.as_ref().expect("a drawing");
        app.point_rows(doc).points.len()
    };
    let before = count(&app);
    ev(&mut app, Event::Batch(Kind::Dedupe));
    ev(&mut app, Event::Window(WindowEvent::Aside(true)));
    ev(&mut app, Event::Window(WindowEvent::Apply));
    assert!(app.dialog.is_none());
    assert_eq!(
        crate::files_testing::last_said(&app),
        format!("Nokta editörü: 2 grupta 2 nokta “{ASIDE_LAYER}” katmanına taşındı.")
    );
    // Nothing deleted: the two are on the new layer.
    assert_eq!(count(&app), before);
    let doc = app.document.as_ref().expect("a drawing");
    let layer = kentos_native_application::import_apply::layer_named(&doc.model, ASIDE_LAYER)
        .map(|l| l.id.clone())
        .expect("the layer was made");
    assert_eq!(
        doc.model
            .entities()
            .filter(|e| e.base().layer_id == layer)
            .count(),
        2
    );
    // One undo step takes the layer and the move back.
    let undone = app.document.as_mut().and_then(|d| d.model.undo());
    assert_eq!(undone.as_deref(), Some("Çift noktaları ayıkla"));
    let doc = app.document.as_ref().expect("a drawing");
    assert!(kentos_native_application::import_apply::layer_named(&doc.model, ASIDE_LAYER).is_none());
    // The choice is kept for the next window.
    ev(&mut app, Event::Batch(Kind::Dedupe));
    assert!(app.gis.points.batch.as_ref().is_some_and(|w| w.aside_on()));
}

/// The point editor's drawing as a KentOS Harita project whose layers label their objects beside them.
fn map_points_app() -> App {
    let mut drawing: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/interaction/v1/point-editor.kcad"
    ))
    .expect("the drawing reads");
    fn label_all(nodes: &mut serde_json::Value) {
        for node in nodes.as_array_mut().into_iter().flatten() {
            node["style"]["label"] = serde_json::json!({ "placement": "beside", "size": 10.5 });
            label_all(&mut node["children"]);
        }
    }
    label_all(&mut drawing["layers"]);
    // Nokta without a point symbol of its own: drawn with the dot (Kot keeps its crosses).
    for node in drawing["layers"].as_array_mut().into_iter().flatten() {
        if node["name"] == "Nokta" {
            node["style"].as_object_mut().map(|s| s.remove("point"));
        }
    }
    drawing["settings"]["workspace"] = serde_json::json!("cad");
    drawing["settings"]["cadWork"] = serde_json::json!("map");
    let (mut app, _) = App::boot(None);
    let snapshot = kentos_commands::DocumentSnapshotV1::from_json(&drawing.to_string())
        .expect("the drawing reads");
    let doc = crate::document::Document::new(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app.program = Some(kentos_launch::Module::Map);
    app
}

/// The task's widget operations run in the picture, as iced's runtime runs them.
fn operate(
    snapshot: &mut kentos_ui::snapshot::Snapshot,
    app: &App,
    task: iced::Task<Message>,
) {
    use iced::futures::StreamExt as _;
    if let Some(mut stream) = iced_runtime::task::into_stream(task) {
        while let Some(action) = iced::futures::executor::block_on(stream.next()) {
            if let iced_runtime::Action::Widget(operation) = action {
                snapshot.operate(app.view(), operation);
            }
        }
    }
}

/// Giriş's Nokta, the points as dots with their names and codes, Görünüm's Nokta adları and kodları, a name edited
/// by a click, Çift noktaları ayıkla's Ayıklananlar and Menülerde ara; wide light and narrow dark,
/// `.run/gui-evidence/nokta-gorunum-1540/`:
///
/// ```text
/// cargo test -p kentos-desktop --lib points::display_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "writes pictures"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../.run/gui-evidence/nokta-gorunum-1540");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [
        ("light", Size::new(1440.0, 900.0), "acik-genis"),
        ("dark", Size::new(1100.0, 700.0), "koyu-dar"),
    ] {
        let mut app = map_points_app();
        let _ = app
            .settings
            .choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let shot = |snapshot: &mut Snapshot, app: &mut App, file: &str| {
            let mut update = |app: &mut App, message| {
                let _ = app.update(message);
            };
            snapshot.settle(app, App::view, &mut update);
            let path = out.join(format!("{file}-{name}.png"));
            snapshot
                .render(app.view(), &app.theme())
                .save(&path)
                .expect("writes");
            println!("{}", path.display());
        };
        let _ = app.update(Message::Run("view.zoomExtents"));
        let _ = app.update(Message::RibbonTab("home"));
        shot(&mut snapshot, &mut app, "1-giris-nokta");
        let _ = app.update(Message::Run("view.pointCodes"));
        let _ = app.update(Message::RibbonTab("view"));
        shot(&mut snapshot, &mut app, "2-gorunum-ad-kod");
        let _ = app.update(Message::Run("view.pointNames"));
        shot(&mut snapshot, &mut app, "3-gorunum-yalniz-kod");
        let _ = app.update(Message::Run("view.pointNames"));
        let _ = app.update(Message::Run("point.editor"));
        let _ = app.update(Message::BottomResized(300.0));
        let (slot, _, _) = named_and_coded(&app);
        app.selection.set([slot]);
        let row = {
            let doc = app.document.as_ref().expect("a drawing");
            app.point_rows(doc)
                .shown
                .iter()
                .position(|&s| s == slot)
                .expect("its row")
        };
        ev(&mut app, Event::Edit(row, 5));
        shot(&mut snapshot, &mut app, "4-nokta-listesi-kod");
        ev(&mut app, Event::Cancel);
        app.selection.clear();
        ev(&mut app, Event::Batch(Kind::Dedupe));
        ev(&mut app, Event::Window(WindowEvent::Aside(true)));
        shot(&mut snapshot, &mut app, "5-cift-ayikla-katmana");
        ev(&mut app, Event::Window(WindowEvent::Close));
        // Alt+Q: the box opens and takes the keyboard, as in the app.
        let task = app.update(Message::Run("view.commandSearch"));
        snapshot.settle(&mut app, App::view, &mut update);
        operate(&mut snapshot, &app, task);
        let _ = app.update(Message::RibbonSearch("nokta".into()));
        shot(&mut snapshot, &mut app, "6-menulerde-ara");
    }
}

/// Three survey points, two of them close enough for the cartographic engine to choose different sides.
fn survey_project(workspace: &str, work: &str, style: serde_json::Value) -> App {
    use serde_json::json;
    let mut drawing: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/interaction/v1/empty.kcad"
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
fn survey_names_keep_their_pixel_offset_during_zoom_and_pan() {
    use crate::viewport::Event as View;
    use iced::{Point, Rectangle, Vector};
    use kentos_interaction::spatial::LabelSpot;
    use serde_json::json;

    for style in [
        serde_json::Value::Null,
        json!({"placement":"beside","size":10.5,"minScale":2.2}),
    ] {
        let mut app = survey_project("cad", "map", style);
        let _ = app.update(Message::Viewport(View::Resized(Rectangle {
            x: 0.,
            y: 0.,
            width: 1000.,
            height: 650.,
        })));
        app.viewport.camera.scale = 30.;
        let read = |app: &App| {
            drop(app.view());
            let spots = app
                .label_spots
                .last()
                .expect("model view placed its labels");
            let mut out = Vec::new();
            for (spot, _) in spots.placed.iter() {
                if let LabelSpot::Placed {
                    slot,
                    at,
                    width,
                    height,
                    ..
                } = spot
                {
                    let Some(Entity::Point(p)) = app.document.as_ref().unwrap().model.get(*slot)
                    else {
                        unreachable!()
                    };
                    let scale = app.viewport.camera.scale;
                    let dx = (at.x - p.p.x) * scale - width / 2.;
                    let dy = (at.y - p.p.y) * scale - height / 2.;
                    assert!(dx > 0. && dy > 0., "slot {slot:?} ({dx}, {dy})");
                    out.push((slot.0, dx, dy));
                }
            }
            out.sort_by_key(|r| r.0);
            assert!(out.len() >= 2, "{out:?}");
            out
        };
        let first = read(&app);
        assert_eq!(first.len(), 2, "overlapping names stay readable");
        let (_, gap_x, gap_y) = first[0];
        let at = Point::new(510., 330.);
        for event in [
            View::Zoomed { factor: 1.1, at },
            View::Panned {
                by: Vector::new(11., -8.),
                at,
            },
            View::Zoomed { factor: 0.95, at },
            View::Settled,
        ] {
            let _ = app.update(Message::Viewport(event));
            for (_, x, y) in read(&app) {
                assert!(
                    (x - gap_x).abs() < 1e-5 && (y - gap_y).abs() < 1e-5,
                    "{x},{y} vs {gap_x},{gap_y}"
                );
            }
        }
        // Zoom around the close pair so both fit without changing either one's side.
        let _ = app.update(Message::Viewport(View::CenterOn(
            kentos_interaction::Vec2::new(487000., 4420000.),
        )));
        let _ = app.update(Message::Viewport(View::Zoomed {
            factor: 12.,
            at: Point::new(500., 325.),
        }));
        let close = read(&app);
        assert!(close.iter().any(|r| r.0 == 1) && close.iter().any(|r| r.0 == 2));
        assert!(
            close
                .iter()
                .all(|(_, x, y)| (x - gap_x).abs() < 1e-5 && (y - gap_y).abs() < 1e-5)
        );
    }

    for (workspace, work) in [("gis", ""), ("cad", "architecture"), ("cad", "road")] {
        assert!(
            !survey_project(workspace, work, serde_json::Value::Null)
                .label_engine()
                .options
                .fixed_survey_names
        );
    }
    let mut app = survey_project(
        "cad",
        "map",
        json!({"placement":"beside","point":"center","size":10.5}),
    );
    let _ = app.update(Message::Viewport(View::Resized(Rectangle {
        x: 0.,
        y: 0.,
        width: 1000.,
        height: 650.,
    })));
    drop(app.view());
    let spots = app.label_spots.last().unwrap();
    assert!(spots.placed.iter().any(|(spot, _)| {
        matches!(spot, LabelSpot::Placed { slot: Slot(3), at, .. }
            if (at.x - 487010.).abs() < 1e-6 && (at.y - 4420003.).abs() < 1e-6)
    }));
}

#[test]
fn survey_names_from_import_can_be_cleared_without_changing_unrelated_attributes() {
    use super::edit::{cell_text, write_cell};
    let app = survey_project("cad", "map", serde_json::Value::Null);
    let mut old = app.document.unwrap().model.to_snapshot();
    old.entities[0].base_mut().label = None;
    old.entities[1]
        .base_mut()
        .attrs
        .insert("Ad".into(), "Parsel kapısı".into());
    let mut doc = kentos_domain::Document::from_snapshot(old).unwrap();
    let Some(Entity::Point(p)) = doc.get(Slot(1)) else {
        panic!("first point")
    };
    assert_eq!(cell_text(p, EditColumn::Name, 1., true), "P101");
    assert!(
        write_cell(&mut doc, Slot(1), EditColumn::Name, "", false)
            .step
            .is_some()
    );
    assert!(!doc.get(Slot(1)).unwrap().base().attrs.contains_key("Ad"));
    assert!(
        write_cell(&mut doc, Slot(2), EditColumn::Name, "S102", false)
            .step
            .is_some()
    );
    let b = doc.get(Slot(2)).unwrap().base();
    assert_eq!(b.label.as_deref(), Some("S102"));
    assert_eq!(b.attrs["Ad"], "Parsel kapısı");
}

#[test]
fn shared_inspector_attributes_keep_imported_point_names_in_sync_and_preserve_independent_values() {
    use crate::properties::{Event as Property, Field};
    for (workspace, work) in [("cad", "map"), ("gis", ""), ("cad", "architecture")] {
        let mut app = survey_project(workspace, work, serde_json::Value::Null);
        let mut snapshot = app.document.as_ref().unwrap().model.to_snapshot();
        snapshot.entities[1]
            .base_mut()
            .attrs
            .insert("Ad".into(), "Bağımsız saha açıklaması".into());
        app.show_document(crate::document::Document::new(snapshot, None).unwrap());
        app.selection.set([Slot(1), Slot(2)]);
        let field = Field::Attributes(app.selection.ids().to_vec(), "Ad".into());
        app.properties_event(Property::Commit(field, "Yeni ad".into()));
        let doc = &app.document.as_ref().unwrap().model;
        let first = doc.get(Slot(1)).unwrap().base();
        let second = doc.get(Slot(2)).unwrap().base();
        assert_eq!(
            first.label.as_deref(),
            Some(if work == "map" { "Yeni ad" } else { "P101" })
        );
        assert_eq!(first.attrs["Ad"], "Yeni ad");
        assert_eq!(second.label.as_deref(), Some("P102"));
        assert_eq!(second.attrs["Ad"], "Yeni ad");
        let _ = app.update(Message::Run("edit.undo"));
        let doc = &app.document.as_ref().unwrap().model;
        assert_eq!(doc.get(Slot(1)).unwrap().base().attrs["Ad"], "P101");
        assert_eq!(
            doc.get(Slot(1)).unwrap().base().label.as_deref(),
            Some("P101")
        );
        assert_eq!(
            doc.get(Slot(2)).unwrap().base().attrs["Ad"],
            "Bağımsız saha açıklaması"
        );
    }
}

/// UR2-015: a computed elevation shows at the project's precision in Öznitelikler; an edit starts from the stored
/// figure; opening the cell and leaving it unchanged writes nothing, so neither the value nor the undo history moves.
#[test]
fn a_points_elevation_shows_rounded_and_edits_from_its_stored_figure() {
    use iced::{Point, Size, keyboard::key::Named};
    use kentos_ui::snapshot::{Input, Snapshot};

    const STORED: f64 = 103.827_311_354_000_61;
    let mut app = survey_project("cad", "map", serde_json::Value::Null);
    let mut survey = app.document.as_ref().unwrap().model.to_snapshot();
    let Entity::Point(point) = &mut survey.entities[1] else {
        unreachable!()
    };
    point.z = Some(STORED);
    app.show_document(crate::document::Document::new(survey, None).unwrap());
    app.selection.set([Slot(2)]);
    let mut snap = Snapshot::new(Size::new(1440., 900.)).unwrap();
    fn update(app: &mut App, message: Message) {
        let _ = app.update(message);
    }
    snap.settle(&mut app, App::view, &mut update);
    let shown = app.format().length_bare(STORED);
    assert!(shown.len() < 10, "rounded: {shown}");
    crate::files_testing::evidence(&mut snap, &app, "ur2-015", "kot");
    assert!(!crate::files_testing::find_texts(&mut snap, &app, &shown).is_empty(), "{shown} at rest");
    assert!(crate::files_testing::find_texts(&mut snap, &app, "103.82731135400061").is_empty());
    let revision = app.document.as_ref().unwrap().model.revision().to_string();
    let label = crate::files_testing::find_texts(&mut snap, &app, "Kot (Z)")
        .into_iter()
        .max_by(|a, b| a.x.total_cmp(&b.x))
        .expect("the row");
    snap.input(&mut app, App::view, &mut update, Input::Click(Point::new(label.x + 214., label.center().y)));
    snap.input(&mut app, App::view, &mut update, Input::Key(Named::Enter));
    let doc = &app.document.as_ref().unwrap().model;
    assert_eq!(doc.revision().to_string(), revision, "nothing written");
    let Some(Entity::Point(p)) = doc.get(Slot(2)) else { unreachable!() };
    assert_eq!(p.z, Some(STORED), "the stored figure kept");
}
