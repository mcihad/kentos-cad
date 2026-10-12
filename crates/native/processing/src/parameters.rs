//! Parameter bookkeeping shared by the dialog, the runner and models (the
//! web's `processing/parameters.ts`): defaults, visibility, validation with
//! messages for the user, and restoring stored values (last run, history)
//! safely after a tool's parameters changed.

use kentos_contracts::LayerNodeType;
use kentos_domain::LayerTree;
use serde_json::{Value, json};

use crate::field_map;
use crate::text::{js_number, js_trim, utf16_len};
use crate::types::{DefaultValue, Defaults, ParamDef, ParamKind, ScopeKind, Tool, Values};
use crate::values::{FeaturesValue, LayerValue, Scope, point};

const DEFAULT_SCOPES: [ScopeKind; 4] = [
    ScopeKind::Selection,
    ScopeKind::Visible,
    ScopeKind::All,
    ScopeKind::Layer,
];

/// The scopes a features parameter offers.
pub fn scopes_of(scopes: Option<&[ScopeKind]>) -> &[ScopeKind] {
    scopes.unwrap_or(&DEFAULT_SCOPES)
}

/// A parameter's value when nothing is stored: its default, or its type's.
pub fn default_value(def: &ParamDef, d: &Defaults) -> Value {
    match &def.default {
        Some(DefaultValue::Value(v)) => return v.clone(),
        Some(DefaultValue::From(from)) => return from(d),
        None => {}
    }
    match &def.kind {
        ParamKind::Features { scopes, .. } => {
            match scopes_of(scopes.as_deref())
                .first()
                .copied()
                .unwrap_or(ScopeKind::Selection)
            {
                ScopeKind::Layer => json!({ "scope": "layer", "layerId": d.active_layer }),
                scope => json!({ "scope": scope.id() }),
            }
        }
        ParamKind::Number { min, .. } => json!(min.unwrap_or(0.0)),
        ParamKind::Text { .. }
        | ParamKind::Expression { .. }
        | ParamKind::Field { .. }
        | ParamKind::SaveFile { .. } => json!(""),
        ParamKind::Boolean => json!(false),
        ParamKind::Choice { options } => options.first().map_or(Value::Null, |o| json!(o.value)),
        ParamKind::Layer { .. } => json!({ "layerId": d.active_layer }),
        ParamKind::Point | ParamKind::File { .. } => Value::Null,
        ParamKind::RasterValues { .. } => json!({}),
        ParamKind::RasterPairs { .. } | ParamKind::FieldMap { .. } | ParamKind::Layers { .. } => {
            json!([])
        }
        ParamKind::AddressSource => d
            .address_sources
            .iter()
            .find(|o| o.why.is_none())
            .map_or(Value::Null, |o| json!(o.id)),
        ParamKind::Network { prefers } => d
            .networks
            .iter()
            .find(|n| n.kind == *prefers)
            .or_else(|| d.networks.first())
            .map_or(
                Value::Null,
                |n| json!({ "network": n.id, "cost": kentos_contracts::LENGTH_COST }),
            ),
    }
}

pub fn default_values(tool: &Tool, d: &Defaults) -> Values {
    tool.parameters
        .iter()
        .map(|p| (p.name.clone(), default_value(p, d)))
        .collect()
}

/// Whether the parameter is shown (and checked) for these values.
pub fn is_visible(def: &ParamDef, values: &Values) -> bool {
    def.visible_when
        .as_ref()
        .is_none_or(|when| when.holds(values))
}

/// The first of these parameters that is shown: the source a field's names
/// come from (docs/adr/0200 §6), or a field map's columns (docs/adr/0216 §2).
pub fn field_source<'t>(tool: &'t Tool, of: &[String], values: &Values) -> Option<&'t ParamDef> {
    of.iter().find_map(|n| {
        tool.parameters
            .iter()
            .find(|p| &p.name == n)
            .filter(|p| is_visible(p, values))
    })
}

