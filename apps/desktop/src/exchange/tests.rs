//! The exchange windows driven as the user drives them, without a window:
//! the file comes from `Picker::File` (the trace player's picker), the app's
//! own tasks run to their end (`drive`), and the files are the shared
//! fixtures the readers are tested with (fixtures/formats/v1).

use std::path::PathBuf;

use kentos_contracts::{Entity, LayerNodeType};

use super::{Event, Window, coord_export, coord_import, dxf_export, dxf_import};
use crate::app::{App, Dialog, Message, Picker};
use crate::files_testing::{app_with_drawing, drive, last_said, scratch};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/formats/v1")
        .join(name)
}

fn send(app: &mut App, event: Event) {
    let task = app.update(Message::Exchange(Box::new(event)));
    drive(app, task);
}

fn run(app: &mut App, id: &'static str) {
    let task = app.run(id);
    drive(app, task);
}

/// Whether the command line said a line starting so.
fn said(app: &App, start: &str) -> bool {
    use kentos_ui::widget::command_line::Entry;
    app.history.iter().any(|e| {
        matches!(e, Entry::Output(t) | Entry::Warning(t) | Entry::Error(t) if t.starts_with(start))
    })
}

fn count(app: &App) -> usize {
    app.document.as_ref().expect("open").model.len()
}

#[test]
fn a_dxf_goes_in_as_one_undo_step_with_its_layers_in_a_group_named_after_the_file() {
    let mut app = app_with_drawing();
    app.picker = Picker::File(fixture("entities.dxf"));
    let before = count(&app);
    run(&mut app, "file.import.dxf");
    assert_eq!(app.dialog, Some(Dialog::Exchange));
    let Some(Window::DxfImport(state)) = &app.exchange else {
        panic!("the DXF window");
    };
    assert!(
        format!("{state:?}").contains("reading: false"),
        "the file was read"
    );

    send(&mut app, Event::DxfImport(dxf_import::Event::Run));
    assert_eq!(app.dialog, None, "the window closes after the import");
    let model = &app.document.as_ref().expect("open").model;
    let added = model.len() - before;
    assert!(added > 0);
    let group = model
        .layers()
        .nodes()
        .iter()
        .find(|n| n.name == "entities.dxf")
        .expect("a group named after the file");
    assert_eq!(group.kind, LayerNodeType::Group);
    assert!(!group.children.is_empty());
    assert!(last_said(&app).contains("nesne"), "{}", last_said(&app));

    // One undo step takes every object back; the layers stay (as on the web).
    let _ = app.update(Message::Run("edit.undo"));
    let model = &app.document.as_ref().expect("open").model;
    assert_eq!(model.len(), before);
    assert!(
        model
            .layers()
            .nodes()
            .iter()
            .any(|n| n.name == "entities.dxf")
    );
}

#[test]
fn another_coordinate_system_blocks_the_import_and_nothing_changes() {
    let mut app = app_with_drawing();
    app.picker = Picker::File(fixture("entities.dxf"));
    let before = count(&app);
    run(&mut app, "file.import.dxf");
    // ED50 / TM36 while the project is TUREF / TM36: a datum transformation that does not exist yet.
    send(&mut app, Event::DxfImport(dxf_import::Event::Crs(2322)));
    send(&mut app, Event::DxfImport(dxf_import::Event::Run));
    assert_eq!(app.dialog, Some(Dialog::Exchange), "the window stays");
    assert_eq!(count(&app), before);
    // Back to the project's system, and it goes in.
    send(&mut app, Event::DxfImport(dxf_import::Event::Crs(5256)));
    send(&mut app, Event::DxfImport(dxf_import::Event::Run));
    assert!(count(&app) > before);
}

#[test]
fn layers_left_out_bring_nothing_and_none_chosen_imports_nothing() {
    let mut app = app_with_drawing();
    app.picker = Picker::File(fixture("entities.dxf"));
    let before = count(&app);
    run(&mut app, "file.import.dxf");
    send(&mut app, Event::DxfImport(dxf_import::Event::ToggleAll));
    send(&mut app, Event::DxfImport(dxf_import::Event::Run));
    assert_eq!(count(&app), before, "every layer left out");
    assert_eq!(app.dialog, Some(Dialog::Exchange));
}

#[test]
fn a_coordinate_list_goes_to_a_new_point_layer_named_after_the_file() {
    let mut app = app_with_drawing();
    app.picker = Picker::File(fixture("netcad.ncn"));
    let before = count(&app);
    run(&mut app, "file.import.ncn");
    send(&mut app, Event::CoordImport(coord_import::Event::Run));
    assert_eq!(app.dialog, None, "{:?}", app.exchange);
    let model = &app.document.as_ref().expect("open").model;
    assert_eq!(model.len() - before, 4);
    let layer = model
        .layers()
        .leaves()
        .into_iter()
        .find(|l| l.name == "netcad")
        .expect("a layer named after the file")
        .clone();
    assert_eq!(layer.style.color, "ink");
    assert!(layer.style.point.is_some() && layer.style.label.is_some());
    // Coordinates exactly as written: Y is east (x), X north (y).
    let first = model.by_layer(&layer.id).next().expect("a point").clone();
    let Entity::Point(p) = first else {
        panic!("a point")
    };
    assert_eq!(
        (p.p.x, p.p.y, p.z),
        (452_345.123, 4_412_345.678, Some(105.2))
    );
    assert_eq!(p.base.label.as_deref(), Some("1001"));
    assert_eq!(
        last_said(&app),
        "“netcad.ncn”: 4 nokta “netcad” katmanına alındı (yeni katman). Tek adımda geri alınabilir."
    );
}

