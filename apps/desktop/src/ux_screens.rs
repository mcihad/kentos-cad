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

#[test]
#[ignore = "CAD source and property-input GPU acceptance; run explicitly"]
fn cad_open_and_property_input() {
    use iced::keyboard::key::Named;
    for (theme, suffix, size) in [("dark", "koyu", 13), ("light", "acik", 13), ("dark", "buyuk", 18)] {
        let mut app = booted(theme, true);
        let _ = app.settings.choose(&[("appearance.textSize", Value::from(size))]);
        app.apply_settings();
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/formats/v1/dxf-units/mm.dxf");
        app.document = None;
        let task = app.start_opening(path, crate::opening::Purpose::File);
        crate::files_testing::drive(&mut app, task);
        let mut snapshot = Snapshot::new(Size::new(1440.0, 900.0)).expect("renderer");
        snapshot.settle(&mut app, App::view, &mut update);
        save(&mut snapshot, &app, &out(), &format!("cad-dogrudan-ac-{suffix}"));
        if size > 13 { continue; }

        // The actual properties cell: draft typing changes no geometry; Enter commits once, Escape cancels.
        app = booted(theme, true);
        app.selection.set([kentos_domain::Slot(11)]);
        update(&mut app, Message::Properties(crate::properties::Event::Toggle("general")));
        snapshot = Snapshot::new(Size::new(1440.0, 900.0)).expect("renderer");
        snapshot.settle(&mut app, App::view, &mut update);
        let point = Point::new(1440.0-app.docks.size(kentos_ui::widget::docking::Side::Right)+250.0, 738.0);
        let contents = |app: &App| match app.document.as_ref().expect("open").model.get(kentos_domain::Slot(11)) {
            Some(kentos_commands::Entity::Text(t)) => t.text.clone(), _ => panic!("text"),
        };
        let before = contents(&app);
        let revision = app.document.as_ref().expect("open").model.revision();
        snapshot.input(&mut app, App::view, &mut update, Input::Click(point));
        let started = std::time::Instant::now();
        snapshot.input(&mut app, App::view, &mut update, Input::Type(" 12345,6789".into()));
        println!("Öznitelik hücresinde 11 karakter: {:?}", started.elapsed());
        assert_eq!(contents(&app), before, "typing is only a draft");
        save(&mut snapshot, &app, &out(), &format!("cad-oznitelik-yaziliyor-{suffix}"));
        snapshot.input(&mut app, App::view, &mut update, Input::Key(Named::Enter));
        assert!(contents(&app).ends_with(" 12345,6789"), "the real text cell must receive every character");
        assert!(app.document.as_ref().expect("open").model.revision() > revision);
        let committed = contents(&app);
        snapshot.input(&mut app, App::view, &mut update, Input::Click(point));
        snapshot.input(&mut app, App::view, &mut update, Input::Type(" CANCEL".into()));
        snapshot.input(&mut app, App::view, &mut update, Input::Key(Named::Escape));
        assert_eq!(contents(&app), committed);
        save(&mut snapshot, &app, &out(), &format!("cad-oznitelik-onaylandi-{suffix}"));
    }
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
            cad_work: None,
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
    use kentos_commands::Workspace;

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
            // Harita / imar planı (docs/adr/0626 §1): metres, 1:1000, on the province's system.
            send(&mut app, W::Work(1));
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
        // A CBS project's one scale, its view's (docs/adr/0626 §3).
        let name = format!("duzen-olcek-cbs{suffix}");
        if wanted(&name) {
            let mut app = booted(theme, false);
            let mut snapshot = Snapshot::new(size).expect("a renderer");
            snapshot.settle(&mut app, App::view, &mut update);
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

/// Each tab's panels' widths at their four levels (labels … icons), for
/// fitting the ribbon to 1440 and 1100 pixels (docs/adr/0622 §3).
#[test]
#[ignore = "a measure for the arrangement, run by hand"]
fn ribbon_widths() {
    let _typography = crate::appearance::tests::TYPOGRAPHY.lock();
    kentos_ui::theme::typography::set(kentos_ui::theme::typography::Typography::DEFAULT);
    kentos_ui::theme::typography::load();
    for (cad, kind) in [(true, "CAD"), (false, "CBS")] {
        let app = booted("dark", cad);
        let tabs: Vec<&'static crate::catalog::Tab> = app
            .ribbon_tabs()
            .chain(app.contextual_tabs())
            .collect();
        for tab in tabs {
            let groups: Vec<_> = tab
                .panels
                .iter()
                .filter_map(|p| app.ribbon_group(tab.id, p).map(|g| (p.label, g)))
                .collect();
            let widths: Vec<[f32; 4]> = groups.iter().map(|(_, g)| g.widths()).collect();
            let total = |level: usize| widths.iter().map(|w| w[level]).sum::<f32>();
            let keep: Vec<bool> = groups.iter().map(|(_, g)| g.keeps()).collect();
            let (l1440, _) = kentos_ui::widget::ribbon::fit(&widths, &keep, 1440.0);
            let (l1100, over) = kentos_ui::widget::ribbon::fit(&widths, &keep, 1100.0);
            println!(
                "{kind} {:<14} L0 {:>5.0} L1 {:>5.0} L2 {:>5.0} L3 {:>5.0} | 1440 {:?} | 1100 {:?}{}",
                tab.id,
                total(0),
                total(1),
                total(2),
                total(3),
                l1440,
                l1100,
                if over { " TAŞAR" } else { "" }
            );
            for ((label, _), w) in groups.iter().zip(&widths) {
                println!("      {label:<22} {:>5.0} {:>5.0} {:>5.0} {:>5.0}", w[0], w[1], w[2], w[3]);
            }
        }
    }
}

/// The tab a selection opens, by its objects' kind (docs/adr/0622 §4): a
/// line, an area, a point, a text, a mixed selection, a raster and a point
/// cloud selected, the tab in front: `duzen-baglam-<tür>-*`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn contextual_tabs() {
    use crate::tools_scenes::{Objects, open};

    let out = out();
    for (size, theme, suffix) in frames() {
        for (cad, kind) in [(true, "cad"), (false, "cbs")] {
            let scene = |app: &mut App| {
                let mut o = Objects::new();
                o.path(
                    "parsel",
                    &[[0.0, 0.0], [40.0, 0.0], [44.0, 32.0], [2.0, 30.0]],
                    true,
                );
                o.path("yol", &[[-20.0, -10.0], [30.0, -12.0], [70.0, 5.0]], false);
                o.point("cizim", [50.0, 20.0], Some(101.5));
                o.text("cizim", [8.0, 12.0], "101/7", 2.5, 0.0);
                open(app, o);
                if cad {
                    make_cad(app);
                }
            };
            for (word, picked) in [
                ("alan", vec![1u32]),
                ("cizgi", vec![2]),
                ("nokta", vec![3]),
                ("yazi", vec![4]),
                ("karisik", vec![1, 2, 3]),
            ] {
                let name = format!("duzen-baglam-{kind}-{word}{suffix}");
                if !wanted(&name) {
                    continue;
                }
                let mut app = booted(theme, cad);
                scene(&mut app);
                select_ids(&mut app, &picked);
                let mut snapshot = Snapshot::new(size).expect("a renderer");
                snapshot.settle(&mut app, App::view, &mut update);
                save(&mut snapshot, &app, &out, &name);
            }
            let name = format!("duzen-baglam-{kind}-raster{suffix}");
            if wanted(&name) {
                let mut app = booted(theme, cad);
                open_rasters(&mut app);
                if cad {
                    make_cad(&mut app);
                }
                select_kind(&mut app, "raster");
                let mut snapshot = Snapshot::new(size).expect("a renderer");
                snapshot.settle(&mut app, App::view, &mut update);
                save(&mut snapshot, &app, &out, &name);
            }
            let name = format!("duzen-baglam-{kind}-bulut{suffix}");
            if wanted(&name) {
                let mut app = booted(theme, cad);
                crate::cloud_scenes::opened(&mut app, [-40.0, -40.0, 260.0, 220.0]);
                if cad {
                    make_cad(&mut app);
                }
                select_kind(&mut app, "pointcloud");
                let mut snapshot = Snapshot::new(size).expect("a renderer");
                snapshot.settle(&mut app, App::view, &mut update);
                save(&mut snapshot, &app, &out, &name);
            }
        }
    }
}

/// Objects selected by their ids in the drawing (a scene's own numbers), as clicks select them; the tab follows.
fn select_ids(app: &mut App, ids: &[u32]) {
    app.selection
        .set(ids.iter().map(|id| kentos_domain::Slot(*id)));
    app.follow_selection_tab();
}

/// The drawing's first object of a kind selected; the tab follows.
fn select_kind(app: &mut App, kind: &str) {
    let slot = app.document.as_ref().and_then(|doc| {
        doc.model
            .entities()
            .find(|e| e.kind() == kind)
            .map(|e| kentos_domain::Slot(e.base().id))
    });
    if let Some(slot) = slot {
        app.selection.set(vec![slot]);
    }
    app.follow_selection_tab();
}

/// The rasters' shared drawing (fixtures/interaction/v1/rasters.kcad) beside its files, fitted.
fn open_rasters(app: &mut App) {
    const DRAWING: &str = include_str!("../../../fixtures/interaction/v1/rasters.kcad");
    let snapshot =
        kentos_commands::DocumentSnapshotV1::from_json(DRAWING).expect("the drawing reads");
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/interaction/v1/rasters.kcad");
    let doc = crate::document::Document::new(snapshot, Some(path)).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    let (x0, y0) = (487_200.0, 4_420_600.0);
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: x0 - 360.0,
            min_y: y0 - 540.0,
            max_x: x0 + 800.0,
            max_y: y0 + 20.0,
        },
        24.0,
    );
}

