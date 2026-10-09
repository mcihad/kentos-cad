//! Ağ analizi (docs/adr/0209 §3–§9) against fixtures/network/v1/cases.json
//! (`scripts/fixtures/network_cases.py`: the ADR's rules in Python, no KentOS
//! code; shapely for the areas): each scene's graph, places, routes, service
//! areas, closest facilities, cost matrices, traces and Denetle. Nodes,
//! pieces, orders, edges and valves exactly; offsets, lengths and costs within
//! 1e-9 m (of their size above 1); areas within 1e-5 of themselves.

// Test harness code, not the core: the std float methods are fine here.
#![allow(clippy::disallowed_methods)]

use kentos_geometry_core::Vec2;
use kentos_geometry_core::api::json::Json;
use kentos_geometry_core::geom::bulge::bulge_path_edges;
use kentos_geometry_core::ops::geoprocess::areas_measure;
use kentos_geometry_core::ops::network::area::{AreaQuery, service_lines, service_polygons};
use kentos_geometry_core::ops::network::check::{ProblemKind, check};
use kentos_geometry_core::ops::network::closest::{Nearest, nearest};
use kentos_geometry_core::ops::network::route::{Reorder, RouteError, route};
use kentos_geometry_core::ops::network::trace::{TraceKind, trace};
use kentos_geometry_core::ops::network::{
    EdgeIn, Graph, JunctionIn, Location, Role, Rules, Searcher, Span,
};
use serde_json::Value;
use std::collections::HashMap;

