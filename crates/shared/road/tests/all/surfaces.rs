//! The terrain surface against the shared cases (fixtures/road/v1/surfaces.json, docs/adr/0900): written by
//! Shewchuk's Triangle and exact fractions (scripts/fixtures/road_surface_cases.py), no KentOS code.

use kentos_road::surface::{Place, Surface, SurfaceInput, build};
use serde_json::Value;

const CASES: &str = include_str!("../../../../../fixtures/road/v1/surfaces.json");

fn corners(s: &Surface, t: [u32; 3]) -> Vec<[f64; 2]> {
    let mut c: Vec<[f64; 2]> = t
        .iter()
        .map(|&i| [s.vertices[i as usize][0], s.vertices[i as usize][1]])
        .collect();
    c.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
    c
}

fn pairs(v: &Value) -> Vec<Vec<[f64; 2]>> {
    v.as_array()
        .expect("a list")
        .iter()
        .map(|t| {
            t.as_array()
                .expect("corners")
                .iter()
                .map(|p| [p[0].as_f64().expect("x"), p[1].as_f64().expect("y")])
                .collect()
        })
        .collect()
}

#[test]
fn every_case_is_what_the_reference_says() {
    run(CASES);
}

/// Random cases Triangle wrote (`road_surface_cases.py --stress N FILE`), not kept in the repository.
#[test]
#[ignore = "needs KENTOS_ROAD_STRESS, a file the reference writes"]
fn stress() {
    let path = std::env::var("KENTOS_ROAD_STRESS").expect("KENTOS_ROAD_STRESS");
    run(&std::fs::read_to_string(path).expect("the file"));
}

fn run(cases: &str) {
    let doc: Value = serde_json::from_str(cases).expect("JSON");
    for case in doc["cases"].as_array().expect("cases") {
        let name = case["name"].as_str().expect("a name");
        let expect = &case["expect"];
        let input: Result<SurfaceInput, _> = serde_json::from_value(case["input"].clone());
        let built = input.map_err(|e| e.to_string()).and_then(|i| build(&i).map_err(|e| e.code.to_owned()));
        if let Some(code) = expect["error"].as_str() {
            // A number that is no number fails when read (serde) or when checked; both are `invalid_input`.
            let got = match &built {
                Err(c) if c == code => true,
                Err(_) if code == "invalid_input" => true,
                _ => false,
            };
            assert!(got, "{name}: expected {code}, got {:?}", built.as_ref().map(|s| s.triangles.len()));
            continue;
        }
        let s = match built {
            Ok(s) => s,
            Err(e) => panic!("{name}: {e}"),
        };
        if let Some(n) = expect["triangleCount"].as_u64() {
            assert_eq!(s.triangles.len() as u64, n, "{name}: triangles");
        }
        if let Some(m) = expect["merged"].as_u64() {
            assert_eq!(s.report.merged as u64, m, "{name}: merged");
        }
        if let Some(want) = expect.get("triangles") {
            let mut got: Vec<Vec<[f64; 2]>> = s.triangles.iter().map(|&t| corners(&s, t)).collect();
            got.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
            assert_eq!(got, pairs(want), "{name}: the triangles are Triangle's");
        }
        if let Some(want) = expect.get("constrained") {
            let mut got: Vec<Vec<[f64; 2]>> = s
                .constrained
                .iter()
                .map(|&[a, b]| {
                    let mut c = vec![
                        [s.vertices[a as usize][0], s.vertices[a as usize][1]],
                        [s.vertices[b as usize][0], s.vertices[b as usize][1]],
                    ];
                    c.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
                    c
                })
                .collect();
            got.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
            assert_eq!(got, pairs(want), "{name}: the constrained edges");
        }
        if let Some(n) = expect["split"].as_u64() {
            assert_eq!(s.report.split as u64, n, "{name}: split");
        }
        if let Some(n) = expect["trimmed"].as_u64() {
            assert_eq!(s.report.trimmed as u64, n, "{name}: trimmed");
        }
        for h in expect["heights"].as_array().into_iter().flatten() {
            let at = [h["at"][0].as_f64().expect("x"), h["at"][1].as_f64().expect("y")];
            let got = s.height(at[0], at[1]);
            match h["z"].as_f64() {
                Some(z) => {
                    assert_eq!(got.place, Place::Inside, "{name} at {at:?}");
                    let gz = got.z.expect("a height");
                    assert!((gz - z).abs() < 1e-9, "{name} at {at:?}: {gz} ≠ {z}");
                }
                None => assert!(got.z.is_none(), "{name} at {at:?}: {got:?}"),
            }
        }
    }
}

