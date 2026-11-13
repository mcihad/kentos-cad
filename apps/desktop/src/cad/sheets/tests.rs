use iced::Size;
use kentos_sheet_ui::questions::QuestionsMessage;
use kentos_sheet_ui::{InspectorTab, Message as Sheet};

/// Yeni pafta: the template's questions (design §12) answered with its own values.
fn new_sheet(app: &mut App) {
    let _ = app.update(Message::Sheet(Sheet::NewSheet));
    if app.sheets.questions().is_some() {
        let _ = app.update(Message::Sheet(Sheet::Questions(QuestionsMessage::Make)));
    }
}

use super::*;
use crate::files_testing::app_with_drawing;

#[test]
fn paper_style_cache_preserves_drawing_pattern_lengths_at_each_viewport_scale() {
    use kentos_native_style::batches::BatchKind;
    for unit in ["drawing", "paper"] {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../fixtures/sheet/v1/native-preflight.json"
        ))
        .unwrap();
        let mut raw = fixture["model"].clone();
        raw["settings"]["lineTypes"] = serde_json::json!([{
            "name":"P", "unit":unit,
            "elements":[{"length":100.},{"length":-50.}]
        }]);
        raw["entities"][0]["linePattern"] = serde_json::json!("P");
        let doc = Document::new(serde_json::from_value(raw).unwrap(), None).unwrap();
        let before = doc.model.to_snapshot_v2();
        let spatial = Spatial::of(&doc.model);
        let library = kentos_plot::book::library_of(&doc.model);
        let maps = SheetMaps::default();
        let layers: MapLayers = serde_json::from_value(serde_json::json!({"type":"all"})).unwrap();
        let mut previous = None;
        for scale in [1., 20., 100.] {
            let view =
                serde_json::from_value(serde_json::json!({"scale":scale,"rotation":0})).unwrap();
            let build = || {
                maps.styled_at(
                    &doc,
                    spatial.store(),
                    &library,
                    &view,
                    (&layers, &[]),
                    None,
                    (None, &[], &[], false, &[]),
                )
            };
            let styled = build();
            assert!(Rc::ptr_eq(&styled, &build()), "same view reuses its build");
            if let Some(old) = previous.as_ref() {
                assert!(
                    !Rc::ptr_eq(old, &styled),
                    "different scales have separate builds"
                );
            }
            let dashes: Vec<_> = styled
                .layers
                .iter()
                .flat_map(|(_, _, l)| &l.batches)
                .filter_map(|b| match &b.kind {
                    BatchKind::Stroke { dash: Some(d), .. } => Some(d),
                    _ => None,
                })
                .collect();
            assert_eq!(dashes.len(), 1, "{unit}/{scale}");
            let factor = if unit == "drawing" {
                0.001
            } else {
                scale / 1000.
            };
            for (&actual, expected) in dashes[0].iter().zip([100. * factor, 50. * factor]) {
                assert!(
                    (actual - expected).abs() < 1e-10,
                    "{unit}/{scale}: {actual} != {expected}"
                );
            }
            previous = Some(styled);
        }
        assert_eq!(doc.model.to_snapshot_v2(), before);
    }
}

#[test]
fn paper_at_the_project_scale_never_reuses_a_more_detailed_model_camera_index() {
    use kentos_geometry_core::geom::wall::Detail;
    let mut app = app_with_drawing();
    crate::files_testing::make_cad(&mut app);
    let model = &mut app.document.as_mut().unwrap().model;
    let mut settings = model.settings().clone();
    settings.plot_scale = 100.;
    settings.arch = None;
    model.set_settings(settings);
    model
        .add_many(
            vec![
                serde_json::from_value(serde_json::json!({
                    "kind":"wall","id":0,"layerId":"cizim","attrs":{},"width":0.2,
                    "pts":[{"x":0.,"y":0.},{"x":10.,"y":0.}]
                }))
                .unwrap(),
            ],
            "test wall",
        )
        .unwrap();
    let doc = app.document.as_ref().unwrap();
    let before = doc.model.to_snapshot_v2();
    let mut main = Spatial::of(&doc.model);
    main.set_screen_wall_scale(&doc.model, Some(20.));
    assert_eq!(main.store().wall_view().detail, Detail::Detailed);
    let view: kentos_sheet::display::MapViewPrim =
        serde_json::from_value(serde_json::json!({"scale":100.,"rotation":0})).unwrap();
    let maps = SheetMaps::default();
    let copied = maps
        .scaled(doc, &view, None, None, None, &[], None, &main)
        .expect("separate paper index despite equal project scale");
    let (_, paper) = copied.view.as_ref().unwrap();
    assert_eq!(paper.store().wall_view().detail, Detail::Schematic);
    assert_eq!(paper.store().wall_view().scale, 100.);
    main.set_screen_wall_scale(&doc.model, Some(10.));
    let again = maps
        .scaled(doc, &view, None, None, None, &[], None, &main)
        .unwrap();
    assert!(
        Rc::ptr_eq(&copied, &again),
        "camera motion does not rebuild the paper cache"
    );
    assert_eq!(doc.model.to_snapshot_v2(), before);
    main.set_screen_wall_scale(&doc.model, None);
    assert!(
        maps.scaled(doc, &view, None, None, None, &[], None, &main)
            .is_none(),
        "matching ordinary paper index may still be shared"
    );
}

