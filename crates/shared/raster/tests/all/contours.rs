//! Eş yükselti eğrileri (docs/adr/0231 §9): every case of
//! `scripts/fixtures/contour_cases.py` (KentOS code not used there; its
//! squares, crossings and chains held to gdal_contour's) run through a
//! whole job. Only +, −, × and ÷ make a line, in the ADR's order: its points
//! are the reference's bit for bit; each level's Kot text is the reference's.

use kentos_raster::job::Spec;
use serde_json::Value;

use crate::host::{Ran, read, run};

fn length(pts: &[[f64; 2]]) -> f64 {
    pts.windows(2)
        .map(|w| {
            let (dx, dy) = (w[1][0] - w[0][0], w[1][1] - w[0][1]);
            (dx * dx + dy * dy).sqrt()
        })
        .sum()
}

fn same_line(name: &str, n: usize, got: &kentos_raster::contours::Line, want: &Value) {
    assert_eq!(
        got.k,
        want["k"].as_i64().unwrap(),
        "{name}: line {n}'s level"
    );
    assert_eq!(
        got.value,
        want["value"].as_f64().unwrap(),
        "{name}: line {n}'s value"
    );
    assert_eq!(
        got.index,
        want["index"].as_bool().unwrap(),
        "{name}: line {n}: Ana or Ara"
    );
    let pts: Vec<[f64; 2]> = want["pts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| [p[0].as_f64().unwrap(), p[1].as_f64().unwrap()])
        .collect();
    assert_eq!(got.pts, pts, "{name}: line {n}'s points");
}

#[test]
fn every_case_draws_the_reference_s_lines() {
    let doc: Value = serde_json::from_slice(&read("contours/v1/cases.json")).unwrap();
    let mut checked = 0;
    for (c, case) in doc["cases"].as_array().unwrap().iter().enumerate() {
        let name = format!("{} {}", case["dem"].as_str().unwrap(), case["spec"]["tool"]);
        let tif = read(case["file"].as_str().unwrap());
        let spec: Spec =
            serde_json::from_value(case["spec"].clone()).unwrap_or_else(|e| panic!("{name}: {e}"));
        let Ran::Lines(lines) =
            run(&tif, &spec, 1 + c % 3).unwrap_or_else(|e| panic!("{name}: {e}"))
        else {
            panic!("{name}: lines");
        };
        if let Some(want) = case["lines"].as_array() {
            assert_eq!(lines.len(), want.len(), "{name}: lines");
            for (n, (g, w)) in lines.iter().zip(want).enumerate() {
                same_line(&name, n, g, w);
            }
        } else {
            for (n, w) in case["first"].as_array().unwrap().iter().enumerate() {
                same_line(&name, n, &lines[n], w);
            }
            for level in case["levels"].as_array().unwrap() {
                let k = level["k"].as_i64().unwrap();
                let of: Vec<_> = lines.iter().filter(|l| l.k == k).collect();
                assert_eq!(
                    of.len() as u64,
                    level["lines"].as_u64().unwrap(),
                    "{name}: level {k}'s lines"
                );
                let vertices: usize = of.iter().map(|l| l.pts.len()).sum();
                assert_eq!(
                    vertices as u64,
                    level["vertices"].as_u64().unwrap(),
                    "{name}: level {k}'s vertices"
                );
                let total: f64 = of.iter().map(|l| length(&l.pts)).sum();
                let want = level["length"].as_f64().unwrap();
                assert!(
                    (total - want).abs() <= 1e-9 * want,
                    "{name}: level {k}'s length {total} for {want}"
                );
            }
        }
        // Each level's Kot as the drawing writes it (the display rule with the spec's decimals).
        let tool = &case["spec"]["tool"];
        let s = kentos_raster::contours::Spec {
            interval: tool["interval"].as_f64().unwrap(),
            base: tool["base"].as_f64().unwrap(),
            index_every: tool["indexEvery"].as_u64().unwrap() as u32,
            simplify: tool["simplify"].as_f64().unwrap(),
        };
        for l in &lines {
            assert_eq!(
                s.level_text(l.value),
                case["texts"][l.k.to_string()].as_str().unwrap(),
                "{name}: level {}'s Kot",
                l.k
            );
        }
        checked += 1;
    }
    assert_eq!(checked, 7);
}

#[test]
fn a_closed_line_ends_where_it_starts_and_the_high_side_is_on_its_left() {
    // tepe's 5 m lines round the hill: every closed one's last point is its first, and walking it the
    // inside (higher) ground lies left: its area by the shoelace is positive.
    let tif = read("terrain/v1/tepe.tif");
    let doc: Value = serde_json::from_slice(&read("contours/v1/cases.json")).unwrap();
    let spec: Spec = serde_json::from_value(doc["cases"][2]["spec"].clone()).unwrap();
    let Ran::Lines(lines) = run(&tif, &spec, 2).unwrap() else {
        panic!()
    };
    let closed: Vec<_> = lines
        .iter()
        .filter(|l| l.pts.first() == l.pts.last())
        .collect();
    assert!(!closed.is_empty(), "the hill's top is ringed");
    for l in closed {
        let area: f64 = l
            .pts
            .windows(2)
            .map(|w| w[0][0] * w[1][1] - w[1][0] * w[0][1])
            .sum();
        assert!(
            area > 0.0,
            "level {}: counter-clockwise round higher ground",
            l.value
        );
    }
}

#[test]
fn too_many_levels_are_refused_before_any_line() {
    let tif = read("terrain/v1/genis.tif");
    let doc: Value = serde_json::from_slice(&read("contours/v1/cases.json")).unwrap();
    let mut spec = doc["cases"][6]["spec"].clone();
    spec["tool"]["interval"] = serde_json::json!(0.001);
    let spec: Spec = serde_json::from_value(spec).unwrap();
    let Err(e) = run(&tif, &spec, 1) else {
        panic!("runs")
    };
    assert!(e.contains("düzey"), "{e}");
}
