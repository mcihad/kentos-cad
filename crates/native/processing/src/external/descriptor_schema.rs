//! An extension's İşlemler tool descriptor (`kentos.processingTool` 1) as a JSON Schema (docs/adr/8057 §28): an
//! editor's checking and completion, an agent's reference. Built here, beside [`super::from_descriptor`], from the same
//! vocabularies it reads by (the parameter kinds an extension's tool takes, the output kinds), and committed beside the
//! SDK's stub (`crates/shared/extensions/sdk/kentos-processing-tool.schema.json`), where the SDK pack gives it.
//!
//! The reader passes over a key it does not use; the schema is closed (`additionalProperties: false`) so that a key
//! written wrong ("lable") is said where it was written. What a schema cannot say (a field parameter's `of` naming a
//! parameter, a features output's `sinkParameter` naming a layer parameter, names once, a parameter's name not a Python
//! word) is the reader's: it stays the judge.

use serde_json::{Map, Value, json};

use super::{KINDS, MOST_MODEL_DESCRIPTION, OUTPUT_KINDS};

/// A parameter of `kind`: the keys every parameter has and its kind's own, closed.
fn parameter(kind: &str) -> Value {
    let text = json!({ "type": "string" });
    let boolean = json!({ "type": "boolean" });
    let mut properties: Map<String, Value> = Map::new();
    properties.insert(
        "name".into(),
        json!({ "type": "string", "pattern": "^[A-Za-z_][A-Za-z0-9_]*$",
                "description": "Aracın işlevine bu adla gider: Python'da bir ad (ctx ve Python'un sözcükleri olmaz)." }),
    );
    properties.insert("label".into(), text.clone());
    properties.insert("type".into(), json!({ "const": kind }));
    properties.insert("description".into(), text.clone());
    properties.insert("optional".into(), boolean.clone());
    properties.insert("advanced".into(), boolean.clone());
    properties.insert("default".into(), json!({ "description": "Türüne göre bir değer." }));
    properties.insert(
        "visibleWhen".into(),
        json!({ "type": "object", "required": ["param"], "additionalProperties": false,
                "properties": { "param": text, "equals": {} } }),
    );
    properties.insert(
        "picks".into(),
        json!({ "type": "object", "required": ["option", "point"], "additionalProperties": false,
                "properties": { "option": text, "point": text } }),
    );
    let own: Vec<(&str, Value)> = match kind {
        "features" => vec![
            ("kinds", json!({ "type": "array", "items": text })),
            (
                "scopes",
                json!({ "type": "array", "items": { "enum": ["selection", "visible", "all", "layer"] } }),
            ),
            ("writes", boolean.clone()),
            ("allowEmpty", boolean.clone()),
        ],
        "number" => vec![
            ("min", json!({ "type": "number" })),
            ("max", json!({ "type": "number" })),
            ("integer", boolean.clone()),
            ("unit", text.clone()),
            ("placeholder", text.clone()),
        ],
        "string" => vec![
            ("placeholder", text.clone()),
            ("maxLength", json!({ "type": "integer", "minimum": 0 })),
            ("allowEmpty", boolean.clone()),
            ("widget", json!({ "enum": ["color"] })),
            ("colorInheritance", boolean.clone()),
        ],
        "enum" => vec![(
            "options",
            json!({ "type": "array", "items": { "type": "object", "required": ["value"],
                    "additionalProperties": false,
                    "properties": { "value": text, "label": text, "hint": text } } }),
        )],
        "field" => vec![
            (
                "of",
                json!({ "anyOf": [ { "type": "string" }, { "type": "array", "items": { "type": "string" } } ],
                        "description": "Alanları okunacak parametre(ler)in adı." }),
            ),
            ("allowNew", boolean.clone()),
            ("multiple", boolean.clone()),
        ],
        "layer" => vec![(
            "newLayerStyle",
            json!({ "type": "object", "additionalProperties": false, "properties": {
                "color": text, "lineWeight": { "type": "number" }, "fill": text } }),
        )],
        _ => Vec::new(),
    };
    for (key, schema) in own {
        properties.insert(key.into(), schema);
    }
    json!({ "type": "object", "required": ["name", "type"], "additionalProperties": false,
            "properties": properties })
}

