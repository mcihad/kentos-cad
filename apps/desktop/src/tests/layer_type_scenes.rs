//! The pictures of docs/adr/0622 §5 (Katman türü and Otomatik seçim): a CBS drawing of typed layers (trees, roads,
//! parcels, texts) and one of any kind, their glyphs in the tree in their colours; Katman türü ▸ on a layer's menu;
//! Yeni katman ▾ in Veri; the Düzenle tab with the area layer active, the tools that draw lines off and Çizgi's
//! start refused with the reason; Otomatik seçim off, a click on a road saying why it took nothing and a box taking
//! the parcels alone; the switch in Proje ayarları. `tools_screens` takes them in both themes at 1440×900 and
//! 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=katman-turu,katman-turu-menu,katman-turu-yeni,katman-turu-kapali,oto-secim-tik,oto-secim-kutu,oto-secim-ayar cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use iced::Point;
use kentos_ui::snapshot::Input;
use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::tools_scenes::{E, N, Objects, at, click, hover, layer, open_edited, run};
use crate::tools_screens::{Pointed, Scene, caption_at, open_ribbon_menu, press_caption};
use crate::viewport::Event;

/// A layer with its type (none: Karışık).
fn typed(id: &str, name: &str, color: &str, weight: f64, geometry: Option<&str>) -> Value {
    let mut n = layer(id, name, color, weight);
    if let Some(g) = geometry {
        n["geometry"] = json!(g);
    }
    n
}

/// Four parcels in a block, the roads round it, trees along the south road, the parcels' numbers and a sketch.
fn ground() -> Objects {
    let mut o = Objects::new();
    for (x, y) in [(0.0, 0.0), (20.0, 0.0), (0.0, 15.0), (20.0, 15.0)] {
        o.path(
            "parsel",
            &[[x, y], [x + 20.0, y], [x + 20.0, y + 15.0], [x, y + 15.0]],
            true,
        );
    }
    o.line("yol", [-6.0, -5.0], [46.0, -5.0]);
    o.line("yol", [45.0, -6.0], [45.0, 36.0]);
    for x in [4.0, 13.0, 22.0, 31.0, 40.0] {
        o.point("agac", [x, -9.0], None);
    }
    for (i, (x, y)) in [(10.0, 7.5), (30.0, 7.5), (10.0, 22.5), (30.0, 22.5)]
        .into_iter()
        .enumerate()
    {
        o.text("yazi", [x - 2.0, y - 1.0], &format!("10{}", i + 1), 2.5, 0.0);
    }
    o.path("cizim", &[[-4.0, 34.0], [8.0, 38.0], [18.0, 33.0]], false);
    o
}

/// The drawing opened as a CBS project of typed layers, the parcels' layer active, fitted.
fn shown(app: &mut App) {
    open_edited(app, ground(), |d| {
        d["settings"]["workspace"] = json!("gis");
        d["layers"] = json!([
            typed("agac", "Ağaçlar", "#30A46C", 0.25, Some("point")),
            typed("yol", "Yol", "#E5484D", 0.5, Some("line")),
            typed("parsel", "Parsel", "#3E63DD", 0.35, Some("area")),
            typed("yazi", "Yazılar", "fg", 0.18, Some("annotation")),
            typed("cizim", "Çizim", "fg-dim", 0.25, None),
        ]);
        d["activeLayer"] = json!("parsel");
    });
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 10.0,
            min_y: N - 14.0,
            max_x: E + 50.0,
            max_y: N + 40.0,
        },
        24.0,
    );
}

/// The tree's glyphs, a parcel selected, the Harita tab with Otomatik seçim (off) beside Seçimi kaldır.
fn tree(app: &mut App) {
    shown(app);
    click(app, [30.0, 20.0]);
    let _ = app.update(Message::RibbonTab("map"));
}

/// Düzenle with the area layer active: the tools that draw lines and points off; Çizgi started from the command
/// line refused with the reason.
fn off(app: &mut App) {
    shown(app);
    let _ = app.update(Message::RibbonTab("edit"));
    run(app, "tool.line");
    hover(app, [30.0, 20.0]);
}

/// Otomatik seçim off: a click on the road selects nothing and says why.
fn click_other(app: &mut App) {
    shown(app);
    click(app, [30.0, -5.0]);
    hover(app, [30.0, -5.0]);
}

/// Otomatik seçim off: a crossing box over the whole block takes the parcels alone, the rest said.
fn boxed(app: &mut App) {
    shown(app);
    let (from, to) = (at(app, [48.0, 37.0]), at(app, [-8.0, -12.0]));
    let _ = app.update(Message::Viewport(Event::Moved(from)));
    let _ = app.update(Message::Viewport(Event::Pressed(from)));
    let _ = app.update(Message::Viewport(Event::Moved(to)));
    let _ = app.update(Message::Viewport(Event::Released(to)));
}

/// Proje ayarları › Genel: the Seçim group's switch, off in a CBS project.
fn setting(app: &mut App) {
    shown(app);
    run(app, "file.settings");
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("katman-turu", tree),
        ("katman-turu-kapali", off),
        ("oto-secim-tik", click_other),
        ("oto-secim-kutu", boxed),
        ("oto-secim-ayar", setting),
    ]
}

/// Katman türü ▸ on Yol's menu; Yeni katman ▾ in Veri.
pub(crate) fn pointed() -> Vec<Pointed> {
    vec![
        ("katman-turu-menu", shown, |s, app| {
            press_caption(s, app, "Yol", true, false);
            let mut update = |app: &mut App, message: Message| {
                let _ = app.update(message);
            };
            s.settle(app, App::view, &mut update);
            let item = caption_at(s, app, "Katman türü", false);
            s.input(app, App::view, &mut update, Input::Click(item));
            s.settle(app, App::view, &mut update);
            s.input(
                app,
                App::view,
                &mut update,
                Input::Move(Point::new(item.x + 4.0, item.y)),
            );
        }),
        (
            "katman-turu-yeni",
            |app| {
                shown(app);
                let _ = app.update(Message::RibbonTab("data"));
            },
            |s, app| open_ribbon_menu(s, app, "Yeni katman"),
        ),
    ]
}