/// A field map's source key (docs/adr/0216 §2): the first of its `key`
/// parameters that names one, trimmed; else empty.
pub fn field_map_key(key: &[String], values: &Values) -> String {
    key.iter()
        .filter_map(|n| values.get(n).and_then(Value::as_str).map(js_trim))
        .find(|s| !s.is_empty())
        .unwrap_or_default()
        .to_owned()
}

/// The names a field parameter holds: one, or several written with commas
/// between them (`multiple`), each trimmed.
pub fn field_names(multiple: bool, value: &str) -> Vec<String> {
    let names: Vec<&str> = if multiple {
        value.split(',').collect()
    } else {
        vec![value]
    };
    names
        .into_iter()
        .map(js_trim)
        .filter(|n| !n.is_empty())
        .map(str::to_owned)
        .collect()
}

/// A file value's rows (`{ name, rows }`), their cells read where they
/// are; none without them.
pub fn file_rows(v: &Value) -> Option<Vec<Vec<&str>>> {
    v.get("rows")?
        .as_array()?
        .iter()
        .map(|r| {
            r.as_array()?
                .iter()
                .map(Value::as_str)
                .collect::<Option<Vec<_>>>()
        })
        .collect()
}

/// Whether a file value's rows, when it has them, are rows of texts: what
/// [`file_rows`] reads, checked without reading it.
fn rows_fit(v: &Value) -> bool {
    v.get("rows").is_none_or(|r| {
        r.as_array().is_some_and(|rows| {
            rows.iter().all(|row| {
                row.as_array()
                    .is_some_and(|cells| cells.iter().all(Value::is_string))
            })
        })
    })
}

/// A file parameter's table as fields read it: the first row's names
/// (trimmed), then the rows; none without rows.
pub fn file_table(v: &Value) -> Option<(Vec<String>, Vec<Vec<&str>>)> {
    let mut rows = file_rows(v)?;
    if rows.is_empty() {
        return None;
    }
    let header = rows
        .remove(0)
        .iter()
        .map(|c| js_trim(c).to_owned())
        .collect();
    Some((header, rows))
}

/// A value as the last values keep it: a file's name (and path, and the
/// sheet chosen) without its rows, its sheets' or what was left out.
pub fn stored_value(def: &ParamDef, v: &Value) -> Value {
    match (&def.kind, v) {
        (ParamKind::File { .. }, Value::Object(o)) => Value::Object(
            o.iter()
                .filter(|(k, _)| !matches!(k.as_str(), "rows" | "sheets" | "leftOut"))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        ),
        _ => v.clone(),
    }
}

/// Values as the last values keep them (`stored_value` for each).
pub fn stored_values(tool: &Tool, values: &Values) -> Values {
    let mut out = values.clone();
    for p in &tool.parameters {
        if let Some(v) = out.get_mut(&p.name) {
            *v = stored_value(p, v);
        }
    }
    out
}

