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
const LAYER: [&str; 7] = [
    "name",
    "title",
    "geometry",
    "fields",
    "form",
    "subtypes",
    "contingents",
];
/// A layer holds at most this many subtypes; value groups, a group's fields and rows (KentOS's own bounds,
/// docs/adr/0422, 0423).
const MOST_SUBTYPES: usize = 256;
const MOST_GROUPS: usize = 50;
const MOST_GROUP_FIELDS: usize = 8;
const MOST_COMBINATIONS: usize = 10_000;
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

/// A layer's field as its subtypes and value groups name it: one of its fields with a value list of this schema,
/// taking one value; its codes. Otherwise why it cannot be.
fn listed_field<'a>(
    listed: &'a [(String, Option<BTreeSet<String>>, bool)],
    field: &str,
) -> Result<&'a BTreeSet<String>, &'static str> {
    match listed.iter().find(|(n, ..)| n == field) {
        None => Err("katmanın alanlarında yok"),
        Some((_, None, _)) => {
            Err("değer listeli bir alan değil (domain: bu şemanın değerli bir listesi)")
        }
        Some((_, Some(_), true)) => Err("çok değerli (multiple): tek değerli olmalı"),
        Some((_, Some(codes), false)) => Ok(codes),
    }
}

/// A layer's `subtypes` (KentOS's `LayerSubtypes`, docs/adr/0422): the field that sorts its records, a listed
/// single-valued one, and its subtypes, each a code of that field's list narrowing the other fields (`values` of
/// their lists, `default`, `required`). The subtypes' codes, for the value groups' rows.
fn subtypes_check(
    c: &mut Check<'_>,
    at: &str,
    layer: &Map<String, Value>,
    listed: &[(String, Option<BTreeSet<String>>, bool)],
) -> BTreeSet<String> {
    let mut codes = BTreeSet::new();
    let Some(given) = layer.get("subtypes") else {
        return codes;
    };
    let sat = format!("{at}/subtypes");
    let Some(st) = given.as_object() else {
        c.add(
            sat,
            "Alt türler bir nesne ({\"field\", \"types\"}) olmalı.".to_owned(),
        );
        return codes;
    };
    c.keys(&sat, st, &["field", "types"]);
    let sorter = c.text(&sat, st, "field").map(str::to_owned);
    let sorter_codes = match sorter.as_deref().map(|f| (f, listed_field(listed, f))) {
        Some((_, Ok(codes))) => Some(codes),
        Some((f, Err(why))) => {
            c.add(
                format!("{sat}/field"),
                format!("“{f}” alt tür alanı olamaz: {why}."),
            );
            None
        }
        None => None,
    };
    let types = c.objects(&format!("{sat}/types"), st.get("types"));
    if types.is_empty() || types.len() > MOST_SUBTYPES {
        c.add(
            format!("{sat}/types"),
            format!("1 ile {MOST_SUBTYPES} arasında alt tür (types) olmalı."),
        );
    }
    for (tat, t) in types {
        c.keys(&tat, t, &["code", "label", "fields"]);
        if let Some(code) = c.text(&tat, t, "code") {
            if !codes.insert(code.to_owned()) {
                c.add(format!("{tat}/code"), format!("“{code}” iki alt türde."));
            } else if let (Some(known), Some(f)) = (sorter_codes, &sorter)
                && !known.contains(code)
            {
                c.add(
                    format!("{tat}/code"),
                    format!("“{code}” alt tür alanının (“{f}”) değer listesinde yok."),
                );
            }
        }
        c.optional_text(&tat, t, "label");
        for (rat, r) in c.objects(&format!("{tat}/fields"), t.get("fields")) {
            c.keys(&rat, r, &["field", "values", "default", "required"]);
            if let Some(field) = c.text(&rat, r, "field") {
                if sorter.as_deref() == Some(field) {
                    c.add(
                        format!("{rat}/field"),
                        "Alt tür alanı kendini daraltamaz.".to_owned(),
                    );
                } else if !listed.iter().any(|(n, ..)| n == field) {
                    c.add(
                        format!("{rat}/field"),
                        format!("“{field}” alanı katmanın alanlarında yok."),
                    );
                } else if let Some(values) = r.get("values") {
                    match (values.as_array(), listed_field(listed, field)) {
                        (None, _) => c.add(
                            format!("{rat}/values"),
                            "values kodların listesi olmalı.".to_owned(),
                        ),
                        (Some(_), Err(why)) => c.add(
                            format!("{rat}/values"),
                            format!("values yalnız değer listeli alanı daraltır: “{field}” {why}."),
                        ),
                        (Some(values), Ok(known)) => {
                            for (j, v) in values.iter().enumerate() {
                                if !v.as_str().is_some_and(|v| known.contains(v)) {
                                    c.add(
                                        format!("{rat}/values/{j}"),
                                        format!("“{field}” alanının değer listesinde olan bir kod olmalı."),
                                    );
                                }
                            }
                        }
                    }
                }
            }
            if r.get("default").is_some_and(|v| !v.is_string()) {
                c.add(
                    format!("{rat}/default"),
                    "“default” metin olarak yazılır.".to_owned(),
                );
            }
            c.boolean(&rat, r, "required");
        }
    }
    codes
}

