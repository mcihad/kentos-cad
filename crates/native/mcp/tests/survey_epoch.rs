//! The GNSS network at its measurement epoch through a real AI host (plan/map-cad.md MAP-03, docs/adr/2120): the shared
//! cases' drawing whose baselines carry their GPS times and whose campaign has the velocity grid, opened as a .kcad
//! file over MCP, gives the case's measurement epoch, where it comes from, the known points moved to it from 2005.0 and
//! how far; a given epoch wins, as its case says.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"survey-epoch-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

const MOVED: &str = "Ölçme epoku bazların gözlem zamanlarından (2 Kasım 2026); 2005,0'daki bilinen noktalar hız ızgarasıyla \
                     oraya taşınır, yeni noktalar karara (2005,0) döner";
const GIVEN: &str = "Ölçme epoku verilir (2026,5): bazların zamanından önce gelir";

fn near(got: &Value, want: &Value, at: &str) {
    match (got.as_f64(), want.as_f64()) {
        (Some(a), Some(b)) => assert!((a - b).abs() < 1e-6, "{at}: {a} != {b}"),
        _ => assert_eq!(got, want, "{at}"),
    }
}

#[test]
fn the_known_points_go_to_the_measurement_epoch_over_mcp() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    // The velocity grid in the core, as the device's grid library gives it.
    let grid = std::fs::read(root.join("fixtures/geoid/v1/hiz.tif")).unwrap();
    let sha = kentos_native_application::crs_models::sha256(&grid);
    let dir = std::env::temp_dir().join(format!("kentos-epoch-grids-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join(format!("{sha}.tif")), &grid).unwrap();
    kentos_native_application::crs_models::load(&sha, kentos_native_application::crs_models::ModelKind::Velocity, Some(&dir))
        .unwrap();
    let cases: Value =
        serde_json::from_str(&std::fs::read_to_string(root.join("fixtures/commands/v1/map.survey.network.json")).unwrap())
            .unwrap();
    let find = |name: &str| cases["cases"].as_array().unwrap().iter().find(|c| c["name"] == name).unwrap().clone();
    let case = find(MOVED);
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(case["setup"].clone()).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-survey-epoch-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    for (name, input) in [(MOVED, json!({})), (GIVEN, json!({"epoch": 2026.5}))] {
        let want = &find(name)["steps"][0]["result"]["output"];
        let mut args = input.clone();
        args["drawing"] = drawing.clone();
        let r = call(&mut server, "map.survey.network", args);
        assert_eq!(r["isError"], false, "{r}");
        let got = &r["structuredContent"]["output"];
        near(&got["epoch"], &want["epoch"], "epoch");
        assert_eq!(got["epochFrom"], want["epochFrom"], "{name}");
        assert_eq!(got["acceptable"], want["acceptable"], "{name}");
        let (gm, wm) = (got["moved"].as_array().unwrap(), want["moved"].as_array().unwrap());
        assert_eq!(gm.len(), wm.len(), "{name}");
        for (g, w) in gm.iter().zip(wm) {
            assert_eq!(g["id"], w["id"]);
            near(&g["years"], &w["years"], "years");
            for k in 0..3 {
                near(&g["shift"][k], &w["shift"][k], "shift");
            }
        }
    }
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_dir_all(dir);
}