/// `kentos.processingTool` 1 as a JSON Schema.
pub fn descriptor_schema() -> Value {
    let text = json!({ "type": "string" });
    let parameters: Vec<Value> = KINDS.iter().map(|k| parameter(k)).collect();
    let output = json!({
        "type": "object", "required": ["name", "type"], "additionalProperties": false,
        "properties": { "name": { "type": "string", "minLength": 1 }, "label": text,
                        "type": { "enum": OUTPUT_KINDS }, "sinkParameter": text },
        "oneOf": [
            { "properties": { "type": { "const": "features" } }, "required": ["sinkParameter"] },
            { "properties": { "type": { "enum": OUTPUT_KINDS.iter().filter(|k| **k != "features").collect::<Vec<_>>() } } }
        ]
    });
    // Its examples (docs/adr/8057 §48): apart, the whole would pass the macro's depth.
    let feature = json!({ "type": "object", "required": ["layer", "geometry"], "additionalProperties": false,
        "properties": { "layer": text, "geometry": { "type": "object" }, "attrs": { "type": "object" } } });
    let expect = json!({ "type": "object", "additionalProperties": false, "properties": {
        "outputs": { "type": "object",
                     "description": "Çıktı adına: sayı ya da metnin değeri; nesne çıktısının count ve has (öznitelik kümeleri)." },
        "summary": { "type": "string", "description": "Özette geçmesi beklenen söz." } } });
    let example = json!({ "type": "object", "required": ["title", "features"], "additionalProperties": false,
        "properties": {
            "title": { "type": "string", "minLength": 1, "maxLength": 80 },
            "features": { "type": "array", "minItems": 1, "maxItems": super::MOST_EXAMPLE_FEATURES, "items": feature },
            "params": { "type": "object",
                        "description": "Aracın parametreleri; nesne parametresi örneğin katmanlarını adıyla anar." },
            "expect": expect } });
    let examples = json!({ "type": "array", "maxItems": super::MOST_EXAMPLES, "items": example,
        "description": "Örnekler: KentOS her birini kendi sentetik çiziminde çalıştırıp beklenenle karşılaştırır." });
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "title": "KentOS extension tool descriptor (kentos.processingTool 1)",
        "type": "object",
        "required": ["format", "schemaVersion", "id", "label", "parameters"],
        "additionalProperties": false,
        "properties": {
            "format": { "const": "kentos.processingTool" },
            "schemaVersion": { "const": 1 },
            "id": { "type": "string", "description": "Manifestin entrypoints.processing kimliği." },
            "version": text,
            "label": { "type": "string", "minLength": 1, "maxLength": 80 },
            "category": { "type": "string", "description": "KentOS eklenti araçlarını kendi kategorisinde gösterir." },
            "description": text,
            "help": text,
            "modelDescription": { "type": "string", "maxLength": MOST_MODEL_DESCRIPTION,
                                  "description": "Yapay zekâ modeli için." },
            "keywords": { "type": "array", "items": text },
            "parameters": { "type": "array", "items": { "oneOf": parameters } },
            "outputs": { "type": "array", "items": output },
            "feedback": { "type": "object", "additionalProperties": false,
                          "properties": { "progress": { "type": "boolean" }, "cancel": { "type": "boolean" } } },
            "capabilities": { "type": "object", "additionalProperties": false, "properties": {
                "runtimes": { "type": "array", "items": text,
                              "description": "Bu KentOS eklenti araçlarını masaüstünde çalıştırır: desktop-python." } } },
            "timeoutSeconds": { "type": "integer", "minimum": 1, "maximum": 3600 },
            "examples": examples
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A descriptor with one parameter of `kind` (and what it needs) and one output of `output`.
    fn descriptor(kind: &str, output: &str) -> Value {
        let mut parameter = json!({ "name": "girdi", "label": "Girdi", "type": kind });
        if kind == "enum" {
            parameter["options"] = json!([{ "value": "a", "label": "A" }]);
        }
        if kind == "field" {
            parameter["of"] = json!("girdi");
        }
        let mut parameters = vec![parameter];
        if kind != "layer" {
            parameters.push(json!({ "name": "katman", "label": "Katman", "type": "layer" }));
        }
        if kind == "field" {
            parameters.push(json!({ "name": "nesneler", "label": "Nesneler", "type": "features" }));
            parameters[0]["of"] = json!("nesneler");
        }
        let mut out = json!({ "name": "sonuc", "label": "Sonuç", "type": output });
        if output == "features" {
            out["sinkParameter"] =
                json!(if kind == "layer" { "girdi" } else { "katman" });
        }
        json!({ "format": "kentos.processingTool", "schemaVersion": 1, "id": "deneme", "label": "Deneme",
                "parameters": parameters, "outputs": [out] })
    }

    /// The committed schema is the one made here (`KENTOS_WRITE_SDK=1 cargo test -p kentos-processing --lib --
    /// descriptor_schema` writes it), and its vocabularies are the reader's: a descriptor of each parameter kind and
    /// each output kind it names reads.
    #[test]
    fn the_committed_schema_is_the_readers() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../shared/extensions/sdk/kentos-processing-tool.schema.json");
        let made = serde_json::to_string_pretty(&descriptor_schema()).unwrap() + "\n";
        if std::env::var_os("KENTOS_WRITE_SDK").is_some() {
            std::fs::write(&path, &made).expect("the schema");
            return;
        }
        let committed = std::fs::read_to_string(&path).expect("the committed schema");
        assert!(
            committed == made,
            "the reader's vocabularies changed: KENTOS_WRITE_SDK=1 cargo test -p kentos-processing --lib -- \
             descriptor_schema"
        );
        for kind in KINDS {
            for output in OUTPUT_KINDS {
                let read = super::super::from_descriptor(
                    "tr.ornek.deneme",
                    "Deneme",
                    "1.0.0",
                    "deneme",
                    &descriptor(kind, output),
                );
                assert!(read.is_ok(), "{kind} → {output}: {read:?}", read = read.err());
            }
        }
    }
}
