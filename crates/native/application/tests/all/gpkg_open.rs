//! A GeoPackage opened as a project (docs/adr/9116): every case of fixtures/data/v1/gpkg-open (written with sqlite3 by
//! scripts/fixtures/gpkg_open_cases.py, which works the expectations out from the ADR's rules) opened from a copy, as
//! opening writes the new project into the file; then the copy opened again gives that project.

use std::path::{Path, PathBuf};

use kentos_native_application::gpkg_open::{self, Opened};
use serde_json::Value;

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/data/v1/gpkg-open")
}

/// A copy of a case's file in a folder of its own.
fn copy(file: &str, tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("kentos-gpkg-open-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder");
    let to = dir.join(file);
    std::fs::copy(fixtures().join(file), &to).expect("the copy");
    to
}

fn cases() -> Vec<Value> {
    let text = std::fs::read_to_string(fixtures().join("expected.json")).expect("expected.json");
    let doc: Value = serde_json::from_str(&text).expect("JSON");
    doc["cases"].as_array().expect("cases").clone()
}

fn codes(built: &gpkg_open::Built) -> Vec<&'static str> {
    built.notes.iter().map(|n| n.code).collect()
}

/// A built project against its case: its system, notes, layers in order with their tables, keys, geometry and fields,
/// the styled layers' renderer kinds and its first view.
fn check_built(case: &Value, built: &gpkg_open::Built) {
    let file = case["file"].as_str().unwrap_or_default();
    assert_eq!(built.name, case["project"].as_str().unwrap_or_default(), "{file}");
    let settings = built.document.settings();
    assert_eq!(u64::from(settings.srid), case["srid"].as_u64().unwrap_or(u64::MAX), "{file}: system");
    assert_eq!(settings.workspace, Some(kentos_commands::Workspace::Gis), "{file}");
    let want_notes: Vec<&str> = case["notes"].as_array().into_iter().flatten().filter_map(Value::as_str).collect();
    assert_eq!(codes(built), want_notes, "{file}: notes {:#?}", built.notes);
    let nodes = built.document.layers().nodes();
    let want = case["layers"].as_array().expect("layers");
    assert_eq!(
        nodes.iter().map(|n| n.name.as_str()).collect::<Vec<_>>(),
        want.iter().map(|l| l["name"].as_str().unwrap_or_default()).collect::<Vec<_>>(),
        "{file}: layers"
    );
    for (node, w) in nodes.iter().zip(want) {
        match w["table"].as_str() {
            None => {
                assert!(node.data.is_none(), "{file}: {} is a drawing layer", node.name);
                assert_eq!(node.id, gpkg_open::DRAFT_LAYER, "{file}");
            }
            Some(table) => {
                let data = node.data.as_ref().unwrap_or_else(|| panic!("{file}: {} is a data layer", node.name));
                assert_eq!(data.driver, "gpkg", "{file}");
                assert_eq!(data.table, table, "{file}");
                assert_eq!(data.key, w["key"].as_str().unwrap_or_default(), "{file}: {table}");
                match &w["geometry"] {
                    Value::Null => assert!(data.geometry.is_none(), "{file}: {table} has no geometry"),
                    g => {
                        let have = data.geometry.as_ref().unwrap_or_else(|| panic!("{file}: {table}'s geometry"));
                        assert_eq!(have.column, g["column"].as_str().unwrap_or_default(), "{file}: {table}");
                        assert_eq!(have.type_name, g["type"].as_str().unwrap_or_default(), "{file}: {table}");
                        assert_eq!(have.srid.map(u64::from), g["srid"].as_u64(), "{file}: {table}'s system");
                    }
                }
                let fields: Vec<&str> = node.fields.iter().map(|f| f.name.as_str()).collect();
                let want_fields: Vec<&str> =
                    w["fields"].as_array().expect("fields").iter().filter_map(Value::as_str).collect();
                assert_eq!(fields, want_fields, "{file}: {table}'s fields");
                assert_eq!(
                    node.style.renderer.as_ref().and_then(|r| r["type"].as_str()),
                    w["styled"].as_str(),
                    "{file}: {table}'s look"
                );
            }
        }
    }
    assert_eq!(built.document.layers().active(), gpkg_open::DRAFT_LAYER, "{file}");
    if let Some(home) = case["home"].as_array() {
        let h = built.document.home_view().unwrap_or_else(|| panic!("{file}: a first view"));
        let got = [h.min_x, h.min_y, h.max_x, h.max_y];
        for (g, w) in got.iter().zip(home) {
            assert!((g - w.as_f64().unwrap_or(f64::NAN)).abs() < 1e-6, "{file}: first view {got:?} ≠ {home:?}");
        }
    }
}

