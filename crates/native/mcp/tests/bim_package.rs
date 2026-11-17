//! Real MCP host, the BIM permit package end to end (docs/adr/8075, 8077; plan/bim-plan.md BIM-14): an agent opens the
//! declared section building (fixtures/arch/v1/kesit-binasi-tam.kcad: the künye, the system, the parcel, the fire
//! classes and the permit set made, no answer given), publishes and is stopped by the questions left to the author,
//! answers them with `cad.form.answer`, and publishes the package whole: the model beside the sheets, the form's record
//! with the drawing's answers.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(server: &mut Server, name: &str, args: Value) -> Value {
    let result = server
        .handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{
            "name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,
            "io.modelcontextprotocol/clientInfo":{"name":"bim-package-test","version":"1"},
            "io.modelcontextprotocol/clientCapabilities":{}}}}))
        .unwrap()["result"]
        .clone();
    assert_eq!(result["resultType"], "complete", "{result}");
    result
}

#[test]
fn an_agent_answers_the_form_and_publishes_the_bim_package() {
    let dir = std::env::temp_dir().join(format!("kentos-mcp-bim-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let source = dir.join("kesit-binasi-tam.kcad");
    std::fs::copy(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../fixtures/arch/v1/kesit-binasi-tam.kcad"), &source)
        .expect("the declared drawing");
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": source}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    let out = dir.join("paket");
    let publish = |server: &mut Server| {
        call(
            server,
            "file.publish.sheets",
            json!({"drawing": drawing, "path": out, "mode": "separate", "pdfa": true,
                   // EK-2 Tablo 2.1's file name, as Ruhsat takımı names a permit set's files.
                   "naming": "{@proje_kodu}_{@bina_kodu}_MM_XXXX_{@dosya_turu}_{@dosya_tipi}_{@kat_konum}_{@sira_no}_{@revizyon_no}",
                   "bim": {}}),
        )
    };
    // No answer yet: the three questions left to the author stop it, nothing is written.
    let stopped = publish(&mut server);
    let error = &stopped["structuredContent"]["error"];
    assert_eq!(error["code"], "publish_blocked", "{stopped}");
    assert!(error["message"].as_str().unwrap().contains("müellifin cevabını bekliyor"), "{stopped}");
    assert!(!out.exists());
    // The agent answers them in the drawing.
    let answered = call(
        &mut server,
        "cad.form.answer",
        json!({"drawing": drawing, "answers": [
            {"no": 1, "yes": true, "note": "Bakanlığın sınırı yayımlanmadı"}, {"no": 7, "yes": true}, {"no": 10, "yes": true}
        ]}),
    );
    assert_eq!(answered["structuredContent"]["status"], "completed", "{answered}");
    assert_eq!(answered["structuredContent"]["output"]["answers"].as_array().map(Vec::len), Some(3));
    // The package whole: its five results, the model in the folder, the record with the drawing's answers.
    let done = publish(&mut server);
    let output = &done["structuredContent"]["output"];
    assert_eq!(output["ready"], true, "{done}");
    let results: Vec<Value> = output["bim"]["results"].as_array().unwrap().iter().map(|r| r["holds"].clone()).collect();
    assert_eq!(results, [json!(true), json!(true), json!(true), json!(true), Value::Null], "{done}");
    let model = out.join(output["bim"]["model"].as_str().unwrap());
    assert!(model.is_file(), "{}", model.display());
    let record: Value =
        serde_json::from_slice(&std::fs::read(out.join(output["bim"]["form"].as_str().unwrap())).unwrap()).unwrap();
    assert_eq!(record["answers"][0]["note"], "Bakanlığın sınırı yayımlanmadı");
    let _ = std::fs::remove_dir_all(dir);
}
