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

use iced::{Point, Rectangle, Size, Vector};
use kentos_ui::snapshot::{Image, Input, Snapshot};

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

/// The file `KENTOS_PROBE_FILE` names open in a window of 1440 × 900, settled; none named, none.
fn opened() -> Option<(Snapshot, App)> {
    let Some(path) = std::env::var_os("KENTOS_PROBE_FILE") else {
        println!("KENTOS_PROBE_FILE verilmedi");
        return None;
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
    Some((snapshot, app))
}

#[test]
#[ignore = "a probe on a file of one's own, run by hand"]
fn label_probe() {
    let Some((mut snapshot, mut app)) = opened() else {
        return;
    };
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
        // Mid-pan, before it settles: the texts on their ground (docs/adr/0626 §7).
        save(&mut snapshot, &app, &format!("yazi-boyu-{size}-kayarken"));
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

/// How far two pictures of the window differ over an area: its pixels, those whose largest channel differs at all, by
/// more than 24 of 255 (seen) and by more than 64 (plain), and the largest difference.
struct Difference {
    pixels: usize,
    any: usize,
    seen: usize,
    plain: usize,
    largest: u8,
}

/// The difference of `a` and `b` over `area`, and its picture: `b` dimmed, a pixel that differs by more than 24 in
/// magenta, by less in a dim yellow.
fn difference(a: &Image, b: &Image, area: Rectangle) -> (Difference, Image) {
    let mut out = b.clone();
    let mut d = Difference {
        pixels: 0,
        any: 0,
        seen: 0,
        plain: 0,
        largest: 0,
    };
    let (x0, y0) = (area.x.max(0.0) as u32, area.y.max(0.0) as u32);
    let x1 = ((area.x + area.width) as u32).min(a.width);
    let y1 = ((area.y + area.height) as u32).min(a.height);
    for y in y0..y1 {
        for x in x0..x1 {
            let i = ((y * a.width + x) * 4) as usize;
            let most = (0..3)
                .map(|c| a.rgba[i + c].abs_diff(b.rgba[i + c]))
                .max()
                .unwrap_or(0);
            d.pixels += 1;
            d.largest = d.largest.max(most);
            let paint: [u8; 3] = if most > 24 {
                d.any += 1;
                d.seen += 1;
                if most > 64 {
                    d.plain += 1;
                }
                [255, 0, 255]
            } else if most > 0 {
                d.any += 1;
                [120, 110, 0]
            } else {
                [b.rgba[i] / 3, b.rgba[i + 1] / 3, b.rgba[i + 2] / 3]
            };
            out.rgba[i..i + 3].copy_from_slice(&paint);
        }
    }
    (d, out)
}

/// The window as the moving view draws it, against the same view once it settles and its texts are laid out
/// again: their difference, and the three pictures (`.run/shots/yazi-kalite-<name>-{hareket,durgun,fark}.png`).
/// A pan of nothing first: the view counts as moving for a while after its last step (viewport.rs), and a slow
/// frame would otherwise draw the settled view twice.
fn against_settled(snapshot: &mut Snapshot, app: &mut App, name: &str) {
    let middle = at(app, 0.5, 0.5);
    update(
        app,
        Message::Viewport(Event::Panned {
            by: Vector::ZERO,
            at: middle,
        }),
    );
    assert!(app.viewport.moving(), "{name}: the view moves");
    let moving = snapshot.render(app.view(), &app.theme());
    update(app, Message::Viewport(Event::Settled));
    let settled = snapshot.render(app.view(), &app.theme());
    let (d, picture) = difference(&moving, &settled, app.viewport.bounds);
    println!(
        "  {name}: alanın {} pikselinden {} tanesi farklı ({:.3} %), {} tanesi gözle seçilir (> 24/255), {} tanesi \
         belirgin (> 64/255); en büyük fark {}",
        d.pixels,
        d.any,
        d.any as f64 * 100.0 / d.pixels.max(1) as f64,
        d.seen,
        d.plain,
        d.largest
    );
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    let _ = std::fs::create_dir_all(&out);
    for (image, word) in [
        (&moving, "hareket"),
        (&settled, "durgun"),
        (&picture, "fark"),
    ] {
        let _ = image.save(out.join(format!("yazi-kalite-{name}-{word}.png")));
    }
}

/// A pan of `steps` steps of `by` pixels, each frame drawn as the window draws it.
fn pan(snapshot: &mut Snapshot, app: &mut App, by: Vector, steps: usize) {
    for _ in 0..steps {
        let middle = at(app, 0.5, 0.5);
        update(app, Message::Viewport(Event::Panned { by, at: middle }));
        let _ = snapshot.render(app.view(), &app.theme());
    }
}

/// `notches` wheel notches at `(fx, fy)` of the area, in (positive) or out, each frame drawn.
fn wheel(snapshot: &mut Snapshot, app: &mut App, notches: i32, fx: f32, fy: f32) {
    for _ in 0..notches.abs() {
        let p = at(app, fx, fy);
        snapshot.input(
            app,
            App::view,
            &mut update,
            Input::Scroll(p, notches.signum() as f32),
        );
        let _ = snapshot.render(app.view(), &app.theme());
    }
}

/// The quality of the texts while the view moves (docs/adr/0626 §7, the owner's 24 October word: faster, but no
/// worse): the window mid-pan and mid-wheel, under true sizes, against the same view drawn afresh once it settles.
/// A pan moves the last picture of the texts as it is, so it should not differ; a wheel scales it, and a text
/// crossing the band's size changes only once the view settles. Run by hand, in release:
///
/// ```text
/// KENTOS_PROBE_FILE=$HOME/Downloads/deneme_suşehri.dxf cargo test --release -p kentos-desktop --bin kentos-cad label_quality -- --ignored --nocapture
/// ```
#[test]
#[ignore = "a probe on a file of one's own, run by hand"]
fn label_quality() {
    let Some((mut snapshot, mut app)) = opened() else {
        return;
    };
    let _ = app
        .settings
        .choose(&[("graphics.annotationSize", serde_json::Value::from("true"))]);
    app.apply_settings();
    println!("Gerçek boy, hareket eden görünüm durulunca çizilenle:");
    working_view(&mut snapshot, &mut app);
    pan(&mut snapshot, &mut app, Vector::new(25.0, 0.0), 12);
    against_settled(&mut snapshot, &mut app, "kaydirma");
    pan(&mut snapshot, &mut app, Vector::new(-37.5, 22.25), 9);
    against_settled(&mut snapshot, &mut app, "kaydirma-capraz");
    pan(&mut snapshot, &mut app, Vector::new(180.0, 0.0), 4);
    against_settled(&mut snapshot, &mut app, "kaydirma-uzun");
    for notches in [1, 2, 4, -1, -2, -4] {
        working_view(&mut snapshot, &mut app);
        wheel(&mut snapshot, &mut app, notches, 0.4, 0.6);
        let name = if notches > 0 {
            format!("tekerlek-yakin-{notches}")
        } else {
            format!("tekerlek-uzak-{}", -notches)
        };
        against_settled(&mut snapshot, &mut app, &name);
    }
    // Close, where the letters read: a pan, and a pan with a wheel notch.
    working_view(&mut snapshot, &mut app);
    wheel(&mut snapshot, &mut app, 5, 0.5, 0.5);
    update(&mut app, Message::Viewport(Event::Settled));
    snapshot.settle(&mut app, App::view, &mut update);
    pan(&mut snapshot, &mut app, Vector::new(-20.0, 12.0), 10);
    against_settled(&mut snapshot, &mut app, "yakin-kaydirma");
    pan(&mut snapshot, &mut app, Vector::new(15.0, -9.0), 6);
    wheel(&mut snapshot, &mut app, 1, 0.55, 0.45);
    against_settled(&mut snapshot, &mut app, "yakin-tekerlek");
}
