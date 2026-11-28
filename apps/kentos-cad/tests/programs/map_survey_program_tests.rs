//! GNSS arşivi across the modules (docs/adr/1340 §6): Harita edits the archive, GIS reads the accepted point in
//! Öznitelikler, Mimari and Yol neither. Moved from the map package's map_survey_panel_tests.rs: it spans the modules
//! (docs/adr/9199 §4.14).

use crate::app::{App, Message, Panel};
use kentos_domain::Slot;
use kentos_launch::Module;
use serde_json::Value;

fn fixtures() -> String {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .canonicalize()
        .expect("the fixtures")
        .to_string_lossy()
        .into_owned()
}

/// `map.survey.query`'s drawing (map_survey_cases.py), its paths made absolute.
fn snapshot() -> kentos_commands::DocumentSnapshotV2 {
    let text = include_str!("../../../../fixtures/commands/v1/map.survey.query.json")
        .replace("$fixture:", &format!("{}/", fixtures()));
    let cases: Value = serde_json::from_str(&text).expect("cases");
    serde_json::from_value(cases["setup"].clone()).expect("the drawing reads")
}

fn opened(snapshot: kentos_commands::DocumentSnapshotV2, program: Option<Module>) -> App {
    let (mut app, _) = App::boot(None);
    app.program = program;
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

/// Öznitelikler's GNSS ölçümü rows of the object in `slot`, selected alone.
fn survey_rows(app: &mut App, slot: u32) -> Vec<(String, String)> {
    app.selection.set(vec![Slot(slot)]);
    let doc = app.document.as_ref().expect("a drawing");
    let panel = app.properties_panel(doc);
    panel
        .sections
        .iter()
        .find(|s| s.id == "mapSurvey")
        .map(|s| {
            s.rows
                .iter()
                .map(|r| (r.label.to_string(), r.value.clone()))
                .collect()
        })
        .unwrap_or_default()
}

fn row<'a>(rows: &'a [(String, String)], label: &str) -> Option<&'a str> {
    rows.iter()
        .find(|(l, _)| l == label)
        .map(|(_, v)| v.as_str())
}

#[test]
fn harita_edits_the_archive_gis_reads_the_point_mimari_and_yol_neither() {
    let mut map = opened(snapshot(), Some(Module::Map));
    assert!(
        map.offers("map.survey")
            && map.offers("map.survey.import")
            && map.offers("map.survey.select")
    );
    assert!(!survey_rows(&mut map, 1).is_empty());
    let mut gis = opened(snapshot(), Some(Module::Gis));
    assert!(!gis.shows_panel(Panel::MapSurvey));
    assert!(!gis.offers("map.survey") && !gis.offers("map.survey.import"));
    assert!(gis.offers("map.survey.select"));
    assert_eq!(
        row(&survey_rows(&mut gis, 1), "Kabul edilen nokta"),
        Some("kn-1 · 1. revizyon")
    );
    for module in [Module::Architecture, Module::Road] {
        let mut app = opened(snapshot(), Some(module));
        assert!(!app.shows_panel(Panel::MapSurvey), "{module:?}");
        assert!(
            !app.offers("map.survey") && !app.offers("map.survey.select"),
            "{module:?}"
        );
        assert!(survey_rows(&mut app, 1).is_empty(), "{module:?}");
    }
}
