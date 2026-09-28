//! The exchange windows driven as the user drives them, without a window:
//! the file comes from `Picker::File` (the trace player's picker), the app's
//! own tasks run to their end (`drive`), and the files are the shared
//! fixtures the readers are tested with (fixtures/formats/v1).

use std::path::PathBuf;

use kentos_contracts::{Entity, LayerNodeType};

use super::{Event, Window, coord_export, coord_import, drawing_import, dxf_export};
use crate::app::{App, Dialog, Message, Picker};
use crate::files_testing::{app_with_drawing, drive, last_said, scratch};

pub(super) fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/formats/v1")
        .join(name)
}

pub(super) fn send(app: &mut App, event: Event) {
    let task = app.update(Message::Exchange(Box::new(event)));
    drive(app, task);
}

pub(super) fn run(app: &mut App, id: &'static str) {
    let task = app.run(id);
    drive(app, task);
}

/// Whether the command line said a line starting so.
pub(super) fn said(app: &App, start: &str) -> bool {
    app.log
        .lines()
        .any(|l| l.level != kentos_interaction::Level::Command && l.text.starts_with(start))
}

pub(super) fn count(app: &App) -> usize {
    app.document.as_ref().expect("open").model.len()
}

#[test]
fn a_dxf_goes_in_as_one_undo_step_with_its_layers_in_a_group_named_after_the_file() {
    let mut app = app_with_drawing();
    app.picker = Picker::File(fixture("entities.dxf"));
    let before = count(&app);
    run(&mut app, "file.import.dxf");
    assert_eq!(app.dialog, Some(Dialog::Exchange));
    let Some(Window::DrawingImport(state)) = &app.exchange else {
        panic!("the DXF window");
    };
    assert!(
        format!("{state:?}").contains("reading: None"),
        "the file was read"
    );

    send(&mut app, Event::DrawingImport(drawing_import::Event::Run));
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

    // One undo step takes every object back, and the layers made for them
    // (as on the web since bdaed77; docs/adr/0076).
    let _ = app.update(Message::Run("edit.undo"));
    let model = &app.document.as_ref().expect("open").model;
    assert_eq!(model.len(), before);
    assert!(
        !model
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
    send(&mut app, Event::DrawingImport(drawing_import::Event::Crs(2322)));
    send(&mut app, Event::DrawingImport(drawing_import::Event::Run));
    assert_eq!(app.dialog, Some(Dialog::Exchange), "the window stays");
    assert_eq!(count(&app), before);
    // Back to the project's system, and it goes in.
    send(&mut app, Event::DrawingImport(drawing_import::Event::Crs(5256)));
    send(&mut app, Event::DrawingImport(drawing_import::Event::Run));
    assert!(count(&app) > before);
}

#[test]
fn layers_left_out_bring_nothing_and_none_chosen_imports_nothing() {
    let mut app = app_with_drawing();
    app.picker = Picker::File(fixture("entities.dxf"));
    let before = count(&app);
    run(&mut app, "file.import.dxf");
    send(&mut app, Event::DrawingImport(drawing_import::Event::ToggleAll));
    send(&mut app, Event::DrawingImport(drawing_import::Event::Run));
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
    assert!(
        said(&app, "“cizim.dxf” yazıldı:"),
        "{:?}",
        app.log.lines().collect::<Vec<_>>()
    );
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
/// theme, at 1440×900 and at the smallest window (1100×650), written to
/// `.run/shots` (never committed); not run by default:
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
    for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
        let picture = |app: &mut App, name: &str| {
            let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
            let mut update = |app: &mut App, message| {
                let _ = app.update(message);
            };
            snapshot.settle(app, App::view, &mut update);
            let _ = snapshot.render(app.view(), &app.theme());
            let file = out.join(format!("{name}-{width}x{height}.png"));
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
            send(&mut app, Event::DrawingImport(drawing_import::Event::Crs(2322)));
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
}

/// A DXF of `n` lines on one layer, 1 m apart: large enough to go in a frame at a time.
fn many_lines(n: usize) -> String {
    let mut s = String::from("0\nSECTION\n2\nENTITIES\n");
    for i in 0..n {
        let y = 4_400_000.0 + i as f64;
        s.push_str(&format!("0\nLINE\n8\nCIZGI\n10\n500000\n20\n{y}\n11\n500010\n21\n{y}\n"));
    }
    s.push_str("0\nENDSEC\n0\nEOF\n");
    s
}

fn frame(app: &mut App) {
    send(app, Event::DrawingImport(drawing_import::Event::Frame));
}

#[test]
fn an_ncz_goes_in_with_its_smart_objects_drawn_as_symbols() {
    let mut app = app_with_drawing();
    app.picker = Picker::File(fixture("ncz/07-akilli-nesneler.ncz"));
    let before = count(&app);
    run(&mut app, "file.import.ncz");
    assert_eq!(app.dialog, Some(Dialog::Exchange));
    send(&mut app, Event::DrawingImport(drawing_import::Event::Run));
    assert_eq!(app.dialog, None, "{:?}", app.exchange);
    let model = &app.document.as_ref().expect("open").model;
    assert_eq!(model.len() - before, 41, "ten symbols drawn, an unknown class a point");
    let group = model
        .layers()
        .nodes()
        .iter()
        .find(|n| n.name == "07-akilli-nesneler.ncz")
        .expect("a group named after the file");
    let names: Vec<&str> = group.children.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["SM_YERLESIM", "SM_YAPILASMA", "SM_YOL", "SM_NOT", "SM_FONKADI"]);
    // The settlement symbol's circle carries its values.
    let layer = &group.children[0].id;
    let circle = model
        .by_layer(layer)
        .find(|e| matches!(e, Entity::Circle(_)))
        .expect("a circle");
    let attrs = &circle.base().attrs;
    assert_eq!(attrs.get("Akıllı nesne").map(String::as_str), Some("Yerleşim"));
    assert_eq!(attrs.get("Nizam").map(String::as_str), Some("AYRIK"));
    assert_eq!(attrs.get("Ön bahçe").map(String::as_str), Some("5"));
    assert!(last_said(&app).contains("41 nesne"), "{}", last_said(&app));
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(count(&app), before, "one undo step");
}

#[test]
fn an_ncz_in_another_zone_waits_for_the_user_to_say_its_system() {
    let mut app = app_with_drawing();
    app.picker = Picker::File(fixture("ncz/01-her-tur.ncz"));
    let before = count(&app);
    run(&mut app, "file.import.ncz");
    // The file says ITRF, 3°, meridian 39: TUREF / TM39, while the project is TM36.
    send(&mut app, Event::DrawingImport(drawing_import::Event::Run));
    assert_eq!(count(&app), before, "nothing goes in");
    assert_eq!(app.dialog, Some(Dialog::Exchange));
    let project = app.document.as_ref().expect("open").settings().srid;
    send(&mut app, Event::DrawingImport(drawing_import::Event::Crs(project)));
    send(&mut app, Event::DrawingImport(drawing_import::Event::Run));
    assert_eq!(count(&app) - before, 18);
}

#[test]
fn a_large_import_goes_in_a_frame_at_a_time_as_one_step_and_durdur_takes_it_back() {
    let n = drawing_import::AT_ONCE + 10_000;
    let dir = scratch("buyuk-ice-aktarma");
    let path = dir.join("cizgiler.dxf");
    std::fs::write(&path, many_lines(n)).expect("written");
    let mut app = app_with_drawing();
    app.picker = Picker::File(path.clone());
    let before = count(&app);
    assert!(!app.document.as_ref().expect("open").model.can_undo(), "a fresh drawing");

    run(&mut app, "file.import.dxf");
    send(&mut app, Event::DrawingImport(drawing_import::Event::Run));
    assert!(app.importing.is_some(), "a large file goes in a frame at a time");
    assert_eq!(app.dialog, None, "the window gives way to the panel");
    // Meanwhile nothing else edits the drawing: the command waits.
    let _ = app.update(Message::Run("edit.undo"));
    assert!(app.importing.is_some());
    let mut frames = 0;
    while app.importing.is_some() {
        frame(&mut app);
        frames += 1;
        assert!(frames < 10_000, "the import ends");
    }
    assert!(frames > 1, "more than one frame");
    assert_eq!(count(&app) - before, n);
    // One undo step takes every object and the layers made for them.
    let model = &mut app.document.as_mut().expect("open").model;
    assert_eq!(model.undo().as_deref(), Some("DXF: cizgiler.dxf"));
    assert!(!model.can_undo(), "one undo step");
    assert_eq!(count(&app), before);
    let model = &app.document.as_ref().expect("open").model;
    assert!(!model.layers().nodes().iter().any(|n| n.name == "cizgiler.dxf"));

    // Durdur half way: every object and layer it made goes, and no step is left.
    app.picker = Picker::File(path);
    run(&mut app, "file.import.dxf");
    send(&mut app, Event::DrawingImport(drawing_import::Event::Run));
    frame(&mut app);
    assert!(count(&app) > before, "some went in");
    send(&mut app, Event::DrawingImport(drawing_import::Event::Stop));
    assert!(app.importing.is_none());
    assert_eq!(count(&app), before);
    let model = &app.document.as_ref().expect("open").model;
    assert!(!model.can_undo(), "no step recorded");
    assert!(!model.layers().nodes().iter().any(|n| n.name == "cizgiler.dxf"));
    assert!(said(&app, "“cizgiler.dxf” içe aktarılması durduruldu"));
    let _ = std::fs::remove_dir_all(&dir);
}

/// A real Netcad NCZ imported as the user imports it, with pictures of the
/// window, of the drawing filling in and of the plan when it is in, and each
/// frame of the writing timed (the UI thread: the app's slice, the view, the
/// picture). Needs the file, which is not in the repository:
///
/// ```text
/// KENTOS_NCZ=/path/plan.ncz KENTOS_SNAPSHOT_BACKEND=wgpu \
///   cargo test --release -p kentos-desktop exchange::tests::ncz_screens -- --ignored --nocapture
/// ```
///
/// `KENTOS_NCZ_AT=easting,northing` also takes a close-up 300 m across there.
#[test]
#[ignore = "pictures and timings for the owner, with a real file, run by hand"]
fn ncz_screens() {
    use std::time::Instant;

    use iced::Size;
    use kentos_render_wgpu::Bounds;
    use kentos_ui::snapshot::Snapshot;

    let Ok(path) = std::env::var("KENTOS_NCZ") else {
        eprintln!("KENTOS_NCZ yok: bir .ncz dosyasının yolunu verin");
        return;
    };
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let size = Size::new(1440.0, 900.0);
    let mut snapshot = Snapshot::new(size).expect("a renderer");
    let mut app = app_with_drawing();
    app.picker = Picker::File(PathBuf::from(&path));
    fn shot(snapshot: &mut Snapshot, out: &std::path::Path, app: &mut App, name: &str) {
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        snapshot.settle(app, App::view, &mut update);
        let file = out.join(format!("{name}.png"));
        snapshot
            .render(app.view(), &app.theme())
            .save(&file)
            .expect("writes the picture");
        println!("{}", file.display());
    }
    let t = Instant::now();
    run(&mut app, "file.import.ncz");
    println!("okuma: {:.0} ms", t.elapsed().as_secs_f64() * 1000.0);
    shot(&mut snapshot, &out, &mut app, "ncz-1-pencere");
    send(&mut app, Event::DrawingImport(drawing_import::Event::Run));
    assert!(app.importing.is_some(), "a large file");
    let (mut frames, mut worst, mut total, mut halfway) = (0usize, 0.0f64, 0.0f64, false);
    let ms = |t: Instant| t.elapsed().as_secs_f64() * 1000.0;
    while app.importing.is_some() {
        let t = Instant::now();
        frame(&mut app);
        let a = ms(t);
        let t1 = Instant::now();
        let element = app.view();
        let v = ms(t1);
        let t2 = Instant::now();
        let _ = snapshot.render(element, &app.theme());
        let r = ms(t2);
        let all = ms(t);
        if all > 0.0 {
            println!("  kare {frames}: uygulama {a:.0} ms, görünüm {v:.0} ms, çizim {r:.0} ms");
        }
        worst = worst.max(all);
        total += all;
        frames += 1;
        if !halfway && frames >= 12 && app.importing.is_some() {
            halfway = true;
            shot(&mut snapshot, &out, &mut app, "ncz-2-yazilirken");
        }
    }
    println!(
        "yazma: {frames} kare, kare başına ort. {:.1} ms, en çok {worst:.1} ms, toplam {total:.0} ms",
        total / frames as f64
    );
    println!("{}", last_said(&app));
    shot(&mut snapshot, &out, &mut app, "ncz-3-bitti");
    if let Ok(at) = std::env::var("KENTOS_NCZ_AT") {
        let v: Vec<f64> = at.split(',').filter_map(|s| s.trim().parse().ok()).collect();
        if let [x, y] = v[..] {
            app.viewport.show(&Bounds {
                min_x: x - 150.0,
                min_y: y - 95.0,
                max_x: x + 150.0,
                max_y: y + 95.0,
            });
            shot(&mut snapshot, &out, &mut app, "ncz-4-semboller");
        }
    }
}
