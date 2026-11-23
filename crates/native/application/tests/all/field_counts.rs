//! A data layer's field counted by its source (docs/adr/9181), on the report's six maintenance points (bedel 0, 100,
//! 250, NULL, 50, 100; durum acil, planlı, tamam twice each). Each value comes back once with how many rows hold it,
//! as a row read from the table reads it; the rows left out (the ones an edit session holds) are not counted; a field
//! the table does not have is said. The keys of the rows holding some values come back in the table's order, at most
//! as many as asked, many values asked a part at a time. The same holds for a PostGIS table, counted by the server, when
//! `KENTOS_DATA_PG` names a database (`postgresql://user@host:port/db`, its password in `PGPASSWORD`).

use std::collections::{BTreeMap, BTreeSet};

use kentos_commands::{CommandResult, LayerDataOperation, LayersData, Workspace};
use kentos_data::{Coord, Driver, Field, FieldKind, Geometry, GeometryField, NewFeature, NewTable, Value};
use kentos_domain::Document;
use kentos_native_application::field_counts::{field_counts, value_keys};
use kentos_native_application::{ExecutionContext, data_describe, data_write};

/// A `table` of `rows` (kod, bedel, durum) made in `source` (one there before is replaced).
fn fill(source: &dyn kentos_data::Source, table: &str, rows: &[(String, Option<f64>, &str)]) {
    let mut tx = source.begin().expect("begin");
    tx.create_table(
        &NewTable {
            name: table.into(),
            title: None,
            description: None,
            key: "fid".into(),
            fields: vec![
                Field::new("kod", FieldKind::Text { max: Some(8) }),
                Field::new("bedel", FieldKind::Real),
                Field::new("durum", FieldKind::Text { max: Some(16) }),
            ],
            geometry: Some(GeometryField {
                name: "geom".into(),
                type_name: "POINT".into(),
                z: false,
                m: false,
                srid: Some(5255),
            }),
            spatial_index: true,
            system: None,
        },
        true,
    )
    .expect("table");
    let features: Vec<NewFeature> = rows
        .iter()
        .enumerate()
        .map(|(i, (kod, bedel, durum))| NewFeature {
            fid: None,
            geometry: Some(Geometry::Point(Some(Coord::xy(484_000.0 + i as f64 * 10.0, 4_420_000.0)))),
            values: vec![
                ("kod".into(), Value::Text(kod.clone())),
                ("bedel".into(), bedel.map_or(Value::Null, Value::Real)),
                ("durum".into(), Value::Text((*durum).into())),
            ],
        })
        .collect();
    tx.insert(table, &features).expect("rows");
    tx.commit().expect("commit");
}

/// A GeoPackage with a `bakim` table of `rows` (kod, bedel, durum); its path.
fn table(tag: &str, rows: &[(String, Option<f64>, &str)]) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("kentos-alan-sayimi-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder");
    let path = dir.join("bakim.gpkg");
    kentos_geopackage::GeoPackage::create(&path).expect("a file");
    let source = kentos_data_gpkg::Gpkg.open(&path.display().to_string()).expect("open");
    fill(source.as_ref(), "bakim", rows);
    path
}

/// The report's six points.
fn report() -> Vec<(String, Option<f64>, &'static str)> {
    let bedel = [Some(0.0), Some(100.0), Some(250.0), None, Some(50.0), Some(100.0)];
    let durum = ["acil", "acil", "planlı", "planlı", "tamam", "tamam"];
    (0..6).map(|i| (format!("B0{}", i + 1), bedel[i], durum[i])).collect()
}

/// A CBS project with the file's `bakim` as a data layer: the document and the layer's id.
fn project(path: &std::path::Path) -> (Document, String) {
    kentos_data_gpkg::register();
    project_on(&path.display().to_string(), "bakim")
}