/// Whether a stored value still fits the parameter (so it can be restored).
pub fn fits(def: &ParamDef, v: &Value) -> bool {
    if v.is_null() {
        return def.optional || matches!(def.kind, ParamKind::Point);
    }
    match &def.kind {
        ParamKind::Number { .. } => v.as_f64().is_some_and(f64::is_finite),
        ParamKind::Text { .. } | ParamKind::Expression { .. } | ParamKind::Field { .. } => {
            v.is_string()
        }
        ParamKind::Boolean => v.is_boolean(),
        ParamKind::Choice { options } => v
            .as_str()
            .is_some_and(|s| options.iter().any(|o| o.value == s)),
        ParamKind::Features { kinds, scopes, .. } => {
            let Some(f) = FeaturesValue::read(v) else {
                return false;
            };
            if let Some(chosen) = &f.kinds
                && !chosen
                    .iter()
                    .all(|k| kinds.as_ref().is_none_or(|kinds| kinds.contains(k)))
            {
                return false;
            }
            match f.scope {
                Scope::Ids(_) | Scope::Layer(_) => true,
                Scope::Selection => scopes_of(scopes.as_deref()).contains(&ScopeKind::Selection),
                Scope::Visible => scopes_of(scopes.as_deref()).contains(&ScopeKind::Visible),
                Scope::All => scopes_of(scopes.as_deref()).contains(&ScopeKind::All),
            }
        }
        ParamKind::Layer { .. } => LayerValue::read(v).is_some(),
        ParamKind::Point => point(v).is_some(),
        ParamKind::File { .. } => v.get("name").is_some_and(Value::is_string) && rows_fit(v),
        ParamKind::SaveFile { .. } => v.is_string(),
        ParamKind::Network { .. } => {
            v.get("network").is_some_and(Value::is_string)
                && v.get("cost").is_some_and(Value::is_string)
        }
        ParamKind::AddressSource => v.is_string(),
        ParamKind::RasterValues { number, .. } => v.as_object().is_some_and(|o| {
            o.values().all(|x| {
                if *number {
                    x.as_f64().is_some_and(f64::is_finite)
                } else {
                    x.is_string()
                }
            })
        }),
        ParamKind::RasterPairs { .. } => v.as_array().is_some_and(|rows| {
            rows.iter().all(|r| {
                r.as_array().is_some_and(|t| {
                    t.len() == 3
                        && t[0].is_string()
                        && t[1].is_string()
                        && t[2].as_f64().is_some_and(f64::is_finite)
                })
            })
        }),
        ParamKind::FieldMap { .. } => field_map::read(v).is_some(),
        ParamKind::Layers { .. } => v
            .as_array()
            .is_some_and(|ids| ids.iter().all(Value::is_string)),
    }
}

/// A comparison of two rasters as the pairs hold it (docs/adr/0237 §9): 9 … 2, 1, −2 … −9.
pub fn is_comparison(v: f64) -> bool {
    v.fract() == 0.0 && (v == 1.0 || (2.0..=9.0).contains(&v.abs()))
}

/// Stored values over the defaults, keeping only those that still fit.
pub fn restore_values(tool: &Tool, stored: Option<&Values>, d: &Defaults) -> Values {
    let mut out = default_values(tool, d);
    if let Some(stored) = stored {
        for p in &tool.parameters {
            if let Some(v) = stored.get(&p.name).filter(|v| fits(p, v)) {
                out.insert(p.name.clone(), v.clone());
            }
        }
    }
    out
}

/// A problem with the values: under the parameter it belongs to, or the
/// tool's own (no parameter).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Issue {
    pub param: Option<String>,
    pub message: String,
}

/// Problems with the values, in parameter order; empty when the tool can run.
pub fn validate_values(tool: &Tool, values: &Values, layers: &LayerTree) -> Vec<Issue> {
    validate_with(tool, values, layers, &[])
}

/// `validate_values` with the project's networks, which a network parameter names (docs/adr/0209 §10).
pub fn validate_with(
    tool: &Tool,
    values: &Values,
    layers: &LayerTree,
    networks: &[kentos_contracts::NetworkDef],
) -> Vec<Issue> {
    let mut issues: Vec<Issue> = tool
        .parameters
        .iter()
        .filter(|p| is_visible(p, values))
        .filter_map(|p| {
            check_param(
                p,
                values.get(&p.name).unwrap_or(&Value::Null),
                layers,
                networks,
            )
            .map(|message| Issue {
                param: Some(p.name.clone()),
                message,
            })
        })
        .collect();
    if issues.is_empty()
        && let Some(message) = tool.validate.and_then(|check| check(values))
    {
        issues.push(Issue {
            param: None,
            message,
        });
    }
    issues
}

