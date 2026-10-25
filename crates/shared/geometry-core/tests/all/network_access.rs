//! Vehicle profiles and dated closures (docs/adr/0567) against fixtures/network/v1/access.json
//! (`scripts/fixtures/network_access_cases.py`: the ADR's rules in Python, no KentOS code, a plain Dijkstra over the
//! grid's corners): what each question's access closed and could not read exactly, the cheapest way's cost and its
//! length and minutes within 1e-9 (of their size above 1), and the refusals' words.

// Test harness code, not the core: the std float methods are fine here.
#![allow(clippy::disallowed_methods)]

use kentos_geometry_core::Vec2;
use kentos_geometry_core::api::json::Json;
use kentos_geometry_core::geom::bulge::bulge_path_edges;
use kentos_geometry_core::ops::network::{
    EdgeIn, Graph, JunctionIn, NetworkSession, Refusal, Reorder, Role, Rules, TurnIn,
};
use serde_json::Value;

fn fixture() -> Value {
    let path = format!(
        "{}/../../../fixtures/network/v1/access.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let file: Value =
        serde_json::from_str(&std::fs::read_to_string(path).expect("the fixture")).expect("JSON");
    assert_eq!(file["format"], "kentos.network-access");
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

fn session(file: &Value) -> NetworkSession {
    let rules = Rules::from_json(&Json::parse(&file["definition"].to_string()).expect("JSON"))
        .expect("rules");
    let attr = |e: &Value, f: &str| e["attrs"][f].as_str().map(str::to_owned);
    let edges: Vec<EdgeIn> = file["edges"]
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
                bulge_path_edges(&pts, None, false),
                attr(e, "yon"),
                file["definition"]["costs"]
                    .as_array()
                    .expect("costs")
                    .iter()
                    .map(|c| attr(e, c["field"].as_str().expect("a field")))
                    .collect(),
                e["closed"].as_bool().unwrap_or(false),
            );
            edge.facts = rules.access.facts.iter().map(|f| attr(e, f)).collect();
            // The level fields, when the definition names them (docs/adr/0640 §3).
            let level = |k: &str| {
                file["definition"]["levels"][k]
                    .as_str()
                    .and_then(|f| attr(e, f))
            };
            edge.levels = [level("start"), level("end"), level("inside")];
            edge
        })
        .collect();
    // The junctions (closed by their attribute) and the turn lines (forbidden: the layer has no field) (docs/adr/0577).
    let junctions: Vec<JunctionIn> = file["junctions"]
        .as_array()
        .map(|js| {
            js.iter()
                .map(|j| JunctionIn {
                    id: num(&j["id"]),
                    p: point(&j["p"]),
                    role: Role::Junction,
                    closed: j["attrs"]["kapali"].as_str() == Some("evet"),
                    level: None,
                    device: None,
                })
                .collect()
        })
        .unwrap_or_default();
    let turns: Vec<TurnIn> = file["turns"]
        .as_array()
        .map(|ts| {
            ts.iter()
                .map(|t| TurnIn {
                    id: num(&t["id"]),
                    pts: t["pts"]
                        .as_array()
                        .expect("points")
                        .iter()
                        .map(point)
                        .collect(),
                    field: false,
                    value: None,
                })
                .collect()
        })
        .unwrap_or_default();
    let (graph, report) = Graph::build_with(rules, &edges, &junctions, &turns);
    NetworkSession::new(graph, report, Vec::new())
}

