//! The table file reader (docs/adr/0184 §4, 0223 §1) as the shared cases
//! say: fixtures/table/v1/files.json, written with its sample files by
//! scripts/fixtures/table_file_cases.py from the rule (Python's zipfile and
//! xml.etree for the workbooks, cross-checked with openpyxl and LibreOffice),
//! not from this code. The web reads the same files through WASM
//! (io/formats.wasm.test.ts).

use std::path::PathBuf;

use kentos_contracts::TableFileRead;
use kentos_formats::table_file::{self, Limits};
use serde_json::Value;

fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/table/v1")
}

#[test]
fn every_sample_file_reads_as_the_cases_say() {
    let text = std::fs::read_to_string(dir().join("files.json")).expect("files.json");
    let cases: Value = serde_json::from_str(&text).expect("JSON");
    assert_eq!(cases["format"], "kentos.table-file-cases");
    assert_eq!(
        cases["processing"],
        serde_json::json!({
            "rows": Limits::PROCESSING.rows,
            "columns": Limits::PROCESSING.columns
        })
    );
    let list = cases["cases"].as_array().expect("cases");
    assert!(list.len() >= 20);
    for c in list {
        let name = c["file"].as_str().expect("a file");
        let bytes = std::fs::read(dir().join("files").join(name)).expect("the sample");
        let want: TableFileRead = serde_json::from_value(c["want"].clone()).expect("a reading");
        let limits = match c.get("limits") {
            Some(l) => Limits {
                rows: l["rows"].as_u64().expect("rows") as usize,
                columns: l["columns"].as_u64().expect("columns") as usize,
            },
            None => Limits::TABLE,
        };
        assert_eq!(
            table_file::read_with(&bytes, limits),
            want,
            "{name} {limits:?}"
        );
    }
}

/// İşlemler reads past a drawn table's rows: the 10 003 records of
/// cok-satir.csv come whole, where Tablo ekle stops at 10 001.
#[test]
fn processing_reads_past_a_tables_rows() {
    let bytes = std::fs::read(dir().join("files/cok-satir.csv")).expect("the sample");
    let read = table_file::read_with(&bytes, Limits::PROCESSING);
    assert_eq!(read.sheets[0].rows.len(), 10_003);
    assert!(!read.sheets[0].cut);
    assert_eq!(read.sheets[0].rows[10_002], ["10002", "20004"]);
    assert!(table_file::read(&bytes).sheets[0].cut);
}

/// Bytes no table file holds are refused or read, never a panic: truncated
/// workbooks, a zip bomb's ratio, broken XML, random bytes.
#[test]
fn broken_files_do_not_panic() {
    for name in [
        "kitap.xlsx",
        "tarihler.xlsx",
        "tablo.ods",
        "libre.xlsx",
        "libre.ods",
    ] {
        let book = std::fs::read(dir().join("files").join(name)).expect("the workbook");
        for cut in [10, 100, book.len() / 2, book.len() - 1] {
            let _ = table_file::read(&book[..cut]);
        }
    }
    let mut seed = 0x2545_f491_4f6c_dd1du64;
    for len in [0usize, 1, 7, 64, 513, 4096] {
        let bytes: Vec<u8> = (0..len)
            .map(|_| {
                seed ^= seed << 13;
                seed ^= seed >> 7;
                seed ^= seed << 17;
                (seed >> 24) as u8
            })
            .collect();
        let _ = table_file::read(&bytes);
    }
}
