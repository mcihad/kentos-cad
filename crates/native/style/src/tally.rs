//! What the layer style window counts as the user edits: which objects each
//! category, class or rule takes, as the style core draws them
//! (`kentos_style_core::style::resolve`), so the Nesne column says what the
//! map will show. The web's window counts values one by one
//! (`categoryCounts`, `countIn`, a rule's own condition over the whole
//! layer); the desktop follows the drawing: an object goes to the first
//! category of its value, a child rule counts among its parent's objects,
//! a “değilse” rule what no sibling took.

use std::collections::HashMap;

use kentos_contracts::Entity;
use kentos_style_core::expr::rows::As;
use kentos_style_core::expr::{Value, compile};

use crate::classify::{ExprScope, evaluate};
use crate::renderer::{Category, Rule};

/// How many objects each category takes, and how many none does
/// (“Diğer değerler”). An object without a value has the value "", as the core reads it.
pub fn category_tally(values: &[Option<String>], categories: &[Category]) -> (Vec<usize>, usize) {
    let mut first: HashMap<&str, usize> = HashMap::new();
    for (i, k) in categories.iter().enumerate() {
        first.entry(k.value.as_str()).or_insert(i);
    }
    let mut counts = vec![0; categories.len()];
    let mut rest = 0;
    for v in values {
        match first.get(v.as_deref().unwrap_or("")) {
            Some(&i) => counts[i] += 1,
            None => rest += 1,
        }
    }
    (counts, rest)
}

/// Whether an earlier category has the same value: this one never draws.
pub fn shadowed(categories: &[Category], i: usize) -> bool {
    categories
        .get(i)
        .is_some_and(|k| categories[..i].iter().any(|o| o.value == k.value))
}

/// What a rule takes, or why its condition cannot be read.
#[derive(Clone, Debug, PartialEq)]
pub enum RuleCount {
    Count(usize),
    /// The condition's message, “12. karakterde: …” when it is not about the start.
    Error(String),
}

/// Which objects a condition takes, or why it cannot be read.
type Mask = Result<Vec<bool>, String>;

/// Each rule's count by its path in the tree (indices from the top). A rule
/// takes its parent's objects that meet its condition (an empty one: all);
/// a “değilse” rule takes its parent's objects no enabled sibling took.
/// Scale ranges and the rule's own switch do not change what it counts.
pub fn rule_counts(
    rules: &[Rule],
    list: &[&Entity],
    scope: &ExprScope,
) -> HashMap<Vec<usize>, RuleCount> {
    let mut masks: HashMap<String, Mask> = HashMap::new();
    let mut mask_of = |filter: Option<&str>| -> Mask {
        let Some(f) = filter.map(str::trim).filter(|f| !f.is_empty()) else {
            return Ok(vec![true; list.len()]);
        };
        masks
            .entry(f.to_owned())
            .or_insert_with(|| match compile(f) {
                Err(e) => Err(e.text()),
                Ok(c) => Ok(evaluate(&c, list, scope, As::Bool)
                    .iter()
                    .map(|v| *v == Value::Bool(true))
                    .collect()),
            })
            .clone()
    };
    let mut out = HashMap::new();
    let everything = vec![true; list.len()];
    walk(rules, &everything, &mut Vec::new(), &mut mask_of, &mut out);
    out
}

fn walk(
    rules: &[Rule],
    parent: &[bool],
    path: &mut Vec<usize>,
    mask_of: &mut dyn FnMut(Option<&str>) -> Mask,
    out: &mut HashMap<Vec<usize>, RuleCount>,
) {
    let own: Vec<Option<Mask>> = rules
        .iter()
        .map(|r| (!r.is_else()).then(|| mask_of(r.filter.as_deref())))
        .collect();
    // What the enabled plain siblings take: the rest is the “değilse” rules'.
    let taken: Vec<bool> = (0..parent.len())
        .map(|i| {
            rules
                .iter()
                .zip(&own)
                .any(|(r, m)| r.enabled() && matches!(m, Some(Ok(m)) if m[i]))
        })
        .collect();
    for (k, (rule, own)) in rules.iter().zip(own).enumerate() {
        path.push(k);
        let mask: Vec<bool> = match own {
            Some(Ok(m)) => parent.iter().zip(m).map(|(a, b)| *a && b).collect(),
            Some(Err(e)) => {
                out.insert(path.clone(), RuleCount::Error(e));
                vec![false; parent.len()]
            }
            None => parent.iter().zip(&taken).map(|(a, t)| *a && !t).collect(),
        };
        out.entry(path.clone())
            .or_insert_with(|| RuleCount::Count(mask.iter().filter(|m| **m).count()));
        walk(rule.children(), &mask, path, mask_of, out);
        path.pop();
    }
}

