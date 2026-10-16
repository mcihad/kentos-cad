//! A dynamic block's variant (docs/adr/0318 §2): its definition's objects as an insert's parameter values draw them.
//! Each action moves its objects (local ids) by its parameter's change from the default, in the definition's order,
//! each on what the ones before left; then the visibility state's hidden objects are left out. The rules of the part
//! are the contract's; here a part that does not read, or an action on a parameter it does not fit, does nothing.

use crate::api::json::{FromJson, Json};
use crate::entity::{Entity, ParamValue, Params};
use crate::geom::affine::{Affine, compose, translation};
use crate::geometry::Bounds;
use crate::jsmath::{atan2, cos, js_hypot, js_max, sin};
use crate::ops::stretch::stretch_entity;
use crate::ops::transform::transform_shape;
use crate::vec2::Vec2;

/// A parameter, as the variant needs it.
#[derive(Clone, Debug, PartialEq)]
pub enum Parameter {
    Linear {
        name: String,
        from: Vec2,
        to: Vec2,
    },
    Polar {
        name: String,
        from: Vec2,
        to: Vec2,
    },
    Angle {
        name: String,
        center: Vec2,
        base: f64,
    },
    Point {
        name: String,
        at: Vec2,
    },
    Flip {
        name: String,
        a: Vec2,
        b: Vec2,
    },
    /// Each state's name and the local ids it hides.
    Visibility {
        name: String,
        states: Vec<(String, Vec<u32>)>,
    },
    /// A lookup sets other parameters; the values kept are theirs.
    Lookup {
        name: String,
    },
}

impl Parameter {
    pub fn name(&self) -> &str {
        match self {
            Parameter::Linear { name, .. }
            | Parameter::Polar { name, .. }
            | Parameter::Angle { name, .. }
            | Parameter::Point { name, .. }
            | Parameter::Flip { name, .. }
            | Parameter::Visibility { name, .. }
            | Parameter::Lookup { name } => name,
        }
    }

    /// Its default value; a lookup has none.
    pub fn default_value(&self) -> Option<ParamValue> {
        Some(match self {
            Parameter::Linear { from, to, .. } => ParamValue::Number(dist(*from, *to)),
            Parameter::Polar { to, .. } => ParamValue::Point(*to),
            Parameter::Angle { base, .. } => ParamValue::Number(*base),
            Parameter::Point { at, .. } => ParamValue::Point(*at),
            Parameter::Flip { .. } => ParamValue::Flag(false),
            Parameter::Visibility { states, .. } => {
                ParamValue::Text(states.first().map(|s| s.0.clone()).unwrap_or_default())
            }
            Parameter::Lookup { .. } => return None,
        })
    }
}

/// What an action does to its objects.
#[derive(Clone, Debug, PartialEq)]
pub enum Act {
    Move,
    Stretch { min: Vec2, max: Vec2 },
    Scale,
    Array { spacing: f64 },
    Rotate,
    Flip,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Action {
    pub act: Act,
    pub parameter: String,
    pub pieces: Vec<u32>,
}

/// A definition's dynamic part.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Dynamic {
    pub parameters: Vec<Parameter>,
    pub actions: Vec<Action>,
}

fn dist(a: Vec2, b: Vec2) -> f64 {
    js_hypot(b.x - a.x, b.y - a.y)
}

fn text(v: &Json, key: &str) -> Option<String> {
    match v.get(key) {
        Json::Str(s) => Some(s.clone()),
        _ => None,
    }
}

fn point(v: &Json, key: &str) -> Option<Vec2> {
    Vec2::from_json(v.get(key)).ok()
}

fn number(v: &Json, key: &str) -> Option<f64> {
    match v.get(key) {
        Json::Num(x) => Some(*x),
        _ => None,
    }
}

