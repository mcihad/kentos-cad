//! Kaynaklar's folder listing (docs/adr/0199 §7) against the shared cases
//! fixtures/sources/v1/cases.json (scripts/fixtures/source_list_cases.py, no
//! KentOS code), the cases the web runs in
//! apps/web/src/model/sources.wasm.test.ts.

use kentos_interaction::sources::{
    FAVOURITES_MOST, RECENT_MOST, SourceEntry, SourceKind, listing, recent_with, search_matches,
    search_shown, search_words, source_kind, starred,
};
use serde_json::{Value, json};

fn fixture() -> Value {
    let file: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/sources/v1/cases.json"
    ))
    .expect("fixture JSON");
    assert_eq!(file["format"], "kentos.source-list-cases");
    assert_eq!(file["version"], 1);
    file
}

#[test]
fn a_folder_shows_its_folders_and_the_files_it_adds() {
    let f = fixture();
    for case in f["cases"].as_array().expect("cases") {
        let name = case["name"].as_str().expect("a name");
        let entries: Vec<SourceEntry> = case["entries"]
            .as_array()
            .expect("entries")
            .iter()
            .map(|e| SourceEntry {
                name: e["name"].as_str().expect("a name").to_owned(),
                dir: e["dir"].as_bool().expect("dir"),
            })
            .collect();
        let got = listing(&entries);
        let files: Vec<Value> = got
            .files
            .iter()
            .map(|f| json!({ "name": f.name, "kind": f.kind.key(), "parts": f.parts }))
            .collect();
        assert_eq!(
            json!({ "folders": got.folders, "files": files }),
            case["expect"],
            "{name}"
        );
    }
}

#[test]
fn the_kinds_are_named_as_the_cases_name_them() {
    let f = fixture();
    for kind in SourceKind::ALL {
        assert_eq!(f["labels"][kind.key()], kind.label());
    }
    assert_eq!(
        [
            source_kind("a.SHP"),
            source_kind(".dxf"),
            source_kind("b"),
            source_kind("c.kcad"),
            source_kind("d.GPKG"),
            source_kind("e.copc.laz"),
        ],
        [
            Some(SourceKind::Shapefile),
            None,
            None,
            None,
            Some(SourceKind::GeoPackage),
            Some(SourceKind::PointCloud)
        ]
    );
}

fn strings(v: &Value) -> Vec<String> {
    v.as_array()
        .expect("a list")
        .iter()
        .map(|s| s.as_str().expect("a string").to_owned())
        .collect()
}

#[test]
fn the_search_shows_the_rows_that_match_and_their_ancestors() {
    let f = fixture();
    let rows = f["search"]["rows"].as_array().expect("rows");
    let depths: Vec<usize> = rows
        .iter()
        .map(|r| r["depth"].as_u64().expect("a depth") as usize)
        .collect();
    let fields: Vec<Vec<String>> = rows.iter().map(|r| strings(&r["fields"])).collect();
    let cases = f["search"]["cases"].as_array().expect("cases");
    assert!(cases.len() >= 10);
    for case in cases {
        let name = case["name"].as_str().expect("a name");
        let words = search_words(case["query"].as_str().expect("a query"));
        let matched: Vec<bool> = fields
            .iter()
            .map(|f| search_matches(&words, &f.iter().map(String::as_str).collect::<Vec<_>>()))
            .collect();
        let got: Vec<usize> = search_shown(&depths, &matched)
            .iter()
            .enumerate()
            .filter(|(_, k)| **k)
            .map(|(i, _)| i)
            .collect();
        let want: Vec<usize> = case["shown"]
            .as_array()
            .expect("shown")
            .iter()
            .map(|i| i.as_u64().expect("a place") as usize)
            .collect();
        assert_eq!(got, want, "{name}");
    }
}

#[test]
fn the_recent_and_favourite_sources_keep_their_rules() {
    let f = fixture();
    assert_eq!(f["recent"]["most"], RECENT_MOST);
    assert_eq!(f["favourites"]["most"], FAVOURITES_MOST);
    for case in f["recent"]["cases"].as_array().expect("cases") {
        let name = case["name"].as_str().expect("a name");
        let got = recent_with(
            &strings(&case["recent"]),
            &case["used"].as_str().expect("used").to_owned(),
        );
        assert_eq!(got, strings(&case["expect"]), "{name}");
    }
    for case in f["favourites"]["cases"].as_array().expect("cases") {
        let name = case["name"].as_str().expect("a name");
        let (got, refused) = starred(
            &strings(&case["favourites"]),
            &case["starred"].as_str().expect("starred").to_owned(),
        );
        assert_eq!(got, strings(&case["expect"]), "{name}");
        assert_eq!(
            refused,
            case["refused"].as_bool().expect("refused"),
            "{name}"
        );
    }
}
