//! Geometric and dimensional constraints (docs/adr/0334; AutoCAD's GEOMCONSTRAINT, DIMCONSTRAINT and PARAMETERS):
//! relations between the parts of a drawing's objects (a point on a point, a segment level, two segments parallel,
//! a circle on a segment …) and dimensions that drive them (a distance, an angle, a radius) from a table of named
//! parameters. They name the objects by their persistent ids and the parts by their places, as an associative
//! dimension does (`SubPlace`); the geometry core solves them (`ops::constraints`), the document keeps them
//! (`DocumentSnapshotV2.constraints`, `.kcad` schema 73) and every step that changes them is one undo step.
//!
//! These are a drawing's constraints, not the sheet layout's (`kentos-sheet`) nor the settings' bounds.

use std::collections::{BTreeMap, BTreeSet, HashSet};

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::constraint_expr::{
    param_expr_names, param_order, parameter_name_valid, parse_param_expr,
};
use crate::identity::{ConstraintId, EntityId};
use crate::subref::{SUBREF_INDEX_LIMIT, SubPlace};

/// The most constraints a drawing keeps.
pub const CONSTRAINTS_MAX: usize = 10_000;

/// The most parameters a drawing keeps.
pub const PARAMETERS_MAX: usize = 10_000;

/// A part of an object a constraint names.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CadElement {
    /// A point: a line's or an arc's start or end, a circle's or an arc's centre, a point object's node, a path's
    /// vertex. Not a place along a curve nor an edge's middle.
    Point { place: SubPlace },
    /// A straight segment: a line (all zero) or a path's straight edge from vertex `index` to the next (part 0 the
    /// object's own path, ring 0 its outer ring).
    Segment {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
        part: u32,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
        ring: u32,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
        index: u32,
    },
    /// A circle, or an arc's circle.
    Circle,
    /// A line of a wall's straight edge `edge` (docs/adr/0770 §4): a face, its axis or a face of its core, left and
    /// right by the path's direction. Only the architectural rules take it (distance, parallel, symmetric, fix).
    Wall {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
        edge: u32,
        line: WallLine,
    },
    /// An opening's start or end jamb, the middle of its jamb line (docs/adr/0770 §4, 0694).
    Jamb { end: JambEnd },
}

/// Which line of a wall's edge an architectural constraint names (docs/adr/0770 §4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum WallLine {
    LeftFace,
    RightFace,
    /// Halfway between the faces.
    Axis,
    /// The core's (the strongest component band's) left and right boundaries; a wall without components: its faces.
    LeftCore,
    RightCore,
}

impl WallLine {
    pub const ALL: [WallLine; 5] = [
        WallLine::LeftFace,
        WallLine::RightFace,
        WallLine::Axis,
        WallLine::LeftCore,
        WallLine::RightCore,
    ];

    /// Its Turkish name.
    pub fn label(self) -> &'static str {
        match self {
            WallLine::LeftFace => "sol yüz",
            WallLine::RightFace => "sağ yüz",
            WallLine::Axis => "eksen",
            WallLine::LeftCore => "çekirdeğin sol yüzü",
            WallLine::RightCore => "çekirdeğin sağ yüzü",
        }
    }
}

/// An opening's jamb: at its start or at its end along its wall (docs/adr/0770 §4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum JambEnd {
    Start,
    End,
}

impl CadElement {
    /// Whether it is an architectural part (a wall line or a jamb): docs/adr/0770 §4.
    pub fn is_architectural(&self) -> bool {
        matches!(self, CadElement::Wall { .. } | CadElement::Jamb { .. })
    }
}

/// A part of an object by the object's persistent id.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ElementRef {
    pub uid: EntityId,
    pub element: CadElement,
}

/// Which way a distance is measured: along the line between its points, or across or along the drawing's axes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum DistanceAxis {
    #[default]
    Aligned,
    /// Along east: `b`'s east minus `a`'s.
    Horizontal,
    /// Along north: `b`'s north minus `a`'s.
    Vertical,
}

fn aligned(axis: &DistanceAxis) -> bool {
    *axis == DistanceAxis::Aligned
}

