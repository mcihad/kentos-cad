//! Pictures of the SVG editor for the owner (docs/adr/0095), in the states
//! of the web's pictures (`apps/web/scripts/e2e/out/shots/svgedit`): a new
//! drawing, shapes drawn and chosen, node editing, a polyline being drawn,
//! a measure, a text, the Hizala, Dönüştür and Dizi tabs, the menus, the XML
//! source, document properties, export, import, tracing, the tracing
//! reference, a library drawing, the library picker and the unsaved
//! question; dark and light at 1440×900 and 1100×650, and light with a
//! larger text at the small size. Not a test of correctness, not run by default:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=yeni,menu-yol cargo test -p kentos-desktop style::svgedit::screens -- --ignored --nocapture
//! ```
//!
//! The pictures go to `.run/shots/svge-*.png` (the reference and tracing
//! scenes need the GPU renderer: `KENTOS_SNAPSHOT_BACKEND=wgpu`).

use std::path::PathBuf;

use iced::Size;
use kentos_ui::snapshot::{Input, Snapshot};

use super::actions::Action;
use super::files::{self, FileCmd, FileDialog};
use super::state::{After, Opening, Tab};
use super::{Event, ToolId};
use crate::app::{App, Message};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn open(app: &mut App) {
    app.open_svg_editor(Opening {
        id: None,
        path: None,
        after: After::Nothing,
    });
}

fn ed(app: &mut App) -> &mut super::SvgEditor {
    app.styles.svg_editor.as_mut().expect("the editor is open")
}

fn send(app: &mut App, e: Event) {
    let _ = app.update(Message::SvgEdit(Box::new(e)));
}

