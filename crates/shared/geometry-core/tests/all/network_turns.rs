//! Turns, levels and time profiles (docs/adr/0521) against
//! fixtures/network/v1/turns.json (`scripts/fixtures/network_turns_cases.py`:
//! the ADR's rules in Python, no KentOS code; exact fractions for the time
//! profiles; every walk tried for the cheapest way): each scene's graph,
//! places, routes, service areas' lines, closest facilities, cost matrices
//! and Denetle's problems the ADR adds. Nodes, pieces, orders exactly;
//! offsets, lengths and costs within 1e-9 (of their size above 1).

// Test harness code, not the core: the std float methods are fine here.
#![allow(clippy::disallowed_methods)]

use kentos_geometry_core::Vec2;
use kentos_geometry_core::api::json::Json;
use kentos_geometry_core::geom::bulge::bulge_path_edges;
use kentos_geometry_core::ops::network::area::{AreaQuery, service_lines};
use kentos_geometry_core::ops::network::check::{ProblemKind, check};
use kentos_geometry_core::ops::network::closest::{Nearest, nearest};
use kentos_geometry_core::ops::network::route::{Reorder, RouteError, route};
use kentos_geometry_core::ops::network::search::way_cost;
use kentos_geometry_core::ops::network::{
    EdgeIn, Graph, JunctionIn, Location, Role, Rules, Searcher, Span, TurnIn,
};
use serde_json::Value;
use std::collections::HashMap;

