//! A running tool's preview while the view moves (docs/adr/9251). The owner, 26 November 2026: a line, a polyline or
//! a wall begun, the wheel turned before the next click, and the lines were gone: the rubber band, and a polyline's or
//! a wall's edges drawn so far, which are the preview until the command is confirmed.
//!
//! Each case begins a tool through the window's own events, rests the pointer with a preview on the screen and moves
//! the view as a user does: a notch of the wheel, a touchpad's pixels, a pan with the middle button, Yakınlaştır. The
//! frame drawn at once, the gesture still running on the area's live camera, is checked twice: the preview's lines are
//! on it where that camera puts them, and it is the picture the same app draws once the view has settled and been built
//! for that camera. The pictures go to `.run/gui-evidence/render/onizleme/` (never committed), each with its preview's
//! box enlarged three times.
//!
//! ```text
//! KENTOS_GPU_TESTS=1 RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib render_preview -- --nocapture --test-threads=1
//! ```

use iced::{Event, Point, mouse};
use kentos_render_wgpu::{Camera, Vec2};

use crate::app::{App, Message};
use crate::render_testing::{CHANNEL, area, booted, compare, keep, on_gpu, still};
use crate::tools_scenes::{E, N, Objects, run};
use crate::viewport::live::SETTLE;
use crate::wall_scenes::{fit, opened, wall_tool};
use crate::window_testing::{self, WINDOW, moved};

/// The points the tools are given and where the pointer rests: around the middle of the view `fit` frames, so a zoom
/// about the middle keeps them on the screen. From `B` to the pointer the band runs slanted: apart from the
/// crosshair's arms.
const A: [f64; 2] = [1.0, 0.5];
const B: [f64; 2] = [2.5, 2.0];
const POINTER: [f64; 2] = [4.5, 3.5];

/// How the view is moved while the tool waits for its next point.
#[derive(Clone, Copy, Debug)]
enum Motion {
    /// The wheel turned by lines (a notch is one), at the pointer.
    Wheel(f32),
    /// A touchpad's two fingers: pixels, at the pointer.
    Touchpad(f32),
    /// The middle button pressed at the pointer and dragged by whole pixels.
    Pan(f32, f32),
    /// Yakınlaştır, the app's own zoom about the middle (the ribbon's button, a key).
    ZoomIn,
}

/// A tool given `A` and `B`, the pointer resting at `POINTER`, then the view moved. The edge from `A` to `B` is the
/// preview of a polyline and a wall until the command is confirmed, a line's object written at the click: either way
/// it stays on the screen, and so does the band from `B` to the pointer.
struct Case {
    name: &'static str,
    /// The tool's command.
    tool: &'static str,
    motion: Motion,
}

/// A point of the drawing (east and north of its origin) in the window, as the app's camera puts it.
fn window(app: &App, p: [f64; 2]) -> Point {
    let b = app.viewport.bounds;
    let [x, y] = app.viewport.camera.world_to_screen(Vec2::new(E + p[0], N + p[1]));
    Point::new(b.x + x as f32, b.y + y as f32)
}

/// The point of the drawing (east and north of its origin) under a place of the window, as `camera` shows it: where
/// the band runs to, the pointer being there.
fn under(app: &App, camera: &Camera, at: Point) -> [f64; 2] {
    let b = app.viewport.bounds;
    let w = camera.screen_to_world(f64::from(at.x - b.x), f64::from(at.y - b.y));
    [w.x - E, w.y - N]
}

/// The places along the line from `a` to `b` (drawing points) that `picture` leaves bare as `camera` shows them: at a
/// fifth of the way, two fifths and so on, no pixel within one of the place is off the drawing area's ground.
fn bare(app: &App, picture: &[u8], camera: &Camera, a: [f64; 2], b: [f64; 2]) -> Vec<[f32; 2]> {
    let ground = crate::viewport::palette(app.canvas()).background.0;
    let bounds = app.viewport.bounds;
    let w = WINDOW.width as usize;
    let inked = |x: i64, y: i64| {
        let i = (y as usize * w + x as usize) * 4;
        (0..3).any(|c| picture[i + c].abs_diff(ground[c]) > CHANNEL)
    };
    let mut out = Vec::new();
    for k in 1..5 {
        let t = f64::from(k) / 5.0;
        let p = Vec2::new(E + a[0] + (b[0] - a[0]) * t, N + a[1] + (b[1] - a[1]) * t);
        let [x, y] = camera.world_to_screen(p);
        let (x, y) = (f64::from(bounds.x) + x, f64::from(bounds.y) + y);
        let near = (-1..=1).any(|dy| (-1..=1).any(|dx| inked(x.round() as i64 + dx, y.round() as i64 + dy)));
        if !near {
            out.push([x as f32, y as f32]);
        }
    }
    out
}

