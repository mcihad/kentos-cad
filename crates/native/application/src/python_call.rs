//! A command's call as Python (docs/adr/0284): the code that runs the same
//! command with the same input through the SDK (`kentos.cad`, docs/adr/0131).
//! The command history gives it for a journal row or a refused call.
//!
//! The input goes in as the catalog's JSON, read by the command's own input
//! type (`command.input_type.from_json`), so every input, nested and tagged
//! ones too, is written the one way the SDK reads it back exactly. The
//! wrapper's path follows the SDK's generator (`scripts/python/sdk.py`):
//! `cad.` is left out, a segment that is a Python keyword gets a trailing
//! underscore (`file.import.dxf` is `cad.file.import_.dxf`). The input's
//! `expectedRevision` is left out: the code writes to the drawing it is
//! run on, as a replay does.

use serde_json::Value;

/// Python's keywords: a segment named so gets a trailing underscore.
const KEYWORDS: &[&str] = &[
    "False", "None", "True", "and", "as", "assert", "async", "await", "break", "class", "continue",
    "def", "del", "elif", "else", "except", "finally", "for", "from", "global", "if", "import",
    "in", "is", "lambda", "nonlocal", "not", "or", "pass", "raise", "return", "try", "while",
    "with", "yield",
];

/// How wide a literal is written on one line before it is broken.
const WIDTH: usize = 88;

/// The wrapper of `command` under `kentos.cad` (`cad.circle.create`).
pub fn python_path(command: &str) -> String {
    let mut parts: Vec<&str> = command.split('.').collect();
    if parts.first() == Some(&"cad") && parts.len() > 2 {
        parts.remove(0);
    }
    let mut path = String::from("cad");
    for part in parts {
        path.push('.');
        path.push_str(part);
        if KEYWORDS.contains(&part) {
            path.push('_');
        }
    }
    path
}

/// The Python that runs `command` with `input` on `doc` (the open drawing in
/// the desktop's Python console), under a comment that says what it is.
pub fn python_call(command: &str, version: u32, title: Option<&str>, input: &Value) -> String {
    python_call_in(command, version, title, input, false)
}

/// [`python_call`]; `relaxed`: the call ran with the constraints relaxed
/// (`relaxConstraints`, docs/adr/0337 §2), so it goes under `with doc.relaxed():`.
pub fn python_call_in(
    command: &str,
    version: u32,
    title: Option<&str>,
    input: &Value,
    relaxed: bool,
) -> String {
    let input = without_revision(input);
    let path = python_path(command);
    let what = title.map_or_else(String::new, |t| format!("{t} — "));
    let body = relaxing(relaxed, |pad| {
        let call = "command.execute_input(doc, command.input_type.from_json(";
        let literal = render(&input, pad, pad + call.len() + 2);
        format!("{call}{literal}))")
    });
    format!("# {what}{command} v{version}\nfrom kentos import cad\n\ncommand = {path}\n{body}")
}

/// The command's call as the Python console's echo writes it (docs/adr/0622 §7): its title as a comment, then one
/// call on `doc` with `cad` as the console has them (no import), the same input as [`python_call_in`].
pub fn python_echo(command: &str, title: Option<&str>, input: &Value, relaxed: bool) -> String {
    let input = without_revision(input);
    let path = python_path(command);
    let body = relaxing(relaxed, |pad| {
        let call = format!("{path}.execute_input(doc, {path}.input_type.from_json(");
        let literal = render(&input, pad, pad + call.chars().count() + 2);
        format!("{call}{literal}))")
    });
    match title {
        Some(t) => format!("# {t}\n{body}"),
        None => body,
    }
}

/// The input without the revision it was planned on: the code writes to the drawing it is run on.
fn without_revision(input: &Value) -> Value {
    let mut input = input.clone();
    if let Value::Object(map) = &mut input {
        map.remove("expectedRevision");
    }
    input
}

/// A call's line, given how deep it starts, under `with doc.relaxed():` when `relaxed`; ends with a newline.
fn relaxing(relaxed: bool, line: impl FnOnce(usize) -> String) -> String {
    if relaxed {
        format!("with doc.relaxed():\n    {}\n", line(4))
    } else {
        format!("{}\n", line(0))
    }
}

/// `value` as a Python literal on a line `indent` spaces deep: objects are
/// dicts, arrays lists, `true`/`false`/`null` `True`/`False`/`None`; a
/// container wider than the line is broken, an item a line, with a trailing comma.
pub fn python_literal(value: &Value, indent: usize) -> String {
    render(value, indent, indent)
}

