//! Mozaik birleşme çizgisi through a real AI host (plan/map-cad.md MAP-12, docs/adr/4700): the shared cases' drawings (two
//! synthetic GeoTIFFs of the fixtures, their paths made absolute, and the footprints) opened as .kcad files over MCP.
//! `cad.orthophoto.seamlines` gives each case's seam, ends, overlap and two polygons (and, by radiometry, its
//! differences) as the cases' plans do; written, each polygon has its object; crossed footprints are refused at
//! `footprints`.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"orthophoto-seamlines-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

/// A text with its `$fixture:` paths made absolute, as the shared runners make them.
fn fixtures(v: &Value) -> Value {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/");
    serde_json::from_str(&v.to_string().replace("$fixture:", &root.display().to_string())).unwrap()
}

/// Two numbers' lists or numbers alike within 10⁻⁷ (the cases' own `within`).
fn near(got: &Value, want: &Value, at: &str) {
    match (got, want) {
        (Value::Array(a), Value::Array(b)) => {
            assert_eq!(a.len(), b.len(), "{at}: {got} {want}");
            for (k, (x, y)) in a.iter().zip(b).enumerate() {
                near(x, y, &format!("{at}[{k}]"));
            }
        }
        _ => match (got.as_f64(), want.as_f64()) {
            (Some(x), Some(y)) => assert!((x - y).abs() < 1e-7, "{at}: {x} {y}"),
            _ => assert_eq!(got, want, "{at}"),
        },
    }
}

#[test]
fn the_seamlines_over_mcp_are_the_cases() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.orthophoto.seamlines.json")).unwrap();
    for name in [
        "iki şerit yan yana: birleşme çizgisi örtüşmenin ortasından, uçları ortak kenarlarda",
        "kaydırılmış şeritler: birleşme çizgisi kesişmelerden köşelerin açıortaylarıyla (Pan ve ark.'nın AEFC'si)",
        "eğri kesim: köşe ile kenar arasında parabol, noktaları aralıkla",
        "radyometrik fark, binalardan geçmeden: birleşme çizgisi binayı kesmez",
        "artı gibi kesişen ayak izleri",
    ] {
        let case = cases["cases"].as_array().unwrap().iter().find(|c| c["name"] == name).unwrap().clone();
        let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(fixtures(&case["setup"])).unwrap();
        let path = std::env::temp_dir().join(format!("kentos-orthophoto-seamlines-{}.kcad", std::process::id()));
        std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
        let mut server = Server::new();
        let opened = call(&mut server, "drawing.open", json!({"path": path}));
        assert_eq!(opened["isError"], false, "{opened}");
        let step = &case["steps"][0];
        let mut args = step["input"].clone();
        args["drawing"] = opened["structuredContent"]["drawing"].clone();
        let r = call(&mut server, "cad.orthophoto.seamlines", args);
        let _ = std::fs::remove_file(&path);
        if step["result"]["status"] == "failed" {
            assert_eq!(r["isError"], true, "{name}: {r}");
            let said = r.to_string();
            assert!(said.contains("footprints") && said.contains("ikiden çok kez el değiştiriyor"), "{name}: {said}");
            continue;
        }
        assert_eq!(r["isError"], false, "{name}: {r}");
        let (got, want) = (&r["structuredContent"]["output"], &step["result"]["output"]);
        for key in ["overlap", "overlapArea", "seam", "length"] {
            near(&got[key], &want[key], &format!("{name} {key}"));
        }
        for k in 0..2 {
            near(&got["ends"][k]["at"], &want["ends"][k]["at"], &format!("{name} ends[{k}]"));
            assert_eq!(got["ends"][k]["kind"], want["ends"][k]["kind"], "{name}");
            for key in ["ring", "area", "kept"] {
                near(&got["polygons"][k][key], &want["polygons"][k][key], &format!("{name} polygons[{k}].{key}"));
            }
            assert!(got["polygons"][k]["uid"].as_str().is_some_and(|u| !u.is_empty()), "{name}: written");
        }
        if want.get("difference").is_some() {
            for key in ["bottleneck", "mean", "searchedMean", "cells", "searched", "size"] {
                near(&got["difference"][key], &want["difference"][key], &format!("{name} difference.{key}"));
            }
        }
        assert_eq!(got["notes"], want["notes"], "{name}");
    }
}