/// The right button as AutoCAD's (docs/adr/0622 §6): the menu with nothing
/// selected, with a selection, while a line is drawn and while a grip is
/// moved (never dropped where the pointer is): `duzen-sagtik2-*`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn right_click_menus() {
    use kentos_domain::Slot;
    use kentos_interaction::grip_menu::GripAction;

    let out = out();
    for (size, theme, suffix) in frames() {
        for (word, setup) in [
            ("bos", 0u8),
            ("secili", 1),
            ("komut", 2),
            ("tutamac", 3),
        ] {
            let name = format!("duzen-sagtik2-{word}{suffix}");
            if !wanted(&name) {
                continue;
            }
            let mut app = booted(theme, true);
            let mut snapshot = Snapshot::new(size).expect("a renderer");
            snapshot.settle(&mut app, App::view, &mut update);
            let area = app.viewport.bounds;
            let mut at = Point::new(area.x + area.width * 0.45, area.y + area.height * 0.35);
            match setup {
                1 => {
                    app.selection.set([Slot(4)]);
                    app.follow_selection_tab();
                }
                2 => {
                    let _ = app.update(Message::Run("tool.line"));
                    snapshot.input(&mut app, App::view, &mut update, Input::Click(at));
                    at = Point::new(at.x + 120.0, at.y + 60.0);
                    snapshot.input(&mut app, App::view, &mut update, Input::Move(at));
                }
                3 => {
                    let parcel = Slot(4);
                    app.selection.set([parcel]);
                    let doc = app.document.as_ref().expect("open");
                    app.spatial.sync(&doc.model);
                    let first = app.spatial.grips(&[parcel])[0].points[0];
                    let [x, y] = app.viewport.camera.world_to_screen(first);
                    let local = Point::new(x as f32, y as f32);
                    let _ = app.update(Message::Viewport(crate::viewport::Event::RightPressed(local)));
                    let _ = app.update(Message::Viewport(crate::viewport::Event::RightClick(local)));
                    let _ = app.update(Message::DrawingMenu(crate::drawing_menus::Event::Grip(GripAction::Stretch)));
                    at = Point::new(area.x + local.x + 90.0, area.y + local.y - 50.0);
                    snapshot.input(&mut app, App::view, &mut update, Input::Move(at));
                }
                _ => {}
            }
            snapshot.settle(&mut app, App::view, &mut update);
            snapshot.input(&mut app, App::view, &mut update, Input::RightClick(at));
            save(&mut snapshot, &app, &out, &name);
        }
    }
}

