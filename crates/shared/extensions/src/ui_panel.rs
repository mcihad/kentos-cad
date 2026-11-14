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
    },
    /// Yes or no.
    Checkbox {
        id: String,
        label: String,
        #[serde(default)]
        value: bool,
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
}

impl Component {
    /// Its id, when it has one.
    pub fn id(&self) -> Option<&str> {
        match self {
            Component::Text { id, .. } => id.as_deref(),
            Component::TextField { id, .. }
            | Component::NumberField { id, .. }
            | Component::Select { id, .. }
            | Component::Checkbox { id, .. }
            | Component::Button { id, .. }
            | Component::Table { id, .. } => Some(id),
        }
    }

    /// Whether the user gives it a value (a command's parameter).
    pub fn is_input(&self) -> bool {
        matches!(
            self,
            Component::TextField { .. }
                | Component::NumberField { .. }
                | Component::Select { .. }
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
        "select" => &["id", "label", "options", "value"],
        "checkbox" => &["id", "label", "value"],
        "button" => &["id", "label", "command", "table", "primary"],
        "table" => &["id", "label"],
        _ => return None,
    })
}

const TYPES: &str = "text, textField, numberField, select, checkbox, button, table";

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
    let tables: BTreeSet<&str> = body
        .iter()
        .filter(|c| c.get("type").and_then(Value::as_str) == Some("table"))
        .filter_map(|c| c.get("id").and_then(Value::as_str))
        .collect();
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
        if kind != "table" && text(c, "label").is_none() {
            add(
                at.clone(),
                format!("{kind} bileşeni boş olmayan bir ad (label) ister."),
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
                        "Düğmenin tablosu (table) bu paneldeki bir tablonun kimliği olmalı."
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
                { "type": "button", "id": "ara", "label": "Ara", "command": "yapi-ara", "table": "sonuc", "primary": true },
                { "type": "table", "id": "sonuc", "label": "Sonuçlar" }
            ]
        })
    }

    #[test]
    fn a_panel_passes_and_reads_as_kentos_draws_it() {
        let value = panel();
        assert!(check("ui/ara.panel.json", &value, &["yapi-ara"]).is_empty());
        let read = read(&value).expect("read");
        assert_eq!(read.title, "Yapı ara");
        assert_eq!(read.body.len(), 7);
        assert_eq!(read.body.iter().filter(|c| c.is_input()).count(), 4);
        assert!(
            matches!(&read.body[5], Component::Button { command, table: Some(t), .. }
            if command == "yapi-ara" && t == "sonuc")
        );
    }

    #[test]
    fn each_mistake_is_said_where_it_is() {
        let mut p = panel();
        p["body"][1]["id"] = json!("Sorgu");
        p["body"][2]["max"] = json!(-1);
        p["body"][3]["value"] = json!("yok");
        p["body"][5]["command"] = json!("ara");
        p["body"][5]["table"] = json!("liste");
        p["body"]
            .as_array_mut()
            .unwrap()
            .push(json!({ "type": "slider", "id": "x" }));
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
                "/body/5/command",
                "/body/5/table",
                "/body/7/type"
            ]
        );
    }
}
