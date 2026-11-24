//! Öznitelik tablosu's rules and the tab over `fixtures/interaction/v1/feature-table.kcad`
//! (docs/adr/0199 §4); the shared trace `feature-table.json` plays it through the shell.

use super::{Event, next_sort};
use crate::app::{App, Message};
use crate::gis;

fn app_with_table() -> App {
    let (mut app, _) = App::boot(None);
    let snapshot = kentos_commands::DocumentSnapshotV1::from_json(include_str!(
        "../../../../../fixtures/interaction/v1/feature-table.kcad"
    ))
    .expect("the drawing reads");
    let doc = crate::document::Document::new(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app.gis.features.reset(None);
    app
}

fn ev(app: &mut App, e: Event) {
    let _ = app.update(Message::Gis(gis::Event::Features(e)));
}

#[test]
fn a_header_sorts_ascending_then_descending_then_the_drawings_order() {
    let (mut sort, mut desc) = (None, false);
    (sort, desc) = next_sort(&sort, desc, "Parsel");
    assert_eq!((sort.as_deref(), desc), (Some("Parsel"), false));
    (sort, desc) = next_sort(&sort, desc, "Parsel");
    assert_eq!((sort.as_deref(), desc), (Some("Parsel"), true));
    (sort, desc) = next_sort(&sort, desc, "Parsel");
    assert_eq!((sort.as_deref(), desc), (None, false));
}

/// Öznitelikler's fields beside the Tablo tab, dark and light, at 1440×900
/// and 1100×650 (`.run/shots/oznitelik-alanlari-*`): the parcel whose Parsel
/// is “7a” selected, Genel and Geometri folded, its fields by their kinds
/// (a value list and yes or no as lists, a date and numbers in boxes, the
/// refused value in the warning colour):
///
/// ```text
/// cargo test -p kentos-desktop features::tests::screens -- --ignored --nocapture
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
            let mut app = app_with_table();
            let _ = app
                .settings
                .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
            app.apply_settings();
            let _ = app.update(Message::Run("data.featureTable"));
            app.selection.set([kentos_domain::Slot(4)]);
            app.props_closed.insert("general");
            app.props_closed.insert("geometry");
            let _ = app.update(Message::WindowResized(Size::new(width, height)));
            let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
            let mut update = |app: &mut App, message| {
                let _ = app.update(message);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            let file = out.join(format!("oznitelik-alanlari-{width}x{height}{suffix}.png"));
            snapshot
                .render(app.view(), &app.theme())
                .save(&file)
                .expect("writes the picture");
            println!("{}", file.display());
        }
    }
}

