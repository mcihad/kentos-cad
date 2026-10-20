//! Terminals and subnetworks (docs/adr/0557) against fixtures/network/v1/terminals.json
//! (`scripts/fixtures/network_terminals_cases.py`: the ADR's rules in Python, no KentOS code, a plain search over
//! (node, terminal) pairs): each scene's edge ends that name no terminal, its traces (Bağlı, Akış aşağı, Akış yukarı and
//! Yalıtım from the middle of an edge or from a node) and the subnetworks of groups of controllers.

// Test harness code, not the core: the std float methods are fine here.
#![allow(clippy::disallowed_methods)]

use kentos_geometry_core::Vec2;
use kentos_geometry_core::api::json::Json;
use kentos_geometry_core::geom::intersect::Edge;
use kentos_geometry_core::ops::network::area::{AreaQuery, service_lines};
use kentos_geometry_core::ops::network::closest::{Nearest, nearest};
use kentos_geometry_core::ops::network::route::{RouteError, route};
use kentos_geometry_core::ops::network::trace::trace;
use kentos_geometry_core::ops::network::{
    EdgeIn, Graph, JunctionIn, Location, Reach, Reorder, Role, Rules, Searcher, TraceKind,
    subnetworks::subnetworks,
};
use serde_json::Value;

fn fixture() -> Value {
    let path = format!(
        "{}/../../../fixtures/network/v1/terminals.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let file: Value =
        serde_json::from_str(&std::fs::read_to_string(path).expect("the fixture")).expect("JSON");
    assert_eq!(file["format"], "kentos.network-terminals");
    assert_eq!(file["version"], 1);
    file
}

fn num(v: &Value) -> f64 {
    v.as_f64().expect("a number")
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
            let mut edge = EdgeIn::new(
                num(&e["id"]),
                0,
                pts.windows(2)
                    .map(|w| Edge::Seg { a: w[0], b: w[1] })
                    .collect(),
                None,
                Vec::new(),
                e["closed"].as_bool().unwrap_or(false),
            );
            edge.terminals = [text(&e["terminals"][0]), text(&e["terminals"][1])];
            edge
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
            level: None,
            // A device's configuration is the one its point's attribute names (docs/adr/0562 §2).
            device: j["device"].as_u64().and_then(|d| {
                let layer = scene["rules"]["terminals"]["devices"][d as usize]["layer"]
                    .as_str()
                    .expect("a device layer");
                rules
                    .device_for(layer, j["config"].as_str())
                    .map(|(i, _)| i)
            }),
        })
        .collect();
    Graph::build(rules, &edges, &junctions)
}

fn kind(name: &str) -> TraceKind {
    match name {
        "downstream" => TraceKind::Downstream,
        "upstream" => TraceKind::Upstream,
        "isolation" => TraceKind::Isolation,
        _ => TraceKind::Connected,
    }
}

