//! A layer style's classes as the layer tree lists them (docs/adr/9105 §2): one row per category and “Diğer
//! değerler”, per graduated class, per rule with its children after it, read from the renderer's JSON (docs/STYLE.md
//! §4). Each says its id, its label, whether it draws, its symbols and its condition in the expression language.
//!
//! [`ClassReader`] says which objects each class takes by the drawing's own rule (the style core's `resolve`): an
//! object goes to the first category of its value and to the first class that takes its number (the last one's
//! maximum included), a child rule takes among its parent's objects, a “değilse” rule what no enabled sibling took.
//! Only objects the style engine draws count (texts and dimensions are drawn elsewhere). A class switched off keeps its
//! objects: it says what it takes and draws nothing. Its expressions are compiled once and asked of any number of
//! object lists (a data layer's rows come a part at a time).
//!
//! [`set_enabled`] writes the renderer's `enabled` flags (`otherEnabled` for Diğer değerler), only `false` being
//! written; what it does not know of the renderer stays as it is. A renderer without classes (tek sembol, the thematic
//! ones) lists none.

use std::collections::HashMap;

use kentos_commands::Entity;
use kentos_expression::rows::As;
use kentos_expression::value::Value as Read;
use kentos_expression::{Expr, Schema, Unit, compile_with};
use kentos_geometry_core::entity::Shape;
use serde_json::{Map, Value};

use crate::layer_filter::evaluate_objects;

/// The id of a categorized renderer's “Diğer değerler”.
pub const OTHER: &str = "other";

/// A class of a layer's renderer, as a row of the layer tree.
#[derive(Clone, Debug, PartialEq)]
pub struct StyleClass {
    /// `c3` (a category), `other`, `g2` (a graduated class), `r0.1` (a rule by its path).
    pub id: String,
    pub label: String,
    /// Whether it draws; switched off, its objects are drawn by nothing.
    pub enabled: bool,
    /// A rule's depth under the top (0 for every other class).
    pub depth: usize,
    /// What it draws with (the renderer's symbol set); none: a rule that only groups its children.
    pub symbols: Option<Value>,
    /// Its condition in the expression language; none: every object of its layer.
    pub expression: Option<String>,
}

/// The renderer's kind when it has classes: `categorized`, `graduated` or `rules`.
pub fn classes_kind(renderer: &Value) -> Option<&'static str> {
    match renderer.get("type").and_then(Value::as_str) {
        Some("categorized") => Some("categorized"),
        Some("graduated") => Some("graduated"),
        Some("rules") => Some("rules"),
        _ => None,
    }
}

fn text_of<'a>(v: &'a Value, key: &str) -> &'a str {
    v.get(key).and_then(Value::as_str).unwrap_or_default()
}

fn on(v: &Value, key: &str) -> bool {
    v.get(key) != Some(&Value::Bool(false))
}

fn list<'a>(v: &'a Value, key: &str) -> &'a [Value] {
    v.get(key).and_then(Value::as_array).map(Vec::as_slice).unwrap_or_default()
}

/// The renderer's classes in the tree's order.
pub fn style_classes(renderer: &Value) -> Vec<StyleClass> {
    match classes_kind(renderer) {
        Some("categorized") => categorized(renderer),
        Some("graduated") => graduated(renderer),
        Some("rules") => {
            let mut out = Vec::new();
            rules(list(renderer, "rules"), None, "r", 0, &mut out);
            out
        }
        _ => Vec::new(),
    }
}

fn categorized(r: &Value) -> Vec<StyleClass> {
    let e = text_of(r, "expr").trim();
    let categories = list(r, "categories");
    let mut out: Vec<StyleClass> = categories
        .iter()
        .enumerate()
        .map(|(i, k)| {
            let value = text_of(k, "value");
            StyleClass {
                id: format!("c{i}"),
                label: match (text_of(k, "label").trim(), value) {
                    ("", "") => "(boş)".to_owned(),
                    ("", v) => v.to_owned(),
                    (l, _) => l.to_owned(),
                },
                enabled: on(k, "enabled"),
                depth: 0,
                symbols: k.get("symbols").cloned(),
                expression: (!e.is_empty()).then(|| equals(e, value)),
            }
        })
        .collect();
    if let Some(other) = r.get("other").filter(|o| !o.is_null()) {
        out.push(StyleClass {
            id: OTHER.to_owned(),
            label: "Diğer değerler".to_owned(),
            enabled: on(r, "otherEnabled"),
            depth: 0,
            symbols: Some(other.clone()),
            expression: (!e.is_empty() && !categories.is_empty()).then(|| others(e, categories)),
        });
    }
    out
}

