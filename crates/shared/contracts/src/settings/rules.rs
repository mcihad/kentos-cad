//! The rules every host applies the same way (docs/adr/0023): validating a
//! value, which layer may hold which setting, resolving the layers into the
//! requested and the effective value, reading and writing the stored
//! document, and presets. The web app's TypeScript twin
//! (`apps/web/src/core/settings`) runs the same cases
//! (`fixtures/settings/v1/cases.json`) and must give the same codes.
//!
//! Checks run in a fixed order, so both hosts report the same code for a
//! value wrong in two ways: the key, the layer, the type, whole number, the
//! range (a text's length), the choices.

use std::collections::BTreeMap;

use serde_json::{Map, Value, json};

use super::{
    ResolveReason, ResolvedSetting, SETTINGS_FORMAT, SETTINGS_VERSION, SettingConstraint,
    SettingDescriptor, SettingDiagnostic, SettingErrorCode, SettingScope, SettingType,
    SettingsFile, SettingsLayer, SettingsPolicy, SettingsPreset, SettingsSchema,
};

use SettingErrorCode as Code;

impl SettingDescriptor {
    /// The value as this setting keeps it, or why it cannot be one. Numbers
    /// come back normalised: a whole number without a fraction (`45.0` → `45`),
    /// so equal values compare equal.
    pub fn validate(&self, value: &Value) -> Result<Value, SettingErrorCode> {
        let value = match self.kind {
            SettingType::Boolean => Value::Bool(value.as_bool().ok_or(Code::WrongType)?),
            SettingType::Enum => Value::String(value.as_str().ok_or(Code::WrongType)?.to_owned()),
            SettingType::Text => {
                let text = value.as_str().ok_or(Code::WrongType)?;
                // Characters, not bytes: the web counts code points the same way.
                if self
                    .max
                    .is_some_and(|max| text.chars().count() as f64 > max)
                {
                    return Err(Code::OutOfRange);
                }
                Value::String(text.to_owned())
            }
            SettingType::Integer | SettingType::Number => {
                let n = value
                    .as_f64()
                    .filter(|n| n.is_finite())
                    .ok_or(Code::WrongType)?;
                if self.kind == SettingType::Integer && n.fract() != 0.0 {
                    return Err(Code::NotInteger);
                }
                if self.min.is_some_and(|min| n < min) || self.max.is_some_and(|max| n > max) {
                    return Err(Code::OutOfRange);
                }
                number(n)
            }
        };
        if !self.choices.is_empty() && !self.choices.iter().any(|c| same_value(&c.value, &value)) {
            // A colour where one may be given instead (the accent).
            return value
                .as_str()
                .filter(|_| self.color)
                .and_then(color_of)
                .map(Value::String)
                .ok_or(Code::NotAllowed);
        }
        Ok(value)
    }

    /// The values it may take, as values (its choices).
    pub fn choice_values(&self) -> Vec<Value> {
        self.choices.iter().map(|c| c.value.clone()).collect()
    }
}

/// Whether a layer may hold a setting of `scope`: its own layer or a more
/// specific one (user < device < session), so a device or a session can
/// override a preference; project settings only in the project. Default and
/// organisation settings are in no layer.
pub fn layer_holds(layer: SettingScope, scope: SettingScope) -> bool {
    if scope == SettingScope::Project || layer == SettingScope::Project {
        return scope == layer;
    }
    match (rank(layer), rank(scope)) {
        (Some(layer), Some(scope)) => layer >= scope,
        _ => false,
    }
}

fn rank(scope: SettingScope) -> Option<u8> {
    match scope {
        SettingScope::User => Some(1),
        SettingScope::Device => Some(2),
        SettingScope::Session => Some(3),
        _ => None,
    }
}