#[test]
fn terminals_and_subnetworks_give_what_the_independent_reference_gives() {
    let file = fixture();
    let (mut traces, mut subnets) = (0, 0);
    for case in file["cases"].as_array().expect("cases") {
        let scene = &case["scene"];
        let name = scene["name"].as_str().expect("a name");
        let (g, report) = build(scene);
        let want: Vec<(f64, Vec2)> = case["unterminated"]
            .as_array()
            .expect("unterminated")
            .iter()
            .map(|u| (num(&u[0]), point(&u[1])))
            .collect();
        assert_eq!(report.unterminated, want, "{name}: unterminated ends");
        for q in case["traces"].as_array().expect("traces") {
            let query = &q["query"];
            let at = match query["edge"].as_u64() {
                Some(i) => {
                    let pts = scene["edges"][i as usize]["pts"].as_array().expect("pts");
                    let (a, b) = (point(&pts[0]), point(&pts[pts.len() - 1]));
                    Vec2::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0)
                }
                None => point(&query["node"]),
            };
            let loc = g
                .locate(at, 1.0)
                .unwrap_or_else(|| panic!("{name}: {query} is on no edge"));
            let t = trace(
                &g,
                &[loc],
                &[],
                kind(query["kind"].as_str().expect("a kind")),
            );
            let ids: Vec<f64> = t.edges.iter().map(|&e| g.edges[e as usize].id).collect();
            let want_ids: Vec<f64> = q["edges"]
                .as_array()
                .expect("edges")
                .iter()
                .map(num)
                .collect();
            assert_eq!(ids, want_ids, "{name}: {query}: edges");
            let want_j: Vec<u32> = q["junctions"]
                .as_array()
                .expect("junctions")
                .iter()
                .map(|j| j.as_u64().expect("an index") as u32)
                .collect();
            assert_eq!(t.junctions, want_j, "{name}: {query}: junctions");
            if let Some(valves) = q["valves"].as_array() {
                let want_v: Vec<u32> = valves
                    .iter()
                    .map(|v| v.as_u64().expect("an index") as u32)
                    .collect();
                assert_eq!(t.valves, want_v, "{name}: {query}: valves");
            }
            traces += 1;
        }
        for q in case["subnets"].as_array().expect("subnets") {
            let query = &q["query"];
            let groups: Vec<Vec<u32>> = query["groups"]
                .as_array()
                .expect("groups")
                .iter()
                .map(|g| {
                    g.as_array()
                        .expect("a group")
                        .iter()
                        .map(|k| k.as_u64().expect("an index") as u32)
                        .collect()
                })
                .collect();
            let reach = match query["reach"].as_str() {
                Some("downstream") => Reach::Downstream,
                _ => Reach::Connected,
            };
            let got = subnetworks(&g, &groups, reach);
            let want = q["reached"].as_array().expect("reached");
            assert_eq!(got.len(), want.len(), "{name}: {query}");
            for (k, (r, w)) in got.iter().zip(want).enumerate() {
                let want_e: Vec<u32> = w["edges"]
                    .as_array()
                    .expect("edges")
                    .iter()
                    .map(|e| e.as_u64().expect("an index") as u32)
                    .collect();
                let want_j: Vec<u32> = w["junctions"]
                    .as_array()
                    .expect("junctions")
                    .iter()
                    .map(|j| j.as_u64().expect("an index") as u32)
                    .collect();
                assert_eq!(r.edges, want_e, "{name}: {query}: group {k}: edges");
                assert_eq!(r.junctions, want_j, "{name}: {query}: group {k}: junctions");
                assert!(
                    (r.length - num(&w["length"])).abs() < 1e-9,
                    "{name}: {query}: group {k}: length {} ≠ {}",
                    r.length,
                    w["length"]
                );
            }
            subnets += 1;
        }
    }
    assert!(
        traces >= 20 && subnets >= 10,
        "{traces} traces, {subnets} subnetworks"
    );
}

/// A location of the fixture: a fraction of an edge's length, or a node.
fn place(g: &Graph, scene: &Value, q: &Value) -> Location {
    let at = match q["edge"].as_u64() {
        Some(i) => {
            let pts = scene["edges"][i as usize]["pts"].as_array().expect("pts");
            let (a, b) = (point(&pts[0]), point(&pts[pts.len() - 1]));
            let f = q["at"].as_f64().unwrap_or(0.5);
            Vec2::new(a.x + (b.x - a.x) * f, a.y + (b.y - a.y) * f)
        }
        None => point(&q["node"]),
    };
    g.locate(at, 1.0)
        .unwrap_or_else(|| panic!("{q} is on no edge"))
}

fn places(g: &Graph, scene: &Value, list: &Value) -> Vec<Location> {
    list.as_array()
        .expect("places")
        .iter()
        .map(|q| place(g, scene, q))
        .collect()
}

fn close(got: f64, want: &Value, what: &str) {
    let want = num(want);
    assert!((got - want).abs() < 1e-6, "{what}: {got} ≠ {want}");
}