/// What a constraint holds. The geometric kinds come first in the order AutoCAD's toolbar has them; the
/// dimensional ones name the parameter that gives (or, for a reference constraint, takes) their value.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ConstraintRule {
    /// A point on a point, or on a segment's line or a circle.
    Coincident {
        a: ElementRef,
        b: ElementRef,
    },
    /// A segment level, or two points level.
    Horizontal {
        a: ElementRef,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        b: Option<ElementRef>,
    },
    /// A segment upright, or two points one above the other.
    Vertical {
        a: ElementRef,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        b: Option<ElementRef>,
    },
    Parallel {
        a: ElementRef,
        b: ElementRef,
    },
    Perpendicular {
        a: ElementRef,
        b: ElementRef,
    },
    /// A segment's line or a circle touching a circle. With a segment, `left`: the circle's centre is on the left of
    /// the segment's direction; with two circles, `inside`: one inside the other. Set when the constraint is made,
    /// from the drawing: the solution keeps that side.
    Tangent {
        a: ElementRef,
        b: ElementRef,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
        inside: bool,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
        left: bool,
    },
    Concentric {
        a: ElementRef,
        b: ElementRef,
    },
    /// Two segments on one line.
    Collinear {
        a: ElementRef,
        b: ElementRef,
    },
    /// Two segments of one length, or two circles of one radius.
    Equal {
        a: ElementRef,
        b: ElementRef,
    },
    /// Two points mirror each other over a segment's line.
    Symmetric {
        a: ElementRef,
        b: ElementRef,
        axis: ElementRef,
    },
    /// A point, a segment's ends or a circle's centre and radius stay where they are.
    Fix {
        a: ElementRef,
    },
    /// The distance from a point to a point, or (aligned only) to a segment's line; with a segment, `left`: the
    /// point is on the left of the segment's direction (set when made). Horizontal and vertical distances are
    /// signed: `b` minus `a`.
    Distance {
        a: ElementRef,
        b: ElementRef,
        #[serde(default, skip_serializing_if = "aligned")]
        #[cfg_attr(feature = "ts", ts(as = "Option<DistanceAxis>", optional))]
        axis: DistanceAxis,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
        left: bool,
        parameter: String,
    },
    /// The angle from segment `a`'s direction to `b`'s, counter-clockwise; `reverseA`, `reverseB`: a segment taken
    /// from its end to its start (chosen when made, so the angle measured is the one the user picked).
    Angle {
        a: ElementRef,
        b: ElementRef,
        // A struct variant's fields keep their Rust names under the enum's `rename_all`: named here.
        #[serde(
            default,
            skip_serializing_if = "std::ops::Not::not",
            rename = "reverseA"
        )]
        #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional, rename = "reverseA"))]
        reverse_a: bool,
        // A struct variant's fields keep their Rust names under the enum's `rename_all`: named here.
        #[serde(
            default,
            skip_serializing_if = "std::ops::Not::not",
            rename = "reverseB"
        )]
        #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional, rename = "reverseB"))]
        reverse_b: bool,
        parameter: String,
    },
    Radius {
        a: ElementRef,
        parameter: String,
    },
    Diameter {
        a: ElementRef,
        parameter: String,
    },
}

