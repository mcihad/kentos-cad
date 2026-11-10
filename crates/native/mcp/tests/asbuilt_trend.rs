//! İzlemede dönemlerden doğru ve iki eşik through a real AI host (plan/map-cad.md MAP-19, docs/adr/4840): the series
//! cases' drawings opened as .kcad files over MCP. `cad.asbuilt.check` with a trend answers each mark's line (its
//! velocities, their σ, the components that move) as the shared case does; with four epochs and both levels each mark's
//! state and the limits it goes beyond are the case's; a warning not below its alarm is refused with the case's words.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"asbuilt-trend-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

/// Every number in `got` within the case's `within` of `want`'s, the rest the same. The case's reference moves the
/// marks by their small local numbers and the drawing holds them on large coordinates: a line through three nearly
/// straight epochs has a σ of some 5e-5 m a year, which those coordinates' last bits move by 1e-9.
fn same(got: &Value, want: &Value, at: &str, within: f64) {
    match (got, want) {
        (Value::Number(a), Value::Number(b)) => assert!((a.as_f64().unwrap() - b.as_f64().unwrap()).abs() <= within, "{at}: {a} ≠ {b}"),
        (Value::Object(a), Value::Object(b)) => {
            assert_eq!(a.keys().collect::<Vec<_>>(), b.keys().collect::<Vec<_>>(), "{at}");
            for (k, v) in b {
                same(&a[k], v, &format!("{at}.{k}"), within);
            }
        }
        (Value::Array(a), Value::Array(b)) => {
            assert_eq!(a.len(), b.len(), "{at}");
            for (i, (x, y)) in a.iter().zip(b).enumerate() {
                same(x, y, &format!("{at}[{i}]"), within);
            }
        }
        _ => assert_eq!(got, want, "{at}"),
    }
}

/// The drawing written as a .kcad file and opened over MCP; its id.
fn open(server: &mut Server, setup: &Value, tag: &str) -> (Value, std::path::PathBuf) {
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(setup.clone()).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-asbuilt-trend-{tag}-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let opened = call(server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    (opened["structuredContent"]["drawing"].clone(), path)
}

#[test]
fn the_line_and_the_two_levels_over_mcp_are_the_case_s() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.asbuilt.check.series.json")).unwrap();
    let case = |prefix: &str| cases["cases"].as_array().unwrap().iter().find(|c| c["name"].as_str().unwrap().starts_with(prefix)).unwrap().clone();
    let mut server = Server::new();
    let (three, three_path) = open(&mut server, &cases["setup"], "three");
    let four_case = case("doğru ve eşikler birlikte, dört dönem");
    let (four, four_path) = open(&mut server, &four_case["setup"], "four");
    for (name, drawing) in [("doğru: üç günlü dönem", &three), ("eşikler: yatay ve düşey", &three), ("doğru ve eşikler birlikte", &four)] {
        let step = case(name)["steps"][0].clone();
        let mut args = step["input"].clone();
        args["drawing"] = drawing.clone();
        let r = call(&mut server, "cad.asbuilt.check", args);
        assert_eq!(r["isError"], false, "{name}: {r}");
        let within = step["within"].as_f64().expect("the case's tolerance");
        same(&r["structuredContent"]["output"]["tracks"], &step["result"]["output"]["tracks"], name, within);
    }
    let tracks = case("doğru ve eşikler birlikte")["steps"][0]["result"]["output"]["tracks"].clone();
    let states: Vec<&str> = tracks.as_array().unwrap().iter().filter_map(|t| t["state"].as_str()).collect();
    assert!(states.contains(&"alarm") && states.contains(&"normal"), "{states:?}");
    assert!(tracks.as_array().unwrap().iter().any(|t| t["fit"]["moving"].as_array().is_some_and(|m| !m.is_empty())), "{tracks}");
    let step = case("uyarı alarmdan büyük")["steps"][0].clone();
    let mut args = step["input"].clone();
    args["drawing"] = three.clone();
    let said = call(&mut server, "cad.asbuilt.check", args).to_string();
    assert!(said.contains(step["result"]["error"]["message"].as_str().unwrap()), "{said}");
    let _ = std::fs::remove_file(three_path);
    let _ = std::fs::remove_file(four_path);
}
