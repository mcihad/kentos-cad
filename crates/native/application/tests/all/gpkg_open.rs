//! A GeoPackage opened as a project (docs/adr/9116): every case of fixtures/data/v1/gpkg-open (written with sqlite3 by
//! scripts/fixtures/gpkg_open_cases.py, which works the expectations out from the ADR's rules) opened from a copy.
//! Opening and looking at the tables leave the file byte for byte as it was, its tables and indexes too (docs/adr/9122);
//! the project goes into the file only by `keep`, the owner's save, and the copy opened again gives that project.

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

/// The file's SHA-256 and its tables, indexes, triggers and views (`sqlite_master`), read by a connection of its own.
fn state(path: &Path) -> (String, Vec<(String, String)>) {
    use sha2::Digest;
    let digest = sha2::Sha256::digest(std::fs::read(path).expect("the file"));
    let hex = digest
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    // A file that is not a GeoPackage (a refused case) has no catalog to compare: its bytes are enough.
    let Ok(gp) = kentos_geopackage::GeoPackage::open(path, false) else {
        return (hex, Vec::new());
    };
    let mut st = gp
        .connection()
        .prepare("SELECT type, name FROM sqlite_master ORDER BY type, name")
        .expect("the catalog");
    let rows = st
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .expect("rows")
        .map(|r| r.expect("a row"))
        .collect();
    (hex, rows)
}

/// Every data layer's rows read through its source, as a look at the map and the table reads them.
fn look_at(built: &gpkg_open::Built) -> usize {
    let mut rows = 0;
    for node in built.document.layers().leaves() {
        let Some(data) = &node.data else { continue };
        let source = kentos_data::registry()
            .source(&data.address)
            .expect("the source");
        let request = kentos_data::Request {
            limit: Some(50),
            ..kentos_data::Request::all()
        };
        source
            .features(
                &data.table,
                &request,
                &kentos_data::Cancel::new(),
                &mut |part| {
                    rows += part.len();
                    true
                },
            )
            .expect("its rows");
    }
    rows
}

fn codes(built: &gpkg_open::Built) -> Vec<&'static str> {
    built.notes.iter().map(|n| n.code).collect()
}

