//! Pictures of the styled drawing for the owner (docs/adr/0090): the web's
//! demo drawing (the sample project and its showcase of every system symbol,
//! written by `apps/web/scripts/style/demo-drawing.test.ts`) at the views the
//! web's pictures show, dark and light, 1440×900 and 1100×650. Not a test of
//! correctness and not run by default:
//!
//! ```text
//! KENTOS_DEMO_OUT=$PWD/.run/demo/demo.json pnpm -C apps/web exec vitest run scripts/style/demo-drawing.test.ts
//! cargo test -p kentos-desktop style::screens -- --ignored --nocapture
//! ```
//!
//! The pictures go to `.run/shots/stil-*.png`; the build and frame times to
//! standard output.

use std::path::PathBuf;
use std::time::Instant;

use iced::Size;
use kentos_contracts::DocumentSnapshotV1;
use kentos_render_wgpu::Bounds;
use kentos_ui::snapshot::Snapshot;

use crate::app::{App, Message};
use crate::document::Document;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The web's demo drawing, when it has been written.
pub fn demo() -> Option<Document> {
    let path = std::env::var_os("KENTOS_DEMO_OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| root().join(".run/demo/demo.json"));
    let text = std::fs::read_to_string(&path).ok()?;
    let snapshot = DocumentSnapshotV1::from_json(&text).ok()?;
    Document::new(snapshot, None).ok()
}

/// The box of a layer's points (the showcase's areas, lines, points and texts).
fn extent(app: &App, layer: &str) -> Option<Bounds> {
    use kentos_contracts::Entity;
    let doc = app.document.as_ref()?;
    let mut b: Option<Bounds> = None;
    let mut grow = |x: f64, y: f64| {
        let a = b.get_or_insert(Bounds {
            min_x: x,
            min_y: y,
            max_x: x,
            max_y: y,
        });
        a.min_x = a.min_x.min(x);
        a.min_y = a.min_y.min(y);
        a.max_x = a.max_x.max(x);
        a.max_y = a.max_y.max(y);
    };
    for e in doc.model.by_layer(layer) {
        match e {
            Entity::Polygon(p) | Entity::Polyline(p) => p.pts.iter().for_each(|q| grow(q.x, q.y)),
            Entity::Point(p) => grow(p.p.x, p.p.y),
            Entity::Text(t) => grow(t.p.x, t.p.y),
            _ => {}
        }
    }
    b
}

/// Views of the showcase: its section layers, at 1:1000 on screen, their top left in view.
const VIEWS: [(&str, &str); 6] = [
    ("temel", "vitrin.temel-cizgi-tipleri"),
    (
        "uip-sinirlar",
        "vitrin.mpyy-uygulama-imar-plani-sinirlar-planlama-sinirlari",
    ),
    (
        "uip-konut",
        "vitrin.mpyy-uygulama-imar-plani-konut-alanlari",
    ),
    (
        "uip-yapi-duzeni",
        "vitrin.mpyy-uygulama-imar-plani-yapi-duzeni-ve-yogunluklari",
    ),
    (
        "uip-sosyal",
        "vitrin.mpyy-uygulama-imar-plani-sosyal-altyapi-alanlari-egitim-tesisleri-alani",
    ),
    (
        "uip-karayollari",
        "vitrin.mpyy-uygulama-imar-plani-teknik-altyapi-ulasim-karayollari",
    ),
];

/// Uygulama ayarları with its new Semboller ve çizgiler rows, at both sizes and in both themes.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn settings_screens() {
    let out = root().join(".run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            let mut app = crate::files_testing::app_with_drawing();
            let _ = app
                .settings
                .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
            app.apply_settings();
            let _ = app.update(Message::Run("tools.options"));
            let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
            let mut update = |app: &mut App, message| {
                let _ = app.update(message);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            let file = out.join(format!("stil-ayarlar-{width}x{height}{suffix}.png"));
            snapshot
                .render(app.view(), &app.theme())
                .save(&file)
                .expect("writes the picture");
            println!("{}", file.display());
        }
    }
}