#[test]
fn a_coordinate_list_merges_into_the_layer_of_the_same_name() {
    let mut app = app_with_drawing();
    app.picker = Picker::File(fixture("netcad.ncn"));
    run(&mut app, "file.import.ncn");
    send(
        &mut app,
        Event::CoordImport(coord_import::Event::Name("çizim".into())),
    );
    send(&mut app, Event::CoordImport(coord_import::Event::Run));
    let model = &app.document.as_ref().expect("open").model;
    // The sample drawing's “Çizim” layer, found without regard to case or Turkish marks.
    assert!(model.layers().leaves().iter().all(|l| l.name != "çizim"));
    assert!(
        last_said(&app).contains("“Çizim” katmanına alındı"),
        "{}",
        last_said(&app)
    );
}

#[test]
fn a_dxf_export_writes_what_the_reader_reads_back() {
    let mut app = app_with_drawing();
    let dir = scratch("export-dxf");
    let path = dir.join("cizim.dxf");
    app.picker = Picker::File(path.clone());
    run(&mut app, "file.export.dxf");
    send(
        &mut app,
        Event::DxfExport(dxf_export::Event::Scope(dxf_export::Scope::All)),
    );
    send(&mut app, Event::DxfExport(dxf_export::Event::Run));
    assert_eq!(app.dialog, None);
    // The writer's notes follow the line that says the file was written.
    assert!(said(&app, "“cizim.dxf” yazıldı:"), "{:?}", app.history);
    let bytes = std::fs::read(&path).expect("written");
    let back = kentos_formats::dxf::read(&bytes, &Default::default()).expect("reads");
    assert_eq!(back.entities.len(), count(&app));
    assert!(
        !app.document.as_ref().expect("open").model.is_dirty(),
        "an export is not an edit"
    );
}

#[test]
fn a_coordinate_list_export_writes_every_point_exactly() {
    let mut app = app_with_drawing();
    let dir = scratch("export-ncn");
    let path = dir.join("noktalar.ncn");
    app.picker = Picker::File(path.clone());
    run(&mut app, "file.export.ncn");
    send(
        &mut app,
        Event::CoordExport(coord_export::Event::Scope(dxf_export::Scope::All)),
    );
    send(&mut app, Event::CoordExport(coord_export::Event::Run));
    let text = std::fs::read_to_string(&path).expect("written");
    let model = &app.document.as_ref().expect("open").model;
    let points: Vec<_> = model
        .entities()
        .filter_map(|e| match e {
            Entity::Point(p) => Some(p),
            _ => None,
        })
        .collect();
    assert_eq!(text.lines().count(), points.len());
    let back = kentos_formats::coords::read(
        text.as_bytes(),
        &kentos_contracts::CoordReadOptions {
            entities: true,
            ..Default::default()
        },
    );
    let result = back.result.expect("objects");
    for (written, read) in points.iter().zip(&result.entities) {
        let Entity::Point(read) = read else {
            panic!("a point")
        };
        assert_eq!((written.p.x, written.p.y), (read.p.x, read.p.y));
    }
    assert!(last_said(&app).contains("nokta."), "{}", last_said(&app));
}

/// Pictures of the four windows for the owner, in the dark and the light
/// theme, written to `.run/shots` (never committed); not run by default:
///
/// ```text
/// cargo test -p kentos-desktop exchange::tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let picture = |app: &mut App, name: &str| {
        let mut snapshot = Snapshot::new(Size::new(1440.0, 900.0)).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        snapshot.settle(app, App::view, &mut update);
        let _ = snapshot.render(app.view(), &app.theme());
        let file = out.join(format!("{name}.png"));
        snapshot
            .render(app.view(), &app.theme())
            .save(&file)
            .expect("writes the picture");
        println!("{}", file.display());
    };
    for mode in ["dark", "light"] {
        let fresh = || {
            let mut app = app_with_drawing();
            let _ = app
                .settings
                .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
            app.apply_settings();
            app
        };
        let mut app = fresh();
        app.picker = Picker::File(fixture("entities.dxf"));
        run(&mut app, "file.import.dxf");
        picture(&mut app, &format!("aktar-{mode}-1-dxf"));
        send(&mut app, Event::DxfImport(dxf_import::Event::Crs(2322)));
        picture(&mut app, &format!("aktar-{mode}-2-dxf-baska-sistem"));

        let mut app = fresh();
        app.picker = Picker::File(fixture("netcad.ncn"));
        run(&mut app, "file.import.ncn");
        picture(&mut app, &format!("aktar-{mode}-3-koordinat"));
        send(&mut app, Event::CoordImport(coord_import::Event::Run));
        picture(&mut app, &format!("aktar-{mode}-4-koordinat-alindi"));

        let mut app = fresh();
        run(&mut app, "file.export.dxf");
        picture(&mut app, &format!("aktar-{mode}-5-dxf-ver"));
        let mut app = fresh();
        run(&mut app, "file.export.ncn");
        picture(&mut app, &format!("aktar-{mode}-6-koordinat-ver"));
    }
}
