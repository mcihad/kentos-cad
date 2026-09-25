//! Which symbols an object gets from its layer's renderer (formerly the
//! TypeScript `style/resolve.ts`). A rule-based renderer can match
//! several rules (each draws, as in QGIS); every match carries the scale
//! range it is drawn in, so the GPU switches it per frame without a rebuild.

use kentos_geometry_core::jsmath::{js_max, js_min};

use super::compile::Values;
use super::model::{Renderer, Rule, SymbolSet};

/// A 1:N scale range (absent ends: unbounded).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Scale {
    pub min: Option<f64>,
    pub max: Option<f64>,
}

/// Symbols an object draws, in the range they are drawn in.
pub struct Resolved<'a> {
    pub symbols: &'a SymbolSet,
    pub scale: Scale,
}

fn narrow(a: Scale, r: &Rule) -> Scale {
    Scale {
        min: match r.min_scale {
            Some(m) => Some(js_max(a.min.unwrap_or(0.0), m)),
            None => a.min,
        },
        max: match r.max_scale {
            Some(m) => Some(js_min(a.max.unwrap_or(f64::INFINITY), m)),
            None => a.max,
        },
    }
}

fn condition(t: &dyn Values, filter: Option<Option<usize>>) -> bool {
    match filter {
        None => true,
        Some(None) => false,
        Some(Some(e)) => t.truth(e) == Some(true),
    }
}

fn match_rules<'a>(rules: &'a [Rule], t: &dyn Values, range: Scale, out: &mut Vec<Resolved<'a>>) {
    let visit = |r: &'a Rule, out: &mut Vec<Resolved<'a>>| {
        let scale = narrow(range, r);
        if let (Some(min), Some(max)) = (scale.min, scale.max)
            && min > max
        {
            return;
        }
        if let Some(symbols) = &r.symbols {
            out.push(Resolved { symbols, scale });
        }
        if !r.children.is_empty() {
            match_rules(&r.children, t, scale, out);
        }
    };
    let mut any = false;
    for r in rules {
        if !r.enabled || r.is_else || !condition(t, r.filter) {
            continue;
        }
        any = true;
        visit(r, out);
    }
    if !any {
        for r in rules.iter().filter(|r| r.enabled && r.is_else) {
            visit(r, out);
        }
    }
}

pub fn resolve_renderer<'a>(renderer: &'a Renderer, t: &dyn Values) -> Vec<Resolved<'a>> {
    let one = |symbols: &'a SymbolSet| {
        vec![Resolved {
            symbols,
            scale: Scale::default(),
        }]
    };
    match renderer {
        Renderer::Single(symbols) => one(symbols),
        Renderer::Categorized {
            expr,
            categories,
            other,
        } => {
            let v = expr.and_then(|e| t.text(e)).unwrap_or_default();
            match categories.iter().find(|c| c.enabled && c.value == v) {
                Some(c) => one(&c.symbols),
                None => other.as_ref().map_or_else(Vec::new, one),
            }
        }
        Renderer::Graduated { expr, classes } => {
            let Some(n) = expr.and_then(|e| t.number(e)) else {
                return Vec::new();
            };
            let last = classes.len().wrapping_sub(1);
            classes
                .iter()
                .enumerate()
                .find(|(i, k)| n >= k.min && (n < k.max || (*i == last && n <= k.max)))
                .map_or_else(Vec::new, |(_, k)| one(&k.symbols))
        }
        Renderer::Rules(rules) => {
            let mut out = Vec::new();
            match_rules(rules, t, Scale::default(), &mut out);
            out
        }
    }
}