impl ConstraintRule {
    /// The rule's stable name (its `kind`).
    pub fn name(&self) -> &'static str {
        match self {
            ConstraintRule::Coincident { .. } => "coincident",
            ConstraintRule::Horizontal { .. } => "horizontal",
            ConstraintRule::Vertical { .. } => "vertical",
            ConstraintRule::Parallel { .. } => "parallel",
            ConstraintRule::Perpendicular { .. } => "perpendicular",
            ConstraintRule::Tangent { .. } => "tangent",
            ConstraintRule::Concentric { .. } => "concentric",
            ConstraintRule::Collinear { .. } => "collinear",
            ConstraintRule::Equal { .. } => "equal",
            ConstraintRule::Symmetric { .. } => "symmetric",
            ConstraintRule::Fix { .. } => "fix",
            ConstraintRule::Distance { .. } => "distance",
            ConstraintRule::Angle { .. } => "angle",
            ConstraintRule::Radius { .. } => "radius",
            ConstraintRule::Diameter { .. } => "diameter",
        }
    }

    /// The parameter a dimensional rule reads or writes; none for a geometric one.
    pub fn parameter(&self) -> Option<&str> {
        match self {
            ConstraintRule::Distance { parameter, .. }
            | ConstraintRule::Angle { parameter, .. }
            | ConstraintRule::Radius { parameter, .. }
            | ConstraintRule::Diameter { parameter, .. } => Some(parameter),
            _ => None,
        }
    }

    /// The parts the rule names, in order.
    pub fn elements(&self) -> Vec<&ElementRef> {
        match self {
            ConstraintRule::Horizontal { a, b } | ConstraintRule::Vertical { a, b } => {
                std::iter::once(a).chain(b.as_ref()).collect()
            }
            ConstraintRule::Fix { a }
            | ConstraintRule::Radius { a, .. }
            | ConstraintRule::Diameter { a, .. } => {
                vec![a]
            }
            ConstraintRule::Symmetric { a, b, axis } => vec![a, b, axis],
            ConstraintRule::Coincident { a, b }
            | ConstraintRule::Parallel { a, b }
            | ConstraintRule::Perpendicular { a, b }
            | ConstraintRule::Tangent { a, b, .. }
            | ConstraintRule::Concentric { a, b }
            | ConstraintRule::Collinear { a, b }
            | ConstraintRule::Equal { a, b }
            | ConstraintRule::Distance { a, b, .. }
            | ConstraintRule::Angle { a, b, .. } => vec![a, b],
        }
    }
}

/// One constraint.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct Constraint {
    pub id: ConstraintId,
    pub rule: ConstraintRule,
    /// A dimensional constraint that only measures (AutoCAD's reference constraint): it never moves anything and its
    /// parameter, read-only, takes the measured value.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub reference: bool,
}

/// What a parameter's value is: a length in the drawing's unit, an angle in the project's angle unit, or a number.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ParameterKind {
    Length,
    Angle,
    Number,
}

/// A named parameter: a dimensional constraint's own, or the user's (AutoCAD's user parameter).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CadParameter {
    pub name: String,
    pub kind: ParameterKind,
    /// Its value as written ([`crate::constraint_expr`]); empty for a reference constraint's parameter, whose value
    /// is measured.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<String>", optional))]
    pub expression: String,
    /// The least and the most its value may be, in its own unit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "parameter")))]
    pub min: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "parameter")))]
    pub max: Option<f64>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<String>", optional))]
    pub description: String,
}

/// A drawing's constraints and parameters.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ConstraintSet {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<CadParameter>>", optional))]
    pub parameters: Vec<CadParameter>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<Constraint>>", optional))]
    pub constraints: Vec<Constraint>,
}

impl ConstraintSet {
    pub fn is_empty(&self) -> bool {
        self.parameters.is_empty() && self.constraints.is_empty()
    }

    pub fn parameter(&self, name: &str) -> Option<&CadParameter> {
        self.parameters.iter().find(|p| p.name == name)
    }

    pub fn constraint(&self, id: ConstraintId) -> Option<&Constraint> {
        self.constraints.iter().find(|c| c.id == id)
    }
}

/// How a solve ended (docs/adr/0334 §4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum SolveStatus {
    /// Every constraint holds and nothing can move without breaking one.
    Fully,
    /// Every constraint holds; `dof` freedoms are left.
    Under,
    /// Every constraint holds, but some repeat what others already say (`redundant`).
    Over,
    /// The constraints ask for what cannot be (`conflicting`): nothing is written.
    Inconsistent,
    /// A constraint lost its meaning on the way (a segment of no length, a circle of no radius: `singular`).
    Singular,
    /// The solver did not reach a solution in its steps.
    NonConvergent,
}