fn graduated(r: &Value) -> Vec<StyleClass> {
    let e = text_of(r, "expr").trim();
    let classes = list(r, "classes");
    let last = classes.len().wrapping_sub(1);
    classes
        .iter()
        .enumerate()
        .map(|(i, k)| StyleClass {
            id: format!("g{i}"),
            label: text_of(k, "label").to_owned(),
            enabled: on(k, "enabled"),
            depth: 0,
            symbols: k.get("symbols").cloned(),
            expression: (!e.is_empty()).then(|| {
                let x = wrap(e);
                let upper = if i == last { "<=" } else { "<" };
                format!("{x} >= {} ve {x} {upper} {}", number(bound(k, "min")), number(bound(k, "max")))
            }),
        })
        .collect()
}

fn bound(k: &Value, key: &str) -> f64 {
    k.get(key).and_then(Value::as_f64).unwrap_or(f64::NAN)
}

fn filter_of(rule: &Value) -> Option<&str> {
    rule.get("filter").and_then(Value::as_str).map(str::trim).filter(|f| !f.is_empty())
}

fn is_else(rule: &Value) -> bool {
    rule.get("isElse") == Some(&Value::Bool(true))
}

fn rules(level: &[Value], parent: Option<&str>, prefix: &str, depth: usize, out: &mut Vec<StyleClass>) {
    // What the enabled plain siblings take: the rest is the “değilse” rules'. A sibling without a condition takes
    // everything.
    let taken: Option<Vec<&str>> = level
        .iter()
        .filter(|r| on(r, "enabled") && !is_else(r))
        .map(filter_of)
        .collect();
    for (i, rule) in level.iter().enumerate() {
        let id = format!("{prefix}{i}");
        let own: Option<String> = if is_else(rule) {
            match &taken {
                Some(t) if t.is_empty() => None,
                Some(t) => Some(format!(
                    "değil ({})",
                    t.iter().map(|f| format!("({f})")).collect::<Vec<_>>().join(" veya ")
                )),
                None => Some("yanlış".to_owned()),
            }
        } else {
            filter_of(rule).map(str::to_owned)
        };
        let expression = match (parent, own) {
            (None, own) => own,
            (Some(p), None) => Some(p.to_owned()),
            (Some(p), Some(o)) => Some(format!("({p}) ve ({o})")),
        };
        let label = text_of(rule, "label").trim();
        out.push(StyleClass {
            id: id.clone(),
            label: if label.is_empty() { "(adsız kural)".to_owned() } else { label.to_owned() },
            enabled: on(rule, "enabled"),
            depth,
            symbols: rule.get("symbols").filter(|s| !s.is_null()).cloned(),
            expression: expression.clone(),
        });
        rules(list(rule, "children"), expression.as_deref(), &format!("{id}."), depth + 1, out);
    }
}

/// The expression bare when it is a field or a single word, in brackets otherwise.
fn wrap(e: &str) -> String {
    let field = e.starts_with('[') && e.ends_with(']') && e[1..].find(']') == Some(e.len() - 2);
    let word = e.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '$');
    if field || word { e.to_owned() } else { format!("({e})") }
}

/// A text in the language: in single quotes, a quote inside written twice.
fn text(v: &str) -> String {
    format!("'{}'", v.replace('\'', "''"))
}

/// A number as the language reads it back.
fn number(x: f64) -> String {
    format!("{x}")
}

