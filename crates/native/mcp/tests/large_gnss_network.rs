//! A GNSS network above the old dense limit through a real AI host (plan/map-cad.md MAP-05, docs/adr/4560): a 26 × 26
//! grid of stations 1 km apart (676 stations, 2 025 unknowns with the first fixed; the dense solve took at most 1 500)
//! given to `cad.survey.baselines` over MCP is adjusted (n, u, f; the χ² test passes); a station named twice is still
//! refused at its path.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"large-gnss-network-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

const W: usize = 26;

fn network() -> (Vec<Value>, Vec<Value>) {
    let base = [4_121_000.0, 2_652_000.0, 4_069_000.0];
    let (east, north) = ([-0.54, 0.84, 0.0], [-0.53, -0.34, 0.78]);
    let at = |r: usize, c: usize| [0, 1, 2].map(|k| base[k] + 1000.0 * (c as f64 * east[k] + r as f64 * north[k]));
    let mut stations = Vec::new();
    for r in 0..W {
        for c in 0..W {
            let i = r * W + c;
            stations.push(match i {
                0 => json!({"name": format!("S{i}"), "xyz": at(r, c), "fixed": true}),
                _ if i % 13 == 0 => json!({"name": format!("S{i}"), "xyz": at(r, c), "sigma": [0.02, 0.02, 0.02]}),
                _ => json!({"name": format!("S{i}")}),
            });
        }
    }
    let mut baselines = Vec::new();
    for r in 0..W {
        for c in 0..W {
            for (dr, dc) in [(0, 1), (1, 0), (1, 1)] {
                if r + dr < W && c + dc < W {
                    let (a, b) = (at(r, c), at(r + dr, c + dc));
                    let k = baselines.len() as f64;
                    let noise = [0.003 * (k * 0.7).sin(), 0.003 * (k * 1.3).cos(), 0.006 * (k * 0.9).sin()];
                    let d: [f64; 3] = [0, 1, 2].map(|q| b[q] - a[q] + noise[q]);
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
fn a_network_above_the_old_limit_over_mcp() {
    let mut server = Server::new();
    let d = call(&mut server, "drawing.new", json!({"workspace": "cad"}));
    let drawing = d["structuredContent"]["drawing"].clone();
    let (stations, baselines) = network();
    let r = call(&mut server, "cad.survey.baselines", json!({"drawing": drawing, "stations": stations, "baselines": baselines}));
    assert_eq!(r["isError"], false, "{}", r.to_string().chars().take(400).collect::<String>());
    let out = &r["structuredContent"]["output"];
    let (n, u, f) = (out["n"].as_u64().unwrap(), out["u"].as_u64().unwrap(), out["f"].as_u64().unwrap());
    assert_eq!(u, 3 * (W * W - 1) as u64);
    assert!(u > 1500, "above the old dense limit");
    assert_eq!(n - u, f);
    assert_eq!(out["passed"], true);
    // A station named twice is refused at its path, as before.
    let mut twice = network().0;
    twice.push(json!({"name": "S5"}));
    let r = call(&mut server, "cad.survey.baselines", json!({"drawing": drawing, "stations": twice, "baselines": network().1}));
    assert_eq!(r["structuredContent"]["error"]["code"], "duplicate_station", "{}", r.to_string().chars().take(400).collect::<String>());
}