/// A CBS project with `table` of the source at `address` as a data layer: the document and the layer's id.
fn project_on(address: &str, table: &str) -> (Document, String) {
    let snapshot = kentos_project::new_project::new_project(&kentos_project::new_project::NewProject {
        name: "Sayım".into(),
        srid: 5255,
        plot_scale: 1000.0,
        workspace: Workspace::Gis,
        drawing_font: kentos_commands::DrawingFont::Barlow,
        province: None,
        drawing_unit: None,
        annotation: None,
        cad_work: None,
        heights: None,
    })
    .expect("a project");
    let mut doc = Document::from_snapshot(snapshot).expect("a document");
    let d = data_describe::describe(address, table).expect("described");
    let input = LayersData {
        operation: LayerDataOperation::Add,
        layer: None,
        name: Some(table.rsplit('.').next().unwrap_or(table).into()),
        parent: None,
        index: None,
        data: Some(d.data.clone()),
        fields: Some(d.fields.clone()),
        expected_revision: None,
    };
    match kentos_native_application::layers_data::execute(&mut ExecutionContext::new(&mut doc), input) {
        CommandResult::Completed { output, .. } => (doc, output.layer),
        other => panic!("not added: {other:?}"),
    }
}

/// The values of `field` with their rows, by the text a row reads them as.
fn tally(groups: impl IntoIterator<Item = (Option<String>, u64)>) -> BTreeMap<Option<String>, u64> {
    let mut out = BTreeMap::new();
    for (text, n) in groups {
        *out.entry(text).or_insert(0) += n;
    }
    out
}

/// The report's points counted by their source: each field as the rows read give it, the empty value last, the
/// rows an edit session holds left out, a field the table does not have said.
fn the_counts_meet_the_rows(doc: &Document, layer: &str) {
    let node = doc.layers().get(layer).expect("the layer");
    let read = data_write::all_records(node, doc.settings(), &BTreeSet::new(), &|| false).expect("read");
    assert_eq!(read.rows.len(), 6);
    for field in ["kod", "bedel", "durum"] {
        let counted = field_counts(node, field, &BTreeSet::new()).expect("counted").expect("by the source");
        let from_source = tally(counted.iter().map(|g| (g.text.clone(), g.rows)));
        let from_rows = tally(read.rows.iter().map(|(_, e)| (e.base().attrs.get(field).cloned(), 1)));
        assert_eq!(from_source, from_rows, "{field}");
    }
    let durum = field_counts(node, "durum", &BTreeSet::new()).expect("counted").expect("by the source");
    let pairs: Vec<(Option<&str>, u64)> = durum.iter().map(|g| (g.text.as_deref(), g.rows)).collect();
    assert_eq!(pairs, [(Some("acil"), 2), (Some("planlı"), 2), (Some("tamam"), 2)]);
    // The empty value last, as the source holds it.
    let bedel = field_counts(node, "bedel", &BTreeSet::new()).expect("counted").expect("by the source");
    let last = bedel.last().expect("a value");
    assert_eq!((last.value.clone(), last.text.clone(), last.rows), (Value::Null, None, 1));
    // The rows an edit session holds are left out: the 3rd (planlı) and the 6th (tamam).
    let left = field_counts(node, "durum", &[3, 6].into()).expect("counted").expect("by the source");
    let pairs: Vec<(Option<&str>, u64)> = left.iter().map(|g| (g.text.as_deref(), g.rows)).collect();
    assert_eq!(pairs, [(Some("acil"), 2), (Some("planlı"), 1), (Some("tamam"), 1)]);
    // A field the table does not have.
    let why = field_counts(node, "yok", &BTreeSet::new()).expect_err("no such field");
    assert!(why.contains("“yok” alanı sayılamadı"), "{why}");
}