/// A category's condition: its value; the empty value takes the objects without one too.
fn equals(e: &str, value: &str) -> String {
    let x = wrap(e);
    if value.is_empty() {
        format!("({x} boş veya {x} = '')")
    } else {
        format!("{x} = {}", text(value))
    }
}

/// “Diğer değerler”'s condition: a value no category holds; without a category of the empty value, an object without
/// one too.
fn others(e: &str, categories: &[Value]) -> String {
    let x = wrap(e);
    let mut values: Vec<&str> = Vec::new();
    for k in categories {
        let v = text_of(k, "value");
        if !values.contains(&v) {
            values.push(v);
        }
    }
    let list = values.iter().map(|v| text(v)).collect::<Vec<_>>().join(", ");
    if values.contains(&"") {
        format!("{x} değil içinde ({list})")
    } else {
        format!("({x} değil içinde ({list}) veya {x} boş)")
    }
}

/// Whether the style engine draws the object (texts and dimensions are drawn elsewhere).
pub fn drawn_by_style(e: &Entity) -> bool {
    !matches!(e, Entity::Text(_) | Entity::Dimension(_))
}

/// An expression compiled, or why it does not compile (“12. karakterde: …”).
type Compiled = Result<Expr, String>;

/// A rule's condition compiled; none: every object (or a “değilse” rule).
struct RuleReader {
    enabled: bool,
    is_else: bool,
    /// It has symbols of its own (else it only groups its children).
    draws: bool,
    filter: Option<Compiled>,
    children: Vec<RuleReader>,
}

enum Reading {
    Categorized {
        expr: Compiled,
        first: HashMap<String, usize>,
        categories: usize,
        other: bool,
    },
    Graduated {
        expr: Compiled,
        bounds: Vec<(f64, f64)>,
    },
    Rules(Vec<RuleReader>),
}

/// A renderer's classes ready to ask of objects: which each takes and how many.
pub struct ClassReader {
    reading: Reading,
    /// Each class switched on, in [`style_classes`]' order.
    enabled: Vec<bool>,
    fields: Vec<String>,
    geometry: bool,
}

fn compile(text: &str, unit: Unit) -> Compiled {
    let schema = Schema {
        unit,
        externals: crate::label_texts::external_defs(),
        ..Schema::default()
    };
    compile_with(text, &schema).map_err(|e| e.text())
}

impl ClassReader {
    /// The renderer's classes with their expressions compiled in `unit` (the project's drawing unit, as the
    /// renderer reads them when it draws); none: the renderer has no classes.
    pub fn new(renderer: &Value, unit: Unit) -> Option<ClassReader> {
        let reading = match classes_kind(renderer)? {
            "categorized" => {
                let mut first = HashMap::new();
                let categories = list(renderer, "categories");
                for (i, k) in categories.iter().enumerate() {
                    first.entry(text_of(k, "value").to_owned()).or_insert(i);
                }
                Reading::Categorized {
                    expr: compile(text_of(renderer, "expr"), unit),
                    first,
                    categories: categories.len(),
                    other: renderer.get("other").is_some_and(|o| !o.is_null()),
                }
            }
            "graduated" => Reading::Graduated {
                expr: compile(text_of(renderer, "expr"), unit),
                bounds: list(renderer, "classes").iter().map(|k| (bound(k, "min"), bound(k, "max"))).collect(),
            },
            _ => {
                fn read(level: &[Value], unit: Unit) -> Vec<RuleReader> {
                    level
                        .iter()
                        .map(|r| RuleReader {
                            enabled: on(r, "enabled"),
                            is_else: is_else(r),
                            draws: r.get("symbols").is_some_and(|s| !s.is_null()),
                            filter: (!is_else(r)).then(|| filter_of(r).map(|f| compile(f, unit))).flatten(),
                            children: read(list(r, "children"), unit),
                        })
                        .collect()
                }
                Reading::Rules(read(list(renderer, "rules"), unit))
            }
        };
        let mut fields: Vec<String> = Vec::new();
        let mut geometry = false;
        let mut note = |c: &Compiled| {
            if let Ok(e) = c {
                for f in &e.fields {
                    if !fields.contains(f) {
                        fields.push(f.clone());
                    }
                }
                geometry |= e.needs.measured || e.needs.vertices || e.needs.kind || e.needs.centroid;
            }
        };
        match &reading {
            Reading::Categorized { expr, .. } | Reading::Graduated { expr, .. } => note(expr),
            Reading::Rules(rules) => {
                fn walk(rules: &[RuleReader], note: &mut dyn FnMut(&Compiled)) {
                    for r in rules {
                        if let Some(f) = &r.filter {
                            note(f);
                        }
                        walk(&r.children, note);
                    }
                }
                walk(rules, &mut note);
            }
        }
        Some(ClassReader {
            reading,
            enabled: style_classes(renderer).iter().map(|c| c.enabled).collect(),
            fields,
            geometry,
        })
    }

