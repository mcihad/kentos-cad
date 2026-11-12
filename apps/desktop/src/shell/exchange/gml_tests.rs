//! GML içe ve dışa aktar and XYZ dışa aktar on the desktop (docs/adr/0545): the window shows the plan of
//! `file.import.gml`; İçe aktar takes the features as one undo step; a file in another system waits for the move;
//! Dışa aktar… writes through `file.export.gml` (the schema beside it) and XYZ through `file.export.xyz`. Pictures:
//! `gml-*` and `xyz-*` (`screens`, ignored).

use std::path::PathBuf;

use kentos_commands::XyzVertices;

use super::gml_exchange::Event as G;
use super::tests::{count, run, said, send};
use super::xyz_exchange::{Event as X, Which};
use super::{Event, Window};
use crate::app::{App, Message, Picker};
use crate::files_testing::{app_with_drawing, scratch};

fn gml(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/gml/v1")
        .join(name)
}

fn importing(app: &App) -> &super::gml_exchange::ImportState {
    match &app.exchange {
        Some(Window::GmlImport(s)) => s,
        _ => panic!("the GML import window"),
    }
}

#[test]
fn a_gml_goes_in_as_one_undo_step() {
    let mut app = app_with_drawing();
    let before = count(&app);
    app.picker = Picker::File(gml("parsel.gml"));
    run(&mut app, "file.import.gml");
    let planned = importing(&app).plan.as_ref().expect("planned").0.objects;
    assert_eq!(planned, 3);
    send(&mut app, Event::Gml(G::Run));
    assert!(app.exchange.is_none());
    assert_eq!(count(&app), before + planned as usize);
    assert!(
        said(&app, "“parsel.gml” alındı"),
        "{:?}",
        app.log.lines().last().map(|l| l.text.clone())
    );
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(count(&app), before);
}

#[test]
fn a_file_in_another_system_waits_for_the_move_to_be_asked() {
    let mut app = app_with_drawing();
    app.picker = Picker::File(gml("yer.gml"));
    run(&mut app, "file.import.gml");
    assert!(importing(&app).plan.is_err(), "nothing moved silently");
    send(&mut app, Event::Gml(G::Reproject));
    assert!(importing(&app).plan.is_ok());
}

#[test]
fn the_drawing_goes_out_as_one_gml_with_its_schema() {
    let mut app = app_with_drawing();
    run(&mut app, "file.export.gml");
    let Some(Window::GmlExport(s)) = &app.exchange else {
        panic!("the export window");
    };
    let planned = s.plan.as_ref().expect("planned").features;
    let path = scratch("gml-disa").join("cizim.gml");
    send(&mut app, Event::Gml(G::Saved(Some(path.clone()))));
    // Written off the window's thread: the answer comes back as a message.
    let text = std::fs::read_to_string(&path).expect("written");
    assert_eq!(text.matches("<ogr:featureMember>").count() as u32, planned);
    assert!(path.with_extension("xsd").exists());
}

#[test]
fn the_schema_can_be_left_out() {
    let mut app = app_with_drawing();
    run(&mut app, "file.export.gml");
    send(&mut app, Event::Gml(G::Schema));
    let Some(Window::GmlExport(s)) = &app.exchange else {
        panic!("the export window");
    };
    assert!(!s.schema);
    assert!(s.plan.as_ref().expect("planned").files.iter().all(|f| !f.name.ends_with(".xsd")));
}

#[test]
fn the_vertices_go_out_as_xyz_lines() {
    let mut app = app_with_drawing();
    run(&mut app, "file.export.xyz");
    send(&mut app, Event::Xyz(X::Which(Which(XyzVertices::All))));
    let Some(Window::XyzExport(s)) = &app.exchange else {
        panic!("the export window");
    };
    let planned = s.plan.as_ref().expect("planned").points;
    assert!(planned > 0);
    let path = scratch("xyz-disa").join("cizim.xyz");
    send(&mut app, Event::Xyz(X::Saved(Some(path.clone()))));
    let text = std::fs::read_to_string(&path).expect("written");
    assert_eq!(text.lines().count() as u32, planned);
    assert!(text.lines().all(|l| l.split(' ').count() == 3), "{text}");
}

#[test]
#[ignore = "pictures, run by hand"]
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
            app.picker = Picker::File(gml("parsel.gml"));
            run(&mut app, "file.import.gml");
            picture(&mut app, &format!("gml-{mode}-1-ice-aktar"));
            send(&mut app, Event::Gml(G::Run));
            picture(&mut app, &format!("gml-{mode}-2-alindi"));
            let mut app = fresh();
            app.picker = Picker::File(gml("yer.gml"));
            run(&mut app, "file.import.gml");
            picture(&mut app, &format!("gml-{mode}-3-sistem"));
            let mut app = fresh();
            run(&mut app, "file.export.gml");
            picture(&mut app, &format!("gml-{mode}-4-disa-aktar"));
            let mut app = fresh();
            run(&mut app, "file.export.xyz");
            send(&mut app, Event::Xyz(X::Which(Which(XyzVertices::All))));
            picture(&mut app, &format!("xyz-{mode}-1-disa-aktar"));
        }
    }
}