/// docs/adr/0636 §2: the copy a map at another scale draws from holds the rows round a bound layer's part too.
#[test]
fn a_copy_at_another_scale_takes_the_band() {
    let drawing = serde_json::json!({
        "format": "kentos.document", "version": 1, "name": "Bant",
        "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2",
            "angleUnit": "grad", "plotScale": 100, "workspace": "gis",
            "sources": [{"id": "kaynak-1", "kind": "geopackage", "path": "/yok/komsu.gpkg", "table": "parsel",
                         "layer": "parsel", "revision": "0".repeat(64), "digest": "0".repeat(64),
                         "srid": 5256, "extent": [0.0, 0.0, 10.0, 10.0], "writable": true}]},
        "origin": {"x": 0.0, "y": 0.0},
        "layers": [{"id": "parsel", "name": "Parsel", "type": "layer", "visible": true, "locked": false,
                    "expanded": true, "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25},
                    "children": []}],
        "activeLayer": "parsel",
        "entities": [{"kind": "text", "id": 1, "layerId": "parsel", "attrs": {}, "p": {"x": 5.0, "y": 5.0},
                      "text": "101", "height": 0.25, "rotation": 0, "annotative": {"at": 100}}],
        "styles": {"items": [], "categories": []}
    });
    let snapshot = kentos_commands::DocumentSnapshotV1::from_json(&drawing.to_string())
        .expect("the drawing reads");
    let doc = Document::new(snapshot, None).expect("opens");
    let mut main = Spatial::new();
    main.reload(&doc.model);
    let square = kentos_geometry_core::entity::Shape::Polygon {
        pts: vec![
            Vec2::new(10.0, 0.0),
            Vec2::new(20.0, 0.0),
            Vec2::new(20.0, 10.0),
            Vec2::new(10.0, 10.0),
        ],
        bulges: None,
        holes: None,
        parts: None,
    };
    main.set_ghosts("parsel", vec![(square, None)]);
    let view: kentos_sheet::display::MapViewPrim =
        serde_json::from_value(serde_json::json!({"scale": 200.0, "rotation": 0})).expect("a view");
    let maps = SheetMaps::default();
    let copy = maps
        .scaled(&doc, &view, None, None, None, &[], None, &main)
        .expect("a copy at 1:200");
    let (_, spatial) = copy.view.as_ref().expect("its drawing");
    assert_eq!(spatial.ghost_total(), 1);
}

/// docs/adr/8039 §2: a map on the screen showing a phase other than the drawing's draws from a copy whose walls
/// are cut as that phase stands; the drawing (Mevcut durum: the window's hole, not the new door's) stays as it is.
#[test]
fn a_map_with_its_own_phase_draws_a_copy_cut_as_it_stands() {
    use kentos_geometry_core::entity::Shape;
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_str(include_str!(
        "../../../../../fixtures/kcad/v2/opening-phases.json"
    ))
    .expect("the sample");
    let doc = Document::from_v2(snapshot, None).expect("opens");
    let main = Spatial::of(&doc.model);
    let holes = |s: &Spatial| -> Vec<(f64, f64)> {
        let x = s
            .store()
            .get(1.0)
            .and_then(|it| it.expanded.clone())
            .expect("the wall's pieces");
        let mut runs: Vec<(f64, f64)> = x
            .shapes
            .iter()
            .filter_map(|p| match p {
                Shape::Polyline { pts, .. }
                    if pts.len() == 2 && pts.iter().all(|q| (q.y - 0.15).abs() < 1e-6) =>
                {
                    Some((pts[0].x.min(pts[1].x), pts[0].x.max(pts[1].x)))
                }
                _ => None,
            })
            .collect();
        runs.sort_by(|a, b| a.0.total_cmp(&b.0));
        let cm = |v: f64| (v * 100.0).round() / 100.0;
        runs.windows(2)
            .filter(|w| w[1].0 - w[0].1 > 1e-6)
            .map(|w| (cm(w[0].1), cm(w[1].0)))
            .collect()
    };
    assert_eq!(holes(&main), vec![(3.4, 4.6), (5.1, 5.7)]);
    let view: kentos_sheet::display::MapViewPrim =
        serde_json::from_value(serde_json::json!({"scale": 50.0, "rotation": 0})).expect("a view");
    let maps = SheetMaps::default();
    let proposed = Some(kentos_commands::PhaseView::Proposed);
    let copy = maps
        .scaled(&doc, &view, None, None, None, &[], proposed, &main)
        .expect("a copy of its own");
    let (_, spatial) = copy.view.as_ref().expect("its drawing");
    assert_eq!(
        holes(spatial),
        vec![(1.05, 1.95), (5.1, 5.7)],
        "Yeni durum: the door cut, the window closed"
    );
    // The drawing's own phase: no copy for it.
    let existing = Some(kentos_commands::PhaseView::Existing);
    assert!(
        maps.scaled(&doc, &view, None, None, None, &[], existing, &main)
            .is_none()
    );
    assert_eq!(holes(&main), vec![(3.4, 4.6), (5.1, 5.7)]);
}

#[test]
fn a_sheet_opens_over_the_drawing_and_the_model_comes_back() {
    let mut app = app_with_drawing();
    assert!(!app.sheets.is_active());
    new_sheet(&mut app);
    assert!(app.sheets.is_active());
    assert!(app.sheet_tab, "the contextual Pafta tab is open");
    // The new sheet's map looks where the drawing area does, and the project's values are the drawing's.
    let ctx = app.sheets.context().clone();
    assert_eq!(
        ctx.center.map(|c| (c.x, c.y)),
        Some((app.viewport.camera.center.x, app.viewport.camera.center.y))
    );
    assert_eq!(ctx.project.name, app.document.as_ref().unwrap().name());
    // The window draws with the mode in it (the stage, the inspector, the tabs, the cells).
    let mut snapshot =
        kentos_ui::snapshot::Snapshot::software(Size::new(1440.0, 900.0)).expect("a renderer");
    let image = snapshot.render(app.view(), &app.theme());
    assert_eq!((image.width, image.height), (1440, 900));
    // Keys no field took go to the sheet mode while a sheet is in front: Ctrl+Z (⌘Z on macOS, the
    // platform's command key the sheet mode reads) takes back the new sheet.
    let _ = app.update(Message::Sheet(Sheet::InspectorTab(InspectorTab::Page)));
    let undo = crate::keys::KeyPress {
        key: iced::keyboard::Key::Character("z".into()),
        physical: iced::keyboard::key::Physical::Code(iced::keyboard::key::Code::KeyZ),
        modifiers: iced::keyboard::Modifiers::COMMAND,
        text: None,
        repeat: false,
    };
    // A letter the sheet mode does not take neither types into the hidden command line nor starts a drawing tool.
    let letter = crate::keys::KeyPress {
        key: iced::keyboard::Key::Character("c".into()),
        physical: iced::keyboard::key::Physical::Code(iced::keyboard::key::Code::KeyC),
        modifiers: iced::keyboard::Modifiers::empty(),
        text: Some("c".into()),
        repeat: false,
    };
    let _ = app.update(Message::Key(letter));
    assert!(app.command_input.is_empty());
    assert_eq!(
        app.session.tool_id(),
        "select",
        "no drawing tool started behind the sheet"
    );
    let _ = app.update(Message::Key(undo));
    assert!(
        app.sheets.book().sheets.is_empty(),
        "the undo of the sheet mode, not the drawing's"
    );
    assert!(!app.sheets.is_active());
    // The model tab.
    new_sheet(&mut app);
    let _ = app.update(Message::Sheet(Sheet::Tab(0)));
    assert!(!app.sheets.is_active());
    assert!(!app.sheet_tab);
}