/// [`python_literal`] starting at `column` of a line `indent` spaces deep.
fn render(value: &Value, indent: usize, column: usize) -> String {
    let flat = flat_literal(value);
    if column + flat.chars().count() <= WIDTH || !is_container(value) {
        return flat;
    }
    let inner = " ".repeat(indent + 4);
    let close = " ".repeat(indent);
    match value {
        Value::Array(items) => {
            let mut out = String::from("[\n");
            for item in items {
                out.push_str(&inner);
                out.push_str(&render(item, indent + 4, indent + 4));
                out.push_str(",\n");
            }
            out.push_str(&close);
            out.push(']');
            out
        }
        Value::Object(map) => {
            let mut out = String::from("{\n");
            for (key, item) in map {
                let key = string(key);
                out.push_str(&inner);
                out.push_str(&key);
                out.push_str(": ");
                out.push_str(&render(
                    item,
                    indent + 4,
                    indent + 4 + key.chars().count() + 2,
                ));
                out.push_str(",\n");
            }
            out.push_str(&close);
            out.push('}');
            out
        }
        _ => flat,
    }
}

fn is_container(value: &Value) -> bool {
    matches!(value, Value::Array(a) if !a.is_empty())
        || matches!(value, Value::Object(o) if !o.is_empty())
}

/// A string as Python reads it: JSON's escapes are Python's (`\"`, `\\`,
/// `\n`, `\uXXXX`), and serde_json writes no `\/`.
fn string(text: &str) -> String {
    serde_json::to_string(text).unwrap_or_else(|_| "\"\"".to_owned())
}

