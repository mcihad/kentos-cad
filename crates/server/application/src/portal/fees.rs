//! Municipal fees and charges (docs/adr/9003; plan BUL-22): a municipality's yearly tariff as data, and the assessments
//! (tahakkuk) it makes on dossiers.
//!
//! The engine is pure (`validate`, `evaluate`): a tariff's inputs become the expression language's `@` values, each rate
//! table becomes the `@` value of the row the inputs match, and each item that applies to the dossier's kind and whose
//! condition holds makes a line: a fixed amount, a quantity times a rate, or its formula's value; within its bounds,
//! rounded as the tariff says, with its VAT. Every line says in words what it rests on.
//!
//! The use cases keep the rules of the law the engine does not know: one active tariff a day, a draft is what changes,
//! an issued assessment is paid with a receipt or cancelled with a reason, and an application is not approved while a
//! fee to be paid before the decision is not (2464 sayılı Kanun Ek Madde 5: the building fee before the permit).

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};

use kentos_contracts::{
    DossierSide, FeeAssess, FeeAssessment, FeeAssessmentState, FeeCancel, FeeChoice, FeeDefinition, FeeInputs, FeeItem,
    FeeLine, FeePaid, FeeParameter, FeeParameterKind, FeePreview, FeeRounding, FeeRule, FeeSchedule, FeeScheduleCopy,
    FeeScheduleEdit, FeeScheduleState, FeeTable, FeeTableRow, FeeTry, TenantRole,
};
use kentos_expression::{Schema, Scope as ExprScope, Value, Variable, compile_with};
use kentos_postgres::Db;
use sqlx::types::Json;
use sqlx::{Postgres, Transaction};
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use super::accounts::user_scope;
use super::dossiers::{Held, hold, record};
use super::flow::{self, Party};
use super::notifications::tell;
use super::organization::require_role;
use super::{date_at, rfc3339, today};
use crate::error::{AppError, AppResult};
use crate::identity::Actor;
use crate::tenancy::Access;

// ── Numbers as people read them ─────────────────────────────────────────────

/// `1234` → `1.234`.
fn grouped(whole: u64) -> String {
    let digits = whole.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push('.');
        }
        out.push(c);
    }
    out
}

/// Kuruş as Turkish lira: `123450` → `1.234,50 TL`.
pub fn tl(kurus: i64) -> String {
    let sign = if kurus < 0 { "-" } else { "" };
    let k = kurus.unsigned_abs();
    format!("{sign}{},{:02} TL", grouped(k / 100), k % 100)
}

/// A quantity or a rate as read: `152`, `3,75`, `0,125`.
fn number(x: f64) -> String {
    let text = format!("{:.4}", x.abs());
    let text = text.trim_end_matches('0').trim_end_matches('.');
    let (whole, frac) = text.split_once('.').unwrap_or((text, ""));
    let whole: u64 = whole.parse().unwrap_or(0);
    let mut out = grouped(whole);
    if !frac.is_empty() {
        out.push(',');
        out.push_str(frac);
    }
    if x < 0.0 {
        out.insert(0, '-');
    }
    out
}

/// TL to kuruş, rounded as the tariff says; a half kuruş up (away from zero), whatever the binary fraction did to it.
fn rounded(tl: f64, rounding: FeeRounding) -> i64 {
    let half_up = |v: f64| {
        let t = v.trunc();
        let frac = (v - t).abs();
        if (frac - 0.5).abs() < 1e-7 || frac > 0.5 {
            t + v.signum()
        } else {
            t
        }
    };
    let kurus = match rounding {
        FeeRounding::Kurus => half_up(tl * 100.0),
        FeeRounding::Lira => half_up(tl) * 100.0,
        FeeRounding::LiraDown => tl.trunc() * 100.0,
    };
    kurus as i64
}

// ── Checking a tariff ───────────────────────────────────────────────────────

fn key_ok(key: &str) -> bool {
    let mut cs = key.chars();
    cs.next().is_some_and(|c| c.is_ascii_lowercase())
        && key.len() <= 40
        && cs.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// The `@name`s an expression reads, by a plain scan: keys are `[a-z0-9_]`, and the language reads `@Alan` as `@alan`.
fn names_in(source: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut rest = source;
    while let Some(at) = rest.find('@') {
        let tail = &rest[at + 1..];
        let end = tail
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .unwrap_or(tail.len());
        if end > 0 {
            out.insert(tail[..end].to_ascii_lowercase());
        }
        rest = &tail[end..];
    }
    out
}

/// A number as a person types it: `152,5`, `1.250` (thousands), `1.234,5`, or `152.5`.
fn typed_number(text: &str) -> Option<f64> {
    let t: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    let grouped_thousands = |s: &str| {
        let mut parts = s.split('.');
        parts.next().is_some_and(|h| (1..=3).contains(&h.len()) && h.chars().all(|c| c.is_ascii_digit()))
            && s.contains('.')
            && parts.all(|g| g.len() == 3 && g.chars().all(|c| c.is_ascii_digit()))
    };
    let plain = if t.contains(',') {
        t.replace('.', "").replace(',', ".")
    } else if grouped_thousands(&t) {
        t.replace('.', "")
    } else {
        t
    };
    plain.parse::<f64>().ok()
}

/// A placeholder value of an input's kind, to compile a formula without the dossier's values.
fn placeholder(kind: FeeParameterKind) -> Value<'static> {
    match kind {
        FeeParameterKind::Number => Value::Num(1.0),
        FeeParameterKind::Choice => Value::Text(Cow::Borrowed("")),
        FeeParameterKind::Flag => Value::Bool(false),
    }
}

fn invalid(path: impl Into<String>, message: impl Into<String>) -> AppError {
    AppError::invalid_at(path, message)
}

