//! Coordinate lists from `fixtures/formats/v1/` (the same files the WASM
//! test reads, `src/io/formats.wasm.test.ts`): Netcad NCN, a Turkish
//! spreadsheet in Windows-1254 with decimal commas, a list with bad lines,
//! and Excel's UTF-16 text. Every coordinate is the float64 nearest to the
//! file's decimal (compared bit for bit with Rust's own literal).

use kentos_contracts::{CoordColumn, CoordDelimiter, CoordRead, CoordReadOptions, DecimalMark, Entity};
use kentos_formats::coords;

fn fixture(name: &str) -> Vec<u8> {
    let path = format!("{}/../../fixtures/formats/v1/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn read(name: &str) -> CoordRead {
    coords::read(&fixture(name), &CoordReadOptions { preview_rows: 20, entities: true, ..Default::default() })
}

/// (label, x = Y, y = X, z) of every point read.
fn points(r: &CoordRead) -> Vec<(String, f64, f64, Option<f64>)> {
    r.result
        .as_ref()
        .expect("objects")
        .entities
        .iter()
        .map(|e| match e {
            Entity::Point(p) => (p.base.label.clone().unwrap_or_default(), p.p.x, p.p.y, p.z),
            other => panic!("not a point: {other:?}"),
        })
        .collect()
}

fn same(got: &[(String, f64, f64, Option<f64>)], want: &[(&str, f64, f64, Option<f64>)]) {
    assert_eq!(got.len(), want.len(), "{got:?}");
    for (g, w) in got.iter().zip(want) {
        assert_eq!(g.0, w.0);
        assert_eq!(g.1.to_bits(), w.1.to_bits(), "{} Y", w.0);
        assert_eq!(g.2.to_bits(), w.2.to_bits(), "{} X", w.0);
        assert_eq!(g.3.map(f64::to_bits), w.3.map(f64::to_bits), "{} Z", w.0);
    }
}

#[test]
fn netcad_ncn_reads_name_y_x_z() {
    let r = read("netcad.ncn");
    assert_eq!(r.encoding, "UTF-8");
    assert_eq!(r.delimiter, CoordDelimiter::Space);
    assert_eq!(r.columns, vec![CoordColumn::Name, CoordColumn::Y, CoordColumn::X, CoordColumn::Z]);
    assert_eq!(r.data_lines, 4);
    assert_eq!(r.error_count, 0);
    same(
        &points(&r),
        &[
            ("1001", 452345.123, 4412345.678, Some(105.20)),
            ("1002", 452360.5, 4412350.25, Some(106.75)),
            ("P3", 452371.004, 4412339.9, Some(107.0)),
            ("K12", 452380.1, 4412360.33, Some(108.125)),
        ],
    );
    let b = r.bounds.expect("extent");
    assert_eq!((b.min_x, b.max_x, b.min_y, b.max_y), (452345.123, 452380.1, 4412339.9, 4412360.33));
}

#[test]
fn a_turkish_spreadsheet_in_windows_1254() {
    let r = read("excel-tr.csv");
    assert_eq!(r.encoding, "Windows-1254 (Türkçe)");
    assert_eq!(r.delimiter, CoordDelimiter::Semicolon);
    assert_eq!(r.decimal, DecimalMark::Comma);
    assert!(r.header);
    assert_eq!(r.header_fields, vec!["Nokta Adı", "Sağa (Y)", "Yukarı (X)", "Kot", "Açıklama"]);
    assert_eq!(r.columns, vec![CoordColumn::Name, CoordColumn::Y, CoordColumn::X, CoordColumn::Z, CoordColumn::Code]);
    same(
        &points(&r),
        &[
            ("Çınar-1", 452345.12, 4412345.5, Some(12.75)),
            ("Şev-2", 452346.0, 4412346.0, None),
            ("İstasyon", 452350.005, 4412340.125, Some(13.001)),
        ],
    );
    let Some(Entity::Point(p)) = r.result.as_ref().and_then(|x| x.entities.first()) else { panic!() };
    assert_eq!(p.base.attrs.get("Kod").map(String::as_str), Some("ağaç"));
    assert_eq!(p.base.attrs.get("Z (m)").map(String::as_str), Some("12.75"));
}

#[test]
fn lines_that_are_not_points_are_named_with_their_numbers() {
    let r = read("bad-lines.txt");
    assert_eq!(r.delimiter, CoordDelimiter::Tab);
    assert_eq!(r.points, 2);
    assert_eq!(r.error_count, 3);
    let messages: Vec<&str> = r.errors.iter().map(|e| e.message.as_str()).collect();
    assert_eq!(
        messages,
        vec![
            "Satır 2: Y (sağa) değeri “abc” sayı değil.",
            "Satır 3: 2 alan var; X (yukarı) için en az 3 alan gerekiyor.",
            "Satır 4: Z (kot) değeri “x12” sayı değil.",
        ]
    );
    same(&points(&r), &[("P1", 452345.1, 4412345.2, Some(10.0)), ("P5", 452345.8, 4412345.9, None)]);
    let skipped = &r.result.as_ref().expect("objects").report.skipped;
    assert_eq!(skipped.iter().map(|s| s.count).sum::<u32>(), 3);
}

#[test]
fn excel_unicode_text_is_utf16() {
    let r = read("unicode.txt");
    assert_eq!(r.encoding, "UTF-16");
    assert!(r.header);
    same(&points(&r), &[("Ş1", 452345.5, 4412345.25, None)]);
}