/// Katman stili's states on the demo drawing (docs/adr/0091), each applied so
/// the drawing behind shows it: Basit on Ada sınırı, Kategorili on Parsel
/// sınırı by Nitelik, Aralıklı on Yapı by the storeys, Kurallar on Parsel
/// sınırı, Tek sembol on Kot noktaları, and the question on closing with
/// changes not applied; dark and light at both sizes, and light with a larger
/// text at the small one.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn layer_style_screens() {
    use crate::style::layer_style::{Event, Field, Kind, RuleEdit};
    use kentos_native_style::classify::Method;
    let Some(doc) = demo() else {
        eprintln!("the web's demo drawing is not written: see the module's comment");
        return;
    };
    let out = root().join(".run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let states: [(&str, &str, Vec<Event>); 6] = [
        ("basit", "ada", vec![]),
        (
            "kategorili",
            "parsel",
            vec![
                Event::Kind(Kind::Categorized),
                Event::Expr(Field::Categories, "Nitelik".into()),
                Event::Classify,
                Event::Other(true),
                Event::Apply,
            ],
        ),
        (
            "aralikli",
            "yapi",
            vec![
                Event::Kind(Kind::Graduated),
                Event::Expr(Field::Classes, "[Kat adedi]".into()),
                Event::Method(Method::Interval),
                Event::Count("4".into()),
                Event::Ramp("maviler"),
                Event::Graduate,
                Event::Apply,
            ],
        ),
        (
            "kurallar",
            "parsel",
            vec![
                Event::Kind(Kind::Rules),
                Event::Rule(vec![0], RuleEdit::Label("Arsalar".into())),
                Event::Expr(Field::Rule(vec![0]), "Nitelik = 'Arsa'".into()),
                Event::Rule(vec![0], RuleEdit::AddChild),
                Event::Rule(vec![0, 0], RuleEdit::Label("Büyük arsalar".into())),
                Event::Expr(Field::Rule(vec![0, 0]), "$alan > 400".into()),
                Event::Rule(vec![0, 0], RuleEdit::MaxScale("5.000".into())),
                Event::AddElse,
                Event::AddRule,
                Event::Rule(vec![2], RuleEdit::Label("Hatalı koşul".into())),
                Event::Expr(Field::Rule(vec![2]), "$alan >".into()),
                Event::Apply,
            ],
        ),
        ("tek", "kot", vec![Event::Kind(Kind::Single), Event::Apply]),
        // Closing with categories not applied: the window asks first.
        (
            "soru",
            "parsel",
            vec![
                Event::Kind(Kind::Categorized),
                Event::Expr(Field::Categories, "Nitelik".into()),
                Event::Classify,
                Event::Close,
            ],
        ),
    ];
    let sizes: [(f32, f32, &str, &str, i64); 5] = [
        (1440.0, 900.0, "dark", "", 13),
        (1100.0, 650.0, "dark", "", 13),
        (1440.0, 900.0, "light", "-acik", 13),
        (1100.0, 650.0, "light", "-acik", 13),
        (1100.0, 650.0, "light", "-acik-buyuk", 16),
    ];
    for (name, layer, events) in &states {
        for (width, height, mode, suffix, text) in sizes {
            let (mut app, _) = App::boot(None);
            let _ = app.update(Message::Opened(Some(Ok(Box::new(doc.clone())))));
            let _ = app.settings.choose(&[
                ("appearance.theme", serde_json::Value::from(mode)),
                ("appearance.textSize", serde_json::Value::from(text)),
            ]);
            app.apply_settings();
            let _ = app.update(Message::LayerStyle(Event::Open(Some((*layer).to_owned()))));
            for e in events {
                let _ = app.update(Message::LayerStyle(e.clone()));
            }
            let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
            let mut update = |app: &mut App, message| {
                let _ = app.update(message);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            for _ in 0..60 {
                let _ = snapshot.render(app.view(), &app.theme());
                if !app.viewport.status().images_pending {
                    break;
                }
            }
            let file = out.join(format!("lstil-{name}-{width}x{height}{suffix}.png"));
            snapshot
                .render(app.view(), &app.theme())
                .save(&file)
                .expect("writes the picture");
            println!("{}", file.display());
            // The style on the drawing: the window closed, the layer in view.
            if matches!(*name, "basit" | "soru") || width < 1400.0 {
                continue;
            }
            let _ = app.update(Message::LayerStyle(Event::Done));
            let Some(b) = extent(&app, layer) else {
                continue;
            };
            let camera = &mut app.viewport.camera;
            let (bw, bh) = ((b.max_x - b.min_x).max(1.0), (b.max_y - b.min_y).max(1.0));
            camera.scale = (camera.width / (bw * 1.15)).min(camera.height / (bh * 1.15));
            camera.center_on(kentos_render_wgpu::Vec2::new(
                (b.min_x + b.max_x) / 2.0,
                (b.min_y + b.max_y) / 2.0,
            ));
            snapshot.settle(&mut app, App::view, &mut update);
            for _ in 0..60 {
                let _ = snapshot.render(app.view(), &app.theme());
                if !app.viewport.status().images_pending {
                    break;
                }
            }
            let file = out.join(format!("lstil-{name}-cizim-{width}x{height}{suffix}.png"));
            snapshot
                .render(app.view(), &app.theme())
                .save(&file)
                .expect("writes the picture");
            println!("{}", file.display());
        }
    }
}

