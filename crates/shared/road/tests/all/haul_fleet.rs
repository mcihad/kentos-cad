//! A haul fleet against the shared cases (fixtures/road/v1/haul-fleet.json, docs/adr/0936 §1): written by
//! scripts/fixtures/road_haul_fleet_cases.py, which integrates the run in time (RK4, 1 ms) and solves the loader's
//! Markov chain numerically; and the hauls a plan's flows split into, by what they must keep.

use kentos_road::haul_fleet::{G, Truck, hauls, loader, reversed, run};
use serde_json::Value;

const CASES: &str = include_str!("../../../../../fixtures/road/v1/haul-fleet.json");

fn n(v: &Value) -> f64 {
    v.as_f64().expect("a number")
}

fn truck(t: &Value) -> Truck {
    Truck {
        empty: n(&t["empty"]),
        gross: n(&t["gross"]),
        power: n(&t["power"]),
        efficiency: n(&t["efficiency"]),
        top: n(&t["top"]),
        downhill: t["downhill"].as_f64(),
        traction: n(&t["traction"]),
        drive: n(&t["drive"]),
        brake: n(&t["brake"]),
    }
}

#[test]
fn every_run_is_the_references() {
    let all: Value = serde_json::from_str(CASES).unwrap();
    let mut worst: f64 = 0.0;
    for c in all["runs"].as_array().unwrap() {
        let path: Vec<[f64; 2]> = c["path"].as_array().unwrap().iter().map(|p| [n(&p[0]), n(&p[1])]).collect();
        let got = run(&path, n(&c["mass"]), &truck(&c["truck"]), n(&c["rolling"]), None);
        let name = c["name"].as_str().unwrap();
        if c["run"].is_null() {
            let stall = got.expect_err(name);
            assert!(stall.at < 1.0, "{name}: it stalls at the foot of the grade: {stall:?}");
            continue;
        }
        let r = got.unwrap_or_else(|e| panic!("{name}: {e:?}"));
        let want = &c["run"];
        let rel = |a: f64, b: f64| (a - b).abs() / b.abs().max(1.0);
        worst = worst.max(rel(r.time, n(&want["time"])));
        assert!(rel(r.time, n(&want["time"])) < 1e-4, "{name}: time {} ≠ {}", r.time, want["time"]);
        assert!((r.length - n(&want["length"])).abs() < 1e-9, "{name}");
        assert!(rel(r.work, n(&want["work"])) < 2e-3, "{name}: work {} ≠ {}", r.work, want["work"]);
        assert!((r.idle - n(&want["idle"])).abs() < 0.05, "{name}: idle {} ≠ {}", r.idle, want["idle"]);
        // The peak where the run meets the braking curve lies between the 0,25 m steps: read on them, a few mm/s low.
        assert!((r.fastest - n(&want["fastest"])).abs() < 5e-3, "{name}: fastest {} ≠ {}", r.fastest, want["fastest"]);
    }
    println!("en büyük göreli süre farkı {worst:e}");
}

#[test]
fn every_loader_is_the_chains() {
    let all: Value = serde_json::from_str(CASES).unwrap();
    for c in all["loaders"].as_array().unwrap() {
        let got = loader(c["trucks"].as_u64().unwrap() as u32, n(&c["away"]), n(&c["service"]));
        let want = &c["loader"];
        for (g, key) in [(got.match_factor, "matchFactor"), (got.steady, "steady"), (got.queued, "queued"), (got.busy, "busy"), (got.wait, "wait")] {
            assert!((g - n(&want[key])).abs() < 1e-9 * n(&want[key]).abs().max(1.0), "{c}: {key} {g}");
        }
    }
}

