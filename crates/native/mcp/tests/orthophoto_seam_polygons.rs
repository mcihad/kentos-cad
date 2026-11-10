//! The seam measured along the two images' seamline polygons' shared boundary through a real AI host (plan/map-cad.md
//! MAP-12, docs/adr/4460): the shared seam polygons cases' drawings (two synthetic GeoTIFFs of the fixtures, their
//! paths made absolute, and the two polygons) opened as .kcad files over MCP. `cad.orthophoto.compare` with
//! `seam.polygons` gives each case's shared boundary (its parts and length) and places (each with its part and its
//! distance along it); the polygons 2 mm apart are refused at `seam.polygons`.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"orthophoto-seam-polygons-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

/// A text with its `$fixture:` paths made absolute, as the shared runners make them.
fn fixtures(v: &Value) -> Value {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/");
    serde_json::from_str(&v.to_string().replace("$fixture:", &root.display().to_string())).unwrap()
}

/// Two numbers' lists or numbers alike within 10⁻⁹.
fn near(got: &Value, want: &Value, at: &str) {
    match (got, want) {
        (Value::Array(a), Value::Array(b)) => {
            assert_eq!(a.len(), b.len(), "{at}");
            for (k, (x, y)) in a.iter().zip(b).enumerate() {
                near(x, y, &format!("{at}[{k}]"));
            }
        }
        _ => match (got.as_f64(), want.as_f64()) {
            (Some(x), Some(y)) => assert!((x - y).abs() < 1e-9, "{at}: {x} {y}"),
            _ => assert_eq!(got, want, "{at}"),
        },
    }
}

#[test]
fn the_seam_along_the_seamline_polygons_over_mcp_is_the_cases() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.orthophoto.compare.json")).unwrap();
    for name in [
        "dikiş çokgenlerinden: ortak sınırları dikiş hattının kendisi, yerleri onun yerleri",
        "dikiş çokgenleri öbür sırayla: ortak sınır öbür yönde, yerler güneyden",
        "dikiş çokgenlerinin iki ortak parçası, her biri kendi başından; kapalı çoklu çizgi de olur",
        "dikiş çokgeni 0,4 mm kaymış: 1 mm içinde ortak sınır",
        "dikiş çokgenleri 2 mm ayrık: ortak sınır yok",
    ] {
        let case = cases["cases"].as_array().unwrap().iter().find(|c| c["name"] == name).unwrap().clone();
        let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(fixtures(&case["setup"])).unwrap();
        let path = std::env::temp_dir().join(format!("kentos-orthophoto-seam-polygons-{}.kcad", std::process::id()));
        std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
        let mut server = Server::new();
        let opened = call(&mut server, "drawing.open", json!({"path": path}));
        assert_eq!(opened["isError"], false, "{opened}");
        let step = &case["steps"][0];
        let mut args = step["input"].clone();
        args["drawing"] = opened["structuredContent"]["drawing"].clone();
        let r = call(&mut server, "cad.orthophoto.compare", args);
        let _ = std::fs::remove_file(&path);
        if step["result"]["status"] == "failed" {
            assert_eq!(r["isError"], true, "{name}: {r}");
            let said = r.to_string();
            assert!(said.contains("seam.polygons") && said.contains("ortak sınırı yok"), "{name}: {said}");
            continue;
        }
        assert_eq!(r["isError"], false, "{name}: {r}");
        let (got, want) = (&r["structuredContent"]["output"]["seam"], &step["result"]["output"]["seam"]);
        assert_eq!(got["shared"]["polygons"], want["shared"]["polygons"], "{name}");
        for key in ["parts", "length", "places", "outside"] {
            near(&got["shared"][key], &want["shared"][key], &format!("{name} shared.{key}"));
        }
        let (g, w) = (got["patches"].as_array().unwrap(), want["patches"].as_array().unwrap());
        assert_eq!(g.len(), w.len(), "{name}");
        for (a, b) in g.iter().zip(w) {
            assert_eq!((&a["state"], &a["part"]), (&b["state"], &b["part"]), "{name}");
            for key in ["at", "dx", "dy", "correlation", "station"] {
                near(&a[key], &b[key], &format!("{name} {key}"));
            }
        }
    }
}
