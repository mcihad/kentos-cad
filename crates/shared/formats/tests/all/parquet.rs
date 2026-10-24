//! Parquet files Apache Arrow wrote (fixtures/parquet/v1, scripts/fixtures/parquet_cases.py) read by our own reader
//! (docs/adr/0628 §1–§2): every type, codec, page version and encoding, nested columns, many row groups; each column's
//! records with their meaning as the fixture's rules give them, and broken files refused.

use std::path::PathBuf;

use kentos_formats::parquet::{self, File, FileSource, record};
use serde_json::Value;

fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/parquet/v1")
}

fn cases() -> Vec<Value> {
    let text = std::fs::read_to_string(dir().join("cases.json")).unwrap();
    let all: Value = serde_json::from_str(&text).unwrap();
    all["files"].as_array().unwrap().clone()
}

/// Every top-level column's records, row group after row group.
fn columns<S: parquet::Source>(f: &File<S>) -> Result<serde_json::Map<String, Value>, String> {
    let mut out = serde_json::Map::new();
    for &top in &f.schema.nodes[0].children {
        let mut all = Vec::new();
        for g in 0..f.meta.row_groups.len() {
            all.extend(record::records(f, g, top)?);
        }
        out.insert(f.schema.nodes[top].name.clone(), Value::Array(all));
    }
    Ok(out)
}

