//! The constraint solver (docs/adr/0334; AutoCAD's geometric and dimensional constraints): given objects' shapes and
//! the constraints between their parts, the shapes moved the least that make every constraint hold, or why none do.
//!
//! - The unknowns are the objects' own numbers (`model`); each constraint gives one to four equations (`rules`), with
//!   exact derivatives from dual numbers (`dual`).
//! - The equations fall into independent groups (no unknown shared) solved one by one (`solve`): Gauss–Newton steps
//!   of the least change, the rows taken in the constraints' order and kept only while independent of the ones before.
//!   A row that repeats the ones before is redundant when it holds, conflicting when it does not.
//! - What it says: every constraint holds and nothing can move (`Fully`), or with freedoms left (`Under`), or some
//!   repeat others (`Over`); they ask for what cannot be (`Inconsistent`), a part lost its meaning (`Singular`), or the
//!   steps ran out (`NonConvergent`). Nothing is written by the solver; its caller writes `shapes` when it is content.
//!
//! The least change keeps the drawing's sense: a step moves the unknowns as little as the equations allow (an arc's
//! angles weighed by its radius), so a small edit does not turn the shape inside out; the caller still compares
//! orientations (`flipped`, docs/adr/0335).

pub mod dual;
mod model;
mod rules;
mod solve;
mod sparse;

use crate::entity::Shape;

/// A point of an object.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointAt {
    Start,
    End,
    Center,
    Node(u32),
    Vertex { part: u32, ring: u32, index: u32 },
}

/// A part of an object a constraint names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Element {
    Point(PointAt),
    /// A line (all zero) or a path's straight edge from vertex `index` to the next.
    Segment {
        part: u32,
        ring: u32,
        index: u32,
    },
    /// A circle or an arc's circle.
    Circle,
}

/// A part of the object at `object` in the problem's shapes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ref {
    pub object: usize,
    pub element: Element,
}

/// Which way a distance is measured.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    Aligned,
    Horizontal,
    Vertical,
}

/// A constraint's rule, as the contract's (`kentos_contracts::ConstraintRule`) with its parts by index.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Rule {
    Coincident {
        a: Ref,
        b: Ref,
    },
    Horizontal {
        a: Ref,
        b: Option<Ref>,
    },
    Vertical {
        a: Ref,
        b: Option<Ref>,
    },
    Parallel {
        a: Ref,
        b: Ref,
    },
    Perpendicular {
        a: Ref,
        b: Ref,
    },
    Tangent {
        a: Ref,
        b: Ref,
        inside: bool,
        left: bool,
    },
    Concentric {
        a: Ref,
        b: Ref,
    },
    Collinear {
        a: Ref,
        b: Ref,
    },
    Equal {
        a: Ref,
        b: Ref,
    },
    Symmetric {
        a: Ref,
        b: Ref,
        axis: Ref,
    },
    Fix {
        a: Ref,
    },
    Distance {
        a: Ref,
        b: Ref,
        axis: Axis,
        left: bool,
    },
    Angle {
        a: Ref,
        b: Ref,
        reverse_a: bool,
        reverse_b: bool,
    },
    Radius {
        a: Ref,
    },
    Diameter {
        a: Ref,
    },
}

impl Rule {
    /// The parts it names, in order.
    pub fn refs(&self) -> Vec<Ref> {
        match *self {
            Rule::Horizontal { a, b } | Rule::Vertical { a, b } => {
                std::iter::once(a).chain(b).collect()
            }
            Rule::Fix { a } | Rule::Radius { a } | Rule::Diameter { a } => vec![a],
            Rule::Symmetric { a, b, axis } => vec![a, b, axis],
            Rule::Coincident { a, b }
            | Rule::Parallel { a, b }
            | Rule::Perpendicular { a, b }
            | Rule::Tangent { a, b, .. }
            | Rule::Concentric { a, b }
            | Rule::Collinear { a, b }
            | Rule::Equal { a, b }
            | Rule::Distance { a, b, .. }
            | Rule::Angle { a, b, .. } => vec![a, b],
        }
    }

    /// Whether it carries a value (a dimension).
    pub fn dimensional(&self) -> bool {
        matches!(
            self,
            Rule::Distance { .. }
                | Rule::Angle { .. }
                | Rule::Radius { .. }
                | Rule::Diameter { .. }
        )
    }
}

/// One constraint: its rule, its value in metres or radians (a dimension's), and whether it only measures.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Constraint {
    pub rule: Rule,
    pub value: f64,
    pub reference: bool,
}

/// Unknowns the solve may not move: an object's every number (`element` none) or a part's (a dragged grip's).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hold {
    pub object: usize,
    pub element: Option<Element>,
}

/// What to solve.
pub struct Problem<'a> {
    pub shapes: &'a [Shape],
    pub constraints: &'a [Constraint],
    pub held: &'a [Hold],
    /// Where a fix holds its parts: these shapes' (an edit's before), else `shapes`' own.
    pub fix_from: Option<&'a [Shape]>,
}

/// How a solve ended (the contract's `SolveStatus`, and `Cancelled`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Fully,
    Under,
    Over,
    Inconsistent,
    Singular,
    NonConvergent,
    Cancelled,
}

/// What a solve found; the indices are the problem's constraints' and shapes'.
#[derive(Clone, Debug)]
pub struct Outcome {
    /// Every shape, the solved ones moved (all of them unchanged unless the status lets the caller write).
    pub shapes: Vec<Shape>,
    pub status: Status,
    pub dof: usize,
    pub redundant: Vec<usize>,
    pub conflicting: Vec<usize>,
    pub singular: Vec<usize>,
    /// Constraints whose parts the shapes do not have (gone, of another kind, a bulged edge).
    pub broken: Vec<usize>,
    /// Objects with a freedom left.
    pub free: Vec<usize>,
    /// The objects the solve moved.
    pub changed: Vec<usize>,
    /// Each reference constraint's measure after the solve.
    pub measured: Vec<(usize, f64)>,
    pub iterations: u32,
}

impl Status {
    /// Whether the solved shapes can be written: every constraint holds.
    pub fn holds(self) -> bool {
        matches!(self, Status::Fully | Status::Under | Status::Over)
    }
}

/// Solves `problem`, asking `stop` between steps (a long solve run as a job, docs/adr/0334 §6).
pub fn solve(problem: &Problem<'_>, stop: &dyn Fn() -> bool) -> Outcome {
    solve::run(problem, stop)
}

/// Whether `shape` has the part `element` (a straight edge, not a bulged one; a point it has).
pub fn has_part(shape: &Shape, element: &Element) -> bool {
    let shapes = std::slice::from_ref(shape);
    model::Model::new(shapes, &[true])
        .element_vars(shapes, 0, element)
        .is_some()
}

/// What a dimensional rule measures in `shapes`: a distance, an angle in [0, 2π), a radius or a diameter, in metres
/// or radians; none for a geometric rule or parts the shapes do not have.
pub fn measure(shapes: &[Shape], rule: &Rule) -> Option<f64> {
    solve::measure(shapes, rule)
}

#[cfg(test)]
mod tests;
