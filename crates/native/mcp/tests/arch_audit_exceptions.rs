//! Real MCP host (docs/adr/0732): an agent audits the shared cases' flat, makes one finding an exception with a note,
//! sees it hidden and counted, saves and reopens the drawing with the exception kept, then takes it back.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(server: &mut Server, name: &str, args: Value) -> Value {
    let result = server
        .handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{
            "name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,
            "io.modelcontextprotocol/clientInfo":{"name":"arch-audit-exceptions-test","version":"1"},
            "io.modelcontextprotocol/clientCapabilities":{}}}}))
        .unwrap()["result"]
        .clone();
    assert_eq!(result["resultType"], "complete", "{result}");
    result
}

const TAG: &str = "tag_target_gone:0192f5a1-7030-7000-8000-000000000013";

#[test]
fn an_exception_hides_its_finding_goes_into_the_file_and_comes_back_out() {
    let fixture: Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.arch.exceptions.json")).unwrap();
    let dir = std::env::temp_dir().join(format!("kentos-mcp-arch-exceptions-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    // The cases' drawing is a v2 snapshot: written as the .kcad an agent would open.
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(fixture["setup"].clone()).unwrap();
    let source = dir.join("daire-kaynak.kcad");
    kentos_headless::files::write(&kentos_domain::Document::from_snapshot_v2(snapshot).unwrap(), &source).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path":source}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    let audit = |server: &mut Server, drawing: &Value, input: Value| {
        let mut args = input;
        args["drawing"] = drawing.clone();
        let answer = call(server, "cad.arch.audit", args);
        assert_eq!(answer["isError"], false, "{answer}");
        answer["structuredContent"]["output"].clone()
    };
    let keys = |output: &Value| -> Vec<String> {
        output["findings"].as_array().unwrap().iter().map(|f| f["key"].as_str().unwrap().to_owned()).collect()
    };
    let before = audit(&mut server, &drawing, json!({}));
    assert!(keys(&before).iter().any(|k| k == TAG), "{before}");
    // Mark: one step, the note kept; the finding is no longer said, only counted.
    let marked = call(&mut server, "cad.arch.exceptions",
        json!({"drawing":drawing,"operation":"mark","keys":[TAG],"note":"Bilerek bırakıldı."}));
    assert_eq!(marked["isError"], false, "{marked}");
    assert_eq!(marked["structuredContent"]["output"]["changed"], json!([TAG]), "{marked}");
    let hidden = audit(&mut server, &drawing, json!({}));
    assert!(!keys(&hidden).iter().any(|k| k == TAG), "{hidden}");
    assert_eq!(hidden["excepted"], 1, "{hidden}");
    let shown = audit(&mut server, &drawing, json!({"exceptions":"only"}));
    let only = shown["findings"].as_array().unwrap();
    assert_eq!(only.len(), 1, "{shown}");
    assert_eq!((only[0]["exception"].clone(), only[0]["note"].clone()), (json!(true), json!("Bilerek bırakıldı.")));
    // An unknown key refuses the whole call with the finding's code, nothing written.
    let unknown = call(&mut server, "cad.arch.exceptions",
        json!({"drawing":drawing,"operation":"mark","keys":["arch_wall_end_gap:yok"]}));
    assert_eq!(unknown["structuredContent"]["error"]["code"], "finding_not_found", "{unknown}");
    // Saved and opened again: the exception is the file's.
    let saved = dir.join("daire.kcad");
    let written = call(&mut server, "drawing.save", json!({"drawing":drawing,"path":saved}));
    assert_eq!(written["isError"], false, "{written}");
    let again = call(&mut server, "drawing.open", json!({"path":saved}));
    let other = again["structuredContent"]["drawing"].clone();
    let reopened = audit(&mut server, &other, json!({}));
    assert!(!keys(&reopened).iter().any(|k| k == TAG), "{reopened}");
    assert_eq!(reopened["excepted"], 1, "{reopened}");
    // Unmark: the finding is said again.
    let unmarked = call(&mut server, "cad.arch.exceptions", json!({"drawing":other,"operation":"unmark","keys":[TAG]}));
    assert_eq!(unmarked["isError"], false, "{unmarked}");
    let back = audit(&mut server, &other, json!({}));
    assert!(keys(&back).iter().any(|k| k == TAG), "{back}");
    assert_eq!(back["excepted"], 0, "{back}");
    let _ = std::fs::remove_dir_all(dir);
}
