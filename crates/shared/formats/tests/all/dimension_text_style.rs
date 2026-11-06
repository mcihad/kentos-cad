//! docs/adr/1023: a DIMSTYLE's text style (DIMTXSTY) comes with it. The file is ezdxf's
//! (`fixtures/exchange/v1/dimension-text-style.dxf`, `scripts/fixtures/dimension_text_style_dxf.py`); what the reader
//! keeps is worked out from how the file is built, not from KentOS's output.

use kentos_contracts::{DrawingUnit, DxfReadOptions};
use serde_json::Value;

const FILE: &[u8] = include_bytes!("../../../../../fixtures/exchange/v1/dimension-text-style.dxf");
const EXPECTED: &str = include_str!("../../../../../fixtures/exchange/v1/dimension-text-style.json");

#[test]
fn a_dimension_styles_text_style_comes_with_it_and_is_kept_without_a_text() {
    let opts = DxfReadOptions { unit: Some(DrawingUnit::M), ..Default::default() };
    let read = kentos_formats::dxf::read(FILE, &opts).expect("reads");
    let want: Value = serde_json::from_str(EXPECTED).expect("the expectations");
    let texts: Vec<(String, Option<String>)> =
        read.text_styles.iter().map(|s| (s.name.clone(), s.font_file.clone())).collect();
    let want_texts: Vec<(String, Option<String>)> = want["textStyles"].as_array().unwrap().iter()
        .map(|s| (s["name"].as_str().unwrap().to_owned(), s["fontFile"].as_str().map(str::to_owned)))
        .collect();
    assert_eq!(texts, want_texts, "A-OLCU kept with its SHX file, A-NOT (followed by nothing) left out");
    let mut dims: Vec<(String, Option<String>)> =
        read.dimension_styles.iter().map(|s| (s.name.clone(), s.text_style.clone())).collect();
    let mut want_dims: Vec<(String, Option<String>)> = want["dimensionStyles"].as_array().unwrap().iter()
        .map(|s| (s["name"].as_str().unwrap().to_owned(), s["textStyle"].as_str().map(str::to_owned)))
        .collect();
    dims.sort();
    want_dims.sort();
    assert_eq!(dims, want_dims, "OFIS writes its value in A-OLCU, DUZ (Standard) in the project's typeface");
}
