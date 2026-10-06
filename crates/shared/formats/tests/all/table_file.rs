//! Tablo ekle's file reader (docs/adr/0184 §4) as the shared cases say:
//! fixtures/table/v1/files.json, written with its sample files by
//! scripts/fixtures/table_file_cases.py from the rule (Python's zipfile and
//! xml.etree for the workbook), not from this code. The web reads the same
//! files through WASM (io/tableFile.test.ts).

use std::path::PathBuf;

use kentos_contracts::TableFileRead;
use kentos_formats::table_file;
use serde_json::Value;

#[test]
fn every_sample_file_reads_as_the_cases_say() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/table/v1");
    let text = std::fs::read_to_string(dir.join("files.json")).expect("files.json");
    let cases: Value = serde_json::from_str(&text).expect("JSON");
    assert_eq!(cases["format"], "kentos.table-file-cases");
    let list = cases["cases"].as_array().expect("cases");
    assert!(list.len() >= 10);
    for c in list {
        let name = c["file"].as_str().expect("a file");
        let bytes = std::fs::read(dir.join("files").join(name)).expect("the sample");
        let want: TableFileRead = serde_json::from_value(c["want"].clone()).expect("a reading");
        assert_eq!(table_file::read(&bytes), want, "{name}");
    }
}

/// Bytes no table file holds are refused or read, never a panic: truncated
/// workbooks, a zip bomb's ratio, broken XML, random bytes.
#[test]
fn broken_files_do_not_panic() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/table/v1/files");
    let book = std::fs::read(dir.join("kitap.xlsx")).expect("the workbook");
    for cut in [10, 100, book.len() / 2, book.len() - 1] {
        let _ = table_file::read(&book[..cut]);
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
