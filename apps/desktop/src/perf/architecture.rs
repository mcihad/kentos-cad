//! ARCH-PERF-01: real Iced pointer events, measured apart from Python/FFI plans.
//! Uses the existing frame harness. This is an offscreen runtime measurement;
//! gpuReadback includes image readback and is NOT a window-present/GPU timestamp.
//! KENTOS_ARCH_PERF_FILE must name an isolated fixture, never a user's live file.
use std::path::PathBuf;

use iced::{Event, Point, mouse};
use serde_json::json;

use super::frame::{Harness, WINDOW, moved, summary};
use crate::app::{App, Message};

fn at(app: &App, x: f64, y: f64) -> Point {
    let [x, y] = app
        .viewport
        .camera
        .world_to_screen(kentos_interaction::Vec2::new(x, y));
    Point::new(x as f32, y as f32)
}

#[test]
#[ignore = "architectural runtime measurement; run manually in release with an isolated file"]
fn moving_preview() {
    let Some(path) = std::env::var_os("KENTOS_ARCH_PERF_FILE") else {
        panic!("KENTOS_ARCH_PERF_FILE must name an isolated benchmark fixture");
    };
    let path = PathBuf::from(path);
    let output =
        PathBuf::from(std::env::var_os("KENTOS_ARCH_PERF_OUTPUT").expect("explicit report path"));
    let origin: [f64; 2] = serde_json::from_str(
        &std::env::var("KENTOS_ARCH_PERF_ORIGIN").unwrap_or_else(|_| "[500000,4400000]".into()),
    )
    .expect("two origin coordinates");
    let samples: usize = std::env::var("KENTOS_ARCH_PERF_SAMPLES")
        .unwrap_or_else(|_| "60".into())
        .parse()
        .expect("sample count");
    assert!(samples >= 20);
    let doc = crate::document::Document::read(&path).expect("benchmark KCAD opens");
    let before = doc.model.to_snapshot_v2();
    let count = doc.entity_count();
    let (mut app, _) = App::boot(None);
    app.window_size = WINDOW;
    let _ = app.settings.choose(&[
        ("appearance.textSize", json!(13)),
        ("appearance.theme", json!("dark")),
        ("drafting.ortho", json!(false)),
        ("drafting.polar", json!(false)),
    ]);
    app.apply_settings();
    let mut h = Harness::new();
    h.settle(&mut app);
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    let _ = app.update(Message::DialogClosed);
    h.settle(&mut app);
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: origin[0] - 2.,
            min_y: origin[1] - 2.,
            max_x: origin[0] + 6.,
            max_y: origin[1] + 5.,
        },
        24.,
    );
    h.settle(&mut app);
    app.tab = "arch";
    let mut rows = Vec::new();
    for snap in [false, true] {
        let _ = app.settings.choose(&[("drafting.snap", json!(snap))]);
        app.apply_settings();
        for tool in ["tool.wall", "tool.wallRectangle"] {
            let _ = app.update(Message::Run("tool.select"));
            h.settle(&mut app);
            h.frame_with(&mut app, &[], |app| {
                let _ = app.update(Message::Run(tool));
            });
            let first = at(&app, origin[0], origin[1]);
            h.frame(
                &mut app,
                &[
                    moved(first),
                    Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                    Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
                ],
            );
            assert_eq!(
                app.session.point_count(),
                1,
                "first corner was not received by {tool}"
            );
            let points: Vec<_> = (0..samples)
                .map(|i| {
                    let angle = i as f64 * std::f64::consts::TAU / 32.;
                    at(
                        &app,
                        origin[0] + 4. + 0.2 * angle.cos(),
                        origin[1] + 3. + 0.2 * angle.sin(),
                    )
                })
                .collect();
            let cold = h.frame(&mut app, &[moved(points[0])]);
            for p in points.iter().take(10) {
                h.frame(&mut app, &[moved(*p)]);
            }
            let frames: Vec<_> = points
                .iter()
                .map(|p| h.frame(&mut app, &[moved(*p)]))
                .collect();
            rows.push(
                json!({"tool":tool,"snap":snap,"cold":summary(&[cold]),"warm":summary(&frames)}),
            );
            let _ = app.update(Message::Run("tool.select"));
            h.settle(&mut app);
            assert_eq!(
                app.document.as_ref().unwrap().model.to_snapshot_v2(),
                before,
                "a pointer-only scenario wrote the drawing"
            );
        }
    }
    let report = json!({"format":"kentos.architecture-runtime-preview","version":1,
        "environment":super::environment(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")),
        "file":path,"entities":count,"origin":origin,"window":[WINDOW.width,WINDOW.height],
        "textSize":13,"profile":if cfg!(debug_assertions){"debug"}else{"release"},
        "renderer":iced::advanced::renderer::Headless::name(&h.renderer),
        "method":"Iced runtime pointer events and offscreen redraw. GPU field includes readback; no window-present or timestamp-query claim.",
        "documentUnchanged":true,"rows":rows});
    std::fs::create_dir_all(output.parent().expect("report folder"))
        .expect("creates report folder");
    std::fs::write(&output, serde_json::to_vec_pretty(&report).unwrap()).expect("writes report");
    println!("ARCH_PREVIEW_RUNTIME {}", output.display());
}
