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
