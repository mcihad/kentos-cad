//! Robust estimation of a GNSS network through a real AI host (plan/map-cad.md MAP-05, docs/adr/4960): an 8 × 8 grid
//! of stations 1 km apart with a 9 cm blunder on one baseline, given to `cad.survey.baselines` over MCP with Huber's
//! weight function: the rounds settle, the blunder's factor is the only one far below 1 and the smallest, no baseline
//! is removed; a constant c of 0 is refused at its path.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"gnss-robust-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

const W: usize = 8;
const BLUNDER: usize = 40;

fn network() -> (Vec<Value>, Vec<Value>) {
    let base = [4_121_000.0, 2_652_000.0, 4_069_000.0];
    let (east, north, up) = ([-0.54, 0.84, 0.0], [-0.53, -0.34, 0.78], [0.65, 0.42, 0.63]);
    let at = |r: usize, c: usize| [0, 1, 2].map(|k| base[k] + 1000.0 * (c as f64 * east[k] + r as f64 * north[k]));
    let mut stations = vec![json!({"name": "S0", "xyz": at(0, 0), "fixed": true})];
    stations.extend((1..W * W).map(|i| json!({"name": format!("S{i}")})));
    let mut baselines = Vec::new();
    for r in 0..W {
        for c in 0..W {
            for (dr, dc) in [(0, 1), (1, 0), (1, 1)] {
                if r + dr < W && c + dc < W {
                    let (a, b) = (at(r, c), at(r + dr, c + dc));
                    let k = baselines.len();
                    let blunder = if k == BLUNDER { 0.09 } else { 0.0 };
                    let kf = k as f64;
                    let noise = [0.003 * (kf * 0.7).sin(), 0.003 * (kf * 1.3).cos(), 0.006 * (kf * 0.9).sin()];
                    let d: [f64; 3] = [0, 1, 2].map(|q| b[q] - a[q] + noise[q] + blunder * up[q]);
                    baselines.push(json!({
                        "from": format!("S{}", r * W + c),
                        "to": format!("S{}", (r + dr) * W + c + dc),
                        "d": d,
                        "cov": [9e-6, 2.7e-6, 0.0, 9e-6, 0.0, 36e-6],
                    }));
                }
            }
        }
    }
    (stations, baselines)
}

#[test]
fn robust_estimation_over_mcp() {
    let mut server = Server::new();
    let d = call(&mut server, "drawing.new", json!({"workspace": "cad"}));
    let drawing = d["structuredContent"]["drawing"].clone();
    let (stations, baselines) = network();
    let r = call(
        &mut server,
        "cad.survey.baselines",
        json!({"drawing": drawing, "stations": stations, "baselines": baselines, "robust": {"method": "huber"}}),
    );
    assert_eq!(r["isError"], false, "{}", r.to_string().chars().take(400).collect::<String>());
    let out = &r["structuredContent"]["output"];
    assert_eq!(out["robust"]["converged"], true);
    assert_eq!(out["robust"]["c"], 1.5);
    let rows = out["baselines"].as_array().expect("baselines");
    assert_eq!(rows.len(), baselines.len(), "nothing removed");
    let factors: Vec<f64> = rows.iter().map(|b| b["factor"].as_f64().expect("a factor")).collect();
    let least = (0..factors.len()).min_by(|&a, &b| factors[a].total_cmp(&factors[b])).expect("a baseline");
    assert_eq!(least, BLUNDER);
    assert!(factors[BLUNDER] < 0.5, "{}", factors[BLUNDER]);
    // c 0 is refused at its path.
    let r = call(
        &mut server,
        "cad.survey.baselines",
        json!({"drawing": drawing, "stations": network().0, "baselines": network().1, "robust": {"method": "huber", "c": 0.0}}),
    );
    assert_eq!(r["structuredContent"]["error"]["code"], "invalid_input", "{}", r.to_string().chars().take(400).collect::<String>());
    assert_eq!(r["structuredContent"]["error"]["path"], "robust.c");
}
