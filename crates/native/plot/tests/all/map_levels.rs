//! A map's level (docs/adr/0716): a map of 1. kat draws its objects and those on no level, not Zemin kat's; with Alt
//! kat soluk, Zemin kat's too, faint; without a level, every level. A level the project no longer has is a preflight
//! finding with its fix.

use kentos_contracts::DocumentSnapshotV1;
use kentos_domain::Document;
use kentos_interaction::Spatial;
use kentos_sheet::{
    display::{self, Prim, RenderInputs},
    ops::{self, Op},
    *,
};
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!("../../../../../fixtures/sheet/v1/viewport-annotations.json")).unwrap()
}

/// Two rooms over each other, ALT on Zemin kat and UST on 1. kat, and a text ORTAK on no level.
fn drawing() -> Document {
    let mut m = fixture()["cases"][0]["model"].clone();
    m["settings"]["arch"] = json!({"levels": [
        {"id": "zemin", "name": "Zemin kat", "elevation": 0.0},
        {"id": "kat-1", "name": "1. kat", "elevation": 3.0}
    ]});
    let room = |id: u32, name: &str, level: &str| {
        json!({"kind": "space", "id": id, "layerId": "geom", "attrs": {},
            "parts": [{"outer": {"pts": [{"x": -2.0, "y": -1.5}, {"x": 2.0, "y": -1.5}, {"x": 2.0, "y": 1.5},
                {"x": -2.0, "y": 1.5}]}}],
            "name": name, "arch": {"level": level}})
    };
    m["entities"] = json!([
        room(1, "ALT", "zemin"),
        room(2, "UST", "kat-1"),
        {"kind": "text", "id": 3, "layerId": "anno", "attrs": {}, "p": {"x": 0.0, "y": 2.0}, "text": "ORTAK",
         "height": 0.2, "rotation": 0.0}
    ]);
    let snapshot: DocumentSnapshotV1 = serde_json::from_value(m).unwrap();
    Document::from_snapshot(snapshot).unwrap()
}

/// The sheet with its map at 1:50 round the rooms, `level` set as `setViewport` sets it.
fn book(level: Value) -> SheetBook {
    let mut book = SheetBook::default();
    book.sheets
        .push(serde_json::from_value(fixture()["cases"][0]["sheet"].clone()).unwrap());
    let op: Op = serde_json::from_value(json!({"op": "setViewport", "id": "viewport", "scale": 50, "locked": false,
        "center": {"x": 0.0, "y": 0.0}, "level": level}))
    .unwrap();
    ops::apply(&book, &op).unwrap().book
}

/// The words the map writes and their colours.
fn words(doc: &Document, level: Value) -> Vec<(String, String)> {
    let book = book(level);
    let list = display::display_list(&book, "page", &RenderInputs::default()).unwrap();
    let m = list
        .prims
        .iter()
        .find_map(|p| match p {
            Prim::Map(m) => Some(m),
            _ => None,
        })
        .unwrap();
    let mut spatial = Spatial::new();
    spatial.reload(doc);
    kentos_plot::labels::map_labels(doc, &spatial, m)
        .into_iter()
        .filter(|l| ["ALT", "UST", "ORTAK"].contains(&l.text.as_str()))
        .map(|l| (l.text, l.color))
        .collect()
}

fn has(words: &[(String, String)], text: &str) -> Option<String> {
    words.iter().find(|(t, _)| t == text).map(|(_, c)| c.clone())
}

#[test]
fn a_map_draws_its_level_and_the_one_below_faint() {
    let doc = drawing();
    let all = words(&doc, json!({"type": "all"}));
    assert!(has(&all, "ALT").is_some() && has(&all, "UST").is_some() && has(&all, "ORTAK").is_some(), "{all:?}");
    let upper = words(&doc, json!({"type": "level", "id": "kat-1"}));
    assert!(has(&upper, "ALT").is_none(), "Zemin kat left out: {upper:?}");
    assert!(has(&upper, "UST").is_some() && has(&upper, "ORTAK").is_some(), "{upper:?}");
    let over = words(&doc, json!({"type": "level", "id": "kat-1", "below": true}));
    let (faint, own) = (has(&over, "ALT").expect("ALT faint"), has(&over, "UST").expect("UST"));
    assert_ne!(faint, own, "the level below is faint");
    assert_eq!(faint, kentos_plot::layers::faint_on_paper(&own), "faint as a faded object on white");
    let ground = words(&doc, json!({"type": "level", "id": "zemin", "below": true}));
    assert!(has(&ground, "UST").is_none() && has(&ground, "ALT").is_some(), "no level under Zemin kat: {ground:?}");
}

#[test]
fn a_level_gone_from_the_project_is_a_finding_with_its_fix() {
    let doc = drawing();
    let mut book = book(json!({"type": "all"}));
    // A level written by the designer's patch, then taken out of the project.
    book = ops::apply(
        &book,
        &serde_json::from_value(json!({"op": "setItemProps", "id": "viewport",
            "patch": {"kind": {"level": {"id": "kat-9", "below": false}}}}))
        .unwrap(),
    )
    .unwrap()
    .book;
    let found = kentos_plot::preflight::inspect(&doc, &book, "page", &RenderInputs::default(), None).unwrap();
    let f = found
        .iter()
        .find(|f| f.code == "map_level_missing")
        .expect("the finding");
    assert!(f.message.contains("kat-9"), "{}", f.message);
    assert_eq!(f.fixes.len(), 1);
    let fixed = ops::apply(&book, &f.fixes[0].ops[0]).unwrap().book;
    let found = kentos_plot::preflight::inspect(&doc, &fixed, "page", &RenderInputs::default(), None).unwrap();
    assert!(found.iter().all(|f| f.code != "map_level_missing"));
}
