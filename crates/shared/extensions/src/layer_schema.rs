//! An extension's layer schema, `schema.json` (docs/adr/8057 §23): the layers it needs with their fields, the value
//! lists the fields link and the relations between the layers, set up on the drawing by the extension's own command
//! (`ctx.apply_declarative_schema`, kentos.ext) as one undo step. Checked here when the package is read, each problem
//! with its JSON pointer, so that a broken schema is said by the inspector, not by a half-made drawing.
//!
//! The words are KentOS's own: a layer's `geometry` is a layer type (`point`, `line`, `area`, `annotation`, `mixed`),
//! a field is a layer field (`name`, `kind`, `alias`, `required`, `default`, `length`, `min`, `max`, `domain` …), a
//! value list is `cad.domains.edit`'s, a relation `cad.relations.edit`'s. A layer's `name` is its id in the drawing:
//! the schema finds it again on the next run, as the template's layers are found by theirs (`parsel`).

use std::collections::BTreeSet;

use serde_json::{Map, Value};

use crate::problem::Problem;

/// `schema.json`'s `format`, when given.
pub const FORMAT: &str = "kentos.layerSchema";
/// The version this KentOS sets up; `version`, when given, may not be later.
pub const VERSION: u64 = 1;

/// A layer's types (`cad.layers.edit`'s `geometry`).
pub const GEOMETRIES: [&str; 5] = ["point", "line", "area", "annotation", "mixed"];
/// What a field holds (KentOS's `LayerFieldKind`).
pub const FIELD_KINDS: [&str; 12] = [
    "text", "integer", "decimal", "date", "boolean", "datetime", "time", "bigint", "instant",
    "uuid", "json", "binary",
];
/// What a relation does to its children when their parent is deleted.
pub const ON_DELETE: [&str; 4] = ["keep", "unlink", "cascade", "restrict"];

const TOP: [&str; 6] = [
    "format",
    "version",
    "group",
    "domains",
    "layers",
    "relations",
];
const LAYER: [&str; 4] = ["name", "title", "geometry", "fields"];
const FIELD: [&str; 12] = [
    "name",
    "kind",
    "alias",
    "required",
    "default",
    "length",
    "min",
    "max",
    "minExclusive",
    "maxExclusive",
    "domain",
    "multiple",
];
const DOMAIN: [&str; 10] = [
    "id",
    "name",
    "kind",
    "values",
    "min",
    "max",
    "minExclusive",
    "maxExclusive",
    "unit",
    "description",
];
const CHOICE: [&str; 3] = ["code", "label", "description"];
const RELATION: [&str; 6] = ["id", "name", "parent", "child", "field", "onDelete"];

/// A layer's schema name: lowercase letters, digits and `_`, a letter first, at most 63 (`mezar_parsel`).
fn is_layer_name(name: &str) -> bool {
    let b = name.as_bytes();
    !b.is_empty()
        && b.len() <= 63
        && b[0].is_ascii_lowercase()
        && b.iter()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == b'_')
}

/// A value list's or a relation's id: ASCII letters, digits, `-`, `_` and `.`, 1–64.
fn is_id(id: &str) -> bool {
    (1..=64).contains(&id.len())
        && id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_' | b'.'))
}

struct Check<'a> {
    file: &'a str,
    problems: Vec<Problem>,
    warnings: Vec<Problem>,
}

