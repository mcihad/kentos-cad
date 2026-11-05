//! A project's line types as the style engine draws them (docs/adr/0313 §3): each pattern's dashes in paper
//! millimetres, as the simple look's built-in types are, so a line's width keeps its unit. A paper pattern's lengths
//! are millimetres already; a drawing pattern's (the project's unit) go to paper by the drawing's scale. Every dash,
//! a built-in type's too, is times the project's line type scale (LTSCALE); the object's own scale is its
//! [`crate::simple::Stroke`]'s. All dash/gap values are retained. Bundled-font text marks share frozen
//! contours; unsupported resources carry explicit loss findings (ADR 0930).

mod text;

use std::collections::HashMap;

use kentos_contracts::{LineTypeDef, PatternUnit, ProjectSettings, ProjectStyles};
use serde_json::{Value, json};

/// A pattern as the engine draws it: its dashes and gaps, millimetres on paper, a dash first.
#[derive(Clone, Debug, PartialEq)]
pub struct Dashes {
    pub dash: Vec<f64>,
    /// Whether it loses a text or shape it cannot yet draw.
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
    Some(Dashes { dash: out, lossy })
}

/// A project's line types, ready to draw.
#[derive(Clone, Debug, PartialEq)]
pub struct LineTypes {
    by_name: HashMap<String, Option<Dashes>>,
    /// Kept separately: a continuous pattern can still lose its complex marks.
    losses: HashMap<String, String>,
    marks: HashMap<String, Value>,
    /// The project's line type scale (LTSCALE), 1 without one.
    pub scale: f64,
}

impl Default for LineTypes {
    /// No type, LTSCALE 1: the built-in types as they are.
    fn default() -> Self {
        Self {
            by_name: HashMap::new(),
            losses: HashMap::new(),
            marks: HashMap::new(),
            scale: 1.0,
        }
    }
}

impl LineTypes {
    /// The settings' line types at the drawing's scale and in its unit.
    pub fn of(settings: &ProjectSettings) -> Self {
        Self::at_scale(settings, settings.plot_scale)
    }

    /// Resolve drawing-unit lengths against the scale where symbols will
    /// actually be compiled. Paper recipes stay in paper mm; drawing recipes
    /// must come back to the same metres at every viewport denominator.
    pub fn at_scale(settings: &ProjectSettings, plot_scale: f64) -> Self {
        Self::at_scale_with_resources(settings, &ProjectStyles::default(), plot_scale)
    }

    pub fn with_resources(settings: &ProjectSettings, styles: &ProjectStyles) -> Self {
        Self::at_scale_with_resources(settings, styles, settings.plot_scale)
    }

    pub fn at_scale_with_resources(
        settings: &ProjectSettings,
        styles: &ProjectStyles,
        plot_scale: f64,
    ) -> Self {
        let plot_scale = plot_scale.max(1e-6);
        let unit = settings
            .drawing_unit
            .unwrap_or(kentos_contracts::DrawingUnit::M)
            .metres();
        let mut losses = HashMap::new();
        let mut marks = HashMap::new();
        for def in &settings.line_types {
            match text::prepare(def, settings, plot_scale, styles) {
                Ok(Some(prepared)) => { marks.insert(def.name.to_lowercase(), prepared); }
                Ok(None) => {},
                Err(error) => { losses.insert(def.name.to_lowercase(), format!("“{}”: {error}", def.name)); }
            }
        }
        Self {
            losses,
            marks,
            by_name: settings
                .line_types
                .iter()
                .map(|t| {
                    (
                        t.name.to_lowercase(),
                        dashes_of(t, plot_scale, unit),
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

    pub fn needs_preflight(&self) -> bool {
        !self.losses.is_empty() || !self.marks.is_empty()
    }

    /// A named type's dashes, `Some(None)` for a continuous one; none for a name the project does not have.
    pub fn dashes(&self, name: &str) -> Option<Option<&Dashes>> {
        self.by_name.get(&name.to_lowercase()).map(Option::as_ref)
    }

    /// A source problem, before simplifying its appearance. Only carried by
    /// actual simple-line primitives: unused definitions are not output errors.
    pub fn loss(&self, name: &str) -> Option<String> {
        let key = name.to_lowercase();
        self.losses.get(&key).cloned().or_else(|| {
            (!self.by_name.contains_key(&key))
                .then(|| format!("“{name}” çizgi tipinin kaynak tanımı projede yok."))
        })
    }

    /// `symbols` with their simple lines drawn in `pattern` (a known type's dashes, LTSCALE times) or, without one,
    /// their built-in dashes LTSCALE times. A name the project does not have keeps the built-in type.
    pub fn apply(&self, pattern: Option<&str>, mut symbols: Value) -> Value {
        let dash = pattern.and_then(|p| self.dashes(p));
        let loss = pattern.and_then(|p| self.loss(p));
        let marks = pattern.and_then(|p| self.marks.get(&p.to_lowercase()));
        if dash.is_none() && self.scale == 1.0 && loss.is_none() && marks.is_none() {
            return symbols;
        }
        fn walk(v: &mut Value, dash: Option<Option<&Dashes>>, scale: f64, loss: Option<&str>, marks: Option<&Value>) {
            match v {
                Value::Object(map) => {
                    if map.get("type").and_then(Value::as_str) == Some("simpleLine") {
                        if let Some(loss) = loss {
                            map.insert("lineTypeLoss".into(), json!(loss));
                        }
                        if let Some(marks) = marks {
                            let mut marks = marks.clone();
                            scale_marks(&mut marks, scale);
                            map.insert("cadMarks".into(), marks);
                        }
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
                        walk(x, dash, scale, loss, marks);
                    }
                }
                Value::Array(list) => list.iter_mut().for_each(|x| walk(x, dash, scale, loss, marks)),
                _ => {}
            }
        }
        walk(&mut symbols, dash, self.scale, loss.as_deref(), marks);
        symbols
    }
}

/// Scale physical placement once; normalized glyph contours remain unchanged.
pub(crate) fn scale_marks(v: &mut Value, scale: f64) {
    if scale == 1. { return; }
    fn times(v: &mut Value, name: &str, scale: f64) {
        if let Some(n) = v.get(name).and_then(Value::as_f64) { v[name] = json!(n * scale); }
    }
    times(v, "period", scale);
    if let Some(marks) = v.get_mut("marks").and_then(Value::as_array_mut) {
        for mark in marks { for key in ["at", "height", "x", "y"] { times(mark, key, scale); } }
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
        assert_eq!((long.dash.len(), long.lossy), (10, false));
    }
}
