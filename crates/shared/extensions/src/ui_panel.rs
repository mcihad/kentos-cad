//! An extension's panel, `ui/<id>.panel.json` (docs/adr/8057 §24): what a dock shows of it, declared in the package,
//! drawn by KentOS with its own components. The extension's code never gets a widget or a toolkit type (QGIS's
//! lesson: 58 % of its plugins were left behind when Qt changed, docs/research/2026-11-13-qgis-eklenti-sistemi.md).
//!
//! A button runs one of the extension's commands (`entrypoints.commands`) with the panel's values as its parameters
//! (`ctx.params`, by each input's id); the rows the command answers fill the panel's table. The command runs as any
//! other: in the extension's process, through the broker, one undo step, Durdur, its log.
//!
//! Checked here when the package is read, each problem with its JSON pointer.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::layer_schema::FIELD_KINDS;
use crate::problem::Problem;

/// `format`.
pub const FORMAT: &str = "kentos.ui.panel";
/// The `schemaVersion` this KentOS draws.
pub const VERSION: u64 = 1;
/// Components a panel may have.
pub const MOST_COMPONENTS: usize = 100;

/// A panel as KentOS draws it, read after [`check`] passed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PanelDefinition {
    pub title: String,
    pub body: Vec<Component>,
}

/// One of a select's choices.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Choice {
    pub value: String,
    pub label: String,
}

/// A panel's component, in KentOS's words.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Component {
    /// A line of text.
    Text {
        #[serde(default)]
        id: Option<String>,
        text: String,
    },
    /// A text the user types.
    TextField {
        id: String,
        label: String,
        #[serde(default)]
        placeholder: Option<String>,
        #[serde(default)]
        value: Option<String>,
    },
    /// A number the user types, in its bounds.
    NumberField {
        id: String,
        label: String,
        #[serde(default)]
        value: Option<f64>,
        #[serde(default)]
        min: Option<f64>,
        #[serde(default)]
        max: Option<f64>,
        #[serde(default)]
        unit: Option<String>,
    },
    /// One of its choices.
    Select {
        id: String,
        label: String,
        options: Vec<Choice>,
        #[serde(default)]
        value: Option<String>,
        /// A command run with the panel's values when the user changes it (its rows to the panel's first table).
        #[serde(default, rename = "onChange")]
        on_change: Option<String>,
    },
    /// One of the open drawing's layers, of `geometry`'s type (and the untyped ones) when given: its id.
    LayerPicker {
        id: String,
        label: String,
        #[serde(default)]
        geometry: Option<String>,
        #[serde(default, rename = "onChange")]
        on_change: Option<String>,
    },
    /// One of the fields of the layer the panel's layer picker `layer` holds, of `kinds` only when given (the layer
    /// schema's words): its name, none when none is chosen.
    FieldPicker {
        id: String,
        label: String,
        layer: String,
        #[serde(default)]
        kinds: Vec<String>,
        #[serde(default, rename = "onChange")]
        on_change: Option<String>,
    },
    /// The objects selected in the drawing, taken when the user asks: of the layer the panel's layer picker `layer`
    /// holds and of `geometry`'s type, when given. Its value is their persistent ids (`multiple`, the default), else
    /// one id or none.
    FeaturePicker {
        id: String,
        label: String,
        #[serde(default)]
        layer: Option<String>,
        #[serde(default)]
        geometry: Option<String>,
        #[serde(default = "many")]
        multiple: bool,
        #[serde(default, rename = "onChange")]
        on_change: Option<String>,
    },
    /// Yes or no.
    Checkbox {
        id: String,
        label: String,
        #[serde(default)]
        value: bool,
        #[serde(default, rename = "onChange")]
        on_change: Option<String>,
    },
    /// Runs `command` with the panel's values; its rows fill `table` (none: the panel's first table).
    Button {
        id: String,
        label: String,
        command: String,
        #[serde(default)]
        table: Option<String>,
        #[serde(default)]
        primary: bool,
    },
    /// The rows a button's command answered.
    Table {
        id: String,
        #[serde(default)]
        label: Option<String>,
    },
    /// The rows a button's command answered, grouped by `group`'s columns (one to three) into nodes that open and
    /// close, each with its count; a row with a `uid` is an object.
    Tree {
        id: String,
        #[serde(default)]
        label: Option<String>,
        group: Vec<String>,
    },
    /// The rows a button's command answered, a card each: `title`'s column heads it (none: the first column), `tag`'s
    /// beside it, the other columns under it by name; a card with a `uid` is an object. Where a table's columns do not
    /// fit (a narrow dock), a card writes them one under the other.
    Card {
        id: String,
        #[serde(default)]
        label: Option<String>,
        #[serde(default)]
        title: Option<String>,
        #[serde(default)]
        tag: Option<String>,
    },
}