/// Whether a tariff can be applied: unique, well-formed keys; every reference names an input or a table of the right
/// kind; every formula and condition compiles and reads only known names; bounds and rates make sense.
pub fn validate(def: &FeeDefinition) -> AppResult<()> {
    let mut seen = BTreeSet::new();
    for (i, p) in def.parameters.iter().enumerate() {
        let at = |f: &str| format!("definition.parameters[{i}].{f}");
        if !key_ok(&p.key) {
            return Err(invalid(at("key"), format!("“{}”: anahtar küçük harf, rakam ve _ olmalı, harfle başlamalı.", p.key)));
        }
        if !seen.insert(p.key.clone()) {
            return Err(invalid(at("key"), format!("“{}” anahtarı iki kez kullanılmış.", p.key)));
        }
        if p.label.trim().is_empty() {
            return Err(invalid(at("label"), "Girdinin adı yok."));
        }
        if p.kind == FeeParameterKind::Choice && p.choices.is_empty() {
            return Err(invalid(at("choices"), format!("“{}” seçmeli bir girdi; seçenekleri yazın.", p.label)));
        }
    }
    let param = |k: &str| def.parameters.iter().find(|p| p.key == k);
    for (i, t) in def.tables.iter().enumerate() {
        let at = |f: &str| format!("definition.tables[{i}].{f}");
        if !key_ok(&t.key) {
            return Err(invalid(at("key"), format!("“{}”: tablo anahtarı küçük harf, rakam ve _ olmalı.", t.key)));
        }
        if !seen.insert(t.key.clone()) {
            return Err(invalid(at("key"), format!("“{}” anahtarı bir girdiyle ya da başka bir tabloyla aynı.", t.key)));
        }
        if let Some(by) = &t.by
            && param(by).map(|p| p.kind) != Some(FeeParameterKind::Choice)
        {
            return Err(invalid(at("by"), format!("“{by}” seçmeli bir girdi değil.")));
        }
        if let Some(q) = &t.quantity
            && param(q).map(|p| p.kind) != Some(FeeParameterKind::Number)
        {
            return Err(invalid(at("quantity"), format!("“{q}” sayı girdisi değil.")));
        }
        if t.rows.is_empty() {
            return Err(invalid(at("rows"), format!("“{}” tablosunda satır yok.", t.label)));
        }
        for (j, r) in t.rows.iter().enumerate() {
            let rat = format!("definition.tables[{i}].rows[{j}]");
            if !r.rate.is_finite() || r.rate < 0.0 {
                return Err(invalid(format!("{rat}.rate"), "Birim tutar sıfır ya da artı bir sayı olmalı."));
            }
            if let (Some(a), Some(b)) = (r.from, r.to)
                && a > b
            {
                return Err(invalid(format!("{rat}.to"), "Dilimin sonu başından küçük."));
            }
            let below = r.min_rate.is_some_and(|lo| r.rate < lo - 1e-9);
            let above = r.max_rate.is_some_and(|hi| r.rate > hi + 1e-9);
            if below || above {
                let bounds: Vec<String> = [
                    r.min_rate.map(|lo| format!("en az {} TL", number(lo))),
                    r.max_rate.map(|hi| format!("en çok {} TL", number(hi))),
                ]
                .into_iter()
                .flatten()
                .collect();
                return Err(invalid(
                    format!("{rat}.rate"),
                    format!("Birim tutar sınırların dışında ({}).", bounds.join(", ")),
                ));
            }
        }
    }
    let table = |k: &str| def.tables.iter().find(|t| t.key == k);
    let variables: Vec<Variable> = def
        .parameters
        .iter()
        .map(|p| Variable { name: p.key.clone(), value: placeholder(p.kind), description: String::new() })
        .chain(def.tables.iter().map(|t| Variable {
            name: t.key.clone(),
            value: Value::Num(1.0),
            description: String::new(),
        }))
        .collect();
    let schema = Schema { variables, ..Schema::default() };
    let compiles = |source: &str, path: String| -> AppResult<()> {
        let e = compile_with(source, &schema).map_err(|e| invalid(path.clone(), format!("İfade okunamadı: {}", e.message)))?;
        if let Some(name) = e.unknown_variables.first() {
            return Err(invalid(path, format!("“@{name}” tarifede bir girdi ya da tablo değil.")));
        }
        if !e.fields.is_empty() {
            return Err(invalid(path, "İfade yalnız tarifenin girdilerini ve tablolarını (@ad) okur."));
        }
        Ok(())
    };
    let mut codes = BTreeSet::new();
    for (i, item) in def.items.iter().enumerate() {
        let at = |f: &str| format!("definition.items[{i}].{f}");
        if !key_ok(&item.code) {
            return Err(invalid(at("code"), format!("“{}”: kod küçük harf, rakam ve _ olmalı.", item.code)));
        }
        if !codes.insert(item.code.clone()) {
            return Err(invalid(at("code"), format!("“{}” kodu iki kez kullanılmış.", item.code)));
        }
        if item.label.trim().is_empty() {
            return Err(invalid(at("label"), "Kalemin adı yok."));
        }
        match &item.rule {
            FeeRule::Fixed { amount } => {
                if !amount.is_finite() || *amount < 0.0 {
                    return Err(invalid(at("rule.amount"), "Tutar sıfır ya da artı bir sayı olmalı."));
                }
            }
            FeeRule::PerUnit { quantity, table: t, rate } => {
                if param(quantity).map(|p| p.kind) != Some(FeeParameterKind::Number) {
                    return Err(invalid(at("rule.quantity"), format!("“{quantity}” sayı girdisi değil.")));
                }
                match (t, rate) {
                    (Some(k), _) if table(k).is_none() => {
                        return Err(invalid(at("rule.table"), format!("“{k}” diye bir tablo yok.")));
                    }
                    (None, None) => {
                        return Err(invalid(at("rule"), "Birim başına kalem bir tablo ya da bir birim tutar ister."));
                    }
                    (None, Some(r)) if !r.is_finite() || *r < 0.0 => {
                        return Err(invalid(at("rule.rate"), "Birim tutar sıfır ya da artı bir sayı olmalı."));
                    }
                    _ => {}
                }
            }
            FeeRule::Formula { expression } => compiles(expression, at("rule.expression"))?,
        }
        if let Some(c) = &item.condition {
            compiles(c, at("condition"))?;
        }
        if let (Some(lo), Some(hi)) = (item.minimum, item.maximum)
            && lo > hi
        {
            return Err(invalid(at("maximum"), "En çok tutar en az tutardan küçük."));
        }
        if !(0.0..=100.0).contains(&item.vat_percent) {
            return Err(invalid(at("vatPercent"), "KDV oranı 0 ile 100 arasında olmalı."));
        }
    }
    Ok(())
}

// ── Applying a tariff ───────────────────────────────────────────────────────

/// What a tariff makes of a dossier's kind and inputs.
#[derive(Clone, Debug, PartialEq)]
pub struct Evaluated {
    pub parameters: Vec<FeeParameter>,
    pub lines: Vec<FeeLine>,
    pub total: i64,
    pub missing: BTreeMap<String, String>,
}

struct NoObject;

impl ExprScope for NoObject {
    fn field(&self, _: usize) -> Option<&str> {
        None
    }
    fn measured(&self) -> kentos_expression::Measured {
        kentos_expression::Measured::default()
    }
    fn vertices(&self) -> Option<f64> {
        None
    }
    fn kind(&self) -> &str {
        ""
    }
    fn layer(&self) -> &str {
        ""
    }
    fn label(&self) -> Option<&str> {
        None
    }
    fn index(&self) -> f64 {
        1.0
    }
    fn id(&self) -> f64 {
        0.0
    }
    fn scale(&self) -> Option<f64> {
        None
    }
}

/// An input's value as given (JSON), checked against its kind: a number (text with a decimal comma passes), one of
/// the choices, or a flag.
fn value_of(p: &FeeParameter, given: Option<&serde_json::Value>) -> Result<Option<Value<'static>>, String> {
    let Some(v) = given.filter(|v| !v.is_null() && v.as_str() != Some("")) else {
        return match p.kind {
            FeeParameterKind::Flag => Ok(Some(Value::Bool(false))),
            _ if p.required => Err("girilmedi".into()),
            _ => Ok(None),
        };
    };
    match p.kind {
        FeeParameterKind::Number => {
            let n = v.as_f64().or_else(|| v.as_str().and_then(typed_number));
            match n {
                Some(n) if n.is_finite() && n >= 0.0 => Ok(Some(Value::Num(n))),
                _ => Err("sıfır ya da artı bir sayı olmalı".into()),
            }
        }
        FeeParameterKind::Choice => match v.as_str() {
            Some(s) if p.choices.iter().any(|c| c.value == s) => Ok(Some(Value::Text(Cow::Owned(s.to_owned())))),
            _ => Err("seçeneklerden biri olmalı".into()),
        },
        FeeParameterKind::Flag => match v.as_bool() {
            Some(b) => Ok(Some(Value::Bool(b))),
            None => Err("evet ya da hayır olmalı".into()),
        },
    }
}

