//! Where a sample line set takes its sections and where its lines cross, against the shared cases
//! (fixtures/road/v1/section-stations.json, docs/adr/0933 §1): written by scripts/fixtures/road_section_stations_cases.py
//! in exact fractions and at 40 digits, no KentOS code.

use kentos_road::alignment::chain::{PiInput, build};
use kentos_road::section::stations::{Break, clip, crossings, simplify};
use serde_json::Value;

const CASES: &str = include_str!("../../../../../fixtures/road/v1/section-stations.json");

fn n(v: &Value) -> f64 {
    v.as_f64().expect("a number")
}

#[test]
fn the_kept_breaks_are_the_references() {
    let all: Value = serde_json::from_str(CASES).unwrap();
    for c in all["simplify"].as_array().unwrap() {
        let name = c["name"].as_str().unwrap();
        let profile: Vec<Break> =
            c["profile"].as_array().unwrap().iter().map(|p| Break { station: n(&p[0]), z: n(&p[1]) }).collect();
        let got = simplify(&profile, n(&c["tolerance"]), c.get("most").map(n));
        let want: Vec<usize> = c["kept"].as_array().unwrap().iter().map(|k| k.as_u64().unwrap() as usize).collect();
        assert_eq!(got, want, "{name}");
    }
}

#[test]
fn the_crossings_and_the_clip_are_the_references() {
    let all: Value = serde_json::from_str(CASES).unwrap();
    for c in all["crossings"].as_array().unwrap() {
        let name = c["name"].as_str().unwrap();
        let pis: Vec<PiInput> = c["pis"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| PiInput {
                id: p["id"].as_str().unwrap().into(),
                x: n(&p["x"]),
                y: n(&p["y"]),
                radius: n(&p["radius"]),
                spiral_in: n(&p["spiralIn"]),
                spiral_out: n(&p["spiralOut"]),
            })
            .collect();
        let al = build(&pis).expect("builds");
        let stations: Vec<f64> = c["stations"].as_array().unwrap().iter().map(n).collect();
        let widths: Vec<(f64, f64)> = c["widths"].as_array().unwrap().iter().map(|w| (n(&w[0]), n(&w[1]))).collect();
        let got = crossings(&al, &stations, &widths);
        let want = c["crossings"].as_array().unwrap();
        assert_eq!(got.len(), want.len(), "{name}");
        for (g, w) in got.iter().zip(want) {
            assert_eq!((g.first, g.second, g.left), (w["first"].as_u64().unwrap() as usize, w["second"].as_u64().unwrap() as usize, w["left"].as_bool().unwrap()), "{name}");
            assert!((g.at_first - n(&w["atFirst"])).abs() < 1e-7 && (g.at_second - n(&w["atSecond"])).abs() < 1e-7, "{name}: {g:?} vs {w}");
        }
        let (clipped, found) = clip(&al, &stations, &widths);
        assert_eq!(found, got, "{name}");
        for (k, (g, w)) in clipped.iter().zip(c["clipped"].as_array().unwrap()).enumerate() {
            assert!((g.0 - n(&w[0])).abs() < 1e-7 && (g.1 - n(&w[1])).abs() < 1e-7, "{name}: line {k} {g:?} vs {w}");
        }
        assert!(crossings(&al, &stations, &clipped).is_empty(), "{name}: the clipped lines still meet");
    }
}

