//! The outside pieces' numbers kept clear, and the distribution by value, through a real AI host (plan/map-cad.md
//! MAP-11, docs/adr/5080): the shared cases' drawings opened as .kcad files over MCP. `cad.readjustment.sheets` with a
//! road's edge all round 101/10 answers the case's labels (each number where it touches the fewest, `clear` false, the
//! road in `clashes`) and says `number_crowded`; `cad.readjustment.distribute` by value answers the case's nets, the
//! values' shares and `value_basis_not_a_rule`, and without the values' source is refused at its path.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"separation-number-clear-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

fn case(cases: &Value, starts: &str) -> Value {
    cases["cases"].as_array().unwrap().iter().find(|c| c["name"].as_str().is_some_and(|n| n.starts_with(starts))).unwrap().clone()
}

fn opened(server: &mut Server, setup: &Value, tag: &str) -> Value {
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(setup.clone()).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-separation-clear-{tag}-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let opened = call(server, "drawing.open", json!({"path": path}));
    let _ = std::fs::remove_file(&path);
    assert_eq!(opened["isError"], false, "{opened}");
    opened["structuredContent"]["drawing"].clone()
}

#[test]
fn the_crowded_numbers_over_mcp_are_the_case_s() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.readjustment.sheets.json")).unwrap();
    let case = case(&cases, "kılavuzlu numara: her yer bir çizgiye değiyorsa");
    let mut server = Server::new();
    let drawing = opened(&mut server, &case["setup"], "sheets");
    let step = &case["steps"][0];
    let mut args = step["input"].clone();
    args["drawing"] = drawing;
    let r = call(&mut server, "cad.readjustment.sheets", args);
    assert_eq!(r["isError"], false, "{r}");
    let got = &r["structuredContent"]["output"]["sheets"][0]["labels"];
    let want = &step["result"]["output"]["sheets"][0]["labels"];
    for (g, w) in got.as_array().unwrap().iter().zip(want.as_array().unwrap()) {
        assert_eq!((&g["clear"], &g["clashes"]), (&w["clear"], &w["clashes"]), "{g}");
        assert_eq!(g["clear"], false);
        for k in 0..2 {
            assert!((g["at"][k].as_f64().unwrap() - w["at"][k].as_f64().unwrap()).abs() < 1e-6, "{g}");
        }
    }
    let warned: Vec<&str> = r["structuredContent"]["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|w| w["code"].as_str())
        .collect();
    assert_eq!(warned, ["number_crowded", "number_crowded"], "{r}");
}

#[test]
fn the_distribution_by_value_over_mcp_is_the_case_s() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.readjustment.distribute.json")).unwrap();
    let case = case(&cases, "değer esaslı: dağıtım kütlesi parsellere değerleri oranında");
    let mut server = Server::new();
    let drawing = opened(&mut server, &cases["setup"], "distribute");
    let step = &case["steps"][0];
    let mut args = step["input"].clone();
    args["drawing"] = drawing.clone();
    let r = call(&mut server, "cad.readjustment.distribute", args);
    assert_eq!(r["isError"], false, "{r}");
    let (got, want) = (&r["structuredContent"]["output"], &step["result"]["output"]);
    assert_eq!(got["basis"], "value");
    for (g, w) in got["parcels"].as_array().unwrap().iter().zip(want["parcels"].as_array().unwrap()) {
        for key in ["net", "deduction", "value", "valueShare"] {
            assert_eq!(g[key], w[key], "{key}: {g}");
        }
    }
    assert!(r.to_string().contains("value_basis_not_a_rule"), "{r}");
    let mut args = step["input"].clone();
    args["drawing"] = drawing;
    args["rules"]["valueSource"] = json!(" ");
    let r = call(&mut server, "cad.readjustment.distribute", args);
    assert_eq!(r["structuredContent"]["error"]["path"], "rules.valueSource", "{r}");
}