fn choice_label<'a>(p: Option<&'a FeeParameter>, value: &'a str) -> &'a str {
    p.and_then(|p| p.choices.iter().find(|c| c.value == value).map(|c| c.label.as_str()))
        .unwrap_or(value)
}

/// The row of `t` the inputs match, and how it reads (“Konut, 151–200 m²”).
fn row_of<'a>(
    def: &'a FeeDefinition,
    t: &'a FeeTable,
    values: &BTreeMap<String, Value<'static>>,
) -> Result<(&'a FeeTableRow, String), String> {
    let chosen = t.by.as_ref().map(|k| match values.get(k) {
        Some(Value::Text(s)) => Ok(s.to_string()),
        _ => Err(format!("“{k}” seçilmedi")),
    });
    let chosen = chosen.transpose()?;
    let quantity = t.quantity.as_ref().map(|k| match values.get(k) {
        Some(Value::Num(x)) => Ok(*x),
        _ => Err(format!("“{k}” girilmedi")),
    });
    let quantity = quantity.transpose()?;
    let fits = |q: Option<f64>| {
        t.rows.iter().find(|r| {
            r.when.as_deref().is_none_or(|w| chosen.as_deref() == Some(w))
                && r.from.is_none_or(|a| q.is_some_and(|q| q >= a - 1e-9))
                && r.to.is_none_or(|b| q.is_some_and(|q| q <= b + 1e-9))
        })
    };
    // Brackets are written in whole units (“100 m²’ye kadar”, “101–120 m²”): 100,5 m² is above 100, so in the next.
    let row = fits(quantity)
        .or_else(|| fits(quantity.map(f64::ceil)))
        .ok_or_else(|| format!("“{}” tablosunda bu değerlere uyan satır yok", t.label))?;
    let param = |k: &Option<String>| k.as_ref().and_then(|k| def.parameters.iter().find(|p| &p.key == k));
    let unit = param(&t.quantity).and_then(|p| p.unit.clone()).unwrap_or_default();
    let mut said = Vec::new();
    if let (Some(_), Some(c)) = (&row.when, &chosen) {
        said.push(choice_label(param(&t.by), c).to_owned());
    }
    match (row.from, row.to) {
        (Some(a), Some(b)) => said.push(format!("{}–{} {unit}", number(a), number(b)).trim().to_owned()),
        (Some(a), None) => said.push(format!("{} {unit} ve üstü", number(a)).trim().to_owned()),
        (None, Some(b)) => said.push(format!("{} {unit}e kadar", number(b)).trim().to_owned()),
        (None, None) => {}
    }
    Ok((row, said.join(", ")))
}

/// The inputs and tables an item reads: its quantity and table, its formula's and its condition's `@` names.
fn needs_of(item: &FeeItem) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    match &item.rule {
        FeeRule::Fixed { .. } => {}
        FeeRule::PerUnit { quantity, table, .. } => {
            out.insert(quantity.clone());
            out.extend(table.iter().cloned());
        }
        FeeRule::Formula { expression } => out.extend(names_in(expression)),
    }
    if let Some(c) = &item.condition {
        out.extend(names_in(c));
    }
    out
}

/// What `def` makes of a dossier of `kind` with `inputs`; lines only for items whose inputs are all there.
pub fn evaluate(def: &FeeDefinition, kind: &str, inputs: &FeeInputs) -> Evaluated {
    let items: Vec<&FeeItem> = def
        .items
        .iter()
        .filter(|i| i.applies_to.is_empty() || i.applies_to.iter().any(|k| k == kind))
        .collect();
    let needs: Vec<BTreeSet<String>> = items.iter().map(|i| needs_of(i)).collect();
    // The inputs the applying items read, directly or through their tables.
    let mut used: BTreeSet<String> = needs.iter().flatten().cloned().collect();
    for t in &def.tables {
        if used.contains(&t.key) {
            used.extend(t.by.iter().cloned());
            used.extend(t.quantity.iter().cloned());
        }
    }
    // Only the inputs these items read: an occupancy permit is not asked the building's use.
    let parameters: Vec<FeeParameter> = def.parameters.iter().filter(|p| used.contains(&p.key)).cloned().collect();
    let mut missing = BTreeMap::new();
    let mut values: BTreeMap<String, Value<'static>> = BTreeMap::new();
    for p in &parameters {
        match value_of(p, inputs.get(&p.key)) {
            Ok(Some(v)) => {
                values.insert(p.key.clone(), v);
            }
            Ok(None) => {}
            Err(why) => {
                missing.insert(p.key.clone(), format!("{}: {why}", p.label));
            }
        }
    }
    let mut rates: BTreeMap<String, (f64, String)> = BTreeMap::new();
    for t in def.tables.iter().filter(|t| used.contains(&t.key)) {
        match row_of(def, t, &values) {
            Ok((row, said)) => {
                rates.insert(t.key.clone(), (row.rate, said));
            }
            Err(why) if !t.by.iter().chain(t.quantity.iter()).any(|k| missing.contains_key(k)) => {
                missing.insert(t.key.clone(), why);
            }
            Err(_) => {}
        }
    }
    let variables: Vec<Variable> = values
        .iter()
        .map(|(k, v)| Variable { name: k.clone(), value: v.clone(), description: String::new() })
        .chain(rates.iter().map(|(k, (r, _))| Variable {
            name: k.clone(),
            value: Value::Num(*r),
            description: String::new(),
        }))
        .collect();
    let schema = Schema { variables, ..Schema::default() };
    let run = |source: &str| -> Result<Value<'static>, String> {
        let e = compile_with(source, &schema).map_err(|e| e.message)?;
        if let Some(name) = e.unknown_variables.first() {
            return Err(format!("“@{name}” girilmedi"));
        }
        Ok(e.evaluate(&NoObject).into_owned())
    };
    let unit_of = |k: &str| {
        def.parameters
            .iter()
            .find(|p| p.key == k)
            .and_then(|p| p.unit.clone())
            .map(|u| format!(" {u}"))
            .unwrap_or_default()
    };
    let mut lines = Vec::new();
    for (item, needs) in items.into_iter().zip(&needs) {
        // A wrong or missing required input is said once, at the input; an optional one not given leaves its items out.
        if needs.iter().any(|k| missing.contains_key(k))
            || !needs.iter().all(|k| values.contains_key(k) || rates.contains_key(k))
        {
            continue;
        }
        if let Some(c) = &item.condition {
            match run(c) {
                Ok(v) if kentos_expression::value::truthy(&v) => {}
                Ok(_) => continue,
                Err(why) => {
                    missing.insert(item.code.clone(), format!("{}: koşul hesaplanamadı ({why})", item.label));
                    continue;
                }
            }
        }
        let (raw, basis) = match &item.rule {
            FeeRule::Fixed { amount } => (*amount, "Maktu".to_owned()),
            FeeRule::PerUnit { quantity, table, rate } => {
                let q = match values.get(quantity) {
                    Some(Value::Num(q)) => *q,
                    _ => continue,
                };
                let (r, said) = match table {
                    Some(t) => match rates.get(t) {
                        Some((r, said)) => (*r, format!(" ({said})")),
                        None => continue,
                    },
                    None => (rate.unwrap_or(0.0), String::new()),
                };
                (q * r, format!("{}{} × {} TL{said}", number(q), unit_of(quantity), number(r)))
            }
            FeeRule::Formula { expression } => match run(expression) {
                Ok(Value::Num(x)) if x.is_finite() => (x, format!("Formül: {expression}")),
                Ok(_) | Err(_) => {
                    missing.insert(item.code.clone(), format!("{}: formül bir tutar vermedi", item.label));
                    continue;
                }
            },
        };
        let mut amount_tl = raw.max(0.0);
        let mut basis = basis;
        if let Some(lo) = item.minimum
            && amount_tl < lo
        {
            amount_tl = lo;
            basis.push_str(&format!("; en az {} TL", number(lo)));
        }
        if let Some(hi) = item.maximum
            && amount_tl > hi
        {
            amount_tl = hi;
            basis.push_str(&format!("; en çok {} TL", number(hi)));
        }
        let amount = rounded(amount_tl, def.rounding);
        let vat = if item.vat_percent > 0.0 {
            rounded(amount as f64 / 100.0 * item.vat_percent / 100.0, FeeRounding::Kurus)
        } else {
            0
        };
        lines.push(FeeLine {
            code: item.code.clone(),
            label: item.label.clone(),
            group: if item.group.trim().is_empty() { "Harç".into() } else { item.group.clone() },
            legal_basis: item.legal_basis.clone(),
            basis,
            amount,
            vat,
            before_decision: item.before_decision,
        });
    }
    let total = lines.iter().map(|l| l.amount + l.vat).sum();
    Evaluated { parameters, lines, total, missing }
}

