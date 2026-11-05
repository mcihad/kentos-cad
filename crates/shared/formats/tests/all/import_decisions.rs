//! CAD-ADV-17 (docs/adr/1010): the reader's decisions on a drawing as a whole in one structured section. The file is
//! ezdxf's (`fixtures/exchange/v1/decisions.dxf`) and the decisions it calls for are worked out from how it is built
//! (`scripts/fixtures/import_decisions_cases.py`), not from KentOS's output.

use kentos_contracts::{DrawingUnit, DxfReadOptions};
use serde_json::Value;

const FILE: &[u8] = include_bytes!("../../../../../fixtures/exchange/v1/decisions.dxf");
const EXPECTED: &str = include_str!("../../../../../fixtures/exchange/v1/decisions.json");

fn read() -> kentos_contracts::ImportReport {
    let opts = DxfReadOptions { unit: Some(DrawingUnit::M), explode_blocks: true, ..Default::default() };
    kentos_formats::dxf::read(FILE, &opts).expect("reads").report
}

#[test]
fn the_report_says_each_decision_once_under_its_topic_in_order() {
    let report = read();
    let want: Value = serde_json::from_str(EXPECTED).expect("the expected decisions");
    let want = want["decisions"].as_array().expect("a list");
    let got: Vec<Value> = report
        .decisions
        .iter()
        .map(|d| serde_json::to_value(d).expect("json"))
        .collect();
    let short = |v: &Value| (v["topic"].clone(), v["code"].clone(), v["subject"].clone(), v["count"].clone());
    assert_eq!(
        got.iter().map(short).collect::<Vec<_>>(),
        want.iter().map(short).collect::<Vec<_>>(),
        "{}",
        serde_json::to_string_pretty(&got).unwrap()
    );
    for (g, w) in got.iter().zip(want) {
        let words = g["decision"].as_str().expect("words");
        assert!(!words.is_empty());
        if let Some(layers) = w["layers"].as_array() {
            assert!(words.starts_with(&format!("{} katmanda nesne yok", layers.len())), "{words}");
            for l in layers {
                assert!(words.contains(l.as_str().unwrap()), "{words} lacks {l}");
            }
        }
        if let Some(n) = w["sheets"].as_u64() {
            assert!(words.starts_with(&format!("{n} pafta, {} görünüm", w["viewports"])), "{words}");
        }
    }
}

#[test]
fn a_decision_the_notes_said_before_is_still_in_the_notes() {
    let report = read();
    for code in ["unit_converted", "block_nonuniform_arcs_nurbs", "block_nonuniform_widths_exact"] {
        let d = report.decisions.iter().find(|d| d.code == code).expect(code);
        assert!(
            report.notes.iter().any(|n| n.what == d.subject && n.reason == d.decision),
            "{code} is not in the notes"
        );
    }
    // A quiet one only in the decisions: the notes stay as they were.
    let quiet = report.decisions.iter().find(|d| d.code == "layer_unused_left_out").expect("unused layers");
    assert!(!report.notes.iter().any(|n| n.reason == quiet.decision));
}
