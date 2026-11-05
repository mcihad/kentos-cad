//! A road corridor against the shared cases (fixtures/road/v1/corridor.json, docs/adr/0905): written by
//! scripts/fixtures/road_corridor_cases.py from the ADR's rules and the reference alignment, no KentOS code.

use kentos_road::alignment::{PiInput, build};
use kentos_road::corridor::{
    Cross, CorridorError, Frequency, RegionCrosses, Role, Station, Unsolved, Why, assemble, check_regions, stations,
};
use kentos_road::section::SampleLine;
use kentos_road::section::template::{Component, Ditch, Kind, Section, Side, SideSection, Template};
use serde_json::{Value, json};

const CASES: &str = include_str!("../../../../../fixtures/road/v1/corridor.json");
const WITHIN: f64 = 1e-6;

fn n(v: &Value) -> f64 {
    v.as_f64().expect("a number")
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

fn error(e: CorridorError) -> Value {
    match e {
        CorridorError::EmptyRegion { index } => json!({"kind": "EmptyRegion", "index": index}),
        CorridorError::Outside { index } => json!({"kind": "Outside", "index": index}),
        CorridorError::Overlap { first, second } => json!({"kind": "Overlap", "first": first, "second": second}),
        CorridorError::BadFrequency => json!({"kind": "BadFrequency"}),
        CorridorError::TooMany => json!({"kind": "TooMany"}),
    }
}

const WHYS: [(Why, &str); 7] = [
    (Why::Region, "region"),
    (Why::Geometry, "geometry"),
    (Why::Critical, "critical"),
    (Why::Profile, "profile"),
    (Why::User, "user"),
    (Why::Curve, "curve"),
    (Why::Interval, "interval"),
];

fn why(w: Why) -> &'static str {
    WHYS.iter().find(|x| x.0 == w).map(|x| x.1).expect("a reason")
}

fn why_of(word: &str) -> Why {
    WHYS.iter().find(|x| x.1 == word).map(|x| x.0).expect("a reason")
}

#[test]
fn regions_are_judged_as_the_reference_says() {
    let all: Value = serde_json::from_str(CASES).expect("json");
    for c in all["regions"].as_array().expect("cases") {
        let name = c["name"].as_str().expect("name");
        let rs: Vec<[f64; 2]> = c["regions"].as_array().expect("regions").iter().map(|r| [n(&r[0]), n(&r[1])]).collect();
        match (check_regions(&rs, n(&c["length"])), c.get("error")) {
            (Err(e), Some(want)) => assert_eq!(&error(e), want, "{name}"),
            (Ok(gaps), None) => {
                let want: Vec<[f64; 2]> = c["gaps"].as_array().expect("gaps").iter().map(|g| [n(&g[0]), n(&g[1])]).collect();
                assert_eq!(gaps, want, "{name}");
            }
            (got, want) => panic!("{name}: {got:?} vs {want:?}"),
        }
    }
}

#[test]
fn stations_are_what_the_reference_samples() {
    let all: Value = serde_json::from_str(CASES).expect("json");
    for c in all["stations"].as_array().expect("cases") {
        let name = c["name"].as_str().expect("name");
        let al = build(&pis(&c["pis"])).expect("the alignment builds");
        let f = Frequency { tangent: n(&c["tangent"]), curve: n(&c["curve"]), sag: c["sag"].as_f64() };
        let events: Vec<Station> = c["events"]
            .as_array()
            .expect("events")
            .iter()
            .map(|e| Station { station: n(&e[0]), why: why_of(e[1].as_str().expect("why")) })
            .collect();
        let got = stations(&al, [n(&c["region"][0]), n(&c["region"][1])], &f, &events);
        match (got, c.get("error")) {
            (Err(e), Some(want)) => assert_eq!(&error(e), want, "{name}"),
            (Ok(got), None) => {
                let want = c["stations"].as_array().expect("stations");
                assert_eq!(got.len(), want.len(), "{name}: {got:?}");
                for (g, w) in got.iter().zip(want) {
                    assert!((g.station - n(&w[0])).abs() < WITHIN, "{name}: {g:?} vs {w}");
                    assert_eq!(why(g.why), w[1].as_str().expect("why"), "{name}: {g:?} vs {w}");
                }
            }
            (got, want) => panic!("{name}: {got:?} vs {want:?}"),
        }
    }
}

fn side(v: &Value) -> SideSection {
    let pt = |p: &Value| [n(&p[0]), n(&p[1])];
    SideSection {
        points: v["points"].as_array().expect("points").iter().map(pt).collect(),
        hinge: pt(&v["hinge"]),
        daylight: (!v["daylight"].is_null()).then(|| pt(&v["daylight"])),
        kind: match v["kind"].as_str().expect("kind") {
            "cut" => Kind::Cut,
            "fill" => Kind::Fill,
            _ => Kind::Open,
        },
    }
}