/// A tariff's structure as 2464 sayılı Kanun lays it out, for a municipality to start from: the inputs, the building
/// fee's table (Ek Madde 6: use and the total area's bracket), the planning fees (m.80, m.84: by zone) and the occupancy
/// fee. Every rate is 0: the fees' amounts are the law's, as decided for the municipality's group (2464 m.95, m.96/A;
/// 2005/8730 sayılı BKK) and revalued each year; the charges' are its council's tariff (m.97).
pub fn template() -> FeeDefinition {
    let choice = |value: &str, label: &str| FeeChoice { value: value.into(), label: label.into() };
    let p = |key: &str, label: &str, kind: FeeParameterKind, unit: Option<&str>, choices: Vec<FeeChoice>, help: &str| {
        FeeParameter {
            key: key.into(),
            label: label.into(),
            kind,
            unit: unit.map(str::to_owned),
            choices,
            required: kind != FeeParameterKind::Flag,
            help: (!help.is_empty()).then(|| help.to_owned()),
        }
    };
    let bracket = |when: &str, from: Option<f64>, to: Option<f64>| FeeTableRow {
        when: Some(when.into()),
        from,
        to,
        rate: 0.0,
        min_rate: None,
        max_rate: None,
    };
    let item = |code: &str, label: &str, basis: &str, rule: FeeRule, kinds: &[&str], before: bool| FeeItem {
        code: code.into(),
        label: label.into(),
        legal_basis: basis.into(),
        group: "Harç".into(),
        rule,
        applies_to: kinds.iter().map(|k| (*k).to_owned()).collect(),
        condition: Some("değil @kamu_istisnasi".into()),
        minimum: None,
        maximum: None,
        vat_percent: 0.0,
        before_decision: before,
    };
    let per = |quantity: &str, table: &str| FeeRule::PerUnit {
        quantity: quantity.into(),
        table: Some(table.into()),
        rate: None,
    };
    FeeDefinition {
        parameters: vec![
            p("yapi_turu", "Yapının kullanımı", FeeParameterKind::Choice, None,
                vec![choice("konut", "Konut"), choice("isyeri", "İşyeri")], "Bina inşaat harcı konut ve işyeri için ayrı tarifelidir."),
            p("insaat_alani", "İnşaat alanı", FeeParameterKind::Number, Some("m²"), Vec::new(),
                "Ruhsata esas toplam inşaat alanı (ilavede yalnız ilave edilen kısım)."),
            p("bolge", "İmar bölgesi", FeeParameterKind::Choice, None,
                vec![choice("konut", "Konut bölgesi"), choice("ticaret", "Ticaret bölgesi")], "İmarla ilgili harçlar bölgeye göre ayrıdır."),
            p("kamu_istisnasi", "Harçtan istisna yapı", FeeParameterKind::Flag, None, Vec::new(),
                "Kamu bütçesinden yapılan, sağlık, sanayi ve organize sanayi yapıları (2464 Ek Madde 2, m.80)."),
        ],
        tables: vec![
            FeeTable {
                key: "bina_insaat_birim".into(),
                label: "Bina inşaat harcı birim tutarı".into(),
                by: Some("yapi_turu".into()),
                quantity: Some("insaat_alani".into()),
                rows: vec![
                    bracket("konut", None, Some(100.0)),
                    bracket("konut", Some(101.0), Some(120.0)),
                    bracket("konut", Some(121.0), Some(150.0)),
                    bracket("konut", Some(151.0), Some(200.0)),
                    bracket("konut", Some(201.0), None),
                    bracket("isyeri", None, Some(25.0)),
                    bracket("isyeri", Some(26.0), Some(50.0)),
                    bracket("isyeri", Some(51.0), Some(100.0)),
                    bracket("isyeri", Some(101.0), None),
                ],
            },
            FeeTable {
                key: "proje_tasdik_birim".into(),
                label: "Plan ve proje tasdik harcı birim tutarı".into(),
                by: Some("bolge".into()),
                quantity: None,
                rows: vec![bracket("konut", None, None), bracket("ticaret", None, None)],
            },
            FeeTable {
                key: "yapi_kullanma_birim".into(),
                label: "Yapı kullanma izni harcı birim tutarı".into(),
                by: Some("bolge".into()),
                quantity: None,
                rows: vec![bracket("konut", None, None), bracket("ticaret", None, None)],
            },
        ],
        items: vec![
            item("bina_insaat_harci", "Bina inşaat harcı", "2464 sayılı Kanun Ek Madde 1, 5 ve 6",
                per("insaat_alani", "bina_insaat_birim"), &["building_permit", "architectural_project"], true),
            item("proje_tasdik_harci", "Plan ve proje tasdik harcı", "2464 sayılı Kanun m.80/c ve m.84",
                per("insaat_alani", "proje_tasdik_birim"),
                &["building_permit", "architectural_project", "structural_project", "mechanical_project", "electrical_project"], true),
            item("yapi_kullanma_harci", "Yapı kullanma izni harcı", "2464 sayılı Kanun m.80/f ve m.84",
                per("insaat_alani", "yapi_kullanma_birim"), &["occupancy_permit"], true),
        ],
        rounding: FeeRounding::Kurus,
        payment_note: None,
    }
}

/// Every rate, fixed amount and bound of `def` times `factor` (a year's revaluation), to the kuruş.
pub fn revalued(def: &FeeDefinition, factor: f64) -> FeeDefinition {
    let r = |x: f64| rounded(x * factor, FeeRounding::Kurus) as f64 / 100.0;
    let mut out = def.clone();
    for t in &mut out.tables {
        for row in &mut t.rows {
            row.rate = r(row.rate);
            row.min_rate = row.min_rate.map(r);
            row.max_rate = row.max_rate.map(r);
        }
    }
    for item in &mut out.items {
        match &mut item.rule {
            FeeRule::Fixed { amount } => *amount = r(*amount),
            FeeRule::PerUnit { rate: Some(rate), .. } => *rate = r(*rate),
            _ => {}
        }
        item.minimum = item.minimum.map(r);
        item.maximum = item.maximum.map(r);
    }
    out
}

