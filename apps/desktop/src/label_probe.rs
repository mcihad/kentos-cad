//! How the drawing's texts behave on a large imported plan (the owner's 24 October report: measuring under Açıklama,
//! the texts went and came, out of step with the scene, very slowly): the time a frame takes at rest, while the view
//! pans and zooms and once it settles, and when a dimension is written, under each text size rule
//! (`graphics.annotationSize`); how many texts each frame shows, how many of them change when a dimension is written,
//! and how many text pictures each frame made and let go (label_picture.rs). Run by hand on a file of one's own, in
//! release:
//!
//! ```text
//! KENTOS_PROBE_FILE=$HOME/Downloads/deneme_suşehri.dxf cargo test --release -p kentos-desktop --bin kentos-cad label_probe -- --ignored --nocapture
//! ```

use std::collections::BTreeSet;
use std::time::{Duration, Instant};

use iced::{Point, Size, Vector};
use kentos_ui::snapshot::{Input, Snapshot};

use crate::app::{App, Message};
use crate::files_testing::app_with_drawing;
use crate::viewport::Event;

fn update(app: &mut App, message: Message) {
    let _ = app.update(message);
}

/// One frame drawn as the window draws it: the view built and rendered; its time, the pictures it made and the
/// times the pictures' store emptied.
fn frame(snapshot: &mut Snapshot, app: &App) -> (Duration, usize, usize) {
    let (made, cleared) = crate::label_picture::counts();
    let t = Instant::now();
    let _ = snapshot.render(app.view(), &app.theme());
    let took = t.elapsed();
    let (made2, cleared2) = crate::label_picture::counts();
    (took, made2 - made, cleared2 - cleared)
}

fn at(app: &App, fx: f32, fy: f32) -> Point {
    let b = app.viewport.bounds;
    Point::new(b.x + b.width * fx, b.y + b.height * fy)
}

/// The texts the last frame showed, by their objects.
fn shown(app: &App) -> BTreeSet<u64> {
    app.label_spots
        .last()
        .map(|spots| {
            spots
                .iter()
                .map(|(spot, _)| crate::labels::slot_of(spot).0 as u64)
                .collect()
        })
        .unwrap_or_default()
}

fn report(name: &str, frames: &[(Duration, usize, usize)], texts: usize) {
    let mut ms: Vec<f64> = frames.iter().map(|f| f.0.as_secs_f64() * 1000.0).collect();
    ms.sort_by(f64::total_cmp);
    let made: usize = frames.iter().map(|f| f.1).sum();
    let cleared: usize = frames.iter().map(|f| f.2).sum();
    println!(
        "  {name}: {} kare, ortanca {:.1} ms, en uzun {:.1} ms; {made} yazı resmi, depo {cleared} kez boşaldı; \
         son karede {texts} yazı",
        ms.len(),
        ms[ms.len() / 2],
        ms[ms.len() - 1]
    );
}

/// The picture of the window, to `.run/shots/<name>.png`.
fn save(snapshot: &mut Snapshot, app: &App, name: &str) {
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    let _ = std::fs::create_dir_all(&out);
    let file = out.join(format!("{name}.png"));
    if snapshot
        .render(app.view(), &app.theme())
        .save(&file)
        .is_ok()
    {
        println!("    {}", file.display());
    }
}

/// The working view: the drawing's extent, then eight wheel steps in round its middle.
fn working_view(snapshot: &mut Snapshot, app: &mut App) {
    update(app, Message::Run("view.zoomExtents"));
    snapshot.settle(app, App::view, &mut update);
    for _ in 0..8 {
        let middle = at(app, 0.5, 0.5);
        snapshot.input(app, App::view, &mut update, Input::Scroll(middle, 1.0));
    }
    update(app, Message::Viewport(Event::Settled));
    snapshot.settle(app, App::view, &mut update);
}