/// The box round the preview's points in the window, as `camera` shows them, 48 pixels wider each way, inside the area.
fn preview_box(app: &App, camera: &Camera, end: [f64; 2]) -> [usize; 4] {
    let b = app.viewport.bounds;
    let r = area(app);
    let (mut lo, mut hi) = ([f64::MAX; 2], [f64::MIN; 2]);
    for p in [A, B, end] {
        let [x, y] = camera.world_to_screen(Vec2::new(E + p[0], N + p[1]));
        let q = [f64::from(b.x) + x, f64::from(b.y) + y];
        for i in 0..2 {
            lo[i] = lo[i].min(q[i]);
            hi[i] = hi[i].max(q[i]);
        }
    }
    let clamp = |v: f64, min: usize, max: usize| (v.max(min as f64) as usize).min(max);
    [
        clamp(lo[0] - 48.0, r[0], r[2]),
        clamp(lo[1] - 48.0, r[1], r[3]),
        clamp(hi[0] + 48.0, r[0], r[2]),
        clamp(hi[1] + 48.0, r[1], r[3]),
    ]
}

/// A region of a window picture enlarged three times, kept as `<name>.png` in `.run/gui-evidence/render/onizleme/`.
fn keep_enlarged(name: &str, picture: &[u8], r: [usize; 4]) {
    const K: usize = 3;
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/render/onizleme");
    let _ = std::fs::create_dir_all(&dir);
    let w = WINDOW.width as usize;
    let (cw, ch) = (r[2] - r[0], r[3] - r[1]);
    let mut rgba = Vec::with_capacity(cw * ch * K * K * 4);
    for y in 0..ch * K {
        for x in 0..cw * K {
            let i = ((r[1] + y / K) * w + r[0] + x / K) * 4;
            rgba.extend_from_slice(&picture[i..i + 4]);
        }
    }
    let image = kentos_ui::snapshot::Image { width: (cw * K) as u32, height: (ch * K) as u32, rgba };
    let file = dir.join(format!("{name}.png"));
    if image.save(&file).is_ok() {
        println!("resim {}", file.display());
    }
}

fn wheel(delta: mouse::ScrollDelta) -> Event {
    Event::Mouse(mouse::Event::WheelScrolled { delta })
}