#[test]
fn profiles_and_closures_give_what_the_independent_reference_gives() {
    let file = fixture();
    let mut s = session(&file);
    let cases = file["cases"].as_array().expect("cases");
    assert_eq!(cases.len(), 26);
    // Twice through: an access asked again, and one after another, gives the same.
    for round in 0..2 {
        for c in cases {
            let name = c["name"].as_str().expect("a name");
            let varied = s
                .set_access(c["profile"].as_str(), c["day"].as_str())
                .unwrap_or_else(|e| panic!("{name}: {e}"));
            assert_eq!(
                varied.by_profile as u64,
                c["byProfile"].as_u64().unwrap(),
                "{name}"
            );
            assert_eq!(
                varied.by_date as u64,
                c["byDate"].as_u64().unwrap(),
                "{name}"
            );
            let unread: Vec<f64> = c["unread"].as_array().unwrap().iter().map(num).collect();
            assert_eq!(varied.unread, unread, "{name}");
            let stops = [point(&c["from"]), point(&c["to"])];
            let cost = c["cost"].as_u64().unwrap() as usize;
            let got = s.route(&stops, &[], 1.0, cost, Reorder::None);
            if c["unreachable"].as_bool() == Some(true) {
                assert!(matches!(got, Err(Refusal::Unreachable(0, 1, _))), "{name}");
                continue;
            }
            let r = got.unwrap_or_else(|e| panic!("{name} ({round}): {e:?}"));
            near(r.cost, num(&c["total"]), name);
            near(r.totals[0].expect("a length"), num(&c["totals"][0]), name);
            near(r.totals[1].expect("minutes"), num(&c["totals"][1]), name);
        }
    }
    for r in file["refusals"].as_array().expect("refusals") {
        let e = s
            .set_access(r["profile"].as_str(), r["day"].as_str())
            .expect_err("a refusal");
        assert_eq!(e, r["error"].as_str().unwrap());
    }
}

#[test]
fn no_access_is_the_network_as_defined() {
    let file = fixture();
    let mut s = session(&file);
    let stops = [
        point(&file["cases"][1]["from"]),
        point(&file["cases"][1]["to"]),
    ];
    let before = s.route(&stops, &[], 1.0, 1, Reorder::None).expect("a way");
    s.set_access(Some("Yaya"), Some("2026-11-03"))
        .expect("an access");
    let varied = s.route(&stops, &[], 1.0, 1, Reorder::None).expect("a way");
    assert!(varied.cost > before.cost);
    s.set_access(None, None).expect("none");
    let after = s.route(&stops, &[], 1.0, 1, Reorder::None).expect("a way");
    assert_eq!(after, before);
}