/// The keys of the report's rows holding some values: in the table's order, the empty value's row, the rows an edit
/// session holds left out, at most as many as asked.
fn the_keys_come_in_order(doc: &Document, layer: &str) {
    let node = doc.layers().get(layer).expect("the layer");
    let none = BTreeSet::new();
    let text = |t: &str| Value::Text(t.into());
    let keys = value_keys(node, "durum", &[text("tamam"), text("acil")], &none, 10, &|| false).expect("read");
    assert_eq!(keys, (vec![1, 2, 5, 6], false));
    assert_eq!(value_keys(node, "bedel", &[Value::Null], &none, 10, &|| false).expect("read"), (vec![4], false));
    assert_eq!(value_keys(node, "bedel", &[Value::Real(100.0)], &none, 10, &|| false).expect("read"), (vec![2, 6], false));
    // The rows an edit session holds are left out; at most as many as asked, and whether there were more.
    assert_eq!(value_keys(node, "durum", &[text("acil")], &[1].into(), 10, &|| false).expect("read"), (vec![2], false));
    assert_eq!(value_keys(node, "durum", &[text("tamam"), text("acil")], &none, 3, &|| false).expect("read"), (vec![1, 2, 5], true));
}

/// 1 200 rows with a value each: every value counted, the keys of all of them asked a part at a time.
fn many_values_are_asked(doc: &Document, layer: &str) {
    let node = doc.layers().get(layer).expect("the layer");
    let kod = field_counts(node, "kod", &BTreeSet::new()).expect("counted").expect("by the source");
    assert_eq!(kod.len(), 1200);
    let values: Vec<Value> = kod.iter().map(|g| g.value.clone()).collect();
    let (keys, more) = value_keys(node, "kod", &values, &BTreeSet::new(), 5000, &|| false).expect("read");
    assert_eq!((keys.len(), more), (1200, false));
    assert!(keys.windows(2).all(|w| w[0] < w[1]), "in the table's order");
    let bedel = field_counts(node, "bedel", &BTreeSet::new()).expect("counted").expect("by the source");
    assert_eq!(bedel.iter().map(|g| g.rows).sum::<u64>(), 1200);
    assert_eq!(bedel.len(), 7);
}

/// The 1 200 rows of [`many_values_are_asked`].
fn many() -> Vec<(String, Option<f64>, &'static str)> {
    (0..1200).map(|i| (format!("K{i:04}"), Some(f64::from(i % 7)), if i % 2 == 0 { "acil" } else { "tamam" })).collect()
}

#[test]
fn a_field_counted_by_its_source_meets_the_rows_read_value_for_value() {
    let path = table("bulusur", &report());
    let (doc, layer) = project(&path);
    the_counts_meet_the_rows(&doc, &layer);
}

#[test]
fn the_keys_of_counted_values_come_in_the_tables_order() {
    let path = table("anahtar", &report());
    let (doc, layer) = project(&path);
    the_keys_come_in_order(&doc, &layer);
}

#[test]
fn many_values_are_asked_a_part_at_a_time() {
    let path = table("parcali", &many());
    let (doc, layer) = project(&path);
    many_values_are_asked(&doc, &layer);
}

#[test]
fn a_postgis_table_is_counted_by_the_server_alike() {
    let Some(address) = std::env::var("KENTOS_DATA_PG").ok().filter(|a| !a.is_empty()) else {
        eprintln!("KENTOS_DATA_PG yok: PostGIS alan sayımı atlandı");
        return;
    };
    kentos_data_postgis::register();
    let source = kentos_data_postgis::PostGis.open(&address).expect("open");
    let (a, b) = (format!("public.kentos_sayim_{}", std::process::id()), format!("public.kentos_sayim_cok_{}", std::process::id()));
    fill(source.as_ref(), &a, &report());
    fill(source.as_ref(), &b, &many());
    let (doc, layer) = project_on(&address, &a);
    the_counts_meet_the_rows(&doc, &layer);
    the_keys_come_in_order(&doc, &layer);
    let (doc, layer) = project_on(&address, &b);
    many_values_are_asked(&doc, &layer);
    let mut tx = source.begin().expect("begin");
    tx.drop_table(&a).expect("drop");
    tx.drop_table(&b).expect("drop");
    tx.commit().expect("commit");
}
