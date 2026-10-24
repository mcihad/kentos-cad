//! A CAD file of one's own on the desktop's frame, measured by its parts (perf/frame.rs): the frames of a pan and
//! of the wheel over a working view, under each text size rule (`graphics.annotationSize`, docs/adr/0205 §5, 0626
//! §6). The owner's 24 October report (texts going and coming, slowly, out of step with the scene) on their own
//! plan. Not a correctness test and not run by default:
//!
//! ```text
//! KENTOS_PERF_FILE=$HOME/Downloads/deneme_suşehri.dxf \
//!   cargo test --release -p kentos-desktop perf::file -- --ignored --nocapture --test-threads=1
//! ```

use iced::mouse::{self, ScrollDelta};
use iced::{Event, Point};

use super::frame::{Harness, Parts, moved, pan, summary};
use crate::app::{App, Message};
use crate::document::Document;

/// Wheel steps at `at`, in and out: their frames.
fn wheel(app: &mut App, h: &mut Harness, at: Point) -> Vec<Parts> {
    (0..16)
        .map(|i| {
            let y = if i % 8 < 4 { 1.0 } else { -1.0 };
            h.frame(
                app,
                &[
                    moved(at),
                    Event::Mouse(mouse::Event::WheelScrolled {
                        delta: ScrollDelta::Lines { x: 0.0, y },
                    }),
                ],
            )
        })
        .collect()
}

#[test]
#[ignore = "a measurement on a file of one's own, run by hand in release mode"]
fn file() {
    let Some(path) = std::env::var_os("KENTOS_PERF_FILE") else {
        println!("KENTOS_PERF_FILE verilmedi");
        return;
    };
    let path = std::path::PathBuf::from(path);
    let data = std::fs::read(&path).expect("the file reads");
    let format = kentos_native_application::cad_open::Format::of(&path).expect("a CAD file");
    let mut watch = kentos_formats::watch::Steps(|_, _| true);
    let opened = kentos_native_application::cad_open::read(&path, &data, format, &mut watch)
        .expect("it opens");
    let doc = Document::of_model(opened.document);
    let (mut app, _) = App::boot(None);
    let mut h = Harness::new();
    h.settle(&mut app);
    h.frame_with(&mut app, &[], move |app| {
        let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    });
    let _ = app.update(Message::DialogClosed);
    h.settle(&mut app);
    println!("çizici: {}", {
        use iced::advanced::renderer::Headless as _;
        h.renderer.name()
    });
    for (size, word) in [("legible", "Kaybolmasın"), ("true", "Gerçek boy")] {
        let _ = app
            .settings
            .choose(&[("graphics.annotationSize", serde_json::Value::from(size))]);
        app.apply_settings();
        // The working view: the drawing's extent, then eight wheel steps in round its middle.
        let _ = app.update(Message::Run("view.zoomExtents"));
        h.settle(&mut app);
        let center = app.viewport.bounds.center();
        let _ = wheel(&mut app, &mut h, center);
        for _ in 0..8 {
            h.frame(
                &mut app,
                &[
                    moved(center),
                    Event::Mouse(mouse::Event::WheelScrolled {
                        delta: ScrollDelta::Lines { x: 0.0, y: 1.0 },
                    }),
                ],
            );
        }
        let _ = app.update(Message::Viewport(crate::viewport::Event::Settled));
        h.settle(&mut app);
        let rest: Vec<Parts> = (0..5).map(|_| h.frame(&mut app, &[])).collect();
        println!("{word}, dinlenirken: {}", summary(&rest));
        let panning = pan(&mut app, &mut h, center);
        println!("{word}, kaydırırken: {}", summary(&panning));
        let settled = h.frame_with(&mut app, &[], |app| {
            let _ = app.update(Message::Viewport(crate::viewport::Event::Settled));
        });
        println!("{word}, kaydırma duruldu: {}", summary(&[settled]));
        let zooming = wheel(&mut app, &mut h, center);
        println!("{word}, tekerlekle: {}", summary(&zooming));
    }
}