/// The value `layer` would keep for `key`, or why it may not: the key, the
/// layer, then the value (the order both hosts report in).
pub fn check_write(
    schema: &SettingsSchema,
    layer: SettingScope,
    key: &str,
    value: &Value,
) -> Result<Value, SettingErrorCode> {
    let descriptor = schema.get(key).ok_or(Code::UnknownKey)?;
    if !layer_holds(layer, descriptor.scope) {
        return Err(Code::WrongScope);
    }
    descriptor.validate(value)
}

/// The layers a host resolves. Session values are never stored; the project
/// layer is the open project's settings.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SettingsLayers {
    pub user: SettingsLayer,
    pub device: SettingsLayer,
    pub session: SettingsLayer,
    pub project: SettingsLayer,
}

/// Every setting resolved, and the values the layers hold but resolution did not use.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Resolution {
    pub settings: BTreeMap<String, ResolvedSetting>,
    pub diagnostics: Vec<SettingDiagnostic>,
}

impl Resolution {
    pub fn get(&self, key: &str) -> Option<&ResolvedSetting> {
        self.settings.get(key)
    }

    /// The value in use.
    pub fn effective(&self, key: &str) -> Option<&Value> {
        self.settings.get(key).map(|r| &r.effective)
    }
}

/// Resolves every setting of the schema (TODOS.md SET-02, SET-03):
/// 1. `requested` is the most specific layer's valid value (session, device,
///    user), else the default; a project setting only from the project;
/// 2. the organisation's policy locks it or bounds it;
/// 3. what the host can use (`constraints`) brings it within reach.
///
/// `effective` is what is used, with the reason when it differs. Invalid
/// values, values in a layer that cannot hold them and unknown keys are
/// reported and skipped: the next layer down decides.
pub fn resolve(
    schema: &SettingsSchema,
    layers: &SettingsLayers,
    policy: &SettingsPolicy,
    constraints: &BTreeMap<String, SettingConstraint>,
) -> Resolution {
    let mut diagnostics = Vec::new();
    for (layer, values) in [
        (SettingScope::User, &layers.user),
        (SettingScope::Device, &layers.device),
        (SettingScope::Session, &layers.session),
        (SettingScope::Project, &layers.project),
    ] {
        for (key, value) in values {
            if let Err(code) = check_write(schema, layer, key, value) {
                diagnostics.push(SettingDiagnostic {
                    layer,
                    key: key.clone(),
                    code,
                });
            }
        }
    }
    for key in policy.rules.keys() {
        if schema.get(key).is_none() {
            diagnostics.push(SettingDiagnostic {
                layer: SettingScope::Organization,
                key: key.clone(),
                code: Code::UnknownKey,
            });
        }
    }
    let settings = schema
        .settings
        .iter()
        .map(|d| {
            let resolved =
                resolve_one(d, layers, policy, constraints.get(&d.key), &mut diagnostics);
            (d.key.clone(), resolved)
        })
        .collect();
    Resolution {
        settings,
        diagnostics,
    }
}

