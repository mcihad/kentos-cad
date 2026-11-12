//! Pictures and measurements for the owner, run by hand: the labels on screen, texts' extras, leaders,
//! dimensions and annotations; a frame's cost; a file's labels painted.

use super::*;

/// Pictures of the drawing's text for the owner: the sample drawing with its
/// “Çizim” layer shown (a text, a dimension, a point's name, the parcel's
/// number), dark and light, at 1440×900 and 1100×650, and closer in;
/// `.run/shots/yazi-*`:
///
/// ```text
/// cargo test -p kentos-desktop labels::screens -- --ignored --nocapture
/// ```
#[cfg(test)]
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use kentos_ui::snapshot::Snapshot;

    use crate::app::App;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height, zoom) in [
            (1440.0, 900.0, 1.0),
            (1100.0, 650.0, 1.0),
            (1440.0, 900.0, 3.0),
        ] {
            let mut app = crate::files_testing::app_with_drawing();
            let _ = app
                .settings
                .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
            app.apply_settings();
            let _ = app.update(Message::LayerVisible("cizim".into()));
            let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
            let mut update = |app: &mut App, message| {
                let _ = app.update(message);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            let _ = app.update(Message::Run("view.zoomExtents"));
            if zoom > 1.0 {
                let camera = &mut app.viewport.camera;
                let (cx, cy) = (camera.width / 2.0, camera.height / 2.0);
                camera.zoom_at(zoom, cx, cy);
            }
            snapshot.settle(&mut app, App::view, &mut update);
            let name = if zoom > 1.0 {
                format!("yazi-yakin{suffix}")
            } else {
                format!("yazi-{width}x{height}{suffix}")
            };
            let file = out.join(format!("{name}.png"));
            snapshot
                .render(app.view(), &app.theme())
                .save(&file)
                .expect("writes the picture");
            println!("{}", file.display());
        }
    }
}

/// Text extras (docs/adr/0145) as the drawing shows them: the twelve
/// alignments at their marked points, a turned centred text, width factors
/// 0.6, 1 and 1.5, and a masked text next to one without over a hatch and a
/// line (fixtures/interaction/v1/text-extras.kcad, the web's
/// `shots.mjs texts`), dark and light, at 1440×900 and 1100×650, and
/// closer in; `.run/shots/yazi-ekleri-*`:
///
/// ```text
/// cargo test -p kentos-desktop labels::text_extras_screens -- --ignored --nocapture
/// ```
#[cfg(test)]
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn text_extras_screens() {
    use kentos_ui::snapshot::Snapshot;

    use crate::app::App;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let drawing = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/interaction/v1/text-extras.kcad"
    );
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height, zoom, name) in [
            (1440.0, 900.0, 1.0, ""),
            (1100.0, 650.0, 1.0, ""),
            (1440.0, 900.0, 2.5, "zemin"),
            (1440.0, 900.0, 5.0, "donuk"),
        ] {
            let (mut app, _) = App::boot(None);
            let _ = app
                .settings
                .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
            app.apply_settings();
            let doc =
                crate::document::Document::read(std::path::Path::new(drawing)).expect("opens");
            let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
            let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
            let mut update = |app: &mut App, message| {
                let _ = app.update(message);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            let _ = app.update(Message::Run("view.zoomExtents"));
            if zoom > 1.0 {
                // Closer in: on the masked text, or on the turned one and its point.
                let camera = &mut app.viewport.camera;
                camera.center = if name == "zemin" {
                    kentos_interaction::Vec2::new(487108.0, 4419985.0)
                } else {
                    kentos_interaction::Vec2::new(487118.0, 4420025.0)
                };
                camera.scale *= zoom;
            }
            snapshot.settle(&mut app, App::view, &mut update);
            let name = if zoom > 1.0 {
                format!("yazi-ekleri-{name}{suffix}")
            } else {
                format!("yazi-ekleri-{width}x{height}{suffix}")
            };
            let file = out.join(format!("{name}.png"));
            snapshot
                .render(app.view(), &app.theme())
                .save(&file)
                .expect("writes the picture");
            println!("{}", file.display());
        }
    }
}

