//! The shared grammar cases (`fixtures/point-input/v1/cases.json`) through the
//! Rust reader of typed point input. The web runs the same file through its
//! own reader (`apps/web/src/tools/coordinateInput.test.ts`), so the two
//! cannot drift (docs/adr/0021).

use kentos_geometry_core::Vec2;
use kentos_geometry_core::tools::point_input::{AngleFrom, Angles};
use kentos_geometry_core::tools::point_text::{looks_like_coordinate, parse_number, point_name};
use kentos_geometry_core::tools::ucs::{Frame, point_from_text_framed};
use serde_json::Value;

const CASES: &str = include_str!("../../../../../fixtures/point-input/v1/cases.json");

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
                // A local project's drawing unit: typed in it, the point in metres, as
                // the desktop's `Format::to_metres` divides (docs/adr/0165 §2).
                // An inch and a foot by their ratio of whole numbers (docs/adr/0262 §1).
                let (mul, div) = match case["unit"].as_str() {
                    None | Some("m") => (1.0, 1.0),
                    Some("cm") => (1.0, 100.0),
                    Some("mm") => (1.0, 1000.0),
                    Some("in") => (254.0, 10_000.0),
                    Some("ft") => (3048.0, 10_000.0),
                    Some(other) => panic!("{name}: bilinmeyen birim {other}"),
                };
                // A polar angle in the project's way and unit (docs/adr/0165 §4); CAD's and degrees by default.
                let angles = Angles {
                    from: match case["convention"].as_str() {
                        None | Some("cad") => AngleFrom::East,
                        Some("gis") => AngleFrom::North,
                        Some(other) => panic!("{name}: bilinmeyen düzen {other}"),
                    },
                    grads: match case["angleUnit"].as_str() {
                        None | Some("deg") => false,
                        Some("grad") => true,
                        Some(other) => panic!("{name}: bilinmeyen açı birimi {other}"),
                    },
                };
                // A user coordinate system (docs/adr/0261 §4); the world without one.
                let frame = match case.get("ucs") {
                    Some(u) => Frame::new(
                        point(&u["origin"]).expect("a UCS origin"),
                        u["angle"].as_f64().expect("a UCS angle"),
                    ),
                    None => Frame::WORLD,
                };
                let got = point_from_text_framed(
                    text,
                    point(&case["last"]),
                    point(&case["cursor"]),
                    |_| None,
                    |v| if mul == 1.0 { v / div } else { v * mul / div },
                    angles,
                    &frame,
                );
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
            Some("pointName") => {
                let got = point_name(text);
                let want = case["expect"].as_str();
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

#[test]
fn single_length_fields_accept_displayed_unit_spacing_without_reinterpreting_coordinates() {
    use kentos_geometry_core::tools::point_text::{
        Len, parse_length, parse_length_pair, parse_point_text,
    };
    for (text, expected) in [
        ("0.300 m", Len::Metres(0.3)),
        ("0,300 m", Len::Metres(0.3)),
        ("250 mm", Len::Metres(0.25)),
        ("25 cm", Len::Metres(0.25)),
        ("0.001 km", Len::Metres(1.0)),
        ("250\u{a0}mm", Len::Metres(0.25)),
        ("250\u{202f}mm", Len::Metres(0.25)),
        ("250\tmm", Len::Metres(0.25)),
        ("8 in", Len::Inches(8.0)),
        ("8in", Len::Inches(8.0)),
        ("2 ft", Len::Inches(24.0)),
        ("2ft", Len::Inches(24.0)),
        ("6'-3\"", Len::Inches(75.0)),
    ] {
        assert_eq!(parse_length(text), Some(expected), "{text:?}");
    }
    for text in [
        "1 2",
        "1 2 m",
        "1 mm extra",
        "1 min",
        "1 inchworm",
        "1 feet",
        "1 m2",
    ] {
        assert_eq!(parse_length(text), None, "{text:?}");
    }
    // Keep the shared point-input v1 grammar's deliberate rejection and pair semantics.
    assert_eq!(parse_point_text("5 m"), None);
    assert_eq!(
        parse_length_pair("3 4"),
        Some((Len::Unit(3.), Len::Unit(4.)))
    );
}