fn flat_literal(value: &Value) -> String {
    match value {
        Value::Null => "None".to_owned(),
        Value::Bool(true) => "True".to_owned(),
        Value::Bool(false) => "False".to_owned(),
        // JSON's numbers are Python's (`5`, `5.0`, `1e-7`).
        Value::Number(n) => n.to_string(),
        Value::String(s) => string(s),
        Value::Array(items) => format!(
            "[{}]",
            items
                .iter()
                .map(flat_literal)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Value::Object(map) => format!(
            "{{{}}}",
            map.iter()
                .map(|(k, v)| format!("{}: {}", string(k), flat_literal(v)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn paths_follow_the_sdk_s_generator() {
        assert_eq!(python_path("cad.circle.create"), "cad.circle.create");
        assert_eq!(python_path("cad.entities.break"), "cad.entities.break_");
        assert_eq!(python_path("file.import.dxf"), "cad.file.import_.dxf");
        assert_eq!(python_path("file.export.dxf"), "cad.file.export.dxf");
    }

    #[test]
    fn literals_are_python_s() {
        assert_eq!(
            python_literal(
                &json!({"a": true, "b": null, "c": [1, 2.5, -1e-7], "d": "x\"y\\\nç"}),
                0
            ),
            r#"{"a": True, "b": None, "c": [1, 2.5, -1e-7], "d": "x\"y\\\nç"}"#
        );
        let wide = json!({"points": [{"x": 500000.125, "y": 4400000.25}, {"x": 500010.5, "y": 4400000.25}, {"x": 500010.5, "y": 4400010.0}]});
        let text = python_literal(&wide, 0);
        assert!(text.starts_with("{\n    \"points\": ["), "{text}");
        assert!(text.lines().all(|l| l.chars().count() <= WIDTH), "{text}");
    }

    #[test]
    fn the_call_leaves_the_expected_revision_out() {
        let code = python_call(
            "cad.circle.create",
            1,
            Some("Daire oluştur"),
            &json!({"c": {"x": 1.0, "y": 2.0}, "expectedRevision": "7", "layerId": "cizim", "r": 5.0}),
        );
        assert_eq!(
            code,
            "# Daire oluştur — cad.circle.create v1\nfrom kentos import cad\n\ncommand = cad.circle.create\ncommand.execute_input(doc, command.input_type.from_json({\n    \"c\": {\"x\": 1.0, \"y\": 2.0},\n    \"layerId\": \"cizim\",\n    \"r\": 5.0,\n}))\n"
        );
        let short = python_call("cad.entities.delete", 1, None, &json!({"uids": ["a"]}));
        assert!(
            short.ends_with("from_json({\"uids\": [\"a\"]}))\n"),
            "{short}"
        );
    }

    #[test]
    fn a_relaxed_call_goes_under_relaxed_and_the_echo_is_one_call() {
        let input = json!({"c": {"x": 1.0, "y": 2.0}, "layerId": "cizim", "r": 5.0});
        let relaxed = python_call_in("cad.circle.create", 1, None, &input, true);
        assert!(
            relaxed.ends_with("command = cad.circle.create\nwith doc.relaxed():\n    command.execute_input(doc, command.input_type.from_json({\n        \"c\": {\"x\": 1.0, \"y\": 2.0},\n        \"layerId\": \"cizim\",\n        \"r\": 5.0,\n    }))\n"),
            "{relaxed}"
        );
        assert_eq!(
            python_echo("cad.circle.create", Some("Daire"), &input, false),
            "# Daire\ncad.circle.create.execute_input(doc, cad.circle.create.input_type.from_json({\n    \"c\": {\"x\": 1.0, \"y\": 2.0},\n    \"layerId\": \"cizim\",\n    \"r\": 5.0,\n}))\n"
        );
        assert_eq!(
            python_echo("cad.entities.delete", None, &json!({"uids": ["a"]}), false),
            "cad.entities.delete.execute_input(doc, cad.entities.delete.input_type.from_json({\n    \"uids\": [\"a\"],\n}))\n"
        );
        // A wide input breaks inside the relaxed block, its lines four deeper.
        let wide = json!({"points": [{"x": 500000.125, "y": 4400000.25}, {"x": 500010.5, "y": 4400000.25}, {"x": 500010.5, "y": 4400010.0}]});
        let echo = python_echo("cad.polyline.create", None, &wide, true);
        assert!(echo.starts_with("with doc.relaxed():\n    cad.polyline.create.execute_input("), "{echo}");
        assert!(echo.contains("\n        \"points\": ["), "{echo}");
        assert!(echo.ends_with("    }))\n"), "{echo}");
    }
}

/// The text a hidden value is written as (docs/adr/0443 §2).
pub const HIDDEN: &str = "‹gizli›";

/// The layer fields' keys and names the project's forms keep out of the recorder's examples, a table's
/// file, a statistic, a chart and an atlas (a field item's `redact`, in any form's draft or release):
/// the one resolver's list (`kentos_forms::reach::redacted_names`, docs/adr/0550), the same one the
/// server cuts a copy by. A local drawing's owner reads every value on screen; what leaves is cut.
pub fn redacted_names(doc: &kentos_domain::Document) -> Vec<String> {
    let layers = doc.layers();
    let empty: &[kentos_commands::LayerField] = &[];
    kentos_forms::reach::redacted_names(&doc.settings().forms, |layer| {
        layers.get(layer).map_or(empty, |n| n.fields.as_slice())
    })
}

/// `input` with every text value under a hidden name written as [`HIDDEN`]: a call's values, attributes,
/// children's values and notes alike, however deep (docs/adr/0443 §2).
pub fn redacted(input: &Value, names: &[String]) -> Value {
    match input {
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(k, v)| {
                    let v = match v {
                        Value::String(_) if names.iter().any(|n| n == k) => {
                            Value::String(HIDDEN.into())
                        }
                        other => redacted(other, names),
                    };
                    (k.clone(), v)
                })
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.iter().map(|v| redacted(v, names)).collect()),
        other => other.clone(),
    }
}

/// The calls as one Python script, in their order, each as [`python_call`] writes it, the hidden values
/// written as [`HIDDEN`] (docs/adr/0443 §2).
pub fn python_script(calls: &[(String, u32, Option<String>, Value, bool)], names: &[String]) -> String {
    let mut out = String::from(
        "# KentOS komut geçmişi: çağrılar sırasıyla. Python konsolunda doc açık çizimdir.\n",
    );
    if !names.is_empty() {
        out.push_str(&format!(
            "# Formda gizlenen alanların değerleri {HIDDEN} olarak yazıldı: çalıştırmadan önce yerlerine değer yazın.\n"
        ));
    }
    for (command, version, title, input, relaxed) in calls {
        out.push('\n');
        out.push_str(&python_call_in(
            command,
            *version,
            title.as_deref(),
            &redacted(input, names),
            *relaxed,
        ));
    }
    out
}

#[cfg(test)]
mod redaction_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_hidden_name_s_texts_are_written_hidden_however_deep() {
        let names = vec!["Kimlik No".to_owned(), "tc".to_owned()];
        let input = json!({ "form": "malik", "values": { "Kimlik No": "12345678901", "Ad": "Ayşe" },
            "children": [{ "values": { "tc": "98765432109", "Pay": "1/2" } }], "count": 3 });
        assert_eq!(
            redacted(&input, &names),
            json!({ "form": "malik", "values": { "Kimlik No": HIDDEN, "Ad": "Ayşe" },
                "children": [{ "values": { "tc": HIDDEN, "Pay": "1/2" } }], "count": 3 })
        );
        let script = python_script(
            &[(
                "cad.forms.submit".into(),
                1,
                Some("Formla kaydet".into()),
                input,
                false,
            )],
            &names,
        );
        assert!(
            script.contains(HIDDEN)
                && !script.contains("12345678901")
                && !script.contains("98765432109")
        );
        assert!(script.contains("Ayşe"));
    }
}
