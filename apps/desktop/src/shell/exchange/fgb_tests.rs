//! FlatGeobuf içe ve dışa aktar on the desktop (docs/adr/0544): the window shows the plan of `file.import.fgb`;
//! İçe aktar takes the features as one undo step; a file in another system waits for the move; Dışa aktar… writes
//! through `file.export.fgb`. Pictures: `fgb-*` (`screens`, ignored).

use std::path::PathBuf;

use super::fgb_exchange::Event as F;
use super::tests::{count, run, said, send};
use super::{Event, Window};
use crate::app::{App, Message, Picker};
use crate::files_testing::{app_with_drawing, scratch};

fn fgb(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/fgb/v1")
        .join(name)
}

fn importing(app: &App) -> &super::fgb_exchange::ImportState {
    match &app.exchange {
        Some(Window::FgbImport(s)) => s,
        _ => panic!("the FlatGeobuf import window"),
    }
}

#[test]
fn a_flatgeobuf_goes_in_as_one_undo_step() {
    let mut app = app_with_drawing();
    let before = count(&app);
    app.picker = Picker::File(fgb("parsel-dizinli.fgb"));
    run(&mut app, "file.import.fgb");
    let planned = importing(&app).plan.as_ref().expect("planned").0.objects;
    assert_eq!(planned, 2);
    send(&mut app, Event::Fgb(F::Run));
    assert!(app.exchange.is_none());
    assert_eq!(count(&app), before + planned as usize);
    assert!(
        said(&app, "“parsel-dizinli.fgb” alındı"),
        "{:?}",
        app.log.lines().last().map(|l| l.text.clone())
    );
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(count(&app), before);
}

#[test]
fn a_file_in_another_system_waits_for_the_move_to_be_asked() {
    let mut app = app_with_drawing();
    app.picker = Picker::File(fgb("yer.fgb"));
    run(&mut app, "file.import.fgb");
    assert!(importing(&app).plan.is_err(), "nothing moved silently");
    send(&mut app, Event::Fgb(F::Reproject));
    assert!(importing(&app).plan.is_ok());
}

#[test]
fn the_drawing_goes_out_as_one_flatgeobuf() {
    let mut app = app_with_drawing();
    run(&mut app, "file.export.fgb");
    let Some(Window::FgbExport(s)) = &app.exchange else {
        panic!("the export window");
    };
    let planned = s.plan.as_ref().expect("planned").features;
    let path = scratch("fgb-disa").join("cizim.fgb");
    send(&mut app, Event::Fgb(F::Saved(Some(path.clone()))));
    // Written off the window's thread: the answer comes back as a message.
    let bytes = std::fs::read(&path).expect("written");
    let back = kentos_geopackage::fgb::read(&bytes).expect("reads");
    assert_eq!(back.features.len() as u32, planned);
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
            app.picker = Picker::File(fgb("parsel.fgb"));
            run(&mut app, "file.import.fgb");
            picture(&mut app, &format!("fgb-{mode}-1-ice-aktar"));
            send(&mut app, Event::Fgb(F::Run));
            picture(&mut app, &format!("fgb-{mode}-2-alindi"));
            let mut app = fresh();
            app.picker = Picker::File(fgb("yer.fgb"));
            run(&mut app, "file.import.fgb");
            picture(&mut app, &format!("fgb-{mode}-3-sistem"));
            let mut app = fresh();
            run(&mut app, "file.export.fgb");
            picture(&mut app, &format!("fgb-{mode}-4-disa-aktar"));
        }
    }
}
