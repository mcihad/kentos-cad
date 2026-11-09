//! Paketle's window on the desktop (docs/adr/0596 §5, 2780): with bound layers it asks how their sources go and which
//! links stay out, and always the open files (Devir dosyaları); its answer is the input `file.package` gets, and the
//! package made says the copies, the links embedded and the open files. Pictures: `paket-*` (`screens`, ignored),
//! `teslim-devir-2780` (`open_file_screens`, ignored).

use std::path::PathBuf;

use kentos_commands::{CallOp, PackageWay};
use kentos_native_application::{ExecutionContext, registry};

use super::package::{Event as P, Way};
use super::tests::{run, send};
use super::{Event, Window};
use crate::app::App;
use crate::files_testing::{app_with_drawing, scratch};

/// A drawing with two tables of a copy of qgis.gpkg bound, the roads' link made a database's.
fn scene(name: &str) -> App {
    let file = scratch(name).join("kadastro.gpkg");
    std::fs::copy(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/geopackage/v1/qgis.gpkg"),
        &file,
    )
    .expect("copied");
    let mut app = app_with_drawing();
    let doc = &mut app.document.as_mut().expect("a drawing").model;
    for table in ["parsel", "yol"] {
        let answer = registry::run(
            &mut ExecutionContext::new(doc),
            "cad.sources.edit",
            CallOp::Execute,
            serde_json::json!({ "edit": { "kind": "link", "path": file.display().to_string(), "table": table } }),
        )
        .expect("runs");
        assert_eq!(answer["status"], "completed", "{answer}");
    }
    let mut settings = doc.settings().clone();
    let roads = &mut settings.sources[1];
    roads.kind = kentos_commands::SourceKind::Postgis;
    roads.path = "postgresql://cbs@veri.belediye.local/kent".into();
    roads.table = "ulasim.yol".into();
    doc.set_settings(settings);
    app
}

fn window(app: &App) -> &super::package::State {
    match &app.exchange {
        Some(Window::Package(s)) => s,
        _ => panic!("Paketle's window"),
    }
}

#[test]
fn the_window_asks_the_ways_and_the_links_left_out() {
    let mut app = scene("paket-pencere");
    run(&mut app, "file.package");
    let s = window(&app);
    assert_eq!(
        s.links
            .iter()
            .map(|l| (l.id.as_str(), l.database, l.take))
            .collect::<Vec<_>>(),
        vec![("kaynak-1", false, true), ("kaynak-2", true, true)]
    );
    assert_eq!(
        (s.geopackages, s.databases),
        (PackageWay::Copy, PackageWay::Keep)
    );
    send(
        &mut app,
        Event::Package(P::Databases(Way(PackageWay::Embed))),
    );
    send(&mut app, Event::Package(P::Take(0)));
    let out = scratch("paket-pencere-cikti").join("teslim");
    let input = app.package_input(&out).expect("an input");
    assert_eq!(input.databases, Some(PackageWay::Embed));
    assert_eq!(input.geopackages, Some(PackageWay::Copy));
    assert_eq!(input.leave_out, vec!["kaynak-1".to_owned()]);
    // The package made with that input: the parcels left out, the roads embedded, nothing copied.
    let doc = &app.document.as_ref().expect("a drawing").model;
    let planned = kentos_headless::package::plan(doc, &input).expect("planned");
    kentos_headless::package::write(&planned).expect("packed");
    assert!(planned.output.files.is_empty());
    assert_eq!(planned.output.embedded.len(), 1);
    send(
        &mut app,
        Event::Package(P::Done(Ok(Box::new((planned.output, planned.warnings))))),
    );
    assert!(
        app.log
            .lines()
            .any(|l| l.text.contains("1 bağlı katman projeye gömüldü")),
        "{:?}",
        app.log.lines().map(|l| l.text.clone()).collect::<Vec<_>>()
    );
    assert!(
        app.log
            .lines()
            .any(|l| l.text.contains("dışarıda bırakıldı"))
    );
}