impl Check<'_> {
    fn add(&mut self, at: String, message: String) {
        self.problems.push(
            Problem::new("layer_schema", message)
                .file(self.file.to_owned())
                .at(at),
        );
    }

    /// The object's keys outside `known`.
    fn keys(&mut self, at: &str, object: &Map<String, Value>, known: &[&str]) {
        for key in object.keys().filter(|k| !known.contains(&k.as_str())) {
            self.add(
                format!("{at}/{key}"),
                format!(
                    "“{key}” bu şemada bilinmeyen bir alan; olanlar: {}.",
                    known.join(", ")
                ),
            );
        }
    }

    /// A text that must be there and not blank.
    fn text<'v>(&mut self, at: &str, object: &'v Map<String, Value>, key: &str) -> Option<&'v str> {
        match object.get(key) {
            Some(Value::String(s)) if !s.trim().is_empty() => Some(s),
            Some(_) => {
                self.add(
                    format!("{at}/{key}"),
                    format!("“{key}” boş olmayan bir metin olmalı."),
                );
                None
            }
            None => {
                self.add(at.to_owned(), format!("“{key}” gerekli."));
                None
            }
        }
    }

    /// A text that may be left out; when given, not blank.
    fn optional_text(&mut self, at: &str, object: &Map<String, Value>, key: &str) {
        if let Some(value) = object.get(key)
            && !value.as_str().is_some_and(|s| !s.trim().is_empty())
        {
            self.add(
                format!("{at}/{key}"),
                format!("“{key}” boş olmayan bir metin olmalı."),
            );
        }
    }

    fn boolean(&mut self, at: &str, object: &Map<String, Value>, key: &str) {
        if object.get(key).is_some_and(|v| !v.is_boolean()) {
            self.add(
                format!("{at}/{key}"),
                format!("“{key}” true ya da false olmalı."),
            );
        }
    }

    /// An array that may be left out; its items, each an object.
    fn objects<'v>(
        &mut self,
        at: &str,
        value: Option<&'v Value>,
    ) -> Vec<(String, &'v Map<String, Value>)> {
        let Some(value) = value else {
            return Vec::new();
        };
        let Some(items) = value.as_array() else {
            self.add(at.to_owned(), "Bir liste ([…]) olmalı.".to_owned());
            return Vec::new();
        };
        let mut out = Vec::new();
        for (i, item) in items.iter().enumerate() {
            match item.as_object() {
                Some(object) => out.push((format!("{at}/{i}"), object)),
                None => self.add(format!("{at}/{i}"), "Bir nesne ({…}) olmalı.".to_owned()),
            }
        }
        out
    }
}

