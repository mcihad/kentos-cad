//! A stair on paper (docs/adr/0724 §4, §6): a U bound to Zemin kat's height with its cut at 1.10 m, the levels' plan
//! sheets made by `cad.sheets.levels`, written as the real PDF and read back with poppler: `pdftotext` finds both
//! sheets, `pdftoppm` draws 1. kat's page, and the stair shows there (its part past its cut, seen from above) where a
//! stair without a cut leaves the page as the walls alone draw it. Zemin kat raised to 3.40 m, the stair's risers follow
//! (its numbers below the cut are the same, its sheet's ink too). `KENTOS_PLOT_OUT=dir` keeps the PDFs and pages there
//! (`merdiven-pafta-*.pdf`, `merdiven-pafta-*-2.pgm`).

use kentos_commands::CallOp;
use kentos_domain::Document;
use kentos_native_application::{ExecutionContext, registry};
use serde_json::{Value, json};

fn run(doc: &mut Document, name: &str, input: Value) -> Value {
    let answer = registry::run(&mut ExecutionContext::new(doc), name, CallOp::Execute, input).expect("runs");
    assert_eq!(answer["status"], "completed", "{name}: {answer}");
    answer
}

fn drawing(ground: f64) -> Document {
    let snapshot = json!({
        "format": "kentos.document", "version": 1, "name": "Merdiven",
        "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
            "plotScale": 50, "workspace": "cad", "arch": {"levels": [
                {"id": "zemin", "name": "Zemin kat", "elevation": 0.0, "height": ground},
                {"id": "kat-1", "name": "1. kat", "elevation": ground, "height": 3.06}]}},
        "origin": {"x": 0, "y": 0},
        "layers": [{"id": "mimari", "name": "Mimari", "type": "layer", "visible": true, "locked": false, "expanded": true,
            "style": {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}, "children": []}],
        "activeLayer": "mimari",
        "styles": {"items": [], "categories": []},
        "entities": []
    });
    Document::from_snapshot(serde_json::from_value(snapshot).unwrap()).unwrap()
}

/// The hall's walls on both levels and the U on Zemin kat (`cut`: its cut at 1.10 m, else none).
fn building(cut: bool) -> Document {
    let mut doc = drawing(3.06);
    for level in ["zemin", "kat-1"] {
        run(&mut doc, "cad.walls.create", json!({"layerId": "mimari", "walls": [{
            "pts": [{"x": 0.0, "y": 0.0}, {"x": 7.0, "y": 0.0}, {"x": 7.0, "y": 5.4}, {"x": 0.0, "y": 5.4}],
            "closed": true, "width": 0.25, "justify": "right", "arch": {"level": level}}]}));
    }
    let mut stair = json!({
        "pts": [{"x": 1.5, "y": 1.2}, {"x": 4.5, "y": 1.2}, {"x": 4.5, "y": 2.5}, {"x": 1.0, "y": 2.5}],
        "width": 1.2, "tread": 0.28, "heightFromLevel": true, "fit": true, "numbering": true, "railings": "both",
        "arch": {"level": "zemin"}});
    if cut {
        stair["cutHeight"] = json!(1.1);
    }
    run(&mut doc, "cad.stairs.create", json!({"layerId": "mimari", "stairs": [stair]}));
    run(&mut doc, "cad.sheets.levels", json!({}));
    doc
}

fn out_dir() -> std::path::PathBuf {
    let dir = std::env::var_os("KENTOS_PLOT_OUT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join(format!("kentos-stair-{}", std::process::id())));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// The PDF's words and its second page's dark pixels (1. kat's plan) drawn by poppler at 50 dpi in grey; none when
/// poppler is not installed (said so).
fn read(pdf: &[u8], name: &str) -> Option<(String, usize)> {
    let dir = out_dir();
    let path = dir.join(format!("{name}.pdf"));
    std::fs::write(&path, pdf).unwrap();
    let words = std::process::Command::new("pdftotext").arg(&path).arg("-").output().ok()?;
    let prefix = dir.join(name);
    let drawn = std::process::Command::new("pdftoppm")
        .args(["-r", "50", "-gray", "-f", "2", "-l", "2"])
        .arg(&path)
        .arg(&prefix)
        .status()
        .ok()?;
    assert!(drawn.success(), "pdftoppm");
    let page = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .find(|p| {
            p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with(name) && n.ends_with(".pgm"))
        })
        .expect("the page drawn");
    let bytes = std::fs::read(&page).unwrap();
    Some((String::from_utf8_lossy(&words.stdout).into_owned(), dark(&bytes)))
}

/// A binary PGM's pixels darker than mid grey.
fn dark(pgm: &[u8]) -> usize {
    // "P5", width, height, maximum, each after white space; the pixels after the last one's single white space.
    let mut fields = 0;
    let mut i = 0;
    while fields < 4 {
        while pgm[i].is_ascii_whitespace() {
            i += 1;
        }
        while !pgm[i].is_ascii_whitespace() {
            i += 1;
        }
        fields += 1;
    }
    pgm[i + 1..].iter().filter(|&&v| v < 128).count()
}

#[test]
fn the_level_above_s_sheet_draws_the_stair_past_its_cut() {
    let with = building(true);
    let pdf = kentos_plot::book::book_pdf(&with, &[], "Merdiven", "2026-10-28").unwrap();
    let Some((text, above)) = read(&pdf.bytes, "merdiven-pafta-kesmeli") else {
        eprintln!("poppler yok; PDF okunmadı");
        return;
    };
    for want in ["Zemin kat planı", "1. kat planı"] {
        assert!(text.contains(want), "{want} PDF'te yok:\n{text}");
    }
    let without = building(false);
    let pdf = kentos_plot::book::book_pdf(&without, &[], "Merdiven", "2026-10-28").unwrap();
    let (_, bare) = read(&pdf.bytes, "merdiven-pafta-kesmesiz").expect("poppler");
    // The stair from above draws its sides, risers, landing, railings and arrow on 1. kat's page: a few hundred dark
    // pixels at 50 dpi more than the walls alone.
    assert!(above > bare + 200, "1. kat'ta merdiven görünmüyor: {above} ≤ {bare} + 200");
    // Zemin kat raised: the stair's risers follow; its plan from above is the same.
    let mut raised = building(true);
    run(&mut raised, "cad.settings.set", json!({"arch": {"levels": [
        {"id": "zemin", "name": "Zemin kat", "elevation": 0.0, "height": 3.4},
        {"id": "kat-1", "name": "1. kat", "elevation": 3.4, "height": 3.06}]}}));
    let stair = raised
        .entities()
        .find_map(|e| match e {
            kentos_commands::Entity::Stair(s) => Some(s.stair.clone()),
            _ => None,
        })
        .expect("the stair");
    assert!((f64::from(stair.risers) * stair.riser_height - 3.4).abs() < 1e-9, "{stair:?}");
    let pdf = kentos_plot::book::book_pdf(&raised, &[], "Merdiven", "2026-10-28").unwrap();
    let (_, again) = read(&pdf.bytes, "merdiven-pafta-yukseldi").expect("poppler");
    assert!(again > bare + 200, "kat yükselince 1. kat'ta merdiven görünmüyor: {again}");
}
