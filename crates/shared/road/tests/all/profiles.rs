//! The terrain profile against the shared cases (fixtures/road/v1/profiles.json, docs/adr/0902 §1): written by walking
//! the alignment on Triangle's surface and refining each change of triangle at 40 digits
//! (scripts/fixtures/road_profile_cases.py), no KentOS code.

use kentos_road::alignment::{PiInput, build as align};
use kentos_road::profile::{TOLERANCE, terrain};
use kentos_road::surface::{SurfaceInput, build};
use serde_json::Value;

const CASES: &str = include_str!("../../../../../fixtures/road/v1/profiles.json");

fn n(v: &Value) -> f64 {
    v.as_f64().expect("a number")
}

#[test]
fn every_profile_crosses_where_the_reference_says() {
    let all: Value = serde_json::from_str(CASES).expect("json");
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
            })
            .collect();
        let al = align(&pis).expect("an alignment");
        let got = terrain(&surface, &al);
        let crossings: Vec<_> = got.iter().filter(|p| p.crossing).collect();
        let want = c["crossings"].as_array().unwrap();
        assert_eq!(crossings.len(), want.len(), "{name}: crossings");
        for (g, w) in crossings.iter().zip(want) {
            assert!((g.station - n(&w["station"])).abs() < 1e-7, "{name}: {} vs {}", g.station, w["station"]);
            let z = g.z.expect("a crossing has a height");
            assert!((z - n(&w["z"])).abs() < 1e-7, "{name}: {z} vs {}", w["z"]);
        }
        // The stretches off the surface: each mark's `gap` (the stretch before it).
        let off: Vec<bool> = c["off"].as_array().unwrap().iter().map(|v| v.as_bool().unwrap()).collect();
        let marks: Vec<_> = got.iter().filter(|p| p.crossing || p.station == 0.0 || p.station == al.length).collect();
        assert_eq!(marks.len(), off.len() + 1, "{name}: marks");
        for (m, o) in marks[1..].iter().zip(&off) {
            assert_eq!(m.gap, *o, "{name}: the stretch before {}", m.station);
        }
        assert!(got.windows(2).all(|w| w[0].station < w[1].station), "{name}: in order");
        // A linear reading is within the tolerance of the true height.
        for w in c["checks"].as_array().unwrap() {
            let s = n(&w["station"]);
            let i = got.partition_point(|p| p.station <= s).clamp(1, got.len() - 1);
            let (a, b) = (got[i - 1], got[i]);
            match (w["z"].as_f64(), a.z, b.z, b.gap) {
                (Some(z), Some(za), Some(zb), false) => {
                    let lin = za + (zb - za) * (s - a.station) / (b.station - a.station);
                    assert!((lin - z).abs() <= TOLERANCE + 1e-9, "{name}: at {s}: {lin} vs {z}");
                }
                (None, _, _, gap) => assert!(gap || a.z.is_none() || b.z.is_none(), "{name}: {s} off the surface"),
                other => panic!("{name}: at {s}: {other:?}"),
            }
        }
    }
}