fn resolve_one(
    d: &SettingDescriptor,
    layers: &SettingsLayers,
    policy: &SettingsPolicy,
    constraint: Option<&SettingConstraint>,
    diagnostics: &mut Vec<SettingDiagnostic>,
) -> ResolvedSetting {
    let order = [
        (SettingScope::Project, &layers.project),
        (SettingScope::Session, &layers.session),
        (SettingScope::Device, &layers.device),
        (SettingScope::User, &layers.user),
    ];
    let (requested, source) = order
        .iter()
        .filter(|(layer, _)| layer_holds(*layer, d.scope))
        .find_map(|(layer, values)| {
            let value = d.validate(values.get(&d.key)?).ok()?;
            Some((value, *layer))
        })
        .unwrap_or_else(|| (d.default.clone(), SettingScope::Default));

    let mut effective = requested.clone();
    let mut reason = None;
    let mut detail = None;
    let mut locked = false;
    if let Some(rule) = policy.rules.get(&d.key) {
        let mut bad_rule = |code| {
            diagnostics.push(SettingDiagnostic {
                layer: SettingScope::Organization,
                key: d.key.clone(),
                code,
            });
        };
        let before = effective.clone();
        match &rule.value {
            Some(lock) => match d.validate(lock) {
                Ok(lock) => {
                    locked = true;
                    effective = lock;
                }
                Err(code) => bad_rule(code),
            },
            None => {
                if let Some(n) = effective.as_f64().filter(|_| is_number(d)) {
                    let raised = rule.min.map_or(n, |min| n.max(min));
                    let bounded = rule.max.map_or(raised, |max| raised.min(max));
                    if bounded != n {
                        effective = within(d, bounded, bounded > n);
                    }
                }
                if !rule.allowed.is_empty() {
                    let allowed: Vec<Value> = rule
                        .allowed
                        .iter()
                        .filter_map(|v| d.validate(v).ok())
                        .collect();
                    if allowed.is_empty() {
                        bad_rule(Code::NotAllowed);
                    } else if !contains(&allowed, &effective) {
                        effective = nearest_allowed(&allowed, &effective);
                    }
                }
            }
        }
        if !same_value(&before, &effective) {
            reason = Some(if locked {
                ResolveReason::OrganizationLocked
            } else {
                ResolveReason::OrganizationLimit
            });
            detail = Some(policy.source.clone());
        }
    }
    // The device last: whatever was asked or allowed, it can only use what it has.
    if let Some(c) = constraint
        && !c.allowed.is_empty()
    {
        let allowed: Vec<Value> = c
            .allowed
            .iter()
            .filter_map(|v| d.validate(v).ok())
            .collect();
        if !allowed.is_empty() && !contains(&allowed, &effective) {
            effective = nearest_allowed(&allowed, &effective);
            reason = Some(c.reason);
            detail = Some(c.detail.clone());
        }
    }
    if same_value(&effective, &requested) {
        reason = None;
        detail = None;
    }
    ResolvedSetting {
        key: d.key.clone(),
        requested,
        source,
        effective,
        reason,
        detail,
        locked,
    }
}

fn is_number(d: &SettingDescriptor) -> bool {
    matches!(d.kind, SettingType::Integer | SettingType::Number)
}

/// A bounded number made valid for `d`: onto one of its choices (the nearest
/// below, as a device would), else to a whole number for an integer, toward
/// the bound it was moved to.
fn within(d: &SettingDescriptor, n: f64, raised: bool) -> Value {
    if !d.choices.is_empty() {
        return nearest_allowed(&d.choice_values(), &number(n));
    }
    if d.kind == SettingType::Integer {
        return number(if raised { n.ceil() } else { n.floor() });
    }
    number(n)
}

/// The allowed value nearest `value` from below (numbers: the largest not
/// above it, else the smallest); for other types the first allowed one.
pub fn nearest_allowed(allowed: &[Value], value: &Value) -> Value {
    if contains(allowed, value) {
        return value.clone();
    }
    if let Some(n) = value.as_f64() {
        let numbers: Vec<f64> = allowed.iter().filter_map(Value::as_f64).collect();
        let below = numbers.iter().copied().filter(|a| *a <= n).reduce(f64::max);
        if let Some(pick) = below.or_else(|| numbers.iter().copied().reduce(f64::min)) {
            return number(pick);
        }
    }
    allowed.first().cloned().unwrap_or_else(|| value.clone())
}

fn contains(values: &[Value], value: &Value) -> bool {
    values.iter().any(|v| same_value(v, value))
}

/// Equal values: numbers by value (`4` = `4.0`), others as JSON.
pub fn same_value(a: &Value, b: &Value) -> bool {
    match (a.as_f64(), b.as_f64()) {
        (Some(x), Some(y)) => x == y,
        _ => a == b,
    }
}

/// A number as settings keep it: whole numbers without a fraction.
fn number(n: f64) -> Value {
    if n.fract() == 0.0 && n.abs() < 9.0e15 {
        json!(n as i64)
    } else {
        json!(n)
    }
}

