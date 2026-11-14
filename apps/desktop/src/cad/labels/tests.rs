use super::*;

/// docs/adr/0597: while the view pans the labels keep their placing (not asked of the store again); once it settles
/// they are placed for the view where it stopped.
#[test]
fn the_labels_wait_for_the_view_to_settle() {
    let mut app = crate::files_testing::app_with_drawing();
    let _ = app.view();
    let first = app.label_spots.key().expect("placed");
    app.viewport.update(
        crate::viewport::Event::Panned {
            by: iced::Vector::new(40.0, 0.0),
            at: iced::Point::new(300.0, 300.0),
        },
        None,
    );
    assert!(app.viewport.moving());
    let _ = app.view();
    assert_eq!(
        app.label_spots.key(),
        Some(first),
        "placed again while moving"
    );
    app.viewport.update(crate::viewport::Event::Settled, None);
    assert!(!app.viewport.moving());
    let _ = app.view();
    assert_ne!(
        app.label_spots.key(),
        Some(first),
        "not placed once settled"
    );
}

/// A zoom out while the view still moves places the labels for the larger view at once: the last placing served
/// whatever the view outside true sizes, and the edges a zoom out showed stayed without labels until the view stopped
/// (14 November 2026, the owner: "zoom yaparken bazen kayboluyor").
#[test]
fn a_zoom_out_while_moving_places_the_labels_again() {
    let mut app = crate::files_testing::app_with_drawing();
    let _ = app.view();
    let first = app.label_spots.key().expect("placed");
    // A small pan: the last placing serves.
    app.viewport.update(
        crate::viewport::Event::Panned {
            by: iced::Vector::new(10.0, 0.0),
            at: iced::Point::new(300.0, 300.0),
        },
        None,
    );
    let _ = app.view();
    assert_eq!(
        app.label_spots.key(),
        Some(first),
        "a small pan keeps the placing"
    );
    // Twice as far, still moving: placed again for the larger view.
    app.viewport.update(
        crate::viewport::Event::Zoomed {
            factor: 0.5,
            at: iced::Point::new(300.0, 300.0),
        },
        None,
    );
    assert!(app.viewport.moving());
    let _ = app.view();
    assert_ne!(
        app.label_spots.key(),
        Some(first),
        "placed again while moving"
    );
}

/// The desktop's default label styles are the web's `DEFAULT_LABELS`
/// (apps/web/src/model/labelDefaults.ts), read from its source.
#[test]
fn the_default_labels_are_the_webs() {
    let source = include_str!("../../../../web/src/model/labelDefaults.ts");
    let start = source.find("DEFAULT_LABELS").expect("the web's defaults");
    let body = &source[start..];
    let body = &body[body.find('{').expect("an object") + 1..body.find("};").expect("its end")];
    for line in body.lines().map(str::trim).filter(|l| !l.is_empty()) {
        let (kind, style) = line.split_once(':').expect("kind: style");
        // The literal as JSON: quoted keys, double quotes.
        let mut json = String::new();
        for part in style
            .trim()
            .trim_end_matches(',')
            .replace('\'', "\"")
            .split(['{', ',', '}'])
        {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            let (k, v) = part.split_once(':').expect("key: value");
            json.push_str(&format!("\"{}\":{},", k.trim(), v.trim()));
        }
        let json = format!("{{{}}}", json.trim_end_matches(','));
        let web: kentos_commands::LabelStyle = serde_json::from_str(&json).expect("a label style");
        assert_eq!(default_label(kind.trim()), Some(web), "{kind}");
    }
}

/// A sheet's map writes the label engine's placed labels as the drawing area draws them
/// (docs/adr/0212 §4): the contours' masks filled, the sliver's callout a line, each on its
/// object's layer beside the texts; the PDF puts them under the texts (sheet_pdf.rs).
#[test]
fn a_map_takes_the_placed_labels_backgrounds_and_callouts() {
    let snapshot = kentos_commands::DocumentSnapshotV1::from_json(crate::label_scenes::DRAWING)
        .expect("the scene reads");
    let doc = crate::document::Document::new(snapshot, None).expect("opens");
    let mut spatial = Spatial::new();
    spatial.reload(&doc.model);
    // The scene at 1:1000 on the paper: CSS pixels per ground metre.
    let camera = Camera {
        map: None,
        map_ratio: 1.0,
        center: kentos_render_wgpu::Vec2::new(487_140.0, 4_420_085.0),
        scale: 1.0 / crate::sheet_pdf::PX_MM,
        width: 1400.0,
        height: 900.0,
        twist: 0.0,
    };
    let view = camera.visible_bounds();
    let spots = spatial.labels(
        Vec2::new(view.min_x, view.min_y),
        Vec2::new(view.max_x, view.max_y),
        camera.scale,
        Default::default(),
    );
    let fence = Fence {
        from: Point::new(700.0, 450.0),
        size: Size::new(1400.0, 900.0),
        turn: 0.0,
        scale: 1.0,
    };
    let texts = texts_in_map(
        &doc.model,
        spots,
        camera,
        paper_colors(),
        DrawingFont::Barlow,
        Format::default(),
        fence,
    );
    let on = |layer: &str, what: fn(&MapLabel) -> bool| {
        texts.iter().any(|t| t.layer == layer && what(t))
    };
    assert!(
        on("esyukselti", |t| t.fill.is_some()),
        "the contours' masks"
    );
    assert!(
        on("parsel", |t| t
            .stroke
            .as_ref()
            .is_some_and(|(line, closed)| !closed && line.len() >= 2)),
        "the sliver's callout"
    );
    assert!(on("parsel", |t| !t.text.is_empty()), "the parcels' numbers");
    assert!(
        texts.iter().all(|t| !t.layer.is_empty()),
        "every piece on its object's layer"
    );
}

