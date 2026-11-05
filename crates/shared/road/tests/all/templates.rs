//! A template's sections against the shared cases (fixtures/road/v1/templates.json, docs/adr/0903 §3–§5): written in
//! exact fractions by scripts/fixtures/road_template_cases.py, no KentOS code.

use kentos_road::section::GroundPoint;
use kentos_road::section::template::{Component, Ditch, Kind, Side, Template, TemplateError, build};
use serde_json::Value;

const CASES: &str = include_str!("../../../../../fixtures/road/v1/templates.json");
const WITHIN: f64 = 1e-9;

fn n(v: &Value) -> f64 {
    v.as_f64().expect("a number")
}

fn side(v: &Value) -> Side {
    Side {
        components: v["components"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| match c["kind"].as_str().unwrap() {
                "lane" => Component::Lane { width: n(&c["width"]), slope: n(&c["slope"]) },
                _ => Component::Shoulder { width: n(&c["width"]), slope: n(&c["slope"]) },
            })
            .collect(),
        cut: n(&v["cut"]),
        fill: n(&v["fill"]),
        ditch: v.get("ditch").map(|d| Ditch { fore: n(&d["fore"]), depth: n(&d["depth"]), bottom: n(&d["bottom"]) }),
        bench: v.get("bench").map(|b| kentos_road::section::template::CutBench { height: n(&b["height"]), width: n(&b["width"]), fall: b.get("fall").map_or(0.0, n) }),
    }
}

fn close(a: &[f64; 2], b: &Value, what: &str) {
    assert!((a[0] - n(&b[0])).abs() < WITHIN && (a[1] - n(&b[1])).abs() < WITHIN, "{what}: {a:?} vs {b}");
}

#[test]
fn every_template_builds_what_the_reference_says() {
    let all: Value = serde_json::from_str(CASES).expect("json");
    for c in all["cases"].as_array().expect("cases") {
        let name = c["name"].as_str().unwrap();
        let t = Template { left: side(&c["template"]["left"]), right: side(&c["template"]["right"]) };
        let ground: Vec<GroundPoint> = c["ground"]
            .as_array()
            .unwrap()
            .iter()
            .map(|g| GroundPoint { offset: n(&g["offset"]), z: g["z"].as_f64(), gap: g["gap"].as_bool().unwrap(), crossing: false })
            .collect();
        let got = build(&t, n(&c["axis"]), &ground);
        if let Some(e) = c.get("error") {
            let want = TemplateError::Bad {
                right: e["right"].as_bool().unwrap(),
                component: e["component"].as_u64().map(|i| i as usize),
            };
            assert_eq!(got, Err(want), "{name}");
            continue;
        }
        let s = got.unwrap_or_else(|e| panic!("{name}: {e:?}"));
        for (got, want, which) in [(&s.left, &c["left"], "sol"), (&s.right, &c["right"], "sağ")] {
            let kind = match got.kind {
                Kind::Cut => "cut",
                Kind::Fill => "fill",
                Kind::Open => "open",
            };
            assert_eq!(kind, want["kind"].as_str().unwrap(), "{name} {which}");
            close(&got.hinge, &want["hinge"], &format!("{name} {which} menteşe"));
            let pts = want["points"].as_array().unwrap();
            assert_eq!(got.points.len(), pts.len(), "{name} {which}: {:?}", got.points);
            for (p, w) in got.points.iter().zip(pts) {
                close(p, w, &format!("{name} {which}"));
            }
            match (&got.daylight, want["daylight"].as_array()) {
                (Some(d), Some(_)) => close(d, &want["daylight"], &format!("{name} {which} daylight")),
                (None, None) => {}
                other => panic!("{name} {which}: {other:?}"),
            }
        }
        assert!((s.cut - n(&c["cut"])).abs() < WITHIN, "{name}: cut {} vs {}", s.cut, c["cut"]);
        assert!((s.fill - n(&c["fill"])).abs() < WITHIN, "{name}: fill {} vs {}", s.fill, c["fill"]);
    }
}