/// A cost row as the fixture has it: costs by target, none where not reached.
fn row(
    found: &[(
        kentos_geometry_core::ops::network::closest::Found,
        Option<kentos_geometry_core::ops::network::Path>,
    )],
    n: usize,
) -> Vec<Option<f64>> {
    let mut out = vec![None; n];
    for (f, _) in found {
        out[f.target] = Some(f.cost);
    }
    out
}

fn check_row(got: &[Option<f64>], want: &Value, what: &str) {
    let want = want.as_array().expect("a row");
    assert_eq!(got.len(), want.len(), "{what}");
    for (j, (g, w)) in got.iter().zip(want).enumerate() {
        match (g, w.as_f64()) {
            (Some(g), Some(w)) => assert!((g - w).abs() < 1e-6, "{what}[{j}]: {g} ≠ {w}"),
            (None, None) => {}
            (g, w) => panic!("{what}[{j}]: {g:?} ≠ {w:?}"),
        }
    }
}

/// Docs/adr/0562 §1: a shortest route, the cost matrix both ways, the closest facilities and the service areas cross
/// a device only along its terminal paths, as the reference's search over (node, terminal) states does.
#[test]
fn every_search_crosses_a_device_only_along_its_paths() {
    let file = fixture();
    let (mut routes, mut matrices, mut areas) = (0, 0, 0);
    for case in file["cases"].as_array().expect("cases") {
        let scene = &case["scene"];
        let name = scene["name"].as_str().expect("a name");
        let (g, _) = build(scene);
        let mut s = Searcher::new();
        for q in case["routes"].as_array().expect("routes") {
            let stops = places(&g, scene, &q["query"]["stops"]);
            let got = route(&g, &mut s, &stops, &[], 0, Reorder::None, None);
            match (got, q["cost"].as_f64()) {
                (Ok(r), Some(want)) => {
                    assert!(
                        (r.cost - want).abs() < 1e-6,
                        "{name}: {}: cost {} ≠ {want}",
                        q["query"],
                        r.cost
                    );
                    let mut ids: Vec<f64> = Vec::new();
                    for span in &r.spans {
                        let id = g.edges[g.pieces[span.piece as usize].edge as usize].id;
                        if ids.last() != Some(&id) {
                            ids.push(id);
                        }
                    }
                    let want_ids: Vec<f64> = q["edges"]
                        .as_array()
                        .expect("edges")
                        .iter()
                        .map(num)
                        .collect();
                    assert_eq!(ids, want_ids, "{name}: {}: edges", q["query"]);
                }
                (Err(RouteError::Unreachable(..)), None) => {}
                (got, want) => panic!("{name}: {}: {got:?} ≠ {want:?}", q["query"]),
            }
            routes += 1;
        }
        for q in case["matrices"].as_array().expect("matrices") {
            let query = &q["query"];
            let origins = places(&g, scene, &query["origins"]);
            let targets = places(&g, scene, &query["targets"]);
            let base = Nearest {
                origins: &origins,
                targets: &targets,
                k: None,
                cutoff: None,
                cost: 0,
                reverse: false,
                barriers: &[],
                at: None,
            };
            for (reverse, key) in [(false, "from"), (true, "to")] {
                let rows = nearest(&g, &mut s, &Nearest { reverse, ..base }, false);
                for (i, found) in rows.iter().enumerate() {
                    check_row(
                        &row(found, targets.len()),
                        &q[key][i],
                        &format!("{name}: {query}: {key}[{i}]"),
                    );
                }
            }
            for c in q["closest"].as_array().expect("closest") {
                let k = c["k"].as_u64().expect("k") as usize;
                let rows = nearest(
                    &g,
                    &mut s,
                    &Nearest {
                        k: Some(k),
                        cutoff: c["cutoff"].as_f64(),
                        ..base
                    },
                    false,
                );
                for (i, found) in rows.iter().enumerate() {
                    let want = c["rows"][i].as_array().expect("a row");
                    assert_eq!(
                        found.len(),
                        want.len(),
                        "{name}: {query}: closest {k}: row {i}"
                    );
                    for ((f, _), w) in found.iter().zip(want) {
                        assert_eq!(f.target as u64, w[0].as_u64().expect("a target"));
                        close(f.cost, &w[1], &format!("{name}: {query}: closest {k}"));
                    }
                }
            }
            matrices += 1;
        }
        for q in case["areas"].as_array().expect("areas") {
            let query = &q["query"];
            let facilities = places(&g, scene, &query["facilities"]);
            let lines = service_lines(
                &g,
                &mut s,
                &AreaQuery {
                    facilities: &facilities,
                    breaks: &[num(&query["break"])],
                    cost: 0,
                    toward: query["toward"].as_bool().unwrap_or(false),
                    separate: false,
                    barriers: &[],
                    at: None,
                },
            );
            let mut by_edge: std::collections::BTreeMap<u64, f64> = Default::default();
            for l in &lines {
                let id = g.edges[g.pieces[l.span.piece as usize].edge as usize].id;
                *by_edge.entry(id as u64).or_default() += (l.span.b - l.span.a).abs();
            }
            let want = q["lengths"].as_array().expect("lengths");
            assert_eq!(by_edge.len(), want.len(), "{name}: {query}: {by_edge:?}");
            for (w, (id, len)) in want.iter().zip(&by_edge) {
                assert_eq!(w[0].as_u64().expect("an id"), *id, "{name}: {query}");
                close(*len, &w[1], &format!("{name}: {query}: edge {id}"));
            }
            areas += 1;
        }
    }
    assert!(
        routes >= 20 && matrices >= 6 && areas >= 15,
        "{routes} routes, {matrices} matrices, {areas} areas"
    );
}

