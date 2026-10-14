//! The pictures of docs/adr/0285: Görünümler with three named views, a view twisted 30° (the parcels, their
//! numbers and the street's name turned with it, the axes' icon and a polyline being drawn), and the area split in
//! two views, the right one active with a tool running. `tools_screens` takes them in both themes at 1440×900 and
//! 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=gorunumler,gorunum-dondurulmus,gorunum-bolunmus cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use crate::app::{App, Message};
use crate::files_testing::make_cad;
use crate::tools_scenes::{E, N, Objects, click, hover, open, run};
use crate::tools_screens::Scene;

/// A block of four parcels with their numbers, a street along their south and its name.
fn block(app: &mut App) {
    let mut o = Objects::new();
    let cells = [
        ([0.0, 0.0], "112/1"),
        ([20.0, 0.0], "112/2"),
        ([0.0, 16.0], "112/3"),
        ([20.0, 16.0], "112/4"),
    ];
    for ([x, y], name) in cells {
        o.path(
            "parsel",
            &[[x, y], [x + 20.0, y], [x + 20.0, y + 16.0], [x, y + 16.0]],
            true,
        );
        o.text("cizim", [x + 6.5, y + 7.0], name, 2.2, 0.0);
    }
    o.line("yol", [-8.0, -6.0], [48.0, -6.0]);
    o.line("yol", [-8.0, -12.0], [48.0, -12.0]);
    o.text("cizim", [8.0, -10.0], "Atatürk Caddesi", 2.0, 0.0);
    open(app, o);
    make_cad(app);
    let _ = app.settings.choose(&[("drafting.snap", serde_json::Value::Bool(false))]);
    app.apply_settings();
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 14.0,
            min_y: N - 18.0,
            max_x: E + 54.0,
            max_y: N + 38.0,
        },
        24.0,
    );
}

/// Görünümler: three views kept, the second chosen.
fn named_views(app: &mut App) {
    block(app);
    app.save_view("Bütün ada");
    app.twist_view(std::f64::consts::FRAC_PI_6);
    app.save_view("Cadde boyunca");
    app.twist_view(0.0);
    app.viewport.camera.zoom_at(2.5, 300.0, 300.0);
    app.save_view("112/3 yakın");
    run(app, "view.namedViews");
    let _ = app.update(Message::Views(crate::views::Event::Choose(
        "Cadde boyunca".into(),
    )));
}

/// The view twisted 30°: the street runs up the screen at 60°, the text turned with the drawing; a polyline being
/// drawn along the parcels' edge.
fn twisted(app: &mut App) {
    block(app);
    app.twist_view(std::f64::consts::FRAC_PI_6);
    run(app, "tool.polyline");
    click(app, [0.0, -2.0]);
    click(app, [40.0, -2.0]);
    hover(app, [44.0, 20.0]);
}

/// Split in two: the left view the whole block, the right one closer and twisted, active, with Çizgi from a corner.
fn split(app: &mut App) {
    block(app);
    run(app, "view.split");
    // The views' sizes as the window lays them out: half the area each.
    let bounds = app.viewport.bounds;
    let half = iced::Size::new((bounds.width - 2.0) / 2.0, bounds.height);
    let _ = app.update(Message::Viewport(crate::viewport::Event::Resized(
        iced::Rectangle::new(bounds.position(), half),
    )));
    let _ = app.update(Message::OtherViewport(crate::viewport::Event::Resized(
        iced::Rectangle::new(
            iced::Point::new(bounds.x + half.width + 2.0, bounds.y),
            half,
        ),
    )));
    if let Some(s) = app.split.as_mut() {
        s.viewport.camera.fit(
            &kentos_render_wgpu::Bounds {
                min_x: E + 10.0,
                min_y: N + 4.0,
                max_x: E + 34.0,
                max_y: N + 24.0,
            },
            24.0,
        );
        s.viewport.camera.twist = std::f64::consts::FRAC_PI_6;
    }
    let at = app
        .split
        .as_ref()
        .map(|s| {
            let [x, y] = s
                .viewport
                .camera
                .world_to_screen(kentos_render_wgpu::Vec2::new(E + 20.0, N + 16.0));
            iced::Point::new(x as f32, y as f32)
        })
        .unwrap_or_default();
    run(app, "tool.line");
    let _ = app.update(Message::OtherViewport(crate::viewport::Event::Pressed(at)));
    let _ = app.update(Message::Viewport(crate::viewport::Event::Released(at)));
    hover(app, [30.0, 22.0]);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("gorunumler", named_views),
        ("gorunum-dondurulmus", twisted),
        ("gorunum-bolunmus", split),
    ]
}
