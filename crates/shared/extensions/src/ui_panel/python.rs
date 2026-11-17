//! `kentos.ui_panel`, the panel components' Python builders (plan/eklenti.md ADIM 22, PY-UI-02; docs/adr/8057 §34):
//! made from the panel's JSON Schema ([`super::json_schema`]), so they never drift from the types KentOS reads a panel
//! into. One function a component, its required keys in order and the rest by name (snake case), returning the
//! component's JSON (`ui.text_field("onek", "Önek", value="Y-")`); a choice's options may be values alone. For a
//! dialog's fields (`ctx.dialog`), a panel's options patched by its code, or a panel written from Python.
//!
//! Generated into `python/kentos/ui_panel.py` by `KENTOS_WRITE_SDK=1 cargo test -p kentos-extensions --lib --
//! python_builders`; a test fails when the two differ.

use serde_json::Value;

/// A camelCase key as a Python name: `onChange` → `on_change`.
fn snake(key: &str) -> String {
    let mut out = String::with_capacity(key.len() + 4);
    for c in key.chars() {
        if c.is_ascii_uppercase() {
            out.push('_');
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

/// A property's Python type (a key not given is left out: none is never written).
fn python_type(root: &Value, property: &Value) -> String {
    let enum_of = |reference: &str| -> Option<String> {
        let name = reference.strip_prefix("#/$defs/")?;
        let values = root.pointer(&format!("/$defs/{name}/enum"))?.as_array()?;
        let quoted: Vec<String> = values
            .iter()
            .filter_map(Value::as_str)
            .map(|v| format!("\"{v}\""))
            .collect();
        Some(format!("Literal[{}]", quoted.join(", ")))
    };
    if let Some(list) = property.get("anyOf").and_then(Value::as_array) {
        return list
            .iter()
            .find_map(|v| v.get("$ref").and_then(Value::as_str).and_then(enum_of))
            .unwrap_or_else(|| "str".to_owned());
    }
    let kind = match property.get("type") {
        Some(Value::String(kind)) => kind.as_str(),
        Some(Value::Array(kinds)) => kinds
            .iter()
            .filter_map(Value::as_str)
            .find(|k| *k != "null")
            .unwrap_or("string"),
        _ => "string",
    };
    match kind {
        "number" => "float".to_owned(),
        "integer" => "int".to_owned(),
        "boolean" => "bool".to_owned(),
        "array" => match property.pointer("/items/$ref").and_then(Value::as_str) {
            Some(r) if r.ends_with("/Choice") => "Sequence[Choice | str]".to_owned(),
            // Another object (a file picker's kinds): as the schema writes it.
            Some(_) => "Sequence[dict[str, Any]]".to_owned(),
            // Numbers (a point's x and y).
            None if property.pointer("/items/type").and_then(Value::as_str) == Some("number") => {
                "Sequence[float]".to_owned()
            }
            _ => "Sequence[str]".to_owned(),
        },
        _ => "str".to_owned(),
    }
}

/// The longest line the module writes.
const WIDTH: usize = 120;

/// `def name(params) -> dict[str, Any]:`, one parameter a line when it would not fit.
fn signature(name: &str, params: &[String]) -> String {
    let line = format!("def {name}({}) -> dict[str, Any]:", params.join(", "));
    if line.chars().count() <= WIDTH {
        return line;
    }
    let mut out = format!("def {name}(\n");
    for param in params {
        out.push_str(&format!("    {param},\n"));
    }
    out.push_str(") -> dict[str, Any]:");
    out
}

/// A docstring in the function's body, its words wrapped to the module's width (room left for the closing quotes).
fn docstring(text: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    let mut line = String::from("    \"\"\"");
    let mut empty = true;
    for word in text.split_whitespace() {
        if !empty && line.chars().count() + 1 + word.chars().count() > WIDTH - 3 {
            lines.push(std::mem::replace(&mut line, String::from("    ")));
            empty = true;
        }
        if !empty {
            line.push(' ');
        }
        line.push_str(word);
        empty = false;
    }
    line.push_str("\"\"\"");
    lines.push(line);
    lines.join("\n")
}

/// The builders' module, from the panel's JSON Schema.
pub fn python_builders() -> String {
    let root = super::json_schema();
    let mut out = String::from(
        "# kentos.ui_panel: the panel components' builders (docs/adr/8057 §34). Generated from the panel's JSON Schema by\n\
         # `KENTOS_WRITE_SDK=1 cargo test -p kentos-extensions --lib -- python_builders`; do not edit.\n\
         \"\"\"Builders of ``kentos.ui.panel`` 1 components: each returns the component's JSON, for ``ctx.dialog``'s fields,\n\
         ``ctx.panel.set(options=…)`` or a panel written from Python (:func:`panel`). Required keys come in order, the rest\n\
         by name; a choice's options may be values alone (``[\"a\", \"b\"]``). Kapı 1 still judges what a schema cannot say\n\
         (a button's command, ids once, a rule's names).\"\"\"\n\n\
         from __future__ import annotations\n\n\
         from collections.abc import Sequence\n\
         from typing import Any, Literal, TypedDict\n\n\
         FORMAT = \"kentos.ui.panel\"\n\
         SCHEMA_VERSION = 1\n\n\n\
         class Choice(TypedDict):\n    \"\"\"One of a choice's options.\"\"\"\n\n    value: str\n    label: str\n\n\n\
         def _choices(options: Sequence[Choice | str]) -> list[dict[str, str]]:\n    \
         return [\n        {\"value\": o, \"label\": o} if isinstance(o, str) else {\"value\": o[\"value\"], \"label\": o.get(\"label\", o[\"value\"])}\n        for o in options\n    ]\n\n\n\
         def choice(value: str, label: str | None = None) -> Choice:\n    \
         \"\"\"One option of a ``select`` or a ``multiSelect``, its label the value when none is given.\"\"\"\n    \
         return {\"value\": str(value), \"label\": str(value if label is None else label)}\n\n\n\
         def file_filter(name: str, *extensions: str) -> dict[str, Any]:\n    \
         \"\"\"A kind of file a ``filePicker`` offers: a name, extensions without dots (``file_filter(\"CSV\", \"csv\")``).\"\"\"\n    \
         return {\"name\": str(name), \"extensions\": [str(e).lstrip(\".\").lower() for e in extensions]}\n\n\n\
         def panel(title: str, body: Sequence[dict[str, Any]], *, split: bool = False) -> dict[str, Any]:\n    \
         \"\"\"A whole panel (``ui/<id>.panel.json``): its title and components; ``split``: its outputs under a sash, in\n    \
         a scroller of their own (docs/adr/8057 §69).\"\"\"\n    \
         out: dict[str, Any] = {\"format\": FORMAT, \"schemaVersion\": SCHEMA_VERSION, \"title\": str(title), \"body\": list(body)}\n    \
         if split:\n        \
         out[\"split\"] = True\n    \
         return out\n",
    );
    let Some(kinds) = root
        .pointer("/$defs/Component/oneOf")
        .and_then(Value::as_array)
    else {
        return out;
    };
    let mut names: Vec<String> = Vec::new();
    for kind in kinds {
        let Some(name) = kind
            .pointer("/properties/type/const")
            .and_then(Value::as_str)
        else {
            continue;
        };
        let Some(keys) = super::keys_of(name) else {
            continue;
        };
        let props = kind.get("properties").cloned().unwrap_or(Value::Null);
        let required: Vec<&str> = kind
            .get("required")
            .and_then(Value::as_array)
            .map(|r| {
                r.iter()
                    .filter_map(Value::as_str)
                    .filter(|k| *k != "type")
                    .collect()
            })
            .unwrap_or_default();
        // Its keys in the order KentOS lists them, then its rules, its worked-out values, its tab and its row.
        let mut order: Vec<&str> = keys.to_vec();
        order.extend(
            super::rules::RULE_KEYS
                .iter()
                .chain(super::values::VALUE_KEYS.iter())
                .chain([super::tabs::TAB_KEY, super::rows::ROW_KEY].iter())
                .copied()
                .filter(|r| props.get(*r).is_some()),
        );
        let function = snake(name);
        names.push(function.clone());
        let mut params: Vec<String> = Vec::new();
        let mut lines: Vec<String> = Vec::new();
        let mut head = format!("{{\"type\": \"{name}\"");
        for key in order.iter().copied().filter(|k| required.contains(k)) {
            let ty = python_type(&root, &props[key]);
            let py = snake(key);
            params.push(format!("{py}: {ty}"));
            let value = if ty.starts_with("Sequence[Choice") {
                format!("_choices({py})")
            } else if ty.starts_with("Sequence") {
                format!("list({py})")
            } else {
                py.clone()
            };
            head.push_str(&format!(", \"{key}\": {value}"));
        }
        head.push('}');
        let mut keyword = false;
        for key in order.iter().copied().filter(|k| !required.contains(k)) {
            let Some(property) = props.get(key) else {
                continue;
            };
            if !keyword {
                params.push("*".to_owned());
                keyword = true;
            }
            let ty = python_type(&root, property);
            let py = snake(key);
            if ty == "bool" {
                let default = property
                    .get("default")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                params.push(format!(
                    "{py}: bool = {}",
                    if default { "True" } else { "False" }
                ));
                lines.push(if default {
                    format!("    if not {py}:\n        out[\"{key}\"] = False")
                } else {
                    format!("    if {py}:\n        out[\"{key}\"] = True")
                });
            } else {
                params.push(format!("{py}: {ty} | None = None"));
                let value = if ty.starts_with("Sequence[Choice") {
                    format!("_choices({py})")
                } else if ty.starts_with("Sequence") {
                    format!("list({py})")
                } else {
                    py.clone()
                };
                lines.push(format!(
                    "    if {py} is not None:\n        out[\"{key}\"] = {value}"
                ));
            }
        }
        let doc = kind
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or("A panel component.")
            .replace('\\', "\\\\")
            .replace("\"\"\"", "\\\"\"\"")
            .replace('\n', " ");
        out.push_str(&format!(
            "\n\n{}\n{}\n    out: dict[str, Any] = {head}\n",
            signature(&function, &params),
            docstring(&doc)
        ));
        for line in lines {
            out.push_str(&line);
            out.push('\n');
        }
        out.push_str("    return out\n");
    }
    let mut all = vec![
        "Choice".to_owned(),
        "choice".to_owned(),
        "file_filter".to_owned(),
        "panel".to_owned(),
    ];
    all.extend(names);
    // `__all__`, its names packed into lines of the module's width.
    out.push_str("\n\n__all__ = [\n");
    let mut line = String::new();
    for name in all {
        let item = format!("\"{name}\",");
        if !line.is_empty() && 4 + line.chars().count() + 1 + item.chars().count() > WIDTH {
            out.push_str(&format!("    {line}\n"));
            line.clear();
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(&item);
    }
    out.push_str(&format!("    {line}\n]\n"));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The committed module is the one made from the schema now (`KENTOS_WRITE_SDK=1 … -- python_builders` writes
    /// it): a key added to a component, a kind renamed, a rule a component takes reach the builders or fail here.
    #[test]
    fn python_builders_are_the_schemas() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../python/kentos/ui_panel.py");
        let made = python_builders();
        if std::env::var_os("KENTOS_WRITE_SDK").is_some() {
            std::fs::write(&path, &made).expect("python/kentos/ui_panel.py");
            return;
        }
        let committed = std::fs::read_to_string(&path).expect("python/kentos/ui_panel.py");
        assert!(
            committed == made,
            "the panel's schema changed: KENTOS_WRITE_SDK=1 cargo test -p kentos-extensions --lib -- python_builders"
        );
        for function in [
            "def text_field(\n    id: str,\n    label: str,\n    *,\n    placeholder: str | None = None,\n    value: str | None = None,\n    required: bool = False,\n    visible_when: str | None = None,\n    required_when: str | None = None,\n    enabled_when: str | None = None,\n    calculate: str | None = None,\n    default_expr: str | None = None,\n    tab: str | None = None,\n    row: str | None = None,\n) -> dict[str, Any]:",
            "    layer: str | None = None,\n    geometry: str | None = None,\n    multiple: bool = True,\n",
            "def select(\n    id: str,\n    label: str,\n    options: Sequence[Choice | str],\n    *,\n    value: str | None = None,\n",
            "    kind: Literal[\"bar\", \"pie\", \"histogram\", \"line\"] | None = None,\n",
            "def text(\n    text: str,\n    *,\n    id: str | None = None,\n    visible_when: str | None = None,\n    calculate: str | None = None,\n    tab: str | None = None,\n    row: str | None = None,\n) -> dict[str, Any]:\n    \"\"\"A line of text.\"\"\"",
        ] {
            assert!(made.contains(function), "{function}\n{made}");
        }
        assert!(
            made.lines().all(|l| l.chars().count() <= WIDTH),
            "a line past {WIDTH}"
        );
    }
}
