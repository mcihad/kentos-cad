//! The horizontal alignment against the shared cases (fixtures/road/v1/alignments.json, docs/adr/0901): written by
//! the textbook PI formulas and mpmath at 40 digits (scripts/fixtures/road_alignment_cases.py), no KentOS code.

use kentos_road::alignment::{
    ChainError, Element, End, Equation, Kind, PiInput, Stationing, StationingError, build, offset,
};
use serde_json::Value;

const CASES: &str = include_str!("../../../../../fixtures/road/v1/alignments.json");

/// Places within this of the reference (large coordinates: 1e-8 of 4.4e6 m is the double's own step).
const PLACE: f64 = 1e-8;
const ANGLE: f64 = 1e-11;

fn n(v: &Value) -> f64 {
    v.as_f64().expect("a number")
}

fn near(a: f64, b: f64, tol: f64, what: &str) {
    assert!((a - b).abs() <= tol, "{what}: {a} against {b} ({:e})", (a - b).abs());
}

fn pis(v: &Value) -> Vec<PiInput> {
    v.as_array()
        .expect("pis")
        .iter()
        .map(|p| PiInput {
            id: p["id"].as_str().expect("id").to_owned(),
            x: n(&p["x"]),
            y: n(&p["y"]),
            radius: n(&p["radius"]),
            spiral_in: n(&p["spiralIn"]),
            spiral_out: n(&p["spiralOut"]),
        })
        .collect()
}

fn error_kind(e: &ChainError) -> &'static str {
    match e {
        ChainError::TooFewPoints => "TooFewPoints",
        ChainError::DuplicateId { .. } => "DuplicateId",
        ChainError::NotFinite { .. } => "NotFinite",
        ChainError::SamePlace { .. } => "SamePlace",
        ChainError::BadCurve { .. } => "BadCurve",
        ChainError::TurnTooSharp { .. } => "TurnTooSharp",
        ChainError::SpiralsLong { .. } => "SpiralsLong",
        ChainError::CurvesOverlap { .. } => "CurvesOverlap",
    }
}