#[test]
fn every_case_opens_as_its_file_says() {
    kentos_data_gpkg::register();
    let all = cases();
    assert!(all.len() >= 10, "{} cases", all.len());
    for case in &all {
        let file = case["file"].as_str().expect("file");
        let path = copy(file, "case");
        let opened = gpkg_open::open(&path, None);
        match case["opens"].as_str() {
            Some("built") => {
                let Ok(Opened::Built(built)) = opened else {
                    panic!("{file}: not built");
                };
                check_built(case, &built);
                built.written.as_ref().unwrap_or_else(|why| panic!("{file}: not written: {why}"));
                assert!(!built.document.is_dirty(), "{file}: the project is its file's, saved");
                // Opened again, the file gives the project it now keeps.
                match gpkg_open::open(&path, None) {
                    Ok(Opened::Kept { name, bytes }) => {
                        assert_eq!(name, built.name, "{file}");
                        let snapshot = kentos_kcad::decode(&bytes).expect("its bytes read");
                        assert_eq!(
                            snapshot.layers.iter().map(|n| n.name.as_str()).collect::<Vec<_>>(),
                            built.document.layers().nodes().iter().map(|n| n.name.as_str()).collect::<Vec<_>>(),
                            "{file}: the kept project's layers"
                        );
                    }
                    _ => panic!("{file}: the project written is not opened again"),
                }
            }
            Some("kept") => {
                let Ok(Opened::Kept { name, .. }) = opened else {
                    panic!("{file}: its project is not opened");
                };
                assert_eq!(name, case["project"].as_str().unwrap_or_default(), "{file}");
                let address = gpkg_open::address_of(&path);
                let kept = kentos_native_application::project_store::list(&address, None).expect("listed");
                assert_eq!(kept.len(), 1, "{file}: no project is added");
            }
            Some("several") => {
                let Ok(Opened::Several(kept)) = opened else {
                    panic!("{file}: the projects are not named");
                };
                let names: Vec<&str> = kept.iter().map(|p| p.name.as_str()).collect();
                let want: Vec<&str> = case["projects"].as_array().expect("projects").iter().filter_map(Value::as_str).collect();
                assert_eq!(names, want, "{file}");
            }
            Some("refused") => match opened {
                Err(why) => assert!(why.contains("GeoPackage değil"), "{file}: {why}"),
                Ok(_) => panic!("{file}: opened"),
            },
            other => panic!("{file}: unknown expectation {other:?}"),
        }
    }
}

#[test]
fn a_name_asked_for_opens_that_project_or_makes_it_in_a_file_without_one() {
    kentos_data_gpkg::register();
    let path = copy("iki-projeli.gpkg", "asked");
    match gpkg_open::open(&path, Some("Zeytinlik")) {
        Ok(Opened::Kept { name, .. }) => assert_eq!(name, "Zeytinlik"),
        _ => panic!("the named project"),
    }
    let Err(missing) = gpkg_open::open(&path, Some("Bağ")) else {
        panic!("a name the file does not keep is refused");
    };
    assert!(missing.contains("“Bağ” adında proje yok") && missing.contains("“Çamlık”, “Zeytinlik”"), "{missing}");
    let empty = copy("katmanlar.gpkg", "named");
    match gpkg_open::open(&empty, Some("Sivas kuyuları")) {
        Ok(Opened::Built(built)) => {
            assert_eq!(built.name, "Sivas kuyuları");
            assert!(built.written.is_ok());
        }
        _ => panic!("built under the name asked for"),
    }
}

#[test]
fn a_file_that_cannot_be_written_opens_unsaved_and_says_why() {
    use std::os::unix::fs::PermissionsExt;
    kentos_data_gpkg::register();
    let path = copy("katmanlar.gpkg", "readonly");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o444)).expect("read only");
    let opened = gpkg_open::open(&path, None);
    let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644));
    let Ok(Opened::Built(built)) = opened else {
        panic!("a read-only file still opens");
    };
    let why = built.written.as_ref().expect_err("not written");
    assert!(why.contains("Proje dosyanın içine yazılamadı"), "{why}");
    assert!(built.document.is_dirty(), "the project is not the file's");
    assert_eq!(built.document.layers().nodes().len(), 6, "its tables are its layers all the same");
    let address = gpkg_open::address_of(&path);
    assert!(kentos_native_application::project_store::list(&address, None).expect("listed").is_empty());
}
