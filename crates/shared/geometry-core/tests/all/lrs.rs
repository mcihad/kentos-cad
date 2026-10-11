//! Doğrusal referanslama (docs/adr/0224) against the independent reference in
//! `fixtures/lrs/v1/cases.json` (`scripts/fixtures/lrs_cases.py`, 50 digits,
//! no KentOS code), through the op table as the web calls it
//! (`apps/web/src/model/ops/lrs.wasm.test.ts`), and the route index against
//! walking every route.

use kentos_geometry_core::Vec2;
use kentos_geometry_core::api::run_named;
use kentos_geometry_core::jsmath::{js_max, js_min};
use kentos_geometry_core::ops::lrs::{Route, RouteIndex, RoutePart};
use serde_json::{Value, json};

fn cases() -> Value {
    let path = format!(
        "{}/../../../fixtures/lrs/v1/cases.json",
        env!("CARGO_MANIFEST_DIR")
    );
    serde_json::from_str(&std::fs::read_to_string(path).expect("the cases")).expect("JSON")
}

fn run(op: &str, args: Value) -> Value {
    let out = run_named(op, &args.to_string()).unwrap_or_else(|e| panic!("{op}: {e}"));
    serde_json::from_str(&out).expect("JSON")
}

const TOL: f64 = 1e-7;

/// Numbers within the tolerance, everything else equal, all the way down.
fn same(got: &Value, want: &Value, what: &str) {
    match (got, want) {
        (Value::Number(g), Value::Number(w)) => {
            let (g, w) = (g.as_f64().expect("a number"), w.as_f64().expect("a number"));
            assert!(
                (g - w).abs() <= TOL * js_max(w.abs(), 1.0),
                "{what}: {g} ≠ {w}"
            );
        }
        (Value::Array(g), Value::Array(w)) => {
            assert_eq!(g.len(), w.len(), "{what}: {got} ≠ {want}");
            for (i, (g, w)) in g.iter().zip(w).enumerate() {
                same(g, w, &format!("{what}[{i}]"));
            }
        }
        (Value::Object(g), Value::Object(w)) => {
            for (k, w) in w {
                same(g.get(k).unwrap_or(&Value::Null), w, &format!("{what}.{k}"));
            }
        }
        _ => assert_eq!(got, want, "{what}"),
    }
}

fn each(kind: &str) -> Vec<Value> {
    cases()[kind].as_array().expect(kind).clone()
}

#[test]
fn a_point_finds_its_place_along_a_route() {
    for c in each("locate") {
        let title = c["title"].as_str().expect("a title");
        let point = json!({"kind": "point", "p": c["p"]});
        let rows = run("lrsLocate", json!([[c["route"]], [point], 1e9, false]));
        let row = &rows[0][0];
        let want = &c["expect"];
        same(&row["m"], &want["m"], title);
        same(&row["distance"], &want["distance"], title);
    }
}

#[test]
fn a_measure_and_a_stretch_of_measures_are_placed() {
    for c in each("pointAt") {
        let title = c["title"].as_str().expect("a title");
        let event = json!({"route": 0, "m": c["m"], "offset": c["offset"]});
        let out = &run("lrsEvents", json!([[c["route"]], [event]]))[0];
        let want = &c["expect"];
        if let Some(fault) = want.get("fault") {
            assert_eq!(&out["fault"]["fault"], fault, "{title}");
            same(&out["fault"], want, title);
        } else {
            same(&out["p"], &want["p"], title);
        }
    }
    for c in each("stretch") {
        let title = c["title"].as_str().expect("a title");
        let event = json!({"route": 0, "from": c["from"], "to": c["to"], "offset": c["offset"]});
        let out = &run("lrsEvents", json!([[c["route"]], [event]]))[0];
        let want = &c["expect"];
        if let Some(fault) = want.get("fault") {
            assert_eq!(&out["fault"]["fault"], fault, "{title}");
        } else {
            same(&out["parts"], &want["parts"], title);
        }
    }
}