/// A layer's `contingents` (KentOS's value groups, docs/adr/0423): each 2–8 listed single-valued fields whose values go
/// together, its rows the combinations allowed (a cell each: `code` of the field's list, `any`, `empty`; a row may be
/// a subtype's).
fn contingents_check(
    c: &mut Check<'_>,
    at: &str,
    layer: &Map<String, Value>,
    listed: &[(String, Option<BTreeSet<String>>, bool)],
    subtypes: &BTreeSet<String>,
) {
    let groups = c.objects(&format!("{at}/contingents"), layer.get("contingents"));
    if groups.len() > MOST_GROUPS {
        c.add(
            format!("{at}/contingents"),
            format!("Katmanda en çok {MOST_GROUPS} değer grubu olur."),
        );
    }
    let mut names = BTreeSet::new();
    for (gat, g) in groups {
        c.keys(&gat, g, &["name", "fields", "restrictive", "rows"]);
        if let Some(name) = c.text(&gat, g, "name")
            && !names.insert(name.to_owned())
        {
            c.add(
                format!("{gat}/name"),
                format!("“{name}” adlı iki değer grubu var."),
            );
        }
        let given = g.get("fields").and_then(Value::as_array);
        let fields: Vec<&str> = given
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect();
        let distinct = fields.iter().collect::<BTreeSet<_>>().len() == fields.len();
        if given.is_none_or(|g| g.len() != fields.len())
            || !(2..=MOST_GROUP_FIELDS).contains(&fields.len())
            || !distinct
        {
            c.add(
                format!("{gat}/fields"),
                format!(
                    "Değer grubu 2 ile {MOST_GROUP_FIELDS} arasında farklı alan (fields) ister."
                ),
            );
        }
        let mut codes: Vec<Option<&BTreeSet<String>>> = Vec::new();
        for (j, f) in fields.iter().enumerate() {
            match listed_field(listed, f) {
                Ok(known) => codes.push(Some(known)),
                Err(why) => {
                    c.add(
                        format!("{gat}/fields/{j}"),
                        format!("“{f}” değer grubunda olamaz: {why}."),
                    );
                    codes.push(None);
                }
            }
        }
        c.boolean(&gat, g, "restrictive");
        let rows = c.objects(&format!("{gat}/rows"), g.get("rows"));
        // Whether some field can be written first, the others empty: a row allowing it there and empty elsewhere.
        let first = rows.iter().any(|(_, r)| {
            let kinds: Vec<&str> = r
                .get("cells")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .map(|cell| cell.get("kind").and_then(Value::as_str).unwrap_or_default())
                .collect();
            kinds.len() == fields.len()
                && (0..kinds.len()).any(|j| {
                    matches!(kinds[j], "code" | "any")
                        && kinds
                            .iter()
                            .enumerate()
                            .all(|(i, k)| i == j || matches!(*k, "empty" | "any"))
                })
        });
        if g.get("restrictive").and_then(Value::as_bool) == Some(true) && !rows.is_empty() && !first
        {
            let name = g.get("name").and_then(Value::as_str).unwrap_or_default();
            c.warnings.push(
                Problem::new(
                    "layer_schema_fill",
                    format!(
                        "“{name}” kısıtlayıcı ve hiçbir satırı alanlarını tek tek doldurmaya izin vermiyor: öznitelik \
                         tablosunda bir alan yazılınca öbürleri boş olduğu için reddedilir. Boş hücreli satırlar ekleyin \
                         (ör. [{{\"kind\": \"any\"}}, {{\"kind\": \"empty\"}}] ve tersi) ya da grubu uyarıcı yapın \
                         (restrictive: false)."
                    ),
                )
                .file(c.file.to_owned())
                .at(gat.clone()),
            );
        }
        if rows.is_empty() || rows.len() > MOST_COMBINATIONS {
            c.add(
                format!("{gat}/rows"),
                format!("1 ile {MOST_COMBINATIONS} arasında bileşim (rows) olmalı."),
            );
        }
        for (rat, r) in rows {
            c.keys(&rat, r, &["cells", "subtype"]);
            let cells = c.objects(&format!("{rat}/cells"), r.get("cells"));
            if cells.len() != fields.len() {
                c.add(
                    format!("{rat}/cells"),
                    format!(
                        "Bileşim grubun her alanı için bir hücre ister ({}).",
                        fields.len()
                    ),
                );
            }
            for (k, (cat, cell)) in cells.into_iter().enumerate() {
                c.keys(&cat, cell, &["kind", "code"]);
                match cell.get("kind").and_then(Value::as_str) {
                    Some("any" | "empty") => {}
                    Some("code") => {
                        if let Some(code) = c.text(&cat, cell, "code")
                            && let Some(Some(known)) = codes.get(k)
                            && !known.contains(code)
                        {
                            c.add(
                                format!("{cat}/code"),
                                format!("“{code}” “{}” alanının değer listesinde yok.", fields[k]),
                            );
                        }
                    }
                    _ => c.add(
                        format!("{cat}/kind"),
                        "Hücrenin türü (kind) code, any ya da empty olmalı.".to_owned(),
                    ),
                }
            }
            if let Some(subtype) = r.get("subtype")
                && !subtype.as_str().is_some_and(|s| subtypes.contains(s))
            {
                c.add(
                    format!("{rat}/subtype"),
                    "Bileşimin alt türü katmanın alt türlerinden biri olmalı.".to_owned(),
                );
            }
        }
    }
}

