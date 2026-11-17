//! An audit rule's examples (plan/eklenti.md ADIM 20–21, docs/adr/8057 §75): what the rule is given and the findings it
//! must give, in the package beside its code (`entrypoints.audits[].examples`, `audits/<kimlik>.examples.json`,
//! `kentos.auditExamples` 1). The İşlemler tools' examples live in their descriptors (§48); a rule has no descriptor.
//!
//! - **What it is given:** objects as data (`features`: a layer by name, a GeoJSON shape, attributes), made into a
//!   synthetic drawing whose facts KentOS measures as for any drawing; and what KentOS measures in an architectural
//!   drawing (`architecture`, `facts.project.architecture`: storeys, rooms, units, doors), given as data, since a
//!   drawing of walls and spaces cannot be written as GeoJSON. A rule reads facts, so an example of facts tests it
//!   whole.
//! - **What it must give:** how many findings (`count`) and findings it must have among them (`has`: the severity,
//!   words of the message, the example's object it names by its place in `features`, the command its fix runs).
//! - **Read without running anything:** the file is checked at packing and at installing (gate 1), each finding with
//!   its place; the desktop's Örnekleri çalıştır, an agent's trial and `kentos kek test` run it.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::audit_facts::ArchitectureFacts;
use crate::problem::Problem;

/// The file's format name.
pub const FORMAT: &str = "kentos.auditExamples";

/// The most examples a rule gives.
pub const MOST_EXAMPLES: usize = 10;

/// The most objects an example gives, as a tool's (§48).
pub const MOST_FEATURES: usize = 500;

/// The shapes an example's object may have: GeoJSON's.
const SHAPES: [&str; 6] = [
    "Point",
    "LineString",
    "Polygon",
    "MultiPoint",
    "MultiLineString",
    "MultiPolygon",
];

/// A rule's examples file.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AuditExamples {
    /// `kentos.auditExamples`.
    pub format: String,
    /// 1.
    pub schema_version: u32,
    /// One to ten examples.
    pub examples: Vec<AuditExample>,
}

/// One example: what the rule is given and what it must give.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AuditExample {
    /// What it shows, in a few words ("8 m² yatak odası").
    pub title: String,
    /// Objects as data (`{layer, geometry, attrs?}`, GeoJSON shapes, at most 500): a synthetic drawing is made of them
    /// and measured (`facts.layers`, `facts.project`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(
        feature = "schema",
        schemars(with = "Vec<serde_json::Map<String, Value>>")
    )]
    pub features: Vec<Value>,
    /// What KentOS measures in an architectural drawing, given as data: `facts.project.architecture`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub architecture: Option<ArchitectureFacts>,
    pub expect: Expected,
}

/// The findings an example must give.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Expected {
    /// How many findings, exactly (0: the rule finds nothing).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<u64>,
    /// Findings it must have, each matched by one of the rule's.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub has: Vec<ExpectedFinding>,
}

/// A finding the rule must give: every key given must match.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExpectedFinding {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub severity: Option<Severity>,
    /// Words its message holds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// The example's object it names (`featureId`), by its place in `features` from 0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feature: Option<usize>,
    /// The command its fix runs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fix: Option<String>,
}

/// A finding's severity, as a rule gives it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

impl Severity {
    /// As a rule writes it.
    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "info",
        }
    }
}

