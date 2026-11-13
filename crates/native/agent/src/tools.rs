//! The tools offered to a model (docs/adr/8056 §4). The app's tools are named with dots (`cad.run`, `catalog.search`);
//! providers take only `^[a-zA-Z0-9_-]{1,64}$` (the research note's trap 38). A turn's names are mapped both ways
//! ([`ToolNames`]: `.` becomes `__`, anything else outside the set `_`, a long name is cut and given a hash), so what
//! the desktop sees in a tool call is always the app's own name. Schemas go to each provider in the form it reads:
//! never in strict mode (the app's schemas have optional fields), Gemini's in its JSON Schema subset.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::providers::Wire;

/// A tool as the app offers it to the model.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolSpec {
    /// The app's name (`cad.run`); the model sees it mapped.
    pub name: String,
    pub description: String,
    /// The input's JSON Schema, an object.
    pub input_schema: Value,
}

impl ToolSpec {
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        input_schema: Value,
    ) -> ToolSpec {
        ToolSpec {
            name: name.into(),
            description: description.into(),
            input_schema,
        }
    }
}

/// The longest name every provider takes (OpenAI's 64).
const MAX: usize = 64;

/// A turn's tool names, the app's and the wire's, both ways.
#[derive(Clone, Debug, Default)]
pub(crate) struct ToolNames {
    to_wire: HashMap<String, String>,
    to_app: HashMap<String, String>,
}

impl ToolNames {
    /// The map of `names` (the tools offered, then those in the conversation's history); the first of two names that
    /// would meet on the wire keeps the plain form, the second is told apart by a hash.
    pub(crate) fn new<'a>(names: impl IntoIterator<Item = &'a str>) -> ToolNames {
        let mut map = ToolNames::default();
        for app in names {
            if map.to_wire.contains_key(app) {
                continue;
            }
            let mut wire = plain(app);
            let mut salt = 0u32;
            while map.to_app.contains_key(&wire) {
                wire = hashed(app, salt);
                salt += 1;
            }
            map.to_wire.insert(app.to_owned(), wire.clone());
            map.to_app.insert(wire, app.to_owned());
        }
        map
    }

    /// The name the model sees for the app's `name`.
    pub(crate) fn wire(&self, app: &str) -> String {
        self.to_wire.get(app).cloned().unwrap_or_else(|| plain(app))
    }

    /// The app's name for a name the model used; one the map does not know is read back as best it can (the desktop
    /// then refuses it as an unknown tool).
    pub(crate) fn app(&self, wire: &str) -> String {
        self.to_app
            .get(wire)
            .cloned()
            .unwrap_or_else(|| wire.replace("__", "."))
    }
}

/// `.` → `__`, characters outside `[a-zA-Z0-9_-]` → `_`, cut to 64 with a hash when longer.
fn plain(app: &str) -> String {
    let mut out = String::with_capacity(app.len() + 4);
    for c in app.chars() {
        match c {
            '.' => out.push_str("__"),
            'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '-' => out.push(c),
            _ => out.push('_'),
        }
    }
    if out.is_empty() {
        out.push_str("tool");
    }
    if out.len() > MAX { hashed(app, 0) } else { out }
}

/// The plain form cut to 55 characters, `_` and eight hex digits of a hash of the app's name.
fn hashed(app: &str, salt: u32) -> String {
    let mut base: String = plain_unbounded(app).chars().take(MAX - 9).collect();
    if base.is_empty() {
        base.push_str("tool");
    }
    format!("{base}_{:08x}", fnv1a(app.as_bytes(), salt) as u32)
}