/// What is wrong with `value`, `schema.json` read as JSON (`file` its path in the package; `forms` the form packages
/// the manifest declares, `entrypoints.forms`' definitions): problems, and warnings (a relation naming a layer the
/// schema does not make, which the project must then have).
pub fn check(file: &str, value: &Value, forms: &[&str]) -> (Vec<Problem>, Vec<Problem>) {
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
    // The value lists, first: the fields link them. A list's codes: what a subtype or a value group may name.
    let mut domains = BTreeSet::new();
    let mut domain_codes: std::collections::BTreeMap<String, BTreeSet<String>> = Default::default();
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
        if let Some(id) = d.get("id").and_then(Value::as_str)
            && !codes.is_empty()
        {
            domain_codes.insert(id.to_owned(), codes);
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
        // The layer's form: one the manifest declares, so the inspector reads it as a form package.
        if let Some(form) = l.get("form") {
            match form.as_str() {
                Some(path) if forms.contains(&path) => {}
                Some(path) => c.add(
                    format!("{at}/form"),
                    format!(
                        "“{path}” manifestin entrypoints.forms listesinde bildirilmiş bir form değil: formu orada \
                         bildirin (dosyası paket incelemesinde denetlenir)."
                    ),
                ),
                None => c.add(format!("{at}/form"), "Form, paketteki .kform dosyasının yolu olmalı.".to_owned()),
            }
        }
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
        // Each field's name, its value list's codes (none: not a listed field) and whether it takes many values.
        let mut listed: Vec<(String, Option<BTreeSet<String>>, bool)> = Vec::new();
        for (fat, f) in c.objects(&format!("{at}/fields"), l.get("fields")) {
            c.keys(&fat, f, &FIELD);
            if let Some(field) = f.get("name").and_then(Value::as_str) {
                let codes = f
                    .get("domain")
                    .and_then(Value::as_str)
                    .and_then(|d| domain_codes.get(d))
                    .cloned();
                let many = f.get("multiple").and_then(Value::as_bool) == Some(true);
                listed.push((field.to_owned(), codes, many));
            }
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
        let subtypes = subtypes_check(&mut c, &at, l, &listed);
        contingents_check(&mut c, &at, l, &listed, &subtypes);
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

    const FORMS: [&str; 2] = ["forms/mezar_yeri.kform", "forms/defin_kaydi.kform"];

    #[test]
    fn the_reference_schema_passes() {
        let (problems, warnings) = check("schema.json", &mebis(), &FORMS);
        assert!(problems.is_empty(), "{problems:#?}");
        assert!(warnings.is_empty(), "{warnings:#?}");
    }

    /// MEBİS's schema with a subtype field and a value group on its graves (docs/adr/0422, 0423).
    fn with_subtypes() -> Value {
        let mut s = mebis();
        s["domains"].as_array_mut().unwrap().push(json!({
            "id": "mezar_turu", "name": "Mezar türü", "kind": "text",
            "values": [{ "code": "TEKLI", "label": "Tekli" }, { "code": "AILE", "label": "Aile" }]
        }));
        let yeri = &mut s["layers"][2];
        yeri["fields"]
            .as_array_mut()
            .unwrap()
            .push(json!({ "name": "tur", "kind": "text", "domain": "mezar_turu" }));
        yeri["subtypes"] = json!({ "field": "tur", "types": [
            { "code": "TEKLI", "label": "Tekli mezar",
              "fields": [{ "field": "durum", "values": ["BOS", "DOLU"], "default": "BOS", "required": true }] },
            { "code": "AILE" }
        ]});
        yeri["contingents"] = json!([{ "name": "Tür ve durum", "fields": ["tur", "durum"], "restrictive": true,
            "rows": [
                { "cells": [{ "kind": "code", "code": "TEKLI" }, { "kind": "any" }] },
                { "cells": [{ "kind": "code", "code": "AILE" }, { "kind": "code", "code": "AYRILMIS" }],
                  "subtype": "AILE" }
            ] }]);
        s
    }

    #[test]
    fn subtypes_and_value_groups_pass_in_kentos_words_and_each_mistake_is_said_where_it_is() {
        let (problems, warnings) = check("schema.json", &with_subtypes(), &FORMS);
        assert!(problems.is_empty(), "{problems:#?}");
        assert!(
            warnings.is_empty(),
            "(TEKLI, any) lets the type be written first: {warnings:#?}"
        );
        // A restrictive group whose every row fills both fields cannot be filled a field at a time (the attribute
        // table writes one): warned (real window, Vitrin 1.0.16's first schema).
        let mut full = with_subtypes();
        full["layers"][2]["contingents"][0]["rows"] = json!([
            { "cells": [{ "kind": "code", "code": "TEKLI" }, { "kind": "code", "code": "BOS" }] },
            { "cells": [{ "kind": "code", "code": "AILE" }, { "kind": "code", "code": "DOLU" }] }
        ]);
        let (problems, warnings) = check("schema.json", &full, &FORMS);
        assert!(problems.is_empty(), "{problems:#?}");
        assert_eq!(
            warnings
                .iter()
                .map(|w| (w.code, w.at.as_deref()))
                .collect::<Vec<_>>(),
            [("layer_schema_fill", Some("/layers/2/contingents/0"))]
        );
        let mut s = with_subtypes();
        let yeri = &mut s["layers"][2];
        yeri["subtypes"]["field"] = json!("mezar_no");
        yeri["subtypes"]["types"][1]["code"] = json!("TEKLI");
        yeri["subtypes"]["types"][0]["fields"][0]["values"] = json!(["BOS", "YOK"]);
        yeri["contingents"][0]["fields"] = json!(["tur", "ada_no"]);
        yeri["contingents"][0]["rows"][0]["cells"] = json!([{ "kind": "code", "code": "UCLU" }]);
        yeri["contingents"][0]["rows"][1]["cells"][1] = json!({ "kind": "hepsi" });
        yeri["contingents"][0]["rows"][1]["subtype"] = json!("COCUK");
        let (problems, _) = check("schema.json", &s, &FORMS);
        let at: Vec<&str> = problems.iter().filter_map(|p| p.at.as_deref()).collect();
        assert_eq!(
            at,
            [
                "/layers/2/subtypes/field",
                "/layers/2/subtypes/types/0/fields/0/values/1",
                "/layers/2/subtypes/types/1/code",
                "/layers/2/contingents/0/fields/1",
                "/layers/2/contingents/0/rows/0/cells",
                "/layers/2/contingents/0/rows/0/cells/0/code",
                "/layers/2/contingents/0/rows/1/cells/1/kind",
                "/layers/2/contingents/0/rows/1/subtype",
            ],
            "{problems:#?}"
        );
        assert!(
            problems[0].message.contains("değer listeli bir alan değil"),
            "{}",
            problems[0].message
        );
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
        s["layers"][1]["form"] = json!("forms/yok.kform");
        s["renk"] = json!("mavi");
        let (problems, _) = check("schema.json", &s, &FORMS);
        let at: Vec<&str> = problems.iter().filter_map(|p| p.at.as_deref()).collect();
        assert_eq!(
            at,
            [
                "/renk",
                "/domains/0/values/1/code",
                "/layers/0/geometry",
                "/layers/1/form",
                "/layers/1/fields/0/kind",
                "/layers/2/fields/2/domain",
                "/layers/3/name",
            ],
            "{problems:#?}"
        );
        // The relation's child is now a layer the schema does not make: a warning, and its field not checked.
        let (_, warnings) = check("schema.json", &s, &FORMS);
        assert_eq!(
            warnings.iter().map(|w| w.code).collect::<Vec<_>>(),
            ["layer_schema_outside"]
        );
        let newer = json!({"version": 2, "layers": [{"name": "a"}]});
        let (problems, _) = check("schema.json", &newer, &[]);
        assert!(
            problems[0].message.contains("daha yeni bir KentOS"),
            "{problems:#?}"
        );
    }
}