/// Sütunlar ▾ with a column hidden and the rows' copy and file, and İstatistik and Grafik over the table's rows,
/// dark and light (`.run/shots/oznitelik-sutun-*`, `oznitelik-istatistik-*`, `oznitelik-grafik-*`):
///
/// ```text
/// cargo test -p kentos-desktop features::tests::outgoing_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn outgoing_screens() {
    use super::outgoing::{Analyse, ColumnsEvent};
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            for scene in ["sutun", "istatistik", "grafik"] {
                let mut app = app_with_secret();
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                let _ = app.update(Message::WindowResized(Size::new(width, height)));
                ev(&mut app, Event::Columns(ColumnsEvent::Toggle("Ruhsat".into())));
                ev(&mut app, Event::Search("102".into()));
                match scene {
                    "istatistik" => {
                        ev(&mut app, Event::Analyse(Analyse::Statistics));
                        // The summaries are counted as frames come: count them all for the picture.
                        let _ = app.statistics_seen();
                    }
                    "grafik" => ev(&mut app, Event::Analyse(Analyse::Chart)),
                    _ => {}
                }
                let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                let mut update = |app: &mut App, message| {
                    let _ = app.update(message);
                };
                snapshot.settle(&mut app, App::view, &mut update);
                let file = out.join(format!("oznitelik-{scene}-{width}x{height}{suffix}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
}

fn write(app: &mut App, e: super::write::WriteEvent) {
    ev(app, Event::Write(e));
}

/// Alan hesapla's preview and a paste's problems, dark and light (`.run/shots/alan-hesapla-*`):
///
/// ```text
/// cargo test -p kentos-desktop features::tests::write_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn write_screens() {
    use super::write::{Scope, WriteEvent as W};
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for mode in ["dark", "light"] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            for scene in ["hesap", "yapistir"] {
                let mut app = app_with_table();
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                let _ = app.update(Message::Run("data.featureTable"));
                let _ = app.update(Message::WindowResized(Size::new(width, height)));
                if scene == "hesap" {
                    write(&mut app, W::Open);
                    write(&mut app, W::Field("Alan".into()));
                    write(&mut app, W::Expression("Alan / 3".into()));
                    write(&mut app, W::Scope(Scope::All));
                } else {
                    app.selection.set([kentos_domain::Slot(1)]);
                    write(
                        &mut app,
                        W::Pasted(Some("Parsel\tKullanım\n8\tTicaret\nx\tKonut\n".into())),
                    );
                }
                write(&mut app, W::Preview);
                let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                let mut update = |app: &mut App, message| {
                    let _ = app.update(message);
                };
                snapshot.settle(&mut app, App::view, &mut update);
                let file = out.join(format!("alan-hesapla-{mode}-{scene}-{width}x{height}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
}

/// The table's drawing with a form that marks the Tapu alanı field (`Alan`) `redact` (docs/adr/0443 §2).
fn app_with_secret() -> App {
    let mut value: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/interaction/v1/feature-table.kcad"
    ))
    .expect("the drawing is JSON");
    value["settings"]["forms"] = serde_json::json!([{
        "id": "parsel-formu", "name": "Parsel formu", "layer": "parsel",
        "draft": {"revision": 1, "items": [{"type": "field", "id": "alan", "field": "Alan", "redact": true}]}
    }]);
    let snapshot =
        kentos_commands::DocumentSnapshotV1::from_json(&value.to_string()).expect("the drawing reads");
    let (mut app, _) = App::boot(None);
    let doc = crate::document::Document::new(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app.gis.features.reset(None);
    let _ = app.update(Message::Run("data.featureTable"));
    app
}

/// [`app_with_kinds`]; `keep`: the parcels' own fields too (else only Ada beside the new ones, for the pictures).
fn kinds_app(keep: bool) -> App {
    use kentos_commands::{LayerField, LayerFieldKind};
    let mut app = app_with_table();
    let model = &mut app.document.as_mut().expect("a drawing").model;
    let slots: Vec<kentos_domain::Slot> = model
        .entities()
        .filter(|e| e.base().layer_id == "parsel")
        .map(|e| kentos_domain::Slot(e.base().id))
        .collect();
    let values = [
        ["9007199254740993", "2024-03-01T10:20:30.5+03:00", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0001", r#"{"kot":[1.25,12345678901234567890]}"#, "QUJD"],
        ["-9223372036854775808", "2024-03-01T07:20:30Z", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0002", "[]", ""],
        ["12", "2023-12-31T23:59:59.999999999-05:00", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0003", r#""ç""#, "a2VudG9zIGNhZA=="],
    ];
    for (slot, row) in slots.iter().zip(values) {
        let mut e = model.get(*slot).cloned().expect("a parcel");
        for (name, v) in ["Kimlik", "Ölçüm anı", "Cihaz", "Ek bilgi", "İmza"].into_iter().zip(row) {
            e.base_mut().attrs.insert(name.into(), v.into());
        }
        model.update(*slot, e);
    }
    let mut fields = model
        .layers()
        .get("parsel")
        .map(|n| n.fields.clone())
        .unwrap_or_default();
    if !keep {
        fields.retain(|f| f.name == "Ada");
    }
    fields.extend([
        LayerField::new("Kimlik", LayerFieldKind::BigInt),
        LayerField::new("Ölçüm anı", LayerFieldKind::Instant),
        LayerField::new("Cihaz", LayerFieldKind::Uuid),
        LayerField::new("Ek bilgi", LayerFieldKind::Json),
        LayerField::new("İmza", LayerFieldKind::Binary),
    ]);
    assert_eq!(model.set_layer_fields("parsel", fields, &[]), Ok(true));
    app.gis.features.reset(None);
    app
}

/// Sorts by the column headed `name` (no column hidden: the headers are the table's columns).
fn sort_by(app: &mut App, name: &str) {
    let (_, headers, _) = app.features_seen().expect("the tab is open");
    let j = headers.iter().position(|h| h == name).expect(name);
    ev(app, Event::Sort(j + 1));
}

/// The new kinds in the table and in Öznitelikler, dark and light, at 1440×900 and 1100×650 (`.run/shots/alan-turleri-*`):
///
/// ```text
/// cargo test -p kentos-desktop features::tests::kinds_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn kinds_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            let mut app = kinds_app(false);
            let _ = app
                .settings
                .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
            app.apply_settings();
            let _ = app.update(Message::Run("data.featureTable"));
            sort_by(&mut app, "Ölçüm anı");
            let first = app
                .document
                .as_ref()
                .and_then(|d| {
                    d.model
                        .entities()
                        .find(|e| e.base().attrs.get("Kimlik").map(String::as_str) == Some("9007199254740993"))
                })
                .map(|e| kentos_domain::Slot(e.base().id))
                .expect("the first parcel");
            app.selection.set([first]);
            app.props_closed.insert("general");
            app.props_closed.insert("geometry");
            let _ = app.update(Message::WindowResized(Size::new(width, height)));
            let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
            let mut update = |app: &mut App, message| {
                let _ = app.update(message);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            let file = out.join(format!("alan-turleri-{width}x{height}{suffix}.png"));
            snapshot
                .render(app.view(), &app.theme())
                .save(&file)
                .expect("writes the picture");
            println!("{}", file.display());
        }
    }
}

/// GIS-028: a locked layer's cell opens no editor; it is said at once, and the value stays.
#[test]
fn a_locked_layers_cell_opens_no_editor() {
    let mut app = app_with_table();
    let layer = app.gis.features.layer.clone().or_else(|| {
        app.document.as_ref().map(|d| d.model.layers().active().to_owned())
    });
    let layer = layer.expect("a layer");
    app.document.as_mut().unwrap().model.set_layer_locked(&layer, true);
    ev(&mut app, Event::Edit(0, 1));
    assert!(!app.gis.features.editing(), "no editor on a locked layer");
    assert!(crate::files_testing::last_said(&app).contains("kilitli"));
}