/// How a field is written in an expression: bare when it can be, in
/// brackets otherwise (the web's `fieldToken`); `"…"` would be a text.
pub fn field_token(name: &str) -> String {
    const RESERVED: [&str; 12] = [
        "VE", "VEYA", "DEGIL", "AND", "OR", "NOT", "DOGRU", "YANLIS", "TRUE", "FALSE", "BOS",
        "NULL",
    ];
    let mut chars = name.chars();
    let plain = chars.next().is_some_and(|c| c.is_alphabetic() || c == '_')
        && chars.all(|c| c.is_alphanumeric() || c == '_');
    let folded = kentos_style_core::js::text::fold_turkish(name);
    if plain && !RESERVED.contains(&folded.as_str()) {
        name.to_owned()
    } else {
        format!("[{name}]")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn entities() -> Vec<Entity> {
        (1..=6)
            .map(|i| {
                serde_json::from_value(json!({
                    "kind": "point", "id": i, "layerId": "k", "p": { "x": i, "y": 0 },
                    "attrs": { "Kat": i.to_string(), "Nitelik": if i % 2 == 0 { "Arsa" } else { "Tarla" } },
                }))
                .unwrap()
            })
            .collect()
    }

    fn rule(v: serde_json::Value) -> Rule {
        serde_json::from_value(v).unwrap()
    }

    #[test]
    fn rules_count_what_they_draw() {
        let list = entities();
        let list: Vec<&Entity> = list.iter().collect();
        let name = |_: &str| "Kadastro".to_owned();
        let measures = |_: &[&Entity]| Vec::new();
        let scope = ExprScope {
            layer_name: &name,
            measures: &measures,
        };
        let rules = vec![
            rule(
                json!({ "id": "a", "label": "Arsa", "filter": "Nitelik = 'Arsa'", "children": [
                { "id": "a1", "label": "Yüksek", "filter": "Kat > 3" },
                { "id": "a2", "label": "Diğer", "isElse": true },
            ] }),
            ),
            rule(json!({ "id": "b", "label": "Kapalı", "filter": "Kat = 1", "enabled": false })),
            rule(json!({ "id": "c", "label": "Diğerleri", "isElse": true })),
            rule(json!({ "id": "d", "label": "Bozuk", "filter": "Kat >" })),
        ];
        let counts = rule_counts(&rules, &list, &scope);
        let n = |p: &[usize]| counts.get(p).cloned();
        assert_eq!(n(&[0]), Some(RuleCount::Count(3)));
        // Among Arsa's 2, 4 and 6: 4 and 6 are above 3, 2 is left for “Diğer”.
        assert_eq!(n(&[0, 0]), Some(RuleCount::Count(2)));
        assert_eq!(n(&[0, 1]), Some(RuleCount::Count(1)));
        // Off, it still says what it would take; it does not keep the others from “Diğerleri”.
        assert_eq!(n(&[1]), Some(RuleCount::Count(1)));
        assert_eq!(n(&[2]), Some(RuleCount::Count(3)));
        assert!(
            matches!(n(&[3]), Some(RuleCount::Error(e)) if e.contains("karakterde") || !e.is_empty())
        );
    }

    #[test]
    fn a_value_goes_to_its_first_category() {
        let cat = |v: &str| Category {
            value: v.into(),
            label: v.into(),
            symbols: Default::default(),
            enabled: None,
            extra: Default::default(),
        };
        let cats = vec![cat("Arsa"), cat("Tarla"), cat("Arsa"), cat("")];
        let values = vec![
            Some("Arsa".to_owned()),
            Some("Arsa".to_owned()),
            None,
            Some("Bağ".to_owned()),
        ];
        assert_eq!(category_tally(&values, &cats), (vec![2, 0, 0, 1], 1));
        assert!(shadowed(&cats, 2) && !shadowed(&cats, 0));
    }

    #[test]
    fn fields_are_written_bare_or_in_brackets() {
        assert_eq!(field_token("Nitelik"), "Nitelik");
        assert_eq!(field_token("Tapu alanı"), "[Tapu alanı]");
        assert_eq!(field_token("değil"), "[değil]");
        assert_eq!(field_token("2kat"), "[2kat]");
    }
}