/// `POST /v1/tenants/{tenant}/fees/try`: a tariff tried before it is saved (its members'; nothing is written).
pub fn try_out(input: &FeeTry) -> AppResult<FeePreview> {
    validate(&input.definition)?;
    let e = evaluate(&input.definition, &input.kind, &input.inputs);
    Ok(FeePreview {
        schedule_id: String::new(),
        schedule_name: "Deneme".into(),
        parameters: e.parameters,
        lines: e.lines,
        total: e.total,
        missing: e.missing,
    })
}

// ── Tariffs ─────────────────────────────────────────────────────────────────

type ScheduleRow = (
    Uuid,
    String,
    i32,
    Date,
    Option<Date>,
    String,
    Option<String>,
    Json<FeeDefinition>,
    OffsetDateTime,
    String,
);

const SCHEDULE_SELECT: &str = "select id, name, year, valid_from, valid_to, state, decision, definition,
        updated_at, updated_by_name from kentos.fee_schedule";

fn state_of(code: &str) -> FeeScheduleState {
    match code {
        "active" => FeeScheduleState::Active,
        "retired" => FeeScheduleState::Retired,
        _ => FeeScheduleState::Draft,
    }
}

fn schedule_view(r: ScheduleRow) -> FeeSchedule {
    let (id, name, year, from, to, state, decision, Json(definition), updated, by) = r;
    FeeSchedule {
        id: id.to_string(),
        name,
        year,
        valid_from: from.to_string(),
        valid_to: to.map(|d| d.to_string()),
        state: state_of(&state),
        decision,
        definition,
        updated_at: rfc3339(updated),
        updated_by_name: by,
    }
}

fn schedule_missing() -> AppError {
    AppError::not_found("Böyle bir ücret tarifesi yok.")
}

/// The fields of an edit, checked.
fn edit_fields(edit: &FeeScheduleEdit) -> AppResult<(String, Date, Option<Date>, Option<String>)> {
    let name = edit.name.trim().to_owned();
    if !(3..=120).contains(&name.chars().count()) {
        return Err(invalid("name", "Tarifenin adı 3–120 karakter olmalı."));
    }
    if !(2000..=2200).contains(&edit.year) {
        return Err(invalid("year", "Yıl geçerli değil."));
    }
    let from = date_at("validFrom", Some(&edit.valid_from))?
        .ok_or_else(|| invalid("validFrom", "Tarifenin yürürlüğe gireceği günü yazın."))?;
    let to = date_at("validTo", edit.valid_to.as_deref())?;
    if to.is_some_and(|t| t < from) {
        return Err(invalid("validTo", "Bitiş başlangıçtan önce olamaz."));
    }
    let decision = edit.decision.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(str::to_owned);
    if decision.as_ref().is_some_and(|d| d.chars().count() > 300) {
        return Err(invalid("decision", "Dayanak karar en çok 300 karakter olabilir."));
    }
    validate(&edit.definition)?;
    Ok((name, from, to, decision))
}

/// `GET /v1/tenants/{tenant}/fees/schedules`: newest year first.
pub async fn schedules(db: &Db, access: &Access) -> AppResult<Vec<FeeSchedule>> {
    let mut tx = db.scoped(access.scope()).await?;
    let rows: Vec<ScheduleRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{SCHEDULE_SELECT} where tenant_id = $1 order by year desc, valid_from desc"
    )))
    .bind(access.tenant)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(rows.into_iter().map(schedule_view).collect())
}

async fn one(tx: &mut Transaction<'static, Postgres>, tenant: Uuid, id: Uuid) -> AppResult<ScheduleRow> {
    sqlx::query_as(sqlx::AssertSqlSafe(format!("{SCHEDULE_SELECT} where tenant_id = $1 and id = $2")))
        .bind(tenant)
        .bind(id)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or_else(schedule_missing)
}

pub async fn schedule(db: &Db, access: &Access, id: Uuid) -> AppResult<FeeSchedule> {
    let mut tx = db.scoped(access.scope()).await?;
    let row = one(&mut tx, access.tenant, id).await?;
    tx.commit().await?;
    Ok(schedule_view(row))
}

/// `POST /v1/tenants/{tenant}/fees/schedules`: a new draft (the admins').
pub async fn create_schedule(db: &Db, access: &Access, edit: &FeeScheduleEdit) -> AppResult<FeeSchedule> {
    require_role(access, TenantRole::Admin, "Ücret tarifesi yazmak")?;
    let (name, from, to, decision) = edit_fields(edit)?;
    let mut tx = db.scoped(access.scope()).await?;
    let id = Uuid::now_v7();
    sqlx::query(
        "insert into kentos.fee_schedule (tenant_id, id, name, year, valid_from, valid_to, decision, definition,
                                          created_by, updated_by, updated_by_name)
         values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $9, $10)",
    )
    .bind(access.tenant)
    .bind(id)
    .bind(&name)
    .bind(edit.year)
    .bind(from)
    .bind(to)
    .bind(&decision)
    .bind(Json(&edit.definition))
    .bind(access.actor.user_id)
    .bind(&access.actor.display_name)
    .execute(&mut *tx)
    .await?;
    let row = one(&mut tx, access.tenant, id).await?;
    tx.commit().await?;
    Ok(schedule_view(row))
}

/// `PUT …/fees/schedules/{id}`: a draft changes; an active or retired tariff does not (copy it to a new draft).
pub async fn update_schedule(db: &Db, access: &Access, id: Uuid, edit: &FeeScheduleEdit) -> AppResult<FeeSchedule> {
    require_role(access, TenantRole::Admin, "Ücret tarifesi yazmak")?;
    let (name, from, to, decision) = edit_fields(edit)?;
    let mut tx = db.scoped(access.scope()).await?;
    let now = one(&mut tx, access.tenant, id).await?;
    if now.5 != "draft" {
        return Err(AppError::invalid(
            "Yürürlükteki ya da geçmiş bir tarife değişmez; kopyalayıp yeni bir taslak yazın.",
        ));
    }
    sqlx::query(
        "update kentos.fee_schedule set name = $3, year = $4, valid_from = $5, valid_to = $6, decision = $7,
                definition = $8, updated_by = $9, updated_by_name = $10, updated_at = now()
          where tenant_id = $1 and id = $2",
    )
    .bind(access.tenant)
    .bind(id)
    .bind(&name)
    .bind(edit.year)
    .bind(from)
    .bind(to)
    .bind(&decision)
    .bind(Json(&edit.definition))
    .bind(access.actor.user_id)
    .bind(&access.actor.display_name)
    .execute(&mut *tx)
    .await?;
    let row = one(&mut tx, access.tenant, id).await?;
    tx.commit().await?;
    Ok(schedule_view(row))
}