#[test]
fn routes_are_made_calibrated_and_summed_up() {
    for c in each("create") {
        let title = c["title"].as_str().expect("a title");
        let out = run("lrsCreate", json!([c["pieces"], c["rules"]]));
        same(&out, &c["expect"], title);
    }
    for c in each("calibrate") {
        let title = c["title"].as_str().expect("a title");
        let out = &run(
            "lrsCalibrate",
            json!([[c["route"]], [c["points"]], c["rules"]]),
        )[0];
        let want = &c["expect"];
        if let Some(fault) = want.get("fault") {
            assert_eq!(&out["fault"]["fault"], fault, "{title}");
            same(&out["used"], &want["used"], title);
        } else {
            same(&out["parts"], &want["parts"], title);
            same(&out["used"], &want["used"], title);
        }
    }
    for c in each("summary") {
        let title = c["title"].as_str().expect("a title");
        let out = &run("lrsSummaries", json!([[c["route"]]]))[0];
        same(out, &c["expect"], title);
    }
}

#[test]
fn line_events_overlay() {
    for c in each("overlay") {
        let title = c["title"].as_str().expect("a title");
        let out = run("lrsOverlay", json!([c["a"], c["b"], c["union"]]));
        same(&out, &c["expect"], title);
    }
}

#[test]
fn lines_and_areas_are_located_along_a_route() {
    for c in each("locateLine") {
        let title = c["title"].as_str().expect("a title");
        let line = json!({"kind": "polyline", "pts": c["pts"]});
        let rows = &run(
            "lrsLocate",
            json!([[c["route"]], [line], c["radius"], true]),
        )[0];
        let want = &c["expect"];
        if want.get("none").is_some() {
            assert_eq!(rows.as_array().map(Vec::len), Some(0), "{title}");
        } else {
            same(&rows[0], want, title);
        }
    }
    for c in each("locateArea") {
        let title = c["title"].as_str().expect("a title");
        let rings = c["rings"].as_array().expect("rings");
        let holes: Vec<Value> = rings[1..].iter().map(|r| json!({"pts": r})).collect();
        let area = json!({"kind": "polygon", "pts": rings[0], "holes": holes});
        let rows = &run("lrsLocate", json!([[c["route"]], [area], 0.0, true]))[0];
        let want: Vec<Value> = c["expect"]
            .as_array()
            .expect("pieces")
            .iter()
            .map(|q| json!({"route": 0, "from": q["from"], "to": q["to"]}))
            .collect();
        same(rows, &Value::Array(want), title);
    }
}

/// A small deterministic generator (SplitMix64) for the index's check.
struct Mix(u64);

impl Mix {
    fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        ((z ^ (z >> 31)) >> 11) as f64 / (1u64 << 53) as f64
    }
}

#[test]
fn the_index_finds_what_walking_every_route_finds() {
    let mut mix = Mix(0x4c52_5320);
    let routes: Vec<Route> = (0..60)
        .map(|_| {
            let n = 2 + (mix.next() * 6.0) as usize;
            let mut x = mix.next() * 1000.0;
            let mut y = mix.next() * 1000.0;
            let mut pts = Vec::new();
            let mut ms = Vec::new();
            let mut bulges = Vec::new();
            for i in 0..n {
                pts.push(Vec2::new(x, y));
                ms.push(Some(i as f64 * 100.0));
                bulges.push(if mix.next() < 0.3 {
                    mix.next() - 0.5
                } else {
                    0.0
                });
                x += mix.next() * 80.0 - 40.0;
                y += mix.next() * 80.0 - 40.0;
            }
            Route {
                parts: vec![RoutePart {
                    pts,
                    bulges: Some(bulges),
                    ms,
                }],
            }
        })
        .collect();
    let index = RouteIndex::new(&routes);
    let mut hits = Vec::new();
    for _ in 0..2000 {
        let p = Vec2::new(mix.next() * 1100.0 - 50.0, mix.next() * 1100.0 - 50.0);
        let radius = mix.next() * 60.0;
        let got = index.near(p, radius, true, &mut hits);
        // Walking every route: its closest place, those within the radius, nearest first.
        let mut want: Vec<(usize, f64)> = routes
            .iter()
            .enumerate()
            .filter_map(|(r, route)| {
                let place = kentos_geometry_core::ops::lrs::locate(route, p)?;
                (place.distance.abs() <= radius).then_some((r, place.distance.abs()))
            })
            .collect();
        want.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
        assert_eq!(
            got.iter().map(|n| n.route).collect::<Vec<_>>(),
            want.iter().map(|w| w.0).collect::<Vec<_>>(),
            "{p:?} {radius}"
        );
        for (n, w) in got.iter().zip(&want) {
            assert_eq!(n.place.distance.abs(), w.1);
            assert_eq!(
                Some(&n.place),
                kentos_geometry_core::ops::lrs::locate(&routes[n.route], p).as_ref()
            );
        }
        let nearest = index.near(p, radius, false, &mut hits);
        assert_eq!(nearest.first().map(|n| n.route), want.first().map(|w| w.0));
    }
}

