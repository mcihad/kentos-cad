//! Parameter bookkeeping shared by the dialog, the runner and models (the
//! web's `processing/parameters.ts`): defaults, visibility, validation with
//! messages for the user, and restoring stored values (last run, history)
//! safely after a tool's parameters changed.

use kentos_domain::LayerTree;
use serde_json::{Value, json};

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

/// The parameter a field's names come from: the first of `of` that is shown
/// (docs/adr/0200 §6).
pub fn field_source<'t>(tool: &'t Tool, of: &[String], values: &Values) -> Option<&'t ParamDef> {
    of.iter().find_map(|n| {
        tool.parameters
            .iter()
            .find(|p| &p.name == n)
            .filter(|p| is_visible(p, values))
    })
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

/// A file value's rows (`{ name, rows }`); none without them.
pub fn file_rows(v: &Value) -> Option<Vec<Vec<String>>> {
    v.get("rows")?
        .as_array()?
        .iter()
        .map(|r| {
            r.as_array()?
                .iter()
                .map(|c| c.as_str().map(str::to_owned))
                .collect::<Option<Vec<_>>>()
        })
        .collect()
}

/// A file parameter's table as fields read it: the first row's names
/// (trimmed), then the rows; none without rows.
pub fn file_table(v: &Value) -> Option<(Vec<String>, Vec<Vec<String>>)> {
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

/// A value as the last values keep it: a file's name (and path) without its rows.
pub fn stored_value(def: &ParamDef, v: &Value) -> Value {
    match (&def.kind, v) {
        (ParamKind::File { .. }, Value::Object(o)) => Value::Object(
            o.iter()
                .filter(|(k, _)| *k != "rows")
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
        ParamKind::File { .. } => {
            v.get("name").is_some_and(Value::is_string)
                && v.get("rows")
                    .is_none_or(|r| file_rows(v).is_some() && r.is_array())
        }
        ParamKind::SaveFile { .. } => v.is_string(),
        ParamKind::Network { .. } => {
            v.get("network").is_some_and(Value::is_string)
                && v.get("cost").is_some_and(Value::is_string)
        }
    }
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
        ParamKind::Features { .. } => {
            let f = FeaturesValue::read(v)?;
            if let Scope::Layer(id) = &f.scope
                && layers.get(id).is_none()
            {
                return Some(format!("{name}: seçilen katman artık yok."));
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
            match file_rows(v) {
                Some(rows) if rows.is_empty() => Some(format!(
                    "{name}: “{file}” boş; başlık satırı olan bir dosya seçin."
                )),
                Some(_) => None,
                None => Some(format!(
                    "{name}: “{file}” dosyasını yeniden seçin; dosyanın içeriği saklanmaz."
                )),
            }
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