#[test]
fn every_file_arrow_wrote_reads_as_its_table() {
    let mut failures = Vec::new();
    for case in cases() {
        let name = case["file"].as_str().unwrap();
        let bytes = std::fs::read(dir().join(name)).unwrap();
        let got = File::open(bytes).and_then(|f| columns(&f));
        match (case.get("error"), got) {
            (Some(_), Ok(_)) => failures.push(format!("{name}: okundu, reddedilmeliydi")),
            (Some(_), Err(_)) => {}
            (None, Err(e)) => failures.push(format!("{name}: {e}")),
            (None, Ok(cols)) => {
                for (column, want) in case["columns"].as_object().unwrap() {
                    let have = &cols[column];
                    if have != want {
                        let at = have
                            .as_array()
                            .unwrap()
                            .iter()
                            .zip(want.as_array().unwrap())
                            .position(|(a, b)| a != b);
                        failures.push(format!(
                            "{name} › {column}: satır {at:?}: {} ≠ {}",
                            at.map_or(Value::Null, |i| have[i].clone()),
                            at.map_or(Value::Null, |i| want[i].clone())
                        ));
                    }
                }
            }
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
fn a_file_read_piece_by_piece_reads_alike() {
    let path = dir().join("gruplar.parquet");
    let whole = columns(&File::open(std::fs::read(&path).unwrap()).unwrap()).unwrap();
    let pieces = columns(&File::open(FileSource::open(&path).unwrap()).unwrap()).unwrap();
    assert_eq!(whole, pieces);
}

#[test]
fn brotli_is_refused_with_a_way_out() {
    let f = File::open(std::fs::read(dir().join("brotli.parquet")).unwrap()).unwrap();
    let e = record::records(&f, 0, f.schema.nodes[0].children[0]).unwrap_err();
    assert!(e.contains("BROTLI") && e.contains("ZSTD"), "{e}");
}

/// A table written here (`KENTOS_WRITE_PARQUET=1` puts it in fixtures/parquet/v1/out for Arrow to read).
fn written_table(rows: usize) -> (Vec<parquet::write::Column>, usize) {
    use parquet::write::Column;
    use parquet::{Bytes, Logical, Physical, TimeUnit, Values};
    let present: Vec<bool> = (0..rows).map(|k| k % 7 != 3).collect();
    let n = present.iter().filter(|&&p| p).count();
    let mut words = Bytes::default();
    for k in 0..n {
        words.push(format!("parsel {} ğüşıöç", k % 97).as_bytes());
    }
    let mut ids = Bytes::default();
    for k in 0..rows {
        ids.push(format!("{k:08}").as_bytes());
    }
    let col = |name: &str, physical, logical, optional: bool, values| Column {
        name: name.into(),
        group: None,
        physical,
        logical,
        optional,
        present: if optional {
            present.clone()
        } else {
            Vec::new()
        },
        values,
    };
    let member = |name: &str, values: Vec<f64>| Column {
        name: name.into(),
        group: Some("bbox".into()),
        physical: Physical::Double,
        logical: None,
        optional: true,
        present: present.clone(),
        values: Values::F64(values),
    };
    let xs: Vec<f64> = (0..n).map(|k| 500_000.0 + k as f64 * 0.25).collect();
    let columns = vec![
        col(
            "kimlik",
            Physical::ByteArray,
            Some(Logical::String),
            false,
            Values::Bytes(ids),
        ),
        col(
            "ad",
            Physical::ByteArray,
            Some(Logical::String),
            true,
            Values::Bytes(words),
        ),
        col(
            "no",
            Physical::Int64,
            None,
            true,
            Values::I64((0..n as i64).map(|k| k * 3 - 1000).collect()),
        ),
        col(
            "alan",
            Physical::Double,
            None,
            true,
            Values::F64((0..n).map(|k| k as f64 / 7.0).collect()),
        ),
        col(
            "var",
            Physical::Boolean,
            None,
            true,
            Values::Bool((0..n).map(|k| k % 3 == 0).collect()),
        ),
        col(
            "gun",
            Physical::Int32,
            Some(Logical::Date),
            true,
            Values::I32((0..n as i32).collect()),
        ),
        col(
            "an",
            Physical::Int64,
            Some(Logical::Timestamp {
                utc: false,
                unit: TimeUnit::Micros,
            }),
            true,
            Values::I64((0..n as i64).map(|k| k * 1_000_001).collect()),
        ),
        member("xmin", xs.clone()),
        member("ymin", xs.iter().map(|x| x - 400_000.0).collect()),
        member("xmax", xs.iter().map(|x| x + 1.0).collect()),
        member("ymax", xs.iter().map(|x| x - 399_999.0).collect()),
    ];
    (columns, rows)
}

#[test]
fn a_written_file_reads_back() {
    use parquet::write::{Options, write};
    let (table, rows) = written_table(70_000);
    for codec in [
        kentos_formats::compress::Codec::Snappy,
        kentos_formats::compress::Codec::None,
    ] {
        let o = Options {
            codec,
            page_bytes: 64 << 10,
            key_value: vec![("geo".into(), "{}".into())],
            ..Options::default()
        };
        let bytes = write(&table, rows, &o).unwrap();
        let f = File::open(bytes.clone()).unwrap();
        assert_eq!(f.meta.num_rows, rows as i64);
        assert_eq!(f.meta.row_groups.len(), 2);
        assert_eq!(f.meta.value("geo"), Some("{}"));
        let cols = columns(&f).unwrap();
        let ad = cols["ad"].as_array().unwrap();
        assert_eq!(ad[0], Value::from("parsel 0 ğüşıöç"));
        assert_eq!(ad[3], Value::Null);
        assert_eq!(cols["gun"][4], Value::from("1970-01-04"));
        assert_eq!(cols["an"][1], Value::from("1970-01-01T00:00:01.000001"));
        assert_eq!(cols["bbox"][3], Value::Null);
        assert_eq!(cols["bbox"][0]["xmax"], Value::from(500_001.0));
        assert_eq!(cols["kimlik"][69_999], Value::from("00069999"));
    }
    // A small one for Arrow: two row groups, pages of 4 KiB.
    if std::env::var_os("KENTOS_WRITE_PARQUET").is_some() {
        let (table, rows) = written_table(1_500);
        let o = Options {
            row_group: 1_000,
            page_bytes: 4 << 10,
            key_value: vec![("kentos".into(), "deneme".into())],
            ..Options::default()
        };
        let out = dir().join("out");
        std::fs::create_dir_all(&out).unwrap();
        std::fs::write(out.join("kentos.parquet"), write(&table, rows, &o).unwrap()).unwrap();
    }
}