/// MODEL (`sheet.model`) returns from a sheet to the drawing, as the web's command does.
#[test]
fn the_model_command_returns_from_a_sheet() {
    let mut app = app_with_drawing();
    let _ = app.run("sheet.model");
    assert_eq!(crate::files_testing::last_said(&app), "Model zaten önde.");
    new_sheet(&mut app);
    assert!(app.sheets.is_active());
    let _ = app.run("sheet.model");
    assert!(!app.sheets.is_active());
}

/// Dışa aktar → PDF olarak…: the window asks which sheets and how, Kaydet asks where
/// (the file window), and the file is the core's PDF with the drawing's faces, its map as
/// the drawing's vectors in layers and its text.
#[test]
fn a_sheet_s_pdf_is_written_with_the_drawing_s_vectors_and_faces() {
    use kentos_sheet_ui::message::{PdfMessage, PdfScope};
    let mut app = app_with_drawing();
    // The sheet's map looks at the drawing (a parcel and a building).
    app.viewport.camera.center = Vec2::new(486_530.0, 4_420_200.0);
    new_sheet(&mut app);
    let _ = app.update(Message::Sheet(Sheet::Export(
        kentos_sheet_ui::ExportKind::Pdf,
    )));
    let _ = app.update(Message::Sheet(Sheet::Pdf(PdfMessage::Scope(PdfScope::All))));
    // Kaydet: the host's file window is asked for a .pdf.
    let effects = app.sheets.update(Sheet::Pdf(PdfMessage::Save));
    assert!(
        matches!(effects.as_slice(), [kentos_sheet_ui::Effect::AskExportPath { kind: kentos_sheet_ui::ExportKind::Pdf, suggested }] if suggested.ends_with(".pdf")),
        "{effects:?}"
    );
    let file = crate::files_testing::scratch("pafta-pdf").join("pafta.pdf");
    let _ = std::fs::create_dir_all(file.parent().unwrap());
    app.sheet_export_to(kentos_sheet_ui::ExportKind::Pdf, Some(file.clone()));
    let pdf = std::fs::read(&file).expect("the PDF is written");
    let text = String::from_utf8_lossy(&pdf);
    assert!(
        text.starts_with("%PDF-1.7"),
        "{}",
        &text[..16.min(text.len())]
    );
    assert!(text.contains("/FontFile2") && text.contains("/ToUnicode"));
    assert!(
        text.contains("/OCProperties"),
        "the drawing's layers as the viewer's"
    );
    // The map's layers are the drawing's shown ones (Çizim is hidden).
    assert!(
        text.contains("/Name (Parsel)") && text.contains("/Name (Bina)"),
        "the layers"
    );
    assert!(
        !text.contains("/Name <FEFF00C7"),
        "a hidden layer is not written"
    );
    assert!(app.log.lines().any(|l| l.text.contains("yazıldı")), "said");
}

