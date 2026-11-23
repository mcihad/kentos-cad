//! A tool's table written with its columns' kinds (docs/adr/9136): Histogram's class number and count are INTEGER, its
//! bounds and shares REAL, their plain values, not the text the window shows. SQLite itself is asked: each cell's
//! type, a threshold over the count and a numeric order. The window's rows stay the formatted text. When
//! `KENTOS_DATA_PG` names a database (`postgresql://user@host:port/db`, its password in `PGPASSWORD`), the same table is
//! written to PostGIS and its columns' types are read from the catalogue.

use std::path::PathBuf;

use kentos_commands::{CallOp, DocumentSnapshotV1};
use kentos_domain::Document;
use kentos_native_application::{ExecutionContext, registry};
use serde_json::{Value, json};

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures")
}

/// The shared raster drawing, taken to be in its rasters' folder.
fn drawing() -> (Document, PathBuf) {
    let path = fixtures().join("processing/v1/raster-ops/raster-ops.kcad");
    let text = std::fs::read_to_string(fixtures().join("processing/v1/raster-ops.kcad")).expect("reads");
    let doc = Document::from_snapshot(DocumentSnapshotV1::from_json(&text).expect("a drawing")).expect("opens");
    (doc, path)
}

fn histogram_to(sink: Value, drawing: &std::path::Path) -> Value {
    json!({
        "tool": "raster.histogram",
        "parameters": { "input": { "scope": "layer", "layerId": "bolge" }, "bins": 4 },
        "sink": sink,
        "drawing": drawing.display().to_string(),
    })
}

fn run(doc: &mut Document, input: Value) -> Value {
    kentos_processing::command::register();
    registry::run(&mut ExecutionContext::new(doc), "cad.processing.run", CallOp::Execute, input).expect("runs")
}