/// The exchange sample drawing (fixtures/exchange/v1/devir.json, docs/adr/1660) opened, no layer bound.
fn sample() -> App {
    let snapshot: kentos_commands::DocumentSnapshotV2 =
        serde_json::from_str(include_str!("../../../../fixtures/exchange/v1/devir.json")).expect("the sample");
    let (mut app, _) = App::boot(None);
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(crate::app::Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

/// docs/adr/2780: without bound layers the window asks only the open files; DXF and GeoPackage ticked go in the matrix's
/// order, the package made carries both files and the report, and its message names them.
#[test]
fn the_window_asks_the_open_files_without_bound_layers() {
    let mut app = sample();
    run(&mut app, "file.package");
    let s = window(&app);
    assert!(s.links.is_empty() && s.formats.is_empty(), "no link, no open file when it opens");
    send(&mut app, Event::Package(P::Format(kentos_commands::ExchangeFormat::Dxf)));
    send(&mut app, Event::Package(P::Format(kentos_commands::ExchangeFormat::Gpkg)));
    send(&mut app, Event::Package(P::Format(kentos_commands::ExchangeFormat::Fgb)));
    send(&mut app, Event::Package(P::Format(kentos_commands::ExchangeFormat::Fgb)));
    let out = scratch("paket-devir-pencere").join("teslim");
    let input = app.package_input(&out).expect("an input");
    let asked = input.exchange.clone().expect("the open files asked");
    assert_eq!(asked.formats, [kentos_commands::ExchangeFormat::Gpkg, kentos_commands::ExchangeFormat::Dxf]);
    let doc = &app.document.as_ref().expect("a drawing").model;
    let planned = kentos_headless::package::plan(doc, &input).expect("planned");
    kentos_headless::package::write(&planned).expect("packed");
    let made: Vec<&str> = planned
        .output
        .files
        .iter()
        .filter(|f| f.kind == kentos_commands::PackagedKind::Exchange)
        .map(|f| f.path.as_str())
        .collect();
    assert_eq!(made.len(), 3, "{made:?}");
    assert!(out.join("devir/devir-raporu.csv").is_file());
    let warned = planned.warnings.iter().any(|w| w.code == "export_losses");
    assert!(warned, "the open files lose the text outside DXF");
    send(&mut app, Event::Package(P::Done(Ok(Box::new((planned.output, planned.warnings))))));
    let said: Vec<String> = app.log.lines().map(|l| l.text.clone()).collect();
    assert!(
        said.iter().any(|l| l.contains("2 devir dosyası: GeoPackage, DXF; kayıpları devir/devir-raporu.csv'de")),
        "{said:?}"
    );
    assert!(said.iter().any(|l| l.contains("Devir dosyaları projenin her şeyini taşımaz")), "{said:?}");
}

/// docs/adr/3180: the package's message counts the grids it carries, not their notes.
#[test]
fn the_message_counts_the_grids() {
    let mut app = sample();
    run(&mut app, "file.package");
    let out = scratch("paket-izgara-ileti").join("teslim");
    let input = app.package_input(&out).expect("an input");
    let doc = &app.document.as_ref().expect("a drawing").model;
    let mut planned = kentos_headless::package::plan(doc, &input).expect("planned");
    for (path, links) in [("izgaralar/a.gtx", 1), ("izgaralar/a.json", 0), ("izgaralar/b.gsb", 2)] {
        planned.output.files.push(kentos_commands::PackagedFile {
            path: path.into(),
            from: path.into(),
            kind: kentos_commands::PackagedKind::Grid,
            bytes: 1,
            sha256: String::new(),
            links,
            table: None,
            rows: None,
        });
    }
    send(&mut app, Event::Package(P::Done(Ok(Box::new((planned.output, planned.warnings))))));
    let said: Vec<String> = app.log.lines().map(|l| l.text.clone()).collect();
    assert!(said.iter().any(|l| l.contains(", 2 ızgara")), "{said:?}");
}

/// The grids the project names in a package (docs/adr/3180); `.run/gui-evidence/paket-izgara-3180/`. Run alone: it
/// points the device's grid library (`KENTOS_GRID_DIR`) at a scratch folder holding the sample's geoid model.
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib exchange::package_tests::grid_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures, run by hand"]
fn grid_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/paket-izgara-3180");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let library = scratch("paket-izgara-kutuphane");
    let bytes = std::fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/geoid/v1/tg.gtx")).expect("the geoid");
    let id = kentos_commands::encoding::sha256_hex(&bytes);
    std::fs::write(library.join(format!("{id}.gtx")), &bytes).expect("in the library");
    // SAFETY: a picture test run by hand on its own; no other thread reads the variable.
    unsafe { std::env::set_var("KENTOS_GRID_DIR", &library) };
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        let mut app = sample();
        let doc = &mut app.document.as_mut().expect("a drawing").model;
        let mut settings = doc.settings().clone();
        settings.heights = Some(kentos_commands::HeightReference {
            kind: kentos_commands::HeightKind::Orthometric,
            vertical_srid: Some(5775),
            geoid: Some(kentos_commands::GeoidModel { name: "TG-20".into(), sha256: Some(id.clone()) }),
        });
        doc.set_settings(settings);
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(crate::app::Message::WindowResized(size));
        let _ = app.update(crate::app::Message::Run("view.zoomExtents"));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        run(&mut app, "file.package");
        let target = scratch(&format!("paket-izgara-{name}")).join("teslim");
        let input = app.package_input(&target).expect("an input");
        let doc = &app.document.as_ref().expect("a drawing").model;
        let planned = kentos_headless::package::plan(doc, &input).expect("planned");
        kentos_headless::package::write(&planned).expect("packed");
        let _ = app.update(crate::app::Message::DialogClosed);
        send(&mut app, Event::Package(P::Done(Ok(Box::new((planned.output, planned.warnings))))));
        snapshot.settle(&mut app, App::view, &mut |app: &mut App, message| {
            let _ = app.update(message);
        });
        let path = out.join(format!("ileti-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&path).expect("writes the picture");
        println!("{}", path.display());
        let _ = std::fs::copy(target.join("paket.json"), out.join(format!("paket-{name}.json")));
    }
}

/// The open files' window and the package's message; `.run/gui-evidence/teslim-devir-2780/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib exchange::package_tests::open_file_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures, run by hand"]
fn open_file_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/teslim-devir-2780");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        let mut app = sample();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(crate::app::Message::WindowResized(size));
        let _ = app.update(crate::app::Message::Run("view.zoomExtents"));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut picture = |app: &mut App, label: &str| {
            snapshot.settle(app, App::view, &mut |app: &mut App, message| {
                let _ = app.update(message);
            });
            let path = out.join(format!("{label}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&path).expect("writes the picture");
            println!("{}", path.display());
        };
        run(&mut app, "file.package");
        for f in [kentos_commands::ExchangeFormat::Gpkg, kentos_commands::ExchangeFormat::Shp, kentos_commands::ExchangeFormat::Dxf] {
            send(&mut app, Event::Package(P::Format(f)));
        }
        picture(&mut app, "1-pencere");
        let target = scratch(&format!("paket-devir-{name}")).join("teslim");
        let input = app.package_input(&target).expect("an input");
        let doc = &app.document.as_ref().expect("a drawing").model;
        let planned = kentos_headless::package::plan(doc, &input).expect("planned");
        kentos_headless::package::write(&planned).expect("packed");
        let _ = app.update(crate::app::Message::DialogClosed);
        send(&mut app, Event::Package(P::Done(Ok(Box::new((planned.output, planned.warnings))))));
        picture(&mut app, "2-ileti");
        // The report as a spreadsheet would show it, for the owner to read beside the pictures.
        let _ = std::fs::copy(target.join("devir/devir-raporu.csv"), out.join(format!("devir-raporu-{name}.csv")));
    }
}

#[test]
#[ignore = "pictures, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (width, height, mode) in [
        (1440.0, 900.0, "dark"),
        (1100.0, 650.0, "dark"),
        (1440.0, 900.0, "light"),
    ] {
        let mut app = scene(&format!("paket-{mode}-{width}"));
        let _ = app
            .settings
            .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
        app.apply_settings();
        let _ = app.update(crate::app::Message::WindowResized(Size::new(width, height)));
        let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
        let mut picture = |app: &mut App, label: &str| {
            snapshot.settle(app, App::view, &mut |app: &mut App, message| {
                let _ = app.update(message);
            });
            let path = out.join(format!("paket-{mode}-{label}-{width}x{height}.png"));
            snapshot
                .render(app.view(), &app.theme())
                .save(&path)
                .expect("writes the picture");
            println!("{}", path.display());
        };
        run(&mut app, "file.package");
        picture(&mut app, "1-pencere");
        send(
            &mut app,
            Event::Package(P::Databases(Way(PackageWay::Copy))),
        );
        send(&mut app, Event::Package(P::Take(0)));
        picture(&mut app, "2-secim");
        send(
            &mut app,
            Event::Package(P::GeoPackages(Way(PackageWay::Embed))),
        );
        send(
            &mut app,
            Event::Package(P::Databases(Way(PackageWay::Embed))),
        );
        send(&mut app, Event::Package(P::Take(0)));
        let target = scratch(&format!("paket-{mode}-{width}-cikti")).join("teslim");
        let input = app.package_input(&target).expect("an input");
        let doc = &app.document.as_ref().expect("a drawing").model;
        let planned = kentos_headless::package::plan(doc, &input).expect("planned");
        kentos_headless::package::write(&planned).expect("packed");
        let _ = app.update(crate::app::Message::DialogClosed);
        send(
            &mut app,
            Event::Package(P::Done(Ok(Box::new((planned.output, planned.warnings))))),
        );
        picture(&mut app, "3-gomuldu");
    }
}