/// The sample project's terrain (samples/yol_ornegi.py: rising ground, a hill, a stream valley, a ripple; a 20 m grid
/// jittered by up to 4 m) and its axis: how well sections every d metres and the simplified breaks carry the
/// centreline's ground. Between two breaks both the profile and a linear reading are linear, so the largest error is
/// at a break: exact. The numbers are docs/adr/0933 §2's.
#[test]
fn the_breaks_carry_the_ground_better_than_an_interval() {
    use kentos_road::section::stations::ground_breaks;
    use kentos_road::surface::{self, SpotPoint, SurfaceInput};
    let (x0, y0) = (585_000.0, 4_400_000.0);
    let ground = |x: f64, y: f64| {
        let (u, v) = (x - x0, y - y0);
        1278.0 + 0.012 * u + 0.018 * v + 9.0 * (-(((u - 420.0) / 140.0).powi(2) + ((v - 330.0) / 110.0).powi(2))).exp()
            - 6.5 * (-((u - 0.55 * v - 560.0) / 45.0).powi(2)).exp()
            + 0.6 * (u / 37.0).sin() * (v / 53.0).cos()
    };
    let mut seed = 58u64;
    let mut jitter = || {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((seed >> 33) as f64 / (1u64 << 31) as f64 - 0.5) * 8.0
    };
    let mut points = Vec::new();
    for i in 0..=45 {
        for j in 0..=26 {
            let inner = i > 0 && i < 45 && j > 0 && j < 26;
            let (dx, dy) = if inner { (jitter(), jitter()) } else { (0.0, 0.0) };
            let (x, y) = (x0 + 20.0 * f64::from(i) + dx, y0 + 20.0 * f64::from(j) + dy);
            points.push(SpotPoint { x, y, z: ground(x, y), source: None });
        }
    }
    let s = surface::build(&SurfaceInput { points, ..SurfaceInput::default() }).expect("builds");
    let pi = |id: &str, x: f64, y: f64, r: f64, l: f64| PiInput { id: id.into(), x: x0 + x, y: y0 + y, radius: r, spiral_in: l, spiral_out: l };
    let al = build(&[pi("a", 60.0, 90.0, 0.0, 0.0), pi("p1", 470.0, 330.0, 300.0, 60.0), pi("b", 860.0, 250.0, 0.0, 0.0)]).unwrap();
    let profile = ground_breaks(&s, &al, 0.0, al.length);
    // The largest and the root-mean-square height error at the breaks of the linear reading through `kept`.
    let error = |kept: &[f64]| {
        let read = |st: f64| {
            let k = kept.partition_point(|x| *x <= st).clamp(1, kept.len() - 1);
            let (a, b) = (kept[k - 1], kept[k]);
            let z = |q: f64| profile.iter().find(|p| (p.station - q).abs() < 1e-9).map(|p| p.z).unwrap_or_else(|| {
                let i = profile.partition_point(|p| p.station <= q).clamp(1, profile.len() - 1);
                let (p, r) = (profile[i - 1], profile[i]);
                p.z + (r.z - p.z) * (q - p.station) / (r.station - p.station)
            });
            z(a) + (z(b) - z(a)) * (st - a) / (b - a)
        };
        let errs: Vec<f64> = profile.iter().map(|p| (p.z - read(p.station)).abs()).collect();
        let most = errs.iter().copied().fold(0.0, f64::max);
        let rms = (errs.iter().map(|e| e * e).sum::<f64>() / errs.len() as f64).sqrt();
        (most, rms)
    };
    println!("{} kırık, aks {:.1} m", profile.len(), al.length);
    for d in [20.0, 10.0, 5.0] {
        let mut kept: Vec<f64> = (0..).map(|i| f64::from(i) * d).take_while(|s| *s < al.length).collect();
        kept.push(al.length);
        let (most, rms) = error(&kept);
        println!("her {d:>4} m: {:>4} kesit, en büyük {most:.3} m, KOK {rms:.3} m", kept.len());
    }
    for tol in [0.20, 0.10, 0.05] {
        let kept: Vec<f64> = simplify(&profile, tol, Some(20.0)).into_iter().map(|k| profile[k].station).collect();
        let (most, rms) = error(&kept);
        println!("kırıklar {tol:.2} m, en çok 20 m: {:>4} kesit, en büyük {most:.3} m, KOK {rms:.3} m", kept.len());
        assert!(most <= tol + 1e-9, "the simplification keeps its promise: {most} > {tol}");
    }
}
