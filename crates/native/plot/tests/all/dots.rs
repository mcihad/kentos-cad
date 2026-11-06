//! ADR 1124: a CAD line type's zero-length dash (a dot) prints as AutoCAD plots it, a round dot of the line's weight,
//! in the PDF (measured in Poppler's independent picture of it) and in the CPU preview, at 1:1, 1:20 and 1:100. The
//! line's dashes keep their butt ends. Expected sizes are the pattern's and the weight's, not the output's.
use kentos_domain::Document;
use kentos_plot::book;
use kentos_sheet::project::project_sheets;
use serde_json::{Value, json};

/// A 40 mm line (paper) through the map's centre, dash 2 mm, gap 1, dot, gap 1, weight 0.5 mm, at 1:`scale`.
fn dotted(scale: u32) -> Document {
    let fx: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/sheet/v1/native-preflight.json"
    ))
    .unwrap();
    let (mut raw, mut page) = (fx["model"].clone(), fx["book"].clone());
    raw["settings"]["lineTypes"] = json!([{"name":"D","unit":"paper",
        "elements":[{"length":2.},{"length":-1.},{"length":0.},{"length":-1.}]}]);
    let half = f64::from(scale) * 0.02;
    raw["entities"] = json!([{"kind":"line","id":1,"layerId":raw["entities"][0]["layerId"].clone(),"attrs":{},
        "a":{"x":-half,"y":0.},"b":{"x":half,"y":0.},"linePattern":"D","lineWeight":0.5}]);
    page["sheets"][0]["items"][0]["kind"]["view"]["scale"] = json!(scale);
    page["sheets"][0]["items"][0]["kind"]["view"]["center"] = json!({"x":0.,"y":0.});
    let mut doc = Document::from_snapshot(serde_json::from_value(raw).unwrap()).unwrap();
    let book: kentos_sheet::SheetBook = serde_json::from_value(page).unwrap();
    doc.set_sheets(project_sheets(&book, |_| None).unwrap()).unwrap();
    doc
}

/// Runs of ink (`dark` at a pixel) along a row: (first pixel, length).
fn runs(len: usize, dark: impl Fn(usize) -> bool) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut start = None;
    for x in 0..=len {
        match (x < len && dark(x), start) {
            (true, None) => start = Some(x),
            (false, Some(s)) => {
                out.push((s, x - s));
                start = None;
            }
            _ => {}
        }
    }
    out
}

/// The dots among `runs` (`mm` pixels a millimetre): every run under 1 mm, each 0.5 mm long, its centre 1 mm after
/// the dash before it; at least nine over the 40 mm line.
fn check_dots(runs: &[(usize, usize)], mm: f64, slack: f64, what: &str) -> Vec<f64> {
    let dots: Vec<_> = runs.iter().filter(|r| (r.1 as f64) < mm).collect();
    let dashes = runs.iter().filter(|r| (r.1 as f64) > 1.5 * mm).count();
    assert!(dots.len() >= 9 && dashes >= 9, "{what}: {} dots, {dashes} dashes: {runs:?}", dots.len());
    let mut centres = Vec::new();
    for w in runs.windows(2) {
        let [dash, dot] = [w[0], w[1]];
        if (dot.1 as f64) >= mm || (dash.1 as f64) < 1.5 * mm {
            continue;
        }
        assert!((dot.1 as f64 - 0.5 * mm).abs() <= slack, "{what}: dot {} px, weight {} px", dot.1, 0.5 * mm);
        let centre = dot.0 as f64 + dot.1 as f64 / 2.;
        let after = centre - (dash.0 + dash.1) as f64;
        assert!((after - mm).abs() <= slack, "{what}: dot centre {after} px after its dash, 1 mm = {mm} px");
        centres.push(centre);
    }
    assert!(centres.len() >= 9, "{what}: {centres:?}");
    centres
}