/// The examples file `path` of rule `rule` read and checked: the examples, or each problem with its place.
pub fn read(path: &str, rule: &str, data: &[u8]) -> Result<AuditExamples, Vec<Problem>> {
    let problem = |at: String, why: String| {
        Problem::new(
            "audit_examples",
            format!("“{rule}” kuralının örnekleri: {why}"),
        )
        .file(path)
        .at(at)
    };
    let text = std::str::from_utf8(data)
        .map_err(|_| vec![problem(String::new(), "dosya UTF-8 değil.".to_owned())])?;
    let file: AuditExamples = serde_json::from_str(text).map_err(|e| {
        let said = e.to_string();
        let said = said.split(" at line ").next().unwrap_or_default();
        vec![problem(
            format!("satır {}", e.line()),
            format!("{said}. Biçim: {{\"format\": \"{FORMAT}\", \"schemaVersion\": 1, \"examples\": [ … ]}}."),
        )]
    })?;
    let mut out = Vec::new();
    if file.format != FORMAT || file.schema_version != 1 {
        out.push(problem(
            "/format".to_owned(),
            format!("“{FORMAT}” sürüm 1 değil (format, schemaVersion)."),
        ));
    }
    if file.examples.is_empty() || file.examples.len() > MOST_EXAMPLES {
        out.push(problem(
            "/examples".to_owned(),
            format!(
                "1 ile {MOST_EXAMPLES} arasında örnek olmalı ({} var).",
                file.examples.len()
            ),
        ));
    }
    for (i, example) in file.examples.iter().enumerate() {
        let at = format!("/examples/{i}");
        if example.title.trim().is_empty() {
            out.push(problem(
                format!("{at}/title"),
                "örneğin adı (title) boş.".to_owned(),
            ));
        }
        if example.features.len() > MOST_FEATURES {
            out.push(problem(
                format!("{at}/features"),
                format!(
                    "en çok {MOST_FEATURES} nesne ({} var).",
                    example.features.len()
                ),
            ));
        }
        for (j, feature) in example.features.iter().enumerate() {
            if let Some(why) = feature_problem(feature) {
                out.push(problem(format!("{at}/features/{j}"), why));
            }
        }
        let expect = &example.expect;
        if expect.count.is_none() && expect.has.is_empty() {
            out.push(problem(
                format!("{at}/expect"),
                "beklenen yok: bulgu sayısını (count) ya da bulunması gereken bulguları (has) yazın.".to_owned(),
            ));
        }
        if let Some(n) = expect.count
            && (expect.has.len() as u64) > n
        {
            out.push(problem(
                format!("{at}/expect/has"),
                format!(
                    "{} bulgu bekleniyor ama sayı (count) {n}.",
                    expect.has.len()
                ),
            ));
        }
        for (k, want) in expect.has.iter().enumerate() {
            let place = format!("{at}/expect/has/{k}");
            if want.severity.is_none()
                && want.message.is_none()
                && want.feature.is_none()
                && want.fix.is_none()
            {
                out.push(problem(
                    place.clone(),
                    "bulgu boş: önemini (severity), sözünü (message), nesnesini (feature) ya da düzeltmesini (fix) \
                     yazın."
                        .to_owned(),
                ));
            }
            if let Some(n) = want.feature
                && n >= example.features.len()
            {
                out.push(problem(
                    format!("{place}/feature"),
                    format!(
                        "sırası {} olan nesne yok: features'ın sırası 0'dan başlar ({} nesne var).",
                        n,
                        example.features.len()
                    ),
                ));
            }
        }
    }
    if out.is_empty() { Ok(file) } else { Err(out) }
}

/// Why an example's object is not one: a layer by name and a GeoJSON shape (its coordinates are read when it runs).
fn feature_problem(feature: &Value) -> Option<String> {
    let Some(object) = feature.as_object() else {
        return Some("nesne bir JSON nesnesi değil ({layer, geometry, attrs?}).".to_owned());
    };
    if let Some(key) = object
        .keys()
        .find(|k| !matches!(k.as_str(), "layer" | "geometry" | "attrs"))
    {
        return Some(format!(
            "“{key}” bilinmiyor: yalnız layer, geometry ve attrs."
        ));
    }
    let layer = object.get("layer").and_then(Value::as_str).map(str::trim);
    if layer.is_none_or(|l| l.is_empty() || l.chars().count() > 80) {
        return Some("katmanın adı (layer, en çok 80 karakter) gerekli.".to_owned());
    }
    let shape = object
        .get("geometry")
        .and_then(|g| g.get("type"))
        .and_then(Value::as_str);
    if !shape.is_some_and(|s| SHAPES.contains(&s)) {
        return Some(format!(
            "geometry bir GeoJSON şekli değil (type: {}).",
            SHAPES.join(", ")
        ));
    }
    if object.get("attrs").is_some_and(|a| !a.is_object()) {
        return Some("attrs bir JSON nesnesi olmalı.".to_owned());
    }
    None
}

/// The examples file's JSON Schema, made from its types and committed as the SDK's `auditExamplesSchema` part
/// (`sdk/kentos-audit-examples.schema.json`).
#[cfg(feature = "schema")]
pub fn json_schema() -> Value {
    use serde_json::json;
    let mut schema = schemars::schema_for!(AuditExamples).to_value();
    if let Some(o) = schema.as_object_mut() {
        o.insert(
            "title".into(),
            Value::from("KentOS audit rule examples (kentos.auditExamples 1)"),
        );
        if let Some(Value::Object(props)) = o.get_mut("properties") {
            props.insert("format".into(), json!({ "const": FORMAT }));
            props.insert("schemaVersion".into(), json!({ "const": 1 }));
            props.insert(
                "examples".into(),
                json!({ "type": "array", "minItems": 1, "maxItems": MOST_EXAMPLES,
                        "items": { "$ref": "#/$defs/AuditExample" } }),
            );
        }
    }
    schema
}

#[cfg(test)]
mod tests;
