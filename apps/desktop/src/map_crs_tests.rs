//! The map's system and a layer's source system on the desktop (docs/adr/0583): a TUREF / TM36 drawing seen in Web
//! Mercator and in WGS 84 degrees picks and snaps as it does in its own system, its status bar says where the
//! pointer is on the map, and a layer takes its source system and moves out of it.

use kentos_domain::Slot;

use crate::app::{App, Message};
use crate::map_crs::Choose;
use crate::tools_scenes::{Objects, at, click, open};

/// A road, a parcel and a 300 km grid of lines round the scenes' place (E 487 000, N 4 420 000).
fn ground() -> Objects {
    let mut o = Objects::new();
    o.line("yol", [0.0, 0.0], [120.0, 40.0]);
    o.path(
        "parsel",
        &[[20.0, -60.0], [90.0, -60.0], [90.0, -20.0], [20.0, -20.0]],
        true,
    );
    for i in -3..=3 {
        let d = f64::from(i) * 50_000.0;
        o.line("cizim", [d, -150_000.0], [d, 150_000.0]);
        o.line("cizim", [-150_000.0, d], [150_000.0, d]);
    }
    o
}

fn app() -> App {
    let (mut app, _) = App::boot(None);
    open(&mut app, ground());
    let _ = app.update(Message::Viewport(crate::viewport::Event::Resized(
        iced::Rectangle::new(iced::Point::ORIGIN, iced::Size::new(1000.0, 700.0)),
    )));
    app.draft.snap = false;
    app
}

/// Shows the road and the parcel as large as they fit.
fn near(app: &mut App) {
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: 487_000.0 - 10.0,
            min_y: 4_420_000.0 - 70.0,
            max_x: 487_000.0 + 130.0,
            max_y: 4_420_000.0 + 50.0,
        },
        20.0,
    );
}

#[test]
fn a_click_on_the_map_selects_what_it_selects_in_the_project() {
    for map in [None, Some(3857), Some(4326), Some(2320)] {
        let mut app = app();
        let _ = app.update(Message::MapCrs(Choose(map)));
        assert_eq!(app.viewport.camera.map.is_some(), map.is_some(), "{map:?}");
        assert_eq!(
            app.document
                .as_ref()
                .expect("a drawing")
                .settings()
                .map_srid,
            map,
            "{map:?}"
        );
        near(&mut app);
        // The road's middle, where the screen shows it on the map.
        click(&mut app, [60.0, 20.0]);
        assert!(app.selection.contains(Slot(1)), "{map:?}: the road");
        // The parcel's edge.
        let _ = app.update(Message::Run("edit.deselect"));
        click(&mut app, [55.0, -60.0]);
        assert!(app.selection.contains(Slot(2)), "{map:?}: the parcel");
        // The pointer reads back where it is: the world point under a pixel is the one drawn there.
        let p = at(&app, [90.0, -20.0]);
        let w = app.viewport.world(p);
        assert!(
            (w.x - 487_090.0).abs() < 1e-3 && (w.y - 4_419_980.0).abs() < 1e-3,
            "{map:?}: {w:?}"
        );
    }
}

#[test]
fn the_map_is_a_view_the_drawing_does_not_move() {
    let mut app = app();
    let before: Vec<_> = app
        .document
        .as_ref()
        .expect("a drawing")
        .model
        .entities()
        .cloned()
        .collect();
    let _ = app.update(Message::MapCrs(Choose(Some(3857))));
    let doc = app.document.as_ref().expect("a drawing");
    let after: Vec<_> = doc.model.entities().cloned().collect();
    assert_eq!(before, after);
    assert!(crate::files_testing::last_said(&app).contains("Çizim dönüştürülmedi"));
    // The status bar's cell names the map and the pointer on it.
    assert!(
        app.map_cell(Some(kentos_interaction::Vec2::new(487_000.0, 4_420_000.0)))
            .is_some()
    );
    // Back to the project's own: the camera's map goes.
    let _ = app.update(Message::MapCrs(Choose(None)));
    assert!(app.viewport.camera.map.is_none());
    assert_eq!(
        app.document
            .as_ref()
            .expect("a drawing")
            .settings()
            .map_srid,
        None
    );
    assert!(app.map_cell(None).is_none());
}

#[test]
fn a_layer_takes_its_source_system_and_moves_out_of_it_in_one_step() {
    use crate::layer_crs::Event;
    let mut app = app();
    let _ = app.update(Message::LayerCrs(Event::Assign("yol".into(), Some(2320))));
    let node = |app: &App| {
        app.document
            .as_ref()
            .expect("a drawing")
            .model
            .layers()
            .get("yol")
            .cloned()
            .expect("the layer")
    };
    let crs = node(&app).source_crs.expect("the record");
    assert_eq!(crs.srid, Some(2320));
    assert!(crs.wkt.contains("ID[\"EPSG\",2320]"), "{}", crs.wkt);
    let road = |app: &App| {
        app.document
            .as_ref()
            .expect("a drawing")
            .model
            .get(Slot(1))
            .cloned()
            .expect("the road")
    };
    let before = road(&app);
    // Yeniden izdüşür on the active layer, from its record: ED50 / TM30's numbers into TUREF / TM36's.
    let _ = app.update(Message::Layer(crate::layering::Event::Activate(
        "yol".into(),
    )));
    let _ = app.update(Message::Run("layer.reproject"));
    let after = road(&app);
    assert_ne!(before, after);
    assert!(crate::files_testing::last_said(&app).contains("1 nesnesi"));
    // One undo step takes the move back, the record with it.
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(road(&app), before);
    assert_eq!(node(&app).source_crs.and_then(|c| c.srid), Some(2320));
    // Taking the record away.
    let _ = app.update(Message::LayerCrs(Event::Assign("yol".into(), None)));
    assert!(node(&app).source_crs.is_none());
}