#[test]
fn every_case_is_what_the_reference_says() {
    let all: Value = serde_json::from_str(CASES).expect("json");
    let cases = all["cases"].as_array().expect("cases");
    assert!(cases.len() >= 20);
    for c in cases {
        let name = c["name"].as_str().expect("name");
        if let Some(e) = c.get("element") {
            element(name, e, c);
            continue;
        }
        let built = build(&pis(&c["pis"]));
        if let Some(want) = c.get("error") {
            let got = built.expect_err(name);
            assert_eq!(error_kind(&got), want["kind"].as_str().expect("kind"), "{name}");
            match &got {
                ChainError::CurvesOverlap { a, b, excess } => {
                    assert_eq!((a.as_str(), b.as_str()), (want["a"].as_str().unwrap(), want["b"].as_str().unwrap()));
                    near(*excess, n(&want["excess"]), PLACE, name);
                }
                ChainError::SpiralsLong { pi, turn, delta } => {
                    assert_eq!(pi, want["pi"].as_str().unwrap());
                    near(*turn, n(&want["turn"]), ANGLE, name);
                    near(*delta, n(&want["delta"]), ANGLE, name);
                }
                ChainError::TurnTooSharp { pi }
                | ChainError::SamePlace { pi }
                | ChainError::BadCurve { pi }
                | ChainError::DuplicateId { pi } => assert_eq!(pi, want["pi"].as_str().unwrap(), "{name}"),
                _ => {}
            }
            continue;
        }
        let al = built.unwrap_or_else(|e| panic!("{name}: {e:?}"));
        near(al.length, n(&c["length"]), PLACE, name);
        assert!(al.residual.place < 1e-9 && al.residual.direction < 1e-12, "{name}: {:?}", al.residual);
        let want = c["elements"].as_array().expect("elements");
        assert_eq!(al.elements.len(), want.len(), "{name}: elements");
        for (e, w) in al.elements.iter().zip(want) {
            let what = format!("{name} {}", e.id);
            assert_eq!(e.id, w["id"].as_str().unwrap(), "{what}");
            let kind = match e.kind {
                Kind::Tangent => "tangent",
                Kind::Arc => "arc",
                Kind::Spiral => "spiral",
            };
            assert_eq!(kind, w["kind"].as_str().unwrap(), "{what}");
            near(e.station, n(&w["station"]), PLACE, &what);
            near(e.length, n(&w["length"]), PLACE, &what);
            near(e.k0, n(&w["k0"]), 1e-15, &what);
            near(e.k1, n(&w["k1"]), 1e-15, &what);
            near(e.start[0], n(&w["start"][0]), PLACE, &what);
            near(e.start[1], n(&w["start"][1]), PLACE, &what);
            near(e.theta0, n(&w["theta0"]), ANGLE, &what);
        }
        for (r, w) in al.curves.iter().zip(c["curves"].as_array().expect("curves")) {
            let what = format!("{name} {}", r.pi);
            assert_eq!(r.pi, w["pi"].as_str().unwrap());
            for (a, b) in [
                (r.delta, &w["delta"]),
                (r.t_in, &w["tIn"]),
                (r.t_out, &w["tOut"]),
                (r.arc, &w["arc"]),
                (r.p_in, &w["pIn"]),
                (r.p_out, &w["pOut"]),
                (r.centre[0], &w["centre"][0]),
                (r.centre[1], &w["centre"][1]),
                (r.radius, &w["radius"]),
            ] {
                near(a, n(b), PLACE, &what);
            }
            for (a, b) in r.stations.iter().zip(w["stations"].as_array().unwrap()) {
                near(*a, n(b), PLACE, &what);
            }
        }
        let joins: Vec<(String, String, bool)> =
            al.joins.iter().map(|j| (j.between[0].clone(), j.between[1].clone(), j.reverse)).collect();
        let want: Vec<(String, String, bool)> = c["joins"]
            .as_array()
            .unwrap()
            .iter()
            .map(|j| {
                (
                    j["between"][0].as_str().unwrap().to_owned(),
                    j["between"][1].as_str().unwrap().to_owned(),
                    j["reverse"].as_bool().unwrap(),
                )
            })
            .collect();
        assert_eq!(joins, want, "{name}: joins");
        for w in c["at"].as_array().unwrap() {
            let s = n(&w["station"]);
            let got = al.at(s).unwrap_or_else(|| panic!("{name}: at {s}"));
            let what = format!("{name} at {s}");
            near(got.x, n(&w["x"]), PLACE, &what);
            near(got.y, n(&w["y"]), PLACE, &what);
            near(got.direction, n(&w["direction"]), ANGLE, &what);
            near(got.curvature, n(&w["curvature"]), 1e-15, &what);
        }
        for w in c["locate"].as_array().unwrap() {
            let p = [n(&w["point"][0]), n(&w["point"][1])];
            let got = al.locate(p);
            let what = format!("{name} locate {p:?}");
            let feet = w["feet"].as_array().unwrap();
            assert_eq!(got.feet.len(), feet.len(), "{what}: {:?}", got.feet);
            for (f, wf) in got.feet.iter().zip(feet) {
                near(f.at.station, n(&wf["station"]), 1e-6, &what);
                near(f.offset, n(&wf["offset"]), 1e-8, &what);
            }
            match (&got.beyond, w["beyond"].as_object()) {
                (None, None) => {}
                (Some((end, foot)), Some(b)) => {
                    let want = if b["end"] == "start" { End::Start } else { End::End };
                    assert_eq!(*end, want, "{what}");
                    near(foot.at.station, n(&b["station"]), PLACE, &what);
                }
                other => panic!("{what}: beyond {other:?}"),
            }
        }
        for w in c["offset"].as_array().unwrap() {
            let d = n(&w["d"]);
            let what = format!("{name} offset {d}");
            match w.get("cusp") {
                Some(cusp) => match offset::check(&al, d) {
                    Err(offset::OffsetError::Cusp { from, to }) => {
                        near(from, n(&cusp[0]), 1e-7, &what);
                        near(to, n(&cusp[1]), 1e-7, &what);
                    }
                    other => panic!("{what}: {other:?}"),
                },
                None => {
                    offset::check(&al, d).unwrap_or_else(|e| panic!("{what}: {e:?}"));
                    near(offset::length(&al, d), n(&w["length"]), PLACE, &what);
                }
            }
        }
        if let Some(st) = c.get("stationing") {
            let stationing = Stationing {
                start: n(&st["start"]),
                equations: st["equations"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|q| Equation {
                        back: n(&q["back"]),
                        ahead: n(&q["ahead"]),
                        increasing: q["increasing"].as_bool().unwrap(),
                    })
                    .collect(),
            };
            let regions = stationing.regions(al.length);
            if let Some(i) = c.get("stationingError") {
                assert_eq!(regions, Err(StationingError::Outside { equation: i.as_u64().unwrap() as usize }), "{name}");
                continue;
            }
            let regions = regions.expect(name);
            for w in c["km"].as_array().unwrap() {
                near(Stationing::km(&regions, n(&w["station"])), n(&w["km"]), PLACE, name);
            }
            for w in c["kmStations"].as_array().unwrap() {
                let got = Stationing::stations(&regions, n(&w["km"]));
                let want = w["stations"].as_array().unwrap();
                assert_eq!(got.len(), want.len(), "{name}: km {}", w["km"]);
                for ((region, s), ws) in got.iter().zip(want) {
                    assert_eq!(*region as u64, ws[0].as_u64().unwrap());
                    near(*s, n(&ws[1]), PLACE, name);
                }
            }
        }
    }
}

fn element(name: &str, e: &Value, c: &Value) {
    let el = Element {
        id: "e".into(),
        kind: Kind::Spiral,
        start: [n(&e["start"][0]), n(&e["start"][1])],
        theta0: n(&e["theta0"]),
        length: n(&e["length"]),
        k0: n(&e["k0"]),
        k1: n(&e["k1"]),
        station: 0.0,
    };
    for w in c["points"].as_array().unwrap() {
        let s = n(&w["s"]);
        let [x, y] = el.point(s);
        let what = format!("{name} {s}");
        near(x, n(&w["x"]), 1e-10, &what);
        near(y, n(&w["y"]), 1e-10, &what);
        near(el.theta(s), n(&w["direction"]), ANGLE, &what);
        near(el.curvature(s), n(&w["curvature"]), 1e-15, &what);
    }
}

