//! Referanstan kot ver through a real AI host (plan/map-cad.md MAP-06, docs/adr/3780): the tool is listed as one that
//! writes over what is there; the shared cases' drawing opened as a .kcad file over MCP; the plan of the two-part area
//! by the typed reference gives the case's answer and writes nothing; execute moves the area whole, and the saved file
//! holds the outer ring's, the hole's and the other part's elevations as the case leaves them.
//! Docs/adr/4980: the first part's hole's vertex as the one the reference gives, every elevation moved as the case's.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn meta() -> Value {
    json!({"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"raise-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}})
}

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":meta()}})).unwrap()["result"].clone()
}

fn close(a: &Value, b: &Value, within: f64) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => (x.as_f64().unwrap() - y.as_f64().unwrap()).abs() <= within,
        (Value::Array(x), Value::Array(y)) => x.len() == y.len() && x.iter().zip(y).all(|(p, q)| close(p, q, within)),
        (Value::Object(x), Value::Object(y)) => x.len() == y.len() && x.iter().all(|(k, v)| y.get(k).is_some_and(|w| close(v, w, within))),
        _ => a == b,
    }
}

#[test]
fn an_area_raised_by_a_reference_over_mcp_is_the_case_s() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.entities.raise.json")).unwrap();
    let case = cases["cases"].as_array().unwrap().iter().find(|c| c["name"].as_str().is_some_and(|n| n.starts_with("Alan: dış halka"))).unwrap().clone();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-raise-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    // Listed as one that writes over what is there.
    let listed = server
        .handle(&json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{"_meta":meta()}}))
        .unwrap();
    let tool = listed["result"]["tools"].as_array().unwrap().iter().find(|t| t["name"] == "cad.entities.raise").expect("listed").clone();
    assert_eq!((&tool["annotations"]["destructiveHint"], &tool["annotations"]["readOnlyHint"]), (&json!(true), &json!(false)), "{tool}");
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    let step = &case["steps"][0];
    let want = &step["result"]["output"];
    // Plan: the case's answer, nothing written.
    let mut args = step["input"].clone();
    args["drawing"] = drawing.clone();
    args["op"] = json!("plan");
    let r = call(&mut server, "cad.entities.raise", args);
    assert_eq!(r["isError"], false, "{r}");
    let got = &r["structuredContent"]["output"];
    for key in ["distance", "before", "after", "by", "grade", "moved", "unset"] {
        assert!(close(&got[key], &want[key], 1e-9), "{key}: {} != {}", got[key], want[key]);
    }
    // Execute, then the saved file: the area as the case leaves it.
    let mut args = step["input"].clone();
    args["drawing"] = drawing.clone();
    let r = call(&mut server, "cad.entities.raise", args);
    assert_eq!(r["isError"], false, "{r}");
    let saved = std::env::temp_dir().join(format!("kentos-raise-saved-{}.kcad", std::process::id()));
    let s = call(&mut server, "drawing.save", json!({"drawing": drawing, "path": saved}));
    assert_eq!(s["isError"], false, "{s}");
    let back = serde_json::to_value(kentos_kcad::decode(&std::fs::read(&saved).unwrap()).unwrap()).unwrap();
    let area = back["entities"].as_array().unwrap().iter().find(|e| e["id"] == json!(3)).unwrap();
    let fields = &step["expect"]["fields"]["3"];
    for key in ["zs", "holes", "parts"] {
        assert!(close(&area[key], &fields[key], 1e-9), "{key}: {} != {}", area[key], fields[key]);
    }
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(saved);
}

#[test]
fn a_hole_s_vertex_raised_by_a_reference_over_mcp_is_the_case_s() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.entities.raise.json")).unwrap();
    let case = cases["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"].as_str().is_some_and(|n| n.starts_with("Alan: ilk parçanın deliğinin")))
        .unwrap()
        .clone();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-raise-hole-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    let _ = std::fs::remove_file(&path);
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    let step = &case["steps"][0];
    let mut args = step["input"].clone();
    args["drawing"] = drawing.clone();
    let r = call(&mut server, "cad.entities.raise", args);
    assert_eq!(r["isError"], false, "{r}");
    let want = &step["result"]["output"];
    for k in ["distance", "before", "after", "by", "moved", "unset"] {
        assert!(close(&r["structuredContent"]["output"][k], &want[k], 1e-9), "{k}: {r}");
    }
    let uid = step["input"]["uid"].as_str().unwrap();
    let e = call(&mut server, "drawing.entity", json!({"drawing": drawing, "uid": uid}));
    let fields = &step["expect"]["fields"]["3"];
    for k in ["zs", "holes", "parts"] {
        assert!(close(&e["structuredContent"]["entity"][k], &fields[k], 1e-9), "{k}: {e}");
    }
}
