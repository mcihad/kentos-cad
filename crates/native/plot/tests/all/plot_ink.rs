use kentos_sheet::{
    *,
    ops::{self, Op},
    plot_profile::{PlotProfile, SavePlotProfiles},
    print_setup::{SavePageSetups, ApplyPageSetup},
    project::project_sheets,
};
fn book(c: &serde_json::Value) -> SheetBook {
    let mut b = SheetBook::default();
    b.sheets
        .push(serde_json::from_value(c["sheet"].clone()).unwrap());
    ops::apply_all(
        &b,
        &[
            Op::SavePlotProfiles(SavePlotProfiles {
                profiles: vec![serde_json::from_value(c["profile"].clone()).unwrap()],
            }),
            Op::SavePageSetups(SavePageSetups {
                setups: vec![serde_json::from_value(c["setup"].clone()).unwrap()],
            }),
            Op::ApplyPageSetup(ApplyPageSetup {
                sheet: "page".into(),
                setup: "office".into(),
            }),
        ],
    )
    .unwrap()
    .book
}
fn cases() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../../../fixtures/sheet/v1/plot-ink.json"
    ))
    .unwrap()
}
fn rgba(hex: &str) -> [u8; 4] {
    let mut a = [255; 4];
    for (i, v) in a.iter_mut().enumerate().take((hex.len() - 1) / 2) {
        *v = u8::from_str_radix(&hex[1 + i * 2..3 + i * 2], 16).unwrap();
    }
    a
}
#[test]
fn twenty_pdfs_have_independent_pen_width_dash_cap_join_rgb_and_physical_model_length() {
    let folder = std::env::temp_dir().join(format!("kentos-sheet20-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    for (n, c) in cases()["cases"].as_array().unwrap().iter().enumerate() {
        let b = book(c);
        let mut doc = kentos_domain::Document::from_snapshot(
            serde_json::from_value(c["model"].clone()).unwrap(),
        )
        .unwrap();
        doc.set_sheets(project_sheets(&b, |_| None).unwrap())
            .unwrap();
        let before = doc.to_snapshot_v2();
        let pdf = kentos_plot::book::book_pdf(&doc, &[], "Test", "2026-10-23").unwrap();
        let pages = super::files::pdf_pages(&pdf.bytes);
        let expected = &c["expected"];
        let pen = pages[0]
            .ink
            .iter()
            .find(|p| {
                p.segment
                    .iter()
                    .zip([80., 72., 180., 72.])
                    .all(|(a, b)| (a - b).abs() < 0.002)
            })
            .unwrap_or_else(|| panic!("{} {:?}", c["name"], pages[0].segments));
        assert!(
            (pen.width - expected["widthMm"].as_f64().unwrap()).abs() < 0.0002,
            "{} {}",
            c["name"],
            pen.width
        );
        let dash: Vec<f64> = serde_json::from_value(expected["dashMm"].clone()).unwrap();
        assert_eq!(pen.dash.len(), dash.len());
        assert!(
            pen.dash
                .iter()
                .zip(dash)
                .all(|(a, b)| (a - b).abs() < 0.0002)
        );
        assert_eq!(
            pen.cap,
            match expected["cap"].as_str().unwrap() {
                "butt" => 0,
                "round" => 1,
                _ => 2,
            }
        );
        assert_eq!(
            pen.join,
            match expected["join"].as_str().unwrap() {
                "miter" => 0,
                "round" => 1,
                _ => 2,
            }
        );
        let want = rgba(expected["modelColor"].as_str().unwrap());
        for (value, want) in pen.color.iter().zip(want) {
            assert!(
                (value - f64::from(want) / 255.).abs() < 0.002,
                "{} got {:?} want {:?}",
                c["name"],
                pen.color,
                expected["modelColor"]
            );
        }
        let path = folder.join(format!("{n}.pdf"));
        std::fs::write(&path, pdf.bytes).unwrap();
        let text = std::process::Command::new("pdftotext")
            .arg(&path)
            .arg("-")
            .output()
            .unwrap();
        assert!(text.status.success());
        assert!(String::from_utf8(text.stdout).unwrap().contains("PRINT"));
        // Poppler independently composites the ink onto white paper, including the exact rounded alpha byte.
        let prefix = folder.join(format!("{n}"));
        let run = std::process::Command::new("pdftoppm")
            .args(["-r", "96", "-singlefile", "-png"])
            .arg(&path)
            .arg(&prefix)
            .output()
            .unwrap();
        assert!(run.status.success());
        let reader = png::Decoder::new(std::io::Cursor::new(
            std::fs::read(prefix.with_extension("png")).unwrap(),
        ))
        .read_info()
        .unwrap();
        let mut reader = reader;
        let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
        let info = reader.next_frame(&mut pixels).unwrap();
        let stride = info.color_type.samples();
        let x = (100. * 96. / 25.4_f64).floor() as usize;
        let y = (80. * 96. / 25.4_f64).floor() as usize;
        let got = &pixels[(y * info.width as usize + x) * stride..][..3];
        let want = rgba(expected["boxColor"].as_str().unwrap());
        for (i, v) in got.iter().enumerate() {
            let expected = (f64::from(want[i]) * f64::from(want[3]) / 255. + 255.
                - f64::from(want[3]))
            .round();
            assert!(
                (f64::from(*v) - expected).abs() <= 2.,
                "{} {got:?} expected {want:?}",
                c["name"]
            );
        }
        assert_eq!(doc.to_snapshot_v2(), before);
        std::fs::remove_file(prefix.with_extension("png")).unwrap();
    }
    std::fs::remove_dir_all(folder).unwrap();
}
#[test]
fn the_cpu_print_picture_has_the_same_stroke_ink_and_keeps_large_coordinate_cameras() {
    for c in cases()["cases"].as_array().unwrap() {
        let b = book(c);
        let doc = kentos_domain::Document::from_snapshot(
            serde_json::from_value(c["model"].clone()).unwrap(),
        )
        .unwrap();
        let mut spatial = kentos_interaction::spatial::Spatial::new();
        spatial.reload(&doc);
        let before = doc.to_snapshot_v2();
        let inputs = kentos_sheet::display::RenderInputs::default();
        let list = display::display_list(&b, "page", &inputs).unwrap();
        let display::Prim::Map(m) = list
            .prims
            .iter()
            .find(|p| matches!(p, display::Prim::Map(_)))
            .unwrap()
        else {
            panic!()
        };
        let library = kentos_plot::book::library_of(&doc);
        let source = kentos_plot::maps::MapSource {
            map: None,
            doc: &doc,
            store: spatial.store(),
            library: &library,
            origin: kentos_geometry_core::Vec2::new(
                m.view.center.unwrap().x,
                m.view.center.unwrap().y,
            ),
        };
        let profile: PlotProfile = serde_json::from_value(c["profile"].clone()).unwrap();
        let image = kentos_plot::render_map_image_profile(
            &source,
            m,
            &[],
            [4000, 2000],
            None,
            Some(&profile),
        )
        .unwrap();
        let want = rgba(c["expected"]["modelColor"].as_str().unwrap());
        let mut matched = 0;
        for p in image.rgba.chunks_exact(4) {
            if p[3] > want[3] / 2 && p[..3].iter().zip(want).all(|(a, b)| a.abs_diff(b) <= 2) {
                matched += 1;
            }
        }
        let column = 2040;
        let alpha_sum = (1470..1531)
            .map(|y| f64::from(image.rgba[(y * 4000 + column) * 4 + 3]))
            .sum::<f64>()
            / f64::from(want[3]);
        if !c["expected"]["dashMm"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v.as_f64() == Some(0.))
        {
            assert!(
                (alpha_sum - c["expected"]["widthMm"].as_f64().unwrap() * 20.).abs() < 0.04,
                "{} width-pixels {alpha_sum} clip {:?} alpha {:?}",
                c["name"],
                m.clip,
                (1495..1505)
                    .map(|y| image.rgba[(y * 4000 + column) * 4 + 3])
                    .collect::<Vec<_>>()
            );
        }
        assert!(
            matched > 50,
            "{} matched={matched} want={want:?}",
            c["name"]
        );
        assert_eq!(doc.to_snapshot_v2(), before);
    }
}

#[test]
fn a_shared_master_raster_map_uses_each_pages_own_captured_ink() {
    use serde_json::json;
    let data = cases();
    let c = &data["cases"][0];
    let mut doc =
        kentos_domain::Document::from_snapshot(serde_json::from_value(c["model"].clone()).unwrap())
            .unwrap();
    let mut style = doc.layers().get("cizim").unwrap().style.clone();
    style.renderer = Some(
        json!({"type":"single","symbols":{"fill":{"type":"fill","layers":[{"id":"pattern","type":"patternFill","marker":{"type":"marker","layers":[{"id":"dot","type":"shape","shape":"circle","size":0.8,"fill":"#3366cc","stroke":null}]},"spacingX":3,"spacingY":3}]}}}),
    );
    doc.set_layer_style("cizim", style, "Desen");
    doc.add(serde_json::from_value(json!({"kind":"polygon","id":0,"layerId":"cizim","attrs":{},"pts":[{"x":0,"y":-1},{"x":5,"y":-1},{"x":5,"y":1},{"x":0,"y":1}]})).unwrap()).unwrap();
    let mut b = book(c);
    let map = b.sheets[0].items.pop().unwrap();
    b.sheets[0].master = Some("master".into());
    b.sheets[0].export.dpi = 96;
    b.sheets[0].id = "red".into();
    let mut blue = b.sheets[0].clone();
    blue.id = "blue".into();
    for item in &mut blue.items {
        item.id = format!("blue-{}", item.id);
    }
    b.sheets.push(blue);
    b.masters.push(serde_json::from_value(json!({"id":"master","name":"Ortak harita","page":b.sheets[0].page,"items":[map],"guides":[]})).unwrap());
    b.plot_profiles.clear();
    for (id, color) in [("red", "#dd2222"), ("blue", "#2244dd")] {
        b.plot_profiles.push(serde_json::from_value(json!({"id":id,"name":id,"kind":"named","entries":[{"key":{"type":"named","name":"Ink"},"style":{"color":{"type":"color","value":color},"screening":100}}],"assignments":[],"defaultStyle":"Ink"})).unwrap());
    }
    for n in 0..2 {
        let p = print_setup::AppliedPageSetup::capture(
            &b,
            None,
            print_setup::PlotSettings {
                profile: Some(b.plot_profiles[n].id.clone()),
                ..Default::default()
            },
        );
        b.sheets[n].plot = Some(p);
    }
    doc.set_sheets(project_sheets(&b, |_| None).unwrap())
        .unwrap();
    let before = doc.to_snapshot_v2();
    let pdf = kentos_plot::book::book_pdf(&doc, &[], "Test", "2026-10-23").unwrap();
    assert!(String::from_utf8_lossy(&pdf.bytes).contains("/Subtype /Image"));
    let folder = std::env::temp_dir().join(format!("kentos-sheet20-master-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    let path = folder.join("set.pdf");
    std::fs::write(&path, pdf.bytes).unwrap();
    let run = std::process::Command::new("pdftoppm")
        .args(["-r", "96", "-png"])
        .arg(&path)
        .arg(folder.join("page"))
        .output()
        .unwrap();
    assert!(run.status.success());
    for page in 1..=2 {
        let mut r = png::Decoder::new(std::io::Cursor::new(
            std::fs::read(folder.join(format!("page-{page}.png"))).unwrap(),
        ))
        .read_info()
        .unwrap();
        let mut pixels = vec![0; r.output_buffer_size().unwrap()];
        let info = r.next_frame(&mut pixels).unwrap();
        let stride = info.color_type.samples();
        let mut counts = [0, 0];
        for y in (150. * 96. / 25.4) as usize..(250. * 96. / 25.4) as usize {
            for x in (30. * 96. / 25.4) as usize..(230. * 96. / 25.4) as usize {
                let p = &pixels[(y * info.width as usize + x) * stride..];
                if u16::from(p[0]) > u16::from(p[2]) + 50 {
                    counts[0] += 1;
                }
                if u16::from(p[2]) > u16::from(p[0]) + 50 {
                    counts[1] += 1;
                }
            }
        }
        assert!(counts[page - 1] > 500, "page {page}: {counts:?}");
        assert_eq!(counts[2 - page], 0, "page {page}: {counts:?}");
    }
    assert_eq!(doc.to_snapshot_v2(), before);
    std::fs::remove_dir_all(folder).unwrap();
}

#[test]
fn a_wide_plot_pen_collects_geometry_outside_the_unstyled_view_box() {
    use serde_json::json;
    let data = cases();
    let c = &data["cases"][0];
    let mut doc =
        kentos_domain::Document::from_snapshot(serde_json::from_value(c["model"].clone()).unwrap())
            .unwrap();
    let b = book(c);
    let list = display::display_list(&b, "page", &display::RenderInputs::default()).unwrap();
    let display::Prim::Map(m) = list
        .prims
        .iter()
        .find(|p| matches!(p, display::Prim::Map(_)))
        .unwrap()
    else {
        panic!()
    };
    let mut profile: PlotProfile = serde_json::from_value(c["profile"].clone()).unwrap();
    profile.entries[0].style = plot_profile::PlotStyle {
        color: plot_profile::PlotColor::Black,
        line_weight: Some(100000),
        cap: Some(LineCap::Butt),
        join: Some(LineJoin::Round),
        ..Default::default()
    };
    let draw = |doc: &kentos_domain::Document| {
        let mut spatial = kentos_interaction::Spatial::new();
        spatial.reload(doc);
        let library = kentos_plot::book::library_of(doc);
        let src = kentos_plot::maps::MapSource {
            map: None,
            doc,
            store: spatial.store(),
            library: &library,
            origin: kentos_geometry_core::Vec2::new(2.5, 1.25),
        };
        kentos_plot::render_map_image_profile(&src, m, &[], [2000, 1000], None, Some(&profile))
            .unwrap()
    };
    let before = draw(&doc);
    assert_eq!(before.rgba[(550 * 2000 + 50) * 4 + 3], 0);
    doc.add(serde_json::from_value(json!({"kind":"line","id":0,"layerId":"cizim","attrs":{},"a":{"x":-4,"y":0},"b":{"x":-4,"y":2}})).unwrap()).unwrap();
    let after = draw(&doc);
    assert_eq!(after.rgba[(550 * 2000 + 50) * 4 + 3], 255);
}