fn ids(v: &Json) -> Vec<u32> {
    match v {
        Json::Arr(list) => list
            .iter()
            .filter_map(|x| match x {
                Json::Num(n) if *n >= 0.0 && *n <= f64::from(u32::MAX) => Some(*n as u32),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

impl Dynamic {
    /// The part read from the contract's JSON (`{ parameters, actions? }`); what does not read is left out.
    pub fn from_json(v: &Json) -> Dynamic {
        let mut d = Dynamic::default();
        if let Json::Arr(list) = v.get("parameters") {
            for p in list {
                let Some(name) = text(p, "name") else {
                    continue;
                };
                let read = match text(p, "kind").as_deref() {
                    Some("linear") => point(p, "from")
                        .zip(point(p, "to"))
                        .map(|(from, to)| Parameter::Linear { name, from, to }),
                    Some("polar") => point(p, "from")
                        .zip(point(p, "to"))
                        .map(|(from, to)| Parameter::Polar { name, from, to }),
                    Some("angle") => point(p, "center")
                        .zip(number(p, "base"))
                        .map(|(center, base)| Parameter::Angle { name, center, base }),
                    Some("point") => point(p, "at").map(|at| Parameter::Point { name, at }),
                    Some("flip") => point(p, "a")
                        .zip(point(p, "b"))
                        .map(|(a, b)| Parameter::Flip { name, a, b }),
                    Some("visibility") => match p.get("states") {
                        Json::Arr(states) => Some(Parameter::Visibility {
                            name,
                            states: states
                                .iter()
                                .filter_map(|s| Some((text(s, "name")?, ids(s.get("hidden")))))
                                .collect(),
                        }),
                        _ => None,
                    },
                    Some("lookup") => Some(Parameter::Lookup { name }),
                    _ => None,
                };
                d.parameters.extend(read);
            }
        }
        if let Json::Arr(list) = v.get("actions") {
            for a in list {
                let Some(parameter) = text(a, "parameter") else {
                    continue;
                };
                let act = match text(a, "kind").as_deref() {
                    Some("move") => Act::Move,
                    Some("scale") => Act::Scale,
                    Some("rotate") => Act::Rotate,
                    Some("flip") => Act::Flip,
                    Some("stretch") => match point(a, "min").zip(point(a, "max")) {
                        Some((min, max)) => Act::Stretch { min, max },
                        None => continue,
                    },
                    Some("array") => match number(a, "spacing") {
                        Some(spacing) => Act::Array { spacing },
                        None => continue,
                    },
                    _ => continue,
                };
                d.actions.push(Action {
                    act,
                    parameter,
                    pieces: ids(a.get("pieces")),
                });
            }
        }
        d
    }

    fn parameter(&self, name: &str) -> Option<&Parameter> {
        self.parameters.iter().find(|p| p.name() == name)
    }

    /// A parameter's value: the insert's, else its default.
    fn value(&self, p: &Parameter, values: Option<&Params>) -> Option<ParamValue> {
        values
            .and_then(|v| v.of(p.name()).cloned())
            .or_else(|| p.default_value())
    }

    /// Whether the insert's values draw the definition as it is (every value its default, nothing hidden).
    pub fn is_default(&self, values: Option<&Params>) -> bool {
        values.is_none_or(|v| {
            v.0.iter().all(|(name, value)| {
                self.parameter(name)
                    .and_then(Parameter::default_value)
                    .is_none_or(|d| same(&d, value))
            })
        })
    }

    /// The definition's objects as the values draw them (docs/adr/0318 §2).
    pub fn vary(&self, entities: &[Entity], values: Option<&Params>) -> Vec<Entity> {
        let mut out: Vec<Entity> = entities.to_vec();
        let mut next_id = out
            .iter()
            .filter_map(local_id)
            .fold(0u32, u32::max)
            .saturating_add(1);
        for action in &self.actions {
            let Some(p) = self.parameter(&action.parameter) else {
                continue;
            };
            let Some(value) = self.value(p, values) else {
                continue;
            };
            let chosen = |e: &Entity| local_id(e).is_some_and(|id| action.pieces.contains(&id));
            match (&action.act, p) {
                (Act::Move | Act::Stretch { .. }, _) => {
                    let Some(d) = change(p, &value) else { continue };
                    if d.x == 0.0 && d.y == 0.0 {
                        continue;
                    }
                    for e in out.iter_mut().filter(|e| chosen(e)) {
                        match &action.act {
                            Act::Stretch { min, max } => {
                                let window = Bounds {
                                    min_x: min.x,
                                    min_y: min.y,
                                    max_x: max.x,
                                    max_y: max.y,
                                };
                                if let Some(s) = stretch_entity(e, &window, d.x, d.y) {
                                    e.shape = s.shape;
                                }
                            }
                            _ => e.shape = transform_shape(&e.shape, &translation(d.x, d.y)),
                        }
                    }
                }
                (
                    Act::Scale,
                    Parameter::Linear { from, to, .. } | Parameter::Polar { from, to, .. },
                ) => {
                    let base = dist(*from, *to);
                    let now = match &value {
                        ParamValue::Number(x) => *x,
                        ParamValue::Point(q) => dist(*from, *q),
                        _ => continue,
                    };
                    let k = now / base;
                    if !(k.is_finite() && k > 0.0) || k == 1.0 {
                        continue;
                    }
                    let m: Affine = [k, 0.0, 0.0, k, from.x * (1.0 - k), from.y * (1.0 - k)];
                    for e in out.iter_mut().filter(|e| chosen(e)) {
                        e.shape = transform_shape(&e.shape, &m);
                    }
                }
                (Act::Array { spacing }, Parameter::Linear { from, to, .. }) => {
                    let ParamValue::Number(now) = value else {
                        continue;
                    };
                    if !(*spacing > 0.0) {
                        continue;
                    }
                    // The definition holds as many as its default length does; each further `spacing` of length
                    // adds one copy, the next past the last (none when shorter).
                    let len = dist(*from, *to);
                    let held = |x: f64| ((x / spacing) + 1e-9).floor();
                    let n = (held(now) - held(len) + 1.0).clamp(1.0, 1000.0) as u32;
                    let (ux, uy) = ((to.x - from.x) / len, (to.y - from.y) / len);
                    let originals: Vec<Entity> =
                        out.iter().filter(|e| chosen(e)).cloned().collect();
                    for k in 1..n {
                        let shift =
                            translation(ux * spacing * f64::from(k), uy * spacing * f64::from(k));
                        for e in &originals {
                            let mut copy = e.clone();
                            copy.shape = transform_shape(&e.shape, &shift);
                            set_local_id(&mut copy, next_id);
                            next_id = next_id.saturating_add(1);
                            out.push(copy);
                        }
                    }
                }
                (Act::Rotate, Parameter::Angle { center, base, .. }) => {
                    let ParamValue::Number(now) = value else {
                        continue;
                    };
                    let turn = now - base;
                    if turn == 0.0 || !turn.is_finite() {
                        continue;
                    }
                    let (s, c) = (sin(turn), cos(turn));
                    let m = compose(
                        &translation(center.x, center.y),
                        &compose(&[c, s, -s, c, 0.0, 0.0], &translation(-center.x, -center.y)),
                    );
                    for e in out.iter_mut().filter(|e| chosen(e)) {
                        e.shape = transform_shape(&e.shape, &m);
                    }
                }
                (Act::Flip, Parameter::Flip { a, b, .. }) => {
                    if value != ParamValue::Flag(true) {
                        continue;
                    }
                    let m = reflection(*a, *b);
                    for e in out.iter_mut().filter(|e| chosen(e)) {
                        e.shape = transform_shape(&e.shape, &m);
                    }
                }
                _ => {}
            }
        }
        // The visibility state's hidden objects (one visibility parameter at most).
        for p in &self.parameters {
            if let Parameter::Visibility { states, .. } = p
                && let Some(ParamValue::Text(state)) = self.value(p, values)
                && let Some((_, hidden)) = states.iter().find(|(n, _)| *n == state)
            {
                out.retain(|e| local_id(e).is_none_or(|id| !hidden.contains(&id)));
            }
        }
        out
    }
}

/// How far a parameter's point moved from its default: a linear's along its line, a polar's and a point's by the
/// moved point; none for another kind.
fn change(p: &Parameter, value: &ParamValue) -> Option<Vec2> {
    match (p, value) {
        (Parameter::Linear { from, to, .. }, ParamValue::Number(now)) => {
            let len = dist(*from, *to);
            if len == 0.0 {
                return None;
            }
            let k = (now - len) / len;
            Some(Vec2::new((to.x - from.x) * k, (to.y - from.y) * k))
        }
        (Parameter::Polar { to, .. }, ParamValue::Point(q)) => {
            Some(Vec2::new(q.x - to.x, q.y - to.y))
        }
        (Parameter::Point { at, .. }, ParamValue::Point(q)) => {
            Some(Vec2::new(q.x - at.x, q.y - at.y))
        }
        _ => None,
    }
}

/// The mirror about the line through `a` and `b`.
fn reflection(a: Vec2, b: Vec2) -> Affine {
    let t = 2.0 * atan2(b.y - a.y, b.x - a.x);
    let (c, s) = (cos(t), sin(t));
    compose(
        &translation(a.x, a.y),
        &compose(&[c, s, s, -c, 0.0, 0.0], &translation(-a.x, -a.y)),
    )
}

/// Two values the same: numbers and points within a hair.
pub fn same(a: &ParamValue, b: &ParamValue) -> bool {
    let near = |x: f64, y: f64| (x - y).abs() <= 1e-9 * js_max(x.abs(), 1.0);
    match (a, b) {
        (ParamValue::Number(x), ParamValue::Number(y)) => near(*x, *y),
        (ParamValue::Point(p), ParamValue::Point(q)) => near(p.x, q.x) && near(p.y, q.y),
        _ => a == b,
    }
}

fn local_id(e: &Entity) -> Option<u32> {
    e.rest
        .iter()
        .find(|(k, _)| k == "id")
        .and_then(|(_, v)| match v {
            Json::Num(n) if *n >= 0.0 => Some(*n as u32),
            _ => None,
        })
}

fn set_local_id(e: &mut Entity, id: u32) {
    match e.rest.iter_mut().find(|(k, _)| k == "id") {
        Some((_, v)) => *v = Json::Num(f64::from(id)),
        None => e.rest.push(("id".into(), Json::Num(f64::from(id)))),
    }
}