fn fixture() -> Value {
    let path = format!(
        "{}/../../../fixtures/network/v1/cases.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let file: Value =
        serde_json::from_str(&std::fs::read_to_string(path).expect("the fixture")).expect("JSON");
    assert_eq!(file["format"], "kentos.network-cases");
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
            EdgeIn {
                id: num(&e["id"]),
                part: 0,
                path: bulge_path_edges(&pts, bulges.as_deref(), false),
                direction: e["direction"].as_str().map(str::to_owned),
                costs: e["costs"]
                    .as_array()
                    .expect("costs")
                    .iter()
                    .map(|c| c.as_str().map(str::to_owned))
                    .collect(),
                closed: e["closed"].as_bool().unwrap_or(false),
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
        })
        .collect();
    Graph::build(rules, &edges, &junctions)
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
fn the_analyses_give_what_the_independent_reference_gives() {
    let file = fixture();
    for case in file["cases"].as_array().expect("cases") {
        let scene = &case["scene"];
        let name = scene["name"].as_str().expect("a name");
        let (g, report) = build(scene);
        // The graph.
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
        // The places.
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
            near(
                loc.offset,
                num(&p["offset"]),
                &format!("{name}: place {key}"),
            );
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
            match q["kind"].as_str().expect("a kind") {
                "route" => {
                    let reorder = match q["reorder"].as_str() {
                        Some("keepFirst") => Reorder::KeepFirst,
                        Some("keepFirstLast") => Reorder::KeepFirstLast,
                        _ => Reorder::None,
                    };
                    match (
                        route(&g, &mut s, &pick(&q["stops"]), &barriers, cost, reorder),
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
                        (Err(RouteError::NoOrder), Some("noOrder")) => {}
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
                        separate: q["separate"].as_bool().unwrap_or(false),
                        barriers: &barriers,
                    };
                    let mut lines = service_lines(&g, &mut s, &aq);
                    let key = |f: Option<usize>| f.map_or(-1, |x| x as i64);
                    lines.sort_by(|a, b| {
                        key(a.facility)
                            .cmp(&key(b.facility))
                            .then(a.band.cmp(&b.band))
                            .then(a.span.piece.cmp(&b.span.piece))
                            .then(a.span.a.total_cmp(&b.span.a))
                    });
                    let expected = want["lines"].as_array().expect("lines");
                    assert_eq!(lines.len(), expected.len(), "{title}: lines {lines:?}");
                    for (l, w) in lines.iter().zip(expected) {
                        assert_eq!(
                            key(l.facility),
                            w["facility"].as_i64().unwrap_or(-1),
                            "{title}"
                        );
                        assert_eq!(
                            l.band as u64,
                            w["band"].as_u64().expect("a band"),
                            "{title}"
                        );
                        assert_eq!(
                            l.span.piece as u64,
                            w["piece"].as_u64().expect("a piece"),
                            "{title}"
                        );
                        near(l.span.a, num(&w["a"]), &title);
                        near(l.span.b, num(&w["b"]), &title);
                    }
                    let trim = num(&q["trim"]);
                    let discs = service_polygons(&g, &lines, breaks.len(), trim, false);
                    let rings = service_polygons(&g, &lines, breaks.len(), trim, true);
                    let areas = want["areas"].as_array().expect("areas");
                    assert_eq!(discs.len(), areas.len(), "{title}: areas");
                    for ((d, r), w) in discs.iter().zip(&rings).zip(areas) {
                        assert_eq!(
                            (key(d.0), d.1 as u64),
                            (
                                w["facility"].as_i64().unwrap_or(-1),
                                w["band"].as_u64().unwrap_or(9)
                            ),
                            "{title}"
                        );
                        let (da, ra) = (areas_measure(&d.2), areas_measure(&r.2));
                        let (wd, wr) = (num(&w["disc"]), num(&w["ring"]));
                        assert!((da - wd).abs() <= 1e-5 * wd, "{title}: disc {da} ≠ {wd}");
                        assert!((ra - wr).abs() <= 1e-5 * wr, "{title}: ring {ra} ≠ {wr}");
                    }
                }
                kind @ ("closest" | "matrix") => {
                    let origins = pick(&q["origins"]);
                    let targets = pick(&q["targets"]);
                    let paths = kind == "closest";
                    let nq = Nearest {
                        origins: &origins,
                        targets: &targets,
                        k: q["k"].as_u64().map(|k| k as usize),
                        cutoff: q["cutoff"].as_f64(),
                        cost,
                        reverse: q["reverse"].as_bool().unwrap_or(false),
                        barriers: &barriers,
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
                                spans_eq(
                                    &path.as_ref().expect("a way").spans,
                                    &wf["spans"],
                                    &title,
                                );
                            }
                        }
                    }
                }
                "trace" => {
                    let kind = match q["trace"].as_str() {
                        Some("downstream") => TraceKind::Downstream,
                        Some("upstream") => TraceKind::Upstream,
                        Some("isolation") => TraceKind::Isolation,
                        _ => TraceKind::Connected,
                    };
                    let t = trace(&g, &pick(&q["starts"]), &barriers, kind);
                    near(t.length, num(&want["length"]), &title);
                    let mut spans: Vec<(u32, f64, f64)> = t
                        .spans
                        .iter()
                        .map(|s| (s.piece, s.a.min(s.b), s.a.max(s.b)))
                        .collect();
                    spans.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.total_cmp(&b.1)));
                    let expected = want["spans"].as_array().expect("spans");
                    assert_eq!(spans.len(), expected.len(), "{title}: {spans:?}");
                    for (got, w) in spans.iter().zip(expected) {
                        assert_eq!(got.0 as u64, w[0].as_u64().expect("a piece"), "{title}");
                        near(got.1, num(&w[1]), &title);
                        near(got.2, num(&w[2]), &title);
                    }
                    let ints = |v: &Value| -> Vec<u64> {
                        v.as_array().map_or_else(Vec::new, |l| {
                            l.iter().map(|x| x.as_u64().unwrap_or(999)).collect()
                        })
                    };
                    assert_eq!(
                        t.edges.iter().map(|&e| e as u64).collect::<Vec<_>>(),
                        ints(&want["edges"]),
                        "{title}: edges"
                    );
                    if kind == TraceKind::Isolation {
                        assert_eq!(
                            t.valves.iter().map(|&v| v as u64).collect::<Vec<_>>(),
                            ints(&want["valves"]),
                            "{title}: valves"
                        );
                        if !want["unfed"].is_null() {
                            assert_eq!(
                                t.unfed.iter().map(|&p| p as u64).collect::<Vec<_>>(),
                                ints(&want["unfed"]),
                                "{title}: unfed"
                            );
                            assert_eq!(
                                t.unfed_edges.iter().map(|&e| e as u64).collect::<Vec<_>>(),
                                ints(&want["unfedEdges"]),
                                "{title}: unfed edges"
                            );
                            near(t.unfed_length, num(&want["unfedLength"]), &title);
                        }
                    }
                }
                "check" => {
                    let c = check(&g, &report);
                    assert_eq!(
                        c.nodes as u64,
                        want["nodes"].as_u64().expect("nodes"),
                        "{title}"
                    );
                    assert_eq!(
                        c.pieces as u64,
                        want["pieces"].as_u64().expect("pieces"),
                        "{title}"
                    );
                    near(c.length, num(&want["length"]), &title);
                    assert_eq!(
                        c.dead_ends as u64,
                        want["deadEnds"].as_u64().expect("dead ends"),
                        "{title}"
                    );
                    let parts = want["parts"].as_array().expect("parts");
                    assert_eq!(c.parts.len(), parts.len(), "{title}: parts");
                    for (got, w) in c.parts.iter().zip(parts) {
                        assert_eq!(got.0 as u64, w[0].as_u64().expect("a count"), "{title}");
                        near(got.1, num(&w[1]), &title);
                    }
                    let expected = want["problems"].as_array().expect("problems");
                    assert_eq!(
                        c.problems.len(),
                        expected.len(),
                        "{title}: {:?}",
                        c.problems
                    );
                    for (p, w) in c.problems.iter().zip(expected) {
                        let kind = match p.kind {
                            ProblemKind::Detached => "detached",
                            ProblemKind::NearMiss => "nearMiss",
                            ProblemKind::Crossing => "crossing",
                            ProblemKind::OffNetwork => "offNetwork",
                            ProblemKind::Short => "short",
                            ProblemKind::Unread => "unread",
                        };
                        assert_eq!(kind, w["kind"].as_str().expect("a kind"), "{title}");
                        near(p.at.x, num(&w["at"][0]), &title);
                        near(p.at.y, num(&w["at"][1]), &title);
                        let ids: Vec<f64> =
                            w["ids"].as_array().expect("ids").iter().map(num).collect();
                        assert_eq!(p.ids, ids, "{title}");
                        match (p.value, w["value"].as_f64()) {
                            (Some(a), Some(b)) => near(a, b, &title),
                            (None, None) => {}
                            other => panic!("{title}: value {other:?}"),
                        }
                    }
                }
                other => panic!("unknown question {other}"),
            }
        }
    }
}

