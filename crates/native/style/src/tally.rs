//! What the layer style window counts as the user edits: which objects each
//! category, class or rule takes, as the style core draws them
//! (`kentos_style_core::style::resolve`), so the Nesne column says what the
//! map will show. Only objects the style engine draws count (texts and
//! dimensions are drawn elsewhere). An object goes to the first category of
//! its value and to the first class that takes its number; a child rule
//! counts among its parent's objects, a “değilse” rule what no sibling took.
//! Both platforms are held to `fixtures/style/v1/tally.json`.

use std::collections::HashMap;

use kentos_contracts::Entity;
use kentos_style_core::expr::rows::As;
use kentos_style_core::expr::{Value, compile};

use crate::classify::{ExprScope, evaluate, geometry_class};
use crate::renderer::{Category, GraduatedClass, Rule};

/// Whether the style engine draws each object (texts and dimensions it does not).
pub fn drawable(list: &[&Entity]) -> Vec<bool> {
    list.iter().map(|e| geometry_class(e).is_some()).collect()
}

/// How many drawn objects each category takes, and how many none does
/// (“Diğer değerler”). An object without a value has the value "", as the
/// core reads it; a switched-off category keeps its objects.
pub fn category_tally(
    values: &[Option<String>],
    drawn: &[bool],
    categories: &[Category],
) -> (Vec<usize>, usize) {
    let mut first: HashMap<&str, usize> = HashMap::new();
    for (i, k) in categories.iter().enumerate() {
        first.entry(k.value.as_str()).or_insert(i);
    }
    let mut counts = vec![0; categories.len()];
    let mut rest = 0;
    for (v, _) in values.iter().zip(drawn).filter(|(_, d)| **d) {
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

/// How many drawn objects each class takes (the first that takes the
/// number: its lower bound included, its upper bound too for the last), and
/// how many none does (no number, or outside every class): those are not drawn.
pub fn class_tally(
    numbers: &[Option<f64>],
    drawn: &[bool],
    classes: &[GraduatedClass],
) -> (Vec<usize>, usize) {
    let mut counts = vec![0; classes.len()];
    let mut rest = 0;
    let last = classes.len().wrapping_sub(1);
    for (n, _) in numbers.iter().zip(drawn).filter(|(_, d)| **d) {
        let taken = n.and_then(|n| {
            classes
                .iter()
                .enumerate()
                .position(|(i, k)| n >= k.min && (n < k.max || (i == last && n <= k.max)))
        });
        match taken {
            Some(i) => counts[i] += 1,
            None => rest += 1,
        }
    }
    (counts, rest)
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

/// Each rule's count by its path in the tree (indices from the top), of the
/// drawn objects. A rule takes its parent's objects that meet its condition
/// (none: all); a “değilse” rule takes its parent's objects that no enabled
/// sibling with a readable condition took. A rule that is off still says
/// what it would take, and does not keep its siblings' objects from “değilse”.
/// Scale ranges do not change what a rule counts.
pub fn rule_counts(
    rules: &[Rule],
    list: &[&Entity],
    scope: &ExprScope,
) -> HashMap<Vec<usize>, RuleCount> {
    let mut masks: HashMap<String, Mask> = HashMap::new();
    let mut mask_of = |filter: Option<&str>| -> Mask {
        let Some(f) = filter else {
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
    let drawn = drawable(list);
    walk(rules, &drawn, &mut Vec::new(), &mut mask_of, &mut out);
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