/// A layer with no vector form here (a pattern fill, the web's own case): the PDF window
/// names it and why, and its map goes to the PDF as a picture at the window's resolution.
#[test]
fn a_map_with_a_pattern_fill_goes_as_a_picture_and_the_window_says_why() {
    use kentos_sheet_ui::message::PdfMessage;
    let mut app = app_with_drawing();
    app.viewport.camera.center = Vec2::new(486_530.0, 4_420_200.0);
    // Bina's areas filled with a pattern of dots.
    let doc = app.document.as_mut().unwrap();
    let mut style = doc
        .model
        .layers()
        .get("bina")
        .map(|l| l.style.clone())
        .expect("the Bina layer");
    style.renderer = Some(
        serde_json::json!({ "type": "single", "symbols": { "fill": { "type": "fill", "layers": [{ "id": "p", "type": "patternFill", "marker": { "type": "marker", "layers": [{ "id": "m", "type": "shape", "shape": "circle", "size": 0.8, "fill": "#3366cc", "stroke": null }] }, "spacingX": 3, "spacingY": 3 }] } } }),
    );
    assert!(doc.model.set_layer_style("bina", style, "Desenli yapılar"));
    // A building's outline on it, as the web's demo has them.
    let building: kentos_commands::Entity = serde_json::from_value(serde_json::json!({
        "kind": "polygon", "layerId": "bina", "attrs": {}, "id": 0,
        "pts": [{ "x": 486530.0, "y": 4420200.0 }, { "x": 486540.0, "y": 4420200.0 },
                { "x": 486540.0, "y": 4420210.0 }, { "x": 486530.0, "y": 4420210.0 }]
    }))
    .expect("a polygon");
    doc.model.add(building).expect("added");
    app.spatial.reload(&app.document.as_ref().unwrap().model);
    new_sheet(&mut app);
    let ways = app.sheets.pdf_ways(&app.sheet_painter());
    // CBS's sheet has the map and its overview (a project not asked its type is CBS, docs/adr/0165).
    assert_eq!(ways.len(), 2, "{ways:?}");
    assert!(
        ways.iter()
            .all(|w| !w.vector && w.why == "Bina (desen dolgusu)"),
        "{ways:?}"
    );
    let _ = app.update(Message::Sheet(Sheet::Export(
        kentos_sheet_ui::ExportKind::Pdf,
    )));
    let file = crate::files_testing::scratch("pafta-pdf-desen").join("desen.pdf");
    let _ = std::fs::create_dir_all(file.parent().unwrap());
    app.sheet_export_to(kentos_sheet_ui::ExportKind::Pdf, Some(file.clone()));
    let pdf = std::fs::read(&file).expect("written");
    let text = String::from_utf8_lossy(&pdf);
    assert!(text.contains("/Subtype /Image"), "the map as a picture");
    assert!(
        text.contains("/SMask"),
        "a clear picture over the paper, not the theme's background"
    );
    assert!(!text.contains("/OCProperties"), "a picture has no layers");
    let _ = app.update(Message::Sheet(Sheet::Pdf(PdfMessage::Save)));
    // The picture is the styled drawing (the styled shaders' CPU twin), not the plain scene:
    // the building's dots in the pattern's blue, the paper between them.
    let m = app
        .sheets
        .display_list()
        .and_then(|l| {
            l.prims.iter().find_map(|p| match p {
                kentos_sheet::display::Prim::Map(m) => Some(m.clone()),
                _ => None,
            })
        })
        .expect("the sheet's map");
    struct MapShot<'a> {
        painter: SheetPainter<'a>,
        m: &'a kentos_sheet::display::MapPrim,
        k: f64,
    }
    impl canvas::Program<Message> for MapShot<'_> {
        type State = ();
        fn draw(
            &self,
            _: &(),
            renderer: &iced::Renderer,
            _: &iced::Theme,
            bounds: iced::Rectangle,
            _: iced::mouse::Cursor,
        ) -> Vec<canvas::Geometry> {
            let mut f = Frame::new(renderer, bounds.size());
            f.fill_rectangle(iced::Point::ORIGIN, bounds.size(), iced::Color::WHITE);
            let r = MapRequest::of(self.m, bounds.size(), self.k, true);
            assert!(self.painter.paint(&r, &mut f), "the map is painted");
            vec![f.into_geometry()]
        }
    }
    // Four pixels a paper millimetre.
    let k = 4.0 / 1000.0;
    let size = Size::new(
        (f64::from(m.clip.width) * k) as f32,
        (f64::from(m.clip.height) * k) as f32,
    );
    let shot = MapShot {
        painter: app.sheet_painter(),
        m: &m,
        k,
    };
    let view: iced::Element<'_, Message> = canvas::Canvas::new(shot)
        .width(iced::Fill)
        .height(iced::Fill)
        .into();
    let image = kentos_ui::snapshot::Snapshot::software(size)
        .expect("a renderer")
        .render(view, &iced::Theme::Light);
    // The building is north-east of the map's middle: 10 m on the ground, 10 mm on the paper.
    let request = MapRequest::of(&m, size, k, true);
    let (w, h) = (image.width as usize, image.height as usize);
    let (mut blue, mut paper) = (0, 0);
    // Inside it, a metre from its outline, every quarter metre.
    for gy in 0..32 {
        for gx in 0..32 {
            let p = request.to_px(
                486_531.0 + f64::from(gx) * 0.25,
                4_420_201.0 + f64::from(gy) * 0.25,
            );
            let (x, y) = (p.x as usize, p.y as usize);
            if x >= w || y >= h {
                continue;
            }
            let i = (y * w + x) * 4;
            let c = [image.rgba[i], image.rgba[i + 1], image.rgba[i + 2]];
            if c[2] > 150 && c[0] < 110 && c[1] < 150 {
                blue += 1;
            } else if c.iter().all(|v| *v > 225) {
                paper += 1;
            }
        }
    }
    // Dots of 0.8 mm every 3 mm ink about a twentieth of the area.
    assert!(
        blue >= 20 && paper >= 500,
        "dots and paper: {blue} blue, {paper} paper"
    );
}

/// An SVG picture (design §9a; the core draws no SVG): drawn here as a PNG at the export
/// window's resolution, embedded in its place, and the log says so.
#[test]
fn an_svg_picture_goes_into_the_pdf_as_a_picture_and_the_log_says_so() {
    use kentos_sheet::kinds::{ItemKind, PictureFit, PictureItem};
    use kentos_sheet::ops::Op;
    let mut app = app_with_drawing();
    new_sheet(&mut app);
    let sheet = app
        .sheets
        .open_sheet()
        .map(|s| s.id.clone())
        .expect("a sheet");
    let item = kentos_sheet::model::Item::new(
        "logo",
        "Kurum logosu",
        kentos_sheet::units::RectUm::new(20_000, 20_000, 20_000, 20_000),
        ItemKind::Picture(PictureItem {
            asset: None,
            fit: PictureFit::Contain,
            clip: true,
        }),
    );
    assert!(app.sheets.apply(
        vec![Op::AddItems(kentos_sheet::ops::AddItems {
            to: kentos_sheet::model::Owner::sheet(&sheet),
            items: vec![item],
            index: None,
        })],
        "Resim çerçevesi",
    ));
    let _ = app.update(Message::Sheet(Sheet::Select(vec!["logo".into()])));
    let _ = app.update(Message::Sheet(Sheet::PictureFile(
        "logo.svg".into(),
        br##"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24"><circle cx="12" cy="12" r="10" fill="#1f4a96"/></svg>"##.to_vec(),
    )));
    let _ = app.update(Message::Sheet(Sheet::Export(
        kentos_sheet_ui::ExportKind::Pdf,
    )));
    let file = crate::files_testing::scratch("pafta-pdf-svg").join("svg.pdf");
    let _ = std::fs::create_dir_all(file.parent().unwrap());
    app.sheet_export_to(kentos_sheet_ui::ExportKind::Pdf, Some(file.clone()));
    let pdf = std::fs::read(&file).expect("written");
    let text = String::from_utf8_lossy(&pdf);
    // 20 mm at the window's 150 dpi: 119 pixels, with its transparency as a soft mask.
    assert!(
        text.contains("/Width 119") && text.contains("/SMask"),
        "the SVG as a picture"
    );
    let said: Vec<String> = app.log.lines().map(|l| l.text.clone()).collect();
    assert!(
        said.iter().any(|l| l.ends_with(
            "“Kurum logosu” (Resim): SVG resim PDF'e resim olarak gömülür (119 × 119 piksel)."
        )),
        "{said:?}"
    );
}