#[test]
#[ignore = "a probe on a file of one's own, run by hand"]
fn label_probe() {
    let Some(path) = std::env::var_os("KENTOS_PROBE_FILE") else {
        println!("KENTOS_PROBE_FILE verilmedi");
        return;
    };
    let path = std::path::PathBuf::from(path);
    let data = std::fs::read(&path).expect("the file reads");
    let format = kentos_native_application::cad_open::Format::of(&path).expect("a CAD file");
    let mut watch = kentos_formats::watch::Steps(|_, _| true);
    let t = Instant::now();
    let opened = kentos_native_application::cad_open::read(&path, &data, format, &mut watch)
        .expect("it opens");
    println!("açılış: {:.0} ms", t.elapsed().as_secs_f64() * 1000.0);
    let doc = crate::document::Document::of_model(opened.document);
    let mut snapshot = Snapshot::new(Size::new(1440.0, 900.0)).expect("a renderer");
    let mut app = app_with_drawing();
    update(&mut app, Message::Opened(Some(Ok(Box::new(doc)))));
    update(&mut app, Message::DialogClosed);
    snapshot.settle(&mut app, App::view, &mut update);
    if let Some(d) = &app.document {
        let s = d.settings();
        println!(
            "birim {:?}, çizim ölçeği 1:{}, nesne {}",
            s.unit(),
            s.plot_scale,
            d.model.len()
        );
    }
    for (size, word) in [("legible", "Kaybolmasın"), ("true", "Gerçek boy")] {
        let _ = app
            .settings
            .choose(&[("graphics.annotationSize", serde_json::Value::from(size))]);
        app.apply_settings();
        println!("{word}:");
        working_view(&mut snapshot, &mut app);
        let rest: Vec<_> = (0..3).map(|_| frame(&mut snapshot, &app)).collect();
        report("dinlenirken", &rest, shown(&app).len());
        // What the store takes to answer the view (the rule's placing), and the picture.
        let camera = app.viewport.camera.upright();
        let view = camera.visible_bounds();
        let t = Instant::now();
        let answered = app.spatial.labels_shown_in(
            kentos_interaction::Vec2::new(view.min_x, view.min_y),
            kentos_interaction::Vec2::new(view.max_x, view.max_y),
            camera.scale,
            app.label_size(),
            app.label_engine().options,
            false,
            None,
        );
        println!(
            "    deponun cevabı {:.1} ms, {} yazı",
            t.elapsed().as_secs_f64() * 1000.0,
            answered.len()
        );
        save(&mut snapshot, &app, &format!("yazi-boyu-{size}-calisma"));
        for _ in 0..5 {
            let middle = at(&app, 0.5, 0.5);
            snapshot.input(&mut app, App::view, &mut update, Input::Scroll(middle, 1.0));
        }
        update(&mut app, Message::Viewport(Event::Settled));
        snapshot.settle(&mut app, App::view, &mut update);
        save(&mut snapshot, &app, &format!("yazi-boyu-{size}-yakin"));
        working_view(&mut snapshot, &mut app);
        // A middle-button pan, 25 px a step.
        let mut panning = Vec::new();
        for _ in 0..12 {
            let middle = at(&app, 0.5, 0.5);
            let t = Instant::now();
            update(
                &mut app,
                Message::Viewport(Event::Panned {
                    by: Vector::new(25.0, 0.0),
                    at: middle,
                }),
            );
            let input = t.elapsed();
            let (took, made, cleared) = frame(&mut snapshot, &app);
            panning.push((input + took, made, cleared));
        }
        report("kaydırırken", &panning, shown(&app).len());
        let before = shown(&app);
        update(&mut app, Message::Viewport(Event::Settled));
        let settled = frame(&mut snapshot, &app);
        let after = shown(&app);
        report("kaydırma duruldu", &[settled], after.len());
        println!(
            "    durulunca {} yazı belirdi, {} yazı kayboldu",
            after.difference(&before).count(),
            before.difference(&after).count()
        );
        let mut zooming = Vec::new();
        for i in 0..12 {
            let p = at(&app, 0.5, 0.5);
            let t = Instant::now();
            snapshot.input(
                &mut app,
                App::view,
                &mut update,
                Input::Scroll(p, if i % 2 == 0 { 1.0 } else { -1.0 }),
            );
            let input = t.elapsed();
            let (took, made, cleared) = frame(&mut snapshot, &app);
            zooming.push((input + took, made, cleared));
        }
        report("tekerlekle yakınlaşırken", &zooming, shown(&app).len());
        let before = shown(&app);
        update(&mut app, Message::Viewport(Event::Settled));
        let settled = frame(&mut snapshot, &app);
        let after = shown(&app);
        report("yakınlaşma duruldu", &[settled], after.len());
        println!(
            "    durulunca {} yazı belirdi, {} yazı kayboldu",
            after.difference(&before).count(),
            before.difference(&after).count()
        );
        // Ölçülendirme (Açıklama › Ölçü): two points and the dimension's place; the frame after it is written.
        update(&mut app, Message::Run("tool.dimension"));
        for (fx, fy) in [(0.35, 0.5), (0.65, 0.5)] {
            let p = at(&app, fx, fy);
            snapshot.input(&mut app, App::view, &mut update, Input::Move(p));
            snapshot.input(&mut app, App::view, &mut update, Input::Click(p));
        }
        let place = at(&app, 0.5, 0.42);
        snapshot.input(&mut app, App::view, &mut update, Input::Move(place));
        let before = shown(&app);
        let t = Instant::now();
        update(&mut app, Message::Viewport(Event::Pressed(place)));
        update(&mut app, Message::Viewport(Event::Released(place)));
        let input = t.elapsed();
        let (took, made, cleared) = frame(&mut snapshot, &app);
        let written = (input + took, made, cleared);
        let after = shown(&app);
        report("ölçü yazıldıktan sonra", &[written], after.len());
        println!(
            "    ölçüyle {} yazı belirdi, {} yazı kayboldu",
            after.difference(&before).count(),
            before.difference(&after).count()
        );
        update(&mut app, Message::Run("tool.cancel"));
        update(&mut app, Message::Run("edit.undo"));
        snapshot.settle(&mut app, App::view, &mut update);
    }
}
