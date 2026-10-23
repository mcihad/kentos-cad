//! Pictures of the interface's arrangement for the owner (the ribbon of
//! each project type tab by tab, the selection's tabs, the right-click
//! menus): what the arrangement work on `ux/arayuz-duzeni` starts from and
//! where it gets to. Dark and light, 1440×900 and 1100×650,
//! `.run/shots/duzen-*`. Not a correctness test.
//!
//! ```text
//! KENTOS_SHOTS_ONLY=serit cargo test -p kentos-desktop ux_screens -- --ignored --nocapture
//! ```
//!
//! `KENTOS_SHOTS_ONLY` (comma separated) keeps the pictures whose name has one of the words.

use std::path::{Path, PathBuf};

use iced::{Point, Size};
use kentos_ui::snapshot::{Input, Snapshot};
use serde_json::Value;

use crate::app::{App, Message};
use crate::files_testing::{app_with_drawing, make_cad};

fn update(app: &mut App, message: Message) {
    let _ = app.update(message);
}

fn wanted(name: &str) -> bool {
    match std::env::var("KENTOS_SHOTS_ONLY") {
        Ok(only) if !only.trim().is_empty() => only
            .split(',')
            .map(str::trim)
            .any(|word| !word.is_empty() && name.contains(word)),
        _ => true,
    }
}

fn out() -> PathBuf {
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    out
}

fn save(snapshot: &mut Snapshot, app: &App, out: &Path, name: &str) {
    let file = out.join(format!("{name}.png"));
    snapshot
        .render(app.view(), &app.theme())
        .save(&file)
        .expect("writes the picture");
    println!("{}", file.display());
}

/// The sample drawing in a theme, as a CAD or a CBS project.
fn booted(theme: &str, cad: bool) -> App {
    let mut app = app_with_drawing();
    let _ = app
        .settings
        .choose(&[("appearance.theme", Value::from(theme))]);
    app.apply_settings();
    if cad {
        make_cad(&mut app);
    }
    app
}

/// The window's sizes and themes the pictures are taken in.
fn frames() -> Vec<(Size, &'static str, &'static str)> {
    let mut all = Vec::new();
    for (size, tag) in [
        (Size::new(1440.0, 900.0), ""),
        (Size::new(1100.0, 650.0), "-dar"),
    ] {
        for (theme, suffix) in [("dark", ""), ("light", "-acik")] {
            all.push((size, theme, if tag.is_empty() { suffix } else { leak(format!("{tag}{suffix}")) }));
        }
    }
    all
}

fn leak(text: String) -> &'static str {
    Box::leak(text.into_boxed_str())
}

/// Every tab of each type's ribbon, one picture each: `duzen-serit-<tür>-<sekme>`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn ribbon_tabs() {
    let out = out();
    for (size, theme, suffix) in frames() {
        for (cad, kind) in [(true, "cad"), (false, "cbs")] {
            let probe = booted(theme, cad);
            let tabs: Vec<&'static str> = probe.ribbon_tabs().map(|t| t.id).collect();
            for tab in tabs {
                let name = format!("duzen-serit-{kind}-{tab}{suffix}");
                if !wanted(&name) {
                    continue;
                }
                let mut app = booted(theme, cad);
                app.choose_tab(tab);
                let mut snapshot = Snapshot::new(size).expect("a renderer");
                snapshot.settle(&mut app, App::view, &mut update);
                save(&mut snapshot, &app, &out, &name);
            }
        }
    }
}

/// A selection and its contextual tab, and the drawing area's right-click
/// menu with and without a selection: `duzen-secim-*`, `duzen-sagtik-*`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn selection_and_menus() {
    let out = out();
    for (size, theme, suffix) in frames() {
        for (cad, kind) in [(true, "cad"), (false, "cbs")] {
            let name = format!("duzen-secim-{kind}{suffix}");
            if wanted(&name) {
                let mut app = booted(theme, cad);
                select_first_area(&mut app);
                let tab = app.contextual_tabs().next().map(|t| t.id);
                if let Some(tab) = tab {
                    app.choose_tab(tab);
                }
                let mut snapshot = Snapshot::new(size).expect("a renderer");
                snapshot.settle(&mut app, App::view, &mut update);
                save(&mut snapshot, &app, &out, &name);
            }
            for (selected, word) in [(false, "bos"), (true, "secili")] {
                let name = format!("duzen-sagtik-{kind}-{word}{suffix}");
                if !wanted(&name) {
                    continue;
                }
                let mut app = booted(theme, cad);
                if selected {
                    select_first_area(&mut app);
                }
                let mut snapshot = Snapshot::new(size).expect("a renderer");
                snapshot.settle(&mut app, App::view, &mut update);
                let area = app.viewport.bounds;
                let at = Point::new(area.x + area.width * 0.45, area.y + area.height * 0.35);
                snapshot.input(&mut app, App::view, &mut update, Input::RightClick(at));
                save(&mut snapshot, &app, &out, &name);
            }
        }
    }
}