/// A picture of the core's own output: a terrain profile and a vertical alignment over it, ten times exaggerated
/// (`.run/gui-evidence/yol-y4-cekirdek.svg`).
#[test]
#[ignore = "writes a picture"]
fn picture() {
    use kentos_road::profile::vertical::{Pvi, VCurve, build as vbuild};
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
        })
        .collect();
    let al = align(&pis).unwrap();
    let ground = terrain(&surface, &al);
    let zs: Vec<f64> = ground.iter().filter_map(|p| p.z).collect();
    let (lo, hi) = (zs.iter().cloned().fold(f64::INFINITY, f64::min), zs.iter().cloned().fold(f64::NEG_INFINITY, f64::max));
    let l = al.length;
    let pv = |id: &str, s: f64, z: f64, curve| Pvi { id: id.into(), station: s, elevation: z, curve };
    let v = vbuild(&[
        pv("a", 0.0, ground[0].z.unwrap_or(lo) + 1.0, VCurve::None),
        pv("v1", l * 0.3, lo + 1.5, VCurve::Parabola { length: 80.0 }),
        pv("v2", l * 0.62, hi - 1.0, VCurve::Circle { radius: 1200.0 }),
        pv("v3", l * 0.85, lo + 3.0, VCurve::Asymmetric { length_in: 50.0, length_out: 35.0 }),
        pv("b", l, hi - 2.0, VCurve::None),
    ])
    .expect("a vertical alignment");
    let (w, h, m) = (1600.0, 600.0, 70.0);
    let ex = 10.0;
    let xs = (w - 2.0 * m) / l;
    let ys = xs * ex;
    let base = lo - 2.0;
    let sx = |s: f64| m + s * xs;
    let sy = |z: f64| h - 140.0 - (z - base) * ys;
    let poly = |pts: &[(f64, f64)], color: &str, width: f64| {
        let p: String = pts.iter().map(|(s, z)| format!("{:.2},{:.2} ", sx(*s), sy(*z))).collect();
        format!("<polyline points='{p}' fill='none' stroke='{color}' stroke-width='{width}'/>")
    };
    let mut svg = format!("<svg xmlns='http://www.w3.org/2000/svg' width='{w}' height='{w}' viewBox='0 0 {w} {w}' style='background:#fff;font-family:Helvetica'>");
    // The grid: every 50 m and every metre of height.
    let mut s = 0.0;
    while s <= l {
        svg += &format!("<line x1='{0:.1}' y1='{1:.1}' x2='{0:.1}' y2='{2:.1}' stroke='#e4e4e4'/>", sx(s), sy(base), sy(hi + 3.0));
        s += 50.0;
    }
    let mut z = base.ceil();
    while z <= hi + 3.0 {
        svg += &format!("<line x1='{0:.1}' y1='{1:.1}' x2='{2:.1}' y2='{1:.1}' stroke='#eeeeee'/><text x='8' y='{3:.1}' font-size='11' fill='#777'>{z:.0}</text>", sx(0.0), sy(z), sx(l), sy(z) + 4.0);
        z += 1.0;
    }
    let g: Vec<(f64, f64)> = ground.iter().filter_map(|p| p.z.map(|z| (p.station, z))).collect();
    svg += &poly(&g, "#222", 1.4);
    let red: Vec<(f64, f64)> = (0..=800).map(|i| l * i as f64 / 800.0).filter_map(|s| v.at(s).map(|(z, _, _)| (s, z))).collect();
    svg += &poly(&red, "#d22", 2.2);
    for c in &v.curves {
        for p in [c.start, c.end] {
            svg += &format!("<circle cx='{:.1}' cy='{:.1}' r='3' fill='#d22'/>", sx(p[0]), sy(p[1]));
        }
        let kind = match c.curve {
            VCurve::Parabola { .. } => "parabol",
            VCurve::Asymmetric { .. } => "asimetrik",
            VCurve::Circle { .. } => "çember",
            VCurve::None => "",
        };
        svg += &format!(
            "<text x='{:.1}' y='{:.1}' font-size='12' fill='#d22'>{} {kind} K={:.1}</text>",
            sx(c.start[0]),
            sy(c.start[1].max(c.end[1])) - 14.0,
            c.pvi,
            c.k.unwrap_or(0.0)
        );
    }
    // The bands: km, black and red elevations every 50 m.
    let mut s = 0.0;
    let y0 = h - 110.0;
    svg += &format!("<text x='8' y='{:.1}' font-size='12'>Km</text><text x='8' y='{:.1}' font-size='12'>Siyah kot</text><text x='8' y='{:.1}' font-size='12' fill='#d22'>Kırmızı kot</text>", y0, y0 + 30.0, y0 + 60.0);
    while s <= l {
        let black = al.at(s).and_then(|p| surface.height(p.x, p.y).z);
        let red = v.at(s).map(|(z, _, _)| z);
        svg += &format!(
            "<text x='{0:.1}' y='{1:.1}' font-size='11' text-anchor='middle'>{2}</text><text x='{0:.1}' y='{3:.1}' font-size='11' text-anchor='middle'>{4}</text><text x='{0:.1}' y='{5:.1}' font-size='11' text-anchor='middle' fill='#d22'>{6}</text>",
            sx(s),
            y0,
            kentos_road::alignment::km_text(s, 0),
            y0 + 30.0,
            black.map_or("—".into(), |z| format!("{z:.2}")),
            y0 + 60.0,
            red.map_or("—".into(), |z| format!("{z:.2}"))
        );
        s += 50.0;
    }
    svg += &format!(
        "<text x='20' y='30' font-size='15'>kentos.profile/1 — siyah: arazi ({} kesişim, üçgen kenarlarında kesin), kırmızı: düşey güzergâh (parabol, çember, asimetrik), düşey abartma 10</text></svg>",
        ground.iter().filter(|p| p.crossing).count()
    );
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../.run/gui-evidence");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("yol-y4-cekirdek.svg"), svg).unwrap();
}