/// The core's own sections drawn (`.run/gui-evidence/yol-y5-tip-kesit.svg`): six of the cases, the terrain black, the
/// platform and slopes red, the daylight points, the cut (brown) and fill (green) areas.
#[test]
#[ignore = "writes a picture"]
fn picture() {
    let all: Value = serde_json::from_str(CASES).expect("json");
    let mut svg = String::from("<svg xmlns='http://www.w3.org/2000/svg' width='1400' height='1400' style='background:#fff;font-family:Helvetica'>");
    for (k, idx) in [0usize, 2, 4, 5, 9, 10].into_iter().enumerate() {
        let c = &all["cases"][idx];
        let t = Template { left: side(&c["template"]["left"]), right: side(&c["template"]["right"]) };
        let ground: Vec<GroundPoint> = c["ground"]
            .as_array()
            .unwrap()
            .iter()
            .map(|g| GroundPoint { offset: n(&g["offset"]), z: g["z"].as_f64(), gap: g["gap"].as_bool().unwrap(), crossing: false })
            .collect();
        let s = build(&t, n(&c["axis"]), &ground).unwrap();
        let (ox, oy) = (30.0 + (k % 2) as f64 * 690.0, 60.0 + (k / 2) as f64 * 300.0);
        let zs: Vec<f64> = ground.iter().filter_map(|g| g.z).chain([s.axis]).collect();
        let lo = zs.iter().cloned().fold(f64::INFINITY, f64::min) - 2.0;
        let sx = 640.0 / 80.0;
        let sy = sx * 2.0;
        let x = |o: f64| ox + (o + 40.0) * sx;
        let y = |z: f64| oy + 250.0 - (z - lo) * sy;
        svg += &format!("<rect x='{ox}' y='{oy}' width='640' height='250' fill='none' stroke='#bbb'/>");
        // The areas: the design line and the terrain under it, as one polygon each sign would need; drawn as the
        // design-to-terrain band, coloured by the side's kind.
        let mut d = String::new();
        let mut first = true;
        for g in &ground {
            if let Some(z) = g.z {
                d += &format!("{}{:.1},{:.1} ", if first || g.gap { "M" } else { "L" }, x(g.offset), y(z));
                first = false;
            }
        }
        let design: Vec<[f64; 2]> = s.left.points.iter().rev().chain(s.right.points.iter().skip(1)).copied().collect();
        let dl: String = design.iter().map(|p| format!("{:.1},{:.1} ", x(p[0]), y(p[1]))).collect();
        svg += &format!("<path d='{d}' fill='none' stroke='#222' stroke-width='1.5'/>");
        svg += &format!("<polyline points='{dl}' fill='none' stroke='#d22' stroke-width='2.2'/>");
        for p in [s.left.daylight, s.right.daylight].into_iter().flatten() {
            svg += &format!("<circle cx='{:.1}' cy='{:.1}' r='4' fill='none' stroke='#d22' stroke-width='1.5'/>", x(p[0]), y(p[1]));
        }
        svg += &format!("<line x1='{0:.1}' y1='{1}' x2='{0:.1}' y2='{2}' stroke='#1d4ed8' stroke-dasharray='4 3'/>", x(0.0), oy, oy + 250.0);
        let kind = |k: Kind| match k {
            Kind::Cut => "yarma",
            Kind::Fill => "dolgu",
            Kind::Open => "açık",
        };
        svg += &format!(
            "<text x='{}' y='{}' font-size='13'>{} — sol {}, sağ {} · yarma {:.2} m² · dolgu {:.2} m²</text>",
            ox + 6.0,
            oy - 8.0,
            c["name"].as_str().unwrap(),
            kind(s.left.kind),
            kind(s.right.kind),
            s.cut,
            s.fill
        );
    }
    svg += "<text x='30' y='24' font-size='15'>Y5.2: tip kesit (şerit, banket, hendek, şev) ve daylight; siyah arazi, kırmızı platform ve şevler, düşey abartma 2</text></svg>";
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../.run/gui-evidence");
    std::fs::write(dir.join("yol-y5-tip-kesit.svg"), svg).unwrap();
}
