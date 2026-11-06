//! A curve's widening against the shared cases (fixtures/road/v1/widening.json, docs/adr/0932 §1): written by
//! scripts/fixtures/road_widening_cases.py, which builds the steady-state turn axle by axle at 40 digits, no KentOS code.

use kentos_road::widening::{
    WideningError, WideningMethod, WideningRoad, WideningVehicle, offtracking, rounded, widening,
};
use serde_json::Value;

const CASES: &str = include_str!("../../../../../fixtures/road/v1/widening.json");
const WITHIN: f64 = 1e-9;

fn n(v: &Value) -> f64 {
    v.as_f64().expect("a number")
}

#[test]
fn every_widening_is_the_references() {
    let all: Value = serde_json::from_str(CASES).expect("json");
    let cases = all["cases"].as_array().expect("cases");
    assert!(cases.len() >= 20);
    for c in cases {
        let name = c["name"].as_str().unwrap();
        let v = &c["vehicle"];
        let vehicle = WideningVehicle {
            track: n(&v["track"]),
            front: n(&v["front"]),
            wheelbase: n(&v["wheelbase"]),
            trailer: v.get("trailer").map(|t| (n(&t[0]), n(&t[1]))),
        };
        let r = &c["road"];
        let road = WideningRoad {
            lanes: r["lanes"].as_u64().unwrap() as u32,
            tangent: n(&r["tangent"]),
            clearance: n(&r["clearance"]),
            speed: r.get("speed").map(n),
            method: if r["method"] == "simple" { WideningMethod::Simple } else { WideningMethod::Standard },
        };
        let got = widening(n(&c["radius"]), &vehicle, &road);
        if c.get("error").is_some() {
            assert_eq!(got, Err(WideningError::Unturnable), "{name}");
            continue;
        }
        let w = got.unwrap_or_else(|e| panic!("{name}: {e}"));
        for (key, value) in [
            ("offtracking", w.offtracking),
            ("track", w.track),
            ("overhang", w.overhang),
            ("allowance", w.allowance),
            ("width", w.width),
            ("widening", w.widening),
        ] {
            assert!((value - n(&c[key])).abs() < WITHIN, "{name}: {key} {value} vs {}", c[key]);
        }
        if let Some(rnd) = c.get("round") {
            let got = rounded(w.widening, rnd.get("step").map(n), rnd.get("least").map(n));
            assert!((got - n(&c["rounded"])).abs() < WITHIN, "{name}: {got} vs {}", c["rounded"]);
        }
    }
}

/// The difference form loses digits at a large radius; the product form does not.
#[test]
fn a_large_radius_keeps_its_digits() {
    let sum = 6.6f64.powi(2) - 0.8f64.powi(2) + 12.5f64.powi(2);
    let r = 1.0e6;
    let exact = sum / (2.0 * r) + sum * sum / (8.0 * r * r * r);
    let got = offtracking(r, sum).unwrap();
    assert!((got - exact).abs() / exact < 1e-12, "{got} vs {exact}");
    let naive = r - (r * r - sum).sqrt();
    assert!((naive - exact).abs() / exact > 1e-8, "the difference form would lose digits: {naive}");
}

#[test]
fn a_bad_size_is_refused() {
    let v = WideningVehicle { track: 2.59, front: 1.2, wheelbase: 6.6, trailer: None };
    let road = WideningRoad { lanes: 2, tangent: 7.2, clearance: 0.9, speed: Some(80.0), method: WideningMethod::Standard };
    for (r, v, road) in [
        (0.0, v, road),
        (f64::NAN, v, road),
        (150.0, WideningVehicle { track: 0.0, ..v }, road),
        (150.0, WideningVehicle { front: -1.0, ..v }, road),
        (150.0, WideningVehicle { trailer: Some((0.0, 0.0)), ..v }, road),
        (150.0, v, WideningRoad { lanes: 0, ..road }),
        (150.0, v, WideningRoad { speed: Some(0.0), ..road }),
        (150.0, v, WideningRoad { clearance: -0.1, ..road }),
    ] {
        assert_eq!(widening(r, &v, &road), Err(WideningError::Input));
    }
}

/// The steady state against the drive (Heald, TRR 1052): the tractrix drive of `swept` along a real curve (with or
/// without clothoids) off-tracks at most the steady state's, and reaches it only on a long enough curve. The numbers
/// are docs/adr/0932 §2's.
#[test]
fn the_steady_state_bounds_the_drive() {
    use kentos_road::alignment::chain::{PiInput, build};
    use kentos_road::swept::{Trailer, Vehicle, drive};
    let vehicle = Vehicle {
        wheelbase: 6.6,
        front: 1.2,
        rear: 0.0,
        width: 2.59,
        trailer: Some(Trailer { kingpin: -0.8, wheelbase: 12.5, front: 0.9, rear: 1.0, width: 2.59 }),
    };
    let sum = 6.6f64.powi(2) - 0.8f64.powi(2) + 12.5f64.powi(2);
    let mut rows = Vec::new();
    for (radius, spiral, deflection) in [
        (60.0, 0.0, 15.0),
        (60.0, 0.0, 45.0),
        (60.0, 0.0, 90.0),
        (150.0, 0.0, 10.0),
        (150.0, 0.0, 30.0),
        (150.0, 60.0, 30.0),
        (150.0, 60.0, 60.0),
        (300.0, 0.0, 5.0),
        (300.0, 80.0, 20.0),
    ] {
        let d: f64 = f64::to_radians(deflection);
        let pis = vec![
            PiInput { id: "a".into(), x: -400.0, y: 0.0, radius: 0.0, spiral_in: 0.0, spiral_out: 0.0, ove: false },
            PiInput { id: "p".into(), x: 0.0, y: 0.0, radius, spiral_in: spiral, spiral_out: spiral, ove: false },
            PiInput { id: "b".into(), x: 400.0 * d.cos(), y: 400.0 * d.sin(), radius: 0.0, spiral_in: 0.0, spiral_out: 0.0, ove: false },
        ];
        let al = build(&pis).expect("builds");
        let n = (al.length / 0.25).ceil() as usize;
        let path: Vec<[f64; 2]> = (0..=n).filter_map(|i| al.at((i as f64 * 0.25).min(al.length))).map(|p| [p.x, p.y]).collect();
        let swept = drive(&vehicle, &path, 0.25).expect("drives");
        let got = swept.trailer_offtracking.expect("a trailer");
        let steady = kentos_road::widening::offtracking(radius, sum).unwrap();
        // The drive's own error: the chords' sagitta and the sampling; within a centimetre of the steady state.
        assert!(got <= steady + 0.01, "R {radius} Ls {spiral} Δ {deflection}°: {got} > {steady}");
        rows.push((radius, spiral, deflection, got, steady));
    }
    // A 90° turn on R 60 m reaches the steady state; a 15° one does not come near it.
    let ratio = |r: f64, s: f64, d: f64| rows.iter().find(|x| x.0 == r && x.1 == s && x.2 == d).map(|x| x.3 / x.4).unwrap();
    assert!(ratio(60.0, 0.0, 90.0) > 0.97, "{rows:?}");
    assert!(ratio(60.0, 0.0, 15.0) < 0.8, "{rows:?}");
    for (r, s, d, got, steady) in &rows {
        println!("R {r:>5} Ls {s:>4} Δ {d:>4}°: drive {got:.3} m, steady {steady:.3} m, {:.0} %", 100.0 * got / steady);
    }
}
