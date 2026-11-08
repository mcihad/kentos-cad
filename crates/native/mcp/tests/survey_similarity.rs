//! BÖHHBÜY Madde 15(1)(c)'s similarity through a real AI host (plan/map-cad.md MAP-05, docs/adr/2160): the shared case's
//! network with three known points opened as a .kcad file over MCP gives the case's similarity (scale, its σ, the
//! test, the translation, rotations and residuals) and C1's rule outcome.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"survey-similarity-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

const CASE: &str = "Üç bilinen nokta: 15(1)(c) benzerlik dönüşümü; ölçek C1'in ±3 ppm'i içinde, istatistikle uyumlu";

fn near(got: &Value, want: &Value, at: &str, within: f64) {
    match (got.as_f64(), want.as_f64()) {
        (Some(a), Some(b)) => assert!((a - b).abs() < within, "{at}: {a} != {b}"),
        _ => assert_eq!(got, want, "{at}"),
    }
}

#[test]
fn the_similarity_over_mcp_is_the_case_s() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/map.survey.network.json")).unwrap();
    let case = cases["cases"].as_array().unwrap().iter().find(|c| c["name"] == CASE).unwrap().clone();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(case["setup"].clone()).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-survey-similarity-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let mut args = case["steps"][0]["input"].clone();
    args["drawing"] = opened["structuredContent"]["drawing"].clone();
    let r = call(&mut server, "map.survey.network", args);
    assert_eq!(r["isError"], false, "{r}");
    let (got, want) = (&r["structuredContent"]["output"], &case["steps"][0]["result"]["output"]);
    let (g, w) = (&got["similarity"], &want["similarity"]);
    assert_eq!((&g["known"], &g["dof"], &g["agrees"]), (&w["known"], &w["dof"], &w["agrees"]));
    // The case's own tolerance (1e-5: t divides 10⁻⁷ by 10⁻⁸).
    for key in ["scale", "sigma", "t", "quantile"] {
        near(&g[key], &w[key], key, 1e-5);
    }
    for k in 0..3 {
        near(&g["translation"][k], &w["translation"][k], "translation", 1e-5);
        near(&g["rotation"][k], &w["rotation"][k], "rotation", 1e-5);
    }
    for (a, b) in g["residuals"].as_array().unwrap().iter().zip(w["residuals"].as_array().unwrap()) {
        for k in 0..3 {
            near(&a["enu"][k], &b["enu"][k], "residual", 1e-5);
        }
    }
    assert_eq!(got["rules"]["similarity"], json!(true));
    let _ = std::fs::remove_file(path);
}