/// Yazdır: the PDF goes to a file of its own and is opened in the system's viewer.
#[test]
fn printing_opens_the_pdf_in_the_system_s_viewer() {
    use kentos_sheet_ui::message::PdfMessage;
    let mut app = app_with_drawing();
    new_sheet(&mut app);
    let _ = app.update(Message::Sheet(Sheet::Print));
    let _ = app.update(Message::Sheet(Sheet::Pdf(PdfMessage::Print)));
    let opened = crate::sheet_pdf::OPENED.with(|o| o.borrow().clone());
    let path = opened.last().expect("a file opened in the viewer");
    assert_eq!(path.extension().and_then(|e| e.to_str()), Some("pdf"));
    assert!(std::fs::read(path).expect("written").starts_with(b"%PDF-"));
    assert!(
        app.log
            .lines()
            .any(|l| l.text.contains("görüntüleyicide açıldı"))
    );
}

/// A preflight fix the desktop does: Koordinat sistemi seç opens the
/// project's settings at its coordinate system, over the sheet.
#[test]
fn the_preflight_s_coordinate_system_fix_opens_the_project_s_settings() {
    let mut app = app_with_drawing();
    new_sheet(&mut app);
    let task = app.sheet_effects(vec![Effect::HostAction("project.crs".into())]);
    crate::files_testing::drive(&mut app, task);
    assert_eq!(app.dialog, Some(crate::app::Dialog::Project));
    assert!(matches!(
        &app.project,
        Some(crate::project::Window::Settings(_))
    ));
    // What the desktop does not have yet is said, not swallowed.
    let _ = app.sheet_effects(vec![Effect::HostAction("sheet.nothing".into())]);
    assert!(crate::files_testing::last_said(&app).contains("sheet.nothing"));
}

/// The map's content keeps inside its box, its view turned or not (a turned
/// frame turns the box, so nothing spills past the frame), and the drawing's
/// text is written in it as the drawing area writes it.
#[test]
fn a_map_keeps_inside_its_frame_and_writes_the_drawing_s_text() {
    let mut app = app_with_drawing();
    app.spatial.reload(&app.document.as_ref().unwrap().model);
    app.sheet_maps.sync(app.document.as_ref());
    let doc = app.document.as_ref().unwrap();
    let b = scene::extents(doc).expect("the drawing has objects");
    let span = (b.max_x - b.min_x).max(b.max_y - b.min_y).max(1.0);
    let picture = |rotation: i32, labels: bool| {
        let view = kentos_sheet::display::MapViewPrim {
            annotation_scale: None,
            center: Some(GroundPoint {
                x: (b.min_x + b.max_x) / 2.0,
                y: (b.min_y + b.max_y) / 2.0,
            }),
            // Large enough for the drawing's labels at the paper's own size (the
            // parcel's name, the dimension's value: five pixels at least).
            scale: 250.0,
            rotation,
        };
        let layers = MapLayers::default();
        let r = MapRequest {
            sheet: None,
            primitive: None,
            overrides: &[],
            item: "m",
            // As large as a map's picture is drawn (no shrinking: the text at its own size).
            size: Size::new(640.0, 480.0),
            view: &view,
            layers: &layers,
            crs: None,
            extent: Some([
                b.min_x - span,
                b.min_y - span,
                b.max_x + span,
                b.max_y + span,
            ]),
            pixels_per_metre: 960.0 / span,
            export: false,
        };
        struct One<'a>(SheetPainter<'a>, MapRequest<'a>, bool);
        impl canvas::Program<()> for One<'_> {
            type State = ();
            fn draw(
                &self,
                _: &(),
                r: &iced::Renderer,
                _: &iced::Theme,
                b: iced::Rectangle,
                _: iced::mouse::Cursor,
            ) -> Vec<canvas::Geometry> {
                let mut f = Frame::new(r, b.size());
                f.fill_rectangle(iced::Point::ORIGIN, b.size(), iced::Color::WHITE);
                if self.2 {
                    assert!(self.0.paint(&self.1, &mut f));
                } else {
                    assert!(self.0.maps.paint_scene(&self.1, &mut f));
                }
                vec![f.into_geometry()]
            }
        }
        let view: iced::Element<'_, ()> = iced::widget::canvas(One(app.sheet_painter(), r, labels))
            .width(iced::Fill)
            .height(iced::Fill)
            .into();
        kentos_ui::snapshot::Snapshot::software(Size::new(800.0, 600.0))
            .expect("a renderer")
            .render(view, &app.theme())
    };
    let ink = |image: &kentos_ui::snapshot::Image| {
        let (mut inside, mut outside) = (0, 0);
        for (i, p) in image.rgba.chunks_exact(4).enumerate() {
            let (x, y) = ((i % 800) as f32, (i / 800) as f32);
            if p[0] < 245 || p[1] < 245 || p[2] < 245 {
                if x <= 641.0 && y <= 481.0 {
                    inside += 1;
                } else {
                    outside += 1;
                }
            }
        }
        (inside, outside)
    };
    for rotation in [0, 30_000] {
        let (scene_only, out) = ink(&picture(rotation, false));
        assert_eq!(out, 0, "turned {rotation}: the scene keeps inside");
        let (with_text, out) = ink(&picture(rotation, true));
        assert_eq!(out, 0, "turned {rotation}: the text keeps inside");
        assert!(
            with_text > scene_only + 30,
            "turned {rotation}: the drawing's text is written ({scene_only} → {with_text})"
        );
    }
}