/// What a picture of Stil yöneticisi shows: its events, then a text to click
/// (a menu's button) or to right-click (a tree node's menu).
struct ManagerState {
    name: &'static str,
    setup: fn(&mut App),
    click: Option<(&'static str, bool, bool)>,
}

/// Stil yöneticisi in the states of the web's pictures
/// (`apps/web/scripts/e2e/shots.mjs stylemanager`): `.run/shots/smgr-*.png`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn manager_screens() {
    use crate::style::manager::{Event, KindFilter, Pick, PickTarget};
    use kentos_native_style::library::Source;
    use kentos_ui::snapshot::Input;
    fn sm(app: &mut App, e: Event) {
        let _ = app.update(Message::StyleManager(Box::new(e)));
    }
    fn open(app: &mut App) {
        let _ = app.update(Message::Run("style.manager"));
    }
    fn with_copy(app: &mut App) {
        open(app);
        sm(app, Event::Copy("temel.cizgi.cift".into(), Source::User));
    }
    let states = [
        ManagerState {
            name: "acik",
            setup: open,
            click: None,
        },
        ManagerState {
            name: "sistem-secili",
            setup: |app| {
                open(app);
                sm(app, Event::Press("temel.cizgi.cift".into()));
            },
            click: None,
        },
        ManagerState {
            name: "kitapligim-secili",
            setup: with_copy,
            click: None,
        },
        ManagerState {
            name: "arama",
            setup: |app| {
                open(app);
                sm(app, Event::Search("konut".into()));
            },
            click: None,
        },
        ManagerState {
            name: "tur-isaret",
            setup: |app| {
                open(app);
                sm(app, Event::Kind(KindFilter::Marker));
            },
            click: None,
        },
        ManagerState {
            name: "tur-cizim",
            setup: |app| {
                open(app);
                sm(app, Event::Kind(KindFilter::Asset));
            },
            click: None,
        },
        ManagerState {
            name: "mpyy",
            setup: |app| {
                open(app);
                sm(app, Event::Go(Source::System, vec!["MPYY".into()], true));
                sm(
                    app,
                    Event::Go(
                        Source::System,
                        vec!["MPYY".into(), "Uygulama imar planı".into()],
                        true,
                    ),
                );
            },
            click: None,
        },
        ManagerState {
            name: "sil-sorusu",
            setup: |app| {
                with_copy(app);
                let id = app
                    .styles
                    .manager
                    .as_ref()
                    .and_then(|m| m.selected.clone())
                    .unwrap_or_default();
                sm(app, Event::Delete(id));
            },
            click: None,
        },
        ManagerState {
            name: "sec-isaret",
            setup: |app| {
                let _ = app.open_style_manager(
                    Some(Pick {
                        kind: Some(KindFilter::Marker),
                        title: "Nokta sembolü seçin".into(),
                        current: None,
                        target: PickTarget::Selection,
                    }),
                    None,
                );
                sm(app, Event::Press("temel.isaret.arti".into()));
            },
            click: None,
        },
        ManagerState {
            name: "ice-aktar",
            setup: |app| {
                with_copy(app);
                let text = kentos_native_style::file::export_styles(
                    &app.styles.library,
                    &["temel.cizgi.cift", "temel.alan.dolu"],
                )
                .to_string();
                app.offer_import("paylasilan.kstil", &text, false);
            },
            click: None,
        },
        ManagerState {
            name: "menu-yeni",
            setup: open,
            click: Some(("Yeni sembol", false, true)),
        },
        ManagerState {
            name: "menu-ice",
            setup: open,
            click: Some(("İçe aktar", false, true)),
        },
        ManagerState {
            name: "menu-disa",
            setup: open,
            click: Some(("Dışa aktar", false, true)),
        },
        // A system symbol's Kopyala, as the web's picture: its details are short, the button in view.
        ManagerState {
            name: "menu-kopyala",
            setup: |app| {
                open(app);
                sm(app, Event::Press("temel.cizgi.cift".into()));
            },
            click: Some(("Kopyala", false, true)),
        },
        ManagerState {
            name: "menu-kategori",
            setup: with_copy,
            click: Some(("Kitaplığım", true, false)),
        },
    ];
    let Some(doc) = demo() else {
        eprintln!("the web's demo drawing is not written: see the module's comment");
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
    for state in &states {
        if only
            .as_deref()
            .is_some_and(|o| !o.split(',').any(|n| n == state.name))
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
            (state.setup)(&mut app);
            let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
            let mut update = |app: &mut App, message| {
                let _ = app.update(message);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            if let Some((label, right, last)) = state.click {
                // The window's text (the last one found, over the ribbon's), or
                // the tree's (the first one, before the details' badge).
                let find = |snapshot: &mut Snapshot, app: &App| {
                    let found: Vec<_> = crate::point_calc::texts(snapshot, app)
                        .into_iter()
                        .filter(|(t, _)| t == label)
                        .map(|(_, at)| at)
                        .collect();
                    let at = if last { found.last() } else { found.first() };
                    at.copied()
                        .unwrap_or_else(|| panic!("“{label}” is not shown"))
                };
                let at = find(&mut snapshot, &app);
                // Below the details' view on a short window: no picture of a closed menu.
                if at.center().y > height - 100.0 {
                    println!(
                        "{}: “{label}” is below the view at {width}x{height}",
                        state.name
                    );
                    continue;
                }
                let input = if right {
                    Input::RightClick(at.center())
                } else {
                    Input::Click(at.center())
                };
                snapshot.input(&mut app, App::view, &mut update, input);
            }
            // The pictures' images reach the atlas over a few frames.
            for _ in 0..8 {
                let _ = snapshot.render(app.view(), &app.theme());
            }
            let file = out.join(format!("smgr-{}-{width}x{height}{suffix}.png", state.name));
            snapshot
                .render(app.view(), &app.theme())
                .save(&file)
                .expect("writes the picture");
            println!("{}", file.display());
        }
    }
}

/// Lejant in the states of the web's pictures (`shots.mjs legend`), and the
/// picture it saves: `.run/shots/lejant-*.png`. The saved picture is drawn
/// by the GPU when `KENTOS_SNAPSHOT_BACKEND=wgpu` names it.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn legend_screens() {
    use crate::style::legend::Event;
    let Some(doc) = demo() else {
        eprintln!("the web's demo drawing is not written: see the module's comment");
        return;
    };
    let out = root().join(".run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    fn first_layer(app: &App) -> String {
        let model = &app.document.as_ref().expect("open").model;
        let window = app.styles.legend.as_ref().expect("open");
        window.groups(model, &app.styles.library)[0]
            .layer_id
            .clone()
    }
    /// A state's events, read from the app once the window is open.
    type Events = fn(&App) -> Vec<Event>;
    let states: [(&str, Events); 4] = [
        ("acik", |_| vec![]),
        ("hepsi", |_| vec![Event::VisibleOnly(false)]),
        ("katman-disarida", |app| {
            vec![Event::Layer(first_layer(app), false)]
        }),
        ("basliksiz", |_| vec![Event::Headings(false)]),
    ];
    let sizes: [(f32, f32, &str, &str, i64); 5] = [
        (1440.0, 900.0, "dark", "", 13),
        (1100.0, 650.0, "dark", "", 13),
        (1440.0, 900.0, "light", "-acik", 13),
        (1100.0, 650.0, "light", "-acik", 13),
        (1100.0, 650.0, "light", "-acik-buyuk", 16),
    ];
    for (name, events) in &states {
        for (width, height, mode, suffix, text) in sizes {
            let (mut app, _) = App::boot(None);
            let _ = app.update(Message::Opened(Some(Ok(Box::new(doc.clone())))));
            let _ = app.settings.choose(&[
                ("appearance.theme", serde_json::Value::from(mode)),
                ("appearance.textSize", serde_json::Value::from(text)),
            ]);
            app.apply_settings();
            let _ = app.update(Message::Run("style.legend"));
            for e in events(&app) {
                let _ = app.update(Message::Legend(e));
            }
            let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
            let mut update = |app: &mut App, message| {
                let _ = app.update(message);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            for _ in 0..8 {
                let _ = snapshot.render(app.view(), &app.theme());
            }
            let file = out.join(format!("lejant-{name}-{width}x{height}{suffix}.png"));
            snapshot
                .render(app.view(), &app.theme())
                .save(&file)
                .expect("writes the picture");
            println!("{}", file.display());
        }
    }
    // The saved picture of the sample project's layers (the showcase's are left
    // out: 800 rows are more than a PNG holds), with and without headings.
    let (mut app, _) = App::boot(None);
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc.clone())))));
    let _ = app.update(Message::Run("style.legend"));
    let showcase: Vec<String> = {
        let model = &app.document.as_ref().expect("open").model;
        let window = app.styles.legend.as_ref().expect("open");
        window
            .groups(model, &app.styles.library)
            .iter()
            .filter(|g| g.layer_id.starts_with("vitrin"))
            .map(|g| g.layer_id.clone())
            .collect()
    };
    for id in showcase {
        let _ = app.update(Message::Legend(Event::Layer(id, false)));
    }
    for (headings, name) in [(true, "resim"), (false, "resim-basliksiz")] {
        let window = app.styles.legend.as_mut().expect("open");
        window.headings = headings;
        let file = out.join(format!("lejant-{name}.png"));
        match crate::style::legend::picture(&app) {
            Some(Ok(bytes)) => {
                std::fs::write(&file, bytes).expect("writes the picture");
                println!("{}", file.display());
            }
            Some(Err(why)) => println!("{name}: {why}"),
            None => println!("{name}: nothing to draw"),
        }
    }
}