/// `DELETE …/fees/schedules/{id}`: a draft only.
pub async fn delete_schedule(db: &Db, access: &Access, id: Uuid) -> AppResult<()> {
    require_role(access, TenantRole::Admin, "Ücret tarifesi silmek")?;
    let mut tx = db.scoped(access.scope()).await?;
    let gone = sqlx::query("delete from kentos.fee_schedule where tenant_id = $1 and id = $2 and state = 'draft'")
        .bind(access.tenant)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    if gone.rows_affected() == 0 {
        return Err(AppError::invalid("Yalnız taslak bir tarife silinir."));
    }
    Ok(())
}

/// `POST …/fees/schedules/{id}/activate`: the draft becomes the active tariff; the one it replaces is retired.
pub async fn activate(db: &Db, access: &Access, id: Uuid) -> AppResult<FeeSchedule> {
    require_role(access, TenantRole::Admin, "Ücret tarifesini yürürlüğe koymak")?;
    let mut tx = db.scoped(access.scope()).await?;
    let now = one(&mut tx, access.tenant, id).await?;
    if now.5 != "draft" {
        return Err(AppError::invalid("Yalnız taslak bir tarife yürürlüğe girer."));
    }
    validate(&now.7.0)?;
    if now.7.0.items.is_empty() {
        return Err(AppError::invalid("Tarifede kalem yok; önce harç ve ücret kalemlerini yazın."));
    }
    sqlx::query("update kentos.fee_schedule set state = 'retired', updated_at = now() where tenant_id = $1 and state = 'active'")
        .bind(access.tenant)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "update kentos.fee_schedule set state = 'active', updated_by = $3, updated_by_name = $4, updated_at = now()
          where tenant_id = $1 and id = $2",
    )
    .bind(access.tenant)
    .bind(id)
    .bind(access.actor.user_id)
    .bind(&access.actor.display_name)
    .execute(&mut *tx)
    .await?;
    let row = one(&mut tx, access.tenant, id).await?;
    tx.commit().await?;
    Ok(schedule_view(row))
}

/// `POST …/fees/schedules/{id}/copy`: next year's draft, every amount revalued by the factor.
pub async fn copy_schedule(db: &Db, access: &Access, id: Uuid, copy: &FeeScheduleCopy) -> AppResult<FeeSchedule> {
    if !copy.factor.is_finite() || copy.factor <= 0.0 || copy.factor > 100.0 {
        return Err(invalid("factor", "Yeniden değerleme katsayısı 0 ile 100 arasında olmalı (yüzde 25 artış için 1,25)."));
    }
    let mut tx = db.scoped(access.scope()).await?;
    let from = schedule_view(one(&mut tx, access.tenant, id).await?);
    tx.commit().await?;
    let edit = FeeScheduleEdit {
        name: format!("{} Ücret Tarifesi", copy.year),
        year: copy.year,
        valid_from: copy.valid_from.clone(),
        valid_to: None,
        decision: None,
        definition: revalued(&from.definition, copy.factor),
    };
    create_schedule(db, access, &edit).await
}

// ── Assessments ─────────────────────────────────────────────────────────────

type FeeRow = (
    Uuid,
    Uuid,
    String,
    Json<FeeInputs>,
    Json<Vec<FeeLine>>,
    i64,
    String,
    Option<Date>,
    String,
    OffsetDateTime,
    Option<Date>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);

const FEE_SELECT: &str = "select id, schedule_id, schedule_name, inputs, lines, total, state, due_on, issued_by_name,
        issued_at, paid_at, receipt_no, paid_recorded_by_name, cancel_reason, payment_note from kentos.dossier_fee";

fn fee_view(r: FeeRow) -> FeeAssessment {
    let (id, schedule, name, Json(inputs), Json(lines), total, state, due, by, at, paid, receipt, recorder, reason, note) =
        r;
    FeeAssessment {
        id: id.to_string(),
        schedule_id: schedule.to_string(),
        schedule_name: name,
        inputs,
        lines,
        total,
        state: match state.as_str() {
            "paid" => FeeAssessmentState::Paid,
            "cancelled" => FeeAssessmentState::Cancelled,
            _ => FeeAssessmentState::Issued,
        },
        due_on: due.map(|d| d.to_string()),
        issued_at: rfc3339(at),
        issued_by_name: by,
        paid_at: paid.map(|d| d.to_string()),
        receipt_no: receipt,
        paid_recorded_by_name: recorder,
        cancel_reason: reason,
        payment_note: note,
    }
}

/// The tariff in force today for the dossier's municipality, if it has one.
async fn in_force(tx: &mut Transaction<'static, Postgres>, tenant: Uuid) -> AppResult<Option<FeeSchedule>> {
    let row: Option<ScheduleRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{SCHEDULE_SELECT} where tenant_id = $1 and state = 'active' and valid_from <= current_date
           and (valid_to is null or valid_to >= current_date)"
    )))
    .bind(tenant)
    .fetch_optional(&mut **tx)
    .await?;
    Ok(row.map(schedule_view))
}

/// The dossier held by a member of its municipality of at least `role`, else why not.
async fn held_by_staff(
    tx: &mut Transaction<'static, Postgres>,
    actor: &Actor,
    dossier: Uuid,
    role: TenantRole,
) -> AppResult<Held> {
    let held = hold(tx, actor, dossier, true).await?;
    match held.party {
        Party::Member(r) if r >= role => Ok(held),
        Party::Member(_) => Err(AppError::forbidden(match role {
            TenantRole::ProjectManager => "Tahakkuku iptal etmek kurumunuzda proje yöneticisi ve üstünün işidir.",
            _ => "Harç ve ücret işlemleri kurumunuzda en az düzenleyicinin işidir.",
        })),
        _ => Err(AppError::forbidden("Harç ve ücreti belediye tahakkuk ettirir.")),
    }
}

/// `GET /v1/dossiers/{id}/fees`: both sides.
pub async fn dossier_fees(db: &Db, actor: &Actor, dossier: Uuid) -> AppResult<Vec<FeeAssessment>> {
    let mut tx = db.scoped(user_scope(actor)).await?;
    hold(&mut tx, actor, dossier, false).await?;
    let rows: Vec<FeeRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{FEE_SELECT} where dossier_id = $1 order by issued_at"
    )))
    .bind(dossier)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(rows.into_iter().map(fee_view).collect())
}

fn preview_of(s: &FeeSchedule, e: Evaluated) -> FeePreview {
    FeePreview {
        schedule_id: s.id.clone(),
        schedule_name: s.name.clone(),
        parameters: e.parameters,
        lines: e.lines,
        total: e.total,
        missing: e.missing,
    }
}

fn no_tariff() -> AppError {
    AppError::invalid(
        "Kurumunuzun yürürlükte bir ücret tarifesi yok; önce Harç ve ücretler sayfasında tarifeyi yürürlüğe koyun.",
    )
}

/// `POST /v1/dossiers/{id}/fees/preview`: what the tariff in force makes of these inputs; nothing is saved.
pub async fn preview(db: &Db, actor: &Actor, dossier: Uuid, input: &FeeAssess) -> AppResult<FeePreview> {
    let mut tx = db.scoped(user_scope(actor)).await?;
    let held = held_by_staff(&mut tx, actor, dossier, TenantRole::Editor).await?;
    let schedule = in_force(&mut tx, held.row.tenant_id).await?.ok_or_else(no_tariff)?;
    tx.commit().await?;
    let e = evaluate(&schedule.definition, &held.row.kind, &input.inputs);
    Ok(preview_of(&schedule, e))
}