fn plain_unbounded(app: &str) -> String {
    app.replace('.', "__")
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// FNV-1a, 64 bits: a stable hash for names and ids (the same in every run, unlike std's).
pub(crate) fn fnv1a(bytes: &[u8], salt: u32) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325 ^ u64::from(salt);
    for b in bytes {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// `schema` as `wire` takes it: a root object with properties, no `$schema`; for Gemini only the keywords of its JSON
/// Schema subset (unknown keywords are dropped, not sent to fail the request).
pub(crate) fn schema_for(wire: Wire, schema: &Value) -> Value {
    let mut root = match schema {
        Value::Object(map) => map.clone(),
        _ => Map::new(),
    };
    for k in ["$schema", "$id", "$comment"] {
        root.remove(k);
    }
    if !root.contains_key("type") {
        root.insert("type".into(), Value::String("object".into()));
    }
    if root.get("type") == Some(&Value::String("object".into())) && !root.contains_key("properties")
    {
        root.insert("properties".into(), Value::Object(Map::new()));
    }
    let root = Value::Object(root);
    match wire {
        Wire::GeminiNative => gemini_schema(&root),
        _ => root,
    }
}

/// The keywords Gemini's `parametersJsonSchema` reads.
const GEMINI_KEYWORDS: &[&str] = &[
    "$defs",
    "$ref",
    "$anchor",
    "type",
    "format",
    "title",
    "description",
    "enum",
    "items",
    "prefixItems",
    "minItems",
    "maxItems",
    "minimum",
    "maximum",
    "anyOf",
    "oneOf",
    "properties",
    "additionalProperties",
    "required",
    "propertyOrdering",
];

fn gemini_schema(schema: &Value) -> Value {
    let Value::Object(map) = schema else {
        return schema.clone();
    };
    let mut out = Map::new();
    for (key, value) in map {
        let kept = match key.as_str() {
            // A constant is an enumeration of one.
            "const" if !map.contains_key("enum") => match value {
                Value::String(_) | Value::Number(_) => {
                    Some(("enum".to_owned(), Value::Array(vec![value.clone()])))
                }
                _ => None,
            },
            "properties" | "$defs" => match value {
                Value::Object(members) => Some((
                    key.clone(),
                    Value::Object(
                        members
                            .iter()
                            .map(|(k, v)| (k.clone(), gemini_schema(v)))
                            .collect(),
                    ),
                )),
                _ => None,
            },
            "items" | "additionalProperties" => Some((key.clone(), gemini_schema(value))),
            "anyOf" | "oneOf" | "prefixItems" => match value {
                Value::Array(list) => Some((
                    key.clone(),
                    Value::Array(list.iter().map(gemini_schema).collect()),
                )),
                _ => None,
            },
            // Enumerations of strings and numbers only.
            "enum" => match value {
                Value::Array(list) => {
                    let list: Vec<Value> = list
                        .iter()
                        .filter(|v| matches!(v, Value::String(_) | Value::Number(_)))
                        .cloned()
                        .collect();
                    (!list.is_empty()).then(|| (key.clone(), Value::Array(list)))
                }
                _ => None,
            },
            k if GEMINI_KEYWORDS.contains(&k) => Some((key.clone(), value.clone())),
            _ => None,
        };
        if let Some((k, v)) = kept {
            out.insert(k, v);
        }
    }
    Value::Object(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn valid(wire: &str) -> bool {
        (1..=MAX).contains(&wire.len())
            && wire
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    }

    #[test]
    fn names_go_to_the_wire_and_come_back_the_same() {
        let long = format!("cad.{}", "çok.uzun.ad".repeat(12));
        let names = [
            "cad.run",
            "catalog.search",
            "audit.list_providers",
            "arch.run",
            "drawing.summary",
            "a__b",
            "a.b",
            "boşluk ve: iki nokta",
            long.as_str(),
            "",
        ];
        let map = ToolNames::new(names.iter().copied());
        assert_eq!(map.wire("cad.run"), "cad__run");
        assert_eq!(map.wire("audit.list_providers"), "audit__list_providers");
        let mut seen = std::collections::HashSet::new();
        for n in names {
            let w = map.wire(n);
            assert!(valid(&w), "{n:?} → {w:?}");
            assert!(
                seen.insert(w.clone()),
                "{n:?} meets another name on the wire as {w:?}"
            );
            assert_eq!(map.app(&w), n, "{n:?} → {w:?}");
        }
        // A name the model made up is read back as best it can be.
        assert_eq!(map.app("scene__layers"), "scene.layers");
        // The same names give the same map: a turn's requests agree.
        let again = ToolNames::new(names.iter().copied());
        assert!(names.iter().all(|n| again.wire(n) == map.wire(n)));
    }

    #[test]
    fn schemas_are_whole_objects_and_gemini_gets_its_subset() {
        assert_eq!(
            schema_for(Wire::OpenAiResponses, &json!(null)),
            json!({"type": "object", "properties": {}})
        );
        let s = json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "properties": {
                "mode": {"type": "string", "enum": ["plan", "execute"], "default": "plan", "description": "d"},
                "bbox": {"type": "array", "items": {"type": "number", "examples": [1]}, "minItems": 4, "maxItems": 4},
                "kind": {"const": "line"},
                "input": {"type": "object", "description": "serbest"},
                "pattern": {"type": "string", "pattern": "^a", "minLength": 1}
            },
            "required": ["mode"],
            "additionalProperties": false
        });
        let anthropic = schema_for(Wire::AnthropicMessages, &s);
        assert!(anthropic.get("$schema").is_none());
        assert_eq!(
            anthropic["properties"]["mode"]["default"], "plan",
            "other providers keep what they read"
        );
        let g = schema_for(Wire::GeminiNative, &s);
        assert_eq!(
            g,
            json!({
                "type": "object",
                "properties": {
                    "mode": {"type": "string", "enum": ["plan", "execute"], "description": "d"},
                    "bbox": {"type": "array", "items": {"type": "number"}, "minItems": 4, "maxItems": 4},
                    "kind": {"enum": ["line"]},
                    "input": {"type": "object", "description": "serbest"},
                    "pattern": {"type": "string"}
                },
                "required": ["mode"],
                "additionalProperties": false
            })
        );
    }
}