/// A package written by hand in `folder`: the sample drawing with its heights on a geoid model of its own (TG-20 with
/// its last node moved three millimetres: no other test, nor the device's library, holds it) as `harita.kcad`, the
/// model in `izgaralar/` under its SHA-256. Answers the project file and the model's SHA-256.
fn package_with_own_geoid(folder: &std::path::Path) -> (PathBuf, String) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut bytes = std::fs::read(root.join("fixtures/geoid/v1/tg.gtx")).expect("the geoid");
    let n = bytes.len();
    let last = f32::from_be_bytes(bytes[n - 4..].try_into().expect("four bytes")) + 0.003;
    bytes[n - 4..].copy_from_slice(&last.to_be_bytes());
    let sha = kentos_native_application::crs_models::sha256(&bytes);
    std::fs::create_dir_all(folder.join("izgaralar")).expect("a folder");
    std::fs::write(folder.join("izgaralar").join(format!("{sha}.gtx")), &bytes).expect("writes");
    let mut drawing: serde_json::Value =
        serde_json::from_str(include_str!("../../../../fixtures/exchange/v1/devir.json")).expect("the sample");
    drawing["settings"]["heights"] =
        serde_json::json!({ "kind": "orthometric", "verticalSrid": 5775, "geoid": { "name": "TG-20 (paket)", "sha256": sha } });
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(drawing).expect("reads");
    let path = folder.join("harita.kcad");
    std::fs::write(&path, kentos_kcad::encode(&snapshot).expect("encodes")).expect("writes");
    (path, sha)
}

