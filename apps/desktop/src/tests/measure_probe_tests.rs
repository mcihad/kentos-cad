//! A probe for the owner's report of 23 October: after importing an AutoCAD drawing, Mesafe ölç froze while
//! measuring a length. The drawing is imported as the window imports it (`file.import.dwg` / `.dxf`), the view fitted,
//! Mesafe ölç started and the pointer moved and pressed through the widgets, each step timed. Run by hand:
//!
//! ```text
//! KENTOS_PROBE_FILE=~/Downloads/fast-food-Restaurant.dwg \
//!   cargo test -p kentos-desktop --lib measure_probe -- --ignored --nocapture
//! ```

use std::time::Instant;

use iced::{Point, Size, mouse};

use crate::app::{App, Message};

fn step(
    snapshot: &mut kentos_ui::snapshot::Snapshot,
    app: &mut App,
    events: &[iced::Event],
) -> f64 {
    let mut update = |app: &mut App, message| {
        let _ = app.update(message);
    };
    let t = Instant::now();
    snapshot.step(app, App::view, &mut update, events);
    t.elapsed().as_secs_f64() * 1000.0
}

#[test]
#[ignore = "a probe run by hand with KENTOS_PROBE_FILE"]
fn measure_probe() {
    let Ok(path) = std::env::var("KENTOS_PROBE_FILE") else {
        return;
    };
    let explode = std::env::var("KENTOS_PROBE_EXPLODE").is_ok();
    let mut app = crate::files_testing::app_with_drawing();
    let size = Size::new(1440.0, 900.0);
    let _ = app.update(Message::WindowResized(size));
    let command = if path.to_lowercase().ends_with(".dwg") {
        "file.import.dwg"
    } else {
        "file.import.dxf"
    };
    if path.ends_with(".kcad") {
        let bytes = std::fs::read(&path).expect("the drawing");
        let snapshot = kentos_kcad::decode(&bytes)
            .map_err(|e| e.message)
            .expect("decodes");
        let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
        let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
        let _ = app.update(Message::WindowResized(size));
    }
    let t = Instant::now();
    let answer = if path.ends_with(".kcad") {
        Ok(serde_json::json!("açıldı"))
    } else {
        let doc = app.document.as_mut().expect("a drawing");
        kentos_native_application::registry::run(
            &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
            command,
            kentos_commands::CallOp::Execute,
            serde_json::json!({ "path": path, "explodeBlocks": explode, "allowPolylineLoss": true }),
        )
    };
    println!(
        "içe aktarma {:.0} ms: {}",
        t.elapsed().as_secs_f64() * 1000.0,
        answer
            .as_ref()
            .map(|v| v.to_string().chars().take(300).collect::<String>())
            .unwrap_or_else(|e| format!("{e:?}"))
    );
    let mut snapshot = crate::files_testing::offscreen(size);
    let mut update = |app: &mut App, message| {
        let _ = app.update(message);
    };
    let t = Instant::now();
    snapshot.settle(&mut app, App::view, &mut update);
    println!("ilk kare {:.0} ms", t.elapsed().as_secs_f64() * 1000.0);
    let _ = app.update(Message::Run("view.zoomExtents"));
    let t = Instant::now();
    snapshot.settle(&mut app, App::view, &mut update);
    println!("sığdırma {:.0} ms", t.elapsed().as_secs_f64() * 1000.0);
    let tool: &'static str = match std::env::var("KENTOS_PROBE_TOOL").as_deref() {
        Ok("dimension") => "tool.dimension",
        _ => "tool.measure",
    };
    let _ = app.update(Message::Run(tool));
    let b = app.viewport.bounds;
    println!("çizim alanı {b:?}");
    // KENTOS_PROBE_AT="ax,ay,bx,by": two world points (the user's last dimension's ends): the view on them, the tool
    // pressed on each and a third time beside them, as Doğrusal ölçü takes them.
    if let Ok(at) = std::env::var("KENTOS_PROBE_AT") {
        let v: Vec<f64> = at
            .split(',')
            .filter_map(|t| t.trim().parse().ok())
            .collect();
        let (pa, pb) = (
            kentos_render_wgpu::Vec2::new(v[0], v[1]),
            kentos_render_wgpu::Vec2::new(v[2], v[3]),
        );
        let span = ((v[2] - v[0]).hypot(v[3] - v[1])).max(0.5) * 3.0;
        app.viewport.camera.fit_size(
            kentos_render_wgpu::Vec2::new((v[0] + v[2]) / 2.0, (v[1] + v[3]) / 2.0),
            span,
            span,
            0.0,
        );
        snapshot.settle(&mut app, App::view, &mut update);
        let screen = |app: &App, w: kentos_render_wgpu::Vec2| {
            let [x, y] = app.viewport.camera.world_to_screen(w);
            Point::new(b.x + x as f32, b.y + y as f32)
        };
        let click = |snapshot: &mut kentos_ui::snapshot::Snapshot, app: &mut App, p: Point| {
            let t = Instant::now();
            let mut update = |app: &mut App, message| {
                let _ = app.update(message);
            };
            snapshot.step(
                app,
                App::view,
                &mut update,
                &[
                    iced::Event::Mouse(mouse::Event::CursorMoved { position: p }),
                    iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                    iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
                ],
            );
            t.elapsed().as_secs_f64() * 1000.0
        };
        for round in 0..3 {
            let a = screen(&app, pa);
            let bb = screen(&app, pb);
            println!(
                "tur {round}: a tık {:.0} ms",
                click(&mut snapshot, &mut app, a)
            );
            println!(
                "tur {round}: b tık {:.0} ms",
                click(&mut snapshot, &mut app, bb)
            );
            let side = Point::new(bb.x + 60.0, (a.y + bb.y) / 2.0);
            for k in 0..5 {
                let p = Point::new(side.x + k as f32 * 4.0, side.y);
                println!(
                    "tur {round}: yer hareketi {k} {:.0} ms",
                    step(
                        &mut snapshot,
                        &mut app,
                        &[iced::Event::Mouse(mouse::Event::CursorMoved {
                            position: p
                        })]
                    )
                );
            }
            println!(
                "tur {round}: yer tık {:.0} ms",
                click(&mut snapshot, &mut app, side)
            );
            println!(
                "tur {round}: ileti {}",
                crate::files_testing::last_said(&app)
            );
        }
        return;
    }
    let at = |fx: f32, fy: f32| Point::new(b.x + b.width * fx, b.y + b.height * fy);
    let mut worst = 0.0f64;
    let mut moves = 0;
    let mut total = 0.0;
    for (i, (fx, fy)) in [
        (0.3, 0.3),
        (0.32, 0.31),
        (0.35, 0.33),
        (0.4, 0.4),
        (0.45, 0.42),
        (0.5, 0.5),
    ]
    .iter()
    .enumerate()
    {
        let ms = step(
            &mut snapshot,
            &mut app,
            &[iced::Event::Mouse(mouse::Event::CursorMoved {
                position: at(*fx, *fy),
            })],
        );
        println!("hareket {i}: {ms:.0} ms");
        worst = worst.max(ms);
        total += ms;
        moves += 1;
    }
    let press = |snapshot: &mut kentos_ui::snapshot::Snapshot, app: &mut App, p: Point| {
        step(
            snapshot,
            app,
            &[
                iced::Event::Mouse(mouse::Event::CursorMoved { position: p }),
                iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            ],
        )
    };
    println!(
        "ilk tık: {:.0} ms",
        press(&mut snapshot, &mut app, at(0.3, 0.3))
    );
    for (i, fx) in [0.35f32, 0.45, 0.55, 0.65, 0.7].iter().enumerate() {
        let ms = step(
            &mut snapshot,
            &mut app,
            &[iced::Event::Mouse(mouse::Event::CursorMoved {
                position: at(*fx, 0.6),
            })],
        );
        println!("ölçerken hareket {i}: {ms:.0} ms");
        worst = worst.max(ms);
        total += ms;
        moves += 1;
    }
    println!(
        "ikinci tık: {:.0} ms",
        press(&mut snapshot, &mut app, at(0.7, 0.6))
    );
    // The whole area swept on a grid, the pointer still measuring from the first point (the second press ended the
    // first measurement; a third starts another).
    let _ = press(&mut snapshot, &mut app, at(0.5, 0.5));
    let mut slow: Vec<(f64, f32, f32)> = Vec::new();
    let t = Instant::now();
    for j in 0..25 {
        for i in 0..40 {
            let (fx, fy) = ((i as f32 + 0.5) / 40.0, (j as f32 + 0.5) / 25.0);
            let ms = step(
                &mut snapshot,
                &mut app,
                &[iced::Event::Mouse(mouse::Event::CursorMoved {
                    position: at(fx, fy),
                })],
            );
            slow.push((ms, fx, fy));
            if ms > 2000.0 {
                println!("YAVAŞ: {ms:.0} ms ({fx:.3}, {fy:.3})");
            }
        }
    }
    slow.sort_by(|a, b| b.0.total_cmp(&a.0));
    println!(
        "ızgara {:.1} s; en yavaş beş: {:?}",
        t.elapsed().as_secs_f64(),
        &slow[..5]
    );
    println!(
        "en kötü hareket {worst:.0} ms, ortalama {:.0} ms; son ileti: {}",
        total / f64::from(moves),
        crate::files_testing::last_said(&app)
    );
}
