//! A project's line types as the style engine draws them (docs/adr/0313 §3): each pattern's dashes in paper
//! millimetres, as the simple look's built-in types are, so a line's width keeps its unit. A paper pattern's lengths
//! are millimetres already; a drawing pattern's (the project's unit) go to paper by the drawing's scale. Every dash,
//! a built-in type's too, is times the project's line type scale (LTSCALE); the object's own scale is its
//! [`crate::simple::Stroke`]'s. A pattern's text and shape take no length and draw nothing (§4); a pattern of more
//! than eight dashes and gaps draws its first eight, as every renderer reads the same eight (§5).

use std::collections::HashMap;

use kentos_contracts::{LineTypeDef, PatternUnit, ProjectSettings};
use serde_json::{Value, json};

/// The most dashes and gaps a renderer draws (the styled pipeline's `dash0`, `dash1`).
pub const MAX_DASHES: usize = 8;

/// A pattern as the engine draws it: its dashes and gaps, millimetres on paper, a dash first.
#[derive(Clone, Debug, PartialEq)]
pub struct Dashes {
    pub dash: Vec<f64>,
    /// Whether it lost dashes past [`MAX_DASHES`], or a text or a shape it does not draw.
    pub lossy: bool,
}

/// The dashes of a type's elements, in its unit's millimetres on paper: alternate dashes and gaps, a dash first;
/// neighbours of one kind summed; a dot a dash of 0; a text or a shape nothing. None for a type without a gap
/// (continuous).
pub fn dashes_of(def: &LineTypeDef, plot_scale: f64, project_unit: (f64, f64)) -> Option<Dashes> {
    let to_paper = |l: f64| match def.unit {
        PatternUnit::Paper => l,
        PatternUnit::Drawing => l * project_unit.0 / project_unit.1 * 1000.0 / plot_scale,
    };
    let mut out: Vec<f64> = Vec::new();
    let mut lossy = false;
    for e in &def.elements {
        if e.is_complex() {
            lossy = true;
            continue;
        }
        let on = e.length >= 0.0;
        let l = to_paper(e.length.abs());
        // Even places are dashes, odd ones gaps: the last is a dash when the count is odd.
        if out.is_empty() {
            if !on {
                out.push(0.0);
            }
            out.push(l);
        } else if (out.len() % 2 == 1) == on {
            if let Some(last) = out.last_mut() {
                *last += l;
            }
        } else {
            out.push(l);
        }
    }
    if out.len() < 2 {
        return None;
    }
    // A pattern ending on a dash runs into its first one: a gap of 0 closes the period.
    if out.len() % 2 == 1 {
        out.push(0.0);
    }
    if out.len() > MAX_DASHES {
        out.truncate(MAX_DASHES);
        lossy = true;
    }
    Some(Dashes { dash: out, lossy })
}

/// A project's line types, ready to draw.
#[derive(Clone, Debug, PartialEq)]
pub struct LineTypes {
    by_name: HashMap<String, Option<Dashes>>,
    /// The project's line type scale (LTSCALE), 1 without one.
    pub scale: f64,
}

impl Default for LineTypes {
    /// No type, LTSCALE 1: the built-in types as they are.
    fn default() -> Self {
        Self {
            by_name: HashMap::new(),
            scale: 1.0,
        }
    }
}

impl LineTypes {
    /// The settings' line types at the drawing's scale and in its unit.
    pub fn of(settings: &ProjectSettings) -> Self {
        let unit = settings
            .drawing_unit
            .unwrap_or(kentos_contracts::DrawingUnit::M)
            .metres();
        Self {
            by_name: settings
                .line_types
                .iter()
                .map(|t| {
                    (
                        t.name.to_lowercase(),
                        dashes_of(t, settings.plot_scale, unit),
                    )
                })
                .collect(),
            scale: settings.line_type_scale.unwrap_or(1.0),
        }
    }

    /// Whether it changes nothing: no type and LTSCALE 1.
    pub fn is_plain(&self) -> bool {
        self.by_name.is_empty() && self.scale == 1.0
    }

    /// A named type's dashes, `Some(None)` for a continuous one; none for a name the project does not have.
    pub fn dashes(&self, name: &str) -> Option<Option<&Dashes>> {
        self.by_name.get(&name.to_lowercase()).map(Option::as_ref)
    }

    /// `symbols` with their simple lines drawn in `pattern` (a known type's dashes, LTSCALE times) or, without one,
    /// their built-in dashes LTSCALE times. A name the project does not have keeps the built-in type.
    pub fn apply(&self, pattern: Option<&str>, mut symbols: Value) -> Value {
        let dash = pattern.and_then(|p| self.dashes(p));
        if dash.is_none() && self.scale == 1.0 {
            return symbols;
        }
        fn walk(v: &mut Value, dash: Option<Option<&Dashes>>, scale: f64) {
            match v {
                Value::Object(map) => {
                    if map.get("type").and_then(Value::as_str) == Some("simpleLine") {
                        match dash {
                            Some(Some(d)) => {
                                map.insert(
                                    "dash".into(),
                                    json!(d.dash.iter().map(|x| x * scale).collect::<Vec<_>>()),
                                );
                            }
                            Some(None) => {
                                map.insert("dash".into(), Value::Null);
                            }
                            None => {
                                if let Some(Value::Array(list)) = map.get_mut("dash") {
                                    for x in list.iter_mut() {
                                        if let Some(f) = x.as_f64() {
                                            *x = json!(f * scale);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    for (_, x) in map.iter_mut() {
                        walk(x, dash, scale);
                    }
                }
                Value::Array(list) => list.iter_mut().for_each(|x| walk(x, dash, scale)),
                _ => {}
            }
        }
        walk(&mut symbols, dash, self.scale);
        symbols
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kentos_contracts::PatternElement;

    fn def(lengths: &[f64], unit: PatternUnit) -> LineTypeDef {
        LineTypeDef {
            name: "X".into(),
            description: String::new(),
            elements: lengths
                .iter()
                .map(|l| PatternElement {
                    length: *l,
                    text: None,
                    shape: None,
                })
                .collect(),
            unit,
            source: None,
        }
    }

    #[test]
    fn a_pattern_becomes_dashes_on_paper() {
        let d = |l: &[f64], u| dashes_of(&def(l, u), 50.0, (1.0, 1000.0)).map(|d| d.dash);
        assert_eq!(d(&[3.0, -1.5], PatternUnit::Paper), Some(vec![3.0, 1.5]));
        // A drawing pattern in a millimetre project at 1:50: 12 mm in the drawing is 0.24 mm on paper.
        let iso = d(&[12.0, -3.0], PatternUnit::Drawing).unwrap();
        assert!(
            (iso[0] - 0.24).abs() < 1e-12 && (iso[1] - 0.06).abs() < 1e-12,
            "{iso:?}"
        );
        // A dot is a dash of 0; neighbours of one kind are summed; a dash at the end closes with a gap of 0.
        assert_eq!(
            d(&[5.0, -1.0, -0.2, 0.0, -1.2, 1.0], PatternUnit::Paper),
            Some(vec![5.0, 1.2, 0.0, 1.2, 1.0, 0.0])
        );
        assert_eq!(d(&[1.0], PatternUnit::Paper), None, "continuous");
        let long = dashes_of(
            &def(
                &[1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0],
                PatternUnit::Paper,
            ),
            1.0,
            (1.0, 1.0),
        )
        .unwrap();
        assert_eq!((long.dash.len(), long.lossy), (MAX_DASHES, true));
    }
}
