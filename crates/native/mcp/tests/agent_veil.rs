//! What an agent reads of a drawing whose forms keep a field out of examples (docs/adr/0738 §1): the owner of the
//! drawing reads and writes every value, but no answer through MCP carries the values of a field a form marks
//! `redact` or a layer marks `withheld`: not the objects' pages, not a record's page, not a refusal's words, not the
//! journal. The other fields' values are there; the summary names the hidden fields.

use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

const HIDDEN: &str = "‹gizli›";
/// The values the drawing holds in its hidden fields.
const SECRETS: [&str; 4] = ["Ali Yılmaz", "Ayşe Kaya", "12345678901", "98765432109"];

fn ask(server: &mut Server, name: &str, arguments: Value) -> Value {
    let request = json!({
        "jsonrpc": "2.0", "id": 1, "method": "tools/call",
        "params": {
            "name": name, "arguments": arguments,
            "_meta": {
                "io.modelcontextprotocol/protocolVersion": MODERN,
                "io.modelcontextprotocol/clientInfo": { "name": "perde", "version": "1" },
                "io.modelcontextprotocol/clientCapabilities": {}
            }
        }
    });
    server.handle(&request).expect("an answer")["result"].clone()
}

/// A tool's answer, checked to hold none of the hidden values in any of its parts.
fn told(server: &mut Server, name: &str, arguments: Value) -> Value {
    let result = ask(server, name, arguments);
    let text = result.to_string();
    for secret in SECRETS {
        assert!(!text.contains(secret), "{name} tells “{secret}”: {text}");
    }
    result["structuredContent"].clone()
}

fn done(server: &mut Server, drawing: &str, name: &str, mut arguments: Value) -> Value {
    arguments["drawing"] = json!(drawing);
    let answer = told(server, name, arguments);
    assert_eq!(answer["status"], "completed", "{name}: {answer}");
    answer["output"].clone()
}

/// A drawing whose `Bina` layer holds two objects with `Malik` (a form's `redact`) and `TC` (`withheld`): the values
/// are written first, the fields hidden after (an agent writes no hidden field, docs/adr/0743 §1). The handle, the
/// layer and the objects' ids.
fn veiled_drawing(server: &mut Server) -> (String, String, Vec<String>) {
    let made = told(
        server,
        "drawing.new",
        json!({ "name": "Perde", "srid": 5256 }),
    );
    let drawing = made["drawing"].as_str().expect("a handle").to_owned();
    done(
        server,
        &drawing,
        "cad.layers.edit",
        json!({ "operation": "add", "name": "Bina", "activate": true }),
    );
    let layers = told(server, "drawing.layers", json!({ "drawing": drawing }));
    let layer = layers["layers"]
        .as_array()
        .and_then(|l| l.iter().find(|n| n["name"] == "Bina"))
        .and_then(|n| n["id"].as_str())
        .expect("the layer")
        .to_owned();
    let fields = |withheld: bool| {
        json!({ "operation": "fields", "layer": layer, "fields": [
            { "name": "Ada", "kind": "text" },
            { "name": "Malik", "kind": "text" },
            { "name": "TC", "kind": "text", "unique": true, "withheld": withheld }
        ] })
    };
    done(server, &drawing, "cad.layers.edit", fields(false));
    let mut uids = Vec::new();
    for (x, (ada, malik, tc)) in [
        (0.0, ("101", "Ali Yılmaz", "12345678901")),
        (20.0, ("102", "Ayşe Kaya", "98765432109")),
    ] {
        let out = done(
            server,
            &drawing,
            "cad.polygon.create",
            json!({ "pts": square(x), "layerId": layer, "attrs": { "Ada": ada, "Malik": malik, "TC": tc } }),
        );
        uids.push(out["uid"].as_str().expect("its id").to_owned());
    }
    done(server, &drawing, "cad.layers.edit", fields(true));
    let body = json!({ "revision": 1, "items": [
        { "type": "field", "id": "ada", "field": "Ada" },
        { "type": "field", "id": "malik", "field": "Malik", "redact": true }
    ] });
    done(
        server,
        &drawing,
        "cad.forms.edit",
        json!({ "edit": { "kind": "create", "name": "Bina kaydı", "layer": layer, "id": "bina", "body": body } }),
    );
    done(
        server,
        &drawing,
        "cad.forms.edit",
        json!({ "edit": { "kind": "publish", "id": "bina" } }),
    );
    (drawing, layer, uids)
}

fn square(x: f64) -> Value {
    json!([{ "x": x, "y": 0.0 }, { "x": x + 10.0, "y": 0.0 }, { "x": x + 10.0, "y": 8.0 }, { "x": x, "y": 8.0 }])
}