/// A feature picker takes many objects unless it says otherwise.
fn many() -> bool {
    true
}

impl Component {
    /// Its id, when it has one.
    pub fn id(&self) -> Option<&str> {
        match self {
            Component::Text { id, .. } => id.as_deref(),
            Component::TextField { id, .. }
            | Component::NumberField { id, .. }
            | Component::Select { id, .. }
            | Component::LayerPicker { id, .. }
            | Component::FieldPicker { id, .. }
            | Component::FeaturePicker { id, .. }
            | Component::Checkbox { id, .. }
            | Component::Button { id, .. }
            | Component::Table { id, .. }
            | Component::Tree { id, .. }
            | Component::Card { id, .. } => Some(id),
        }
    }

    /// The command it runs when the user changes it, when it names one.
    pub fn on_change(&self) -> Option<&str> {
        match self {
            Component::Select { on_change, .. }
            | Component::LayerPicker { on_change, .. }
            | Component::FieldPicker { on_change, .. }
            | Component::FeaturePicker { on_change, .. }
            | Component::Checkbox { on_change, .. } => on_change.as_deref(),
            _ => None,
        }
    }

    /// Whether the user gives it a value (a command's parameter).
    pub fn is_input(&self) -> bool {
        matches!(
            self,
            Component::TextField { .. }
                | Component::NumberField { .. }
                | Component::Select { .. }
                | Component::LayerPicker { .. }
                | Component::FieldPicker { .. }
                | Component::FeaturePicker { .. }
                | Component::Checkbox { .. }
        )
    }
}

/// The panel, when `value` passed [`check`]; none otherwise.
pub fn read(value: &Value) -> Option<PanelDefinition> {
    serde_json::from_value(value.clone()).ok()
}

/// The component types and the keys each takes besides `type`.
fn keys_of(kind: &str) -> Option<&'static [&'static str]> {
    Some(match kind {
        "text" => &["id", "text"],
        "textField" => &["id", "label", "placeholder", "value"],
        "numberField" => &["id", "label", "value", "min", "max", "unit"],
        "select" => &["id", "label", "options", "value", "onChange"],
        "layerPicker" => &["id", "label", "geometry", "onChange"],
        "fieldPicker" => &["id", "label", "layer", "kinds", "onChange"],
        "featurePicker" => &["id", "label", "layer", "geometry", "multiple", "onChange"],
        "checkbox" => &["id", "label", "value", "onChange"],
        "button" => &["id", "label", "command", "table", "primary"],
        "table" => &["id", "label"],
        "tree" => &["id", "label", "group"],
        "card" => &["id", "label", "title", "tag"],
        _ => return None,
    })
}

const TYPES: &str = "text, textField, numberField, select, layerPicker, fieldPicker, featurePicker, checkbox, button, \
                     table, tree, card";

/// A component's id: lowercase letters, digits, `_` and `-`, a letter first, at most 63: the key its value is given to
/// the command under (`ctx.params["sorgu"]`).
fn is_component_id(id: &str) -> bool {
    let b = id.as_bytes();
    !b.is_empty()
        && b.len() <= 63
        && b[0].is_ascii_lowercase()
        && b.iter()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, b'_' | b'-'))
}

/// A key's text, when it is there and not blank.
fn text<'a>(c: &'a Map<String, Value>, key: &str) -> Option<&'a str> {
    c.get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
}

