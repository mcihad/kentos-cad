//! Pictures of Sembol tasarımcısı for the owner (docs/adr/0094), in the
//! states of the web's pictures (`apps/web/scripts/e2e/out/shots/symboldesigner`):
//! every layer type's form, Katman ekle's menu, a marker's layer, an
//! expression, a symbol of the library, a layer style's symbol, the sample
//! with a hole and the question on closing; dark and light at 1440×900 and
//! 1100×650, and light with a larger text at the small size. Not a test of
//! correctness and not run by default:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=tarama,soru cargo test -p kentos-desktop style::designer::screens -- --ignored --nocapture
//! ```
//!
//! The pictures go to `.run/shots/sdes-*.png`.

use std::path::PathBuf;

use iced::Size;
use kentos_native_style::designer::{LayerPath, set};
use kentos_native_style::preview::Geometry;
use kentos_native_style::renderer::GeometryClass;
use kentos_ui::snapshot::{Input, Snapshot};
use serde_json::json;

use super::{Edit, Event};
use crate::app::{App, Message};
use crate::style::layer_style::{Event as LayerEvent, Kind, SetAt};
use crate::style::manager::Event as ManagerEvent;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn de(app: &mut App, e: Event) {
    let _ = app.update(Message::Designer(Box::new(e)));
}

/// A new symbol of a kind from Stil yöneticisi, with layers added.
fn new(app: &mut App, kind: &'static str, add: &[&'static str]) {
    let _ = app.update(Message::Run("style.manager"));
    let _ = app.update(Message::StyleManager(Box::new(ManagerEvent::NewSymbol(kind))));
    for t in add {
        de(app, Event::Add(t, None));
    }
}

/// A picture's state: what is done, then a text to click (a menu's button).
struct Scene {
    name: &'static str,
    setup: fn(&mut App),
    click: Option<&'static str>,
}

