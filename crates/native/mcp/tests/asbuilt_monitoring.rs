//! A monitoring job through a real AI host (plan/map-cad.md MAP-19, docs/adr/3620): the shared cases' drawing of
//! `cad.asbuilt.check` opened as a .kcad file over MCP. The second epoch checked against the first gives the shared
//! case's movements, statuses and control points (K2 moved, M5 not measured, M6 new); the job's kind is said back; a
//! surface in a monitoring job is refused at `surface`.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"asbuilt-monitoring-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

#[test]
fn a_monitoring_job_over_mcp() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.asbuilt.check.json")).unwrap();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).unwrap();
    let dir = std::env::temp_dir().join(format!("kentos-asbuilt-monitoring-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("izleme.kcad");
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();

    let case = cases["cases"].as_array().unwrap().iter().find(|c| c["name"].as_str().is_some_and(|n| n.starts_with("izleme: iki dönem adla"))).unwrap();
    let want = &case["steps"][0]["result"]["output"];
    let mut args = case["steps"][0]["input"].clone();
    args["drawing"] = drawing.clone();
    let r = call(&mut server, "cad.asbuilt.check", args.clone());
    assert_eq!(r["isError"], false, "{r}");
    let got = &r["structuredContent"]["output"];
    assert_eq!(got["kind"], "monitoring");
    assert_eq!(got["control"], json!({"checked": ["K1", "K2"], "moved": ["K2"], "missing": []}));
    assert_eq!(got["control"], want["control"]);
    for key in ["within", "outside", "unmatched", "missing", "lowQuality"] {
        assert_eq!(got[key], want[key], "{key}");
    }
    for (g, w) in got["points"].as_array().unwrap().iter().zip(want["points"].as_array().unwrap()) {
        assert_eq!((&g["name"], &g["status"], &g["flags"], &g["by"]), (&w["name"], &w["status"], &w["flags"], &w["by"]));
        for key in ["de", "dn", "dh", "horizontal"] {
            match (g[key].as_f64(), w[key].as_f64()) {
                (Some(a), Some(b)) => assert!((a - b).abs() < 1e-9, "{} {key}: {a} {b}", w["name"]),
                (a, b) => assert_eq!(a, b, "{} {key}", w["name"]),
            }
        }
    }

    // A design check's input in a monitoring job.
    args["surface"] = json!("Tasarım");
    let r = call(&mut server, "cad.asbuilt.check", args);
    assert_eq!(r["isError"], true, "{r}");
    assert!(r.to_string().contains("invalid_asbuilt_check") && r.to_string().contains("tasarım denetimlerinindir"), "{r}");
    let _ = std::fs::remove_dir_all(dir);
}
