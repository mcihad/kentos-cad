//! Where a pointer frame's GPU time goes on the mixed drawing (docs/adr/1064 §5): the drawing of `perf::mixed`
//! (`KENTOS_PERF_SIZES`, 10 000) opened, far (all of it) and near (1:200), the pointer moved over it as the fixed trace
//! moves it, each frame's GPU time beside what the view did for it: whether the picture was kept, the bytes uploaded,
//! the segments and draw calls of the picture. With `KENTOS_PERF_SHORTCUTS` as `perf::large` reads it.
//! `KENTOS_PERF_BEFORE` puts something before each view's pointer frames: `heavy` (40 frames drawn afresh with the
//! drawing area's shortcuts off, as a far pan of the plain build loads the GPU) or `idle` (3 s without a frame), to
//! tell the GPU's own clock from the frame's work (docs/adr/1064 §5). A diagnosis, not a correctness test and not
//! run by default:
//!
//! ```text
//! KENTOS_PERF_SHORTCUTS=instancing cargo test --release -p kentos-desktop --lib perf::cursor -- --ignored --nocapture
//! ```

use super::frame::{Harness, moved, path, percentile};
use super::large::{settings_from_env, shortcuts_from_env};
use super::mixed;
use crate::app::{App, Message};
use crate::document::Document;

/// 1:200 at 96 dpi, logical pixels per metre.
const NEAR: f64 = 1.0 / (200.0 * 0.000_264_58);

#[test]
#[ignore = "a diagnosis, run by hand in release mode"]
fn cursor() {
    let n: usize = std::env::var("KENTOS_PERF_SIZES")
        .ok()
        .and_then(|v| v.split(',').next().and_then(|s| s.trim().parse().ok()))
        .unwrap_or(10_000);
    let (snapshot, _) = mixed::drawing(n, mixed::SEED);
    let mut h = Harness::new();
    let (mut app, _) = App::boot(None);
    settings_from_env(&mut app);
    shortcuts_from_env(&mut app);
    h.settle(&mut app);
    let doc = Document::from_v2(snapshot, None).expect("the drawing opens");
    h.frame_with(&mut app, &[], move |app| {
        let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    });
    app.dialog = None;
    h.settle(&mut app);
    let e = mixed::extent(n);
    let mid = [(e[0] + e[2]) / 2.0, (e[1] + e[3]) / 2.0];
    for (name, near) in [("uzak", false), ("yakın 1:200", true)] {
        app.viewport.show(&kentos_render_wgpu::Bounds { min_x: e[0], min_y: e[1], max_x: e[2], max_y: e[3] });
        if near {
            app.viewport.camera.center.x = mid[0];
            app.viewport.camera.center.y = mid[1];
            app.viewport.camera.scale = NEAR;
        }
        h.frame(&mut app, &[]);
        h.settle(&mut app);
        match std::env::var("KENTOS_PERF_BEFORE").as_deref() {
            Ok("heavy") => {
                let (i, c, l) = (app.styles.instancing, app.styles.curve_lod, app.styles.line_lod);
                app.styles.instancing = false;
                app.styles.curve_lod = false;
                app.styles.line_lod = false;
                let c0 = app.viewport.camera;
                for k in 0..40 {
                    app.viewport.camera.center.x = c0.center.x + f64::from(k % 2) / c0.scale;
                    h.frame(&mut app, &[]);
                }
                app.viewport.camera = c0;
                app.styles.instancing = i;
                app.styles.curve_lod = c;
                app.styles.line_lod = l;
                h.frame(&mut app, &[]);
                h.settle(&mut app);
            }
            Ok("idle") => std::thread::sleep(std::time::Duration::from_secs(3)),
            _ => {}
        }
        let area = app.viewport.bounds;
        let mut gpu = Vec::new();
        for (i, p) in path(area, 60).into_iter().enumerate() {
            let parts = h.frame(&mut app, &[moved(p)]);
            let s = app.viewport.status().stats;
            gpu.push(parts.gpu_ms());
            if i < 12 || i % 10 == 0 {
                println!(
                    "{n} {name} imleç {i}: GPU {:.2} ms, MİB {:.2} ms; resim tutuldu {}, yüklenen {} B, parça {} çizim çağrısı {}",
                    parts.gpu_ms(),
                    parts.cpu(),
                    s.picture_kept,
                    s.uploaded_bytes,
                    s.segments,
                    s.draw_calls
                );
            }
        }
        println!(
            "{n} {name} imleç: GPU p50 {:.2} ms p95 {:.2} ms",
            percentile(&gpu, 0.5),
            percentile(&gpu, 0.95)
        );
    }
}

/// The far wheel's UI thread frame by frame (docs/adr/1064 §6): its parts (event, app, view, layout, draw) as the
/// fixed trace turns it, 8 in, 8 out, twice, settling between.
///
/// `cargo test --release -p kentos-desktop --lib perf::cursor::wheel -- --ignored --nocapture`
#[test]
#[ignore = "a diagnosis, run by hand in release mode"]
fn wheel() {
    let n: usize = std::env::var("KENTOS_PERF_SIZES")
        .ok()
        .and_then(|v| v.split(',').next().and_then(|s| s.trim().parse().ok()))
        .unwrap_or(100_000);
    let (snapshot, _) = mixed::drawing(n, mixed::SEED);
    let mut h = Harness::new();
    let (mut app, _) = App::boot(None);
    settings_from_env(&mut app);
    shortcuts_from_env(&mut app);
    h.settle(&mut app);
    let doc = Document::from_v2(snapshot, None).expect("the drawing opens");
    h.frame_with(&mut app, &[], move |app| {
        let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    });
    app.dialog = None;
    h.settle(&mut app);
    let e = mixed::extent(n);
    app.viewport.show(&kentos_render_wgpu::Bounds { min_x: e[0], min_y: e[1], max_x: e[2], max_y: e[3] });
    h.frame(&mut app, &[]);
    h.settle(&mut app);
    let center = app.viewport.bounds.center();
    h.frame(&mut app, &[moved(center)]);
    for round in 0..2 {
        for lines in [1.0f32, -1.0] {
            for i in 0..8 {
                let event = iced::Event::Mouse(iced::mouse::Event::WheelScrolled {
                    delta: iced::mouse::ScrollDelta::Lines { x: 0.0, y: lines },
                });
                let (made, cleared) = crate::label_picture::counts();
                let p = h.frame(&mut app, &[event]);
                let (made2, cleared2) = crate::label_picture::counts();
                let [ev, ap, vw, ly, dr] = p.split();
                println!(
                    "{n} tekerlek {round}/{lines}/{i}: MİB {:.2} ms (olay {ev:.2}, uygulama {ap:.2}, görünüm {vw:.2}, düzen {ly:.2}, çizim {dr:.2}), GPU {:.2} ms, ölçek {:.4}; yazı resmi yapılan {}, boşaltma {}",
                    p.cpu(),
                    p.gpu_ms(),
                    app.viewport.camera.scale,
                    made2 - made,
                    cleared2 - cleared
                );
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
        let p = h.frame(&mut app, &[]);
        println!("{n} tekerlek {round} duruş: MİB {:.2} ms, GPU {:.2} ms", p.cpu(), p.gpu_ms());
    }
}