/// The physics the run keeps: on a level road at its top speed the wheels pull only the rolling resistance; the work
/// over a climb is at least the climb's potential energy; a truck coming back the same way meets the opposite grade.
#[test]
fn the_run_keeps_its_physics() {
    let t = Truck { empty: 15000.0, gross: 40000.0, power: 300000.0, efficiency: 0.8, top: 10.0, downhill: None, traction: 0.6, drive: 0.7, brake: 2.0 };
    // Level, long: the work is the kinetic energy at the top speed plus the rolling resistance up to where it brakes
    // (v²/2b = 25 m before the end: there the brakes and the rolling stop it, not the engine).
    let level = run(&[[0.0, 0.0], [5000.0, 0.0]], 40000.0, &t, 0.03, Some(100.0)).unwrap();
    let rolling = 40000.0 * G * 0.03 * (5000.0 - 25.0);
    let kinetic = 0.5 * 40000.0 * 10.0f64.powi(2);
    assert!((level.work - rolling - kinetic).abs() < 1e-3 * rolling, "{} vs {}", level.work, rolling + kinetic);
    assert_eq!(level.speeds.first().unwrap(), &[0.0, 0.0]);
    assert_eq!(level.speeds.last().unwrap()[0], 5000.0);
    assert!(level.speeds.len() == 51, "every 100 m and the ends: {}", level.speeds.len());
    // A climb of 30 m: at least m·g·Δz more than the level's rolling.
    let climb = run(&[[0.0, 0.0], [600.0, 30.0]], 40000.0, &t, 0.03, None).unwrap();
    assert!(climb.work > 40000.0 * G * 30.0, "{}", climb.work);
    // Back the same way: the reversed path descends from the end.
    let back = reversed(&[[0.0, 0.0], [600.0, 30.0], [800.0, 30.0]]);
    assert_eq!(back, vec![[0.0, 30.0], [200.0, 30.0], [800.0, 0.0]]);
    let down = run(&back, 15000.0, &t, 0.03, None).unwrap();
    assert!(down.idle > 0.0 && down.time < climb.time, "{down:?}");
}

/// The underpowered 5 % climb against its closed form: at a constant resistance R and power ηP the run is
/// `du = m·v²·dv/(ηP − R·v)`, `dt = m·v·dv/(ηP − R·v)` after the grip's constant acceleration, braking at b where
/// `v² = 2b(L − u)`: 160,416 249 691 s (mpmath, 30 digits). Fixed 0,25 m steps made it 160,493.
#[test]
fn an_underpowered_climb_is_its_closed_form() {
    let t = Truck { empty: 15000.0, gross: 40000.0, power: 150000.0, efficiency: 0.8, top: 60.0 / 3.6, downhill: Some(30.0 / 3.6), traction: 0.6, drive: 0.7, brake: 2.0 };
    let r = run(&[[0.0, 100.0], [600.0, 130.0]], 40000.0, &t, 0.03, None).unwrap();
    assert!((r.time - 160.416_249_691_327).abs() < 1e-3, "{}", r.time);
}

/// The hauls carry each place's surplus to the demands, moment for moment the plan's, first in, first out.
#[test]
fn a_plans_flows_split_into_hauls() {
    // Supplies at 0 and 100, demands at 200 and 300; a backward haul from 500 to 400.
    let segs = [(0.0, 100.0, 50.0), (100.0, 200.0, 80.0), (200.0, 300.0, 30.0), (300.0, 400.0, 0.0), (400.0, 500.0, -20.0)];
    let h = hauls(&segs);
    let moment: f64 = h.iter().map(|x| x.volume * (x.to - x.from).abs()).sum();
    let plan: f64 = segs.iter().map(|s| s.2.abs() * (s.1 - s.0)).sum();
    assert!((moment - plan).abs() < 1e-9, "{h:?}");
    // First in, first out: the 50 m³ from 0 meet the first demand (50 of 200's 50), 100's 30 go on to 300.
    assert_eq!(h[0], kentos_road::haul_fleet::Haul { from: 0.0, to: 200.0, volume: 50.0 });
    assert_eq!(h[1], kentos_road::haul_fleet::Haul { from: 100.0, to: 300.0, volume: 30.0 });
    assert_eq!(h[2], kentos_road::haul_fleet::Haul { from: 500.0, to: 400.0, volume: 20.0 });
    assert_eq!(h.len(), 3);
    // Nothing flows: no haul.
    assert!(hauls(&[(0.0, 10.0, 0.0)]).is_empty());
}
