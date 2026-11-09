//! İzlemenin dönem dizisi through a real AI host (plan/map-cad.md MAP-19, docs/adr/4420): the series cases' drawing
//! opened as a .kcad file over MCP. `cad.asbuilt.check` with the first two epochs as the series answers each mark's
//! track (its reference, its steps, its rates) as the shared case does; a day that is no day is refused with the case's
//! words.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"asbuilt-series-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

/// Every number in `got` within 1e-9 of `want`'s, the rest the same.
fn same(got: &Value, want: &Value, at: &str) {
    match (got, want) {
        (Value::Number(a), Value::Number(b)) => assert!((a.as_f64().unwrap() - b.as_f64().unwrap()).abs() < 1e-9, "{at}: {a} ≠ {b}"),
        (Value::Object(a), Value::Object(b)) => {
            assert_eq!(a.keys().collect::<Vec<_>>(), b.keys().collect::<Vec<_>>(), "{at}");
            for (k, v) in b {
                same(&a[k], v, &format!("{at}.{k}"));
            }
        }
        (Value::Array(a), Value::Array(b)) => {
            assert_eq!(a.len(), b.len(), "{at}");
            for (i, (x, y)) in a.iter().zip(b).enumerate() {
                same(x, y, &format!("{at}[{i}]"));
            }
        }
        _ => assert_eq!(got, want, "{at}"),
    }
}

#[test]
fn the_series_over_mcp_is_the_case_s() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.asbuilt.check.series.json")).unwrap();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-asbuilt-series-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    let case = |prefix: &str| cases["cases"].as_array().unwrap().iter().find(|c| c["name"].as_str().unwrap().starts_with(prefix)).unwrap()["steps"][0].clone();
    for name in ["dizi: üç dönem", "dizi: birinci dönemin günü yok"] {
        let step = case(name);
        let mut args = step["input"].clone();
        args["drawing"] = drawing.clone();
        let r = call(&mut server, "cad.asbuilt.check", args);
        assert_eq!(r["isError"], false, "{name}: {r}");
        same(&r["structuredContent"]["output"]["tracks"], &step["result"]["output"]["tracks"], name);
    }
    let step = case("dönemin günü gün değil");
    let mut args = step["input"].clone();
    args["drawing"] = drawing.clone();
    let said = call(&mut server, "cad.asbuilt.check", args).to_string();
    assert!(said.contains(step["result"]["error"]["message"].as_str().unwrap()), "{said}");
    let _ = std::fs::remove_file(path);
}