/// A closure's weekdays and hours (docs/adr/0568) against fixtures/network/v1/timed-closures.json
/// (`scripts/fixtures/network_timed_closures_cases.py`: an earliest-arrival Dijkstra stepping through the speed's
/// changes and the windows, no KentOS code): what each access closed and could not read, whether the hours went
/// unasked, the cheapest way's cost, length and minutes.
#[test]
fn weekdays_and_hours_give_what_the_independent_reference_gives() {
    let path = format!(
        "{}/../../../fixtures/network/v1/timed-closures.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let file: Value =
        serde_json::from_str(&std::fs::read_to_string(path).expect("the fixture")).expect("JSON");
    assert_eq!(file["format"], "kentos.network-timed-closures");
    let mut s = session(&file);
    let cases = file["cases"].as_array().expect("cases");
    assert_eq!(cases.len(), 22);
    for round in 0..2 {
        for c in cases {
            let name = c["name"].as_str().expect("a name");
            s.set_access(None, c["day"].as_str())
                .unwrap_or_else(|e| panic!("{name}: {e}"));
            let varied = s.set_time(c["time"].as_f64());
            assert_eq!(
                varied.by_date as u64,
                c["byDate"].as_u64().unwrap(),
                "{name}"
            );
            assert_eq!(
                varied.by_hours as u64,
                c["byHours"].as_u64().unwrap(),
                "{name}"
            );
            assert_eq!(
                varied.hours_unasked,
                c["hoursUnasked"].as_bool().unwrap(),
                "{name}"
            );
            let unread: Vec<f64> = c["unread"].as_array().unwrap().iter().map(num).collect();
            assert_eq!(varied.unread, unread, "{name}");
            let stops = [point(&c["from"]), point(&c["to"])];
            let cost = c["cost"].as_u64().unwrap() as usize;
            let got = s.route(&stops, &[], 1.0, cost, Reorder::None);
            if c["unreachable"].as_bool() == Some(true) {
                assert!(matches!(got, Err(Refusal::Unreachable(0, 1, _))), "{name}");
                continue;
            }
            let r = got.unwrap_or_else(|e| panic!("{name} ({round}): {e:?}"));
            near(r.cost, num(&c["total"]), name);
            near(r.totals[0].expect("a length"), num(&c["totals"][0]), name);
            near(r.totals[1].expect("minutes"), num(&c["totals"][1]), name);
        }
    }
}

/// Why two stops have no way (docs/adr/0570) against fixtures/network/v1/blocked.json
/// (`scripts/fixtures/network_blocked_cases.py`: a plain Dijkstra by fewest blockers then length (docs/adr/0575), each edge's reason
/// from the ADR's rules, no KentOS code): whether a way exists, and if not the refusal's `blocked` exactly.
#[test]
fn an_unreachable_stop_says_what_shuts_the_way_as_the_independent_reference() {
    let path = format!(
        "{}/../../../fixtures/network/v1/blocked.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let file: Value =
        serde_json::from_str(&std::fs::read_to_string(path).expect("the fixture")).expect("JSON");
    assert_eq!(file["format"], "kentos.network-blocked");
    let mut s = session(&file);
    let cases = file["cases"].as_array().expect("cases");
    assert_eq!(cases.len(), 20);
    for c in cases {
        let name = c["name"].as_str().expect("a name");
        s.set_access(c["profile"].as_str(), c["day"].as_str())
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        s.set_time(c["time"].as_f64());
        let stops = [point(&c["from"]), point(&c["to"])];
        let barriers: Vec<Vec2> = c["barriers"]
            .as_array()
            .unwrap()
            .iter()
            .map(point)
            .collect();
        let cost = c["cost"].as_u64().unwrap() as usize;
        let got = s.route(&stops, &barriers, 1.0, cost, Reorder::None);
        if c["reachable"].as_bool() == Some(true) {
            assert!(got.is_ok(), "{name}: {got:?}");
            continue;
        }
        let Err(Refusal::Unreachable(0, 1, why)) = got else {
            panic!("{name}: {got:?}")
        };
        let json: Value = serde_json::from_str(&why.json()).expect("JSON");
        assert_eq!(json, c["blocked"], "{name}: {}", why.words());
        if barriers.is_empty() {
            assert_eq!(
                s.blocked_between(stops[0], stops[1], 1.0, cost),
                Some(why),
                "{name}"
            );
        }
    }
}

/// What cuts a service area off and the edges on parts no facility meets (docs/adr/0640 §2) against the `areas` of
/// fixtures/network/v1/blocked.json (`scripts/fixtures/network_blocked_cases.py`: one search to the last break, each
/// unreached edge's reason from the reached end, the levels beside reached places; no KentOS code), exactly.
#[test]
fn a_service_area_says_what_cuts_it_off_as_the_independent_reference() {
    let path = format!(
        "{}/../../../fixtures/network/v1/blocked.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let file: Value =
        serde_json::from_str(&std::fs::read_to_string(path).expect("the fixture")).expect("JSON");
    let mut s = session(&file);
    let areas = file["areas"].as_array().expect("areas");
    assert_eq!(areas.len(), 11);
    for c in areas {
        let name = c["name"].as_str().expect("a name");
        s.set_access(c["profile"].as_str(), c["day"].as_str())
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        s.set_time(c["time"].as_f64());
        let breaks: Vec<f64> = c["breaks"].as_array().unwrap().iter().map(num).collect();
        let barriers: Vec<Vec2> = c["barriers"]
            .as_array()
            .unwrap()
            .iter()
            .map(point)
            .collect();
        let cost = c["cost"].as_u64().unwrap() as usize;
        let served = s
            .service_area(
                &[point(&c["facility"])],
                &breaks,
                1.0,
                cost,
                false,
                false,
                &barriers,
                0.0,
                false,
                false,
            )
            .unwrap_or_else(|e| panic!("{name}: {e:?}"));
        let json: Value =
            serde_json::from_str(&kentos_geometry_core::api::json::to_string(&served.cut))
                .expect("JSON");
        assert_eq!(json, c["cut"], "{name}");
    }
}
