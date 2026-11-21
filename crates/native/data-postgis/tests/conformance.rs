//! The PostGIS driver answers the conformance suite's questions (docs/adr/9100 §8) on a database named by
//! `KENTOS_DATA_PG` (`postgresql://user@host:port/db`, its password in `PGPASSWORD`); without it the test says so and
//! passes, as every database test of the workspace does on a machine without one.

use kentos_data::{Driver, conformance};
use kentos_data_postgis::PostGis;

fn address() -> Option<String> {
    std::env::var("KENTOS_DATA_PG")
        .ok()
        .filter(|a| !a.is_empty())
}

#[test]
fn the_driver_passes_the_conformance_suite() {
    let Some(address) = address() else {
        eprintln!("KENTOS_DATA_PG yok: PostGIS uygunluk takımı atlandı");
        return;
    };
    let source = PostGis.open(&address).expect("open");
    let report = conformance::run(source.as_ref(), "public.kentos_uygunluk", 5256).expect("run");
    assert!(
        report.failures.is_empty(),
        "{} of {} checks failed:\n{}",
        report.failures.len(),
        report.checks,
        report.failures.join("\n")
    );
    assert!(report.checks > 80, "{} checks", report.checks);
    let tables = source.tables().expect("tables");
    let t = tables
        .iter()
        .find(|t| t.name == "public.kentos_uygunluk")
        .expect("table in the catalog");
    assert!(t.writable);
    assert_eq!(t.geometry.as_ref().and_then(|g| g.srid), Some(5256));
    let mut tx = source.begin().expect("begin");
    tx.drop_table("public.kentos_uygunluk").expect("drop");
    tx.commit().expect("commit");
}

#[test]
fn a_password_in_the_address_is_refused() {
    let e = PostGis
        .open("postgresql://ali:gizli@127.0.0.1:1/kent")
        .err()
        .expect("refused");
    assert!(e.to_string().contains("parola"), "{e}");
}

#[test]
fn the_database_keeps_projects_in_a_schema() {
    let Some(address) = address() else {
        eprintln!("KENTOS_DATA_PG yok: PostGIS proje deposu atlandı");
        return;
    };
    let source = PostGis.open(&address).expect("open");
    let store = source.project_store().expect("a database keeps projects");
    let report = conformance::projects(store, Some("public")).expect("projects");
    assert!(report.failures.is_empty(), "{} of {}:\n{}", report.failures.len(), report.checks, report.failures.join("\n"));
    assert!(report.checks >= 14, "{} checks", report.checks);
    // The projects table is not a layer of the catalog.
    let tables = source.tables().expect("tables");
    assert!(tables.iter().all(|t| t.name != "public.kentos_projects"), "the projects table listed as a layer");
}