/// Leaders (docs/adr/0146) as the drawing shows them: the four arrowheads,
/// one without a note, one masked, one turned and one in a block
/// (fixtures/interaction/v1/leaders.kcad, the web's `shots.mjs leaders`),
/// the second selected so that Öznitelikler shows it, dark and light, at
/// 1440×900 and 1100×650; `.run/shots/kilavuz-*`:
///
/// ```text
/// cargo test -p kentos-desktop labels::leader_screens -- --ignored --nocapture
/// ```
#[cfg(test)]
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn leader_screens() {
    use kentos_ui::snapshot::Snapshot;

    use crate::app::App;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let drawing = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/interaction/v1/leaders.kcad"
    );
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            let (mut app, _) = App::boot(None);
            let _ = app
                .settings
                .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
            app.apply_settings();
            let doc =
                crate::document::Document::read(std::path::Path::new(drawing)).expect("opens");
            let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
            let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
            let mut update = |app: &mut App, message| {
                let _ = app.update(message);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            let _ = app.update(Message::Run("view.zoomExtents"));
            app.selection.set([kentos_domain::Slot(2)]);
            snapshot.settle(&mut app, App::view, &mut update);
            let file = out.join(format!("kilavuz-{width}x{height}{suffix}.png"));
            snapshot
                .render(app.view(), &app.theme())
                .save(&file)
                .expect("writes the picture");
            println!("{}", file.display());
        }
    }
}

/// The dimensions of docs/adr/0147 as the drawing shows them, over a parcel
/// and a curved road (fixtures/interaction/v1/dimensions.kcad, the web's
/// `shots.mjs dimensions`): the north-west corner's Y and X, the road
/// edge's arc length and its jogged radius, the west edge's azimuth, the
/// east edge's slope between its corners' elevations, the north edge's
/// value masked over the hatch and a slope in a block; the arc length
/// selected so that Öznitelikler shows it, dark and light, at 1440×900 and
/// 1100×650; `.run/shots/olcu-turleri-*`:
///
/// ```text
/// cargo test -p kentos-desktop labels::dimension_screens -- --ignored --nocapture
/// ```
#[cfg(test)]
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn dimension_screens() {
    use kentos_ui::snapshot::Snapshot;

    use crate::app::App;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let drawing = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/interaction/v1/dimensions.kcad"
    );
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            let (mut app, _) = App::boot(None);
            let _ = app
                .settings
                .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
            app.apply_settings();
            let doc =
                crate::document::Document::read(std::path::Path::new(drawing)).expect("opens");
            let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
            let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
            let mut update = |app: &mut App, message| {
                let _ = app.update(message);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            let _ = app.update(Message::Run("view.zoomExtents"));
            app.selection.set([kentos_domain::Slot(9)]);
            snapshot.settle(&mut app, App::view, &mut update);
            let file = out.join(format!("olcu-turleri-{width}x{height}{suffix}.png"));
            snapshot
                .render(app.view(), &app.theme())
                .save(&file)
                .expect("writes the picture");
            println!("{}", file.display());
        }
    }
}