    /// Whether the renderer draws each object (docs/adr/9105 §3): its class is switched on; a rule's own symbols, its
    /// rule and every rule above it on. An object no class takes is drawn by nothing. A condition that cannot be read
    /// draws nothing; a layer whose renderer cannot be read at all draws everything (the drawing's own look then).
    pub fn drawn<'s>(
        &self,
        list: &[&Entity],
        layer_name: &dyn Fn(&str) -> String,
        shape: &dyn Fn(usize) -> Option<&'s Shape>,
    ) -> Vec<bool> {
        match &self.reading {
            Reading::Rules(rules) => {
                let drawn: Vec<bool> = list.iter().map(|o| drawn_by_style(o)).collect();
                let mut masks = Vec::new();
                Self::rule_masks(rules, &drawn, list, layer_name, shape, &mut masks);
                // Each rule's place, whether it draws and whether it and the rules above it are on.
                let mut drawing: Vec<bool> = Vec::new();
                fn walk(rules: &[RuleReader], above: bool, out: &mut Vec<bool>) {
                    for r in rules {
                        let on = above && r.enabled;
                        out.push(on && r.draws);
                        walk(&r.children, on, out);
                    }
                }
                walk(rules, true, &mut drawing);
                let mut out = vec![false; list.len()];
                for (mask, draws) in masks.into_iter().zip(drawing) {
                    if let (Ok(m), true) = (mask, draws) {
                        for (o, t) in out.iter_mut().zip(m) {
                            *o |= t;
                        }
                    }
                }
                out
            }
            _ => match self.placed(list, layer_name, shape) {
                Some(Ok(placed)) => placed
                    .into_iter()
                    .map(|p| p.is_some_and(|i| self.enabled.get(i).copied().unwrap_or(false)))
                    .collect(),
                Some(Err(_)) => vec![false; list.len()],
                None => vec![true; list.len()],
            },
        }
    }

    /// The fields the classes read (a data layer's scan asks only for these).
    pub fn fields(&self) -> &[String] {
        &self.fields
    }

    /// Whether a class reads the objects' geometry (`$alan`, `$uzunluk`, `$x`, `$tür`…).
    pub fn reads_geometry(&self) -> bool {
        self.geometry
    }

    /// Which class (by its place in [`style_classes`]' order) each object goes to by its value; rules aside.
    fn placed<'s>(
        &self,
        list: &[&Entity],
        layer_name: &dyn Fn(&str) -> String,
        shape: &dyn Fn(usize) -> Option<&'s Shape>,
    ) -> Option<Result<Vec<Option<usize>>, String>> {
        match &self.reading {
            Reading::Categorized { expr, first, categories, other } => Some(expr.clone().and_then(|e| {
                let column = evaluate_objects(&e, list, layer_name, shape, As::Text)?;
                Ok(column
                    .values()
                    .into_iter()
                    .zip(list)
                    .map(|(v, o)| {
                        if !drawn_by_style(o) {
                            return None;
                        }
                        let key = match v {
                            Read::Text(t) => t.into_owned(),
                            _ => String::new(),
                        };
                        match first.get(&key) {
                            Some(&i) => Some(i),
                            None => other.then_some(*categories),
                        }
                    })
                    .collect())
            })),
            Reading::Graduated { expr, bounds } => Some(expr.clone().and_then(|e| {
                let column = evaluate_objects(&e, list, layer_name, shape, As::TextNumber)?;
                let last = bounds.len().wrapping_sub(1);
                Ok(column
                    .values()
                    .into_iter()
                    .zip(list)
                    .map(|(v, o)| match v {
                        Read::Num(n) if n.is_finite() && drawn_by_style(o) => bounds
                            .iter()
                            .enumerate()
                            .position(|(i, (min, max))| n >= *min && (n < *max || (i == last && n <= *max))),
                        _ => None,
                    })
                    .collect())
            })),
            Reading::Rules(_) => None,
        }
    }

    /// Each rule's objects, in [`style_classes`]' order.
    fn rule_masks<'s>(
        rules: &[RuleReader],
        parent: &[bool],
        list: &[&Entity],
        layer_name: &dyn Fn(&str) -> String,
        shape: &dyn Fn(usize) -> Option<&'s Shape>,
        out: &mut Vec<Result<Vec<bool>, String>>,
    ) {
        let own: Vec<Option<Result<Vec<bool>, String>>> = rules
            .iter()
            .map(|r| {
                if r.is_else {
                    return None;
                }
                Some(match &r.filter {
                    None => Ok(vec![true; list.len()]),
                    Some(Err(e)) => Err(e.clone()),
                    Some(Ok(e)) => evaluate_objects(e, list, layer_name, shape, As::Bool).map(|c| {
                        (0..list.len())
                            .map(|k| c.kinds.get(k) == Some(&3) && c.numbers.get(k) == Some(&1.0))
                            .collect()
                    }),
                })
            })
            .collect();
        let taken: Vec<bool> = (0..list.len())
            .map(|i| {
                rules
                    .iter()
                    .zip(&own)
                    .any(|(r, m)| r.enabled && matches!(m, Some(Ok(m)) if m[i]))
            })
            .collect();
        for (rule, own) in rules.iter().zip(own) {
            let mask = match own {
                Some(Ok(m)) => Ok(parent.iter().zip(m).map(|(a, b)| *a && b).collect()),
                Some(Err(e)) => Err(e),
                None => Ok(parent.iter().zip(&taken).map(|(a, t)| *a && !t).collect()),
            };
            let below = mask.clone().unwrap_or_else(|_| vec![false; list.len()]);
            out.push(mask);
            Self::rule_masks(&rule.children, &below, list, layer_name, shape, out);
        }
    }

    /// How many of `list` each class takes, in [`style_classes`]' order, or why its expression cannot be read.
    pub fn counts<'s>(
        &self,
        list: &[&Entity],
        layer_name: &dyn Fn(&str) -> String,
        shape: &dyn Fn(usize) -> Option<&'s Shape>,
    ) -> Vec<Result<u64, String>> {
        let n = self.len();
        match &self.reading {
            Reading::Rules(rules) => {
                let drawn: Vec<bool> = list.iter().map(|o| drawn_by_style(o)).collect();
                let mut masks = Vec::new();
                Self::rule_masks(rules, &drawn, list, layer_name, shape, &mut masks);
                masks
                    .into_iter()
                    .map(|m| m.map(|m| m.iter().filter(|t| **t).count() as u64))
                    .collect()
            }
            _ => match self.placed(list, layer_name, shape) {
                Some(Ok(placed)) => {
                    let mut out = vec![0u64; n];
                    for i in placed.into_iter().flatten() {
                        if let Some(c) = out.get_mut(i) {
                            *c += 1;
                        }
                    }
                    out.into_iter().map(Ok).collect()
                }
                Some(Err(e)) => vec![Err(e); n],
                None => Vec::new(),
            },
        }
    }

    /// Which of `list` the class at `at` ([`style_classes`]' order) takes, or why its expression cannot be read.
    pub fn members<'s>(
        &self,
        list: &[&Entity],
        layer_name: &dyn Fn(&str) -> String,
        shape: &dyn Fn(usize) -> Option<&'s Shape>,
        at: usize,
    ) -> Result<Vec<bool>, String> {
        match &self.reading {
            Reading::Rules(rules) => {
                let drawn: Vec<bool> = list.iter().map(|o| drawn_by_style(o)).collect();
                let mut masks = Vec::new();
                Self::rule_masks(rules, &drawn, list, layer_name, shape, &mut masks);
                masks.into_iter().nth(at).unwrap_or_else(|| Ok(vec![false; list.len()]))
            }
            _ => match self.placed(list, layer_name, shape) {
                Some(Ok(placed)) => Ok(placed.into_iter().map(|p| p == Some(at)).collect()),
                Some(Err(e)) => Err(e),
                None => Ok(vec![false; list.len()]),
            },
        }
    }

    /// How many classes there are ([`style_classes`]' length).
    fn len(&self) -> usize {
        match &self.reading {
            Reading::Categorized { categories, other, .. } => categories + usize::from(*other),
            Reading::Graduated { bounds, .. } => bounds.len(),
            Reading::Rules(rules) => {
                fn count(rules: &[RuleReader]) -> usize {
                    rules.iter().map(|r| 1 + count(&r.children)).sum()
                }
                count(rules)
            }
        }
    }
}

