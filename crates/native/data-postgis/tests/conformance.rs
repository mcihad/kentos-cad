//! The PostGIS driver answers the conformance suite's questions (docs/adr/9100 §8) on a database named by
//! `KENTOS_DATA_PG` (`postgresql://user@host:port/db`, its password in `PGPASSWORD`); without it the test says so and
//! passes, as every database test of the workspace does on a machine without one.

use kentos_data::{Driver, QueryTable, conformance};
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

/// Each right on a table's rows read on its own (docs/adr/9162): the account that made the table may add, change and
/// delete its rows; the catalog's own view may only be read.
#[test]
fn the_rights_on_rows_are_read_each_on_its_own() {
    let Some(address) = address() else {
        eprintln!("KENTOS_DATA_PG yok: satır yetkileri atlandı");
        return;
    };
    let source = PostGis.open(&address).expect("open");
    let mut tx = source.begin().expect("begin");
    tx.create_table(
        &kentos_data::NewTable {
            name: "public.kentos_yetki".into(),
            title: None,
            description: None,
            key: "fid".into(),
            fields: vec![kentos_data::Field::new("ad", kentos_data::FieldKind::Text { max: Some(8) })],
            geometry: None,
            spatial_index: false,
            system: None,
        },
        false,
    )
    .expect("table");
    tx.commit().expect("commit");
    let caps = source.capabilities("public.kentos_yetki").expect("its capabilities");
    assert!(caps.insert && caps.update_values && caps.delete && !caps.update_geometry, "{caps:?}");
    let mut tx = source.begin().expect("begin");
    tx.drop_table("public.kentos_yetki").expect("drop");
    tx.commit().expect("commit");
}

/// A query read as a layer (docs/adr/9113 §2) is only ever run read-only: one that would write (a large object made by
/// a function it calls) is refused when its shape is read, though the transaction it is read in is taken back anyway.
#[test]
fn a_query_layer_is_run_read_only() {
    let Some(address) = address() else {
        eprintln!("KENTOS_DATA_PG yok: PostGIS sorgu katmanı atlandı");
        return;
    };
    let source = PostGis.open(&address).expect("open");
    let point = "SELECT 1::int4 AS fid, ST_SetSRID(ST_MakePoint(0, 0), 5256) AS geom";
    let reading = QueryTable { sql: point.into(), key: "fid".into(), geometry: Some("geom".into()) };
    let schema = source.schema(&reading.text()).expect("a reading query");
    assert_eq!(schema.geometry.as_ref().and_then(|g| g.srid), Some(5256));
    let writing = QueryTable { sql: format!("{point}, lo_create(0) AS nesne"), ..reading.clone() };
    let refused = source.schema(&writing.text()).expect_err("a query that writes");
    assert!(refused.to_string().contains("read-only"), "{refused}");
    // Without a geometry its shape does not run it: counting it, reading its values and its groups do, read-only too.
    let table = QueryTable { geometry: None, ..writing }.text();
    source.schema(&table).expect("its columns, without running it");
    let all = kentos_data::Request::all();
    let counted = source.count(&table, &all, false).expect_err("counted read-only");
    assert!(counted.to_string().contains("read-only"), "{counted}");
    let values = source.unique(&table, "fid", None, 10).expect_err("its values read read-only");
    assert!(values.to_string().contains("read-only"), "{values}");
    let groups = source.groups(&table, "fid", None, 10).expect_err("its groups read read-only");
    assert!(groups.to_string().contains("read-only"), "{groups}");
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

/// Write receipts (docs/adr/9126): set up only when asked, a receipt goes in with the rows in one transaction and is
/// read back by its id; a rolled back transaction leaves none.
#[test]
fn a_receipt_commits_with_its_rows_and_only_with_them() {
    let Some(address) = address() else {
        eprintln!("KENTOS_DATA_PG yok: PostGIS işlem kayıtları atlandı");
        return;
    };
    let source = PostGis.open(&address).expect("open");
    let scope = "public";
    source.set_up_receipts(scope).expect("set up");
    assert!(source.receipts(scope).expect("asked"));
    let id = format!("sinama-{}", std::process::id());
    let receipt = kentos_data::Receipt { id: id.clone(), by: Some("sınama".into()), written: "{\"t\": {\"added\": []}}".into() };
    let mut tx = source.begin().expect("begin");
    tx.receipt(scope, &receipt).expect("written");
    tx.rollback().expect("rolled back");
    assert_eq!(source.receipt(scope, &id).expect("read"), None, "rolled back with its transaction");
    let mut tx = source.begin().expect("begin");
    tx.receipt(scope, &receipt).expect("written");
    tx.commit().expect("committed");
    let back = source.receipt(scope, &id).expect("read").expect("there");
    assert_eq!((back.id, back.by.as_deref()), (id, Some("sınama")));
}
