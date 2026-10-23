//! Independent PDF operators, SFNT names and Poppler: fonts, analytic paper curves and model curves.
use kentos_sheet::{SheetBook, project::project_sheets};
use serde_json::Value;
#[test]
fn twenty_two_real_vector_pages_embed_the_requested_faces_and_keep_curves_and_text() {
    let data: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/sheet/v1/plot-document.json"
    ))
    .unwrap();
    let cases = data["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 22);
    let mut book = SheetBook::default();
    for c in cases {
        book.sheets
            .push(serde_json::from_value(c["sheet"].clone()).unwrap());
    }
    let mut doc = kentos_domain::Document::from_snapshot(
        serde_json::from_value(data["model"].clone()).unwrap(),
    )
    .unwrap();
    doc.set_sheets(project_sheets(&book, |_| None).unwrap())
        .unwrap();
    let before = doc.to_snapshot_v2();
    let output = kentos_plot::book::book_pdf(&doc, &[], "Test", "2026-10-23").unwrap();
    assert!(output.notes.is_empty(), "{:?}", output.notes);
    let pages = super::files::pdf_pages(&output.bytes);
    assert_eq!(pages.len(), 22);
    for (c, page) in cases.iter().zip(pages) {
        assert!((page.width - 420.).abs() < 0.002 && (page.height - 297.).abs() < 0.002);
        let paper = &c["expected"]["paperCircle"];
        let center: [f64; 2] = serde_json::from_value(paper["center"].clone()).unwrap();
        let radius = paper["radius"].as_f64().unwrap();
        let curves: Vec<_> = page
            .cubics
            .iter()
            .filter(|p| (p.color[0] - 0.8).abs() < 0.002 && (p.color[1] - 0.2).abs() < 0.002)
            .collect();
        assert!(curves.len() >= 8, "{} {:?}", c["name"], page.cubics);
        for curve in curves {
            for step in 0..=100 {
                let t = f64::from(step) / 100.;
                let u = 1. - t;
                let p = curve.points;
                let point = [
                    u * u * u * p[0]
                        + 3. * u * u * t * p[2]
                        + 3. * u * t * t * p[4]
                        + t * t * t * p[6],
                    u * u * u * p[1]
                        + 3. * u * u * t * p[3]
                        + 3. * u * t * t * p[5]
                        + t * t * t * p[7],
                ];
                assert!(
                    ((point[0] - center[0]).hypot(point[1] - center[1]) - radius).abs() < 0.002,
                    "{} {point:?}",
                    c["name"]
                );
            }
        }
        let ellipse: Vec<_> = page
            .cubics
            .iter()
            .filter(|p| {
                (p.color[0] - 221. / 255.).abs() < 0.002 && (p.color[1] - 0.4).abs() < 0.002
            })
            .collect();
        assert!(!ellipse.is_empty());
        for curve in ellipse {
            for step in 0..=100 {
                let t = f64::from(step) / 100.;
                let u = 1. - t;
                let p = curve.points;
                let x = u * u * u * p[0]
                    + 3. * u * u * t * p[2]
                    + 3. * u * t * t * p[4]
                    + t * t * t * p[6]
                    - 265.;
                let y = u * u * u * p[1]
                    + 3. * u * u * t * p[3]
                    + 3. * u * t * t * p[5]
                    + t * t * t * p[7]
                    - 212.;
                let a = (20. * x - 10. * y) / 500.;
                let b = (-4. * x - 8. * y) / 80.;
                assert!(
                    (a.hypot(b) - 1.).abs() * 500_f64.sqrt() < 0.002,
                    "ellipse {x} {y}"
                );
            }
        }
        let quadratic: Vec<_> = page
            .cubics
            .iter()
            .filter(|p| (p.color[0] - 0.6).abs() < 0.002 && (p.color[1] - 68. / 255.).abs() < 0.002)
            .collect();
        assert_eq!(quadratic.len(), 1);
        let expected = [
            175.,
            207.,
            175. + 20. / 3.,
            207. + 40. / 3.,
            195. - 20. / 3.,
            207. + 40. / 3.,
            195.,
            207.,
        ];
        assert!(
            quadratic[0]
                .points
                .iter()
                .zip(expected)
                .all(|(a, b)| (a - b).abs() < 0.002),
            "{:?}",
            quadratic[0]
        );
        let rational: Vec<_> = page
            .ink
            .iter()
            .filter(|p| {
                (p.color[0] - 0.2).abs() < 0.002
                    && (p.color[1] - 0.6).abs() < 0.002
                    && (p.color[2] - 0.4).abs() < 0.002
            })
            .collect();
        assert!(!rational.is_empty());
        for edge in rational {
            for t in [0., 0.5, 1.] {
                let x = edge.segment[0] * (1. - t) + edge.segment[2] * t - 100.;
                let y = edge.segment[1] * (1. - t) + edge.segment[3] * t - 227.;
                assert!((x.hypot(y) - 50.).abs() < 0.002, "rational {x} {y}");
            }
        }
        let hatch: Vec<_> = page
            .ink
            .iter()
            .filter(|p| {
                (p.color[0] - 187. / 255.).abs() < 0.002 && (p.color[1] - 136. / 255.).abs() < 0.002
            })
            .collect();
        assert!(!hatch.is_empty());
        for edge in hatch {
            for p in [
                [edge.segment[0], edge.segment[1]],
                [edge.segment[2], edge.segment[3]],
            ] {
                assert!(
                    (159.998..=176.002).contains(&p[0]) && (91.998..=116.002).contains(&p[1]),
                    "hatch {p:?}"
                );
            }
        }
        let blue: Vec<_> = page
            .ink
            .iter()
            .filter(|p| {
                (p.color[0] - 0.2).abs() < 0.002
                    && (p.color[1] - 0.4).abs() < 0.002
                    && (p.color[2] - 0.6).abs() < 0.002
            })
            .collect();
        assert!(blue.len() >= 16, "{} {:?}", c["name"], page.ink);
        for edge in blue {
            for point in [
                [edge.segment[0], edge.segment[1]],
                [edge.segment[2], edge.segment[3]],
            ] {
                assert!(
                    ((point[0] - 130.).hypot(point[1] - 97.) - 20.).abs() < 0.002,
                    "endpoint {point:?}"
                );
            }
            let point = [
                (edge.segment[0] + edge.segment[2]) / 2.,
                (edge.segment[1] + edge.segment[3]) / 2.,
            ];
            assert!(
                ((point[0] - 130.).hypot(point[1] - 97.) - 20.).abs() < 0.01,
                "chord {point:?} {:?}",
                edge.segment
            );
        }
    }
    let folder = std::env::temp_dir().join(format!("kentos-sheet22-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    let pdf = folder.join("document.pdf");
    std::fs::write(&pdf, &output.bytes).unwrap();
    let text = std::process::Command::new("pdftotext")
        .arg(&pdf)
        .arg("-")
        .output()
        .unwrap();
    assert!(text.status.success());
    let words = String::from_utf8(text.stdout).unwrap();
    let text_pages: Vec<_> = words.split('\u{c}').collect();
    for (n, c) in cases.iter().enumerate() {
        assert!(
            text_pages[n].contains(c["expected"]["text"].as_str().unwrap()),
            "{} {:?}",
            c["name"],
            text_pages[n]
        );
        assert!(text_pages[n].contains("MODEL 5000 mm"));
    }
    let fonts = std::process::Command::new("pdffonts")
        .arg(&pdf)
        .output()
        .unwrap();
    assert!(fonts.status.success());
    let fonts = String::from_utf8(fonts.stdout).unwrap();
    let rows: Vec<_> = fonts
        .lines()
        .skip(2)
        .map(|r| r.split_whitespace().collect::<Vec<_>>())
        .collect();
    assert_eq!(rows.len(), 22, "{fonts}");
    for row in &rows {
        assert_eq!(
            &row[row.len() - 5..row.len() - 2],
            ["yes", "yes", "yes"],
            "{row:?}"
        );
    }
    for c in cases {
        let name = c["expected"]["postscript"].as_str().unwrap();
        assert!(
            rows.iter()
                .any(|r| r[0].split_once('+').is_some_and(|(_, n)| n == name)),
            "{name} absent: {fonts}"
        );
    }
    let images = std::process::Command::new("pdfimages")
        .arg("-list")
        .arg(&pdf)
        .output()
        .unwrap();
    assert!(images.status.success());
    assert_eq!(String::from_utf8(images.stdout).unwrap().lines().count(), 2);
    if let Some(dir) = std::env::var_os("KENTOS_PLOT_OUT") {
        std::fs::write(
            std::path::Path::new(&dir).join("sheet22-document.pdf"),
            &output.bytes,
        )
        .unwrap();
    }
    assert_eq!(doc.to_snapshot_v2(), before);
    std::fs::remove_dir_all(folder).unwrap();
}
#[test]
fn twenty_raster_pages_use_their_own_resolution_instead_of_the_books_maximum() {
    let data: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/sheet/v1/plot-document.json"
    ))
    .unwrap();
    let cases = data["dpiCases"].as_array().unwrap();
    assert_eq!(cases.len(), 20);
    let folder = std::env::temp_dir().join(format!("kentos-sheet22-raster-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    let image = folder.join("source.png");
    let mut bytes = Vec::new();
    {
        let mut e = png::Encoder::new(&mut bytes, 2, 1);
        e.set_color(png::ColorType::Rgb);
        e.set_depth(png::BitDepth::Eight);
        e.write_header()
            .unwrap()
            .write_image_data(&[20, 160, 200, 40, 200, 140])
            .unwrap();
    }
    std::fs::write(&image, bytes).unwrap();
    let mut model = data["model"].clone();
    model["entities"] = serde_json::json!([{"kind":"image","id":1,"layerId":"cizim","attrs":{},"file":image,"p":{"x":0,"y":0},"width":2,"height":1,"rotation":0}]);
    let mut doc =
        kentos_domain::Document::from_snapshot(serde_json::from_value(model).unwrap()).unwrap();
    let mut book = SheetBook::default();
    for c in cases {
        book.sheets
            .push(serde_json::from_value(c["sheet"].clone()).unwrap());
    }
    doc.set_sheets(project_sheets(&book, |_| None).unwrap())
        .unwrap();
    let before = doc.to_snapshot_v2();
    let output = kentos_plot::book::book_pdf(&doc, &[], "Test", "2026-10-23").unwrap();
    let path = folder.join("resolution.pdf");
    std::fs::write(&path, output.bytes).unwrap();
    let run = std::process::Command::new("pdfimages")
        .arg("-list")
        .arg(&path)
        .output()
        .unwrap();
    assert!(run.status.success());
    let rows = String::from_utf8(run.stdout).unwrap();
    let images: Vec<_> = rows
        .lines()
        .skip(2)
        .map(|l| l.split_whitespace().collect::<Vec<_>>())
        .filter(|r| r[2] == "image")
        .collect();
    assert_eq!(images.len(), 20, "{rows}");
    for (c, row) in cases.iter().zip(images) {
        assert_eq!(
            row[3].parse::<u64>().unwrap(),
            c["pixels"][0].as_u64().unwrap()
        );
        assert_eq!(
            row[4].parse::<u64>().unwrap(),
            c["pixels"][1].as_u64().unwrap()
        );
    }
    assert_eq!(doc.to_snapshot_v2(), before);
    std::fs::remove_dir_all(folder).unwrap();
}
#[test]
fn an_impossible_visible_curve_refuses_the_export_and_an_offscreen_one_is_culled() {
    let data: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/sheet/v1/plot-document.json"
    ))
    .unwrap();
    for outside in [false, true] {
        let mut model = data["model"].clone();
        model["entities"][1]["r"] = serde_json::json!(1e10);
        if outside {
            model["entities"][1]["c"] = serde_json::json!({"x":1e12,"y":1e12});
        }
        let mut doc =
            kentos_domain::Document::from_snapshot(serde_json::from_value(model).unwrap()).unwrap();
        let mut book = SheetBook::default();
        book.sheets
            .push(serde_json::from_value(data["cases"][0]["sheet"].clone()).unwrap());
        doc.set_sheets(project_sheets(&book, |_| None).unwrap())
            .unwrap();
        let before = doc.to_snapshot_v2();
        let result = kentos_plot::book::book_pdf(&doc, &[], "Test", "2026-10-23");
        if outside {
            assert!(result.is_ok(), "{result:?}");
        } else {
            assert!(
                result
                    .unwrap_err()
                    .message
                    .contains("plot_curve_too_complex")
            );
        }
        assert_eq!(doc.to_snapshot_v2(), before);
    }
}