/// The annotations of docs/adr/0205 as the drawing and its windows show
/// them (fixtures/interaction/v1/annotations.kcad, the web's `shots.mjs
/// annotations`; scripts/fixtures/annotation_scene.py writes it), dark and
/// light, at 1440×900 (and the drawing at 1100×650); `.run/shots/aciklama-*`:
///
/// - the parcel with its dimensions, the one following “Renkli çizgiler”
///   selected so that Öznitelikler shows its lines' rows; the leaders' rows
///   of arrowheads closer in;
/// - far out at 1:1000, Kaybolmasın and Gerçek boy;
/// - Ölçü stilleri on the style, Proje ayarları › Ölçek ve yazılar, Ölçek yaz….
///
/// ```text
/// cargo test -p kentos-desktop labels::annotation_screens -- --ignored --nocapture
/// ```
#[cfg(test)]
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn annotation_screens() {
    use kentos_ui::snapshot::Snapshot;

    use crate::app::App;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let drawing = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/interaction/v1/annotations.kcad"
    );
    let only = std::env::var("KENTOS_SHOTS_ONLY").ok();
    let wanted = |name: &str| {
        only.as_deref()
            .is_none_or(|o| o.split(',').any(|w| name.starts_with(w)))
    };
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            let open = || {
                let (mut app, _) = App::boot(None);
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                let doc =
                    crate::document::Document::read(std::path::Path::new(drawing)).expect("opens");
                let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
                app
            };
            let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
            // Settled, then written when named.
            let mut picture = |app: &mut App, name: Option<&str>| {
                let mut update = |app: &mut App, message| {
                    let _ = app.update(message);
                };
                snapshot.settle(app, App::view, &mut update);
                let Some(name) = name else { return };
                let file = out.join(format!("{name}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            };
            // The drawing, the styled dimension selected.
            let mut app = open();
            picture(&mut app, None);
            let _ = app.update(Message::Run("view.zoomExtents"));
            app.selection.set([kentos_domain::Slot(4)]);
            // Genel closed, so that the dimension's line rows show.
            let _ = app.update(Message::Properties(crate::properties::Event::Toggle(
                "general",
            )));
            let name = format!("aciklama-{width}x{height}{suffix}");
            if wanted("aciklama-cizim") {
                picture(&mut app, Some(&name));
            }
            if width < 1440.0 {
                continue;
            }
            // The arrowheads closer in, nothing selected.
            app.selection.set([]);
            let camera = &mut app.viewport.camera;
            // The two rows (−40 … 140 m east) across the drawing area: 5.6 px a metre.
            camera.center = kentos_interaction::Vec2::new(500_050.0, 4_400_072.0);
            camera.scale = 5.6;
            if wanted("aciklama-oklar") {
                picture(&mut app, Some(&format!("aciklama-oklar{suffix}")));
            }
            // Far out: the texts 2.5 m high under a few pixels.
            for (size, word) in [("legible", "kaybolmasin"), ("true", "gercek")] {
                let _ = app
                    .settings
                    .choose(&[("graphics.annotationSize", serde_json::Value::from(size))]);
                app.apply_settings();
                let _ = app.update(Message::Run("view.zoomExtents"));
                let camera = &mut app.viewport.camera;
                camera.scale /= 5.0;
                if wanted("aciklama-uzak") {
                    picture(&mut app, Some(&format!("aciklama-uzak-{word}{suffix}")));
                }
            }
            // The windows.
            let _ = app.update(Message::Run("view.zoomExtents"));
            app.open_annotation_styles(crate::annotation_styles::Kind::Dimension);
            let _ = app.update(Message::AnnotationStyles(
                crate::annotation_styles::Event::Choose(Some(
                    "0192f6a0-0000-7000-8000-000000000203".into(),
                )),
            ));
            if wanted("aciklama-olcu-stilleri") {
                picture(&mut app, Some(&format!("aciklama-olcu-stilleri{suffix}")));
            }
            let _ = app.update(Message::AnnotationStyles(
                crate::annotation_styles::Event::Cancel,
            ));
            let _ = app.update(Message::Run("file.settings"));
            let _ = app.update(crate::project::settings_message(
                crate::project::SettingsEvent::Section(crate::project::settings::Section::Units),
            ));
            if wanted("aciklama-proje-olcek") {
                picture(&mut app, Some(&format!("aciklama-proje-olcek{suffix}")));
            }
            let _ = app.update(Message::Project(Box::new(crate::project::Event::Close)));
            let _ = app.update(Message::PlotScale(crate::annotation_scale::Event::Open));
            let _ = app.update(Message::PlotScale(crate::annotation_scale::Event::Edit(
                "2500".into(),
            )));
            if wanted("aciklama-olcek-yaz") {
                picture(&mut app, Some(&format!("aciklama-olcek-yaz{suffix}")));
            }
            // Öznitelikler's rows of the dimension's lines, in a window tall enough for them (an
            // app of its own: the drawing area keeps the size it was laid out at).
            if wanted("aciklama-oznitelikler") {
                let mut app = open();
                let mut tall = Snapshot::new(Size::new(1440.0, 1500.0)).expect("a renderer");
                let mut update = |app: &mut App, message| {
                    let _ = app.update(message);
                };
                tall.settle(&mut app, App::view, &mut update);
                let _ = app.update(Message::Run("view.zoomExtents"));
                app.selection.set([kentos_domain::Slot(4)]);
                let _ = app.update(Message::Properties(crate::properties::Event::Toggle(
                    "general",
                )));
                tall.settle(&mut app, App::view, &mut update);
                let file = out.join(format!("aciklama-oznitelikler{suffix}.png"));
                tall.render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
}

/// Frame times with thousands of labels on screen, in release:
///
/// ```text
/// cargo test --release -p kentos-desktop labels::perf -- --ignored --nocapture
/// ```
///
/// 50 176 named points 1 m apart, viewed at 4 px/m (labels beside points
/// show from 2 px/m): the store offers the labels in view, the room thins
/// them. Each frame pans a little, so the labels are laid out again.
#[cfg(test)]
#[test]
#[ignore = "a measurement, run by hand in release"]
fn perf() {
    use std::time::Instant;

    use kentos_commands::{DocumentSnapshotV1, EntityBase, PointEntity};
    use kentos_ui::snapshot::Snapshot;

    use crate::app::App;

    let mut snapshot: DocumentSnapshotV1 =
        serde_json::from_str(include_str!("../../../../../fixtures/document/v1/sample.json"))
            .expect("the sample");
    let first = snapshot
        .entities
        .iter()
        .map(|e| e.base().id)
        .max()
        .unwrap_or(0);
    let side = 224;
    for i in 0..side * side {
        snapshot.entities.push(Entity::Point(PointEntity {
            base: EntityBase {
                look: Default::default(),
                id: first + i as u32 + 1,
                layer_id: "parsel".into(),
                color: None,
                attrs: Default::default(),
                label: Some(format!("N{i}")),
                symbol: None,
                line_weight: None,
                label_pins: Vec::new(),
                attachments: Vec::new(),
            },
            p: kentos_commands::Vec2 {
                x: 486_400.0 + (i % side) as f64,
                y: 4_420_000.0 + (i / side) as f64,
            },
            z: None,
            parts: None,
        }));
    }
    let doc = crate::document::Document::new(snapshot, None).expect("opens");
    let (mut app, _) = App::boot(None);
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    let mut shot = Snapshot::new(Size::new(1440.0, 900.0)).expect("a renderer");
    let mut update = |app: &mut App, message| {
        let _ = app.update(message);
    };
    shot.settle(&mut app, App::view, &mut update);
    app.viewport.camera.scale = 4.0;
    app.viewport.camera.center = Vec2::new(486_512.0, 4_420_112.0);
    let _ = shot.render(app.view(), &app.theme());
    let spots = {
        let v = app.viewport.camera.visible_bounds();
        app.spatial
            .labels(
                Vec2::new(v.min_x, v.min_y),
                Vec2::new(v.max_x, v.max_y),
                4.0,
                Default::default(),
            )
            .len()
    };
    let frames = 10;
    let started = Instant::now();
    for _ in 0..frames {
        app.viewport.camera.pan_by(3.0, 2.0);
        let _ = shot.render(app.view(), &app.theme());
    }
    let per_frame = started.elapsed().as_secs_f64() * 1000.0 / f64::from(frames);
    println!("{spots} labels offered in view; {per_frame:.1} ms a frame (render included)");
    // The parts: the store's query, and laying the labels out (no drawing).
    let camera = app.viewport.camera;
    let v = camera.visible_bounds();
    let started = Instant::now();
    for _ in 0..frames {
        let _ = app.spatial.labels(
            Vec2::new(v.min_x, v.min_y),
            Vec2::new(v.max_x, v.max_y),
            4.0,
            Default::default(),
        );
    }
    let query = started.elapsed().as_secs_f64() * 1000.0 / f64::from(frames);
    let doc = &app.document.as_ref().expect("open").model;
    let labels = Labels {
        doc,
        map_layer_colors: None,
        own_colors: own_colors(doc),
        spots: parts_of(
            app.spatial
                .labels(
                    Vec2::new(v.min_x, v.min_y),
                    Vec2::new(v.max_x, v.max_y),
                    4.0,
                    Default::default(),
                )
                .into_iter()
                .map(|s| (s, Grow::NONE))
                .collect(),
        ),
        view: camera,
        camera: Cell::new(camera),
        colors: colors(Canvas::Slate, &crate::viewport::palette(Canvas::Slate)),
        font: DrawingFont::Barlow,
        format: Format::of(doc.settings()),
        key: 0,
        fence: None,
        current: Cell::new(None),
        faded: Rc::default(),
        map_marks: Some(0.0),
        preview: None,
        pinned: false,
        turned: None,
        true_size: false,
        bands: RefCell::new(Vec::new()),
        margin: Cell::new(0.0),
        band_memo: None,
        band_key: 0,
        bands_out: None,
        moving: false,
        held: None,
    };
    let started = Instant::now();
    for _ in 0..frames {
        let mut frame = Frame::new(shot.renderer(), Size::new(1440.0, 900.0));
        labels.paint(&mut frame);
    }
    let paint = started.elapsed().as_secs_f64() * 1000.0 / f64::from(frames);
    println!("query {query:.1} ms, laying out {paint:.1} ms a frame");
}

/// The drawing's texts of a file of one's own laid out under true sizes (docs/adr/0626 §6), in release: what a
/// frame's label layer costs, the store's answer, the pieces, and the bands' fill alone.
///
/// ```text
/// KENTOS_PROBE_FILE=$HOME/Downloads/deneme_suşehri.dxf cargo test --release -p kentos-desktop labels::file_paint -- --ignored --nocapture
/// ```
#[cfg(test)]
#[test]
#[ignore = "a measurement on a file of one's own, run by hand in release"]
fn file_paint() {
    use std::time::Instant;

    use kentos_ui::snapshot::Snapshot;

    use crate::app::App;

    let Some(path) = std::env::var_os("KENTOS_PROBE_FILE") else {
        println!("KENTOS_PROBE_FILE verilmedi");
        return;
    };
    let path = std::path::PathBuf::from(path);
    let data = std::fs::read(&path).expect("the file reads");
    let format = kentos_native_application::cad_open::Format::of(&path).expect("a CAD file");
    let opened = kentos_native_application::cad_open::read(
        &path,
        &data,
        format,
        &mut kentos_formats::watch::Steps(|_, _| true),
    )
    .expect("it opens");
    let doc = crate::document::Document::of_model(opened.document);
    let (mut app, _) = App::boot(None);
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    let _ = app.update(Message::DialogClosed);
    let mut shot = Snapshot::new(Size::new(1440.0, 900.0)).expect("a renderer");
    let mut update = |app: &mut App, message| {
        let _ = app.update(message);
    };
    shot.settle(&mut app, App::view, &mut update);
    let _ = app.update(Message::Run("view.zoomExtents"));
    shot.settle(&mut app, App::view, &mut update);
    // Eight wheel steps in (0.15 each, docs/adr/0056).
    app.viewport.camera.scale *= (8.0f64 * 0.15).exp();
    let camera = app.viewport.camera;
    let v = camera.visible_bounds();
    let frames = 10;
    let started = Instant::now();
    let mut answered = Vec::new();
    for _ in 0..frames {
        answered = app.spatial.labels_shown_in(
            Vec2::new(v.min_x, v.min_y),
            Vec2::new(v.max_x, v.max_y),
            camera.scale,
            LabelSize::Print,
            Default::default(),
            false,
            None,
        );
    }
    let query = started.elapsed().as_secs_f64() * 1000.0 / f64::from(frames);
    let doc = &app.document.as_ref().expect("open").model;
    let labels = Labels {
        doc,
        map_layer_colors: None,
        own_colors: own_colors(doc),
        spots: parts_of(answered),
        view: camera,
        camera: Cell::new(camera),
        colors: colors(Canvas::Slate, &crate::viewport::palette(Canvas::Slate)),
        font: DrawingFont::Barlow,
        format: Format::of(doc.settings()),
        key: 0,
        fence: None,
        current: Cell::new(None),
        faded: Rc::default(),
        map_marks: None,
        preview: None,
        pinned: false,
        turned: None,
        true_size: true,
        bands: RefCell::new(Vec::new()),
        margin: Cell::new(0.0),
        band_memo: None,
        band_key: 0,
        bands_out: None,
        moving: false,
        held: None,
    };
    let started = Instant::now();
    for _ in 0..frames {
        let mut frame = Frame::new(shot.renderer(), Size::new(1440.0, 900.0));
        labels.paint(&mut frame);
    }
    let paint = started.elapsed().as_secs_f64() * 1000.0 / f64::from(frames);
    // The bands alone: the pieces' walk without their fill, then the fill.
    let size = Size::new(1440.0, 900.0);
    let near = labels
        .spots
        .iter()
        .filter(|(s, _)| labels.near_view(s))
        .count();
    let segments: Vec<[Point; 2]> = (0..near)
        .map(|i| {
            let x = (i % 120) as f32 * 12.0;
            let y = (i / 120) as f32 * 6.0;
            [Point::new(x, y), Point::new(x + 8.0, y + 1.0)]
        })
        .collect();
    let started = Instant::now();
    for _ in 0..frames {
        let mut frame = Frame::new(shot.renderer(), size);
        frame.stroke_segments(&segments, Color::WHITE, 1.0);
    }
    let fill = started.elapsed().as_secs_f64() * 1000.0 / f64::from(frames);
    println!(
        "{} yazı soruldu ({query:.1} ms), {near} tanesi görünüme yakın; katmanın kurulması {paint:.1} ms, \
         {near} şeridin çizgisi tek başına {fill:.1} ms",
        labels.spots.len()
    );
}