/// The grid and the parcels in the project's system, in Web Mercator and in WGS 84 degrees, the map menu and the
/// layer's Koordinat sistemi ▸, light and dark, 1440 × 900 and 1100 × 650; `.run/shots/harita-sistemi-*`:
///
/// ```text
/// cargo test -p kentos-desktop map_crs_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::{Point, Size};
    use kentos_ui::snapshot::{Input, Snapshot};

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let only = std::env::var("KENTOS_SHOTS_ONLY").ok();
    let wanted = |name: &str| {
        only.as_deref()
            .is_none_or(|o| o.split(',').any(|n| n == name))
    };
    for (theme, w, h) in [
        ("dark", 1440.0, 900.0),
        ("light", 1440.0, 900.0),
        ("dark", 1100.0, 650.0),
    ] {
        let tag = format!("{theme}-{w}");
        let save = |snapshot: &mut Snapshot, app: &App, name: &str| {
            let file = out.join(format!("harita-sistemi-{name}-{tag}.png"));
            snapshot
                .render(app.view(), &app.theme())
                .save(&file)
                .expect("writes the picture");
            println!("{}", file.display());
        };
        for (name, map) in [
            ("proje", None),
            ("mercator", Some(3857)),
            ("cografi", Some(4326)),
        ] {
            if !wanted(name) {
                continue;
            }
            let (mut app, _) = App::boot(None);
            let _ = app
                .settings
                .choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            open(&mut app, ground());
            let _ = app.update(Message::MapCrs(Choose(map)));
            let mut snapshot = Snapshot::new(Size::new(w, h)).expect("a renderer");
            let mut update = |app: &mut App, message| {
                let _ = app.update(message);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            let _ = app.update(Message::Run("view.zoomExtents"));
            app.follow.flash = None;
            snapshot.settle(&mut app, App::view, &mut update);
            snapshot.input(
                &mut app,
                App::view,
                &mut update,
                Input::Move(Point::new(w * 0.62, h * 0.4)),
            );
            save(&mut snapshot, &app, name);
        }
        if w < 1400.0 {
            continue;
        }
        // The coordinate system cell's menu, Harita sistemi open.
        if wanted("menu") {
            let (mut app, _) = App::boot(None);
            let _ = app
                .settings
                .choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            open(&mut app, ground());
            let _ = app.update(Message::MapCrs(Choose(Some(3857))));
            let mut snapshot = Snapshot::new(Size::new(w, h)).expect("a renderer");
            let mut update = |app: &mut App, message| {
                let _ = app.update(message);
            };
            app.follow.flash = None;
            snapshot.settle(&mut app, App::view, &mut update);
            if let Some(cell) = crate::files_testing::find_text(&mut snapshot, &app, "TUREF / TM36")
            {
                snapshot.input(
                    &mut app,
                    App::view,
                    &mut update,
                    Input::RightClick(cell.center()),
                );
                let item = Point::new(cell.center().x - 100.0, cell.center().y + 77.0);
                snapshot.input(&mut app, App::view, &mut update, Input::Move(item));
                save(&mut snapshot, &app, "menu");
            }
        }
        // The layer's Koordinat sistemi ▸ with its record.
        if wanted("katman") {
            let (mut app, _) = App::boot(None);
            let _ = app
                .settings
                .choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            open(&mut app, ground());
            let _ = app.update(Message::LayerCrs(crate::layer_crs::Event::Assign(
                "yol".into(),
                Some(2320),
            )));
            let mut snapshot = Snapshot::new(Size::new(w, h)).expect("a renderer");
            let mut update = |app: &mut App, message| {
                let _ = app.update(message);
            };
            app.follow.flash = None;
            snapshot.settle(&mut app, App::view, &mut update);
            let rows = crate::files_testing::find_texts(&mut snapshot, &app, "Yol");
            if let Some(row) = rows.into_iter().find(|r| r.x > 900.0) {
                snapshot.input(
                    &mut app,
                    App::view,
                    &mut update,
                    Input::RightClick(row.center()),
                );
                // Koordinat sistemi ▸, the menu's thirteenth row, open.
                let item = Point::new(1080.0, row.center().y + 290.0);
                snapshot.input(&mut app, App::view, &mut update, Input::Move(item));
                save(&mut snapshot, &app, "katman-menu");
            }
        }
    }
}
