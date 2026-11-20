//! GIS-035 (docs/adr/1092): a point cloud's result never takes an earlier one's file by itself — two runs with no
//! output asked, the forward and the inverse, get two files beside the scan — and a file the user picks that is there
//! is said under Çıktı dosyası with the drawing's objects linked to it, before the run writes over it.

use std::collections::HashMap;

use kentos_processing::files::{Beside, Files};
use serde_json::Value;

use super::Event as P;
use crate::app::{App, Message};
use crate::gis;

/// The scan beside which both automatic results go: forward then inverse, each its own file, neither the scan's.
#[test]
fn the_forward_and_the_inverse_take_two_files_beside_the_scan() {
    let dir = crate::files_testing::scratch("bulut-otomatik-ad");
    let scan = dir.join("Tarama-2.laz");
    std::fs::write(&scan, b"scan").expect("a scan");
    let files = crate::pointclouds::files::DesktopFiles::new(Some(dir.clone()), HashMap::new());
    let file = scan.to_string_lossy().into_owned();
    let beside = || Beside {
        file: Some(&file),
        url: None,
        asset: None,
        name: None,
        fallback: "bulut",
    };
    let forward = files
        .output_path("", Some(beside()), "-donusmus", ".laz")
        .expect("a path");
    let inverse = files
        .output_path("", Some(beside()), "-donusmus", ".laz")
        .expect("a path");
    assert_ne!(forward, inverse, "two runs, two files");
    assert_ne!(forward, file);
    assert!(forward.ends_with("Tarama-2-donusmus.laz"), "{forward}");
    assert!(inverse.ends_with("Tarama-2-donusmus-2.laz"), "{inverse}");
}

/// The fitted scan written to a file; the transform's window opened again with that file picked says the moved scan
/// is linked to it, a new name says nothing.
#[test]
fn a_picked_file_that_is_there_says_the_objects_linked_to_it() {
    use crate::processing::RunStatus;
    let mut app = crate::register_window_tests::opened(Some(kentos_launch::Module::Map));
    let _ = app.update(Message::Run("pointcloud.register"));
    let _ = app.update(Message::Map(crate::map::Event::Register(
        crate::register_window::Event::Apply,
    )));
    let dir = crate::files_testing::scratch("bulut-uzerine-yazma");
    let first = dir.join("kayitli.laz");
    let pick = |app: &mut App, path: &std::path::Path| {
        let _ = app.update(Message::Gis(gis::Event::Processing(P::Value(
            "output".into(),
            Value::from(path.to_string_lossy().into_owned()),
        ))));
    };
    pick(&mut app, &first);
    let window = app.gis.processing.dialog.as_ref().expect("the window");
    assert!(window.overwrites.is_empty(), "a new file: nothing said");
    let task = app.update(Message::Gis(gis::Event::Processing(P::Run)));
    crate::files_testing::drive(&mut app, task);
    let status = app.gis.processing.dialog.as_ref().map(|w| w.status.clone());
    assert!(matches!(status, Some(RunStatus::Ok { .. })), "{status:?}");
    // The same file picked again: the moved scan's.
    pick(&mut app, &first);
    let window = app.gis.processing.dialog.as_ref().expect("the window");
    let said = window.overwrites.get("output").expect("the file is there");
    assert_eq!(said.bound.len(), 1, "{said:?}");
    let line = said.line();
    assert!(line.contains("1 nesne ona bağlı"), "{line}");
    assert!(line.contains("üzerine yazılır"), "{line}");
    // Another name: nothing.
    pick(&mut app, &dir.join("ters.laz"));
    let window = app.gis.processing.dialog.as_ref().expect("the window");
    assert!(window.overwrites.is_empty());
}

/// The window with a linked file picked again: its line under Çıktı dosyası, on the screen.
#[test]
fn the_line_is_on_the_screen() {
    let mut app = crate::register_window_tests::opened(Some(kentos_launch::Module::Map));
    let _ = app.update(Message::Run("pointcloud.register"));
    let _ = app.update(Message::Map(crate::map::Event::Register(
        crate::register_window::Event::Apply,
    )));
    let dir = crate::files_testing::scratch("bulut-uzerine-yazma-resim");
    let first = dir.join("Tarama-2-donusmus.laz");
    let pick = |app: &mut App| {
        let _ = app.update(Message::Gis(gis::Event::Processing(P::Value(
            "output".into(),
            Value::from(first.to_string_lossy().into_owned()),
        ))));
    };
    pick(&mut app);
    let task = app.update(Message::Gis(gis::Event::Processing(P::Run)));
    crate::files_testing::drive(&mut app, task);
    pick(&mut app);
    let mut shot = crate::files_testing::pictured(iced::Size::new(1440., 900.));
    shot.settle(&mut app, App::view, &mut |app: &mut App, m| {
        drop(app.update(m))
    });
    form_to_its_end(&mut shot, &app);
    shot.settle(&mut app, App::view, &mut |app: &mut App, m| {
        drop(app.update(m))
    });
    let line = app
        .gis
        .processing
        .dialog
        .as_ref()
        .and_then(|w| w.overwrites.get("output"))
        .expect("said")
        .line();
    assert!(
        crate::files_testing::find_text(&mut shot, &app, &line).is_some(),
        "the line is shown: {line}"
    );
    crate::files_testing::evidence(&mut shot, &app, "cbs-kullanim", "gis-035-uzerine-yazma");
}

/// The window's form scrolled to its end, where Çıktı dosyası is.
fn form_to_its_end(shot: &mut kentos_ui::snapshot::Snapshot, app: &App) {
    use iced::advanced::widget::{Id, Operation, operation::Scrollable};
    use iced::{Rectangle, Vector};
    struct End;
    impl Operation for End {
        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
            operate(self);
        }
        fn scrollable(
            &mut self,
            id: Option<&Id>,
            _: Rectangle,
            _: Rectangle,
            _: Vector,
            state: &mut dyn Scrollable,
        ) {
            if id == Some(&Id::new(super::window::FORM)) {
                state.snap_to(iced::widget::scrollable::RelativeOffset::END.into());
            }
        }
    }
    shot.operate(app.view(), Box::new(End));
}
