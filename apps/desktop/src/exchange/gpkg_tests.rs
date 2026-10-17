//! GeoPackage içe ve dışa aktar on the desktop (docs/adr/0502): the window lists the tables, takes the chosen ones as
//! one undo step through `file.import.gpkg`; a file in another system waits for Projenin sistemine çevir; Dışa aktar
//! writes through `file.export.gpkg`. Pictures: `gpkg-*` (`screens`, ignored).

use std::path::PathBuf;

use super::gpkg::Event as G;
use super::tests::{count, run, said, send};
use super::{Event, Window};
use crate::app::{App, Message, Picker};
use crate::files_testing::{app_with_drawing, scratch};

fn gpkg(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/geopackage/v1")
        .join(name)
}

fn window(app: &App) -> &super::gpkg::State {
    match &app.exchange {
        Some(Window::Gpkg(s)) => s,
        _ => panic!("the GeoPackage window"),
    }
}

#[test]
fn a_geopackage_goes_in_as_one_undo_step_with_its_tables_as_layers() {
    let mut app = app_with_drawing();
    let before = count(&app);
    app.picker = Picker::File(gpkg("qgis.gpkg"));
    run(&mut app, "file.import.gpkg");
    let s = window(&app);
    assert_eq!(s.tables.len(), 6);
    assert_eq!(s.srid, Some(5256));
    assert!(s.chosen.iter().all(|c| *c));
    // Without the road table.
    send(&mut app, Event::Gpkg(G::Toggle(1)));
    send(&mut app, Event::Gpkg(G::Run));
    assert!(
        app.exchange.is_none(),
        "{:?}",
        app.exchange.as_ref().map(|_| "open")
    );
    assert_eq!(count(&app), before + 8);
    assert!(
        said(&app, "“qgis.gpkg” alındı"),
        "{:?}",
        app.log.lines().last().map(|l| l.text.clone())
    );
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(count(&app), before);
}

#[test]
fn a_file_in_another_system_waits_for_the_move_to_be_asked() {
    let mut app = app_with_drawing();
    let before = count(&app);
    app.picker = Picker::File(gpkg("wgs84.gpkg"));
    run(&mut app, "file.import.gpkg");
    assert_eq!(window(&app).srid, Some(4326));
    // İçe aktar is off until the move is asked; asked, the point goes into TM36.
    send(&mut app, Event::Gpkg(G::Run));
    assert_eq!(count(&app), before, "nothing moved silently");
    assert!(
        window(&app)
            .status
            .as_ref()
            .is_some_and(|(e, t)| *e && t.contains("reproject"))
    );
    send(&mut app, Event::Gpkg(G::Reproject));
    send(&mut app, Event::Gpkg(G::Run));
    assert_eq!(count(&app), before + 1);
}

#[test]
fn the_drawing_goes_out_as_a_geopackage() {
    let dir = scratch("gpkg-export");
    let mut app = app_with_drawing();
    app.picker = Picker::File(gpkg("qgis.gpkg"));
    run(&mut app, "file.import.gpkg");
    send(&mut app, Event::Gpkg(G::Run));
    let out = dir.join("teslim.gpkg");
    app.picker = Picker::File(out.clone());
    run(&mut app, "file.export.gpkg");
    assert!(
        out.exists(),
        "{:?}",
        app.log.lines().map(|l| l.text.clone()).collect::<Vec<_>>()
    );
    assert!(
        said(&app, "“teslim.gpkg” yazıldı"),
        "{:?}",
        app.log.lines().last().map(|l| l.text.clone())
    );
}

#[test]
#[ignore = "pictures, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let only = std::env::var("KENTOS_SHOTS_ONLY").unwrap_or_default();
    let wanted = |name: &str| only.is_empty() || only.split(',').any(|w| name.contains(w));
    for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
        let picture = |app: &mut App, name: &str| {
            if !wanted(name) {
                return;
            }
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
            app.picker = Picker::File(gpkg("qgis.gpkg"));
            run(&mut app, "file.import.gpkg");
            picture(&mut app, &format!("gpkg-{mode}-1-ice-aktar"));
            send(&mut app, Event::Gpkg(G::Run));
            picture(&mut app, &format!("gpkg-{mode}-3-alindi"));
            let mut app = fresh();
            app.picker = Picker::File(gpkg("wgs84.gpkg"));
            run(&mut app, "file.import.gpkg");
            send(&mut app, Event::Gpkg(G::Reproject));
            picture(&mut app, &format!("gpkg-{mode}-2-cevir"));
        }
    }
}