/// How [`set_enabled`] changes the classes it names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Switch {
    On,
    Off,
    /// Each one the other way.
    Invert,
}

/// Why `set_enabled` wrote nothing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SwitchRefused {
    /// The renderer lists no class (tek sembol, a thematic one, none).
    NoClasses,
    /// `ids[i]` names no class of the renderer.
    UnknownClass(usize),
}

/// The renderer (its JSON) with the classes of `ids` (every class when none) switched on, off or the other way; what
/// it does not know of the renderer stays as it is. Whether anything changed; nothing is written on a refusal.
pub fn set_enabled(renderer: &mut Value, ids: Option<&[String]>, switch: Switch) -> Result<bool, SwitchRefused> {
    let listed = style_classes(renderer);
    if listed.is_empty() {
        return Err(SwitchRefused::NoClasses);
    }
    let chosen: Vec<(String, bool)> = match ids {
        None => listed.iter().map(|c| (c.id.clone(), c.enabled)).collect(),
        Some(ids) => {
            let mut out = Vec::new();
            for (i, id) in ids.iter().enumerate() {
                let Some(c) = listed.iter().find(|c| c.id == *id) else {
                    return Err(SwitchRefused::UnknownClass(i));
                };
                out.push((c.id.clone(), c.enabled));
            }
            out
        }
    };
    let mut changed = false;
    for (id, enabled) in chosen {
        let want = match switch {
            Switch::On => true,
            Switch::Off => false,
            Switch::Invert => !enabled,
        };
        if want == enabled {
            continue;
        }
        let Some((holder, key)) = flag_of(renderer, &id) else {
            continue;
        };
        if want {
            holder.remove(key);
        } else {
            holder.insert(key.to_owned(), Value::Bool(false));
        }
        changed = true;
    }
    Ok(changed)
}

fn index(id: &str, letter: char) -> Option<usize> {
    id.strip_prefix(letter)?.parse().ok()
}

/// The object holding a class's flag and the flag's key.
fn flag_of<'a>(renderer: &'a mut Value, id: &str) -> Option<(&'a mut Map<String, Value>, &'static str)> {
    if id == OTHER {
        return Some((renderer.as_object_mut()?, "otherEnabled"));
    }
    if let Some(i) = index(id, 'c') {
        let k = renderer.get_mut("categories")?.get_mut(i)?;
        return Some((k.as_object_mut()?, "enabled"));
    }
    if let Some(i) = index(id, 'g') {
        let k = renderer.get_mut("classes")?.get_mut(i)?;
        return Some((k.as_object_mut()?, "enabled"));
    }
    let path: Vec<usize> = id.strip_prefix('r')?.split('.').map(|p| p.parse().ok()).collect::<Option<_>>()?;
    let (first, rest) = path.split_first()?;
    let mut rule = renderer.get_mut("rules")?.get_mut(*first)?;
    for i in rest {
        rule = rule.get_mut("children")?.get_mut(*i)?;
    }
    Some((rule.as_object_mut()?, "enabled"))
}