/// The operations' Python in the console (docs/adr/0622 §7): with the setting on, a selection made by hand and a
/// move with the tool are written to the Python tab as the code that does the same: `duzen-python-*`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn python_echo() {
    use kentos_domain::Slot;

    let out = out();
    for (size, theme, suffix) in frames() {
        let name = format!("duzen-python{suffix}");
        if !wanted(&name) {
            continue;
        }
        let mut app = booted(theme, true);
        let _ = app
            .settings
            .choose(&[(crate::python::echo::SETTING, Value::Bool(true))]);
        app.show_bottom(crate::bottom::BottomTab::Python);
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        snapshot.settle(&mut app, App::view, &mut update);
        // A selection by hand, then Taşı from a corner of the parcel 6 m east.
        app.selection.set([Slot(4)]);
        update(&mut app, Message::DialogClosed);
        let doc = app.document.as_ref().expect("open");
        app.spatial.sync(&doc.model);
        let corner = app.spatial.grips(&[Slot(4)])[0].points[0];
        let area = app.viewport.bounds;
        let screen = |w: kentos_geometry_core::Vec2, app: &App| {
            let [x, y] = app.viewport.camera.world_to_screen(w);
            Point::new(area.x + x as f32, area.y + y as f32)
        };
        let from = screen(corner, &app);
        let to = screen(kentos_geometry_core::Vec2::new(corner.x + 6.0, corner.y), &app);
        update(&mut app, Message::Run("tool.move"));
        snapshot.input(&mut app, App::view, &mut update, Input::Move(from));
        snapshot.input(&mut app, App::view, &mut update, Input::Click(from));
        snapshot.input(&mut app, App::view, &mut update, Input::Move(to));
        snapshot.input(&mut app, App::view, &mut update, Input::Click(to));
        snapshot.settle(&mut app, App::view, &mut update);
        // A taller panel shows the code whole.
        app.bottom_dragged(Some(330.0), std::time::Instant::now());
        snapshot.settle(&mut app, App::view, &mut update);
        save(&mut snapshot, &app, &out, &name);
    }
}

