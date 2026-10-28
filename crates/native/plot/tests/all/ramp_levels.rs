//! A ramp on paper (docs/adr/0723 §5): bound to 1. kat 15 cm below it, drawn with En uzun kol, its level's plan sheet
//! made by `cad.sheets.levels`, written as the real PDF and read back with poppler's `pdftotext`: its slope beside its
//! arrow and its levels at its ends. The level raised by `cad.settings.set`, the PDF says the new levels.
//! `KENTOS_PLOT_OUT=dir` keeps both PDFs there (`rampa-pafta-1.pdf`, `rampa-pafta-2.pdf`).

use kentos_commands::CallOp;
use kentos_domain::Document;
use kentos_native_application::{ExecutionContext, registry};
use serde_json::{Value, json};

fn run(doc: &mut Document, name: &str, input: Value) -> Value {
    let answer = registry::run(&mut ExecutionContext::new(doc), name, CallOp::Execute, input).expect("runs");
    assert_eq!(answer["status"], "completed", "{name}: {answer}");
    answer
}

fn drawing() -> Document {
    let snapshot = json!({
        "format": "kentos.document", "version": 1, "name": "Rampa",
        "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
            "plotScale": 100, "workspace": "cad", "arch": {"levels": [
                {"id": "zemin", "name": "Zemin kat", "elevation": 0.0, "height": 3.06},
                {"id": "kat-1", "name": "1. kat", "elevation": 3.06, "height": 3.06}]}},
        "origin": {"x": 0, "y": 0},
        "layers": [{"id": "rampa", "name": "Rampa", "type": "layer", "visible": true, "locked": false, "expanded": true,
            "style": {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}, "children": []}],
        "activeLayer": "rampa",
        "styles": {"items": [], "categories": []},
        "entities": []
    });
    Document::from_snapshot(serde_json::from_value(snapshot).unwrap()).unwrap()
}

/// The PDF's words, by poppler; none when poppler is not installed (said so).
fn words(pdf: &[u8], name: &str) -> Option<String> {
    let dir = std::env::var_os("KENTOS_PLOT_OUT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join(format!("kentos-ramp-{}", std::process::id())));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    std::fs::write(&path, pdf).unwrap();
    let out = std::process::Command::new("pdftotext").arg(&path).arg("-").output().ok()?;
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

#[test]
fn a_bound_ramp_writes_its_slope_and_levels_on_its_level_plan_and_follows_its_level() {
    let mut doc = drawing();
    // 1.20 m up at 5 %, landings 1.50 m, no run longer than 9 m; its start 15 cm below 1. kat.
    run(&mut doc, "cad.ramps.create", json!({"layerId": "rampa", "ramps": [{
        "pts": [{"x": 0.0, "y": 0.0}, {"x": 30.0, "y": 0.0}], "width": 1.5, "rise": 1.2, "slope": 5.0, "landing": 1.5,
        "fit": true, "maxRun": 9.0, "baseOffset": -0.15, "railings": "both", "arch": {"level": "kat-1"}}]}));
    run(&mut doc, "cad.sheets.levels", json!({"levels": ["kat-1"]}));
    let first = kentos_plot::book::book_pdf(&doc, &[], "Rampa", "2026-10-28").unwrap();
    let Some(text) = words(&first.bytes, "rampa-pafta-1.pdf") else {
        eprintln!("pdftotext yok; PDF'in yazıları okunmadı");
        return;
    };
    for want in ["%5.00", "+2.91", "+4.11", "1. kat"] {
        assert!(text.contains(want), "{want} PDF'te yok:\n{text}");
    }
    // 1. kat raised to 3.40: the ramp's start and end follow, its slope stays.
    run(&mut doc, "cad.settings.set", json!({"arch": {"levels": [
        {"id": "zemin", "name": "Zemin kat", "elevation": 0.0, "height": 3.06},
        {"id": "kat-1", "name": "1. kat", "elevation": 3.4, "height": 3.06}]}}));
    let second = kentos_plot::book::book_pdf(&doc, &[], "Rampa", "2026-10-28").unwrap();
    let text = words(&second.bytes, "rampa-pafta-2.pdf").expect("pdftotext");
    for want in ["%5.00", "+3.25", "+4.45"] {
        assert!(text.contains(want), "{want} PDF'te yok:\n{text}");
    }
    assert!(!text.contains("+2.91"), "eski kot kaldı:\n{text}");
}
