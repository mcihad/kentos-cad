//! The municipal report's 6th item, the rest (docs/adr/9191): `file.export.gpkg` written into a GeoPackage that is
//! there. `write: tables` writes only its tables again and keeps the file's others (a relation of a table written
//! again goes with its old rows); `write: append` adds the rows to the file's table of the same name, an object whose
//! lasting id the table has not added again, a field the table has no column for left out and said; a table that
//! cannot hold the rows is refused with the file as it was; a delivery stopped between two batches writes nothing.
//! The counts are read back by SQLite and, where the machine has it, by GDAL's `ogrinfo`.

use std::cell::Cell;
use std::path::{Path, PathBuf};

use kentos_geopackage as gpkg;
use kentos_headless::{Op, Session};
use serde_json::{Value, json};

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/kcad/v2/sheets.kcad")
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("kentos-gpkg-into-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder");
    dir
}

/// `n` points on the fixture's active layer, from `from` on, each named; their lasting ids.
fn points(s: &mut Session, from: u32, n: u32, attrs: Value) -> Vec<String> {
    let layer = s.summary()["activeLayer"]
        .as_str()
        .expect("a layer")
        .to_owned();
    let objects: Vec<Value> = (from..from + n)
        .map(|i| {
            let mut a = attrs.clone();
            a["ad"] = json!(format!("Durak {i}"));
            json!({"geometry": {"kind": "point", "p": {"x": 500000.0 + f64::from(i), "y": 4400000.0 + f64::from(i)}}, "attrs": a})
        })
        .collect();
    let answer = s
        .run(
            "cad.entities.create",
            None,
            Op::Execute,
            json!({"layerId": layer, "objects": objects}),
        )
        .expect("runs");
    assert_eq!(answer["status"], "completed", "{answer}");
    answer["output"]["created"]
        .as_array()
        .expect("created")
        .iter()
        .map(|u| u.as_str().expect("an id").to_owned())
        .collect()
}

fn export(s: &mut Session, input: Value) -> Value {
    s.run("file.export.gpkg", None, Op::Execute, input)
        .expect("answers")
}

/// The rows of `table` in the file at `path`, by SQLite.
fn rows(path: &Path, table: &str) -> i64 {
    let gp = gpkg::GeoPackage::open(path, false).expect("opens");
    gp.connection()
        .query_row(
            &format!("SELECT count(*) FROM {}", gpkg::quote(table)),
            [],
            |r| r.get(0),
        )
        .expect("counted")
}

/// The rows of `table` as GDAL reads them; none where the machine has no `ogrinfo`.
fn ogr_rows(path: &Path, table: &str) -> Option<i64> {
    let out = std::process::Command::new("ogrinfo")
        .args([
            "-ro",
            "-q",
            "-sql",
            &format!("SELECT COUNT(*) AS n FROM \"{table}\""),
            &path.display().to_string(),
        ])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    let n = text
        .lines()
        .find_map(|l| l.trim().strip_prefix("n (Integer) = ").map(str::to_owned))?;
    n.trim().parse().ok()
}

/// A point table of the file's own beside the delivery's, with one row.
fn foreign_table(path: &Path, name: &str) {
    let mut gp = gpkg::GeoPackage::open(path, true).expect("opens");
    let session = gp.session().expect("a session");
    let mut w = session
        .start_table(&gpkg::TableDef {
            name: name.into(),
            data_type: gpkg::DataType::Attributes,
            identifier: Some(name.into()),
            description: None,
            srs_id: None,
            geometry: None,
            columns: vec![gpkg::Column {
                name: "not".into(),
                decl: String::new(),
                kind: gpkg::ColumnKind::Text(None),
                not_null: false,
                default: None,
            }],
        })
        .expect("started");
    session
        .append(
            &mut w,
            &[gpkg::NewRow {
                fid: None,
                geometry: None,
                values: vec![gpkg::Value::Text("eski".into())],
            }],
            None,
        )
        .expect("a row");
    session.finish_table(w).expect("finished");
    session.commit().expect("committed");
    gp.close().expect("closed");
}

/// The delivery's table taken out of the file and made again as `def` says (no rows): a table another program made.
fn table_instead(path: &Path, def: gpkg::TableDef) {
    let mut gp = gpkg::GeoPackage::open(path, true).expect("opens");
    let session = gp.session().expect("a session");
    session.drop_table(&def.name).expect("dropped");
    let w = session.start_table(&def).expect("started");
    session.finish_table(w).expect("finished");
    session.commit().expect("committed");
    gp.close().expect("closed");
}

