//! docs/adr/1128: DXF table styles. The import file is ezdxf's with TABLESTYLE and ACAD_TABLE groups added from
//! Autodesk's DXF reference (`scripts/fixtures/table_style_dxf.py`); what the reader keeps is worked out there from
//! the groups, not from KentOS's output. The export is written here from the script's input and read back by KentOS
//! (exactly the project's styles) and by ezdxf (the script's `--check`: the groups the ADR's rules give).

use kentos_contracts::{DrawingUnit, DxfReadOptions, Entity};
use serde_json::Value;

const FILE: &[u8] = include_bytes!("../../../../../fixtures/exchange/v1/table-styles.dxf");
const EXPECTED: &str = include_str!("../../../../../fixtures/exchange/v1/table-styles.json");
const EXPORT_INPUT: &str = include_str!("../../../../../fixtures/exchange/v1/table-styles-export.json");

/// JSON equality with numbers within a nanometre's worth of a millimetre (the reference's metres × 1000 / 100).
fn close(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => (x.as_f64().unwrap() - y.as_f64().unwrap()).abs() < 1e-9,
        (Value::Object(x), Value::Object(y)) => x.len() == y.len() && x.iter().all(|(k, v)| y.get(k).is_some_and(|w| close(v, w))),
        (Value::Array(x), Value::Array(y)) => x.len() == y.len() && x.iter().zip(y).all(|(p, q)| close(p, q)),
        _ => a == b,
    }
}

#[test]
fn another_programs_table_style_comes_with_its_acad_table_and_what_is_lost_is_said() {
    let want: Value = serde_json::from_str(EXPECTED).expect("the expectations");
    let opts = DxfReadOptions { unit: Some(DrawingUnit::M), scale: want["scale"].as_f64(), ..Default::default() };
    let read = kentos_formats::dxf::read(FILE, &opts).expect("reads");
    let mut got = serde_json::to_value(&read.table_styles).unwrap();
    for s in got.as_array_mut().unwrap() {
        assert!(s["id"].as_str().unwrap().starts_with("dxf-table-"), "the reader numbers another program's");
        s.as_object_mut().unwrap().remove("id");
    }
    assert!(close(&got, &want["tableStyles"]), "{got:#}\n{:#}", want["tableStyles"]);
    let texts: Vec<(String, Option<String>)> =
        read.text_styles.iter().map(|s| (s.name.clone(), s.font_file.clone())).collect();
    assert_eq!(texts, vec![("A-TABLO".to_owned(), Some("romans.shx".to_owned()))], "the data cells' text style, kept");
    let codes: Vec<&str> = read.report.decisions.iter().map(|d| d.code.as_str()).collect();
    for code in want["decisions"].as_array().unwrap() {
        assert!(codes.contains(&code.as_str().unwrap()), "{code}: {codes:?}");
    }
    let lossy = read.report.decisions.iter().find(|d| d.code == "table_style_lossy").expect("what is lost");
    for words in want["lost"].as_array().unwrap() {
        assert!(lossy.decision.contains(words.as_str().unwrap()), "{words}: {}", lossy.decision);
    }
    let unused = read.report.decisions.iter().find(|d| d.code == "table_style_unused_left_out").unwrap();
    for name in want["unused"].as_array().unwrap() {
        assert!(unused.decision.contains(name.as_str().unwrap()), "{}", unused.decision);
    }
}

#[test]
fn the_project_table_styles_go_out_as_tablestyle_objects_and_come_back_exactly() {
    let input = kentos_formats::dxf::input_from_json(EXPORT_INPUT).expect("writer input");
    let (bytes, _) = kentos_formats::dxf::write(&input);
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/exchange/v1/table-styles-export.dxf");
    if std::env::var("KENTOS_WRITE_TABLE_STYLE_EXPORT").as_deref() == Ok("1") {
        std::fs::write(&path, &bytes).expect("writes the export for ezdxf to read");
    }
    assert_eq!(
        std::fs::read(&path).unwrap_or_default(),
        bytes,
        "table-styles-export.dxf is not the writer's: KENTOS_WRITE_TABLE_STYLE_EXPORT=1, then the script's --check"
    );
    let opts = DxfReadOptions { unit: Some(DrawingUnit::M), scale: Some(input.scale), ..Default::default() };
    let read = kentos_formats::dxf::read(&bytes, &opts).expect("reads back");
    assert_eq!(read.table_styles, input.table_styles, "KentOS's data carries each style exactly, unused ones too");
    let table = read.entities.iter().find_map(|e| if let Entity::Table(t) = e { Some(t) } else { None }).expect("the table");
    assert_eq!(table.table_style.as_deref(), Some("ofis-cetvel"));
    assert!(
        !read.report.decisions.iter().any(|d| d.code.starts_with("table_style")),
        "KentOS's own styles are no decision"
    );
}