/// The case run: `None` when the preview is on every frame, what is missing otherwise.
fn check(case: &Case) -> Option<String> {
    let name = case.name;
    let (mut app, mut h) = booted();
    opened(&mut app, Objects::new(), 50.0);
    still(&mut app, &mut h);
    fit(&mut app, 7.0, 5.0);
    // The clicks land where they are given: no snap, no tracking, no ortho or polar steps.
    app.draft.snap = false;
    app.draft.tracking = false;
    app.draft.ortho = false;
    app.draft.polar = None;
    still(&mut app, &mut h);
    if case.tool == "tool.wall" {
        wall_tool(&mut app, "Dış duvar 25");
    } else {
        run(&mut app, case.tool);
    }
    still(&mut app, &mut h);
    for p in [A, B] {
        let at = window(&app, p);
        h.frame(&mut app, &window_testing::click(at));
    }
    let pointer = window(&app, POINTER);
    h.frame(&mut app, &[moved(pointer)]);
    let before = still(&mut app, &mut h);
    let built = app.viewport.camera;
    let mut problems = Vec::new();
    for (what, a, b) in [("the band", B, POINTER), ("the edge drawn", A, B)] {
        let missing = bare(&app, &before, &built, a, b);
        if !missing.is_empty() {
            problems.push(format!("{name}: {what} is not on the screen before the view moves, at {missing:?}"));
        }
    }

    // The view moved as a user moves it; the frame drawn at once. The pointer stays, or a pan's drag takes it along.
    let mut now = pointer;
    match case.motion {
        Motion::Wheel(lines) => {
            h.frame(&mut app, &[moved(pointer), wheel(mouse::ScrollDelta::Lines { x: 0.0, y: lines })]);
        }
        Motion::Touchpad(pixels) => {
            h.frame(&mut app, &[moved(pointer), wheel(mouse::ScrollDelta::Pixels { x: 0.0, y: pixels })]);
        }
        Motion::Pan(dx, dy) => {
            h.frame(&mut app, &[moved(pointer), Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Middle))]);
            now = Point::new(pointer.x + dx, pointer.y + dy);
            h.frame(&mut app, &[moved(now)]);
        }
        Motion::ZoomIn => {
            h.frame_with(&mut app, &[], |app| {
                let _ = app.update(Message::Run("view.zoomIn"));
            });
        }
    }
    let during = h.picture(&app);
    // The camera the frame was drawn with: the area's live one (the app's while no gesture runs).
    let shown = app.viewport.live.camera();
    let running = app.viewport.live.moving();
    if !matches!(case.motion, Motion::ZoomIn) && shown == built {
        problems.push(format!("{name}: the view did not move"));
    }
    let end = under(&app, &shown, now);
    for (what, a, b) in [("the band", B, end), ("the edge drawn", A, B)] {
        let missing = bare(&app, &during, &shown, a, b);
        if !missing.is_empty() {
            problems.push(format!(
                "{name}: {what} is gone while the view moves (gesture running: {running}), at {missing:?}"
            ));
        }
    }

    // The gesture let go and settled: the app takes the camera and builds the view for it.
    if let Motion::Pan(..) = case.motion {
        h.frame(&mut app, &[Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Middle))]);
    }
    std::thread::sleep(SETTLE + std::time::Duration::from_millis(60));
    let after = still(&mut app, &mut h);
    if app.viewport.live.moving() || app.viewport.camera != shown {
        problems.push(format!("{name}: the view did not settle at the camera it moved to"));
    }
    let settled = app.viewport.camera;
    let end = under(&app, &settled, now);
    for (what, a, b) in [("the band", B, end), ("the edge drawn", A, B)] {
        let missing = bare(&app, &after, &settled, a, b);
        if !missing.is_empty() {
            problems.push(format!("{name}: {what} is not on the screen once the view settled, at {missing:?}"));
        }
    }
    let r = area(&app);
    let d = compare(&during, &after, r);
    println!(
        "{name}: hareket {:?}, jest sürüyordu: {running}; {} piksel karşılaştırıldı, hareketteki kare durulan kareden \
         farklı {}, eşi olmayan {}",
        case.motion, d.compared, d.different, d.unmatched
    );
    if d.unmatched > 0 {
        problems.push(format!(
            "{name}: the frame drawn while the view moved is not the settled one: {} pixels have no match ({} not the \
             same)",
            d.unmatched, d.different
        ));
    }
    keep("onizleme", name, [("hareket", &during[..]), ("durunca", &after[..])], r);
    keep_enlarged(&format!("{name}-once-buyuk"), &before, preview_box(&app, &built, POINTER));
    keep_enlarged(&format!("{name}-hareket-buyuk"), &during, preview_box(&app, &shown, under(&app, &shown, now)));
    keep_enlarged(&format!("{name}-durunca-buyuk"), &after, preview_box(&app, &settled, end));
    (!problems.is_empty()).then(|| problems.join("\n"))
}

#[test]
fn a_tools_preview_stays_on_the_screen_while_the_view_moves() {
    if !on_gpu() {
        eprintln!("render_preview: KENTOS_GPU_TESTS=1 is not set; the GPU cases did not run");
        return;
    }
    let cases = [
        Case { name: "cizgi-tekerlek", tool: "tool.line", motion: Motion::Wheel(1.0) },
        Case { name: "cizgi-tekerlek-uzaklas", tool: "tool.line", motion: Motion::Wheel(-1.0) },
        Case { name: "cizgi-dokunmatik", tool: "tool.line", motion: Motion::Touchpad(60.0) },
        Case { name: "cizgi-kaydirma", tool: "tool.line", motion: Motion::Pan(40.0, -25.0) },
        Case { name: "cizgi-yakinlastir", tool: "tool.line", motion: Motion::ZoomIn },
        Case { name: "coklu-cizgi-tekerlek", tool: "tool.polyline", motion: Motion::Wheel(1.0) },
        Case { name: "coklu-cizgi-kaydirma", tool: "tool.polyline", motion: Motion::Pan(-30.0, 20.0) },
        Case { name: "duvar-tekerlek", tool: "tool.wall", motion: Motion::Wheel(1.0) },
        Case { name: "duvar-kaydirma", tool: "tool.wall", motion: Motion::Pan(35.0, 15.0) },
    ];
    let failed: Vec<String> = cases.iter().filter_map(check).collect();
    assert!(failed.is_empty(), "a tool's preview left the screen:\n{}", failed.join("\n"));
}
