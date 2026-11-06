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

/// The writer gives the dimension style its text style's STYLE record as DIMTXSTY (group 340), so another program
/// writes the value in it: read here from the file's groups, not by KentOS's reader.
#[test]
fn the_writer_names_a_dimension_styles_text_style_as_dimtxsty() {
    let input = kentos_formats::dxf::input_from_json(
        &serde_json::json!({"entities": [], "layers": [], "scale": 100., "lengthDecimals": 3, "grads": false,
            "textStyles": [{"id": "t1", "name": "A-OLCU", "font": "barlow", "fontFile": "romans.shx"}],
            "dimensionStyles": [{"id": "d1", "name": "OFIS", "height": 2.5, "textStyle": "a-olcu"},
                                {"id": "d2", "name": "DUZ", "height": 2.0}]})
        .to_string(),
    )
    .expect("writer input");
    let (bytes, _) = kentos_formats::dxf::write(&input);
    let text = String::from_utf8(bytes).expect("an ASCII DXF");
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    let pairs: Vec<(&str, &str)> = lines.chunks(2).filter(|c| c.len() == 2).map(|c| (c[0], c[1])).collect();
    // Each record: its groups from its 0 to the next.
    let records: Vec<&[(&str, &str)]> = {
        let starts: Vec<usize> = pairs.iter().enumerate().filter(|(_, p)| p.0 == "0").map(|(i, _)| i).collect();
        starts.iter().zip(starts.iter().skip(1).chain(std::iter::once(&pairs.len())))
            .map(|(a, b)| &pairs[*a..*b]).collect()
    };
    let group = |r: &[(&str, &str)], code: &str| r.iter().find(|p| p.0 == code).map(|p| p.1.to_owned());
    let style = records.iter().find(|r| r[0].1 == "STYLE" && group(r, "2").as_deref() == Some("A-OLCU"))
        .expect("the STYLE record");
    let handle = group(style, "5").expect("its handle");
    let dim = |name: &str| *records.iter().find(|r| r[0].1 == "DIMSTYLE" && group(r, "2").as_deref() == Some(name))
        .unwrap_or_else(|| panic!("the {name} DIMSTYLE"));
    assert_eq!(group(dim("OFIS"), "340"), Some(handle), "OFIS's DIMTXSTY is A-OLCU's record (name case aside)");
    assert_eq!(group(dim("DUZ"), "340"), None, "DUZ names none");
}