#[test]
fn a_void_is_said_and_the_outside_too() {
    let doc: Value = serde_json::from_str(CASES).expect("JSON");
    let case = doc["cases"]
        .as_array()
        .expect("cases")
        .iter()
        .find(|c| c["name"] == "sınır ve boşluk")
        .expect("the case");
    let s = build(&serde_json::from_value(case["input"].clone()).expect("input")).expect("built");
    assert_eq!(s.height(38.0, 72.0).place, Place::Void);
    assert_eq!(s.height(2.0, 50.0).place, Place::Outside);
    assert_eq!(s.height(500.0, 500.0).place, Place::Outside);
    assert_eq!(s.height(50.0, 30.0).place, Place::Inside);
}

/// 100 000 points and 200 breaklines across them: the build and 100 000 heights (release, by hand).
#[test]
#[ignore = "timing, run by hand with --release"]
fn timing() {
    use kentos_road::surface::{Breakline, SpotPoint};
    let mut seed = 0x9e37_79b9_7f4a_7c15_u64;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed >> 11) as f64 / (1u64 << 53) as f64
    };
    let ground = |x: f64, y: f64| 100.0 + 0.05 * x - 0.02 * y + 3.0 * ((x - 500.0) / 500.0).powi(2);
    let mut input = SurfaceInput::default();
    for _ in 0..100_000 {
        let (x, y) = (next() * 1000.0, next() * 1000.0);
        input.points.push(SpotPoint { x, y, z: ground(x, y), source: None });
    }
    for k in 0..200 {
        let y = 2.5 + 5.0 * k as f64;
        input.breaklines.push(Breakline {
            points: (0..20)
                .map(|i| {
                    let x = 10.0 + 49.0 * i as f64 + next();
                    [x, y, ground(x, y)]
                })
                .collect(),
            closed: false,
            source: None,
        });
    }
    let t = std::time::Instant::now();
    let s = build(&input).expect("built");
    let built = t.elapsed();
    // A profile's places: 100 000 along a line across the surface, each near the last.
    let places: Vec<[f64; 2]> = (0..100_000)
        .map(|i| [i as f64 / 100.0, 300.0 + 0.4 * i as f64 / 100.0])
        .collect();
    let t = std::time::Instant::now();
    let hs = s.heights(&places);
    let queried = t.elapsed();
    println!(
        "{} köşe, {} üçgen, {} kısıt: kurma {built:?}, 100 000 kot {queried:?} ({} içeride)",
        s.vertices.len(),
        s.triangles.len(),
        s.constrained.len(),
        hs.iter().filter(|h| h.z.is_some()).count()
    );
}

/// The shared cases' surfaces as JSON for their pictures (`KENTOS_ROAD_OUT=dir`), by hand.
#[test]
#[ignore = "pictures, run by hand"]
fn pictures() {
    let dir = std::env::var("KENTOS_ROAD_OUT").expect("KENTOS_ROAD_OUT");
    let doc: Value = serde_json::from_str(CASES).expect("JSON");
    let mut out = Vec::new();
    for case in doc["cases"].as_array().expect("cases") {
        let Ok(input) = serde_json::from_value::<SurfaceInput>(case["input"].clone()) else {
            continue;
        };
        let Ok(s) = build(&input) else { continue };
        out.push(serde_json::json!({
            "name": case["name"], "vertices": s.vertices, "triangles": s.triangles,
            "constrained": s.constrained, "breaklines": input.breaklines.iter().map(|b| &b.points).collect::<Vec<_>>(),
            "boundary": input.boundary.as_ref().map(|r| &r.points), "voids": input.voids.iter().map(|r| &r.points).collect::<Vec<_>>(),
            "report": s.report,
        }));
    }
    std::fs::write(format!("{dir}/surfaces-out.json"), serde_json::to_string(&out).expect("json")).expect("written");
}