#[test]
fn a_dot_prints_round_and_as_wide_as_the_line_in_the_pdf_and_the_cpu_preview() {
    let dir = std::env::temp_dir().join(format!("kentos-adr1124-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for scale in [1u32, 20, 100] {
        let doc = dotted(scale);
        let before = doc.to_snapshot_v2();
        let pdf = book::book_pdf(&doc, &[], "Test", "2026-11-06").unwrap();
        assert!(!String::from_utf8_lossy(&pdf.bytes).contains("/Subtype /Image"), "{scale}: vectors only");
        let path = dir.join(format!("dots-{scale}.pdf"));
        std::fs::write(&path, &pdf.bytes).unwrap();
        // Poppler at 1200 dpi over x 190–240 mm, y 120–130 mm of the page; the line runs along y = 125 mm.
        let mm = 1200. / 25.4;
        let crop = |v: f64| (v * mm).round() as u32;
        let prefix = dir.join(format!("dots-{scale}"));
        let run = std::process::Command::new("pdftoppm")
            .args(["-r", "1200", "-gray", "-singlefile", "-png"])
            .args(["-x", &crop(190.).to_string(), "-y", &crop(120.).to_string()])
            .args(["-W", &crop(50.).to_string(), "-H", &crop(10.).to_string()])
            .arg(&path)
            .arg(&prefix)
            .output()
            .unwrap();
        assert!(run.status.success(), "{}", String::from_utf8_lossy(&run.stderr));
        let mut reader = png::Decoder::new(std::io::Cursor::new(
            std::fs::read(prefix.with_extension("png")).unwrap(),
        ))
        .read_info()
        .unwrap();
        let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
        let info = reader.next_frame(&mut pixels).unwrap();
        let (w, h) = (info.width as usize, info.height as usize);
        let stride = info.color_type.samples();
        let dark = |x: usize, y: usize| x < w && y < h && pixels[(y * w + x) * stride] < 128;
        let row = (5. * mm).round() as usize;
        let along = runs(w, |x| dark(x, row));
        let centres = check_dots(&along, mm, 3., &format!("PDF 1:{scale}"));
        // Round: across the dot at its centre the line's weight, 0.2 mm off its centre 2·√(0.25² − 0.2²) = 0.3 mm.
        for c in centres {
            // The column whose pixel centre (its index + ½) is nearest `x` (pixel edges from 0).
            let height = |x: f64| runs(h, |y| dark((x - 0.5).round() as usize, y)).iter().map(|r| r.1).max().unwrap_or(0) as f64;
            assert!((height(c) - 0.5 * mm).abs() <= 3., "1:{scale} dot at {c}: {} px", height(c));
            for off in [-0.2, 0.2] {
                let at = height(c + off * mm);
                assert!((at - 0.3 * mm).abs() <= 3., "1:{scale} dot at {c}{off:+} mm: {at} px, round {}", 0.3 * mm);
            }
        }
        // The CPU preview at 200 dpi (7.9 px a millimetre): the same dots, a dot 0.5 mm long.
        let image = kentos_plot::preview::page(&doc, "ground", "Test", "2026-11-06", None, 200).unwrap();
        let k = 200. / 25.4;
        let (iw, y) = (image.width as usize, (125. * k) as usize);
        let lit = |x: usize, y: usize| image.rgba[(y * iw + x) * 4] < 128 && image.rgba[(y * iw + x) * 4 + 3] > 128;
        let x0 = (190. * k) as usize;
        let along = runs((50. * k) as usize, |x| lit(x0 + x, y) || lit(x0 + x, y - 1));
        check_dots(&along, k, 2., &format!("CPU 1:{scale}"));
        assert_eq!(doc.to_snapshot_v2(), before);
    }
    std::fs::remove_dir_all(dir).unwrap();
}

/// The evidence pictures of ADR 1124 (not a check): the dotted line's PDF and CPU preview at 1:20, written to
/// `.run/gui-evidence/` (never into git). `cargo test -p kentos-plot --test all dots::dot_pictures -- --ignored`.
#[test]
#[ignore]
fn dot_pictures() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../.run/gui-evidence");
    std::fs::create_dir_all(&dir).unwrap();
    let doc = dotted(20);
    let pdf = book::book_pdf(&doc, &[], "Test", "2026-11-06").unwrap();
    std::fs::write(dir.join("adr1124-nokta-1-20.pdf"), &pdf.bytes).unwrap();
    let image = kentos_plot::preview::page(&doc, "ground", "Test", "2026-11-06", None, 600).unwrap();
    std::fs::write(dir.join("adr1124-nokta-1-20-cpu.png"), &image.png).unwrap();
}