/// `POST /v1/dossiers/{id}/fees`: issues the tariff's assessment (tahakkuk); the professional is told the total.
pub async fn issue(db: &Db, actor: &Actor, dossier: Uuid, input: &FeeAssess) -> AppResult<FeeAssessment> {
    let mut tx = db.scoped(user_scope(actor)).await?;
    let held = held_by_staff(&mut tx, actor, dossier, TenantRole::Editor).await?;
    if held.direction() != kentos_contracts::DossierDirection::Application {
        return Err(AppError::invalid("Harç ve ücret başvurulara tahakkuk eder; aktarılan işe değil."));
    }
    if flow::closed(held.status()) {
        return Err(AppError::invalid("Kapanmış dosyaya tahakkuk yapılmaz."));
    }
    let schedule = in_force(&mut tx, held.row.tenant_id).await?.ok_or_else(no_tariff)?;
    let e = evaluate(&schedule.definition, &held.row.kind, &input.inputs);
    if let Some((key, why)) = e.missing.iter().next() {
        return Err(invalid(format!("inputs.{key}"), why.clone()));
    }
    if e.lines.is_empty() {
        return Err(AppError::invalid("Bu dosyaya tarifenin hiçbir kalemi uymuyor; tahakkuk edilecek tutar yok."));
    }
    let due = date_at("dueOn", input.due_on.as_deref())?;
    if due.is_some_and(|d| d < today()) {
        return Err(invalid("dueOn", "Son ödeme günü geçmiş bir gün olamaz."));
    }
    let schedule_id = Uuid::parse_str(&schedule.id).map_err(|_| schedule_missing())?;
    let used: FeeInputs = e
        .parameters
        .iter()
        .filter_map(|p| input.inputs.get(&p.key).map(|v| (p.key.clone(), v.clone())))
        .collect();
    let id = Uuid::now_v7();
    let row: FeeRow = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "with made as (
           insert into kentos.dossier_fee (id, dossier_id, tenant_id, schedule_id, schedule_name, inputs, lines, total,
                                           payment_note, due_on, issued_by, issued_by_name)
           values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12) returning *)
         {}",
        FEE_SELECT.replace("from kentos.dossier_fee", "from made")
    )))
    .bind(id)
    .bind(dossier)
    .bind(held.row.tenant_id)
    .bind(schedule_id)
    .bind(&schedule.name)
    .bind(Json(&used))
    .bind(Json(&e.lines))
    .bind(e.total)
    .bind(&schedule.definition.payment_note)
    .bind(due)
    .bind(actor.user_id)
    .bind(&actor.display_name)
    .fetch_one(&mut *tx)
    .await?;
    let side = held.party.side();
    record(&mut tx, dossier, actor, side, "fee_issued", held.row.round(), serde_json::json!({ "total": tl(e.total) }))
        .await?;
    tell(
        &mut tx,
        dossier,
        DossierSide::Professional,
        "dossier.fee_issued",
        &format!("Harç ve ücret tahakkuk etti: {}", held.row.title),
        &format!("Ödenecek tutar {}.", tl(e.total)),
    )
    .await?;
    tx.commit().await?;
    Ok(fee_view(row))
}

async fn held_fee(tx: &mut Transaction<'static, Postgres>, dossier: Uuid, fee: Uuid) -> AppResult<FeeRow> {
    sqlx::query_as(sqlx::AssertSqlSafe(format!("{FEE_SELECT} where dossier_id = $1 and id = $2 for update")))
        .bind(dossier)
        .bind(fee)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or_else(|| AppError::not_found("Böyle bir tahakkuk yok."))
}

/// `POST /v1/dossiers/{id}/fees/{fee}/paid`: the receipt that it was paid.
pub async fn paid(db: &Db, actor: &Actor, dossier: Uuid, fee: Uuid, input: &FeePaid) -> AppResult<FeeAssessment> {
    let receipt = input.receipt_no.trim();
    if !(1..=60).contains(&receipt.chars().count()) {
        return Err(invalid("receiptNo", "Makbuz numarasını yazın (en çok 60 karakter)."));
    }
    let on = date_at("paidOn", input.paid_on.as_deref())?.unwrap_or_else(today);
    if on > today() {
        return Err(invalid("paidOn", "Ödeme günü ileri bir gün olamaz."));
    }
    let mut tx = db.scoped(user_scope(actor)).await?;
    let held = held_by_staff(&mut tx, actor, dossier, TenantRole::Editor).await?;
    let now = held_fee(&mut tx, dossier, fee).await?;
    if now.6 != "issued" {
        return Err(AppError::invalid("Bu tahakkuk ödenecek durumda değil (ödenmiş ya da iptal edilmiş)."));
    }
    let row: FeeRow = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "with done as (
           update kentos.dossier_fee set state = 'paid', paid_at = $3, receipt_no = $4, paid_recorded_by_name = $5
            where dossier_id = $1 and id = $2 returning *)
         {}",
        FEE_SELECT.replace("from kentos.dossier_fee", "from done")
    )))
    .bind(dossier)
    .bind(fee)
    .bind(on)
    .bind(receipt)
    .bind(&actor.display_name)
    .fetch_one(&mut *tx)
    .await?;
    record(
        &mut tx,
        dossier,
        actor,
        held.party.side(),
        "fee_paid",
        held.row.round(),
        serde_json::json!({ "total": tl(now.5), "receipt": receipt }),
    )
    .await?;
    tell(
        &mut tx,
        dossier,
        DossierSide::Professional,
        "dossier.fee_paid",
        &format!("Ödeme kaydedildi: {}", held.row.title),
        &format!("{} ödendi, makbuz {receipt}.", tl(now.5)),
    )
    .await?;
    tx.commit().await?;
    Ok(fee_view(row))
}

/// `POST /v1/dossiers/{id}/fees/{fee}/cancel`: with a reason; a paid one too (its refund is the cash office's).
pub async fn cancel(db: &Db, actor: &Actor, dossier: Uuid, fee: Uuid, input: &FeeCancel) -> AppResult<FeeAssessment> {
    let reason = input.reason.trim();
    if !(3..=1000).contains(&reason.chars().count()) {
        return Err(invalid("reason", "İptalin gerekçesini yazın."));
    }
    let mut tx = db.scoped(user_scope(actor)).await?;
    let held = held_by_staff(&mut tx, actor, dossier, TenantRole::ProjectManager).await?;
    let now = held_fee(&mut tx, dossier, fee).await?;
    if now.6 == "cancelled" {
        return Err(AppError::invalid("Bu tahakkuk zaten iptal edilmiş."));
    }
    let row: FeeRow = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "with done as (
           update kentos.dossier_fee set state = 'cancelled', cancel_reason = $3, cancelled_at = now(),
                  receipt_no = null, paid_at = null
            where dossier_id = $1 and id = $2 returning *)
         {}",
        FEE_SELECT.replace("from kentos.dossier_fee", "from done")
    )))
    .bind(dossier)
    .bind(fee)
    .bind(reason)
    .fetch_one(&mut *tx)
    .await?;
    record(
        &mut tx,
        dossier,
        actor,
        held.party.side(),
        "fee_cancelled",
        held.row.round(),
        serde_json::json!({ "total": tl(now.5), "reason": reason, "wasPaid": now.6 == "paid" }),
    )
    .await?;
    tx.commit().await?;
    Ok(fee_view(row))
}

