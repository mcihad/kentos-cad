//! The GeoPackage driver answers the conformance suite's questions (docs/adr/9100 §8), on a file it made itself.

use std::path::PathBuf;

use kentos_data::{Driver, conformance};
use kentos_data_gpkg::Gpkg;

fn fresh(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("kentos-data-gpkg");
    std::fs::create_dir_all(&dir).expect("dir");
    let path = dir.join(format!("{name}-{}.gpkg", std::process::id()));
    let _ = std::fs::remove_file(&path);
    kentos_geopackage::GeoPackage::create(&path).expect("create");
    path
}

#[test]
fn the_driver_passes_the_conformance_suite() {
    let path = fresh("uygunluk");
    let source = Gpkg.open(path.to_str().expect("utf-8")).expect("open");
    let report = conformance::run(source.as_ref(), "parseller", 5256).expect("run");
    assert!(
        report.failures.is_empty(),
        "{} of {} checks failed:\n{}",
        report.failures.len(),
        report.checks,
        report.failures.join("\n")
    );
    assert!(report.checks > 80, "{} checks", report.checks);
    // The catalog says the table without reading its rows.
    let tables = source.tables().expect("tables");
    let t = tables
        .iter()
        .find(|t| t.name == "parseller")
        .expect("table");
    assert_eq!(t.rows, Some(conformance::N as u64));
    assert!(t.writable);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn a_second_run_replaces_the_table_and_the_spatial_index_follows_edits() {
    let path = fresh("yeniden");
    let source = Gpkg.open(path.to_str().expect("utf-8")).expect("open");
    for _ in 0..2 {
        let report = conformance::run(source.as_ref(), "parseller", 5256).expect("run");
        assert!(report.failures.is_empty(), "{}", report.failures.join("\n"));
    }
    let _ = std::fs::remove_file(&path);
}

#[test]
fn a_new_table_writes_its_system_by_name_and_definition() {
    let path = fresh("sistem");
    let source = Gpkg.open(path.to_str().expect("utf-8")).expect("open");
    let mut table = conformance::table("parseller", 5256);
    table.system = Some(kentos_data::SystemText {
        name: "TUREF / TM36".into(),
        wkt: "PROJCS[\"TUREF / TM36\",GEOGCS[\"TUREF\"]]".into(),
    });
    let mut tx = source.begin().expect("begin");
    tx.create_table(&table, false).expect("table");
    // A second table in the same system adds no second row.
    table.name = "parseller_2".into();
    tx.create_table(&table, false).expect("second table");
    tx.commit().expect("commit");
    let conn = rusqlite::Connection::open(&path).expect("file");
    let rows: Vec<(String, String, String, i64)> = conn
        .prepare("SELECT srs_name, organization, definition, organization_coordsys_id FROM gpkg_spatial_ref_sys WHERE srs_id = 5256")
        .expect("query")
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
        .expect("rows")
        .collect::<Result<_, _>>()
        .expect("read");
    assert_eq!(
        rows,
        vec![("TUREF / TM36".into(), "EPSG".into(), "PROJCS[\"TUREF / TM36\",GEOGCS[\"TUREF\"]]".into(), 5256)]
    );
    // The second table, titled as the first, is listed under a name of its own (the identifier is unique).
    let ids: Vec<(String, String)> = conn
        .prepare("SELECT table_name, identifier FROM gpkg_contents ORDER BY table_name")
        .expect("query")
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .expect("rows")
        .collect::<Result<_, _>>()
        .expect("read");
    assert_eq!(
        ids,
        vec![
            ("parseller".into(), "Uygunluk parselleri".into()),
            ("parseller_2".into(), "Uygunluk parselleri (parseller_2)".into())
        ]
    );
    drop(conn);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn a_table_without_a_system_text_is_named_by_its_code() {
    let path = fresh("kod");
    let source = Gpkg.open(path.to_str().expect("utf-8")).expect("open");
    let mut tx = source.begin().expect("begin");
    tx.create_table(&conformance::table("parseller", 2320), false).expect("table");
    tx.commit().expect("commit");
    let conn = rusqlite::Connection::open(&path).expect("file");
    let row: (String, String) = conn
        .query_row("SELECT srs_name, definition FROM gpkg_spatial_ref_sys WHERE srs_id = 2320", [], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })
        .expect("row");
    assert_eq!(row, ("EPSG:2320".into(), "undefined".into()));
    drop(conn);
    let _ = std::fs::remove_file(&path);
}
