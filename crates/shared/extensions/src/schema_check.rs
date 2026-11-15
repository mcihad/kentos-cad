//! A JSON Schema checker for the part of the language the SDK's schemas use (made by schemars from KentOS's types,
//! docs/adr/8057 §25 and §28), for tests: a manifest or a panel KentOS ships that its schema refused would make the
//! schema a lie told to every editor and agent that reads it. `$ref` into the schema, `type`, `properties`, `required`,
//! `additionalProperties`, `items`, `minItems`, `maxItems`, `enum`, `const`, `oneOf`, `anyOf`, `allOf`, `minimum`,
//! `maximum`; the rest (`format`, `pattern`, `uniqueItems`, `description`, `default`, `title`) is left to the files'
//! own checks.

use serde_json::Value;

/// Why `value` does not meet the schema `root`, each at its JSON pointer; none when it does.
pub(crate) fn errors(root: &Value, value: &Value) -> Vec<String> {
    let mut out = Vec::new();
    check(root, root, value, "", &mut out);
    out
}

fn is_type(value: &Value, kind: &str) -> bool {
    match kind {
        "object" => value.is_object(),
        "array" => value.is_array(),
        "string" => value.is_string(),
        "integer" => value.is_i64() || value.is_u64(),
        "number" => value.is_number(),
        "boolean" => value.is_boolean(),
        "null" => value.is_null(),
        _ => false,
    }
}

/// The alternatives `value` meets, and the errors of the one that came nearest when it meets none.
fn alternatives(root: &Value, list: &[Value], value: &Value, at: &str) -> (usize, Vec<String>) {
    let mut met = 0;
    let mut nearest: Option<Vec<String>> = None;
    for schema in list {
        let mut errors = Vec::new();
        check(root, schema, value, at, &mut errors);
        if errors.is_empty() {
            met += 1;
        } else if nearest.as_ref().is_none_or(|n| errors.len() < n.len()) {
            nearest = Some(errors);
        }
    }
    (met, nearest.unwrap_or_default())
}

fn check(root: &Value, schema: &Value, value: &Value, at: &str, out: &mut Vec<String>) {
    let schema = match schema {
        Value::Bool(true) => return,
        Value::Bool(false) => {
            out.push(format!("{at}: burada değer olamaz"));
            return;
        }
        Value::Object(_) => schema,
        _ => return,
    };
    if let Some(reference) = schema.get("$ref").and_then(Value::as_str) {
        match reference.strip_prefix('#').and_then(|p| root.pointer(p)) {
            Some(target) => check(root, target, value, at, out),
            None => out.push(format!("{at}: çözülemeyen $ref {reference}")),
        }
    }
    let kinds: Vec<&str> = match schema.get("type") {
        Some(Value::String(kind)) => vec![kind.as_str()],
        Some(Value::Array(list)) => list.iter().filter_map(Value::as_str).collect(),
        _ => Vec::new(),
    };
    if !kinds.is_empty() && !kinds.iter().any(|k| is_type(value, k)) {
        out.push(format!("{at}: {} bekleniyordu, {value} var", kinds.join(" | ")));
        return;
    }
    if let Some(constant) = schema.get("const")
        && constant != value
    {
        out.push(format!("{at}: {constant} olmalı, {value} var"));
    }
    if let Some(list) = schema.get("enum").and_then(Value::as_array)
        && !list.contains(value)
    {
        out.push(format!("{at}: {value} izinli değerlerden değil"));
    }
    if let (Some(min), Some(v)) = (schema.get("minimum").and_then(Value::as_f64), value.as_f64())
        && v < min
    {
        out.push(format!("{at}: en az {min}"));
    }
    if let (Some(max), Some(v)) = (schema.get("maximum").and_then(Value::as_f64), value.as_f64())
        && v > max
    {
        out.push(format!("{at}: en çok {max}"));
    }
    if let Some(all) = schema.get("allOf").and_then(Value::as_array) {
        for one in all {
            check(root, one, value, at, out);
        }
    }
    for (key, exactly_one) in [("anyOf", false), ("oneOf", true)] {
        if let Some(list) = schema.get(key).and_then(Value::as_array) {
            let (met, nearest) = alternatives(root, list, value, at);
            if met == 0 {
                out.push(format!("{at}: {key} seçeneklerinin hiçbirine uymuyor"));
                out.extend(nearest);
            } else if exactly_one && met > 1 {
                out.push(format!("{at}: oneOf seçeneklerinden {met} tanesine birden uyuyor"));
            }
        }
    }
    if let Value::Object(map) = value {
        if let Some(required) = schema.get("required").and_then(Value::as_array) {
            for key in required.iter().filter_map(Value::as_str) {
                if !map.contains_key(key) {
                    out.push(format!("{at}: “{key}” gerekli"));
                }
            }
        }
        let properties = schema.get("properties").and_then(Value::as_object);
        for (key, item) in map {
            let here = format!("{at}/{key}");
            match properties.and_then(|p| p.get(key)) {
                Some(property) => check(root, property, item, &here, out),
                None => match schema.get("additionalProperties") {
                    Some(Value::Bool(false)) => out.push(format!("{here}: bilinmeyen anahtar")),
                    Some(other @ Value::Object(_)) => check(root, other, item, &here, out),
                    _ => {}
                },
            }
        }
    }
    if let Value::Array(items) = value {
        if let Some(min) = schema.get("minItems").and_then(Value::as_u64)
            && (items.len() as u64) < min
        {
            out.push(format!("{at}: en az {min} öğe"));
        }
        if let Some(max) = schema.get("maxItems").and_then(Value::as_u64)
            && (items.len() as u64) > max
        {
            out.push(format!("{at}: en çok {max} öğe"));
        }
    }
    if let (Value::Array(items), Some(each)) = (value, schema.get("items")) {
        for (i, item) in items.iter().enumerate() {
            check(root, each, item, &format!("{at}/{i}"), out);
        }
    }
}