fn fixture() -> Value {
    let path = format!(
        "{}/../../../fixtures/network/v1/turns.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let file: Value =
        serde_json::from_str(&std::fs::read_to_string(path).expect("the fixture")).expect("JSON");
    assert_eq!(file["format"], "kentos.network-turns");
    assert_eq!(file["version"], 1);
    file
}

fn num(v: &Value) -> f64 {
    v.as_f64().expect("a number")
}

fn near(a: f64, b: f64, what: &str) {
    assert!(
        (a - b).abs() <= 1e-9 * b.abs().max(1.0),
        "{what}: {a} ≠ {b}"
    );
}

fn point(v: &Value) -> Vec2 {
    Vec2::new(num(&v[0]), num(&v[1]))
}

fn text(v: &Value) -> Option<String> {
    v.as_str().map(str::to_owned)
}

fn build(scene: &Value) -> (Graph, kentos_geometry_core::ops::network::BuildReport) {
    let rules =
        Rules::from_json(&Json::parse(&scene["rules"].to_string()).expect("JSON")).expect("rules");
    let edges: Vec<EdgeIn> = scene["edges"]
        .as_array()
        .expect("edges")
        .iter()
        .map(|e| {
            let pts: Vec<Vec2> = e["pts"]
                .as_array()
                .expect("points")
                .iter()
                .map(point)
                .collect();
            let bulges: Option<Vec<f64>> =
                e["bulges"].as_array().map(|b| b.iter().map(num).collect());
            let levels = &e["levels"];
            EdgeIn {
                id: num(&e["id"]),
                part: 0,
                path: bulge_path_edges(&pts, bulges.as_deref(), false),
                direction: text(&e["direction"]),
                costs: e["costs"]
                    .as_array()
                    .expect("costs")
                    .iter()
                    .map(text)
                    .collect(),
                closed: e["closed"].as_bool().unwrap_or(false),
                levels: [text(&levels[0]), text(&levels[1]), text(&levels[2])],
                periods: e["periods"].as_array().map_or_else(Vec::new, |p| {
                    p.iter()
                        .map(|c| c.as_array().expect("a cost's").iter().map(text).collect())
                        .collect()
                }),
                terminals: [None, None],
                facts: Vec::new(),
            }
        })
        .collect();
    let junctions: Vec<JunctionIn> = scene["junctions"]
        .as_array()
        .expect("junctions")
        .iter()
        .map(|j| JunctionIn {
            id: num(&j["id"]),
            p: point(&j["p"]),
            role: match j["role"].as_str() {
                Some("source") => Role::Source,
                Some("valve") => Role::Valve,
                _ => Role::Junction,
            },
            closed: j["closed"].as_bool().unwrap_or(false),
            level: text(&j["level"]),
            device: None,
        })
        .collect();
    let turns: Vec<TurnIn> = scene["turns"]
        .as_array()
        .expect("turns")
        .iter()
        .map(|t| TurnIn {
            id: num(&t["id"]),
            pts: t["pts"].as_array().expect("points").iter().map(point).collect(),
            field: t["field"].as_bool().unwrap_or(false),
            value: text(&t["value"]),
        })
        .collect();
    Graph::build_with(rules, &edges, &junctions, &turns)
}

fn spans_eq(got: &[Span], want: &Value, what: &str) {
    let want = want.as_array().expect("spans");
    assert_eq!(got.len(), want.len(), "{what}: spans {got:?}");
    for (g, w) in got.iter().zip(want) {
        assert_eq!(g.piece as u64, w[0].as_u64().expect("a piece"), "{what}");
        near(g.a, num(&w[1]), what);
        near(g.b, num(&w[2]), what);
    }
}

#[test]
fn turns_levels_and_times_give_what_the_independent_reference_gives() {
    let file = fixture();
    let mut asked = 0;
    for case in file["cases"].as_array().expect("cases") {
        let scene = &case["scene"];
        let name = scene["name"].as_str().expect("a name");
        let (g, report) = build(scene);
        let nodes = case["graph"]["nodes"].as_array().expect("nodes");
        assert_eq!(g.nodes.len(), nodes.len(), "{name}: nodes");
        for (k, n) in nodes.iter().enumerate() {
            near(g.nodes[k].x, num(&n[0]), name);
            near(g.nodes[k].y, num(&n[1]), name);
        }
        let pieces = case["graph"]["pieces"].as_array().expect("pieces");
        assert_eq!(g.pieces.len(), pieces.len(), "{name}: pieces");
        for (k, p) in pieces.iter().enumerate() {
            let got = &g.pieces[k];
            assert_eq!(
                [got.from as u64, got.to as u64, got.edge as u64],
                [
                    p[0].as_u64().unwrap_or(0),
                    p[1].as_u64().unwrap_or(0),
                    p[2].as_u64().unwrap_or(0)
                ],
                "{name}: piece {k}"
            );
            near(got.s0, num(&p[3]), name);
            near(got.s1, num(&p[4]), name);
        }
        let mut places: HashMap<String, Location> = HashMap::new();
        for (key, p) in case["places"].as_object().expect("places") {
            let loc = g
                .locate(Vec2::new(num(&p["x"]), num(&p["y"])), num(&p["reach"]))
                .unwrap_or_else(|| panic!("{name}: {key} not found"));
            assert_eq!(
                loc.piece as u64,
                p["piece"].as_u64().expect("a piece"),
                "{name}: place {key}"
            );
            near(loc.offset, num(&p["offset"]), &format!("{name}: place {key}"));
            places.insert(key.clone(), loc);
        }
        let pick = |list: &Value| -> Vec<Location> {
            list.as_array().map_or_else(Vec::new, |l| {
                l.iter()
                    .map(|k| places[k.as_str().expect("a place")])
                    .collect()
            })
        };
        let mut s = Searcher::new();
        for q in case["questions"].as_array().expect("questions") {
            let title = format!("{name}: {}", q["title"].as_str().expect("a title"));
            let want = &q["expect"];
            let barriers = pick(&q["barriers"]);
            let cost = q["cost"].as_u64().unwrap_or(0) as usize;
            let at = q["at"].as_f64();
            asked += 1;
            match q["kind"].as_str().expect("a kind") {
                "route" => {
                    let reorder = match q["reorder"].as_str() {
                        Some("keepFirst") => Reorder::KeepFirst,
                        Some("keepFirstLast") => Reorder::KeepFirstLast,
                        _ => Reorder::None,
                    };
                    let stops = pick(&q["stops"]);
                    match (
                        route(&g, &mut s, &stops, &barriers, cost, reorder, at),
                        want["error"].as_str(),
                    ) {
                        (Err(RouteError::Unreachable(a, b)), Some("unreachable")) => {
                            assert_eq!(
                                [a as u64, b as u64],
                                [
                                    want["between"][0].as_u64().unwrap_or(9),
                                    want["between"][1].as_u64().unwrap_or(9)
                                ],
                                "{title}"
                            );
                        }
                        (Ok(r), None) => {
                            let order: Vec<u64> = r.order.iter().map(|&i| i as u64).collect();
                            let expected: Vec<u64> = want["order"]
                                .as_array()
                                .expect("an order")
                                .iter()
                                .map(|v| v.as_u64().unwrap_or(99))
                                .collect();
                            assert_eq!(order, expected, "{title}");
                            near(r.cost, num(&want["cost"]), &title);
                            spans_eq(&r.spans, &want["spans"], &title);
                            for (c, t) in want["totals"]
                                .as_array()
                                .expect("totals")
                                .iter()
                                .enumerate()
                            {
                                match (r.total(&g, c), t.as_f64()) {
                                    (Some(a), Some(b)) => near(a, b, &format!("{title}: cost {c}")),
                                    (None, None) => {}
                                    other => panic!("{title}: cost {c}: {other:?}"),
                                }
                            }
                        }
                        (got, want) => panic!("{title}: {got:?} ≠ {want:?}"),
                    }
                }
                "area" => {
                    let breaks: Vec<f64> = q["breaks"]
                        .as_array()
                        .expect("breaks")
                        .iter()
                        .map(num)
                        .collect();
                    let facilities = pick(&q["facilities"]);
                    let aq = AreaQuery {
                        facilities: &facilities,
                        breaks: &breaks,
                        cost,
                        toward: q["toward"].as_bool().unwrap_or(false),
                        separate: false,
                        barriers: &barriers,
                        at,
                    };
                    let mut lines = service_lines(&g, &mut s, &aq);
                    lines.sort_by(|a, b| {
                        a.band
                            .cmp(&b.band)
                            .then(a.span.piece.cmp(&b.span.piece))
                            .then(a.span.a.total_cmp(&b.span.a))
                    });
                    let expected = want["lines"].as_array().expect("lines");
                    assert_eq!(lines.len(), expected.len(), "{title}: lines {lines:?}");
                    for (l, w) in lines.iter().zip(expected) {
                        assert_eq!(l.band as u64, w["band"].as_u64().expect("a band"), "{title}");
                        assert_eq!(
                            l.span.piece as u64,
                            w["piece"].as_u64().expect("a piece"),
                            "{title}"
                        );
                        near(l.span.a, num(&w["a"]), &title);
                        near(l.span.b, num(&w["b"]), &title);
                    }
                }
                kind @ ("closest" | "matrix") => {
                    let origins = pick(&q["origins"]);
                    let targets = pick(&q["targets"]);
                    let paths = kind == "closest";
                    let reverse = q["reverse"].as_bool().unwrap_or(false);
                    let nq = Nearest {
                        origins: &origins,
                        targets: &targets,
                        k: q["k"].as_u64().map(|k| k as usize),
                        cutoff: q["cutoff"].as_f64(),
                        cost,
                        reverse,
                        barriers: &barriers,
                        at,
                    };
                    let rows = nearest(&g, &mut s, &nq, paths);
                    let expected = want.as_array().expect("rows");
                    assert_eq!(rows.len(), expected.len(), "{title}");
                    for (row, w) in rows.iter().zip(expected) {
                        let w = w.as_array().expect("a row");
                        assert_eq!(row.len(), w.len(), "{title}: {row:?}");
                        for ((f, path), wf) in row.iter().zip(w) {
                            assert_eq!(
                                f.target as u64,
                                wf["target"].as_u64().expect("a target"),
                                "{title}"
                            );
                            near(f.cost, num(&wf["cost"]), &title);
                            if paths {
                                let path = path.as_ref().expect("a way");
                                spans_eq(&path.spans, &wf["spans"], &title);
                                for (c, t) in wf["totals"]
                                    .as_array()
                                    .expect("totals")
                                    .iter()
                                    .enumerate()
                                {
                                    near(
                                        way_cost(&g, &path.spans, c, at, reverse),
                                        num(t),
                                        &format!("{title}: cost {c}"),
                                    );
                                }
                            }
                        }
                    }
                }
                "check" => {
                    let c = check(&g, &report);
                    assert_eq!(c.nodes as u64, want["nodes"].as_u64().expect("nodes"), "{title}");
                    assert_eq!(c.pieces as u64, want["pieces"].as_u64().expect("pieces"), "{title}");
                    near(c.length, num(&want["length"]), &title);
                    // The problems this ADR adds or changes: crossings at one level, values and turns not read,
                    // turns not placed.
                    let got: Vec<_> = c
                        .problems
                        .iter()
                        .filter(|p| {
                            matches!(
                                p.kind,
                                ProblemKind::Crossing | ProblemKind::Unread | ProblemKind::Turn
                            )
                        })
                        .collect();
                    let expected = want["problems"].as_array().expect("problems");
                    assert_eq!(got.len(), expected.len(), "{title}: {got:?}");
                    for (p, w) in got.iter().zip(expected) {
                        let kind = match p.kind {
                            ProblemKind::Crossing => "crossing",
                            ProblemKind::Unread => "unread",
                            _ => "turn",
                        };
                        assert_eq!(kind, w["kind"].as_str().expect("a kind"), "{title}");
                        near(p.at.x, num(&w["at"][0]), &title);
                        near(p.at.y, num(&w["at"][1]), &title);
                        let ids: Vec<f64> =
                            w["ids"].as_array().expect("ids").iter().map(num).collect();
                        assert_eq!(p.ids, ids, "{title}");
                        assert_eq!(
                            p.cost.map(|c| c as u64),
                            w["cost"].as_u64(),
                            "{title}: cost"
                        );
                    }
                }
                other => panic!("unknown question {other}"),
            }
        }
    }
    assert!(asked >= 20, "{asked} questions");
}


/// ADR 0209 §12's budgets with turns and a time profile (docs/adr/0521 §7): the same synthetic grid city (225 × 225
/// crossings 50 m apart, 100 800 pieces, a tenth one way, speeds 30, 50 or 70 km/h, a street in a hundred closed), with
/// turn delays (right 10 s, left 30 s, U-turns at dead ends, 60 s), a forbidden turn at every tenth crossing and a
/// morning period at half speed on a third of the streets; beside it the same city without turns (ADR 0209's search),
/// on the same machine at the same moment. Each time the least of five. Release, by hand:
/// `cargo test --release -p kentos-geometry-core --test all network_turns::timing -- --ignored --nocapture`.
#[test]
#[ignore = "timings, run by hand in release"]
fn timing() {
    use kentos_geometry_core::ops::network::NetworkSession;
    use std::time::Instant;

    const N: usize = 225;
    const STEP: f64 = 50.0;
    let (e0, n0) = (487_000.0, 4_420_000.0);
    let mut seed = 0x9e37_79b9_7f4a_7c15_u64;
    let mut next = move || {
        seed ^= seed >> 12;
        seed ^= seed << 25;
        seed ^= seed >> 27;
        (seed.wrapping_mul(0x2545_f491_4f6c_dd1d) >> 11) as f64 / (1u64 << 53) as f64
    };
    let at = |i: usize, j: usize| Vec2::new(e0 + i as f64 * STEP, n0 + j as f64 * STEP);
    let mut edges = Vec::new();
    let mut street = |a: Vec2, b: Vec2, r: &mut dyn FnMut() -> f64| {
        let one_way = r();
        let speed = [30, 50, 70][(r() * 3.0) as usize % 3];
        let morning = if r() < 0.33 {
            (speed / 2).to_string()
        } else {
            String::new()
        };
        edges.push(EdgeIn {
            id: edges.len() as f64 + 1.0,
            part: 0,
            path: bulge_path_edges(&[a, b], None, false),
            direction: (one_way < 0.05)
                .then(|| "FT".to_owned())
                .or_else(|| (one_way < 0.1).then(|| "TF".to_owned())),
            costs: vec![Some(speed.to_string())],
            closed: r() < 0.01,
            levels: [None, None, None],
            periods: vec![vec![Some(morning)]],
            terminals: [None, None],
            facts: Vec::new(),
        });
    };
    for i in 0..N {
        for j in 0..N {
            if i + 1 < N {
                street(at(i, j), at(i + 1, j), &mut next);
            }
            if j + 1 < N {
                street(at(i, j), at(i, j + 1), &mut next);
            }
        }
    }
    // A forbidden left turn at every tenth crossing: from the west going on north.
    let mut turns = Vec::new();
    for i in (1..N - 1).step_by(10) {
        for j in (1..N - 1).step_by(10) {
            let c = at(i, j);
            turns.push(TurnIn {
                id: 1e7 + turns.len() as f64,
                pts: vec![Vec2::new(c.x - 10.0, c.y), c, Vec2::new(c.x, c.y + 10.0)],
                field: false,
                value: None,
            });
        }
    }
    let base = r#"{"connect":"ends","tolerance":0.01,"direction":{"kind":"field","field":"yon","forward":["FT"],"backward":["TF"],"closed":["N"]},"costs":[{"name":"Süre","kind":"speed","field":"hiz","speed":50,"periods":[{"from":420,"to":540,"field":"hiz_sabah"}]}]"#;
    let rules = |turns: bool| {
        let text = if turns {
            format!(r#"{base},"turns":{{"uTurns":"deadEnds","right":10,"left":30,"back":60}}}}"#)
        } else {
            format!("{base}}}")
        };
        Rules::from_json(&Json::parse(&text).expect("JSON")).expect("rules")
    };
    let row = |what: &str, ms: f64, budget: f64| {
        println!(
            "{what:<56} {ms:>9.3} ms   bütçe {budget:>7} ms  {}",
            if ms <= budget { "✓" } else { "✗" }
        );
    };
    let least = |f: &mut dyn FnMut()| {
        (0..5)
            .map(|_| {
                let t = Instant::now();
                f();
                t.elapsed().as_secs_f64() * 1e3
            })
            .fold(f64::INFINITY, f64::min)
    };
    let off = |p: Vec2, dx: f64, dy: f64| Vec2::new(p.x + dx, p.y + dy);
    let middle = off(at(N / 2, N / 2), STEP / 2.0, 0.0);
    let far = [
        off(at(5, 7), 0.0, STEP / 2.0),
        off(at(N - 6, N - 9), STEP / 2.0, 0.0),
    ];

    let mut plain = None;
    let ms = least(&mut || {
        let (g, r) = Graph::build(rules(false), &edges, &[]);
        plain = Some(NetworkSession::new(g, r, Vec::new()));
    });
    let mut plain = plain.expect("built");
    row("kurma, dönüşsüz (ADR 0209)", ms, 150.0);
    let ms = least(&mut || assert!(plain.tree_at(middle, 30.0, 1, &[])));
    row("bütün ağda Dijkstra, dönüşsüz (ADR 0209)", ms, 30.0);

    let mut net = None;
    let ms = least(&mut || {
        let (g, r) = Graph::build_with(rules(true), &edges, &[], &turns);
        net = Some(NetworkSession::new(g, r, Vec::new()));
    });
    let mut net = net.expect("built");
    row(
        &format!(
            "kurma, dönüşlerle ({} parça, {} dönüş)",
            net.graph.pieces.len(),
            turns.len()
        ),
        ms,
        150.0,
    );
    let ms = least(&mut || assert!(net.tree_at(middle, 30.0, 1, &[])));
    row("bütün ağda Dijkstra, dönüşlerle (Süre)", ms, 30.0);
    {
        use kentos_geometry_core::ops::network::search::{POPS, PUSHES};
        use std::sync::atomic::Ordering::Relaxed;
        PUSHES.store(0, Relaxed); POPS.store(0, Relaxed);
        assert!(net.tree_at(middle, 30.0, 1, &[]));
        println!("pushes {} pops {}", PUSHES.load(Relaxed), POPS.load(Relaxed));
        PUSHES.store(0, Relaxed); POPS.store(0, Relaxed);
        assert!(plain.tree_at(middle, 30.0, 1, &[]));
        println!("plain pushes {} pops {}", PUSHES.load(Relaxed), POPS.load(Relaxed));
    }
    let ms = least(&mut || assert!(net.tree_at(middle, 30.0, 0, &[])));
    row("bütün ağda Dijkstra, dönüşlerle (Uzunluk)", ms, 30.0);
    net.time = Some(450.0);
    let ms = least(&mut || assert!(net.tree_at(middle, 30.0, 1, &[])));
    row("bütün ağda Dijkstra, dönüşler ve 07:30 profili", ms, 30.0);
    let span = STEP * (N - 1) as f64;
    let probes: Vec<Vec2> = (0..1_000)
        .map(|_| Vec2::new(e0 + next() * span, n0 + next() * span))
        .collect();
    let mut out = Vec::new();
    let ms = least(&mut || {
        for p in &probes {
            out.clear();
            net.path_to(p.x, p.y, 30.0, &mut out);
        }
    });
    row(
        "ağaçtan imlece yol, bir nokta (dönüş, profil)",
        ms / 1_000.0,
        0.1,
    );
    let mut minutes = 0.0;
    let ms = least(&mut || {
        minutes = net
            .route(&far, &[], 30.0, 1, Reorder::None)
            .expect("a route across the city")
            .cost;
    });
    row(
        &format!("köşeden köşeye rota, 07:30 ({minutes:.1} dk)"),
        ms,
        30.0,
    );
    let mut count = 0;
    let ms = least(&mut || {
        count = net
            .service_area(
                &[middle],
                &[5.0, 10.0],
                30.0,
                1,
                false,
                false,
                &[],
                25.0,
                false,
                false,
            )
            .expect("lines")
            .lines
            .len();
    });
    row(
        &format!("10 dakikalık hizmet alanının çizgileri, 07:30 ({count})"),
        ms,
        30.0,
    );
    net.time = None;
    let ms = least(&mut || {
        count = net
            .service_area(
                &[middle],
                &[5.0, 10.0],
                30.0,
                1,
                false,
                false,
                &[],
                25.0,
                false,
                false,
            )
            .expect("lines")
            .lines
            .len();
    });
    row(&format!("aynısı saatsiz, dönüşlerle ({count})"), ms, 30.0);
}