/// A built project against its case: its system, notes, layers in order with their tables, keys, geometry and fields,
/// the styled layers' renderer kinds and its first view.
fn check_built(case: &Value, built: &gpkg_open::Built) {
    let file = case["file"].as_str().unwrap_or_default();
    assert_eq!(
        built.name,
        case["project"].as_str().unwrap_or_default(),
        "{file}"
    );
    let settings = built.document.settings();
    assert_eq!(
        u64::from(settings.srid),
        case["srid"].as_u64().unwrap_or(u64::MAX),
        "{file}: system"
    );
    assert_eq!(
        settings.workspace,
        Some(kentos_commands::Workspace::Gis),
        "{file}"
    );
    let want_notes: Vec<&str> = case["notes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    assert_eq!(codes(built), want_notes, "{file}: notes {:#?}", built.notes);
    let nodes = built.document.layers().nodes();
    let want = case["layers"].as_array().expect("layers");
    assert_eq!(
        nodes.iter().map(|n| n.name.as_str()).collect::<Vec<_>>(),
        want.iter()
            .map(|l| l["name"].as_str().unwrap_or_default())
            .collect::<Vec<_>>(),
        "{file}: layers"
    );
    for (node, w) in nodes.iter().zip(want) {
        if let Some(raster) = w.get("raster") {
            // A tile table: a raster linked to it on a drawing layer of its own, at the finest zoom.
            assert!(
                node.data.is_none(),
                "{file}: {} is a drawing layer",
                node.name
            );
            let rasters: Vec<_> = built
                .document
                .entities()
                .filter(|e| e.base().layer_id == node.id)
                .filter_map(|e| match e {
                    kentos_commands::Entity::Raster(r) => Some(r.raster.clone()),
                    _ => None,
                })
                .collect();
            assert_eq!(rasters.len(), 1, "{file}: {}'s raster", node.name);
            let tiles = rasters[0]
                .tiles
                .as_ref()
                .unwrap_or_else(|| panic!("{file}: linked to its table"));
            assert_eq!(
                tiles.table,
                raster["table"].as_str().unwrap_or_default(),
                "{file}"
            );
            assert_eq!(
                u64::from(tiles.zoom),
                raster["zoom"].as_u64().unwrap_or(u64::MAX),
                "{file}: the finest zoom"
            );
            assert!(
                rasters[0]
                    .file
                    .as_deref()
                    .is_some_and(|f| f.ends_with(file)),
                "{file}: {:?}",
                rasters[0].file
            );
            continue;
        }
        match w["table"].as_str() {
            None => {
                assert!(
                    node.data.is_none(),
                    "{file}: {} is a drawing layer",
                    node.name
                );
                assert_eq!(node.id, gpkg_open::DRAFT_LAYER, "{file}");
            }
            Some(table) => {
                let data = node
                    .data
                    .as_ref()
                    .unwrap_or_else(|| panic!("{file}: {} is a data layer", node.name));
                assert_eq!(data.driver, "gpkg", "{file}");
                assert_eq!(data.table, table, "{file}");
                assert_eq!(
                    data.key,
                    w["key"].as_str().unwrap_or_default(),
                    "{file}: {table}"
                );
                match &w["geometry"] {
                    Value::Null => {
                        assert!(data.geometry.is_none(), "{file}: {table} has no geometry")
                    }
                    g => {
                        let have = data
                            .geometry
                            .as_ref()
                            .unwrap_or_else(|| panic!("{file}: {table}'s geometry"));
                        assert_eq!(
                            have.column,
                            g["column"].as_str().unwrap_or_default(),
                            "{file}: {table}"
                        );
                        assert_eq!(
                            have.type_name,
                            g["type"].as_str().unwrap_or_default(),
                            "{file}: {table}"
                        );
                        assert_eq!(
                            have.srid.map(u64::from),
                            g["srid"].as_u64(),
                            "{file}: {table}'s system"
                        );
                    }
                }
                let fields: Vec<&str> = node.fields.iter().map(|f| f.name.as_str()).collect();
                let want_fields: Vec<&str> = w["fields"]
                    .as_array()
                    .expect("fields")
                    .iter()
                    .filter_map(Value::as_str)
                    .collect();
                assert_eq!(fields, want_fields, "{file}: {table}'s fields");
                assert_eq!(
                    node.style
                        .renderer
                        .as_ref()
                        .and_then(|r| r["type"].as_str()),
                    w["styled"].as_str(),
                    "{file}: {table}'s look"
                );
            }
        }
    }
    assert_eq!(
        built.document.layers().active(),
        gpkg_open::DRAFT_LAYER,
        "{file}"
    );
    if let Some(home) = case["home"].as_array() {
        let h = built
            .document
            .home_view()
            .unwrap_or_else(|| panic!("{file}: a first view"));
        let got = [h.min_x, h.min_y, h.max_x, h.max_y];
        for (g, w) in got.iter().zip(home) {
            assert!(
                (g - w.as_f64().unwrap_or(f64::NAN)).abs() < 1e-6,
                "{file}: first view {got:?} ≠ {home:?}"
            );
        }
    }
}

#[test]
fn every_case_opens_as_its_file_says() {
    kentos_data_gpkg::register();
    let all = cases();
    assert!(all.len() >= 12, "{} cases", all.len());
    for case in &all {
        let file = case["file"].as_str().expect("file");
        let path = copy(file, "case");
        let before = state(&path);
        let opened = gpkg_open::open(&path, None);
        match case["opens"].as_str() {
            Some("built") => {
                let Ok(Opened::Built(built)) = opened else {
                    panic!("{file}: not built");
                };
                check_built(case, &built);
                assert!(
                    !built.document.is_dirty(),
                    "{file}: nothing the owner did waits to be saved"
                );
                // Opened, its rows looked at, closed: not a byte of the file changed, no table or index added.
                let _ = look_at(&built);
                assert_eq!(
                    state(&path),
                    before,
                    "{file}: opening and looking wrote into the file"
                );
                let address = gpkg_open::address_of(&path);
                assert!(
                    kentos_native_application::project_store::list(&address, None)
                        .expect("listed")
                        .is_empty()
                );
                // The owner keeps it there: opened again, the file gives that project.
                gpkg_open::keep(&built.document, &path, &built.name)
                    .unwrap_or_else(|why| panic!("{file}: not kept: {why}"));
                match gpkg_open::open(&path, None) {
                    Ok(Opened::Kept { name, bytes }) => {
                        assert_eq!(name, built.name, "{file}");
                        let snapshot = kentos_kcad::decode(&bytes).expect("its bytes read");
                        assert_eq!(
                            snapshot
                                .layers
                                .iter()
                                .map(|n| n.name.as_str())
                                .collect::<Vec<_>>(),
                            built
                                .document
                                .layers()
                                .nodes()
                                .iter()
                                .map(|n| n.name.as_str())
                                .collect::<Vec<_>>(),
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
                let kept =
                    kentos_native_application::project_store::list(&address, None).expect("listed");
                assert_eq!(kept.len(), 1, "{file}: no project is added");
                assert_eq!(
                    state(&path),
                    before,
                    "{file}: opening its project wrote into the file"
                );
            }
            Some("several") => {
                let Ok(Opened::Several(kept)) = opened else {
                    panic!("{file}: the projects are not named");
                };
                let names: Vec<&str> = kept.iter().map(|p| p.name.as_str()).collect();
                let want: Vec<&str> = case["projects"]
                    .as_array()
                    .expect("projects")
                    .iter()
                    .filter_map(Value::as_str)
                    .collect();
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
    assert!(
        missing.contains("“Bağ” adında proje yok") && missing.contains("“Çamlık”, “Zeytinlik”"),
        "{missing}"
    );
    let empty = copy("katmanlar.gpkg", "named");
    match gpkg_open::open(&empty, Some("Sivas kuyuları")) {
        Ok(Opened::Built(built)) => {
            assert_eq!(built.name, "Sivas kuyuları");
            let address = gpkg_open::address_of(&empty);
            assert!(
                kentos_native_application::project_store::list(&address, None)
                    .expect("listed")
                    .is_empty(),
                "kept nowhere yet"
            );
        }
        _ => panic!("built under the name asked for"),
    }
}

#[test]
fn a_file_that_cannot_be_written_opens_as_any_other_and_keeping_it_there_says_why_not() {
    use std::os::unix::fs::PermissionsExt;
    kentos_data_gpkg::register();
    let path = copy("katmanlar.gpkg", "readonly");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o444)).expect("read only");
    let before = state(&path);
    let opened = gpkg_open::open(&path, None);
    let Ok(Opened::Built(built)) = opened else {
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644));
        panic!("a read-only file opens");
    };
    assert!(
        !built.document.is_dirty(),
        "opened as any file is: nothing waits to be saved"
    );
    assert_eq!(
        built.document.layers().nodes().len(),
        6,
        "its tables are its layers"
    );
    assert!(look_at(&built) > 0, "its rows read");
    let kept = gpkg_open::keep(&built.document, &path, &built.name);
    let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644));
    let why = kept.expect_err("a read-only file keeps no project");
    assert!(why.contains("salt okunur"), "{why}");
    assert_eq!(state(&path), before, "the file as it was");
}

#[test]
fn a_raster_of_the_file_is_kept_by_its_name_and_found_where_the_file_moved() {
    kentos_data_gpkg::register();
    let path = copy("ortofotolu.gpkg", "moved");
    let Ok(Opened::Built(built)) = gpkg_open::open(&path, None) else {
        panic!("built");
    };
    gpkg_open::keep(&built.document, &path, &built.name).expect("kept by the owner's save");
    let address = gpkg_open::address_of(&path);
    let bytes =
        kentos_native_application::project_store::read(&address, None, "ortofotolu").expect("kept");
    let file_of = |snapshot: &kentos_commands::DocumentSnapshotV2| {
        snapshot.entities.iter().find_map(|e| match e {
            kentos_commands::Entity::Raster(r) => r.raster.file.clone(),
            _ => None,
        })
    };
    // Kept in the file: the raster names its own file by its name.
    let raw = kentos_kcad::decode(&bytes).expect("its bytes");
    assert_eq!(file_of(&raw).as_deref(), Some("ortofotolu.gpkg"));
    // The file moved to another folder: read from there, the raster's file is that one.
    let folder =
        std::env::temp_dir().join(format!("kentos-gpkg-open-{}-elsewhere", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).expect("a folder");
    let moved = folder.join("ortofotolu.gpkg");
    std::fs::copy(&path, &moved).expect("moved");
    let moved_address = gpkg_open::address_of(&moved);
    let Ok(Opened::Kept { bytes, .. }) = gpkg_open::open(&moved, None) else {
        panic!("the kept project");
    };
    let read =
        kentos_native_application::project_store::decode(&moved_address, &bytes).expect("decoded");
    assert_eq!(
        file_of(&read),
        Some(moved_address.clone()),
        "absolute again, from where the file is now"
    );
}

/// The tables' style rows: table, name, whether default, description.
fn style_rows(path: &std::path::Path) -> Vec<(String, String, bool, String)> {
    let gp = kentos_geopackage::GeoPackage::open(path, false).expect("the file");
    let mut st = gp
        .connection()
        .prepare("SELECT f_table_name, styleName, useAsDefault, coalesce(description, '') FROM layer_styles ORDER BY id")
        .expect("the style table");
    st.query_map([], |r| {
        Ok((r.get(0)?, r.get(1)?, r.get::<_, bool>(2)?, r.get(3)?))
    })
    .expect("rows")
    .collect::<Result<_, _>>()
    .expect("read")
}

#[test]
fn a_changed_look_becomes_its_tables_default_style_and_an_unchanged_one_is_not_written() {
    kentos_data_gpkg::register();
    let path = copy("katmanlar.gpkg", "looks");
    let Ok(Opened::Built(built)) = gpkg_open::open(&path, None) else {
        panic!("built");
    };
    let mut doc = built.document;
    let before = style_rows(&path);
    assert_eq!(before.len(), 3, "{before:?}");
    // Opened as the file says: nothing to write.
    let looks = kentos_native_application::gpkg_looks::write(&doc, &path).expect("written");
    assert!(
        looks.written.is_empty() && looks.left.is_empty(),
        "{looks:?}"
    );
    assert_eq!(style_rows(&path), before);

    // binalar drawn in one colour now: its table's new default; the file's earlier styles stay, none default.
    let binalar = doc
        .layers()
        .nodes()
        .iter()
        .find(|n| n.name == "binalar")
        .map(|n| n.id.clone())
        .expect("binalar");
    let single = serde_json::json!({ "type": "single", "symbols": { "fill": { "type": "fill", "layers": [
        { "id": "f", "type": "simpleFill", "color": "#2a6fdb" },
        { "id": "o", "type": "simpleLine", "color": "#123456", "width": 0.5 }
    ] } } });
    let done = kentos_native_application::layers_renderer::execute(
        &mut kentos_native_application::ExecutionContext::new(&mut doc),
        kentos_commands::LayersRenderer {
            layer: binalar,
            renderer: Some(single),
            expected_revision: None,
        },
    );
    assert!(
        matches!(done, kentos_commands::CommandResult::Completed { .. }),
        "{done:?}"
    );
    let looks = kentos_native_application::gpkg_looks::write(&doc, &path).expect("written");
    assert_eq!(looks.written, ["binalar"], "{looks:?}");
    let after = style_rows(&path);
    assert_eq!(after.len(), 4, "{after:?}");
    let defaults: Vec<_> = after.iter().filter(|r| r.0 == "binalar" && r.2).collect();
    assert_eq!(defaults.len(), 1, "one default: {after:?}");
    assert_eq!(defaults[0].3, "KentOS", "{after:?}");
    assert_eq!(
        after.last().map(|r| (r.0.as_str(), r.2)),
        Some(("binalar", true))
    );
    // Written again unchanged: nothing more.
    let again = kentos_native_application::gpkg_looks::write(&doc, &path).expect("written");
    assert!(again.written.is_empty(), "{again:?}");
    assert_eq!(style_rows(&path).len(), 4);
}
