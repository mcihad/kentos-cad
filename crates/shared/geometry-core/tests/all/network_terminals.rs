//! Terminals and subnetworks (docs/adr/0557) against fixtures/network/v1/terminals.json
//! (`scripts/fixtures/network_terminals_cases.py`: the ADR's rules in Python, no KentOS code, a plain search over
//! (node, terminal) pairs): each scene's edge ends that name no terminal, its traces (Bağlı, Akış aşağı, Akış yukarı and
//! Yalıtım from the middle of an edge or from a node) and the subnetworks of groups of controllers.

// Test harness code, not the core: the std float methods are fine here.
#![allow(clippy::disallowed_methods)]

use kentos_geometry_core::Vec2;
use kentos_geometry_core::api::json::Json;
use kentos_geometry_core::geom::intersect::Edge;
use kentos_geometry_core::ops::network::trace::trace;
use kentos_geometry_core::ops::network::{
    EdgeIn, Graph, JunctionIn, Reach, Role, Rules, TraceKind, subnetworks::subnetworks,
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
            device: j["device"].as_u64().map(|d| d as u32),
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