impl SettingsFile {
    /// Reads a stored or exported settings document (TODOS.md SET-04). A
    /// leading byte order mark is skipped. The format and version are checked
    /// first; another format or version is refused, not guessed. Within the
    /// layers, an older key is read under today's ([`RENAMED`]) and a
    /// retired one dropped ([`RETIRED`]); a value that is invalid, sensitive
    /// or in a layer that cannot hold it is dropped and reported; the others
    /// are kept. An unknown key (from a newer version) is kept as it is and
    /// reported, so saving here does not lose it.
    pub fn from_json(
        text: &str,
        schema: &SettingsSchema,
    ) -> Result<(Self, Vec<SettingDiagnostic>), SettingErrorCode> {
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);
        let value: Value = serde_json::from_str(text).map_err(|_| Code::NotJson)?;
        let Value::Object(mut root) = value else {
            return Err(Code::NotSettings);
        };
        if root.get("format").and_then(Value::as_str) != Some(SETTINGS_FORMAT) {
            return Err(Code::NotSettings);
        }
        match root.get("version").and_then(whole_number) {
            Some(v) if v == f64::from(SETTINGS_VERSION) => {}
            Some(_) => return Err(Code::UnsupportedVersion),
            None => return Err(Code::NotSettings),
        }
        let mut diagnostics = Vec::new();
        let mut layer =
            |name: &str, scope: SettingScope| -> Result<SettingsLayer, SettingErrorCode> {
                match root.remove(name) {
                    None | Some(Value::Null) => Ok(SettingsLayer::new()),
                    Some(Value::Object(values)) => {
                        Ok(read_layer(schema, scope, values, &mut diagnostics))
                    }
                    Some(_) => Err(Code::NotSettings),
                }
            };
        let user = layer("user", SettingScope::User)?;
        let device = layer("device", SettingScope::Device)?;
        // A malformed migration record does not cost the values.
        let migrations = root
            .remove("migrations")
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default();
        Ok((
            Self {
                format: SETTINGS_FORMAT.into(),
                version: SETTINGS_VERSION,
                user,
                device,
                migrations,
            },
            diagnostics,
        ))
    }

    /// The document as text, sensitive values left out.
    pub fn to_json(&self, schema: &SettingsSchema) -> String {
        let keep = |layer: &SettingsLayer| -> SettingsLayer {
            layer
                .iter()
                .filter(|(key, _)| !schema.get(key).is_some_and(|d| d.sensitive))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        };
        let out = Self {
            user: keep(&self.user),
            device: keep(&self.device),
            ..self.clone()
        };
        let mut text = serde_json::to_string_pretty(&out).unwrap_or_default();
        text.push('\n');
        text
    }
}

fn whole_number(v: &Value) -> Option<f64> {
    v.as_f64().filter(|n| n.fract() == 0.0)
}

/// Keys earlier versions kept settings under, and today's (docs/adr/0126):
/// the desktop's accent, typefaces and the web's five text sizes. Every
/// settings document, stored or imported, is read through them.
pub const RENAMED: &[(&str, &str)] = &[
    ("appearance.accentColor", "appearance.accent"),
    ("appearance.typeface", "appearance.uiFont"),
    ("appearance.monoTypeface", "appearance.monoFont"),
    ("appearance.uiScale", "appearance.textSize"),
];

/// Keys earlier versions kept settings under that no longer mean anything:
/// dropped as they are read, without a report (a key from a newer version is
/// kept and reported). The web's shell, since it shows the ribbon only
/// (docs/adr/0155).
pub const RETIRED: &[&str] = &["appearance.shell"];