#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    let scenes = [
        Scene {
            name: "dolgu",
            setup: |app| new(app, "fill", &[]),
            click: None,
        },
        Scene {
            name: "tarama",
            setup: |app| new(app, "fill", &["hatchFill"]),
            click: None,
        },
        Scene {
            name: "desen",
            setup: |app| new(app, "fill", &["patternFill"]),
            click: None,
        },
        Scene {
            name: "goruntu-dolgusu",
            setup: |app| new(app, "fill", &["imageFill"]),
            click: None,
        },
        Scene {
            name: "ic-noktada-isaret",
            setup: |app| new(app, "fill", &["centroidMarker"]),
            click: None,
        },
        Scene {
            name: "cizgi",
            setup: |app| new(app, "line", &[]),
            click: None,
        },
        Scene {
            name: "dalga",
            setup: |app| {
                new(app, "line", &[]);
                let at = app.styles.designer.as_ref().map_or(LayerPath::Top(0), |d| d.selected);
                de(
                    app,
                    Event::Edit(Edit {
                        at,
                        key: "wave".into(),
                        text: None,
                        patch: Some(set(
                            "wave",
                            json!({ "shape": "sine", "length": 5, "amplitude": 0.8, "connect": true, "offsetAlong": 0 }),
                        )),
                        focus: None,
                    }),
                );
            },
            click: None,
        },
        Scene {
            name: "cizgi-boyunca-isaret",
            setup: |app| new(app, "line", &["markerLine"]),
            click: None,
        },
        Scene {
            name: "isaretin-katmani",
            setup: |app| {
                new(app, "line", &["markerLine"]);
                de(app, Event::Select(LayerPath::Child(1, 0)));
            },
            click: None,
        },
        Scene {
            name: "sekil",
            setup: |app| new(app, "marker", &[]),
            click: None,
        },
        Scene {
            name: "yazi",
            setup: |app| new(app, "marker", &["text"]),
            click: None,
        },
        Scene {
            name: "svg",
            setup: |app| new(app, "marker", &["svg"]),
            click: None,
        },
        Scene {
            name: "goruntu",
            setup: |app| new(app, "marker", &["raster"]),
            click: None,
        },
        Scene {
            name: "katman-ekle",
            setup: |app| new(app, "line", &[]),
            click: Some("Katman ekle"),
        },
        Scene {
            name: "katman-ekle-isarete",
            setup: |app| new(app, "line", &["markerLine"]),
            click: Some("Katman ekle"),
        },
        Scene {
            name: "ifade",
            setup: |app| {
                new(app, "fill", &[]);
                let at = app.styles.designer.as_ref().map_or(LayerPath::Top(0), |d| d.selected);
                de(
                    app,
                    Event::Edit(Edit {
                        at,
                        key: "color".into(),
                        text: None,
                        patch: Some(set(
                            "color",
                            json!({ "expr": "", "fallback": "#C9D6E3" }),
                        )),
                        focus: Some("color:expr".into()),
                    }),
                );
            },
            click: None,
        },
        Scene {
            name: "adali-alan",
            setup: |app| {
                new(app, "fill", &["hatchFill"]);
                de(app, Event::Sample(Geometry::Hole));
            },
            click: None,
        },
        Scene {
            name: "kitaplik",
            setup: |app| {
                let _ = app.update(Message::Run("style.manager"));
                let _ = app.update(Message::StyleManager(Box::new(ManagerEvent::Edit(
                    "temel.alan.capraz".into(),
                ))));
            },
            click: None,
        },
        Scene {
            name: "katman-stilinden",
            setup: |app| {
                let _ = app.update(Message::LayerStyle(LayerEvent::Open(Some("parsel".into()))));
                let _ = app.update(Message::LayerStyle(LayerEvent::Kind(Kind::Single)));
                let symbol = json!({ "type": "fill", "layers": [
                    { "id": "0", "type": "hatchFill", "angle": 45, "spacing": 2, "width": 0.2, "color": "#4E79A7" },
                    { "id": "1", "type": "simpleLine", "color": "ink", "width": 0.35 },
                ] });
                let _ = app.update(Message::LayerStyle(LayerEvent::Design(
                    SetAt::Single,
                    GeometryClass::Fill,
                    "Tek sembol (alan)".into(),
                    symbol,
                )));
            },
            click: None,
        },
        Scene {
            name: "soru",
            setup: |app| {
                new(app, "fill", &[]);
                de(app, Event::Name("Bahçe alanı".into()));
                de(app, Event::Close);
            },
            click: None,
        },
    ];
    let Some(doc) = crate::style::screens::demo() else {
        eprintln!("the web's demo drawing is not written: see style/screens.rs");
        return;
    };
    let out = root().join(".run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let only = std::env::var("KENTOS_SHOTS_ONLY").ok();
    let sizes: [(f32, f32, &str, &str, i64); 5] = [
        (1440.0, 900.0, "dark", "", 13),
        (1100.0, 650.0, "dark", "", 13),
        (1440.0, 900.0, "light", "-acik", 13),
        (1100.0, 650.0, "light", "-acik", 13),
        (1100.0, 650.0, "light", "-acik-buyuk", 16),
    ];
    for scene in &scenes {
        if only
            .as_deref()
            .is_some_and(|o| !o.split(',').any(|n| n == scene.name))
        {
            continue;
        }
        for (width, height, mode, suffix, text) in sizes {
            let (mut app, _) = App::boot(None);
            let _ = app.update(Message::Opened(Some(Ok(Box::new(doc.clone())))));
            let _ = app.settings.choose(&[
                ("appearance.theme", serde_json::Value::from(mode)),
                ("appearance.textSize", serde_json::Value::from(text)),
            ]);
            app.apply_settings();
            (scene.setup)(&mut app);
            let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
            let mut update = |app: &mut App, message| {
                let _ = app.update(message);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            if let Some(label) = scene.click {
                // The window's own text: the last one found, over the manager's.
                let at = crate::point_calc::texts(&mut snapshot, &app)
                    .into_iter()
                    .filter(|(t, _)| t == label)
                    .map(|(_, at)| at)
                    .last()
                    .unwrap_or_else(|| panic!("“{label}” is not shown"));
                snapshot.input(&mut app, App::view, &mut update, Input::Click(at.center()));
            }
            // The pictures' images reach the atlas over a few frames.
            for _ in 0..8 {
                let _ = snapshot.render(app.view(), &app.theme());
            }
            let file = out.join(format!("sdes-{}-{width}x{height}{suffix}.png", scene.name));
            snapshot
                .render(app.view(), &app.theme())
                .save(&file)
                .expect("writes the picture");
            println!("{}", file.display());
        }
    }
}