#[test]
fn only_these_tables_are_written_again_and_the_files_others_stay() {
    let dir = scratch("tables");
    let out = dir.join("teslim.gpkg");
    let path = out.display().to_string();
    let mut s = Session::open(&fixture()).expect("the fixture opens");
    let first = points(&mut s, 0, 5, json!({}));
    let made = export(
        &mut s,
        json!({"path": path, "uids": first, "overwrite": true}),
    );
    assert_eq!(made["status"], "completed", "{made}");
    let table = made["output"]["tables"][0]["table"]
        .as_str()
        .expect("its table")
        .to_owned();
    // The file's own table, and a relation between it and the delivery's table.
    foreign_table(&out, "eski_tablo");
    // Another program's table without an integer key, a row of it deleted: its rows' ids are its rows' identity.
    {
        let gp = gpkg::GeoPackage::open(&out, true).expect("opens");
        gp.connection()
            .execute_batch(
                "CREATE TABLE notlar (metin TEXT);
                 INSERT INTO notlar VALUES ('bir'), ('iki'), ('üç');
                 DELETE FROM notlar WHERE metin = 'iki';
                 INSERT INTO gpkg_contents (table_name, data_type, identifier) VALUES ('notlar', 'attributes', 'notlar');",
            )
            .expect("a table");
        gp.close().expect("closed");
    }
    {
        let mut gp = gpkg::GeoPackage::open(&out, true).expect("opens");
        let layers = gp.layers().expect("tables");
        let base = layers
            .iter()
            .find(|l| l.name == table)
            .expect("the delivery's")
            .clone();
        let related = layers
            .iter()
            .find(|l| l.name == "eski_tablo")
            .expect("the file's")
            .clone();
        gp.add_relation(&base, &related, "teslim_eski", &[(1, 1)])
            .expect("a relation");
        gp.close().expect("closed");
    }
    let more = points(&mut s, 5, 2, json!({}));
    let uids: Vec<String> = first.iter().chain(&more).cloned().collect();
    let again = export(
        &mut s,
        json!({"path": path, "uids": uids, "overwrite": true, "write": "tables"}),
    );
    assert_eq!(again["status"], "completed", "{again}");
    let o = &again["output"];
    assert_eq!(o["tables"][0]["replaced"], true, "{o}");
    assert_eq!(o["tables"][0]["rows"], 7, "{o}");
    assert_eq!(o["kept"], json!(["eski_tablo", "notlar"]), "{o}");
    assert_eq!(rows(&out, &table), 7);
    assert_eq!(
        rows(&out, "eski_tablo"),
        1,
        "the file's own table is as it was"
    );
    // Copied page for page: the table without an integer key keeps its rows' ids.
    {
        let gp = gpkg::GeoPackage::open(&out, false).expect("opens");
        let ids: Vec<(i64, String)> = {
            let mut st = gp
                .connection()
                .prepare("SELECT rowid, metin FROM notlar ORDER BY rowid")
                .expect("a query");
            st.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
                .expect("rows")
                .map(|r| r.expect("a row"))
                .collect()
        };
        assert_eq!(ids, [(1, "bir".to_owned()), (3, "üç".to_owned())]);
    }
    // The relation of the table written again went, and was said.
    let gp = gpkg::GeoPackage::open(&out, false).expect("opens");
    assert!(gp.relations().expect("relations").is_empty());
    let check: String = gp
        .connection()
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .expect("checked");
    assert_eq!(check, "ok");
    drop(gp);
    let said: Vec<String> = again["warnings"]
        .as_array()
        .expect("warnings")
        .iter()
        .map(|w| w["message"].as_str().unwrap_or("").to_owned())
        .collect();
    assert!(
        said.iter()
            .any(|w| w.contains("ilişkisi çıktı") && w.contains("eski_tablo")),
        "{said:#?}"
    );
    match (ogr_rows(&out, &table), ogr_rows(&out, "eski_tablo")) {
        (Some(a), Some(b)) => assert_eq!((a, b), (7, 1), "GDAL reads the same rows"),
        _ => eprintln!("ogrinfo yok: GDAL'ın okuması atlandı"),
    }
    // A file not there: made new whatever `write` says.
    let fresh = dir.join("yeni.gpkg");
    let made = export(
        &mut s,
        json!({"path": fresh.display().to_string(), "uids": first, "write": "append"}),
    );
    assert_eq!(made["status"], "completed", "{made}");
    assert_eq!(
        made["output"]["tables"][0]["appended"],
        Value::Null,
        "{made}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn rows_are_added_to_the_files_table_and_an_object_it_has_is_not_added_again() {
    let dir = scratch("append");
    let out = dir.join("teslim.gpkg");
    let path = out.display().to_string();
    let mut s = Session::open(&fixture()).expect("the fixture opens");
    let first = points(&mut s, 0, 5, json!({}));
    let made = export(
        &mut s,
        json!({"path": path, "uids": first, "overwrite": true}),
    );
    assert_eq!(made["status"], "completed", "{made}");
    let table = made["output"]["tables"][0]["table"]
        .as_str()
        .expect("its table")
        .to_owned();
    foreign_table(&out, "eski_tablo");
    // Three new points, one field the table has no column for.
    let more = points(&mut s, 5, 3, json!({"yeni_alan": "x"}));
    let uids: Vec<String> = first.iter().chain(&more).cloned().collect();
    let added = export(
        &mut s,
        json!({"path": path, "uids": uids, "overwrite": true, "write": "append"}),
    );
    assert_eq!(added["status"], "completed", "{added}");
    let t = &added["output"]["tables"][0];
    assert_eq!(
        (
            t["appended"].clone(),
            t["rows"].clone(),
            t["already"].clone()
        ),
        (json!(true), json!(3), json!(5)),
        "{t}"
    );
    assert_eq!(added["output"]["kept"], json!(["eski_tablo"]));
    let said: Vec<String> = added["warnings"]
        .as_array()
        .expect("warnings")
        .iter()
        .map(|w| w["message"].as_str().unwrap_or("").to_owned())
        .collect();
    assert!(
        said.iter()
            .any(|w| w.contains("sütunu olmayan alanlar yazılmadı: yeni_alan")),
        "{said:#?}"
    );
    assert!(
        said.iter().any(|w| w.contains("5 nesne zaten vardı")),
        "{said:#?}"
    );
    assert_eq!(rows(&out, &table), 8);
    assert_eq!(rows(&out, "eski_tablo"), 1);
    {
        let gp = gpkg::GeoPackage::open(&out, false).expect("opens");
        let c = gp.connection();
        // Each row its own lasting id; the new rows found by the spatial index; the box grown to them.
        let distinct: i64 = c
            .query_row(
                "SELECT count(DISTINCT uid) FROM kentos_uids WHERE table_name = ?1",
                [&table],
                |r| r.get(0),
            )
            .expect("counted");
        assert_eq!(distinct, 8);
        let layer = gp
            .layers()
            .expect("tables")
            .into_iter()
            .find(|l| l.name == table)
            .expect("the table");
        let column = layer
            .geometry
            .as_ref()
            .expect("a geometry column")
            .name
            .clone();
        let indexed: i64 = c
            .query_row(
                &format!(
                    "SELECT count(*) FROM {}",
                    gpkg::quote(&format!("rtree_{table}_{column}"))
                ),
                [],
                |r| r.get(0),
            )
            .expect("the index");
        assert_eq!(indexed, 8, "the file's triggers kept its spatial index");
        let max_x = layer.bounds.expect("a box")[2];
        assert!(max_x >= 500007.0, "{max_x}");
        let names: Vec<String> = {
            let mut st = c
                .prepare(&format!(
                    "SELECT ad FROM {} ORDER BY fid",
                    gpkg::quote(&table)
                ))
                .expect("a query");
            st.query_map([], |r| r.get::<_, String>(0))
                .expect("rows")
                .map(|r| r.expect("a row"))
                .collect()
        };
        assert_eq!(names.last().map(String::as_str), Some("Durak 7"));
        let check: String = c
            .query_row("PRAGMA integrity_check", [], |r| r.get(0))
            .expect("checked");
        assert_eq!(check, "ok");
    }
    match ogr_rows(&out, &table) {
        Some(n) => assert_eq!(n, 8, "GDAL reads the added rows"),
        None => eprintln!("ogrinfo yok: GDAL'ın okuması atlandı"),
    }
    // Again: every object is there, nothing added.
    let none = export(
        &mut s,
        json!({"path": path, "uids": uids, "overwrite": true, "write": "append"}),
    );
    assert_eq!(none["status"], "completed", "{none}");
    assert_eq!(none["output"]["tables"][0]["rows"], 0, "{none}");
    assert_eq!(none["output"]["tables"][0]["already"], 8, "{none}");
    assert_eq!(rows(&out, &table), 8);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_table_that_cannot_hold_the_rows_is_refused_and_the_file_stays() {
    let dir = scratch("unfit");
    let out = dir.join("teslim.gpkg");
    let path = out.display().to_string();
    let mut s = Session::open(&fixture()).expect("the fixture opens");
    let srid = s.document().settings().srid as i32;
    let first = points(&mut s, 0, 4, json!({}));
    let made = export(
        &mut s,
        json!({"path": path, "uids": first, "overwrite": true}),
    );
    assert_eq!(made["status"], "completed", "{made}");
    let table = made["output"]["tables"][0]["table"]
        .as_str()
        .expect("its table")
        .to_owned();
    let def = |type_name: &str, srs: i32, columns: Vec<gpkg::Column>| gpkg::TableDef {
        name: table.clone(),
        data_type: gpkg::DataType::Features,
        identifier: Some(table.clone()),
        description: None,
        srs_id: Some(srs),
        geometry: Some(gpkg::GeometryColumn {
            name: "geom".into(),
            type_name: type_name.into(),
            z: 2,
            m: 2,
        }),
        columns,
    };
    let required = gpkg::Column {
        name: "zorunlu".into(),
        decl: String::new(),
        kind: gpkg::ColumnKind::Text(None),
        not_null: true,
        default: None,
    };
    for (made_as, code, words) in [
        (
            def("POLYGON", srid, Vec::new()),
            "geometry_mismatch",
            "alan geometri tutuyor",
        ),
        (
            def("GEOMETRY", 4326, Vec::new()),
            "crs_mismatch",
            "EPSG:4326",
        ),
        (
            def("GEOMETRY", srid, vec![required.clone()]),
            "source_not_null",
            "“zorunlu” sütunu boş geçilemez",
        ),
    ] {
        table_instead(&out, made_as);
        let before = std::fs::read(&out).expect("the file");
        let refused = export(
            &mut s,
            json!({"path": path, "uids": first, "overwrite": true, "write": "append"}),
        );
        assert_eq!(refused["error"]["code"], code, "{refused}");
        let message = refused["error"]["message"].as_str().unwrap_or("");
        assert!(
            message.contains(words) && message.contains("Tabloyu yeniden yazmayı seçin"),
            "{message}"
        );
        assert_eq!(
            std::fs::read(&out).expect("the file"),
            before,
            "{code}: the file as it was"
        );
    }
    // Written again instead, the table takes the layer's rows.
    let again = export(
        &mut s,
        json!({"path": path, "uids": first, "overwrite": true, "write": "tables"}),
    );
    assert_eq!(again["status"], "completed", "{again}");
    assert_eq!(rows(&out, &table), 4);
    // A file that does not read as a GeoPackage takes no tables.
    let text = dir.join("metin.gpkg");
    std::fs::write(&text, b"bir GeoPackage degil").expect("a file");
    let refused = export(
        &mut s,
        json!({"path": text.display().to_string(), "uids": first, "overwrite": true, "write": "tables"}),
    );
    assert_eq!(refused["error"]["code"], "file_unwritable", "{refused}");
    assert!(
        refused["error"]["message"]
            .as_str()
            .unwrap_or("")
            .contains("GeoPackage olarak okunamadı"),
        "{refused}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_delivery_stopped_between_two_batches_writes_nothing() {
    let dir = scratch("stop");
    let out = dir.join("teslim.gpkg");
    let mut s = Session::open(&fixture()).expect("the fixture opens");
    let first = points(&mut s, 0, 3, json!({}));
    let made = export(
        &mut s,
        json!({"path": out.display().to_string(), "uids": first, "overwrite": true}),
    );
    assert_eq!(made["status"], "completed", "{made}");
    // More rows than a batch holds (5 000), so the write is asked to stop after its first batch.
    let many = points(&mut s, 3, 12_000, json!({}));
    let uids: Vec<String> = first.iter().chain(&many).cloned().collect();
    let before = std::fs::read(&out).expect("the file");
    for write in ["file", "tables", "append"] {
        let input: kentos_commands::ExportGpkg = serde_json::from_value(
            json!({"path": out.display().to_string(), "uids": uids, "overwrite": true, "write": write}),
        )
        .expect("an input");
        let batches = Cell::new(0u32);
        let stopped = || batches.get() >= 1;
        let progress = |_: u64, _: u64| batches.set(batches.get() + 1);
        let flow = kentos_native_application::geopackage::Flow {
            stopped: &stopped,
            progress: &progress,
        };
        let refused = kentos_headless::file_commands::export_gpkg_flow(s.document(), &input, &flow)
            .expect_err("stopped");
        assert_eq!(refused.code, "cancelled", "{write}: {}", refused.message);
        assert_eq!(batches.get(), 1, "{write}: stopped after its first batch");
        assert_eq!(
            std::fs::read(&out).expect("the file"),
            before,
            "{write}: the file as it was"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}