#[test]
fn a_histogram_goes_to_a_geopackage_as_numbers() {
    let dir = std::env::temp_dir().join(format!("kentos-typed-tables-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder");
    let path = dir.join("histogram.gpkg");
    let (mut doc, at) = drawing();
    let sink = json!({ "path": path.display().to_string(), "mode": "new", "tables": true });
    let got = run(&mut doc, histogram_to(sink, &at));
    assert_eq!(got["status"], "completed", "{got}");
    let shown = &got["output"]["outputs"]["table"];
    // The window's rows are the formatted text; the plain values beside them are numbers.
    assert!(shown["rows"][0].as_array().expect("a row").iter().all(Value::is_string), "{shown}");
    let values = shown["values"].as_array().expect("plain values");
    let counts: Vec<i64> = values.iter().map(|r| r[3].as_i64().expect("a whole count")).collect();
    let table = got["output"]["sinkReport"]["tables"][0]["table"].as_str().expect("a table").to_owned();
    // SQLite's own answer: the types, a threshold over the count, the order by number.
    let gp = kentos_geopackage::GeoPackage::open(&path, false).expect("opens");
    let db = gp.connection();
    let columns = ["Aralık", "Alt sınır", "Üst sınır", "Sayı", "Oran (%)", "Birikimli (%)"];
    let want = ["integer", "real", "real", "integer", "real", "real"];
    for (c, w) in columns.iter().zip(want) {
        let mut st = db.prepare(&format!(r#"SELECT DISTINCT typeof("{c}") FROM "{table}""#)).expect("a query");
        let kinds: Vec<String> = st.query_map([], |r| r.get(0)).expect("rows").map(|r| r.expect("a row")).collect();
        assert_eq!(kinds, [w], "{c}");
    }
    let least = *counts.iter().min().expect("a count");
    let over: i64 = db
        .query_row(&format!(r#"SELECT count(*) FROM "{table}" WHERE "Sayı" > {least}"#), [], |r| r.get(0))
        .expect("a count");
    assert_eq!(over as usize, counts.iter().filter(|c| **c > least).count());
    let mut by_count = counts.clone();
    by_count.sort_unstable_by(|a, b| b.cmp(a));
    let mut st = db.prepare(&format!(r#"SELECT "Sayı" FROM "{table}" ORDER BY "Sayı" DESC"#)).expect("a query");
    let ordered: Vec<i64> = st.query_map([], |r| r.get(0)).expect("rows").map(|r| r.expect("a row")).collect();
    assert_eq!(ordered, by_count);
    drop(st);
    // The shares are not rounded for the eye: they add up to a hundred.
    let total: f64 =
        db.query_row(&format!(r#"SELECT sum("Oran (%)") FROM "{table}""#), [], |r| r.get(0)).expect("a sum");
    assert!((total - 100.0).abs() < 1e-9, "{total}");
    drop(gp);
    // KENTOS_KEEP_TYPED=<path> keeps the file for an outside reader (GDAL's ogrinfo, sqlite3).
    if let Some(keep) = std::env::var_os("KENTOS_KEEP_TYPED") {
        std::fs::copy(&path, keep).expect("kept");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_histogram_goes_to_postgis_with_the_same_kinds() {
    let Some(address) = std::env::var("KENTOS_DATA_PG").ok().filter(|a| !a.is_empty()) else {
        eprintln!("KENTOS_DATA_PG yok: PostGIS sütun türleri durumu atlandı");
        return;
    };
    // The sink writes through the table provider the hosts register, the catalogue is read through the data driver:
    // both registered before the run.
    kentos_postgis::register();
    kentos_data_postgis::register();
    let schema = format!("kentos_tur_{}", std::process::id());
    let (mut doc, at) = drawing();
    let sink = json!({ "path": address, "schema": schema, "mode": "new", "tables": true });
    let got = run(&mut doc, histogram_to(sink, &at));
    assert_eq!(got["status"], "completed", "{got}");
    let table = got["output"]["sinkReport"]["tables"][0]["table"].as_str().expect("a table").to_owned();
    // The table's columns as PostgreSQL's catalogue gives them, through the data driver.
    let source = kentos_data::registry().source(&address).expect("the database");
    // The report names the table with its schema.
    let qualified = if table.contains('.') { table } else { format!("{schema}.{table}") };
    let fields = source.schema(&qualified).expect("the table").fields;
    let kind = |name: &str| fields.iter().find(|f| f.name == name).map(|f| f.kind.clone());
    assert!(
        matches!(kind("Sayı"), Some(kentos_data::FieldKind::Int { .. })),
        "{:?}",
        kind("Sayı")
    );
    assert!(matches!(kind("Alt sınır"), Some(kentos_data::FieldKind::Real)), "{:?}", kind("Alt sınır"));
    // The schema is the test's own; it is left for the database's owner to drop with the others of `kentos_tur_`.
}

/// Özet istatistik's table goes to a GeoPackage typed too (docs/adr/9158): the group text, the counts INTEGER, the
/// figures REAL, an empty standard deviation NULL; the shared query drawing's buildings by their storeys.
#[test]
fn a_summary_goes_to_a_geopackage_as_numbers() {
    let dir = std::env::temp_dir().join(format!("kentos-typed-summary-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder");
    let path = dir.join("ozet.gpkg");
    let text = std::fs::read_to_string(fixtures().join("processing/v1/queries.kcad")).expect("reads");
    let mut doc = Document::from_snapshot(DocumentSnapshotV1::from_json(&text).expect("a drawing")).expect("opens");
    let got = run(
        &mut doc,
        json!({
            "tool": "statistics.summary",
            "parameters": { "input": { "scope": "layer", "layerId": "yapi" }, "field": "Taban alanı", "group": "Kat" },
            "sink": { "path": path.display().to_string(), "mode": "new", "tables": true },
        }),
    );
    assert_eq!(got["status"], "completed", "{got}");
    let table = got["output"]["sinkReport"]["tables"][0]["table"].as_str().expect("a table").to_owned();
    let gp = kentos_geopackage::GeoPackage::open(&path, false).expect("opens");
    let db = gp.connection();
    let kind_of = |column: &str, row: &str| -> String {
        db.query_row(&format!(r#"SELECT typeof("{column}") FROM "{table}" WHERE "Grup" = ?1"#), [row], |r| r.get(0))
            .expect("a cell")
    };
    assert_eq!(kind_of("Grup", "Toplam"), "text");
    assert_eq!(kind_of("Nesne", "Toplam"), "integer");
    assert_eq!(kind_of("Toplam", "Toplam"), "real");
    // Group 1 has one value: no standard deviation, NULL rather than an empty text.
    assert_eq!(kind_of("Std. sapma", "1"), "null");
    let total: f64 = db
        .query_row(&format!(r#"SELECT "Toplam" FROM "{table}" WHERE "Grup" = 'Toplam'"#), [], |r| r.get(0))
        .expect("a number");
    assert!((total - 231.5).abs() < 1e-9, "{total}");
    drop(gp);
    let _ = std::fs::remove_dir_all(&dir);
}

/// The column kinds SQLite holds for `columns` of `table`, one each (an all-NULL column says null).
fn kinds_of(gp: &kentos_geopackage::GeoPackage, table: &str, columns: &[&str]) -> Vec<Vec<String>> {
    columns
        .iter()
        .map(|c| {
            let mut st = gp
                .connection()
                .prepare(&format!(r#"SELECT DISTINCT typeof("{c}") FROM "{table}" WHERE "{c}" IS NOT NULL"#))
                .expect("a query");
            st.query_map([], |r| r.get(0)).expect("rows").map(|r| r.expect("a row")).collect()
        })
        .collect()
}

/// The tools that give neither kinds nor plain values (docs/adr/9190) go to a file typed from the cells they show:
/// Bölgesel istatistik's count INTEGER, its sum, mean and spread REAL, the zone without cells NULL; Uzaklık matrisi's
/// rank INTEGER and distance REAL; the names text. SQLite filters and orders them as numbers.
#[test]
fn zonal_statistics_and_a_distance_matrix_go_to_a_geopackage_typed_from_their_cells() {
    let dir = std::env::temp_dir().join(format!("kentos-typed-tables-zonal-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder");
    // Bölgesel istatistik over the shared raster drawing's zones.
    let path = dir.join("bolgesel.gpkg");
    let (mut doc, at) = drawing();
    let got = run(
        &mut doc,
        json!({
            "tool": "raster.zonalStatistics",
            "parameters": {
                "input": { "scope": "layer", "layerId": "bolge" },
                "zones": { "scope": "layer", "layerId": "bolgeler" },
                "stat": "mean",
                "output": "",
            },
            "sink": { "path": path.display().to_string(), "mode": "new", "tables": true },
            "drawing": at.display().to_string(),
        }),
    );
    assert_eq!(got["status"], "completed", "{got}");
    let table = got["output"]["sinkReport"]["tables"][0]["table"].as_str().expect("a table").to_owned();
    let gp = kentos_geopackage::GeoPackage::open(&path, false).expect("opens");
    let db = gp.connection();
    let columns = ["Nesne", "Sayı", "Toplam", "Ortalama", "En küçük", "En büyük", "Standart sapma"];
    assert_eq!(
        kinds_of(&gp, &table, &columns),
        [["text"], ["integer"], ["real"], ["real"], ["real"], ["real"], ["real"]].map(|k| k.map(str::to_owned).to_vec())
    );
    let over: i64 = db
        .query_row(&format!(r#"SELECT count(*) FROM "{table}" WHERE "Ortalama" > 100"#), [], |r| r.get(0))
        .expect("a count");
    assert_eq!(over, 3, "B1, B2 and B3 over 100; B4 has none");
    let empty: i64 = db
        .query_row(&format!(r#"SELECT count(*) FROM "{table}" WHERE "Ortalama" IS NULL"#), [], |r| r.get(0))
        .expect("a count");
    assert_eq!(empty, 1, "the zone without cells is NULL, not an empty text");
    let mut st = db.prepare(&format!(r#"SELECT "Sayı" FROM "{table}" ORDER BY "Sayı" DESC"#)).expect("a query");
    let counts: Vec<i64> = st.query_map([], |r| r.get(0)).expect("rows").map(|r| r.expect("a row")).collect();
    assert_eq!(counts, [35, 30, 23, 0]);
    drop(st);
    drop(gp);
    // Uzaklık matrisi, Liste: the parcels' two nearest stops.
    let path = dir.join("matris.gpkg");
    let text = std::fs::read_to_string(fixtures().join("processing/v1/proximity.kcad")).expect("reads");
    let mut doc = Document::from_snapshot(DocumentSnapshotV1::from_json(&text).expect("a drawing")).expect("opens");
    let got = run(
        &mut doc,
        json!({
            "tool": "proximity.matrix",
            "parameters": {
                "input": { "scope": "layer", "layerId": "parsel" },
                "targets": { "scope": "layer", "layerId": "durak" },
                "measure": "edges", "k": 2, "max": 0, "form": "list", "name": "Ad", "targetName": "Ad",
            },
            "sink": { "path": path.display().to_string(), "mode": "new", "tables": true },
        }),
    );
    assert_eq!(got["status"], "completed", "{got}");
    let table = got["output"]["sinkReport"]["tables"][0]["table"].as_str().expect("a table").to_owned();
    let gp = kentos_geopackage::GeoPackage::open(&path, false).expect("opens");
    let db = gp.connection();
    assert_eq!(
        kinds_of(&gp, &table, &["Kaynak", "Hedef", "Sıra", "Uzaklık"]),
        [["text"], ["text"], ["integer"], ["real"]].map(|k| k.map(str::to_owned).to_vec())
    );
    let firsts: i64 = db
        .query_row(&format!(r#"SELECT count(*) FROM "{table}" WHERE "Sıra" = 1"#), [], |r| r.get(0))
        .expect("a count");
    assert_eq!(firsts, 7, "one nearest stop for each of the seven parcels");
    drop(gp);
    if let Some(keep) = std::env::var_os("KENTOS_KEEP_TYPED") {
        let keep = std::path::PathBuf::from(keep);
        std::fs::copy(dir.join("bolgesel.gpkg"), keep.with_extension("bolgesel.gpkg")).expect("kept");
        std::fs::copy(dir.join("matris.gpkg"), keep.with_extension("matris.gpkg")).expect("kept");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// Bölgesel istatistik's table typed from its cells goes to PostGIS with the same kinds (docs/adr/9190): the count an
/// integer, the mean a real in PostgreSQL's catalogue. Skipped without `KENTOS_DATA_PG`.
#[test]
fn zonal_statistics_go_to_postgis_typed_from_their_cells() {
    let Some(address) = std::env::var("KENTOS_DATA_PG").ok().filter(|a| !a.is_empty()) else {
        eprintln!("KENTOS_DATA_PG yok: bölgesel istatistiğin PostGIS sütun türleri atlandı");
        return;
    };
    kentos_postgis::register();
    kentos_data_postgis::register();
    let schema = format!("kentos_tur_bolge_{}", std::process::id());
    let (mut doc, at) = drawing();
    let got = run(
        &mut doc,
        json!({
            "tool": "raster.zonalStatistics",
            "parameters": {
                "input": { "scope": "layer", "layerId": "bolge" },
                "zones": { "scope": "layer", "layerId": "bolgeler" },
                "stat": "mean",
                "output": "",
            },
            "sink": { "path": address, "schema": schema, "mode": "new", "tables": true },
            "drawing": at.display().to_string(),
        }),
    );
    assert_eq!(got["status"], "completed", "{got}");
    let table = got["output"]["sinkReport"]["tables"][0]["table"].as_str().expect("a table").to_owned();
    let source = kentos_data::registry().source(&address).expect("the database");
    let qualified = if table.contains('.') { table } else { format!("{schema}.{table}") };
    let fields = source.schema(&qualified).expect("the table").fields;
    let kind = |name: &str| fields.iter().find(|f| f.name == name).map(|f| f.kind.clone());
    assert!(matches!(kind("Sayı"), Some(kentos_data::FieldKind::Int { .. })), "{:?}", kind("Sayı"));
    assert!(matches!(kind("Ortalama"), Some(kentos_data::FieldKind::Real)), "{:?}", kind("Ortalama"));
    assert!(matches!(kind("Nesne"), Some(kentos_data::FieldKind::Text { .. })), "{:?}", kind("Nesne"));
}
