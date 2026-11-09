//! İş grafiği through a real AI host (plan/map-cad.md MAP-20, docs/adr/1940): the same small production job as the
//! shared cases of `road.project.graph` (which the desktop's handlers and the Python SDK pass) made over MCP — the zero
//! line of Tümsek over Düz drawn, one of Tümsek's heights set, the graph read, the drawing drawn again, the graph read
//! again — gives the cases' graphs: the same nodes, recipes, sources' stamps, sheets and counts.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"project-graph-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

/// `want` with its `$any` and `$current` places taken from `got`, and a revision a step captured (`$r1`, docs/adr/2840)
/// where `got` has a revision's text; the rest as it is.
fn filled(want: &Value, got: &Value) -> Value {
    match (want, got) {
        (Value::String(s), _) if s == "$any" || s == "$current" => got.clone(),
        (Value::String(s), Value::String(g))
            if s.strip_prefix("$r").is_some_and(|n| n.parse::<u32>().is_ok()) && g.parse::<u64>().is_ok() =>
        {
            got.clone()
        }
        (Value::Object(w), Value::Object(g)) => {
            Value::Object(w.iter().map(|(k, v)| (k.clone(), g.get(k).map_or_else(|| v.clone(), |x| filled(v, x)))).collect())
        }
        (Value::Array(w), Value::Array(g)) if w.len() == g.len() => {
            Value::Array(w.iter().zip(g).map(|(a, b)| filled(a, b)).collect())
        }
        _ => want.clone(),
    }
}

/// The graph the case named `name` expects at its last step.
fn expected(cases: &Value, name: &str) -> Value {
    let case = cases["cases"].as_array().unwrap().iter().find(|c| c["name"] == name).unwrap();
    case["steps"].as_array().unwrap().last().unwrap()["result"]["output"].clone()
}

#[test]
fn the_same_job_over_mcp_gives_the_cases_graphs() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/road.project.graph.json")).unwrap();
    // The setup as a .kcad file, as a host opens a project.
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-project-graph-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    let mut run = |name: &str, args: Value| {
        let mut args = args;
        args["drawing"] = drawing.clone();
        let r = call(&mut server, name, args);
        assert_eq!(r["isError"], false, "{name}: {r}");
        r["structuredContent"]["output"].clone()
    };
    run("road.terrains.zeroline", json!({"base": "Düz", "comparison": "Tümsek", "layerId": "sifir"}));
    let surface = json!({"id": "t-10", "name": "Tümsek", "points": {"layers": ["tumsek"]},
                         "edits": [{"op": "setHeight", "x": 6000.0, "y": 0.0, "z": 119.0}]});
    run("road.terrains.edit", json!({"operation": "set", "surface": surface}));
    let stale = run("road.project.graph", json!({}));
    let want = expected(&cases, "Tümsek'in bir kotu değişince çizim güncel değil, onu gösteren pafta teslim edilemez");
    assert_eq!(stale, filled(&want, &stale));
    assert_eq!((stale["outdated"].as_u64(), stale["blocked"].as_u64()), (Some(1), Some(1)));
    run("road.project.rebuild", json!({}));
    let current = run("road.project.graph", json!({}));
    let want = expected(&cases, "Yeniden üretilince zincir yine güncel, kaynakların damgaları yeni");
    assert_eq!(current, filled(&want, &current));
    assert_eq!((current["outdated"].as_u64(), current["blocked"].as_u64()), (Some(0), Some(0)));
    let _ = std::fs::remove_file(path);
}
