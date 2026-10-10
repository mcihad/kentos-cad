//! Which symbols an object gets from its layer's renderer (formerly the
//! TypeScript `style/resolve.ts`). A rule-based renderer can match
//! several rules (each draws, as in QGIS); every match carries the scale
//! range it is drawn in, so the GPU switches it per frame without a rebuild.

use kentos_geometry_core::jsmath::{js_max, js_min};

use super::compile::Values;
use super::model::{Renderer, Rule, SymbolSet, Unit};
use super::thematic::{class_of, ramp_color, share, size_at, step};

/// A 1:N scale range (absent ends: unbounded).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Scale {
    pub min: Option<f64>,
    pub max: Option<f64>,
}

/// What a thematic renderer changes in the symbols (docs/adr/0213 §2.1–§2.3).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Patch {
    #[default]
    None,
    /// The main colour, `0xRRGGBB`.
    Color(u32),
    /// The size (a marker symbol's) or the width (a line symbol's) in `unit`, and its step of 256.
    Size { size: f64, unit: Unit, step: u8 },
}

/// Symbols an object draws, in the range they are drawn in.
pub struct Resolved<'a> {
    pub symbols: &'a SymbolSet,
    pub scale: Scale,
    pub patch: Patch,
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
            out.push(Resolved {
                symbols,
                scale,
                patch: Patch::None,
            });
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

fn rgb(c: [u8; 4]) -> u32 {
    (u32::from(c[0]) << 16) | (u32::from(c[1]) << 8) | u32::from(c[2])
}

pub fn resolve_renderer<'a>(renderer: &'a Renderer, t: &dyn Values) -> Vec<Resolved<'a>> {
    let one = |symbols: &'a SymbolSet| {
        vec![Resolved {
            symbols,
            scale: Scale::default(),
            patch: Patch::None,
        }]
    };
    let patched = |symbols: &'a SymbolSet, patch: Patch| {
        vec![Resolved {
            symbols,
            scale: Scale::default(),
            patch,
        }]
    };
    let other = |o: &'a Option<SymbolSet>| o.as_ref().map_or_else(Vec::new, one);
    match renderer {
        Renderer::Single(symbols) => one(symbols),
        Renderer::Categorized {
            expr,
            categories,
            other,
        } => {
            let v = expr.and_then(|e| t.text(e)).unwrap_or_default();
            // The first category of the value takes the object; switched off, it
            // draws nothing (as in QGIS). The others take only values no category holds.
            match categories.iter().find(|c| c.value == v) {
                Some(c) if c.enabled => one(&c.symbols),
                Some(_) => Vec::new(),
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
        // The main colour at the value's share, in 256 steps (docs/adr/0213 §2.1).
        Renderer::Unclassed {
            expr,
            min,
            max,
            ramp,
            symbols,
            other: o,
        } => match expr.and_then(|e| t.number(e)) {
            Some(v) => {
                let k = step(share(v, *min, *max));
                let c = ramp_color(ramp, f64::from(k) / 255.0);
                patched(symbols, Patch::Color(rgb(c)))
            }
            None => other(o),
        },
        // The size at the value's share, in 256 steps (§2.2).
        Renderer::Proportional {
            expr,
            min_value,
            max_value,
            min_size,
            max_size,
            unit,
            exponent,
            symbols,
            other: o,
        } => match expr.and_then(|e| t.number(e)) {
            Some(v) => {
                let k = step(share(v, *min_value, *max_value));
                let size = size_at(f64::from(k) / 255.0, *min_size, *max_size, *exponent);
                patched(
                    symbols,
                    Patch::Size {
                        size,
                        unit: *unit,
                        step: k,
                    },
                )
            }
            None => other(o),
        },
        // The grid's colour of the two classes (§2.3).
        Renderer::Bivariate {
            expr_x,
            expr_y,
            breaks_x,
            breaks_y,
            colors,
            symbols,
            other: o,
        } => match (
            expr_x.and_then(|e| t.number(e)),
            expr_y.and_then(|e| t.number(e)),
        ) {
            (Some(x), Some(y)) => {
                let n = breaks_x.len() + 1;
                let (i, j) = (class_of(x, breaks_x), class_of(y, breaks_y));
                match colors.get(j * n + i).copied().flatten() {
                    Some(c) => patched(symbols, Patch::Color(rgb(c))),
                    None => one(symbols),
                }
            }
            _ => other(o),
        },
        // The background under the dots or the chart.
        Renderer::DotDensity { symbols, .. } | Renderer::Chart { symbols, .. } => {
            symbols.as_ref().map_or_else(Vec::new, one)
        }
        // Single points and other objects: the inner renderer.
        Renderer::Cluster { inner, .. } | Renderer::Displacement { inner, .. } => inner
            .as_deref()
            .map_or_else(Vec::new, |r| resolve_renderer(r, t)),
        // Drawn for the whole layer at once.
        Renderer::Heatmap { .. } | Renderer::Inverted { .. } => Vec::new(),
    }
}