#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    let Some(doc) = demo() else {
        eprintln!("the web's demo drawing is not written: see the module's comment");
        return;
    };
    let out = root().join(".run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            let (mut app, _) = App::boot(None);
            let _ = app.update(Message::Opened(Some(Ok(Box::new(doc.clone())))));
            let _ = app
                .settings
                .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
            app.apply_settings();
            let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
            let mut update = |app: &mut App, message| {
                let _ = app.update(message);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            for (name, layer) in VIEWS {
                let Some(b) = extent(&app, layer) else {
                    eprintln!("{layer}: no objects");
                    continue;
                };
                let camera = &mut app.viewport.camera;
                // 1:1000 on a 96 dpi screen, the section's top left a little inside the view.
                camera.scale = 1.0 / (kentos_native_style::program::METRES_PER_PX * 1000.0);
                let (w, h) = (camera.width / camera.scale, camera.height / camera.scale);
                camera.center_on(kentos_render_wgpu::Vec2::new(
                    b.min_x - 4.0 + w / 2.0,
                    b.max_y + 16.0 - h / 2.0,
                ));
                let started = Instant::now();
                snapshot.settle(&mut app, App::view, &mut update);
                // More frames while the atlas leaves images for later (a frame's budget; the app asks
                // for the next frame itself, docs/adr/0090).
                for _ in 0..60 {
                    let _ = snapshot.render(app.view(), &app.theme());
                    if !app.viewport.status().images_pending {
                        break;
                    }
                }
                let file = out.join(format!("stil-{name}-{width}x{height}{suffix}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!(
                    "{} ({:?} with the build {:?})",
                    file.display(),
                    started.elapsed(),
                    app.viewport.styled_build()
                );
            }
        }
    }
}
