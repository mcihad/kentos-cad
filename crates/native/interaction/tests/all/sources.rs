//! Kaynaklar's folder listing (docs/adr/0199 §7) against the shared cases
//! fixtures/sources/v1/cases.json (scripts/fixtures/source_list_cases.py, no
//! KentOS code), the cases the web runs in
//! apps/web/src/model/sources.wasm.test.ts.

use kentos_interaction::sources::{SourceEntry, SourceKind, listing, source_kind};
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
            source_kind("c.kcad")
        ],
        [Some(SourceKind::Shapefile), None, None, None]
    );
}