/// Why an application may not be decided yet for its fees, if so: a line to be paid before the decision is issued and
/// not paid, or the tariff in force has such an item for the dossier's kind and nothing was issued.
pub(crate) async fn decision_refusal(tx: &mut Transaction<'static, Postgres>, held: &Held) -> AppResult<Option<String>> {
    let open: Vec<(Json<Vec<FeeLine>>, String)> =
        sqlx::query_as("select lines, state from kentos.dossier_fee where dossier_id = $1 and state <> 'cancelled'")
            .bind(held.id())
            .fetch_all(&mut **tx)
            .await?;
    if open.iter().any(|(lines, state)| state == "issued" && lines.0.iter().any(|l| l.before_decision)) {
        return Ok(Some(
            "Onaydan önce ödenmesi gereken harç ödenmedi (2464 sayılı Kanun Ek Madde 5); ödeme kaydedilince onaylayın."
                .into(),
        ));
    }
    if open.is_empty()
        && let Some(s) = in_force(tx, held.row.tenant_id).await?
        && s.definition.items.iter().any(|i| {
            i.before_decision && (i.applies_to.is_empty() || i.applies_to.iter().any(|k| k == &held.row.kind))
        })
    {
        return Ok(Some(
            "Bu başvuruya onaydan önce harç tahakkuk ettirilmeli; önce Harç ve ücret bölümünden tahakkuk edin.".into(),
        ));
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The template with example rates: the building fee's brackets and the planning fees by zone.
    fn priced() -> FeeDefinition {
        let mut d = template();
        let rates = [12.0, 24.0, 36.0, 48.0, 60.0, 48.0, 72.0, 96.0, 120.0];
        for (row, r) in d.tables[0].rows.iter_mut().zip(rates) {
            row.rate = r;
        }
        d.tables[1].rows[0].rate = 1.25;
        d.tables[1].rows[1].rate = 3.75;
        d.tables[2].rows[0].rate = 1.25;
        d.tables[2].rows[1].rate = 3.75;
        d
    }

    fn inputs(pairs: &[(&str, serde_json::Value)]) -> FeeInputs {
        pairs.iter().map(|(k, v)| ((*k).to_owned(), v.clone())).collect()
    }

    #[test]
    fn the_building_fee_takes_the_rate_of_the_whole_areas_bracket() {
        let d = priced();
        validate(&d).unwrap();
        let e = evaluate(
            &d,
            "building_permit",
            &inputs(&[
                ("yapi_turu", serde_json::json!("konut")),
                ("insaat_alani", serde_json::json!("152,5")),
                ("bolge", serde_json::json!("ticaret")),
            ]),
        );
        assert!(e.missing.is_empty(), "{:?}", e.missing);
        let building = &e.lines[0];
        // 152,5 m² is in 151–200 m²: the whole area at 48 TL.
        assert_eq!(building.amount, 732_000);
        assert_eq!(building.basis, "152,5 m² × 48 TL (Konut, 151–200 m²)");
        let project = &e.lines[1];
        assert_eq!(project.amount, 57_188, "152,5 × 3,75 = 571,875 → 571,88 TL");
        assert_eq!(e.total, 732_000 + 57_188);
        assert_eq!(tl(e.total), "7.891,88 TL");
        // An occupancy permit takes only its own item.
        let e = evaluate(&d, "occupancy_permit", &inputs(&[("insaat_alani", serde_json::json!(200)), ("bolge", serde_json::json!("konut"))]));
        assert_eq!(e.lines.iter().map(|l| l.code.as_str()).collect::<Vec<_>>(), ["yapi_kullanma_harci"]);
        assert_eq!(e.lines[0].amount, 25_000);
    }

    #[test]
    fn an_exemption_a_formula_bounds_vat_and_what_is_missing() {
        let mut d = priced();
        d.items.push(FeeItem {
            code: "kat_irtifaki_ucreti".into(),
            label: "Kat irtifakı proje onay ücreti".into(),
            legal_basis: "2464 sayılı Kanun m.97, meclis tarifesi".into(),
            group: "Ücret".into(),
            rule: FeeRule::Formula { expression: "@insaat_alani * 2 + 150".into() },
            applies_to: Vec::new(),
            condition: Some("@yapi_turu = 'konut'".into()),
            minimum: Some(500.0),
            maximum: Some(1000.0),
            vat_percent: 20.0,
            before_decision: false,
        });
        validate(&d).unwrap();
        let base = [("yapi_turu", serde_json::json!("konut")), ("insaat_alani", serde_json::json!(100)), ("bolge", serde_json::json!("konut"))];
        let e = evaluate(&d, "building_permit", &inputs(&base));
        let fee = e.lines.iter().find(|l| l.code == "kat_irtifaki_ucreti").unwrap();
        // 100 × 2 + 150 = 350, raised to the minimum 500 TL, with 20% VAT.
        assert_eq!((fee.amount, fee.vat), (50_000, 10_000));
        assert!(fee.basis.ends_with("; en az 500 TL"), "{}", fee.basis);
        // Exempt: the law's fees go, the charge without the exemption's condition stays.
        let mut exempt = inputs(&base);
        exempt.insert("kamu_istisnasi".into(), serde_json::json!(true));
        let e = evaluate(&d, "building_permit", &exempt);
        assert_eq!(e.lines.iter().map(|l| l.code.as_str()).collect::<Vec<_>>(), ["kat_irtifaki_ucreti"]);
        // Missing and wrong inputs are named, and their lines are not made.
        let e = evaluate(&d, "building_permit", &inputs(&[("yapi_turu", serde_json::json!("ahır"))]));
        assert!(e.missing.contains_key("yapi_turu") && e.missing.contains_key("insaat_alani"), "{:?}", e.missing);
        assert!(e.lines.is_empty());
    }

    #[test]
    fn a_tariff_that_could_not_be_applied_is_refused_where_it_is_wrong() {
        let mut d = priced();
        d.items[0].rule = FeeRule::Formula { expression: "@alan * @olmayan".into() };
        let e = validate(&d).unwrap_err();
        assert_eq!(e.path(), Some("definition.items[0].rule.expression"));
        let mut d = priced();
        d.tables[0].rows[0].min_rate = Some(20.0);
        assert!(validate(&d).unwrap_err().to_string().contains("sınırların dışında (en az 20 TL)"));
        let mut d = priced();
        d.parameters[1].key = "İnşaat".into();
        assert!(validate(&d).is_err());
    }

    #[test]
    fn a_year_is_revalued_and_halves_round_up() {
        let d = revalued(&priced(), 1.4393);
        assert_eq!(d.tables[0].rows[0].rate, 17.27, "12 × 1,4393 = 17,2716");
        assert_eq!(d.tables[1].rows[1].rate, 5.4, "3,75 × 1,4393 = 5,397375 → 5,40");
        assert_eq!(rounded(1.005, FeeRounding::Kurus), 101);
        assert_eq!(rounded(2.675, FeeRounding::Kurus), 268);
        assert_eq!(rounded(1234.5, FeeRounding::Lira), 123_500);
        assert_eq!(rounded(1234.99, FeeRounding::LiraDown), 123_400);
        assert_eq!(number(0.125), "0,125");
        assert_eq!(tl(-50), "-0,50 TL");
    }
}