/// The owner's choice (docs/adr/8053 §14): a CAD drawing's texts in their own colours, else their layers'; a
/// block's text in its own, not the insert's; a GIS drawing's every text in the label colour, as before.
#[test]
fn a_cad_drawings_texts_take_their_own_colours() {
    let mut raw: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/kcad/v2/dynamic-blocks.json"
    ))
    .expect("the sample");
    raw["version"] = serde_json::json!(1);
    let text = |id: u32, x: f64, color: Option<&str>| {
        let mut t = serde_json::json!({
            "kind":"text", "id":id, "layerId":"kapilar", "attrs":{}, "p":{"x":x,"y":5.0},
            "text":format!("T{id}"), "height":0.5, "rotation":0.0
        });
        if let Some(c) = color {
            t["color"] = c.into();
        }
        t
    };
    let block = &mut raw["blocks"][0];
    block.as_object_mut().expect("a block").remove("dynamic");
    block["entities"] = serde_json::json!([{
        "kind":"text", "id":1, "layerId":"", "attrs":{}, "color":"#00FF00", "p":{"x":0.0,"y":0.0},
        "text":"P1", "height":0.5, "rotation":0.0
    }]);
    let mut insert = raw["entities"][0].clone();
    insert.as_object_mut().expect("insert").remove("parameters");
    insert["id"] = 3.into();
    raw["entities"] =
        serde_json::json!([text(1, 0.0, Some("#FF0000")), text(2, 4.0, None), insert]);
    let painted = |workspace: &str| {
        let mut raw = raw.clone();
        raw["settings"]["workspace"] = workspace.into();
        let snapshot = kentos_commands::DocumentSnapshotV1::from_json(&raw.to_string())
            .expect("the drawing reads");
        let doc = crate::document::Document::new(snapshot, None).expect("opens");
        let mut spatial = Spatial::new();
        spatial.reload(&doc.model);
        let camera = Camera {
            map: None,
            map_ratio: 1.0,
            center: kentos_render_wgpu::Vec2::new(3.0, 3.0),
            scale: 40.0,
            width: 800.0,
            height: 600.0,
            twist: 0.0,
        };
        let view = camera.visible_bounds();
        let spots = spatial.labels(
            Vec2::new(view.min_x, view.min_y),
            Vec2::new(view.max_x, view.max_y),
            camera.scale,
            Default::default(),
        );
        let mut texts: Vec<(String, String)> =
            texts_on_screen(&doc.model, spots, camera, paper_colors())
                .into_iter()
                .filter(|t| !t.text.is_empty())
                .map(|t| (t.text, t.color.to_lowercase()))
                .collect();
        texts.sort();
        texts
    };
    let label = crate::sheet_pdf::hex_of(paper_colors().label).to_lowercase();
    assert_eq!(
        painted("cad"),
        [
            ("P1".to_owned(), "#00ff00".to_owned()),
            ("T1".to_owned(), "#ff0000".to_owned()),
            ("T2".to_owned(), "#c8a165".to_owned()),
        ]
    );
    assert_eq!(
        painted("gis"),
        [
            ("P1".to_owned(), label.clone()),
            ("T1".to_owned(), label.clone()),
            ("T2".to_owned(), label),
        ]
    );
}

#[test]
fn the_sample_drawing_shows_its_texts_labels_and_dimensions() {
    let app = crate::files_testing::app_with_drawing();
    let doc = &app.document.as_ref().expect("open").model;
    let mut spatial = Spatial::new();
    spatial.reload(doc);
    let extent = spatial.extent().expect("objects");
    let spots = spatial.labels(
        Vec2::new(extent.min_x, extent.min_y),
        Vec2::new(extent.max_x, extent.max_y),
        4.0,
        Default::default(),
    );
    assert!(!spots.is_empty());
    for slot in slots(&spots) {
        assert!(doc.get(slot).is_some());
    }
}