/// TODOS.md `GEO-01`: the overlay's union of its own result with buffers whose end circles lie on that result's
/// boundary counts area twice. Here a service area's first band (one area, no holes) joined again with the second
/// band's buffers gives more than all the buffers joined at once (and than shapely's 27 590.47 m²). Kept ignored
/// until the overlay is fixed; `service_polygons` joins every band from its buffers.
#[test]
#[ignore = "the overlay's bug GEO-01, not fixed yet"]
fn geo01_a_union_joined_again_with_touching_circles() {
    use kentos_geometry_core::ops::geoprocess::buffer::edge_pieces;
    use kentos_geometry_core::ops::geoprocess::union_all;
    let file = fixture();
    let case = &file["cases"][0];
    let (g, _) = build(&case["scene"]);
    let q = &case["questions"][9];
    let trim = num(&q["trim"]);
    let (mut first, mut second) = (Vec::new(), Vec::new());
    for l in q["expect"]["lines"].as_array().expect("lines") {
        let mut edges = Vec::new();
        g.span_edges(
            l["piece"].as_u64().unwrap_or(0) as u32,
            num(&l["a"]),
            num(&l["b"]),
            &mut edges,
        );
        let into = if l["band"].as_u64() == Some(0) {
            &mut first
        } else {
            &mut second
        };
        for e in &edges {
            edge_pieces(e, trim, into);
        }
    }
    let mut all = first.clone();
    all.extend(second.iter().cloned());
    let at_once = areas_measure(&union_all(&all));
    let mut again = union_all(&first);
    again.extend(second);
    let joined_again = areas_measure(&union_all(&again));
    assert!(
        (joined_again - at_once).abs() <= 1e-6 * at_once,
        "{joined_again} ≠ {at_once}"
    );
}