/// An older key's value under today's key: `None` when `key` is not an
/// older key; an error when its value has no reading today.
///
/// - the desktop's accent names (`mavi`, `turkuaz` …, as it wrote them)
///   become the shared ids; near twins join the web's (turkuaz → teal,
///   kehribar → amber); `#RRGGBB` is kept in lower case;
/// - the typefaces keep their ids;
/// - the web's text sizes become pixels: 12, 13, 14, 15 and 16.
pub fn renamed_setting(
    key: &str,
    value: &Value,
) -> Option<Result<(&'static str, Value), SettingErrorCode>> {
    let (_, to) = RENAMED.iter().find(|(from, _)| *from == key)?;
    let Some(word) = value.as_str() else {
        return Some(Err(Code::WrongType));
    };
    let read = match key {
        "appearance.accentColor" => older_accent(word).map(Value::String),
        "appearance.uiScale" => match word {
            "small" => Some(json!(12)),
            "standard" => Some(json!(13)),
            "large" => Some(json!(14)),
            "xlarge" => Some(json!(15)),
            "xxlarge" => Some(json!(16)),
            _ => None,
        },
        _ => Some(Value::String(word.to_owned())),
    };
    Some(read.map(|v| (*to, v)).ok_or(Code::NotAllowed))
}

/// A colour, `#rrggbb` in either case, as settings keep it: lower case.
pub fn color_of(text: &str) -> Option<String> {
    let digits = text.strip_prefix('#')?;
    (digits.len() == 6 && digits.chars().all(|c| c.is_ascii_hexdigit()))
        .then(|| format!("#{}", digits.to_ascii_lowercase()))
}

/// The desktop's accent as it stored it, in today's ids.
fn older_accent(word: &str) -> Option<String> {
    let preset = match word {
        "mavi" => "blue",
        "turkuaz" => "teal",
        "yesil" => "green",
        "kehribar" => "amber",
        "turuncu" => "orange",
        "pembe" => "pink",
        "mor" => "violet",
        "gri" => "gray",
        _ => return color_of(word),
    };
    Some(preset.to_owned())
}

/// A layer's older keys under today's: a value already under today's key
/// wins; an older value with no reading today is dropped and reported
/// under its older key.
fn renamed_layer(
    mut values: Map<String, Value>,
    scope: SettingScope,
    diagnostics: &mut Vec<SettingDiagnostic>,
) -> Map<String, Value> {
    for key in RETIRED {
        values.remove(*key);
    }
    for (from, _) in RENAMED {
        let Some(value) = values.remove(*from) else {
            continue;
        };
        match renamed_setting(from, &value) {
            Some(Ok((to, value))) => {
                values.entry(to).or_insert(value);
            }
            Some(Err(code)) => diagnostics.push(SettingDiagnostic {
                layer: scope,
                key: (*from).to_owned(),
                code,
            }),
            None => {}
        }
    }
    values
}

fn read_layer(
    schema: &SettingsSchema,
    scope: SettingScope,
    values: Map<String, Value>,
    diagnostics: &mut Vec<SettingDiagnostic>,
) -> SettingsLayer {
    let values = renamed_layer(values, scope, diagnostics);
    let mut kept = SettingsLayer::new();
    for (key, value) in values {
        let checked = match schema.get(&key) {
            // From a newer version: kept untouched, used by none.
            None => Err((Code::UnknownKey, Some(value))),
            Some(d) if d.sensitive => Err((Code::Sensitive, None)),
            Some(_) => check_write(schema, scope, &key, &value).map_err(|code| (code, None)),
        };
        match checked {
            Ok(value) => {
                kept.insert(key, value);
            }
            Err((code, keep)) => {
                diagnostics.push(SettingDiagnostic {
                    layer: scope,
                    key: key.clone(),
                    code,
                });
                if let Some(value) = keep {
                    kept.insert(key, value);
                }
            }
        }
    }
    kept
}

impl SettingsSchema {
    /// The preset of `group` whose every value equals the one `value_of`
    /// gives (the requested values), if any: the settings window names it,
    /// or says “Özel”.
    pub fn matching_preset(
        &self,
        group: &str,
        value_of: impl Fn(&str) -> Option<Value>,
    ) -> Option<&SettingsPreset> {
        self.presets.iter().filter(|p| p.group == group).find(|p| {
            p.values
                .iter()
                .all(|(key, value)| value_of(key).is_some_and(|v| same_value(&v, value)))
        })
    }
}
