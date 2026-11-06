//! Sample lines and the terrain along them against the shared cases (fixtures/road/v1/sections.json, docs/adr/0903
//! §1): written by cutting each line against Triangle's edges at 40 digits (scripts/fixtures/road_section_cases.py),
//! no KentOS code.

use kentos_road::alignment::{PiInput, build as align};
use kentos_road::section::{ground, sample_line};
use kentos_road::surface::{SurfaceInput, build};
use serde_json::Value;

const CASES: &str = include_str!("../../../../../fixtures/road/v1/sections.json");

fn n(v: &Value) -> f64 {
    v.as_f64().expect("a number")
}

#[test]
fn every_section_crosses_where_the_reference_says() {
    let all: Value = serde_json::from_str(CASES).expect("json");
    let within = n(&all["tolerance"]);
    for c in all["cases"].as_array().expect("cases") {
        let name = c["name"].as_str().expect("name");
        let input: SurfaceInput = serde_json::from_value(c["input"].clone()).expect("input");
        let surface = build(&input).expect("a surface");
        let pis: Vec<PiInput> = c["pis"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| PiInput {
                id: p["id"].as_str().unwrap().to_owned(),
                x: n(&p["x"]),
                y: n(&p["y"]),
                radius: n(&p["radius"]),
                spiral_in: n(&p["spiralIn"]),
                spiral_out: n(&p["spiralOut"]),
                ove: false,
            })
            .collect();
        let al = align(&pis).expect("an alignment");
        for l in c["lines"].as_array().unwrap() {
            let s = n(&l["station"]);
            let line = sample_line(&al, s, n(&l["left"]), n(&l["right"])).expect("on the alignment");
            for (got, want) in line.ends.iter().zip(l["ends"].as_array().unwrap()) {
                for k in 0..2 {
                    assert!((got[k] - n(&want[k])).abs() < within, "{name} km {s}: end {got:?} vs {want}");
                }
            }
            let got = ground(&surface, &line);
            let want = l["ground"].as_array().unwrap();
            assert_eq!(got.len(), want.len(), "{name} km {s}: places");
            for (g, w) in got.iter().zip(want) {
                assert!((g.offset - n(&w["offset"])).abs() < within, "{name} km {s}: {} vs {}", g.offset, w["offset"]);
                match (g.z, w["z"].as_f64()) {
                    (Some(a), Some(b)) => assert!((a - b).abs() < within, "{name} km {s}: {a} vs {b}"),
                    (None, None) => {}
                    other => panic!("{name} km {s} at {}: {other:?}", g.offset),
                }
                assert_eq!(g.gap, w["gap"].as_bool().unwrap(), "{name} km {s}: gap at {}", g.offset);
                assert_eq!(g.crossing, w["crossing"].as_bool().unwrap(), "{name} km {s}: crossing at {}", g.offset);
            }
        }
        // Off the alignment, or with no width, there is no line.
        assert!(sample_line(&al, al.length + 1.0, 10.0, 10.0).is_none());
        assert!(sample_line(&al, 0.0, 0.0, 10.0).is_none());
    }
}

/// The core's own output drawn: the TIN, the alignment, the sample lines, and under them the terrain along each
/// (`.run/gui-evidence/yol-y5-kesit-hatlari.svg`).
#[test]
#[ignore = "writes a picture"]
fn picture() {
    let all: Value = serde_json::from_str(CASES).expect("json");
    let c = &all["cases"][1];
    let surface = build(&serde_json::from_value(c["input"].clone()).unwrap()).unwrap();
    let pis: Vec<PiInput> = c["pis"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| PiInput {
            id: p["id"].as_str().unwrap().to_owned(),
            x: n(&p["x"]),
            y: n(&p["y"]),
            radius: n(&p["radius"]),
            spiral_in: n(&p["spiralIn"]),
            spiral_out: n(&p["spiralOut"]),
            ove: false,
        })
        .collect();
    let al = align(&pis).unwrap();
    let (w, plan_h) = (1400.0, 560.0);
    let k = 1.25;
    let px = |x: f64, y: f64| (40.0 + x * k, plan_h - 20.0 - y * k);
    let mut svg = format!("<svg xmlns='http://www.w3.org/2000/svg' width='{w}' height='1400' style='background:#fff;font-family:Helvetica'>");
    for t in &surface.triangles {
        let pts: String = t.iter().map(|&i| {
            let v = surface.vertices[i as usize];
            let (x, y) = px(v[0], v[1]);
            format!("{x:.1},{y:.1} ")
        }).collect();
        svg += &format!("<polygon points='{pts}' fill='none' stroke='#ccc' stroke-width='0.6'/>");
    }
    let axis: String = (0..=400).map(|i| {
        let p = al.at(al.length * i as f64 / 400.0).unwrap();
        let (x, y) = px(p.x, p.y);
        format!("{x:.1},{y:.1} ")
    }).collect();
    svg += &format!("<polyline points='{axis}' fill='none' stroke='#1d4ed8' stroke-width='2'/>");
    let stations = [100.0, 210.0, 276.0, 330.0, 500.0];
    for (i, s) in stations.iter().enumerate() {
        let line = sample_line(&al, *s, 35.0, 35.0).unwrap();
        let (a, b) = (px(line.ends[0][0], line.ends[0][1]), px(line.ends[1][0], line.ends[1][1]));
        svg += &format!("<line x1='{:.1}' y1='{:.1}' x2='{:.1}' y2='{:.1}' stroke='#d22' stroke-width='1.5'/>", a.0, a.1, b.0, b.1);
        svg += &format!("<text x='{:.1}' y='{:.1}' font-size='12' fill='#d22'>{}</text>", b.0 + 4.0, b.1, i + 1);
        // Its terrain, below.
        let g = ground(&surface, &line);
        let (ox, oy) = (40.0 + i as f64 * 270.0, 900.0);
        let zs: Vec<f64> = g.iter().filter_map(|p| p.z).collect();
        let lo = zs.iter().cloned().fold(f64::INFINITY, f64::min);
        let sx = 3.4;
        let sy = 20.0;
        svg += &format!("<rect x='{ox}' y='{}' width='{}' height='300' fill='none' stroke='#999'/>", oy - 300.0, 70.0 * sx);
        svg += &format!("<line x1='{0}' y1='{1}' x2='{0}' y2='{2}' stroke='#1d4ed8' stroke-dasharray='4 3'/>", ox + 35.0 * sx, oy - 300.0, oy);
        let mut d = String::new();
        for p in &g {
            if let Some(z) = p.z {
                let (x, y) = (ox + (p.offset + 35.0) * sx, oy - 20.0 - (z - lo) * sy);
                d += &format!("{}{x:.1},{y:.1} ", if d.is_empty() || p.gap { "M" } else { "L" });
                if p.crossing {
                    svg += &format!("<circle cx='{x:.1}' cy='{y:.1}' r='2' fill='#222'/>");
                }
            }
        }
        svg += &format!("<path d='{d}' fill='none' stroke='#222' stroke-width='1.5'/>");
        svg += &format!("<text x='{}' y='{}' font-size='13'>{} · km {s} · {} yer</text>", ox + 4.0, oy - 284.0, i + 1, g.len());
    }
    svg += "<text x='20' y='26' font-size='15'>Y5.1: kesit hatları aksa dik (sol 35, sağ 35 m); altta her hattın arazi kesiti, noktalar üçgen kenarlarında kesin (düşey abartma ~6)</text></svg>";
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../.run/gui-evidence");
    std::fs::write(dir.join("yol-y5-kesit-hatlari.svg"), svg).unwrap();
}
