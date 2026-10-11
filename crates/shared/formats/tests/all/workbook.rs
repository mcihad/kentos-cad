//! The workbook writer (docs/adr/0223 §2) against fixtures/table/v1/written:
//! each case's input written and compared byte for byte with the kept file
//! (KENTOS_WRITE_WORKBOOK=1 rewrites them; read the difference), read back
//! by the table reader into the words the case expects, its report's counts
//! the case's. scripts/fixtures/workbook_cases.py, written from the ADR,
//! holds the kept files to the rule with openpyxl and LibreOffice.

use std::path::PathBuf;

use kentos_contracts::WorkbookWriteInput;
use kentos_formats::table_file::{self, Limits};
use kentos_formats::workbook;
use serde_json::{Value, json};

#[test]
fn every_case_writes_its_kept_file() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/table/v1/written");
    let doc: Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("cases.json")).expect("cases.json"))
            .expect("JSON");
    assert_eq!(doc["format"], "kentos.workbook-cases");
    let rewrite = std::env::var_os("KENTOS_WRITE_WORKBOOK").is_some();
    let cases = doc["cases"].as_array().expect("cases");
    assert!(cases.len() >= 5);
    for c in cases {
        let name = c["name"].as_str().expect("a name");
        let input: WorkbookWriteInput =
            serde_json::from_value(c["input"].clone()).expect("an input");
        let (bytes, report) = workbook::write(&input).expect("written");
        let path = dir.join(name);
        if rewrite {
            std::fs::write(&path, &bytes).expect("kept");
        } else {
            let kept = std::fs::read(&path).expect("the kept file");
            assert!(
                bytes == kept,
                "{name}: the writer's bytes are not the kept file's"
            );
        }
        let Some(expect) = c.get("expect") else {
            continue;
        };
        let read = table_file::read_with(&bytes, Limits::PROCESSING);
        assert_eq!(read.problem, None, "{name}");
        let sheets: Vec<Value> = read
            .sheets
            .iter()
            .map(|s| json!({ "name": s.name, "rows": s.rows }))
            .collect();
        assert_eq!(Value::from(sheets), expect["sheets"], "{name}: read back");
        let fallbacks = [
            "Uzun sayı",
            "Sayı olmayan değer",
            "Tarih olmayan değer",
            "Eski tarih",
            "Evet/hayır olmayan değer",
        ];
        for what in fallbacks {
            let got = report
                .notes
                .iter()
                .find(|n| n.what == what)
                .map_or(0, |n| n.count);
            let want = expect["notes"][what].as_u64().unwrap_or(0);
            assert_eq!(u64::from(got), want, "{name}: {what}");
        }
    }
}

/// The same sheets give the same bytes; a sheet past XLSX's rows is refused.
#[test]
fn writing_is_steady_and_bounded() {
    let input: WorkbookWriteInput = serde_json::from_value(json!({
        "format": "ods",
        "sheets": [{ "name": "A", "columns": [{ "name": "x", "kind": "auto" }], "rows": [["1"], ["a"]] }]
    }))
    .expect("an input");
    assert_eq!(
        workbook::write(&input).expect("written").0,
        workbook::write(&input).expect("written").0
    );
    let big: WorkbookWriteInput = serde_json::from_value(json!({
        "format": "xlsx",
        "sheets": [{ "name": "B", "columns": [{ "name": "x", "kind": "text" }], "rows": vec![Vec::<String>::new(); workbook::MAX_ROWS] }]
    }))
    .expect("an input");
    let why = workbook::write(&big).expect_err("refused");
    assert!(why.contains("CSV"), "{why}");
}
