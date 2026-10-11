//! Tablo alışverişi on this machine, release (docs/adr/0223 §8):
//!
//!     cargo test --release -p kentos-formats --test all workbook_timing -- --ignored --nocapture
//!
//! A table of 100 000 rows and 10 columns (three texts, four numbers, a date,
//! a yes or no and a column of no kind) written as XLSX, ODS and CSV; a sheet
//! of 250 000 rows and the same columns, İşlemler's most, written once in each
//! workbook and read back within İşlemler's limits. The web runs the same
//! through WASM (apps/web/scripts/perf/workbook.test.ts).

use std::time::Instant;

use kentos_contracts::{
    CellKind, WorkbookColumn, WorkbookFormat, WorkbookSheet, WorkbookWriteInput,
};
use kentos_formats::table_file::{self, Limits};
use kentos_formats::workbook;

/// `rows` rows of a parcel register: names, numbers, a date, a yes or no.
fn sheet(rows: usize) -> WorkbookSheet {
    let column = |name: &str, kind: CellKind, decimals: Option<u32>| WorkbookColumn {
        name: name.to_owned(),
        kind,
        decimals,
    };
    let columns = vec![
        column("Ad", CellKind::Text, None),
        column("Mahalle", CellKind::Text, None),
        column("Malik", CellKind::Text, None),
        column("Ada", CellKind::Number, Some(0)),
        column("Parsel", CellKind::Number, Some(0)),
        column("Alan", CellKind::Number, Some(2)),
        column("Doğu", CellKind::Number, Some(3)),
        column("Tarih", CellKind::Date, None),
        column("Ruhsat", CellKind::Boolean, None),
        column("Not", CellKind::Auto, None),
    ];
    const PLACES: [&str; 4] = ["Kızılay", "Çankaya", "Güvenevler", "Öveçler"];
    let rows = (0..rows)
        .map(|i| {
            vec![
                format!("P{i}"),
                PLACES[i % 4].to_owned(),
                format!("Ayşe Öztürk {}", i % 997),
                format!("{}", 100 + i / 50),
                format!("{}", 1 + i % 50),
                format!("{}.{:02}", 300 + i % 900, i % 100),
                format!("{}.{:03}", 487_000 + i % 4000, i % 1000),
                format!("20{:02}-{:02}-{:02}", 10 + i % 15, 1 + i % 12, 1 + i % 28),
                (if i % 3 == 0 { "true" } else { "false" }).to_owned(),
                if i % 5 == 0 {
                    "köşe".to_owned()
                } else {
                    format!("{}", i % 7)
                },
            ]
        })
        .collect();
    WorkbookSheet {
        name: "Parseller".to_owned(),
        columns,
        rows,
    }
}

fn time<T>(what: &str, budget_ms: f64, f: impl Fn() -> T) -> T {
    // One warm-up, then the best of three.
    let _ = f();
    let mut best = f64::INFINITY;
    let mut out = None;
    for _ in 0..3 {
        let t = Instant::now();
        let r = f();
        best = best.min(t.elapsed().as_secs_f64() * 1e3);
        out = Some(r);
    }
    println!("{what}: {best:.0} ms (bütçe {budget_ms:.0} ms)");
    assert!(
        best <= budget_ms,
        "{what}: {best:.0} ms > {budget_ms:.0} ms"
    );
    out.expect("ran")
}

fn write(sheet: &WorkbookSheet, format: WorkbookFormat) -> Vec<u8> {
    workbook::write(&WorkbookWriteInput {
        format,
        sheets: vec![sheet.clone()],
    })
    .expect("writes")
    .0
}

#[test]
#[ignore = "timings, run by hand in release"]
fn workbook_timing() {
    let table = sheet(100_000);
    for (format, name, budget) in [
        (WorkbookFormat::Xlsx, "XLSX", 700.0),
        (WorkbookFormat::Ods, "ODS", 900.0),
        (WorkbookFormat::Csv, "CSV", 150.0),
    ] {
        let bytes = time(
            &format!("{name} yaz (100 000 satır, 10 sütun)"),
            budget,
            || write(&table, format),
        );
        println!("  {name}: {:.1} MB", bytes.len() as f64 / 1e6);
        if let Ok(dir) = std::env::var("KENTOS_PERF_FILES") {
            std::fs::write(format!("{dir}/tablo-100k.{}", name.to_lowercase()), &bytes)
                .expect("writes");
        }
    }
    let most = sheet(table_file::PROCESSING_ROWS);
    for (format, name, budget) in [
        (WorkbookFormat::Xlsx, "XLSX", 2000.0),
        (WorkbookFormat::Ods, "ODS", 2200.0),
    ] {
        let bytes = write(&most, format);
        println!(
            "  {name}, 250 000 satır: {:.1} MB",
            bytes.len() as f64 / 1e6
        );
        if let Ok(dir) = std::env::var("KENTOS_PERF_FILES") {
            std::fs::write(format!("{dir}/tablo-250k.{}", name.to_lowercase()), &bytes)
                .expect("writes");
        }
        let read = time(
            &format!("{name} oku (250 000 satır, 10 sütun; İşlemler'in sınırıyla)"),
            budget,
            || table_file::read_with(&bytes, Limits::PROCESSING),
        );
        assert_eq!(read.problem, None);
        assert_eq!(read.sheets[0].rows.len(), table_file::PROCESSING_ROWS + 1);
        assert!(!read.sheets[0].cut);
    }
}