/// Three shapes as the web's pictures have them: a rectangle, an ellipse and a hexagon.
fn three(app: &mut App) {
    let e = ed(app);
    e.set_tool(ToolId::Rect);
    e.place([10.0, 10.0], [45.0, 40.0], false);
    e.set_tool(ToolId::Ellipse);
    e.place([55.0, 10.0], [90.0, 40.0], false);
    e.set_tool(ToolId::Polygon);
    e.place([50.0, 72.0], [50.0, 50.0], false);
    e.set_tool(ToolId::Select);
    e.select_all();
}

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
            name: "yeni",
            setup: open,
            click: None,
        },
        Scene {
            name: "dikdortgen",
            setup: |app| {
                open(app);
                let e = ed(app);
                e.set_tool(ToolId::Rect);
                e.place([15.0, 15.0], [50.0, 40.0], false);
            },
            click: None,
        },
        Scene {
            name: "hepsi-secili",
            setup: |app| {
                open(app);
                three(app);
            },
            click: None,
        },
        Scene {
            name: "dugum",
            setup: |app| {
                open(app);
                let e = ed(app);
                e.set_tool(ToolId::Ellipse);
                e.place([25.0, 25.0], [75.0, 70.0], false);
                e.path_op(super::actions::PathOp::ToPath);
                let id = e.selection[0].clone();
                e.edit_nodes(Some(id));
                e.set_node_selected(vec![(0, 0)]);
            },
            click: None,
        },
        Scene {
            name: "kirik-cizgi",
            setup: |app| {
                open(app);
                let e = ed(app);
                e.set_tool(ToolId::Line);
                for p in [[15.0, 80.0], [35.0, 30.0], [60.0, 60.0]] {
                    e.draft_click(p);
                }
                e.draw.cursor = Some([85.0, 25.0]);
            },
            click: None,
        },
        Scene {
            name: "olc",
            setup: |app| {
                open(app);
                three(app);
                let e = ed(app);
                e.select(Vec::new());
                e.set_tool(ToolId::Measure);
                e.measure_down([10.0, 40.0]);
                e.measure_move([90.0, 10.0]);
                e.measure_up();
            },
            click: None,
        },
        Scene {
            name: "yazi",
            setup: |app| {
                open(app);
                let e = ed(app);
                e.set_tool(ToolId::Text);
                e.place([15.0, 55.0], [15.0, 55.0], false);
            },
            click: None,
        },
        Scene {
            name: "hizala",
            setup: |app| {
                open(app);
                three(app);
                ed(app).ui.tab = Tab::Align;
            },
            click: None,
        },
        Scene {
            name: "donustur",
            setup: |app| {
                open(app);
                three(app);
                ed(app).ui.tab = Tab::Transform;
            },
            click: None,
        },
        Scene {
            name: "dizi",
            setup: |app| {
                open(app);
                three(app);
                ed(app).ui.tab = Tab::Array;
            },
            click: None,
        },
        Scene {
            name: "menu-dosya",
            setup: |app| {
                open(app);
                three(app);
            },
            click: Some("Dosya"),
        },
        Scene {
            name: "menu-yol",
            setup: |app| {
                open(app);
                three(app);
            },
            click: Some("Yol"),
        },
        Scene {
            name: "menu-nesne",
            setup: |app| {
                open(app);
                three(app);
            },
            click: Some("Nesne"),
        },
        Scene {
            name: "menu-sec",
            setup: |app| {
                open(app);
                three(app);
            },
            click: Some("Seç"),
        },
        Scene {
            name: "kaynak",
            setup: |app| {
                open(app);
                three(app);
                let e = ed(app);
                let last = e.selection.last().cloned().expect("a shape");
                e.select(vec![last]);
                send(app, Event::File(files::Event::Cmd(FileCmd::ToggleSource)));
            },
            click: None,
        },
        Scene {
            name: "belge",
            setup: |app| {
                open(app);
                send(app, Event::File(files::Event::Cmd(FileCmd::DocProps)));
            },
            click: None,
        },
        Scene {
            name: "disa-aktar",
            setup: |app| {
                open(app);
                three(app);
                send(app, Event::File(files::Event::Cmd(FileCmd::Export)));
            },
            click: None,
        },
        Scene {
            name: "ice-al",
            setup: |app| {
                open(app);
                let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64"><circle cx="32" cy="32" r="28" fill="#1d1d1b"/><path d="M20 34l8 8 16-18" fill="none" stroke="#e30613" stroke-width="6"/><linearGradient id="g"/><rect x="4" y="4" width="10" height="10" fill="url(#g)"/></svg>"##;
                files::import::open(ed(app), svg, "onay");
            },
            click: None,
        },
        Scene {
            name: "altlik",
            setup: |app| {
                open(app);
                let images = app.styles.images.clone();
                files::reference::load(ed(app), "ornek.png", &sample_png(), &images);
                let e = ed(app);
                e.set_tool(ToolId::Pen);
            },
            click: None,
        },
        Scene {
            name: "izle",
            setup: |app| {
                open(app);
                let images = app.styles.images.clone();
                files::reference::load(ed(app), "ornek.png", &sample_png(), &images);
                send(app, Event::File(files::Event::Cmd(FileCmd::Trace)));
                files::trace::run_now(ed(app));
            },
            click: None,
        },
        Scene {
            name: "kitaplik",
            setup: |app| {
                let id = app
                    .styles
                    .library
                    .items(None)
                    .into_iter()
                    .find(|(i, _)| i.format() == Some("svg"))
                    .map(|(i, _)| i.id().to_owned())
                    .expect("a system drawing");
                app.open_svg_editor(Opening {
                    id: Some(id),
                    path: None,
                    after: After::Nothing,
                });
            },
            click: None,
        },
        Scene {
            name: "kitaplik-sec",
            setup: |app| {
                open(app);
                ed(app).files.dialog = Some(FileDialog::Library(files::library::Picker::default()));
            },
            click: None,
        },
        Scene {
            name: "soru",
            setup: |app| {
                open(app);
                three(app);
                ed(app).action(Action::Duplicate);
                send(app, Event::Close);
            },
            click: None,
        },
    ];
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
            // The canvas told its size: the drawing is fitted to it.
            if let Some(e) = app.styles.svg_editor.as_mut() {
                e.camera.fit(&e.doc, &e.options);
                e.touch();
            }
            snapshot.settle(&mut app, App::view, &mut update);
            if let Some(label) = scene.click {
                // The editor's bar, from its Dosya on (the ribbon behind and the tool list have a
                // Seç of their own): the first match, row by row.
                let texts = crate::point_calc::texts(&mut snapshot, &app);
                let bar = texts
                    .iter()
                    .filter(|(t, at)| t == "Dosya" && at.y > 50.0)
                    .map(|(_, at)| *at)
                    .min_by(|a, b| a.y.total_cmp(&b.y))
                    .expect("the editor's bar");
                let at = texts
                    .into_iter()
                    .filter(|(t, at)| t == label && at.y > bar.y - 4.0 && at.x > bar.x - 4.0)
                    .map(|(_, at)| at)
                    .min_by(|a, b| a.y.total_cmp(&b.y).then(a.x.total_cmp(&b.x)))
                    .unwrap_or_else(|| panic!("“{label}” is not shown"));
                snapshot.input(&mut app, App::view, &mut update, Input::Click(at.center()));
            }
            for _ in 0..4 {
                let _ = snapshot.render(app.view(), &app.theme());
            }
            let file = out.join(format!("svge-{}-{width}x{height}{suffix}.png", scene.name));
            snapshot
                .render(app.view(), &app.theme())
                .save(&file)
                .expect("writes the picture");
            println!("{}", file.display());
        }
    }
}

/// A small picture to trace: a dark disc with a light hole and a bar, as a PNG.
pub fn sample_png() -> Vec<u8> {
    let (w, h) = (120u32, 90u32);
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let (dx, dy) = (x as f64 - 45.0, y as f64 - 45.0);
            let d = dx.hypot(dy);
            let ink = (d < 36.0 && d > 14.0) || (x > 90 && x < 110 && y > 15 && y < 75);
            let v = if ink { 30 } else { 245 };
            rgba.extend_from_slice(&[v, v, v, 255]);
        }
    }
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, w, h);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().expect("a header");
        writer.write_image_data(&rgba).expect("the pixels");
    }
    bytes
}