/// What a solve found, by the constraints' and the objects' ids.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ConstraintDiagnosis {
    pub status: SolveStatus,
    /// The freedoms left: unknowns less the independent equations.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub dof: u32,
    /// Constraints that repeat what the ones before them already say.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<ConstraintId>>", optional))]
    pub redundant: Vec<ConstraintId>,
    /// Constraints that cannot hold together with the ones before them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<ConstraintId>>", optional))]
    pub conflicting: Vec<ConstraintId>,
    /// Constraints that lost their meaning (a segment of no length …).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<ConstraintId>>", optional))]
    pub singular: Vec<ConstraintId>,
    /// Constraints whose objects or parts are gone, or are not what the rule takes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<ConstraintId>>", optional))]
    pub broken: Vec<ConstraintId>,
    /// Objects that can still move.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<EntityId>>", optional))]
    pub free: Vec<EntityId>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub iterations: u32,
}

/// An architectural bond in the way of a write (docs/adr/0780 §1): one of a `constraint_conflict`'s constraints with
/// the wall lines and jambs it holds, the value it keeps and what the drawing measures as the write leaves it, and
/// whether taking it away alone lets the write through. A refusal (`CommandError.bonds`) carries them so that a
/// program removes the right one (`cad.constraints.edit` `remove`) and the desktop marks them in place.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct BondInTheWay {
    pub id: ConstraintId,
    /// Its Turkish name (`Mesafe`, `Sabit`).
    pub label: String,
    /// A distance's parameter (`d1`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub parameter: Option<String>,
    /// The wall lines and jambs it names, in the rule's order (`a`, `b`, `axis`).
    pub parts: Vec<ElementRef>,
    /// A distance's value, metres: what it keeps.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub kept: Option<f64>,
    /// A distance's clear distance as the write leaves the drawing before the bonds follow, metres (along the wall
    /// for two jambs); none when its parts no longer fit it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub measured: Option<f64>,
    /// Taking this bond away alone lets the write through.
    pub frees: bool,
    /// When no bond alone lets the write through: the other bonds (their ids) of the smallest set whose taking away
    /// together does, this one in it (docs/adr/0790 §2): two, else three. Empty when a single bond frees it, when no
    /// set within the search's bound does, and on the bonds outside the set.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<ConstraintId>>", optional))]
    pub frees_with: Vec<ConstraintId>,
}

// Its numbers are finite lengths: a refusal compares whole (`CommandError: Eq`).
impl Eq for BondInTheWay {}

/// What kind of part an element is, for a rule's checks.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Shape {
    Point,
    Segment,
    Circle,
    WallLine,
    Jamb,
}

fn shape(e: &ElementRef) -> Shape {
    match e.element {
        CadElement::Point { .. } => Shape::Point,
        CadElement::Segment { .. } => Shape::Segment,
        CadElement::Circle => Shape::Circle,
        CadElement::Wall { .. } => Shape::WallLine,
        CadElement::Jamb { .. } => Shape::Jamb,
    }
}

fn element_problem(e: &ElementRef) -> Option<String> {
    if e.uid.is_nil() {
        return Some("nesne kimliği boş".into());
    }
    let index = |n: u32| n > SUBREF_INDEX_LIMIT;
    match e.element {
        CadElement::Point { place } => match place {
            SubPlace::Mid { .. } | SubPlace::Along { .. } => {
                Some("nokta öğesi bir uç, köşe, merkez ya da düğümdür; kenar ortası ya da eğri üstündeki yer değil".into())
            }
            SubPlace::Node { part } if index(part) => Some("düğümün sırası sınırı aşıyor".into()),
            SubPlace::Vertex { part, ring, index: i } if index(part) || index(ring) || index(i) => {
                Some("köşenin sırası sınırı aşıyor".into())
            }
            _ => None,
        },
        CadElement::Segment { part, ring, index: i } if index(part) || index(ring) || index(i) => {
            Some("kenarın sırası sınırı aşıyor".into())
        }
        CadElement::Wall { edge, .. } if index(edge) => Some("duvar kenarının sırası sınırı aşıyor".into()),
        _ => None,
    }
}