/// A sheet's map at one scale is laid out the same on the screen and in the PDF whatever size the map is
/// drawn at (docs/adr/0533 §5): the same window on the ground, the same spots, the same texts; the PDF writes
/// what the screen shows.
#[test]
fn a_map_s_labels_are_placed_the_same_on_the_screen_and_in_the_pdf() {
    let mut app = app_with_drawing();
    app.spatial.reload(&app.document.as_ref().unwrap().model);
    app.sheet_maps.sync(app.document.as_ref());
    let doc = app.document.as_ref().unwrap();
    let b = scene::extents(doc).expect("the drawing has objects");
    let span = (b.max_x - b.min_x).max(b.max_y - b.min_y).max(1.0);
    let view = kentos_sheet::display::MapViewPrim {
        annotation_scale: None,
        center: Some(GroundPoint {
            x: (b.min_x + b.max_x) / 2.0,
            y: (b.min_y + b.max_y) / 2.0,
        }),
        scale: 250.0,
        rotation: 0,
    };
    let layers = MapLayers::default();
    let request = |k: f64| MapRequest {
        sheet: None,
        primitive: None,
        overrides: &[],
        item: "m",
        // The same frame, 160 × 120 mm of paper, at k pixels per micrometre (the screen's zoom, the file's).
        size: Size::new((160_000.0 * k) as f32, (120_000.0 * k) as f32),
        view: &view,
        layers: &layers,
        crs: None,
        extent: Some([
            b.min_x - span,
            b.min_y - span,
            b.max_x + span,
            b.max_y + span,
        ]),
        pixels_per_metre: k * 1_000_000.0 / 250.0,
        export: false,
    };
    let painter = app.sheet_painter();
    let lay = |k: f64| {
        let r = request(k);
        let mut keep = None;
        let (drawing, spatial) = painter.source(&r, &mut keep);
        let model = &drawing.expect("a drawing").model;
        let (camera, spots, fence) = painter
            .label_layout(model, spatial, &r)
            .expect("the map has a place");
        let texts = crate::labels::texts_in_map(
            model,
            spots.clone(),
            camera,
            crate::labels::paper_colors(),
            painter.font,
            painter.format,
            fence,
        );
        (camera, spots, texts)
    };
    let (screen, pdf, odd) = (lay(0.0007), lay(0.0035), lay(0.00123456789));
    assert!(!screen.1.is_empty(), "the map has labels");
    for (what, got) in [("the PDF's size", &pdf), ("an odd zoom", &odd)] {
        assert_eq!(screen.0.width, got.0.width, "{what}: the window's width");
        assert_eq!(screen.0.height, got.0.height, "{what}: the window's height");
        assert_eq!(screen.0.scale, got.0.scale, "{what}: the scale");
        assert_eq!(screen.1, got.1, "{what}: the labels' places");
        assert_eq!(screen.2, got.2, "{what}: the texts the PDF writes");
    }
}

#[test]
fn a_map_shows_the_drawing_from_the_drawing_pipeline_s_scene() {
    let mut app = app_with_drawing();
    new_sheet(&mut app);
    let list = app.sheets.display_list().expect("a list").clone();
    let map = list
        .prims
        .iter()
        .find_map(|p| match p {
            kentos_sheet::display::Prim::Map(m) => Some(m.clone()),
            _ => None,
        })
        .expect("the sheet has a map");
    // Fit the map on the drawing: its centre and a scale that shows it all.
    let doc = app.document.as_ref().unwrap();
    let b = scene::extents(doc).expect("the drawing has objects");
    let view = kentos_sheet::display::MapViewPrim {
        annotation_scale: None,
        center: Some(GroundPoint {
            x: (b.min_x + b.max_x) / 2.0,
            y: (b.min_y + b.max_y) / 2.0,
        }),
        ..map.view.clone()
    };
    let span = (b.max_x - b.min_x).max(b.max_y - b.min_y).max(1.0);
    let size = Size::new(400.0, 300.0);
    let k = 300.0 / (span * 1.2) / 1_000_000.0 * map.view.scale;
    let extent = Some([
        b.min_x - span,
        b.min_y - span,
        b.max_x + span,
        b.max_y + span,
    ]);
    let prim = kentos_sheet::display::MapPrim {
        view,
        extent,
        ..map
    };
    let request = MapRequest::of(&prim, size, k, false);
    app.sheet_maps.sync(app.document.as_ref());
    struct One<'a>(SheetPainter<'a>, MapRequest<'a>);
    impl canvas::Program<()> for One<'_> {
        type State = ();
        fn draw(
            &self,
            _: &(),
            r: &iced::Renderer,
            _: &iced::Theme,
            b: iced::Rectangle,
            _: iced::mouse::Cursor,
        ) -> Vec<canvas::Geometry> {
            let mut f = Frame::new(r, b.size());
            f.fill_rectangle(iced::Point::ORIGIN, b.size(), iced::Color::WHITE);
            assert!(self.0.paint(&self.1, &mut f));
            vec![f.into_geometry()]
        }
    }
    let view: iced::Element<'_, ()> = iced::widget::canvas(One(app.sheet_painter(), request))
        .width(iced::Fill)
        .height(iced::Fill)
        .into();
    let image = kentos_ui::snapshot::Snapshot::software(size)
        .expect("a renderer")
        .render(view, &app.theme());
    let inked = image
        .rgba
        .chunks_exact(4)
        .filter(|p| p[0] < 200 || p[1] < 200 || p[2] < 200)
        .count();
    assert!(
        inked > 50,
        "the drawing's lines are on the map: {inked} pixels"
    );
    // A changed drawing is a new revision: the stage paints its maps again.
    let before = app.sheet_maps.revision();
    app.sheet_maps.sync(app.document.as_ref());
    assert_eq!(
        app.sheet_maps.revision(),
        before,
        "nothing changed, nothing built"
    );
}