/// Halka through real keyboard/mouse events, with the exact semantic call echoed; three reading settings.
#[test]
#[ignore = "GPU acceptance and screenshots; run by hand"]
fn donut_command_and_echo() {
    use iced::keyboard::key::Named;
    use kentos_ui::widget::python::EntryKind;

    // An actual window sends ignored widget events to the app's keyboard subscription.
    fn key(snapshot: &mut Snapshot, app: &mut App, key: iced::keyboard::Key, text: Option<&str>) {
        use iced::{Event, keyboard, window};
        use iced::keyboard::key::{NativeCode, Physical};
        let event = Event::Keyboard(keyboard::Event::KeyPressed {
            key: key.clone(), modified_key: key,
            physical_key: Physical::Unidentified(NativeCode::Unidentified),
            location: keyboard::Location::Standard, modifiers: keyboard::Modifiers::default(),
            text: text.map(Into::into), repeat: false,
        });
        let (messages, status) = snapshot.deliver(app.view(), &event);
        for message in messages { update(app, message); }
        if let Some(message) = crate::keys::key_event(event, status, window::Id::unique()) {
            update(app, message);
        }
        snapshot.settle(app, App::view, &mut update);
    }

    for (theme, text_size, name) in [
        ("light", 13, "duzen-halka-acik"),
        ("dark", 13, "duzen-halka-koyu"),
        ("dark", 18, "duzen-halka-buyuk"),
    ] {
        let mut app = booted(theme, true);
        let _ = app.settings.choose(&[
            (crate::python::echo::SETTING, Value::Bool(true)),
            ("appearance.textSize", Value::from(text_size)),
        ]);
        app.apply_settings();
        update(&mut app, Message::DialogClosed);
        let mut snapshot = Snapshot::new(Size::new(1440.0, 900.0)).expect("a renderer");
        snapshot.settle(&mut app, App::view, &mut update);
        let area = app.viewport.bounds;
        let at = Point::new(area.x + area.width * 0.7, area.y + area.height * 0.4);
        snapshot.input(&mut app, App::view, &mut update, Input::Move(at));
        let task = app.update(Message::Run("tool.donut"));
        if let Some(mut stream) = iced_runtime::task::into_stream(task) {
            use iced::futures::StreamExt as _;
            while let Some(action) = iced::futures::executor::block_on(stream.next()) {
                if let iced_runtime::Action::Widget(operation) = action {
                    snapshot.operate(app.view(), operation);
                }
            }
        }
        for (option, value) in [("I", "2"), ("D", "6")] {
            // Option letters act immediately; only a numeric value needs Enter.
            key(&mut snapshot, &mut app, iced::keyboard::Key::Character(option.into()), Some(option));
            key(&mut snapshot, &mut app, iced::keyboard::Key::Character(value.into()), Some(value));
            key(&mut snapshot, &mut app, iced::keyboard::Key::Named(Named::Enter), None);
        }
        snapshot.input(&mut app, App::view, &mut update, Input::Move(at));
        snapshot.input(&mut app, App::view, &mut update, Input::Click(at));
        let doc = &app.document.as_ref().expect("drawing").model;
        let call = doc.journal().last().expect("the donut call");
        assert_eq!(call.command.as_deref(), Some("cad.donut.create"));
        let input = call.input.as_ref().expect("semantic input");
        assert_eq!(input["inner"], 2.0);
        assert_eq!(input["outer"], 6.0);
        assert!(!input.as_object().expect("object").contains_key("objects"));
        assert!(app.python.repl.entries().any(|e| e.kind == EntryKind::Echo && e.text.contains("cad.donut.create.execute_input")));
        let before = doc.len();
        snapshot.input(&mut app, App::view, &mut update, Input::RightClick(at));
        assert_eq!(app.document.as_ref().expect("drawing").model.len(), before, "right click places nothing");
        key(&mut snapshot, &mut app, iced::keyboard::Key::Named(Named::Escape), None);
        key(&mut snapshot, &mut app, iced::keyboard::Key::Named(Named::Escape), None);
        app.show_bottom(crate::bottom::BottomTab::Python);
        app.bottom_dragged(Some(330.0), std::time::Instant::now());
        snapshot.settle(&mut app, App::view, &mut update);
        save(&mut snapshot, &app, &out(), name);
    }
}