/// Whether a rule names an architectural part (docs/adr/0770 §4): the architectural adapter solves it, the general
/// solver never sees it.
pub fn rule_is_architectural(rule: &ConstraintRule) -> bool {
    rule.elements().iter().any(|e| e.element.is_architectural())
}

/// Why an architectural rule's parts do not fit it (docs/adr/0770 §4): only distance, parallel, symmetric and fix, and
/// architectural parts only.
fn architectural_problem(rule: &ConstraintRule, kinds: &[Shape]) -> Option<String> {
    use Shape::{Jamb, WallLine};
    if kinds.iter().any(|k| !matches!(k, WallLine | Jamb)) {
        return Some("duvar çizgisi ve pervaz yalnız duvar çizgisi ve pervazla bağlanır".into());
    }
    let fits = match rule {
        ConstraintRule::Distance { axis, left, .. } => {
            *axis == DistanceAxis::Aligned
                && match (kinds[0], kinds[1]) {
                    (WallLine, WallLine) | (Jamb, WallLine) => true,
                    (Jamb, Jamb) => !left,
                    _ => false,
                }
        }
        ConstraintRule::Parallel { .. } => kinds == [WallLine, WallLine],
        ConstraintRule::Symmetric { .. } => kinds == [WallLine, WallLine, WallLine],
        ConstraintRule::Fix { .. } => true,
        _ => false,
    };
    (!fits).then(|| {
        format!(
            "{} kısıtı duvar çizgisi ve pervaz almaz; mimari bağlar uzaklık, paralel, simetrik (eşit aralık) ve sabittir",
            rule_label(rule)
        )
    })
}

/// Why `rule`'s parts do not fit it (docs/adr/0334 §2).
fn rule_problem(rule: &ConstraintRule) -> Option<String> {
    use Shape::{Circle, Point, Segment};
    let elements = rule.elements();
    for e in &elements {
        if let Some(p) = element_problem(e) {
            return Some(p);
        }
    }
    for (i, a) in elements.iter().enumerate() {
        if elements[i + 1..].iter().any(|b| b == a) {
            return Some("aynı öğe iki kez anılmış".into());
        }
    }
    let kinds: Vec<Shape> = elements.iter().map(|e| shape(e)).collect();
    if rule_is_architectural(rule) {
        return architectural_problem(rule, &kinds);
    }
    let fits = match rule {
        ConstraintRule::Coincident { .. } => kinds[0] == Point,
        ConstraintRule::Horizontal { b, .. } | ConstraintRule::Vertical { b, .. } => match b {
            None => kinds[0] == Segment,
            Some(_) => kinds == [Point, Point],
        },
        ConstraintRule::Parallel { .. }
        | ConstraintRule::Perpendicular { .. }
        | ConstraintRule::Collinear { .. } => kinds == [Segment, Segment],
        ConstraintRule::Tangent { inside, left, .. } => match kinds[0] {
            Segment => kinds[1] == Circle && !inside,
            Circle => kinds[1] == Circle && !left,
            _ => false,
        },
        ConstraintRule::Concentric { .. } => kinds == [Circle, Circle],
        ConstraintRule::Equal { .. } => kinds == [Segment, Segment] || kinds == [Circle, Circle],
        ConstraintRule::Symmetric { .. } => kinds == [Point, Point, Segment],
        ConstraintRule::Fix { .. } => true,
        ConstraintRule::Distance { axis, left, .. } => match kinds[1] {
            Point => kinds[0] == Point && !left,
            Segment => kinds[0] == Point && *axis == DistanceAxis::Aligned,
            _ => false,
        },
        ConstraintRule::Angle { .. } => kinds == [Segment, Segment],
        ConstraintRule::Radius { .. } | ConstraintRule::Diameter { .. } => kinds[0] == Circle,
    };
    (!fits).then(|| format!("{} kısıtı bu öğeleri almaz", rule_label(rule)))
}

