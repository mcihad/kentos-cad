//! What the table's rules give a cell (docs/adr/0413 §1), by the shared cases
//! (fixtures/table-styles/v1/looks.json, written by scripts/fixtures/table_styles_command_cases.py).

use kentos_contracts::{CellLook, TableStyle, cell_look};
use serde_json::Value;

const LOOKS: &str = include_str!("../../../../../fixtures/table-styles/v1/looks.json");

#[test]
fn a_cells_look_is_the_shared_cases() {
    let data: Value = serde_json::from_str(LOOKS).expect("JSON");
    let styles: Vec<TableStyle> = serde_json::from_value(data["styles"].clone()).expect("rules");
    let cases = data["cases"].as_array().expect("cases");
    assert!(cases.len() >= 20);
    for case in cases {
        let holds: Vec<bool> = serde_json::from_value(case["holds"].clone()).expect("holds");
        let field = case["field"].as_str();
        let want = &case["want"];
        let text = |v: &Value| v.as_str().map(str::to_owned);
        let want = CellLook {
            background: text(&want["background"]),
            color: text(&want["color"]),
            bold: want["bold"].as_bool().unwrap_or(false),
        };
        assert_eq!(cell_look(&styles, &holds, field), want, "{case}");
    }
}
