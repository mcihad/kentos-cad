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

#[test]
fn an_agent_reads_no_value_of_a_hidden_field() {
    let mut server = Server::new();
    let made = told(
        &mut server,
        "drawing.new",
        json!({ "name": "Perde", "srid": 5256 }),
    );
    let drawing = made["drawing"].as_str().expect("a handle").to_owned();
    done(
        &mut server,
        &drawing,
        "cad.layers.edit",
        json!({ "operation": "add", "name": "Bina", "activate": true }),
    );
    let layers = told(&mut server, "drawing.layers", json!({ "drawing": drawing }));
    let layer = layers["layers"]
        .as_array()
        .and_then(|l| l.iter().find(|n| n["name"] == "Bina"))
        .and_then(|n| n["id"].as_str())
        .expect("the layer")
        .to_owned();
    done(
        &mut server,
        &drawing,
        "cad.layers.edit",
        json!({ "operation": "fields", "layer": layer, "fields": [
            { "name": "Ada", "kind": "text" },
            { "name": "Malik", "kind": "text" },
            { "name": "TC", "kind": "text", "unique": true, "withheld": true }
        ] }),
    );
    let body = json!({ "revision": 1, "items": [
        { "type": "field", "id": "ada", "field": "Ada" },
        { "type": "field", "id": "malik", "field": "Malik", "redact": true }
    ] });
    done(
        &mut server,
        &drawing,
        "cad.forms.edit",
        json!({ "edit": { "kind": "create", "name": "Bina kaydı", "layer": layer, "id": "bina", "body": body } }),
    );
    done(
        &mut server,
        &drawing,
        "cad.forms.edit",
        json!({ "edit": { "kind": "publish", "id": "bina" } }),
    );
    let square = |x: f64| json!([{ "x": x, "y": 0.0 }, { "x": x + 10.0, "y": 0.0 }, { "x": x + 10.0, "y": 8.0 }, { "x": x, "y": 8.0 }]);
    let mut uids = Vec::new();
    for (x, (ada, malik, tc)) in [
        (0.0, ("101", "Ali Yılmaz", "12345678901")),
        (20.0, ("102", "Ayşe Kaya", "98765432109")),
    ] {
        let out = done(
            &mut server,
            &drawing,
            "cad.polygon.create",
            json!({ "pts": square(x), "layerId": layer, "attrs": { "Ada": ada, "Malik": malik, "TC": tc } }),
        );
        uids.push(out["uid"].as_str().expect("its id").to_owned());
    }

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
    // A refusal's words: the unique field's value is said to be taken, and is not said.
    let refused = told(
        &mut server,
        "cad.polygon.create",
        json!({ "drawing": drawing, "pts": square(40.0), "layerId": layer,
                "attrs": { "Ada": "103", "TC": "12345678901" } }),
    );
    assert_eq!(refused["status"], "failed", "{refused}");
    assert!(refused.to_string().contains(HIDDEN), "{refused}");
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