/// ADR 0209 §12's budgets on a synthetic grid city: 225 × 225 crossings 50 m apart, every block its own line
/// (100 800 pieces), a tenth of them one way (half each way), speeds 30, 50 or 70 km/h, a street closed in a
/// hundred; the questions from the middle. Prints each time against its budget; release, by hand:
/// `cargo test --release -p kentos-geometry-core --test all network::timing -- --ignored --nocapture`.
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
        // xorshift64*: the same city every run.
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
        edges.push(EdgeIn {
            id: edges.len() as f64 + 1.0,
            part: 0,
            path: bulge_path_edges(&[a, b], None, false),
            direction: (one_way < 0.05)
                .then(|| "FT".to_owned())
                .or_else(|| (one_way < 0.1).then(|| "TF".to_owned())),
            costs: vec![Some(speed.to_string())],
            closed: r() < 0.01,
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
    let rules = Rules::from_json(
        &Json::parse(
            r#"{"connect":"ends","tolerance":0.01,"direction":{"kind":"field","field":"yon","forward":["FT"],"backward":["TF"],"closed":["N"]},
                "costs":[{"name":"Süre","kind":"speed","field":"hiz","speed":50}]}"#,
        )
        .expect("JSON"),
    )
    .expect("rules");
    let row = |what: &str, ms: f64, budget: f64| {
        println!(
            "{what:<44} {ms:>9.3} ms   bütçe {budget:>7} ms  {}",
            if ms <= budget { "✓" } else { "✗" }
        );
    };
    let t = Instant::now();
    let (graph, report) = Graph::build(rules, &edges, &[]);
    let mut net = NetworkSession::new(graph, report, Vec::new());
    row(
        &format!(
            "kurma ({} parça, {} düğüm)",
            net.graph.pieces.len(),
            net.graph.nodes.len()
        ),
        t.elapsed().as_secs_f64() * 1e3,
        150.0,
    );
    assert!(net.graph.pieces.len() >= 100_000);

    let span = STEP * (N - 1) as f64;
    let probes: Vec<Vec2> = (0..10_000)
        .map(|_| Vec2::new(e0 + next() * span, n0 + next() * span))
        .collect();
    let t = Instant::now();
    let found = probes
        .iter()
        .filter(|p| net.locate(**p, 30.0).is_some())
        .count();
    row(
        &format!("konum (10 000 nokta, {found} bulundu), bir nokta"),
        t.elapsed().as_secs_f64() * 1e3 / 10_000.0,
        0.05,
    );

    let off = |p: Vec2, dx: f64, dy: f64| Vec2::new(p.x + dx, p.y + dy);
    let middle = off(at(N / 2, N / 2), STEP / 2.0, 0.0);
    let t = Instant::now();
    assert!(net.tree_at(middle, 30.0, 1, &[]));
    row(
        "bütün ağda Dijkstra (ortadan, Süre)",
        t.elapsed().as_secs_f64() * 1e3,
        30.0,
    );

    let mut out = Vec::new();
    let t = Instant::now();
    for p in probes.iter().take(1_000) {
        out.clear();
        net.path_to(p.x, p.y, 30.0, &mut out);
    }
    row(
        "ağaçtan imlece yol, bir nokta",
        t.elapsed().as_secs_f64() * 1e3 / 1_000.0,
        0.1,
    );

    let far = [
        off(at(5, 7), 0.0, STEP / 2.0),
        off(at(N - 6, N - 9), STEP / 2.0, 0.0),
    ];
    let t = Instant::now();
    let r = net
        .route(&far, &[], 30.0, 1, Reorder::None)
        .expect("a route across the city");
    row(
        &format!("köşeden köşeye rota ({:.1} dk)", r.cost),
        t.elapsed().as_secs_f64() * 1e3,
        30.0,
    );

    let t = Instant::now();
    let traced = net
        .trace_of(&[middle], &[], 30.0, TraceKind::Connected)
        .expect("a trace");
    row(
        &format!("izleme, bağlı ({} nesne)", traced.objects.len()),
        t.elapsed().as_secs_f64() * 1e3,
        20.0,
    );

    let t = Instant::now();
    let lines = net
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
        .expect("lines");
    row(
        &format!(
            "10 dakikalık hizmet alanının çizgileri ({})",
            lines.lines.len()
        ),
        t.elapsed().as_secs_f64() * 1e3,
        30.0,
    );
    // The areas with the tools' Kenar payı (50 m: neighbouring streets' buffers overlap), and with half a block
    // (25 m: they touch along every block, the worst case of the overlay).
    for (trim, budget) in [(50.0, 300.0), (25.0, 300.0)] {
        let t = Instant::now();
        let areas = net
            .service_area(
                &[middle],
                &[5.0, 10.0],
                30.0,
                1,
                false,
                false,
                &[],
                trim,
                false,
                true,
            )
            .expect("areas");
        row(
            &format!(
                "aynısının alanları, pay {trim} m ({} alan)",
                areas.areas.len()
            ),
            t.elapsed().as_secs_f64() * 1e3,
            budget,
        );
    }

    // Smaller areas: a minute, two and four from the middle, the tools' Kenar payı.
    for minutes in [1.0, 2.0, 4.0] {
        let l = net
            .service_area(
                &[middle],
                &[minutes],
                30.0,
                1,
                false,
                false,
                &[],
                50.0,
                false,
                false,
            )
            .expect("lines");
        let t = Instant::now();
        net.service_area(
            &[middle],
            &[minutes],
            30.0,
            1,
            false,
            false,
            &[],
            50.0,
            false,
            true,
        )
        .expect("areas");
        row(
            &format!(
                "{minutes} dakikalık alan, pay 50 m ({} çizgi)",
                l.lines.len()
            ),
            t.elapsed().as_secs_f64() * 1e3,
            300.0,
        );
    }
    let t = Instant::now();
    let checked = net.checked();
    row(
        &format!("Denetle ({} sorun)", checked.problems.len()),
        t.elapsed().as_secs_f64() * 1e3,
        1000.0,
    );
}
