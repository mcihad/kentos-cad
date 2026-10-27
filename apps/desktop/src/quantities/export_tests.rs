//! Export regressions: the selected report must be the actual CSV on disk;
//! multi-sheet formats and feedback retain their different row counts.
use super::*;
use crate::files_testing::{app_with_drawing, scratch};
use crate::table_write::{Outcome, write};

#[test]
fn csv_exports_selected_quantity_table_including_empty_table_workbooks_keep_all() {
    let app = app_with_drawing();
    let q: SpacesQuantified = serde_json::from_str(include_str!(
        "../../../../docs/baseline/2026-10-25-architecture-quantity-cache/workload-quantities-before.json"
    )).unwrap();
    assert_eq!((q.spaces.len(), q.walls.len(), q.openings.len()), (400, 200, 0));
    let dir = scratch("quantity-export");
    for (tab, name, count, heading) in [
        (Tab::Spaces, "Mahaller", 400, "Numara;Ad;Kullanım;Kat;Yöntem;Alan;Çevre;Yükseklik;Hacim;Net;Eksen;Brüt"),
        (Tab::Walls, "Duvarlar", 200, "Stil;Kat;Uzunluk;Kalınlık;Yükseklik;Brüt yüzey;Açıklıklar;Net yüzey;Hacim"),
        // A niche's or flue's depth (docs/adr/0719 §4).
        (Tab::Openings, "Açıklıklar", 0, "Numara;Biçim;Tür;Genişlik;Yükseklik;Alan;Derinlik"),
    ] {
        for format in [WorkbookFormat::Csv, WorkbookFormat::Xlsx, WorkbookFormat::Ods] {
            let sheets = app.quantities_sheets(&q, tab, format);
            let path = dir.join(format!("{name}-{format:?}"));
            let expected_rows = if format == WorkbookFormat::Csv { count } else { 600 };
            match write(sheets, format, &path) {
                Outcome::Written { rows, .. } => assert_eq!(rows, expected_rows),
                other => panic!("{other:?}"),
            }
            let bytes = std::fs::read(path).unwrap();
            if format == WorkbookFormat::Csv {
                let csv = std::str::from_utf8(&bytes).unwrap().trim_start_matches('\u{feff}');
                assert_eq!(csv.lines().next(), Some(heading), "{tab:?}");
                assert_eq!(csv.lines().count(), count + 1);
            } else {
                let read = kentos_formats::table_file::read(&bytes);
                assert_eq!(read.problem, None);
                assert_eq!(read.sheets.iter().map(|s| (s.name.as_deref(), s.rows.len())).collect::<Vec<_>>(),
                    [(Some("Mahaller"), 401), (Some("Duvarlar"), 201), (Some("Açıklıklar"), 1)]);
            }
        }
    }
}

#[test]
fn csv_feedback_counts_only_written_sheet_preserves_quoting_and_failures() {
    let sheets: Vec<WorkbookSheet> = serde_json::from_value(serde_json::json!([
        {"name":"Bir","columns":[{"name":"Metin","kind":"text"}],"rows":[["a;b"],["satır1\nsatır2"]]},
        {"name":"İki","columns":[{"name":"Başka","kind":"text"}],"rows":[["x"]]}
    ])).unwrap();
    let dir = scratch("csv-feedback");
    let path = dir.join("quoted.csv");
    assert_eq!(write(sheets.clone(), WorkbookFormat::Csv, &path), Outcome::Written {
        file:"quoted.csv".into(), rows:2, notes:vec![]
    });
    assert_eq!(std::fs::read_to_string(path).unwrap(), "\u{feff}Metin\r\n\"a;b\"\r\n\"satır1\nsatır2\"\r\n");
    assert!(matches!(write(sheets, WorkbookFormat::Csv, &dir.join("missing/file.csv")), Outcome::Failed(_)));
    assert!(matches!(write(vec![], WorkbookFormat::Csv, &dir.join("none.csv")), Outcome::Failed(_)));
    assert!(!dir.join("none.csv").exists());
}