fn check_param(
    p: &ParamDef,
    v: &Value,
    layers: &LayerTree,
    networks: &[kentos_contracts::NetworkDef],
) -> Option<String> {
    let name = format!("“{}”", p.label);
    if v.is_null() {
        return (!p.optional).then(|| match p.kind {
            ParamKind::File { .. } => format!("{name}: bir dosya seçin."),
            ParamKind::Network { .. } => format!(
                "{name}: projede ağ yok; Ağlar penceresinden yol ya da şebeke ağı tanımlayın."
            ),
            ParamKind::AddressSource => format!(
                "{name}: adres kaynağı yok; Adres verisi penceresinden projenin verisini tanımlayın ya da Adres sağlayıcıları'ndan bir sağlayıcıyı açın."
            ),
            _ => format!("{name} boş bırakılamaz."),
        });
    }
    if !fits(p, v) {
        return Some(format!("{name} için geçersiz değer."));
    }
    match &p.kind {
        ParamKind::Number {
            min, max, integer, ..
        } => {
            let n = v.as_f64().unwrap_or(0.0);
            if *integer && n.fract() != 0.0 {
                return Some(format!("{name} bir tam sayı olmalı."));
            }
            if let Some(min) = min.filter(|min| n < *min) {
                return Some(format!("{name} en az {} olmalı.", js_number(min)));
            }
            if let Some(max) = max.filter(|max| n > *max) {
                return Some(format!("{name} en çok {} olmalı.", js_number(max)));
            }
            None
        }
        ParamKind::Text {
            max_length,
            allow_empty,
            ..
        } => {
            let s = v.as_str().unwrap_or("");
            if !p.optional && !allow_empty && js_trim(s).is_empty() {
                return Some(format!("{name} boş bırakılamaz."));
            }
            if let Some(max) = max_length.filter(|max| utf16_len(s) > *max) {
                return Some(format!("{name} en çok {max} karakter olabilir."));
            }
            None
        }
        ParamKind::Features { writes, .. } => {
            let f = FeaturesValue::read(v)?;
            if let Scope::Layer(id) = &f.scope {
                let Some(node) = layers.get(id) else {
                    return Some(format!("{name}: seçilen katman artık yok."));
                };
                // An input that may be empty and is written to is where a run adds (docs/adr/0216 §3): a layer,
                // and one that is not locked (what is added to it would be left out).
                if p.allow_empty && *writes {
                    if node.kind == LayerNodeType::Group {
                        return Some(format!(
                            "{name}: grup değil, bir katman seçin; eklenen nesneler bu katmana yazılır."
                        ));
                    }
                    if layers.is_locked(id) {
                        return Some(format!(
                            "{name}: katman kilitli. Kilidi Katmanlar panelinden açın ya da başka katman seçin."
                        ));
                    }
                }
            }
            if f.kinds.as_ref().is_some_and(Vec::is_empty) {
                return Some(format!("{name}: en az bir nesne türü seçin."));
            }
            None
        }
        ParamKind::Expression { .. } => {
            let src = js_trim(v.as_str().unwrap_or(""));
            if src.is_empty() {
                return (!p.optional).then(|| format!("{name}: bir ifade yazın."));
            }
            // İşlemler give the calls to other layers (docs/adr/0214 §1); the `@` values are read at the run.
            let schema = kentos_expression::Schema {
                world: true,
                ..kentos_expression::Schema::default()
            };
            kentos_expression::compile_with(src, &schema)
                .err()
                .map(|e| format!("{name}: {}", e.text()))
        }
        ParamKind::Field {
            allow_new,
            multiple,
            ..
        } => {
            let names = field_names(*multiple, v.as_str().unwrap_or(""));
            if names.is_empty() {
                let or_type = if *allow_new { " ya da yazın" } else { "" };
                return (!p.optional).then(|| format!("{name}: bir alan adı seçin{or_type}."));
            }
            if names.iter().any(|f| utf16_len(f) > 64) {
                return Some(format!("{name}: alan adı en çok 64 karakter olabilir."));
            }
            if names.iter().any(|f| f.contains(['[', ']'])) {
                return Some(format!("{name}: alan adında köşeli parantez kullanılamaz."));
            }
            None
        }
        ParamKind::File { .. } => {
            let file = v.get("name").and_then(Value::as_str).unwrap_or("");
            // Its rows fit (`fits` above); a stored value has none.
            match v.get("rows").and_then(Value::as_array) {
                Some(rows) if rows.is_empty() => Some(format!(
                    "{name}: “{file}” boş; başlık satırı olan bir dosya seçin."
                )),
                Some(_) => None,
                None => Some(format!(
                    "{name}: “{file}” dosyasını yeniden seçin; dosyanın içeriği saklanmaz."
                )),
            }
        }
        ParamKind::RasterValues { min, max, .. } => {
            let o = v.as_object()?;
            o.iter().find_map(|(raster, x)| {
                let n = x.as_f64()?;
                if min.is_some_and(|m| n < m) || max.is_some_and(|m| n > m) {
                    Some(format!(
                        "{name}: {raster} {} ile {} arasında olmalı.",
                        js_number(min.unwrap_or(f64::MIN)),
                        js_number(max.unwrap_or(f64::MAX))
                    ))
                } else {
                    None
                }
            })
        }
        ParamKind::FieldMap { .. } => field_map::problem(&name, &field_map::read(v)?),
        ParamKind::Layers { min } => {
            let ids: Vec<&str> = v.as_array()?.iter().filter_map(Value::as_str).collect();
            if ids.iter().any(|id| layers.get(id).is_none()) {
                return Some(format!("{name}: seçilen katman artık yok."));
            }
            (ids.len() < *min && !(p.optional && ids.is_empty()))
                .then(|| format!("{name}: en az {min} katman seçin."))
        }
        ParamKind::RasterPairs { .. } => v.as_array()?.iter().find_map(|r| {
            let t = r.as_array()?;
            let n = t.get(2)?.as_f64()?;
            (!is_comparison(n)).then(|| {
                format!(
                    "{name}: {} — {} 1, 2 … 9 ya da −2 … −9 olmalı.",
                    t[0].as_str().unwrap_or(""),
                    t[1].as_str().unwrap_or("")
                )
            })
        }),
        ParamKind::AddressSource => {
            let id = v.as_str().unwrap_or("");
            (!(id.starts_with(crate::address::LOCATOR) || id.starts_with(crate::address::PROVIDER)))
                .then(|| format!("{name}: projenin adres verisini ya da bir sağlayıcıyı seçin."))
        }
        ParamKind::Network { .. } => {
            let id = v.get("network").and_then(Value::as_str).unwrap_or("");
            let cost = v.get("cost").and_then(Value::as_str).unwrap_or("");
            match networks.iter().find(|n| n.id == id) {
                None => Some(format!(
                    "{name}: “{id}” ağı projede yok; Ağlar penceresinden tanımlayın ya da başka ağ seçin."
                )),
                Some(n) if !n.cost_names().contains(&cost) => Some(format!(
                    "{name}: “{cost}” maliyeti “{}” ağında yok.",
                    n.name
                )),
                Some(_) => None,
            }
        }
        ParamKind::Layer { .. } => match LayerValue::read(v)? {
            LayerValue::New(name_) if js_trim(&name_).is_empty() => {
                Some(format!("{name}: yeni katmanın adını yazın."))
            }
            LayerValue::New(_) => None,
            LayerValue::Existing(id) if layers.get(&id).is_none() => {
                Some(format!("{name}: seçilen katman artık yok."))
            }
            LayerValue::Existing(id) if layers.is_locked(&id) => Some(format!(
                "{name}: katman kilitli. Kilidi Katmanlar panelinden açın ya da yeni katman seçin."
            )),
            LayerValue::Existing(_) => None,
        },
        // An empty path is beside the input; a chosen one has a file name.
        ParamKind::SaveFile { .. } => {
            let path = v.as_str().unwrap_or("");
            (!path.is_empty() && (path.ends_with('/') || path.ends_with('\\')))
                .then(|| format!("{name}: dosyanın adını da yazın."))
        }
        ParamKind::Boolean | ParamKind::Choice { .. } | ParamKind::Point => None,
    }
}