/// The rule's Turkish name.
pub fn rule_label(rule: &ConstraintRule) -> &'static str {
    match rule {
        ConstraintRule::Coincident { .. } => "Çakışık",
        ConstraintRule::Horizontal { .. } => "Yatay",
        ConstraintRule::Vertical { .. } => "Düşey",
        ConstraintRule::Parallel { .. } => "Paralel",
        ConstraintRule::Perpendicular { .. } => "Dik",
        ConstraintRule::Tangent { .. } => "Teğet",
        ConstraintRule::Concentric { .. } => "Eş merkezli",
        ConstraintRule::Collinear { .. } => "Doğrudaş",
        ConstraintRule::Equal { .. } => "Eşit",
        ConstraintRule::Symmetric { .. } => "Simetrik",
        ConstraintRule::Fix { .. } => "Sabit",
        ConstraintRule::Distance { .. } => "Uzaklık",
        ConstraintRule::Angle { .. } => "Açı",
        ConstraintRule::Radius { .. } => "Yarıçap",
        ConstraintRule::Diameter { .. } => "Çap",
    }
}

/// `set` as the objects `uids` names can hold it (docs/adr/0336 §3; a snapshot of a database project another editor
/// changed, another editor's set taken): the constraints naming an object no longer there go, a dimension's parameter
/// with its constraint unless another parameter reads it; then, while the set breaks its rules, the parameter or
/// constraint at fault goes, with the constraints naming such a parameter. What is left keeps the rules.
pub fn constraints_kept(set: &ConstraintSet, uids: &HashSet<EntityId>) -> ConstraintSet {
    let mut out = set.clone();
    let (gone, kept): (Vec<Constraint>, Vec<Constraint>) = std::mem::take(&mut out.constraints)
        .into_iter()
        .partition(|c| c.rule.elements().iter().any(|e| !uids.contains(&e.uid)));
    out.constraints = kept;
    let read: BTreeSet<String> = out
        .parameters
        .iter()
        .filter_map(|p| parse_param_expr(&p.expression).ok())
        .flat_map(|e| param_expr_names(&e))
        .collect();
    for c in &gone {
        if let Some(name) = c.rule.parameter()
            && !read.contains(name)
        {
            out.parameters.retain(|p| p.name != name);
        }
    }
    for _ in 0..(set.constraints.len() + set.parameters.len() + 1) {
        let Some((place, _)) = constraints_problem(&out, Some(uids)) else {
            break;
        };
        let index = |prefix: &str| {
            place
                .strip_prefix(prefix)
                .and_then(|i| i.parse::<usize>().ok())
        };
        if let Some(i) = index("parameters/") {
            let name = out.parameters.remove(i).name;
            out.constraints
                .retain(|c| c.rule.parameter() != Some(name.as_str()));
        } else if let Some(i) = index("constraints/") {
            out.constraints.remove(i);
        } else {
            return ConstraintSet::default();
        }
    }
    if constraints_problem(&out, Some(uids)).is_some() {
        return ConstraintSet::default();
    }
    out
}