/// The timing's routes (docs/adr/0224 §4): 1 000 routes of 1 km, 50 m
/// apart, each of 100 edges zigzagging 2 m, every tenth an arc, M 0 to
/// 1 000 by tens. The web's `scripts/perf/lrs.test.ts` makes the same.
fn timing_routes() -> Vec<Route> {
    (0..1000)
        .map(|r| {
            let y0 = r as f64 * 50.0;
            let pts = (0..=100)
                .map(|i| Vec2::new(i as f64 * 10.0, y0 + if i % 2 == 1 { 2.0 } else { 0.0 }))
                .collect();
            let bulges = (0..100)
                .map(|i| if i % 10 == 5 { 0.2 } else { 0.0 })
                .collect();
            let ms = (0..=100).map(|i| Some(i as f64 * 10.0)).collect();
            Route {
                parts: vec![RoutePart {
                    pts,
                    bulges: Some(bulges),
                    ms,
                }],
            }
        })
        .collect()
}

/// The least of `runs` runs after a warm-up, in milliseconds, and the work's count.
fn least(runs: usize, mut work: impl FnMut() -> usize) -> (f64, usize) {
    let mut count = work();
    let mut best = f64::INFINITY;
    for _ in 0..runs {
        let t = std::time::Instant::now();
        count = work();
        best = js_min(best, t.elapsed().as_secs_f64() * 1000.0);
    }
    (best, count)
}

