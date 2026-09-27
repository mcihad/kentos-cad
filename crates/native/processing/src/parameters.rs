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
        ParamKind::Text { .. } | ParamKind::Expression { .. } | ParamKind::Field { .. } => {
            json!("")
        }
        ParamKind::Boolean => json!(false),
        ParamKind::Choice { options } => options.first().map_or(Value::Null, |o| json!(o.value)),
        ParamKind::Layer { .. } => json!({ "layerId": d.active_layer }),
        ParamKind::Point => Value::Null,
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
    def.visible_when.is_none_or(|when| when(values))
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
    let mut issues: Vec<Issue> = tool
        .parameters
        .iter()
        .filter(|p| is_visible(p, values))
        .filter_map(|p| {
            check_param(p, values.get(&p.name).unwrap_or(&Value::Null), layers).map(|message| {
                Issue {
                    param: Some(p.name.clone()),
                    message,
                }
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

fn check_param(p: &ParamDef, v: &Value, layers: &LayerTree) -> Option<String> {
    let name = format!("“{}”", p.label);
    if v.is_null() {
        return (!p.optional).then(|| format!("{name} boş bırakılamaz."));
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
            kentos_style_core::expr::compile(src)
                .err()
                .map(|e| format!("{name}: {}", e.text()))
        }
        ParamKind::Field { allow_new, .. } => {
            let f = js_trim(v.as_str().unwrap_or(""));
            if f.is_empty() {
                let or_type = if *allow_new { " ya da yazın" } else { "" };
                return (!p.optional).then(|| format!("{name}: bir alan adı seçin{or_type}."));
            }
            if utf16_len(f) > 64 {
                return Some(format!("{name}: alan adı en çok 64 karakter olabilir."));
            }
            if f.contains(['[', ']']) {
                return Some(format!("{name}: alan adında köşeli parantez kullanılamaz."));
            }
            None
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
        ParamKind::Boolean | ParamKind::Choice { .. } | ParamKind::Point => None,
    }
}