fn template_side(v: &Value) -> Side {
    Side {
        components: match v.get(2).and_then(Value::as_array) {
            Some(kinds) => kinds
                .iter()
                .map(|k| match k.as_str() {
                    Some("shoulder") => Component::Shoulder { width: 1.0, slope: 0.0 },
                    _ => Component::Lane { width: 1.0, slope: 0.0 },
                })
                .collect(),
            None => vec![Component::Lane { width: 1.0, slope: 0.0 }; v[0].as_u64().expect("count") as usize],
        },
        cut: 1.0,
        fill: 1.0,
        ditch: v[1].as_f64().map(|bottom| Ditch { fore: 1.0, depth: 1.0, bottom }),
        bench: None,
        fill_bench: None,
    }
}

fn regions(v: &Value) -> Vec<RegionCrosses> {
    v.as_array()
        .expect("regions")
        .iter()
        .map(|r| RegionCrosses {
            template: Template { left: template_side(&r["template"]["left"]), right: template_side(&r["template"]["right"]) },
            crosses: r["crosses"]
                .as_array()
                .expect("crosses")
                .iter()
                .map(|c| Cross {
                    line: SampleLine {
                        station: n(&c["station"]),
                        left: 30.0,
                        right: 30.0,
                        ends: [[0.0, 0.0]; 2],
                        at: [n(&c["at"][0]), n(&c["at"][1])],
                        direction: n(&c["direction"]),
                    },
                    section: (!c["section"].is_null()).then(|| Section {
                        axis: n(&c["section"]["axis"]),
                        left: side(&c["section"]["left"]),
                        right: side(&c["section"]["right"]),
                        cut: 0.0,
                        fill: 0.0,
                    }),
                })
                .collect(),
        })
        .collect()
}

fn close(a: &[f64], b: &Value, what: &str) {
    let b = b.as_array().expect("numbers");
    assert_eq!(a.len(), b.len(), "{what}");
    for (x, y) in a.iter().zip(b) {
        assert!((x - n(y)).abs() < WITHIN, "{what}: {a:?} vs {b:?}");
    }
}

#[test]
fn sections_are_joined_as_the_reference_joins_them() {
    let all: Value = serde_json::from_str(CASES).expect("json");
    for c in all["assemble"].as_array().expect("cases") {
        let name = c["name"].as_str().expect("name");
        let built = assemble(&regions(&c["regions"]));
        let mut features = built.features.clone();
        features.sort_by(|a, b| (a.region, &a.code).cmp(&(b.region, &b.code)).then(a.points[0][0].total_cmp(&b.points[0][0])));
        let want = c["features"].as_array().expect("features");
        assert_eq!(features.len(), want.len(), "{name}: {:?}", features.iter().map(|f| &f.code).collect::<Vec<_>>());
        for (f, w) in features.iter().zip(want) {
            assert_eq!((f.code.as_str(), f.region), (w["code"].as_str().expect("code"), w["region"].as_u64().expect("r") as usize), "{name}");
            let pts = w["points"].as_array().expect("points");
            assert_eq!(f.points.len(), pts.len(), "{name}: {}", f.code);
            for (p, q) in f.points.iter().zip(pts) {
                close(p, q, &format!("{name}: {}", f.code));
            }
        }
        let tris = c["triangles"].as_array().expect("triangles");
        assert_eq!(built.triangles.len(), tris.len(), "{name}: the triangles");
        for (t, w) in built.triangles.iter().zip(tris) {
            for (p, q) in t.iter().zip(w.as_array().expect("corners")) {
                close(p, q, &format!("{name}: a triangle"));
            }
        }
        let roles: Vec<&str> = built
            .roles
            .iter()
            .map(|r| match r {
                Role::Lane => "lane",
                Role::Shoulder => "shoulder",
                Role::Cut => "cut",
                Role::Fill => "fill",
            })
            .collect();
        let want: Vec<&str> = c["roles"].as_array().expect("roles").iter().map(|r| r.as_str().expect("a role")).collect();
        assert_eq!(roles, want, "{name}: the triangles' roles");
        let mut unsolved = built.unsolved.clone();
        unsolved.sort_by(|a, b| (a.region, a.right).cmp(&(b.region, b.right)).then(a.from.total_cmp(&b.from)));
        let got: Vec<Value> = unsolved
            .iter()
            .map(|u| {
                let why = match u.why {
                    Unsolved::NoSection => "noSection",
                    Unsolved::Open => "open",
                };
                json!({"region": u.region, "right": u.right, "why": why, "from": u.from, "to": u.to})
            })
            .collect();
        let want = c["unsolved"].as_array().expect("unsolved");
        assert_eq!(got.len(), want.len(), "{name}: {got:?}");
        for (g, w) in got.iter().zip(want) {
            assert_eq!((&g["region"], &g["right"], &g["why"]), (&w["region"], &w["right"], &w["why"]), "{name}");
            close(&[n(&g["from"]), n(&g["to"])], &json!([w["from"], w["to"]]), name);
        }
        let want = c["folded"].as_array().expect("folded");
        assert_eq!(built.folded.len(), want.len(), "{name}: {:?}", built.folded);
        for (f, w) in built.folded.iter().zip(want) {
            close(&[f.0 as f64, f.1, f.2], w, name);
        }
    }
}