/// ADR 0562 §1's cost on the synthetic grid city of ADR 0209 §12 (225 × 225 crossings 50 m apart, 100 800 pieces, a tenth
/// of the streets one way, speeds 30, 50 or 70 km/h, a street in a hundred closed): beside the city as it is, the same
/// city with a four-way device (terminals w, e, s, n) at every tenth crossing as a straight-through valve (the paths w–e
/// and s–n) and at every crossing as a junction box (all four linked), every edge end there naming its terminal. The searches of a
/// network with devices go by (node, terminal) states, so each question is timed with and without them on the same
/// machine at the same moment, each the least of five. Release, by hand:
/// `cargo test --release -p kentos-geometry-core --test all network_terminals::timing -- --ignored --nocapture`.
#[test]
#[ignore = "timings, run by hand in release"]
fn timing() {
    use kentos_geometry_core::geom::bulge::bulge_path_edges;
    use kentos_geometry_core::ops::network::NetworkSession;
    use std::time::Instant;

    const N: usize = 225;
    const STEP: f64 = 50.0;
    let (e0, n0) = (487_000.0, 4_420_000.0);
    let at = |i: usize, j: usize| Vec2::new(e0 + i as f64 * STEP, n0 + j as f64 * STEP);
    let devices =
        |every: usize, i: usize, j: usize| i.is_multiple_of(every) && j.is_multiple_of(every);
    let city = |every: Option<usize>| {
        let mut seed = 0x9e37_79b9_7f4a_7c15_u64;
        let mut next = move || {
            seed ^= seed >> 12;
            seed ^= seed << 25;
            seed ^= seed >> 27;
            (seed.wrapping_mul(0x2545_f491_4f6c_dd1d) >> 11) as f64 / (1u64 << 53) as f64
        };
        let mut edges = Vec::new();
        let mut street = |a: Vec2, b: Vec2, ends: [Option<&str>; 2], r: &mut dyn FnMut() -> f64| {
            let one_way = r();
            let speed = [30, 50, 70][(r() * 3.0) as usize % 3];
            let mut edge = EdgeIn::new(
                edges.len() as f64 + 1.0,
                0,
                bulge_path_edges(&[a, b], None, false),
                (one_way < 0.05)
                    .then(|| "FT".to_owned())
                    .or_else(|| (one_way < 0.1).then(|| "TF".to_owned())),
                vec![Some(speed.to_string())],
                r() < 0.01,
            );
            edge.terminals = [ends[0].map(str::to_owned), ends[1].map(str::to_owned)];
            edges.push(edge);
        };
        let named = |i: usize, j: usize, name: &'static str| {
            every.filter(|k| devices(*k, i, j)).map(|_| name)
        };
        for i in 0..N {
            for j in 0..N {
                if i + 1 < N {
                    street(
                        at(i, j),
                        at(i + 1, j),
                        [named(i, j, "e"), named(i + 1, j, "w")],
                        &mut next,
                    );
                }
                if j + 1 < N {
                    street(
                        at(i, j),
                        at(i, j + 1),
                        [named(i, j, "n"), named(i, j + 1, "s")],
                        &mut next,
                    );
                }
            }
        }
        let mut junctions = Vec::new();
        if let Some(k) = every {
            for i in (0..N).filter(|i| i.is_multiple_of(k)) {
                for j in (0..N).filter(|j| j.is_multiple_of(k)) {
                    junctions.push(JunctionIn {
                        id: 1e7 + junctions.len() as f64,
                        p: at(i, j),
                        role: Role::Valve,
                        closed: false,
                        level: None,
                        device: Some(0),
                    });
                }
            }
        }
        (edges, junctions)
    };
    // Every tenth crossing a straight-through valve (w–e and s–n); a device at every crossing a junction box (all four
    // linked), so the worst case measures the machinery and not a city nobody can cross.
    let rules = |every: Option<usize>| {
        let devices = |paths: &str| {
            format!(
                r#","terminals":{{"start":"uc_bas","end":"uc_son","devices":[{{"layer":"vana",
            "terminals":[{{"id":"w","name":"batı"}},{{"id":"e","name":"doğu"}},{{"id":"s","name":"güney"}},{{"id":"n","name":"kuzey"}}],
            "paths":{paths}}}]}}"#
            )
        };
        let text = format!(
            r#"{{"connect":"ends","tolerance":0.01,"direction":{{"kind":"field","field":"yon","forward":["FT"],"backward":["TF"],"closed":["N"]}},
                "costs":[{{"name":"Süre","kind":"speed","field":"hiz","speed":50}}]{}}}"#,
            match every {
                None => String::new(),
                Some(1) => devices(r#"[["w","e","s","n"]]"#),
                Some(_) => devices(r#"[["w","e"],["s","n"]]"#),
            }
        );
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
    let mut nets: Vec<(&str, NetworkSession)> = Vec::new();
    for (label, every) in [
        ("cihazsız (ADR 0209)", None),
        ("her onuncu kavşakta cihaz", Some(10)),
        ("her kavşakta cihaz", Some(1)),
    ] {
        let (edges, junctions) = city(every);
        let mut net = None;
        let ms = least(&mut || {
            let (g, r) = Graph::build(rules(every), &edges, &junctions);
            net = Some(NetworkSession::new(g, r, Vec::new()));
        });
        let net = net.expect("built");
        row(
            &format!(
                "kurma, {label} ({} parça, {} cihaz)",
                net.graph.pieces.len(),
                junctions.len()
            ),
            ms,
            150.0 * if every.is_some() { 2.0 } else { 1.0 },
        );
        nets.push((label, net));
    }
    for (label, net) in &mut nets {
        let ms = least(&mut || assert!(net.tree_at(middle, 30.0, 1, &[])));
        row(&format!("bütün ağda Dijkstra, {label}"), ms, 30.0);
        let mut minutes = 0.0;
        let ms = least(&mut || {
            minutes = net
                .route(&far, &[], 30.0, 1, Reorder::None)
                .expect("a route across the city")
                .cost;
        });
        row(
            &format!("köşeden köşeye rota, {label} ({minutes:.1} dk)"),
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
            &format!("10 dakikalık hizmet alanının çizgileri, {label} ({count})"),
            ms,
            30.0,
        );
    }
}