/// What is wrong with `value`, a panel read as JSON (`file` its path in the package; `commands` the extension's
/// commands, `entrypoints.commands`' ids), each problem with its JSON pointer.
pub fn check(file: &str, value: &Value, commands: &[&str]) -> Vec<Problem> {
    let mut out = Vec::new();
    let mut add = |at: String, message: String| {
        out.push(
            Problem::new("ui_panel", message)
                .file(file.to_owned())
                .at(at),
        );
    };
    let Some(top) = value.as_object() else {
        add(
            String::new(),
            "Panel bir JSON nesnesi ({…}) olmalı.".to_owned(),
        );
        return out;
    };
    for key in top
        .keys()
        .filter(|k| !["format", "schemaVersion", "title", "body"].contains(&k.as_str()))
    {
        add(
            format!("/{key}"),
            format!(
                "“{key}” panelde bilinmeyen bir alan; olanlar: format, schemaVersion, title, body."
            ),
        );
    }
    if top.get("format").and_then(Value::as_str) != Some(FORMAT) {
        add("/format".to_owned(), format!("Biçim “{FORMAT}” olmalı."));
    }
    match top.get("schemaVersion").and_then(Value::as_u64) {
        Some(VERSION) => {}
        Some(v) if v > VERSION => add(
            "/schemaVersion".to_owned(),
            format!(
                "Panel sürüm {v}; bu KentOS {VERSION}. sürümü çizer: daha yeni bir KentOS gerekir."
            ),
        ),
        _ => add(
            "/schemaVersion".to_owned(),
            format!("schemaVersion {VERSION} olmalı."),
        ),
    }
    match top.get("title").and_then(Value::as_str) {
        Some(t) if !t.trim().is_empty() && t.chars().count() <= 80 => {}
        _ => add(
            "/title".to_owned(),
            "Başlık boş olmayan, en çok 80 karakterlik bir metin olmalı.".to_owned(),
        ),
    }
    let Some(body) = top.get("body").and_then(Value::as_array) else {
        add(
            "/body".to_owned(),
            "body bir bileşen listesi ([…]) olmalı.".to_owned(),
        );
        return out;
    };
    if body.is_empty() || body.len() > MOST_COMPONENTS {
        add(
            "/body".to_owned(),
            format!("Panelde 1 ile {MOST_COMPONENTS} arasında bileşen olmalı."),
        );
    }
    let of_type = |kind: &str| -> BTreeSet<&str> {
        body.iter()
            .filter(|c| c.get("type").and_then(Value::as_str) == Some(kind))
            .filter_map(|c| c.get("id").and_then(Value::as_str))
            .collect()
    };
    // Where a button's rows may go: a table, a tree or a card.
    let tables: BTreeSet<&str> = ["table", "tree", "card"]
        .into_iter()
        .flat_map(of_type)
        .collect();
    let layer_pickers = of_type("layerPicker");
    let mut ids = BTreeSet::new();
    for (i, component) in body.iter().enumerate() {
        let at = format!("/body/{i}");
        let Some(c) = component.as_object() else {
            add(at, "Bileşen bir nesne ({…}) olmalı.".to_owned());
            continue;
        };
        let kind = c.get("type").and_then(Value::as_str).unwrap_or_default();
        let Some(known) = keys_of(kind) else {
            add(
                format!("{at}/type"),
                format!("Bileşenin türü (type) şunlardan biri olmalı: {TYPES}."),
            );
            continue;
        };
        for key in c
            .keys()
            .filter(|k| k.as_str() != "type" && !known.contains(&k.as_str()))
        {
            add(
                format!("{at}/{key}"),
                format!(
                    "“{key}” {kind} bileşeninde bilinmeyen bir alan; olanlar: {}.",
                    known.join(", ")
                ),
            );
        }
        match c.get("id") {
            None if kind == "text" => {}
            None => add(
                at.clone(),
                format!("{kind} bileşeni bir kimlik (id) ister."),
            ),
            Some(id) => match id.as_str() {
                Some(id) if !is_component_id(id) => add(
                    format!("{at}/id"),
                    format!(
                        "“{id}” bir bileşen kimliği değil: küçük harf, rakam, _ ve -, harfle başlar (ör. sorgu)."
                    ),
                ),
                Some(id) if !ids.insert(id.to_owned()) => add(
                    format!("{at}/id"),
                    format!("“{id}” kimlikli iki bileşen var."),
                ),
                Some(_) => {}
                None => add(format!("{at}/id"), "Kimlik metin olmalı.".to_owned()),
            },
        }
        if kind == "text" {
            if text(c, "text").is_none() {
                add(
                    at.clone(),
                    "text bileşeni boş olmayan bir metin (text) ister.".to_owned(),
                );
            }
            continue;
        }
        if !matches!(kind, "table" | "tree" | "card") && text(c, "label").is_none() {
            add(
                at.clone(),
                format!("{kind} bileşeni boş olmayan bir ad (label) ister."),
            );
        }
        // A value changed runs a command, as a button does: the command must be the extension's.
        if let Some(command) = c.get("onChange")
            && !command.as_str().is_some_and(|c| commands.contains(&c))
        {
            add(
                format!("{at}/onChange"),
                "onChange eklentinin bir komutu olmalı (manifestin entrypoints.commands listesinde): değer \
                 değişince panelin değerleriyle o çalışır."
                    .to_owned(),
            );
        }
        match kind {
            "numberField" => {
                let number = |key: &str| c.get(key).map(Value::as_f64);
                for key in ["value", "min", "max"] {
                    if let Some(None) = number(key) {
                        add(format!("{at}/{key}"), format!("“{key}” bir sayı olmalı."));
                    }
                }
                if let (Some(Some(min)), Some(Some(max))) = (number("min"), number("max"))
                    && min > max
                {
                    add(
                        format!("{at}/min"),
                        "En küçük değer (min) en büyükten (max) büyük.".to_owned(),
                    );
                }
            }
            "select" => {
                let options = c.get("options").and_then(Value::as_array);
                let mut values = BTreeSet::new();
                match options {
                    Some(options) if !options.is_empty() => {
                        for (j, option) in options.iter().enumerate() {
                            let oat = format!("{at}/options/{j}");
                            match (
                                option
                                    .get("value")
                                    .and_then(Value::as_str)
                                    .filter(|v| !v.is_empty()),
                                option
                                    .get("label")
                                    .and_then(Value::as_str)
                                    .filter(|l| !l.trim().is_empty()),
                            ) {
                                (Some(v), Some(_)) => {
                                    if !values.insert(v.to_owned()) {
                                        add(
                                            format!("{oat}/value"),
                                            format!("“{v}” seçeneği iki kez."),
                                        );
                                    }
                                }
                                _ => add(
                                    oat,
                                    "Seçenek boş olmayan bir değer (value) ve ad (label) ister."
                                        .to_owned(),
                                ),
                            }
                        }
                    }
                    _ => add(
                        format!("{at}/options"),
                        "Seçim en az bir seçenek (options) ister.".to_owned(),
                    ),
                }
                if let Some(value) = c.get("value")
                    && !value.as_str().is_some_and(|v| values.contains(v))
                {
                    add(
                        format!("{at}/value"),
                        "Başlangıç değeri seçeneklerden biri olmalı.".to_owned(),
                    );
                }
            }
            "layerPicker" | "featurePicker" => {
                if let Some(geometry) = c.get("geometry")
                    && !geometry
                        .as_str()
                        .is_some_and(|g| ["point", "line", "area", "annotation"].contains(&g))
                {
                    add(
                        format!("{at}/geometry"),
                        "Katman türü point, line, area ya da annotation olmalı.".to_owned(),
                    );
                }
                if kind == "featurePicker" {
                    if let Some(layer) = c.get("layer")
                        && !layer.as_str().is_some_and(|l| layer_pickers.contains(l))
                    {
                        add(
                            format!("{at}/layer"),
                            "Nesne seçicinin katmanı (layer) bu paneldeki bir katman seçicinin (layerPicker) \
                             kimliği olmalı: nesneler orada seçilen katmandan alınır."
                                .to_owned(),
                        );
                    }
                    if c.get("multiple").is_some_and(|v| !v.is_boolean()) {
                        add(
                            format!("{at}/multiple"),
                            "multiple true ya da false olmalı.".to_owned(),
                        );
                    }
                }
            }
            "fieldPicker" => {
                match c.get("layer") {
                    None => add(
                        at.clone(),
                        "Alan seçici, alanlarını listeleyeceği katman seçicinin kimliğini (layer) ister."
                            .to_owned(),
                    ),
                    Some(layer) if !layer.as_str().is_some_and(|l| layer_pickers.contains(l)) => add(
                        format!("{at}/layer"),
                        "Alan seçicinin katmanı (layer) bu paneldeki bir katman seçicinin (layerPicker) kimliği \
                         olmalı: alanlar orada seçilen katmanınkilerdir."
                            .to_owned(),
                    ),
                    Some(_) => {}
                }
                match c.get("kinds").map(Value::as_array) {
                    None => {}
                    Some(Some(kinds)) => {
                        for (j, k) in kinds.iter().enumerate() {
                            if !k.as_str().is_some_and(|k| FIELD_KINDS.contains(&k)) {
                                add(
                                    format!("{at}/kinds/{j}"),
                                    format!(
                                        "Alan türü şunlardan biri olmalı: {}.",
                                        FIELD_KINDS.join(", ")
                                    ),
                                );
                            }
                        }
                    }
                    Some(None) => add(
                        format!("{at}/kinds"),
                        "kinds alan türlerinin listesi ([…]) olmalı.".to_owned(),
                    ),
                }
            }
            "checkbox" => {
                if c.get("value").is_some_and(|v| !v.is_boolean()) {
                    add(
                        format!("{at}/value"),
                        "Onay kutusunun değeri true ya da false olmalı.".to_owned(),
                    );
                }
            }
            "button" => {
                match c.get("command").and_then(Value::as_str) {
                    Some(command) if commands.contains(&command) => {}
                    Some(command) => add(
                        format!("{at}/command"),
                        format!(
                            "“{command}” eklentinin komutlarında yok: manifestin entrypoints.commands listesine ekleyin \
                             (düğme bir komut çalıştırır)."
                        ),
                    ),
                    None => add(
                        at.clone(),
                        "Düğme çalıştıracağı komutu (command) ister.".to_owned(),
                    ),
                }
                if let Some(table) = c.get("table")
                    && !table.as_str().is_some_and(|t| tables.contains(t))
                {
                    add(
                        format!("{at}/table"),
                        "Düğmenin tablosu (table) bu paneldeki bir tablonun, ağacın ya da kartın kimliği olmalı."
                            .to_owned(),
                    );
                }
                if c.get("primary").is_some_and(|v| !v.is_boolean()) {
                    add(
                        format!("{at}/primary"),
                        "primary true ya da false olmalı.".to_owned(),
                    );
                }
            }
            "tree" => {
                let given = c.get("group").and_then(Value::as_array);
                let names: Vec<&str> = given
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .filter(|s| !s.trim().is_empty())
                    .collect();
                let fits = given.is_some_and(|g| g.len() == names.len())
                    && (1..=3).contains(&names.len())
                    && names.iter().collect::<BTreeSet<_>>().len() == names.len();
                if !fits {
                    add(
                        format!("{at}/group"),
                        "Ağaç, satırlarını gruplayacağı 1–3 farklı sütun adı (group) ister: ör. [\"ada\"]."
                            .to_owned(),
                    );
                }
            }
            "card" => {
                for key in ["title", "tag"] {
                    if c.get(key).is_some() && text(c, key).is_none() {
                        add(
                            format!("{at}/{key}"),
                            format!(
                                "“{key}” kartta gösterilecek bir sütunun adı olmalı (boş olmayan metin)."
                            ),
                        );
                    }
                }
                if let (Some(title), Some(tag)) = (text(c, "title"), text(c, "tag"))
                    && title == tag
                {
                    add(
                        format!("{at}/tag"),
                        "Kartın etiketi (tag) başlığıyla (title) aynı sütun: başka bir sütun seçin.".to_owned(),
                    );
                }
            }
            "textField" => {
                for key in ["placeholder", "value"] {
                    if c.get(key).is_some_and(|v| !v.is_string()) {
                        add(format!("{at}/{key}"), format!("“{key}” metin olmalı."));
                    }
                }
            }
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn panel() -> Value {
        json!({
            "format": "kentos.ui.panel", "schemaVersion": 1, "title": "Yapı ara",
            "body": [
                { "type": "text", "text": "Numarası yazılan yapıları listeler." },
                { "type": "textField", "id": "sorgu", "label": "Yapı no", "placeholder": "Y-01" },
                { "type": "numberField", "id": "en-az", "label": "En az alan", "min": 0, "unit": "m²" },
                { "type": "select", "id": "durum", "label": "Durum", "value": "hepsi",
                  "options": [{ "value": "hepsi", "label": "Hepsi" }, { "value": "numarasiz", "label": "Numarasız" }] },
                { "type": "checkbox", "id": "secili", "label": "Yalnız seçililer" },
                { "type": "layerPicker", "id": "katman", "label": "Katman", "geometry": "area" },
                { "type": "button", "id": "ara", "label": "Ara", "command": "yapi-ara", "table": "sonuc", "primary": true },
                { "type": "table", "id": "sonuc", "label": "Sonuçlar" },
                { "type": "fieldPicker", "id": "alan", "label": "Alan", "layer": "katman", "kinds": ["integer", "decimal"],
                  "onChange": "yapi-ara" },
                { "type": "featurePicker", "id": "nesneler", "label": "Nesneler", "layer": "katman", "geometry": "area" },
                { "type": "tree", "id": "agac", "label": "Adalara göre", "group": ["ada"] },
                { "type": "card", "id": "kartlar", "label": "Yapılar", "title": "no", "tag": "durum" }
            ]
        })
    }

    #[test]
    fn a_panel_passes_and_reads_as_kentos_draws_it() {
        let value = panel();
        assert!(check("ui/ara.panel.json", &value, &["yapi-ara"]).is_empty());
        let read = read(&value).expect("read");
        assert_eq!(read.title, "Yapı ara");
        assert_eq!(read.body.len(), 12);
        assert_eq!(read.body.iter().filter(|c| c.is_input()).count(), 7);
        assert!(
            matches!(&read.body[5], Component::LayerPicker { geometry: Some(g), .. } if g == "area")
        );
        assert!(
            matches!(&read.body[6], Component::Button { command, table: Some(t), .. }
            if command == "yapi-ara" && t == "sonuc")
        );
        assert!(
            matches!(&read.body[8], Component::FieldPicker { layer, kinds, .. }
            if layer == "katman" && kinds == &["integer", "decimal"])
        );
        assert!(
            matches!(&read.body[9], Component::FeaturePicker { layer: Some(l), multiple: true, .. } if l == "katman"),
            "it takes many objects unless it says otherwise"
        );
        assert_eq!(read.body[8].on_change(), Some("yapi-ara"));
        assert_eq!(read.body[9].on_change(), None);
        assert!(matches!(&read.body[10], Component::Tree { group, .. } if group == &["ada"]));
        assert!(
            matches!(&read.body[11], Component::Card { title: Some(t), tag: Some(g), .. }
            if t == "no" && g == "durum")
        );
        // A button's rows may go to a card; a card needs no title or tag.
        let mut to_card = panel();
        to_card["body"][6]["table"] = json!("kartlar");
        to_card["body"][11] = json!({ "type": "card", "id": "kartlar" });
        assert!(check("ui/ara.panel.json", &to_card, &["yapi-ara"]).is_empty());
    }

    /// ADR 8057 §25, the coverage report of the panel's components, names every type Kapı 1 knows: a new type is
    /// written there with what it covers.
    #[test]
    fn every_component_type_is_in_the_coverage_report() {
        let adr = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../../docs/adr/8057-phase-3-extension-packages.md"),
        )
        .expect("ADR 8057");
        let report = adr.split("\n## §25.").nth(1).expect("its §25");
        let report = report.split("\n## §").next().unwrap_or(report);
        for kind in TYPES.split(", ") {
            assert!(keys_of(kind).is_some(), "{kind}");
            assert!(
                report.contains(&format!("`{kind}`")),
                "ADR 8057 §25 does not name `{kind}`: add it to the coverage report"
            );
        }
    }

    #[test]
    fn each_mistake_is_said_where_it_is() {
        let mut p = panel();
        p["body"][1]["id"] = json!("Sorgu");
        p["body"][2]["max"] = json!(-1);
        p["body"][3]["value"] = json!("yok");
        p["body"][5]["geometry"] = json!("polygon");
        p["body"][6]["command"] = json!("ara");
        p["body"][6]["table"] = json!("liste");
        p["body"][8]["layer"] = json!("sonuc");
        p["body"][8]["kinds"] = json!(["sayi"]);
        p["body"][9]["multiple"] = json!("evet");
        p["body"][9]["onChange"] = json!("say");
        p["body"][10]["group"] = json!(["ada", "ada"]);
        p["body"][11]["tag"] = json!("no");
        p["body"]
            .as_array_mut()
            .unwrap()
            .push(json!({ "type": "slider", "id": "x" }));
        p["body"]
            .as_array_mut()
            .unwrap()
            .push(json!({ "type": "fieldPicker", "id": "y", "label": "Y" }));
        p["renk"] = json!("mavi");
        let at: Vec<String> = check("ui/ara.panel.json", &p, &["yapi-ara"])
            .into_iter()
            .filter_map(|p| p.at)
            .collect();
        assert_eq!(
            at,
            [
                "/renk",
                "/body/1/id",
                "/body/2/min",
                "/body/3/value",
                "/body/5/geometry",
                "/body/6/command",
                "/body/6/table",
                "/body/8/layer",
                "/body/8/kinds/0",
                "/body/9/onChange",
                "/body/9/multiple",
                "/body/10/group",
                "/body/11/tag",
                "/body/12/type",
                "/body/13"
            ]
        );
    }
}
