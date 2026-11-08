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