/// A picture of the core's own output: the five-PI case, its elements by kind, km every 100 m, a parallel axis each
/// side and a point's foot (`.run/gui-evidence/yol-y3-cekirdek.svg`).
#[test]
#[ignore = "writes a picture"]
fn picture() {
    use kentos_road::alignment::km_text;
    let all: Value = serde_json::from_str(CASES).expect("json");
    let c = all["cases"].as_array().unwrap().iter().find(|c| c["name"] == "Büyük koordinatlarda beş PI").unwrap();
    let raw = pis(&c["pis"]);
    let al = build(&raw).expect("builds");
    let (x0, y0) = (raw[0].x - 100.0, raw[0].y - 700.0);
    let k = 0.4;
    let sx = |x: f64| (x - x0) * k;
    let sy = |y: f64| 520.0 - (y - y0) * k;
    let mut svg = String::from(
        "<svg xmlns='http://www.w3.org/2000/svg' width='1700' height='1700' viewBox='0 0 1700 1700' style='background:#fff;font-family:Helvetica'>",
    );
    let path = |pts: &[[f64; 2]]| {
        pts.iter().map(|p| format!("{:.2},{:.2} ", sx(p[0]), sy(p[1]))).collect::<String>()
    };
    let pi_line: Vec<[f64; 2]> = raw.iter().map(|p| [p.x, p.y]).collect();
    svg += &format!("<polyline points='{}' fill='none' stroke='#bbb' stroke-dasharray='6 4'/>", path(&pi_line));
    for d in [-7.5, 7.5] {
        let pts: Vec<[f64; 2]> =
            (0..=(al.length as usize)).filter_map(|s| offset::place(&al, s as f64, d)).collect();
        svg += &format!("<polyline points='{}' fill='none' stroke='#8a8' stroke-width='1'/>", path(&pts));
    }
    for e in &al.elements {
        let n = (e.length / 2.0).ceil() as usize;
        let pts: Vec<[f64; 2]> = (0..=n).map(|i| e.point(e.length * i as f64 / n as f64)).collect();
        let color = match e.kind {
            Kind::Tangent => "#222",
            Kind::Arc => "#c33",
            Kind::Spiral => "#27c",
        };
        svg += &format!("<polyline points='{}' fill='none' stroke='{color}' stroke-width='3'/>", path(&pts));
    }
    let mut s = 0.0;
    while s <= al.length {
        let at = al.at(s).unwrap();
        let (nx, ny) = (at.direction.sin(), -at.direction.cos());
        let long = (s as i64) % 500 == 0;
        let l = if long { 18.0 } else { 9.0 };
        svg += &format!(
            "<line x1='{:.1}' y1='{:.1}' x2='{:.1}' y2='{:.1}' stroke='#222'/>",
            sx(at.x - nx * l / k),
            sy(at.y - ny * l / k),
            sx(at.x + nx * l / k),
            sy(at.y + ny * l / k)
        );
        if long {
            svg += &format!(
                "<text x='{:.1}' y='{:.1}' font-size='13'>{}</text>",
                sx(at.x + nx * 30.0 / k),
                sy(at.y + ny * 30.0 / k),
                km_text(12000.0 + s, 0)
            );
        }
        s += 100.0;
    }
    for p in &raw {
        svg += &format!("<circle cx='{:.1}' cy='{:.1}' r='4' fill='#fff' stroke='#555'/><text x='{:.1}' y='{:.1}' font-size='13' fill='#555'>{}</text>", sx(p.x), sy(p.y), sx(p.x) + 6.0, sy(p.y) - 6.0, p.id);
    }
    let q = [raw[0].x + 1500.0, raw[0].y - 150.0];
    let f = al.locate(q).nearest.unwrap();
    svg += &format!(
        "<line x1='{:.1}' y1='{:.1}' x2='{:.1}' y2='{:.1}' stroke='#d80' stroke-dasharray='4 3'/><circle cx='{:.1}' cy='{:.1}' r='4' fill='#d80'/><text x='{:.1}' y='{:.1}' font-size='13' fill='#d80'>km {}  öteleme {:+.3} m</text>",
        sx(q[0]), sy(q[1]), sx(f.at.x), sy(f.at.y), sx(q[0]), sy(q[1]), sx(q[0]) + 8.0, sy(q[1]) + 16.0,
        km_text(12000.0 + f.at.station, 3), f.offset
    );
    svg += "<text x='20' y='30' font-size='15'>kentos.alignment/1 — siyah aliyman, mavi klotoid, kırmızı kurp, yeşil ±7,5 m paralel aks</text></svg>";
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../.run/gui-evidence");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("yol-y3-cekirdek.svg"), svg).unwrap();
}
