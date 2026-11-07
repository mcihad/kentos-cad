//! ROAD-SEC-06 (docs/adr/1035): a view's scales, its place and its text size change what is drawn, never what is
//! computed. The same section set and the same profile drawn at another vertical exaggeration, in other columns, with
//! other text and elsewhere on the sheet give the same stations, elevations, grades and areas, and every text they
//! write says the same (only where it stands moves).

use kentos_commands::{CommandResult, DocumentSnapshotV2, Entity, ProfilesDraw, SectionsDraw};
use kentos_domain::Document;
use kentos_native_application::{ExecutionContext, road_profiles_draw, road_sections_draw};

fn doc_of(file: &str) -> Document {
    let cases: serde_json::Value = serde_json::from_str(file).expect("cases");
    Document::from_snapshot_v2(serde_json::from_value::<DocumentSnapshotV2>(cases["setup"].clone()).expect("setup")).expect("opens")
}

/// The texts on `layer` the call wrote, sorted: what they say, not where.
fn texts(doc: &Document, layer: &str, from: usize) -> Vec<String> {
    let mut out: Vec<String> = doc
        .entities()
        .skip(from)
        .filter(|e| e.base().layer_id == layer)
        .filter_map(|e| match e {
            Entity::Text(t) => Some(t.text.clone()),
            _ => None,
        })
        .collect();
    out.sort();
    out
}

#[test]
fn section_views_say_the_same_at_every_scale() {
    let file = include_str!("../../../../../fixtures/commands/v1/road.sections.draw.json");
    let run = |input: serde_json::Value| {
        let mut doc = doc_of(file);
        let before = doc.entities().count();
        let input: SectionsDraw = serde_json::from_value(input).expect("input");
        let out = match road_sections_draw::execute(&mut ExecutionContext::new(&mut doc), input) {
            CommandResult::Completed { output, .. } => output,
            other => panic!("draws: {other:?}"),
        };
        let said = texts(&doc, "0", before);
        (out, said)
    };
    let (a, ta) = run(serde_json::json!({"set": "k-1", "layerId": "0", "at": [0.0, -100.0]}));
    let (b, tb) = run(serde_json::json!({
        "set": "k-1", "layerId": "0", "at": [500.0, -900.0], "exaggeration": 5.0, "columns": 3, "height": 3.5
    }));
    assert_eq!(a.views.len(), b.views.len());
    for (x, y) in a.views.iter().zip(&b.views) {
        assert_eq!((x.station, x.km, &x.km_text), (y.station, y.km, &y.km_text));
        assert_eq!((x.datum, x.top, x.cut, x.fill), (y.datum, y.top, y.cut, y.fill), "km {}", x.km_text);
        assert_ne!(x.frame, y.frame, "the frame is the drawing's");
    }
    assert_eq!((a.grid, a.offsets), (b.grid, b.offsets), "the grids are the metres', not the paper's");
    assert!(!ta.is_empty());
    assert_eq!(ta, tb, "every text says the same");
    println!("enkesit: {} görünüş, {} yazı", a.views.len(), ta.len());
}

#[test]
fn profile_views_say_the_same_at_every_scale() {
    let file = include_str!("../../../../../fixtures/commands/v1/road.profiles.draw.json");
    let run = |input: serde_json::Value| {
        let mut doc = doc_of(file);
        let before = doc.entities().count();
        let input: ProfilesDraw = serde_json::from_value(input).expect("input");
        let out = match road_profiles_draw::execute(&mut ExecutionContext::new(&mut doc), input) {
            CommandResult::Completed { output, .. } => output,
            other => panic!("draws: {other:?}"),
        };
        let said = texts(&doc, "0", before);
        (out, said)
    };
    let (a, ta) = run(serde_json::json!({"profile": "b-1", "layerId": "0", "at": [0.0, -200.0]}));
    let (b, tb) = run(serde_json::json!({
        "profile": "b-1", "layerId": "0", "at": [1000.0, -3000.0], "exaggeration": 20.0, "height": 3.5
    }));
    assert_eq!(a.columns, b.columns, "the bands' stations, elevations and grades");
    assert_eq!((a.datum, a.top, a.grid), (b.datum, b.top, b.grid), "the elevations the frame spans");
    assert_ne!(a.frame, b.frame, "the frame is the drawing's");
    assert!(!ta.is_empty());
    assert_eq!(ta, tb, "every text says the same");
    println!("boykesit: {} sütun, {} yazı", a.columns.len(), ta.len());
}

/// A view's texts as what it computed (km, axis elevations, areas: words) and its grid's labels (plain numbers).
fn words_and_labels(t: &[String]) -> (Vec<String>, Vec<String>) {
    t.iter().cloned().partition(|w| w.parse::<f64>().is_err())
}

/// The sheets' scale is the paper's (1:500 against 1:1000): the sections laid on them write the same km, axis
/// elevations and areas. A sheet's views share its tallest elevation range: fewer sheets give taller frames, and the
/// grid labels that come with them are more elevations, never other ones.
#[test]
fn section_sheets_say_the_same_at_every_scale() {
    let file = include_str!("../../../../../fixtures/commands/v1/road.sheets.sections.json");
    let run = |input: serde_json::Value| {
        let mut doc = doc_of(file);
        let before = doc.entities().count();
        let input: kentos_commands::RoadSheetsSections = serde_json::from_value(input).expect("input");
        match kentos_native_application::road_sheets_sections::execute(&mut ExecutionContext::new(&mut doc), input) {
            CommandResult::Completed { .. } => {}
            other => panic!("lays: {other:?}"),
        }
        texts(&doc, "0", before)
    };
    let a = run(serde_json::json!({"set": "k-1", "layerId": "0", "at": [0.0, -500.0]}));
    let b = run(serde_json::json!({"set": "k-1", "layerId": "0", "at": [0.0, -500.0], "scale": 1000}));
    let (words, labels) = words_and_labels(&a);
    let (words_b, mut labels_b) = words_and_labels(&b);
    assert!(words.iter().any(|w| w.starts_with("Yarma")) && words.iter().any(|w| w.starts_with("Eksen kotu")));
    assert_eq!(words, words_b, "km, axis elevations and areas");
    for l in &labels {
        let at = labels_b.iter().position(|m| m == l).unwrap_or_else(|| panic!("{l} at 1:500 only"));
        labels_b.remove(at);
    }
    assert!(labels_b.iter().all(|l| l.contains('.')), "the extra labels are elevations of taller frames: {labels_b:?}");
    println!("enkesit paftaları: {} hesap yazısı aynı, 1:1000'de {} kot etiketi fazla", words.len(), labels_b.len());
}