/// Why `set` cannot be kept, and where (docs/adr/0334 §1): more than [`CONSTRAINTS_MAX`] constraints or
/// [`PARAMETERS_MAX`] parameters; an id given twice; a rule's parts that do not fit it, or an object not among `uids`
/// (when given); a reference geometric rule; a dimensional rule's parameter missing, of the wrong kind or shared; a
/// parameter's name not valid or given twice, its expression not parsed, empty where it must be written or written
/// where it is measured, a name it reads missing, names reading each other round, or bounds not finite or crossed.
/// The place is `constraints/<i>` or `parameters/<i>`.
pub fn constraints_problem(
    set: &ConstraintSet,
    uids: Option<&HashSet<EntityId>>,
) -> Option<(String, String)> {
    if set.constraints.len() > CONSTRAINTS_MAX {
        return Some((
            "constraints".into(),
            format!("en çok {CONSTRAINTS_MAX} kısıt tutulur"),
        ));
    }
    if set.parameters.len() > PARAMETERS_MAX {
        return Some((
            "parameters".into(),
            format!("en çok {PARAMETERS_MAX} parametre tutulur"),
        ));
    }
    let mut names: BTreeMap<&str, usize> = BTreeMap::new();
    for (i, p) in set.parameters.iter().enumerate() {
        let at = format!("parameters/{i}");
        if !parameter_name_valid(&p.name) {
            return Some((
                at,
                format!(
                    "“{}” bir parametre adı değil (harfle başlar; harf, rakam ve _)",
                    p.name
                ),
            ));
        }
        if names.insert(&p.name, i).is_some() {
            return Some((at, format!("“{}” adı iki kez verilmiş", p.name)));
        }
        if p.min.is_some_and(|v| !v.is_finite()) || p.max.is_some_and(|v| !v.is_finite()) {
            return Some((at, "sınırlar sonlu sayılardır".into()));
        }
        if let (Some(lo), Some(hi)) = (p.min, p.max)
            && lo > hi
        {
            return Some((at, "en az değer en çok değerden büyük".into()));
        }
        if p.description.chars().count() > 1_000 {
            return Some((at, "açıklama 1000 karakterden uzun".into()));
        }
    }
    let mut ids = HashSet::new();
    let mut owner: BTreeMap<&str, usize> = BTreeMap::new();
    for (i, c) in set.constraints.iter().enumerate() {
        let at = format!("constraints/{i}");
        if c.id.is_nil() || !ids.insert(c.id) {
            return Some((at, "kısıtın kimliği boş ya da iki kez verilmiş".into()));
        }
        if let Some(p) = rule_problem(&c.rule) {
            return Some((at, p));
        }
        if let Some(uids) = uids
            && let Some(e) = c
                .rule
                .elements()
                .into_iter()
                .find(|e| !uids.contains(&e.uid))
        {
            return Some((at, format!("{} kimlikli nesne çizimde yok", e.uid)));
        }
        match c.rule.parameter() {
            None if c.reference => return Some((at, "yalnız ölçü kısıtı başvuru olur".into())),
            None => {}
            Some(name) => {
                let Some(&k) = names.get(name) else {
                    return Some((at, format!("“{name}” diye bir parametre yok")));
                };
                if owner.insert(name, i).is_some() {
                    return Some((at, format!("“{name}” parametresi iki kısıtın")));
                }
                let want = match c.rule {
                    ConstraintRule::Angle { .. } => ParameterKind::Angle,
                    _ => ParameterKind::Length,
                };
                let p = &set.parameters[k];
                if p.kind != want {
                    return Some((
                        at,
                        format!("“{name}” parametresinin türü bu kısıta uymuyor"),
                    ));
                }
                if c.reference != p.expression.is_empty() {
                    let words = if c.reference {
                        "başvuru kısıtının parametresi ölçülür; ifadesi yazılmaz"
                    } else {
                        "sürücü kısıtın parametresinin bir ifadesi olur"
                    };
                    return Some((format!("parameters/{k}"), words.into()));
                }
            }
        }
    }
    // Expressions: parsed, reading known names, not round.
    let mut reads: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (i, p) in set.parameters.iter().enumerate() {
        let at = format!("parameters/{i}");
        if p.expression.is_empty() {
            if !owner.contains_key(p.name.as_str()) {
                return Some((at, "kullanıcı parametresinin bir ifadesi olur".into()));
            }
            reads.insert(p.name.clone(), BTreeSet::new());
            continue;
        }
        let expr = match parse_param_expr(&p.expression) {
            Ok(e) => e,
            Err(words) => return Some((at, format!("“{}” okunamadı: {words}", p.name))),
        };
        let used = param_expr_names(&expr);
        if let Some(missing) = used.iter().find(|n| !names.contains_key(n.as_str())) {
            return Some((at, format!("“{missing}” diye bir parametre yok")));
        }
        reads.insert(p.name.clone(), used);
    }
    if let Err(circle) = param_order(&reads) {
        let first = circle
            .first()
            .and_then(|n| names.get(n.as_str()))
            .copied()
            .unwrap_or(0);
        return Some((
            format!("parameters/{first}"),
            format!("parametreler birbirini okuyor: {}", circle.join(", ")),
        ));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_set_kept_by_the_objects_left_keeps_its_rules() {
        let set = rect();
        let all: HashSet<EntityId> = set
            .constraints
            .iter()
            .flat_map(|c| c.rule.elements().into_iter().map(|e| e.uid))
            .collect();
        assert_eq!(constraints_kept(&set, &all), set);
        let none = HashSet::new();
        let kept = constraints_kept(&set, &none);
        assert!(kept.constraints.is_empty());
        assert_eq!(constraints_problem(&kept, Some(&none)), None);
    }

    #[test]
    fn an_angle_s_flags_are_written_in_camel_case() {
        let rule: ConstraintRule = serde_json::from_value(serde_json::json!({
            "kind": "angle",
            "a": { "uid": "0192f5a6-1000-7000-8000-000000000101", "element": { "kind": "segment", "part": 0, "ring": 0, "index": 0 } },
            "b": { "uid": "0192f5a6-1000-7000-8000-000000000102", "element": { "kind": "segment", "part": 0, "ring": 0, "index": 0 } },
            "reverseB": true,
            "parameter": "a1"
        }))
        .expect("reads");
        let ConstraintRule::Angle { reverse_b, .. } = &rule else {
            panic!("an angle")
        };
        assert!(*reverse_b);
        let back = serde_json::to_value(&rule).expect("writes");
        assert_eq!(back["reverseB"], true);
        assert!(back.get("reverse_b").is_none());
    }

    fn uid(n: u8) -> EntityId {
        EntityId([n; 16])
    }

    fn point(n: u8, place: SubPlace) -> ElementRef {
        ElementRef {
            uid: uid(n),
            element: CadElement::Point { place },
        }
    }

    fn segment(n: u8) -> ElementRef {
        ElementRef {
            uid: uid(n),
            element: CadElement::Segment {
                part: 0,
                ring: 0,
                index: 0,
            },
        }
    }

    fn rect() -> ConstraintSet {
        ConstraintSet {
            parameters: vec![
                CadParameter {
                    name: "d1".into(),
                    kind: ParameterKind::Length,
                    expression: "genişlik * 2".into(),
                    min: None,
                    max: None,
                    description: String::new(),
                },
                CadParameter {
                    name: "genişlik".into(),
                    kind: ParameterKind::Length,
                    expression: "3".into(),
                    min: Some(0.0),
                    max: None,
                    description: String::new(),
                },
            ],
            constraints: vec![
                Constraint {
                    id: ConstraintId([1; 16]),
                    rule: ConstraintRule::Parallel {
                        a: segment(1),
                        b: segment(2),
                    },
                    reference: false,
                },
                Constraint {
                    id: ConstraintId([2; 16]),
                    rule: ConstraintRule::Distance {
                        a: point(1, SubPlace::Start),
                        b: point(1, SubPlace::End),
                        axis: DistanceAxis::Aligned,
                        left: false,
                        parameter: "d1".into(),
                    },
                    reference: false,
                },
            ],
        }
    }

    #[test]
    fn a_good_set_passes_and_reads_back() {
        let set = rect();
        assert_eq!(constraints_problem(&set, None), None);
        let text = serde_json::to_string(&set).unwrap();
        assert_eq!(serde_json::from_str::<ConstraintSet>(&text).unwrap(), set);
        assert!(text.contains("\"kind\":\"parallel\""), "{text}");
    }

    #[test]
    fn broken_sets_say_where() {
        let mut set = rect();
        set.constraints[0].rule = ConstraintRule::Parallel {
            a: segment(1),
            b: point(2, SubPlace::Start),
        };
        assert_eq!(constraints_problem(&set, None).unwrap().0, "constraints/0");
        let mut set = rect();
        set.parameters[1].expression = "d1".into();
        assert!(
            constraints_problem(&set, None)
                .unwrap()
                .1
                .contains("birbirini okuyor")
        );
        let mut set = rect();
        set.constraints[1].reference = true;
        assert_eq!(constraints_problem(&set, None).unwrap().0, "parameters/0");
        let set = rect();
        let some: HashSet<EntityId> = [uid(1)].into_iter().collect();
        assert!(
            constraints_problem(&set, Some(&some))
                .unwrap()
                .1
                .contains("çizimde yok")
        );
    }
}