/// docs/adr/0802: a module's program gives the sheet mode its own catalog types. KentOS Mimari's new sheet is the
/// architectural sheet and its gallery has no survey or zoning sheet; KentOS Harita's is a map sheet (not the ISO
/// technical one) and its gallery has no building or CBS sheet; KentOS GIS's and kentos-cad's are as before.
#[test]
fn each_programs_sheets_start_with_its_own_template_and_gallery() {
    use kentos_launch::Module;
    use kentos_sheet_ui::gallery::GalleryMessage;
    let of = |program: Option<Module>| {
        let mut app = app_with_drawing();
        app.program = program;
        let doc = app.document.as_mut().expect("a drawing");
        let mut s = doc.model.settings().clone();
        s.workspace =
            Some(program.map_or(kentos_commands::Workspace::Cad, crate::program::type_of));
        s.cad_work = program.and_then(|m| crate::program::works_of(m).first().copied());
        doc.model.set_settings(s);
        let ctx = app.sheet_context();
        let mut d = kentos_sheet_ui::Designer::new(ctx.clone());
        let _ = d.update(Sheet::Gallery(GalleryMessage::Open));
        let shown: Vec<String> = d.gallery_cards().into_iter().map(|c| c.id).collect();
        (
            ctx.project_types,
            d.profile().default_template.clone(),
            shown,
        )
    };
    use kentos_commands::ProjectType::*;
    let (types, first, _) = of(None);
    assert!(types.is_empty(), "kentos-cad: every type");
    assert_eq!(
        first, "sys:cad-teknik",
        "kentos-cad's CAD drawing as before"
    );

    let (types, first, shown) = of(Some(Module::Architecture));
    assert_eq!(types, [Architecture]);
    assert_eq!(first, "sys:cad-mimari");
    assert_eq!(
        shown.first().map(String::as_str),
        Some("sys:cad-mimari"),
        "{shown:?}"
    );
    for id in [
        "sys:aplikasyon-krokisi",
        "sys:ifraz-paftasi",
        "sys:imar-plani",
        "sys:gis-tematik",
    ] {
        assert!(!shown.iter().any(|s| s == id), "{id}: {shown:?}");
    }
    assert!(
        shown.iter().any(|s| s == "sys:genel-a3-yatay"),
        "the common ones: {shown:?}"
    );

    let (types, first, shown) = of(Some(Module::Map));
    assert_eq!(types, [Cad, LandReadjustment, ZoningPlan, Subdivision]);
    assert_eq!(first, "sys:genel-a3-yatay", "a map sheet");
    for id in [
        "sys:imar-plani",
        "sys:ifraz-paftasi",
        "sys:aplikasyon-krokisi",
        "sys:genel-a3-yatay",
    ] {
        assert!(shown.iter().any(|s| s == id), "{id}: {shown:?}");
    }
    for id in ["sys:cad-mimari", "sys:gis-tematik", "sys:gis-atlas"] {
        assert!(!shown.iter().any(|s| s == id), "{id}: {shown:?}");
    }

    // KentOS Yol's sheets are a road's (plan/road.md); its own templates come with its plan, profile and
    // section sheets (Y10).
    let (types, _, _) = of(Some(Module::Road));
    assert_eq!(types, [Road]);

    let (types, first, shown) = of(Some(Module::Gis));
    assert_eq!(types, [Gis]);
    assert_eq!(first, "sys:gis-tematik", "unchanged");
    assert_eq!(&shown[..2], ["sys:gis-tematik", "sys:gis-atlas"]);
    assert!(
        !shown
            .iter()
            .any(|s| s == "sys:imar-plani" || s == "sys:cad-teknik"),
        "{shown:?}"
    );
}

mod screens;

/// UR-011: a new sheet's map looks where the drawing area does; zoomed in on one parcel, it leaves the rest of the
/// drawing out, and the preflight says so with a fix that fits the whole drawing; zoomed out to all of it, nothing
/// is said.
#[test]
fn a_map_that_leaves_part_of_the_drawing_out_is_said_and_fitted() {
    let cut = |app: &App| app.sheets.findings().iter().position(|f| f.code == "drawing_cut");
    // The drawing area on the whole drawing (about 50 × 70 m: 1:1000 holds it on A3).
    let mut app = app_with_drawing();
    let b = kentos_native_application::sheets_fit::object_extent(&app.document.as_ref().expect("open").model, None)
        .ok()
        .flatten()
        .expect("the drawing's extent");
    app.viewport.camera.set_size(1200.0, 800.0);
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds { min_x: b[0], min_y: b[1], max_x: b[2], max_y: b[3] },
        24.0,
    );
    new_sheet(&mut app);
    assert_eq!(cut(&app), None, "the whole drawing in view: {:?}", app.sheets.findings());

    // At 1:100, zoomed in on a corner of it: the map at the project's scale leaves the rest out.
    let mut app = app_with_drawing();
    let doc = app.document.as_mut().expect("open");
    let mut settings = doc.settings().clone();
    settings.plot_scale = 100.0;
    doc.model.set_settings(settings);
    let b = kentos_native_application::sheets_fit::object_extent(&app.document.as_ref().expect("open").model, None)
        .ok()
        .flatten()
        .expect("the drawing's extent");
    let c = kentos_render_wgpu::Vec2::new(b[0], b[1]);
    app.viewport.camera.set_size(1200.0, 800.0);
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds { min_x: c.x - 2.0, min_y: c.y - 2.0, max_x: c.x + 2.0, max_y: c.y + 2.0 },
        0.0,
    );
    new_sheet(&mut app);
    let i = cut(&app).expect("the cut said");
    let finding = &app.sheets.findings()[i];
    assert_eq!(finding.severity, kentos_sheet::preflight::Severity::Warning, "a detail view may be meant");
    assert!(finding.message.contains("çizimin tamamını göstermiyor"), "{}", finding.message);
    assert_eq!(finding.fixes.first().map(|f| f.label.as_str()), Some("Çizimin tamamını sığdır"));
    let _ = app.update(Message::Sheet(Sheet::Fix(i, 0)));
    assert_eq!(cut(&app), None, "fitted: {:?}", app.sheets.findings());
}

