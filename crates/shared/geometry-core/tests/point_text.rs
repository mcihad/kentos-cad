//! The shared grammar cases (`fixtures/point-input/v1/cases.json`) through the
//! Rust reader of typed point input. The web runs the same file through its
//! own reader (`apps/web/src/tools/coordinateInput.test.ts`), so the two
//! cannot drift (docs/adr/0021).

use kentos_geometry_core::Vec2;
use kentos_geometry_core::tools::point_text::{
    looks_like_coordinate, parse_number, point_from_text,
};
use serde_json::Value;

const CASES: &str = include_str!("../../../../fixtures/point-input/v1/cases.json");

fn point(v: &Value) -> Option<Vec2> {
    Some(Vec2::new(v.get("x")?.as_f64()?, v.get("y")?.as_f64()?))
}

#[test]
fn every_shared_case_reads_as_the_web_reads_it() {
    let file: Value = serde_json::from_str(CASES).expect("the cases are JSON");
    assert_eq!(file["format"], "kentos.point-input-cases");
    assert_eq!(file["version"], 1);
    let cases = file["cases"].as_array().expect("a case list");
    assert!(cases.len() > 40, "the cases are all there");
    let mut problems = Vec::new();
    for case in cases {
        let name = case["name"].as_str().unwrap_or("?");
        let text = case["text"].as_str().expect("every case has a text");
        let tolerance = case["tolerance"].as_f64().unwrap_or(0.0);
        let bad = match case["fn"].as_str() {
            Some("point") => {
                let got =
                    point_from_text(text, point(&case["last"]), point(&case["cursor"]), |_| None);
                let want = point(&case["expect"]);
                let same = match (got, want) {
                    (Some(g), Some(w)) => {
                        (g.x - w.x).abs() <= tolerance && (g.y - w.y).abs() <= tolerance
                    }
                    (None, None) => true,
                    _ => false,
                };
                (!same).then(|| format!("{got:?}, beklenen {want:?}"))
            }
            Some("number") => {
                let got = parse_number(text);
                let want = case["expect"].as_f64();
                (got != want).then(|| format!("{got:?}, beklenen {want:?}"))
            }
            Some("looksLikeCoordinate") => {
                let got = looks_like_coordinate(text);
                let want = case["expect"].as_bool().expect("true or false");
                (got != want).then(|| format!("{got}, beklenen {want}"))
            }
            other => Some(format!("bilinmeyen işlev {other:?}")),
        };
        if let Some(bad) = bad {
            problems.push(format!("{name} ({text:?}): {bad}"));
        }
    }
    assert!(problems.is_empty(), "\n{}", problems.join("\n"));
}