/// How long the tools' work takes (docs/adr/0224 §4), release build:
/// `cargo test --release -p kentos-geometry-core --test all lrs::timing -- --ignored --nocapture`.
#[test]
#[ignore = "süre ölçümü: cargo test --release -p kentos-geometry-core --test all lrs::timing -- --ignored --nocapture"]
fn timing() {
    use kentos_geometry_core::entity::Shape;
    use kentos_geometry_core::ops::lrs::{
        CalibrateRules, CreateRules, Event, EventIn, Mark, Method, Piece, calibrate_run, create,
        events_run, locate_run, overlay,
    };

    let routes = timing_routes();
    let mut mix = Mix(0x4c52_5321);
    let points: Vec<EventIn> = (0..100_000)
        .map(|i| EventIn {
            route: Some(i % 1000),
            m: Some(mix.next() * 1000.0),
            from: None,
            to: None,
            offset: Some(if i % 2 == 0 { 5.0 } else { -5.0 }),
        })
        .collect();
    let lines: Vec<EventIn> = (0..10_000)
        .map(|i| {
            let from = mix.next() * 900.0;
            EventIn {
                route: Some(i % 1000),
                m: None,
                from: Some(from),
                to: Some(from + 10.0 + mix.next() * 90.0),
                offset: Some(8.0),
            }
        })
        .collect();
    let features: Vec<Shape> = (0..100_000)
        .map(|_| Shape::Point {
            p: Vec2::new(mix.next() * 1000.0, mix.next() * 50_000.0 - 25.0),
            z: None,
            parts: None,
        })
        .collect();
    // 10 000 pieces of 100 routes, 100 a route, in no order.
    let mut pieces: Vec<Piece> = (0..10_000)
        .map(|i| {
            let (k, j) = (i / 100, i % 100);
            let y = k as f64 * 50.0;
            Piece {
                id: format!("R{k}"),
                pts: vec![
                    Vec2::new(j as f64 * 10.0, y),
                    Vec2::new(j as f64 * 10.0 + 10.0, y),
                ],
                bulges: None,
                value: None,
                from: None,
                to: None,
            }
        })
        .collect();
    for i in (1..pieces.len()).rev() {
        pieces.swap(i, (mix.next() * (i + 1) as f64) as usize);
    }
    let side = |mix: &mut Mix| -> Vec<Event> {
        (0..50_000)
            .map(|i| {
                let from = mix.next() * 950.0;
                Event {
                    route: format!("R{}", i % 1000),
                    from,
                    to: from + 1.0 + mix.next() * 49.0,
                }
            })
            .collect()
    };
    let (a, b) = (side(&mut mix), side(&mut mix));
    // Ten marks a route, 1 % off its measures.
    let marks: Vec<Vec<Mark>> = (0..1000)
        .map(|r| {
            (0..10)
                .map(|j| {
                    let x = j as f64 * 100.0 + 40.0;
                    Mark {
                        p: Vec2::new(x, r as f64 * 50.0),
                        m: x * 1.01,
                    }
                })
                .collect()
        })
        .collect();

    let mut over = Vec::new();
    let mut say = |name: &str, (ms, n): (f64, usize), budget: f64| {
        let ok = ms <= budget;
        println!(
            "{name:<52} {ms:>8.2} ms  {n:>7} kayıt  bütçe {budget:>5.0} ms: {}",
            if ok {
                "bütçede"
            } else {
                "BÜTÇEYİ AŞIYOR"
            }
        );
        if !ok {
            over.push(name.to_owned());
        }
    };
    let placed = |out: Vec<kentos_geometry_core::ops::lrs::EventOut>| {
        out.iter().filter(|e| e.fault.is_none()).count()
    };
    say(
        "Olayları rotada göster: 100 000 nokta, 1 000 rota",
        least(5, || placed(events_run(&routes, &points))),
        100.0,
    );
    say(
        "Olayları rotada göster: 10 000 çizgi, ötelemeli",
        least(5, || placed(events_run(&routes, &lines))),
        100.0,
    );
    say(
        "Rota boyunca konumla: 100 000 nokta, 20 m",
        least(5, || {
            locate_run(&routes, &features, 20.0, false)
                .iter()
                .filter(|r| !r.is_empty())
                .count()
        }),
        300.0,
    );
    say(
        "Rota oluştur: 10 000 parçadan 100 rota",
        least(5, || {
            create(
                &pieces,
                &CreateRules {
                    method: Method::Length,
                    priority: None,
                    factor: None,
                    start: None,
                    ignore_gaps: None,
                },
            )
            .routes
            .len()
        }),
        20.0,
    );
    say(
        "Olayları çakıştır: 50 000 ve 50 000 olay, birleşim",
        least(5, || overlay(&a, &b, true).len()),
        150.0,
    );
    say(
        "Rotayı kalibre et: 1 000 rota, 10 000 nokta",
        least(5, || {
            calibrate_run(
                &routes,
                &marks,
                &CalibrateRules {
                    radius: 1.0,
                    before: None,
                    after: None,
                    ignore_gaps: None,
                    split: None,
                },
            )
            .iter()
            .filter(|c| c.fault.is_none())
            .count()
        }),
        100.0,
    );
    // What the scene must give: every point event on its route, every route made, every route calibrated.
    assert_eq!(placed(events_run(&routes, &points)), points.len());
    assert_eq!(placed(events_run(&routes, &lines)), lines.len());
    assert!(over.is_empty(), "bütçeyi aşanlar: {over:?}");
}