/// UR-011's pictures (`.run/gui-evidence/kullanim-UR-011/`): a 1:100 sheet zoomed in on a corner, Ön denetim with
/// the cut said and its fix, light and dark.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn drawing_cut_screens() {
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/kullanim-UR-011");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, name) in [("light", "acik"), ("dark", "koyu")] {
        let mut app = app_with_drawing();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let doc = app.document.as_mut().expect("open");
        let mut settings = doc.settings().clone();
        settings.plot_scale = 100.0;
        doc.model.set_settings(settings);
        let b = kentos_native_application::sheets_fit::object_extent(&app.document.as_ref().expect("open").model, None)
            .ok()
            .flatten()
            .expect("the drawing's extent");
        app.viewport.camera.set_size(1200.0, 800.0);
        app.viewport.camera.fit(
            &kentos_render_wgpu::Bounds { min_x: b[0] - 2.0, min_y: b[1] - 2.0, max_x: b[0] + 12.0, max_y: b[1] + 12.0 },
            0.0,
        );
        new_sheet(&mut app);
        let _ = app.update(Message::Sheet(Sheet::InspectorTab(InspectorTab::Preflight)));
        let mut snapshot = kentos_ui::snapshot::Snapshot::new(Size::new(1440.0, 900.0)).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("pafta-kesik-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}

#[test]
fn shift_held_in_the_window_reaches_the_sheet_item_list() {
    let mut app = app_with_drawing();
    new_sheet(&mut app);
    let ids: Vec<_> = app.sheets.open_sheet().expect("a sheet").items.iter().take(2).map(|i| i.id.clone()).collect();
    let _ = app.update(Message::Sheet(Sheet::TreeClick(ids[0].clone(), false)));
    let _ = app.update(Message::Modifiers(iced::keyboard::Modifiers::SHIFT));
    let _ = app.update(Message::Sheet(Sheet::TreeClick(ids[1].clone(), false)));
    let _ = app.update(Message::Modifiers(iced::keyboard::Modifiers::empty()));
    assert_eq!(app.sheets.selection(), ids.as_slice());
}

#[test]
#[ignore = "GPU: the item list with two rows chosen by Shift"]
fn item_list_shift_screens() {
    crate::drawing_fonts::load();
    kentos_ui::theme::motion::set_reduced(true);
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/kullanim-UR-042");
    std::fs::create_dir_all(&out).expect("folder");
    let mut app = app_with_drawing();
    new_sheet(&mut app);
    let ids: Vec<_> = app.sheets.open_sheet().expect("a sheet").items.iter().take(2).map(|i| i.id.clone()).collect();
    let _ = app.update(Message::Sheet(Sheet::TreeClick(ids[0].clone(), false)));
    let _ = app.update(Message::Modifiers(iced::keyboard::Modifiers::SHIFT));
    let _ = app.update(Message::Sheet(Sheet::TreeClick(ids[1].clone(), false)));
    let _ = app.update(Message::Modifiers(iced::keyboard::Modifiers::empty()));
    let mut snapshot = kentos_ui::snapshot::Snapshot::new(Size::new(1440.0, 900.0)).expect("a renderer");
    let mut update = |app: &mut App, m: Message| {
        let _ = app.update(m);
    };
    snapshot.settle(&mut app, App::view, &mut update);
    let file = out.join("oge-listesi-shift.png");
    snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
    let _ = app.update(Message::Sheet(Sheet::Select(Vec::new())));
    snapshot.settle(&mut app, App::view, &mut update);
    snapshot.render(app.view(), &app.theme()).save(out.join("bos-secim-aciklamasi.png")).expect("writes");
}

#[test]
#[ignore = "GPU: two sheets of one template, numbered, in the tabs, the list and the publish window"]
fn same_template_sheets_screens() {
    use kentos_sheet_ui::drawing_set::{Field as SetField, SetMessage};
    crate::drawing_fonts::load();
    kentos_ui::theme::motion::set_reduced(true);
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/kullanim-UR-048");
    std::fs::create_dir_all(&out).expect("folder");
    let mut app = app_with_drawing();
    new_sheet(&mut app);
    new_sheet(&mut app);
    for m in [
        SetMessage::Open,
        SetMessage::Select(0),
        SetMessage::Edit(SetField::Number, "QA-001".into()),
        SetMessage::Select(1),
        SetMessage::Edit(SetField::Number, "QA-002".into()),
        SetMessage::Save,
    ] {
        let _ = app.update(Message::Sheet(Sheet::DrawingSet(m)));
    }
    let names: Vec<String> = app.sheets.book().sheets.iter().map(|s| s.listed_name().into_owned()).collect();
    assert_eq!(names.len(), 2);
    assert!(names[0].starts_with("QA-001 · ") && names[1].starts_with("QA-002 · "), "{names:?}");
    let mut snapshot = kentos_ui::snapshot::Snapshot::new(Size::new(1440.0, 900.0)).expect("a renderer");
    let mut update = |app: &mut App, m: Message| {
        let _ = app.update(m);
    };
    snapshot.settle(&mut app, App::view, &mut update);
    snapshot.render(app.view(), &app.theme()).save(out.join("paftalar.png")).expect("writes");
    let _ = app.sheet_publish_open();
    snapshot.settle(&mut app, App::view, &mut update);
    snapshot.render(app.view(), &app.theme()).save(out.join("toplu-yayin.png")).expect("writes");
}
