//! Dış çap and the rule package through a real AI host (plan/map-cad.md MAP-16, docs/adr/4780): the shared cases'
//! outers drawing opened as a .kcad file over MCP. `cad.pipes.check` with `diameter.outer` and `diameter.wall` says the
//! pipes whose two do not agree; with `rules` (KentOS's sewer system) each pipe's class and findings are the shared
//! case's; a system the package does not have is refused at `rules.system`; `cad.pipes.profile` draws the run's own
//! outer walls, as many objects as the case.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"pipe-rules-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

fn open(server: &mut Server, setup: &Value, name: &str) -> (Value, std::path::PathBuf) {
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(setup.clone()).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-pipe-rules-{name}-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let opened = call(server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    (opened["structuredContent"]["drawing"].clone(), path)
}

#[test]
fn the_outer_diameters_and_the_rules_over_mcp_are_the_cases() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.pipes.check.json")).unwrap();
    let case = |name: &str| cases["cases"].as_array().unwrap().iter().find(|c| c["name"] == name).unwrap().clone();
    let mut server = Server::new();
    let both = case("dış çap ve et birlikte: uyuşmayan söylenir, etsiz ölçülür");
    let (drawing, path) = open(&mut server, &both["setup"], "outers");
    let run = |server: &mut Server, c: &Value, drawing: &Value| {
        let mut args = c["steps"][0]["input"].clone();
        args["drawing"] = drawing.clone();
        call(server, "cad.pipes.check", args)
    };
    let r = run(&mut server, &both, &drawing);
    assert_eq!(r["isError"], false, "{r}");
    let want = &both["steps"][0]["result"]["output"];
    let got = &r["structuredContent"]["output"];
    assert_eq!(got["coverTo"], "outside");
    for (g, w) in got["pipes"].as_array().unwrap().iter().zip(want["pipes"].as_array().unwrap()) {
        assert_eq!(g["findings"], w["findings"], "{}", g["uid"]);
        match (g["wall"].as_f64(), w["wall"].as_f64()) {
            (Some(a), Some(b)) => assert!((a - b).abs() < 1e-9, "wall: {a} vs {b}"),
            (a, b) => assert_eq!(a, b, "wall of {}", g["uid"]),
        }
    }
    let mismatched = got["findings"].as_array().unwrap().iter().filter(|f| f["kind"] == "wallMismatch").count();
    assert_eq!(mismatched, 2, "P4 and P8");

    let rules = case("kural: atıksu yönetmeliğinin EK-1'i; eğim çap sınıfından, en az örtü 2,70 m");
    let r = run(&mut server, &rules, &drawing);
    assert_eq!(r["isError"], false, "{r}");
    let want = &rules["steps"][0]["result"]["output"];
    let got = &r["structuredContent"]["output"];
    for (g, w) in got["pipes"].as_array().unwrap().iter().zip(want["pipes"].as_array().unwrap()) {
        assert_eq!((&g["class"], &g["findings"]), (&w["class"], &w["findings"]), "{}", g["uid"]);
    }
    assert_eq!(got["counts"], want["counts"]);

    let refused = case("kural: pakette olmayan sistem");
    let r = run(&mut server, &refused, &drawing);
    assert_eq!(r["isError"], true, "{r}");
    assert_eq!(r["structuredContent"]["error"]["code"], "invalid_pipe_check", "{r}");
    assert_eq!(r["structuredContent"]["error"]["path"], "rules.system", "{r}");
    let _ = std::fs::remove_file(path);

    // The run's own outer walls in the profile: as many objects as the shared case.
    let profiles: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.pipes.profile.json")).unwrap();
    let own = profiles["cases"].as_array().unwrap().iter().find(|c| c["name"] == "Hattın kendi borularının dış duvarı: et kalınlığıyla").unwrap();
    let (drawing, path) = open(&mut server, &own["setup"], "profile");
    let mut args = own["steps"][0]["input"].clone();
    args["drawing"] = drawing;
    let r = call(&mut server, "cad.pipes.profile", args);
    assert_eq!(r["isError"], false, "{r}");
    let want = &own["steps"][0]["result"]["output"];
    let got = &r["structuredContent"]["output"];
    assert_eq!(got["uids"].as_array().unwrap().len(), want["uids"].as_array().unwrap().len());
    let walls: Vec<f64> = got["pipes"].as_array().unwrap().iter().filter_map(|p| p["wall"].as_f64()).collect();
    assert_eq!(walls.len(), 6, "{walls:?}");
    assert!((got["datum"].as_f64().unwrap() - want["datum"].as_f64().unwrap()).abs() < 1e-9);
    let _ = std::fs::remove_file(path);
}