/// What is wrong with `value`, `schema.json` read as JSON (`file` its path in the package): problems, and warnings
/// (a relation naming a layer the schema does not make, which the project must then have).
pub fn check(file: &str, value: &Value) -> (Vec<Problem>, Vec<Problem>) {
    let mut c = Check {
        file,
        problems: Vec::new(),
        warnings: Vec::new(),
    };
    let Some(top) = value.as_object() else {
        c.add(
            String::new(),
            "Katman şeması bir JSON nesnesi ({…}) olmalı.".to_owned(),
        );
        return (c.problems, c.warnings);
    };
    c.keys("", top, &TOP);
    if let Some(format) = top.get("format")
        && format.as_str() != Some(FORMAT)
    {
        c.add("/format".to_owned(), format!("Biçim “{FORMAT}” olmalı."));
    }
    if let Some(version) = top.get("version") {
        match version.as_u64() {
            Some(v) if (1..=VERSION).contains(&v) => {}
            Some(v) if v > VERSION => c.add(
                "/version".to_owned(),
                format!("Şema sürüm {v}; bu KentOS {VERSION}. sürümü kurar: daha yeni bir KentOS gerekir."),
            ),
            _ => c.add("/version".to_owned(), format!("Sürüm 1 ile {VERSION} arasında bir tam sayı olmalı.")),
        }
    }
    if let Some(group) = top.get("group") {
        match group.as_object() {
            Some(g) => {
                c.keys("/group", g, &["id", "name"]);
                if let Some(id) = c.text("/group", g, "id")
                    && !is_layer_name(id)
                {
                    c.add(
                        "/group/id".to_owned(),
                        format!("“{id}” bir grup kimliği değil: küçük harf, rakam ve _, harfle başlar (ör. mezarlik)."),
                    );
                }
                c.optional_text("/group", g, "name");
            }
            None => c.add(
                "/group".to_owned(),
                "Grup bir nesne ({\"id\", \"name\"}) olmalı.".to_owned(),
            ),
        }
    }
    // The value lists, first: the fields link them.
    let mut domains = BTreeSet::new();
    for (at, d) in c.objects("/domains", top.get("domains")) {
        c.keys(&at, d, &DOMAIN);
        if let Some(id) = c.text(&at, d, "id") {
            if !is_id(id) {
                c.add(
                    format!("{at}/id"),
                    format!(
                        "“{id}” bir değer listesi kimliği değil: harf, rakam, -, _ ve ., en çok 64."
                    ),
                );
            } else if !domains.insert(id.to_owned()) {
                c.add(
                    format!("{at}/id"),
                    format!("“{id}” kimlikli iki değer listesi var."),
                );
            }
        }
        c.optional_text(&at, d, "name");
        if let Some(kind) = d.get("kind")
            && !kind.as_str().is_some_and(|k| FIELD_KINDS.contains(&k))
        {
            c.add(
                format!("{at}/kind"),
                format!("Tür şunlardan biri olmalı: {}.", FIELD_KINDS.join(", ")),
            );
        }
        let values = c.objects(&format!("{at}/values"), d.get("values"));
        let ranged = d.contains_key("min") || d.contains_key("max");
        if values.is_empty() && !ranged {
            c.add(
                at.clone(),
                "Değer listesi değerler (values) ya da aralık (min, max) ister.".to_owned(),
            );
        }
        let mut codes = BTreeSet::new();
        for (vat, v) in values {
            c.keys(&vat, v, &CHOICE);
            if let Some(code) = c.text(&vat, v, "code")
                && !codes.insert(code.to_owned())
            {
                c.add(
                    format!("{vat}/code"),
                    format!("“{code}” kodu listede iki kez."),
                );
            }
            c.text(&vat, v, "label");
        }
        for key in ["min", "max"] {
            if d.get(key).is_some_and(|v| !v.is_string()) {
                c.add(
                    format!("{at}/{key}"),
                    format!("“{key}” metin olarak yazılır (\"0\", \"10,5\" değil \"10.5\")."),
                );
            }
        }
        c.boolean(&at, d, "minExclusive");
        c.boolean(&at, d, "maxExclusive");
    }
    // The layers and their fields.
    let layers_at = "/layers";
    let layer_items = match top.get("layers") {
        Some(v) => c.objects(layers_at, Some(v)),
        None => {
            c.add(
                String::new(),
                "“layers” gerekli: en az bir katman.".to_owned(),
            );
            Vec::new()
        }
    };
    if top
        .get("layers")
        .and_then(Value::as_array)
        .is_some_and(Vec::is_empty)
    {
        c.add(layers_at.to_owned(), "En az bir katman olmalı.".to_owned());
    }
    let mut layers: Vec<(String, BTreeSet<String>)> = Vec::new();
    for (at, l) in layer_items {
        c.keys(&at, l, &LAYER);
        let name = c.text(&at, l, "name").map(str::to_owned);
        if let Some(name) = &name {
            if !is_layer_name(name) {
                c.add(
                    format!("{at}/name"),
                    format!("“{name}” bir katman şeması adı değil: küçük harf, rakam ve _, harfle başlar (ör. mezar_parsel)."),
                );
            } else if layers.iter().any(|(n, _)| n == name) {
                c.add(
                    format!("{at}/name"),
                    format!("“{name}” adlı iki katman var."),
                );
            }
        }
        c.optional_text(&at, l, "title");
        if let Some(geometry) = l.get("geometry")
            && !geometry.as_str().is_some_and(|g| GEOMETRIES.contains(&g))
        {
            c.add(
                format!("{at}/geometry"),
                format!(
                    "Katman türü şunlardan biri olmalı: {}.",
                    GEOMETRIES.join(", ")
                ),
            );
        }
        let mut fields = BTreeSet::new();
        for (fat, f) in c.objects(&format!("{at}/fields"), l.get("fields")) {
            c.keys(&fat, f, &FIELD);
            if let Some(field) = c.text(&fat, f, "name")
                && !fields.insert(field.to_lowercase())
            {
                c.add(
                    format!("{fat}/name"),
                    format!("“{field}” alanı katmanda iki kez."),
                );
            }
            match f.get("kind").and_then(Value::as_str) {
                Some(kind) if FIELD_KINDS.contains(&kind) => {}
                Some(_) | None => c.add(
                    format!("{fat}/kind"),
                    format!(
                        "Alanın türü (kind) şunlardan biri olmalı: {}.",
                        FIELD_KINDS.join(", ")
                    ),
                ),
            }
            if let Some(domain) = f.get("domain") {
                match domain.as_str() {
                    Some(id) if domains.contains(id) => {}
                    Some(id) => c.add(
                        format!("{fat}/domain"),
                        format!("“{id}” değer listesi bu şemanın domains listesinde yok."),
                    ),
                    None => c.add(
                        format!("{fat}/domain"),
                        "Değer listesinin kimliği metin olmalı.".to_owned(),
                    ),
                }
            }
            c.optional_text(&fat, f, "alias");
            c.boolean(&fat, f, "required");
            c.boolean(&fat, f, "multiple");
            for key in ["default", "min", "max"] {
                if f.get(key).is_some_and(|v| !v.is_string()) {
                    c.add(
                        format!("{fat}/{key}"),
                        format!("“{key}” metin olarak yazılır."),
                    );
                }
            }
            if let Some(length) = f.get("length")
                && !length.as_u64().is_some_and(|n| (1..=10_000).contains(&n))
            {
                c.add(
                    format!("{fat}/length"),
                    "Uzunluk 1 ile 10 000 arasında bir tam sayı olmalı.".to_owned(),
                );
            }
        }
        if let Some(name) = name {
            layers.push((name, fields));
        }
    }
    // The relations: a child's field holds its parent's persistent id.
    let mut relations = BTreeSet::new();
    for (at, r) in c.objects("/relations", top.get("relations")) {
        c.keys(&at, r, &RELATION);
        if let Some(id) = c.text(&at, r, "id") {
            if !is_id(id) {
                c.add(
                    format!("{at}/id"),
                    format!("“{id}” bir ilişki kimliği değil: harf, rakam, -, _ ve ., en çok 64."),
                );
            } else if !relations.insert(id.to_owned()) {
                c.add(
                    format!("{at}/id"),
                    format!("“{id}” kimlikli iki ilişki var."),
                );
            }
        }
        c.optional_text(&at, r, "name");
        let field = c.text(&at, r, "field").map(str::to_owned);
        for side in ["parent", "child"] {
            let Some(layer) = c.text(&at, r, side) else {
                continue;
            };
            match layers.iter().find(|(n, _)| n == layer) {
                Some((_, fields)) => {
                    if side == "child"
                        && let Some(field) = &field
                        && !fields.contains(&field.to_lowercase())
                    {
                        c.add(format!("{at}/field"), format!("“{field}” alanı “{layer}” katmanının alanlarında yok."));
                    }
                }
                None => c.warnings.push(
                    Problem::new(
                        "layer_schema_outside",
                        format!(
                            "“{layer}” katmanını şema kurmuyor: kurulurken projede bu kimlikle bir katman bulunmalı \
                             (ör. şablonun parsel katmanı)."
                        ),
                    )
                    .file(file.to_owned())
                    .at(format!("{at}/{side}")),
                ),
            }
        }
        if let Some(on_delete) = r.get("onDelete")
            && !on_delete.as_str().is_some_and(|o| ON_DELETE.contains(&o))
        {
            c.add(
                format!("{at}/onDelete"),
                format!("Şunlardan biri olmalı: {}.", ON_DELETE.join(", ")),
            );
        }
    }
    (c.problems, c.warnings)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn mebis() -> Value {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../fixtures/extensions/v1/mebis/schema.json");
        serde_json::from_str(&std::fs::read_to_string(path).expect("the fixture")).expect("JSON")
    }

    #[test]
    fn the_reference_schema_passes() {
        let (problems, warnings) = check("schema.json", &mebis());
        assert!(problems.is_empty(), "{problems:#?}");
        assert!(warnings.is_empty(), "{warnings:#?}");
    }

    #[test]
    fn each_mistake_is_said_where_it_is() {
        let mut s = mebis();
        s["layers"][0]["geometry"] = json!("polygon");
        s["layers"][1]["fields"][0]["kind"] = json!("string");
        s["layers"][2]["fields"][2]["domain"] = json!("yok");
        s["layers"][3]["name"] = json!("Defin Kaydi");
        s["relations"][0]["field"] = json!("mezar_kimligi");
        s["domains"][0]["values"][1]["code"] = json!("BOS");
        s["renk"] = json!("mavi");
        let (problems, _) = check("schema.json", &s);
        let at: Vec<&str> = problems.iter().filter_map(|p| p.at.as_deref()).collect();
        assert_eq!(
            at,
            [
                "/renk",
                "/domains/0/values/1/code",
                "/layers/0/geometry",
                "/layers/1/fields/0/kind",
                "/layers/2/fields/2/domain",
                "/layers/3/name",
            ],
            "{problems:#?}"
        );
        // The relation's child is now a layer the schema does not make: a warning, and its field not checked.
        let (_, warnings) = check("schema.json", &s);
        assert_eq!(
            warnings.iter().map(|w| w.code).collect::<Vec<_>>(),
            ["layer_schema_outside"]
        );
        let newer = json!({"version": 2, "layers": [{"name": "a"}]});
        let (problems, _) = check("schema.json", &newer);
        assert!(
            problems[0].message.contains("daha yeni bir KentOS"),
            "{problems:#?}"
        );
    }
}
