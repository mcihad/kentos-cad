//! Paketle's window on the desktop (docs/adr/0596 §5): with bound layers it asks how their sources go and which links
//! stay out; its answer is the input `file.package` gets, and the package made says the copies and the links embedded.
//! Pictures: `paket-*` (`screens`, ignored).

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

#[test]
fn without_bound_layers_the_folder_is_asked_at_once() {
    let mut app = app_with_drawing();
    app.picker = crate::app::Picker::File(scratch("paket-baglisiz").join("teslim"));
    run(&mut app, "file.package");
    assert!(app.exchange.is_none());
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