/// docs/adr/3580: a project opened from its package finds the geoid model it names in `izgaralar/` beside it (its
/// SHA-256 checked), says so, and the commands find it in the core; the device's library is not written.
#[test]
fn a_package_s_grid_is_found_when_it_is_opened() {
    let folder = scratch("paket-izgara-acilis");
    let (path, sha) = package_with_own_geoid(&folder);
    let mut app = crate::files_testing::app_with_drawing();
    let snapshot = kentos_kcad::decode(&std::fs::read(&path).expect("reads")).expect("decodes");
    let doc = crate::document::Document::from_v2(snapshot, Some(path.clone())).expect("opens");
    let _ = app.update(crate::app::Message::Opened(Some(Ok(Box::new(doc)))));
    let said: Vec<String> = app.log.lines().map(|l| l.text.clone()).collect();
    assert!(
        said.iter().any(|l| l.contains("izgaralar/ klasöründen 1 ızgara kullanılıyor") && l.contains("TG-20 (paket)")),
        "{said:?}"
    );
    let kind = kentos_native_application::crs_models::ModelKind::Geoid;
    assert!(kentos_native_application::crs_models::load(&sha, kind, Some(&folder.join("bos"))).is_ok(), "in the core");
}

/// docs/adr/3580: the drawing opened from its package, its message in the status bar and the history;
/// `.run/gui-evidence/paket-izgara-acilis-3580/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib exchange::package_tests::found_grid_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn found_grid_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/paket-izgara-acilis-3580");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 700.0), "koyu-dar")] {
        let folder = scratch(&format!("paket-izgara-acilis-{name}"));
        let (path, _) = package_with_own_geoid(&folder);
        let mut app = crate::files_testing::app_with_drawing();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(crate::app::Message::WindowResized(size));
        let snapshot = kentos_kcad::decode(&std::fs::read(&path).expect("reads")).expect("decodes");
        let doc = crate::document::Document::from_v2(snapshot, Some(path.clone())).expect("opens");
        let _ = app.update(crate::app::Message::Opened(Some(Ok(Box::new(doc)))));
        let _ = app.update(crate::app::Message::Run("view.zoomExtents"));
        let _ = app.update(crate::app::Message::CommandHistoryToggled);
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: crate::app::Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("acilis-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}