/// The sample drawing's first closed area selected, as a click selects it.
fn select_first_area(app: &mut App) {
    let Some(doc) = app.document.as_ref() else {
        return;
    };
    let slot = doc
        .model
        .entities()
        .find(|e| matches!(e, kentos_commands::Entity::Polygon(_)))
        .map(|e| kentos_domain::Slot(e.base().id));
    if let Some(slot) = slot {
        app.selection.set(vec![slot]);
    }
}

/// What a new 1:1000 project in metres gives its dimensions and texts.
#[test]
#[ignore = "a probe for the owner's scale report, run by hand"]
fn scale_probe() {
    use kentos_commands::{DrawingFont, Workspace};
    use kentos_project::new_project::NewProject;
    for (cad, srid) in [(true, 0u32), (true, 5256), (false, 5256)] {
        let doc = crate::project::wizard::new_document(&NewProject {
            name: "Ölçek".to_owned(),
            srid,
            plot_scale: 1000.0,
            workspace: if cad { Workspace::Cad } else { Workspace::Gis },
            drawing_font: DrawingFont::Barlow,
            province: None,
            drawing_unit: Some(kentos_commands::DrawingUnit::M),
            annotation: None,
        })
        .expect("a new project");
        let (mut app, _) = App::boot(None);
        let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
        let s = app.document.as_ref().expect("a drawing").settings().clone();
        println!(
            "cad={cad} srid={srid} plot_scale={} unit={:?} dim_height={} text_height={}",
            s.plot_scale,
            s.unit(),
            s.annotation_height(kentos_commands::AnnotationKind::Dimension),
            s.annotation_height(kentos_commands::AnnotationKind::Text)
        );
        let origin = app.document.as_ref().map(|d| d.model.origin()).expect("origin");
        let at = |app: &App, x: f64, y: f64| {
            let [sx, sy] = app
                .viewport
                .camera
                .world_to_screen(kentos_render_wgpu::Vec2::new(origin.x + x, origin.y + y));
            Point::new(sx as f32, sy as f32)
        };
        let _ = app.update(Message::RunMethod {
            id: "tool.dimension",
            option: "H",
            label: "",
        });
        for (x, y) in [(0.0, 0.0), (10.0, 0.0), (5.0, 3.0)] {
            let p = at(&app, x, y);
            let _ = app.update(Message::Viewport(crate::viewport::Event::Moved(p)));
            let _ = app.update(Message::Viewport(crate::viewport::Event::Pressed(p)));
            let _ = app.update(Message::Viewport(crate::viewport::Event::Released(p)));
        }
        let doc = app.document.as_ref().expect("a drawing");
        for e in doc.model.entities() {
            if let kentos_commands::Entity::Dimension(d) = e {
                println!("  dimension height={} style={:?}", d.height, d.style);
            }
        }
        println!("  said: {}", crate::files_testing::last_said(&app));
    }
}

/// The new project's scale and heights (docs/adr/0622 §2): a local CAD plan
/// in metres starts at 1:1000 and says what its texts are in the drawing;
/// the opened project's status bar with its drawing scale and its type,
/// which says it and offers no switch: `duzen-olcek-*`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn scale_and_type() {
    use kentos_commands::{DrawingUnit, Workspace};
    use kentos_project::wizard::Coords;

    use crate::project::Event as ProjectEvent;
    use crate::project::wizard::Event as W;

    fn send(app: &mut App, e: W) {
        let _ = app.update(Message::Project(Box::new(ProjectEvent::New(e))));
    }
    let out = out();
    for (size, theme, suffix) in frames() {
        let name = format!("duzen-olcek-sihirbaz{suffix}");
        if wanted(&name) {
            let mut app = booted(theme, false);
            let _ = app.update(Message::Run("file.new"));
            send(&mut app, W::Type(Workspace::Cad));
            send(&mut app, W::Next);
            send(&mut app, W::Coords(Coords::Local));
            send(&mut app, W::Unit(DrawingUnit::M));
            send(&mut app, W::Next);
            send(&mut app, W::Name("İmar planı".into()));
            let mut snapshot = Snapshot::new(size).expect("a renderer");
            snapshot.settle(&mut app, App::view, &mut update);
            save(&mut snapshot, &app, &out, &name);
            // Oluştur: the new project, its status bar.
            let name = format!("duzen-olcek-proje{suffix}");
            send(&mut app, W::Next);
            let mut snapshot = Snapshot::new(size).expect("a renderer");
            snapshot.settle(&mut app, App::view, &mut update);
            save(&mut snapshot, &app, &out, &name);
            // The status bar's drawing scale, open.
            let name = format!("duzen-olcek-menu{suffix}");
            if let Some(at) = crate::files_testing::find_text(&mut snapshot, &app, "1:1.000") {
                snapshot.input(&mut app, App::view, &mut update, Input::Click(at.center()));
            }
            save(&mut snapshot, &app, &out, &name);
        }
        let name = format!("duzen-tur-ayarlar{suffix}");
        if wanted(&name) {
            let mut app = booted(theme, true);
            let _ = app.update(Message::Run("file.settings"));
            let mut snapshot = Snapshot::new(size).expect("a renderer");
            snapshot.settle(&mut app, App::view, &mut update);
            save(&mut snapshot, &app, &out, &name);
        }
    }
}