#[test]
fn an_agent_reads_no_value_of_a_hidden_field() {
    let mut server = Server::new();
    let (drawing, layer, uids) = veiled_drawing(&mut server);

    // The summary names the hidden fields.
    let summary = told(
        &mut server,
        "drawing.summary",
        json!({ "drawing": drawing }),
    );
    let hidden: Vec<&str> = summary["hidden"]
        .as_array()
        .expect("hidden")
        .iter()
        .filter_map(Value::as_str)
        .collect();
    assert!(
        hidden.contains(&"Malik") && hidden.contains(&"TC"),
        "{hidden:?}"
    );
    // The objects' pages and one object: the open field's value, the hidden ones written hidden.
    let page = told(
        &mut server,
        "drawing.entities",
        json!({ "drawing": drawing, "layer": layer }),
    );
    let text = page.to_string();
    assert!(
        text.contains("\"Ada\":\"101\"") && text.contains(HIDDEN),
        "{text}"
    );
    let one = told(
        &mut server,
        "drawing.entity",
        json!({ "drawing": drawing, "uid": uids[0] }),
    );
    assert_eq!(one["entity"]["attrs"]["Malik"], HIDDEN);
    assert_eq!(one["entity"]["attrs"]["Ada"], "101");
    // A read command: the records' page.
    let records = done(
        &mut server,
        &drawing,
        "cad.records.page",
        json!({ "layer": layer }),
    );
    assert!(records.to_string().contains("\"102\""), "{records}");
    // A write of a hidden field is refused before it runs: a unique field's refusal would tell a value is taken
    // (docs/adr/0743 §1).
    let refused = told(
        &mut server,
        "cad.polygon.create",
        json!({ "drawing": drawing, "pts": square(40.0), "layerId": layer,
                "attrs": { "Ada": "103", "TC": "12345678901" } }),
    );
    assert_eq!(refused["status"], "failed", "{refused}");
    assert_eq!(refused["error"]["code"], "hidden_field", "{refused}");
    assert_eq!(refused["error"]["path"], "input.attrs.TC", "{refused}");
    // The journal: the calls' inputs as the command history writes them.
    let journal = told(
        &mut server,
        "drawing.journal",
        json!({ "drawing": drawing }),
    );
    assert!(journal.to_string().contains(HIDDEN), "{journal}");
    // The drawing itself holds the values: the owner's.
    let session_text = told(
        &mut server,
        "drawing.measure",
        json!({ "drawing": drawing, "uid": uids[1] }),
    );
    assert!(session_text.is_object());
}

/// An agent computes, filters and sorts with no hidden field, and the files it writes hold none (docs/adr/0743).
#[test]
fn an_agent_neither_reads_a_hidden_field_nor_writes_one_to_a_file() {
    let mut server = Server::new();
    let (drawing, layer, _) = veiled_drawing(&mut server);
    let refused = |server: &mut Server, name: &str, mut arguments: Value| -> Value {
        arguments["drawing"] = json!(drawing);
        let answer = told(server, name, arguments);
        assert_eq!(answer["status"], "failed", "{name}: {answer}");
        answer["error"].clone()
    };
    // An expression that reads the field: the value would be copied to an open one.
    let e = refused(
        &mut server,
        "cad.fields.write",
        json!({ "layer": layer, "write": { "kind": "calculate", "field": "Ada", "expression": "upper(Malik)" } }),
    );
    assert_eq!(e["code"], "hidden_field");
    assert_eq!(e["path"], "input.write.expression");
    // A field in brackets, and a template.
    let e = refused(
        &mut server,
        "cad.fields.write",
        json!({ "layer": layer, "write": { "kind": "calculate", "field": "Ada", "expression": "[TC] || ''" } }),
    );
    assert_eq!(e["code"], "hidden_field", "{e}");
    // The name in a quoted text is a text, not the field: the call runs.
    let fine = told(
        &mut server,
        "cad.fields.write",
        json!({ "drawing": drawing, "layer": layer,
                "write": { "kind": "calculate", "field": "Ada", "expression": "'Malik'" } }),
    );
    assert_eq!(fine["status"], "completed", "{fine}");
    told(&mut server, "drawing.undo", json!({ "drawing": drawing }));
    // A processing result to a file would carry the inputs' attributes.
    let e = refused(
        &mut server,
        "cad.processing.run",
        json!({ "tool": "geometry.buffer", "sink": { "path": "/nowhere/x.gpkg" } }),
    );
    assert_eq!(e["code"], "hidden_field_sink");
    assert_eq!(e["path"], "input.sink");

    // A file the agent writes: made without the hidden fields, its call in the drawing's journal.
    let dir = std::env::temp_dir().join(format!("kentos-perde-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    let csv = dir.join("bina.csv");
    let wrote = done(
        &mut server,
        &drawing,
        "file.export.csv",
        json!({ "path": csv.display().to_string() }),
    );
    assert!(wrote.is_object(), "{wrote}");
    let text = std::fs::read_to_string(&csv).expect("the file");
    assert!(text.contains("Ada"), "{text}");
    for hidden in ["Malik", "TC"] {
        assert!(!text.contains(hidden), "the column {hidden}: {text}");
    }
    for secret in SECRETS {
        assert!(!text.contains(secret), "“{secret}” in the file: {text}");
    }
    let journal = told(
        &mut server,
        "drawing.journal",
        json!({ "drawing": drawing }),
    );
    assert!(journal.to_string().contains("file.export.csv"), "{journal}");
    // As a job too.
    let csv2 = dir.join("bina-is.csv");
    let started = told(
        &mut server,
        "drawing.job.start",
        json!({ "drawing": drawing, "command": "file.export.csv",
                "input": { "path": csv2.display().to_string() } }),
    );
    let id = started["id"].as_str().expect("a job").to_owned();
    let waited = told(
        &mut server,
        "drawing.job.wait",
        json!({ "drawing": drawing, "id": id }),
    );
    assert_eq!(waited["phase"], "done", "{waited}");
    let text = std::fs::read_to_string(&csv2).expect("the job's file");
    for secret in SECRETS {
        assert!(
            !text.contains(secret),
            "“{secret}” in the job's file: {text}"
        );
    }
    assert!(!text.contains("Malik"), "{text}");
    // A job that reads a hidden field is refused before it starts.
    let refused_job = ask(
        &mut server,
        "drawing.job.start",
        json!({ "drawing": drawing, "command": "cad.fields.write",
                "input": { "layer": layer, "write": { "kind": "calculate", "field": "Ada", "expression": "TC" } } }),
    );
    assert!(
        refused_job.to_string().contains("hidden_field"),
        "{refused_job}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
