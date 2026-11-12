//! The pictures of docs/adr/0285: Görünümler with three named views, a view twisted 30° (the parcels, their
//! numbers and the street's name turned with it, the axes' icon and a polyline being drawn), and the area split in
//! two views, the right one active with a tool running. `tools_screens` takes them in both themes at 1440×900 and
//! 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=gorunumler,gorunum-dondurulmus,gorunum-bolunmus cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! And docs/adr/0523's: the two views linked, each with a theme of its own (the parcels in blue, thick; the road
//! alone), and Kaydırmalı karşılaştır over them (`gorunum-tema`, `gorunum-karsilastir`).
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
    let _ = app
        .settings
        .choose(&[("drafting.snap", serde_json::Value::Bool(false))]);
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
    let _ = app.update(Message::OtherViewport(0, crate::viewport::Event::Resized(
        iced::Rectangle::new(
            iced::Point::new(bounds.x + half.width + 2.0, bounds.y),
            half,
        ),
    )));
    if let Some(s) = app.split.as_mut() {
        s.others[0].viewport.camera.fit(
            &kentos_render_wgpu::Bounds {
                min_x: E + 10.0,
                min_y: N + 4.0,
                max_x: E + 34.0,
                max_y: N + 24.0,
            },
            24.0,
        );
        s.others[0].viewport.camera.twist = std::f64::consts::FRAC_PI_6;
    }
    let at = app
        .split
        .as_ref()
        .map(|s| {
            let [x, y] = s
                .others[0]
                .viewport
                .camera
                .world_to_screen(kentos_render_wgpu::Vec2::new(E + 20.0, N + 16.0));
            iced::Point::new(x as f32, y as f32)
        })
        .unwrap_or_default();
    run(app, "tool.line");
    let _ = app.update(Message::OtherViewport(0, crate::viewport::Event::Pressed(at)));
    let _ = app.update(Message::Viewport(crate::viewport::Event::Released(at)));
    hover(app, [30.0, 22.0]);
}

/// Two layer states of the block (docs/adr/0523 §1): Kadastro (the parcels in blue and thick, the road hidden)
/// and Yol (the road alone).
pub(crate) fn themes(app: &mut App) {
    let Some(doc) = app.document.as_mut() else {
        return;
    };
    let parts = kentos_domain::layer_states::LayerStateParts {
        locks: false,
        styles: true,
    };
    let tree = doc.model.layers().nodes().to_vec();
    let mut cadastre = kentos_domain::layer_states::capture(&tree, "durum-1", "Kadastro", parts);
    let mut road = kentos_domain::layer_states::capture(&tree, "durum-2", "Yol", parts);
    for n in &mut cadastre.nodes {
        if n.node == "yol" {
            n.visible = false;
        }
        if n.node == "parsel"
            && let Some(style) = n.style.as_mut()
        {
            style.color = "#3B82F6".into();
            style.line_weight = 0.7;
        }
    }
    for n in &mut road.nodes {
        if n.node == "parsel" || n.node == "cizim" {
            n.visible = false;
        }
    }
    let mut settings = doc.model.settings().clone();
    settings.layer_states = vec![cadastre, road];
    doc.model.set_settings(settings);
}

/// Halves the area as the window lays it out, the active view on the left.
fn halves(app: &mut App) {
    let bounds = app.viewport.bounds;
    let half = iced::Size::new((bounds.width - 2.0) / 2.0, bounds.height);
    let _ = app.update(Message::Viewport(crate::viewport::Event::Resized(
        iced::Rectangle::new(bounds.position(), half),
    )));
    let _ = app.update(Message::OtherViewport(0, crate::viewport::Event::Resized(
        iced::Rectangle::new(
            iced::Point::new(bounds.x + half.width + 2.0, bounds.y),
            half,
        ),
    )));
}

/// Linked views with themes: Kadastro on the left, active; Yol on the right, at the same place.
fn themed(app: &mut App) {
    block(app);
    themes(app);
    run(app, "view.split");
    halves(app);
    run(app, "view.splitLink");
    let theme = |active: bool, state| {
        Message::Split(crate::split_view::Event::Theme {
            view: (!active).then_some(0),
            state: Some(state),
        })
    };
    let _ = app.update(theme(true, 0));
    let _ = app.update(theme(false, 1));
}

/// Kaydırmalı karşılaştır: Kadastro left of the divider, Yol right of it, at 45 %.
fn swiped(app: &mut App) {
    block(app);
    themes(app);
    run(app, "view.swipe");
    let theme = |active: bool, state| {
        Message::Split(crate::split_view::Event::Theme {
            view: (!active).then_some(0),
            state: Some(state),
        })
    };
    let _ = app.update(theme(true, 0));
    let _ = app.update(theme(false, 1));
    if let Some(s) = app.split.as_mut() {
        s.swipe = Some(0.45);
    }
    hover(app, [12.0, 8.0]);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("gorunumler", named_views),
        ("gorunum-dondurulmus", twisted),
        ("gorunum-bolunmus", split),
        ("gorunum-tema", themed),
        ("gorunum-karsilastir", swiped),
    ]
}
