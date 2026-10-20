//! The constraints over the drawing (docs/adr/0338 §1, plan/cad.md `CAD-PARAM-11`; AutoCAD's constraint bars): a mark
//! by each part a constraint names, its kind's icon, a dimension's name and value beside it, a constraint the drawing
//! does not meet marked as such; and the parts one constraint names, for the drawing to show when the pointer is on
//! its mark. Read from the document; nothing is written.

use kentos_commands::{
    CadElement, ConstraintId, ConstraintRule, Entity, ParameterKind, rule_label,
};
use kentos_domain::constraints::core_shape;
use kentos_domain::dimension_ties::core_place;
use kentos_domain::{Document, Uuid};
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::ops::edges::entity_edges;
use kentos_geometry_core::ops::subref::{Place, point};
use kentos_native_application::blocks::core_entity;
use kentos_native_application::constraint_solve::{parameter_values, unmet};

use crate::Vec2;
use crate::dimension_bonds::{Bond, chords};

/// At most this many constraints show their marks: a large set is listed in Parametreler, not covered in marks.
pub const MOST_CONSTRAINTS: usize = 2000;

/// One mark: a constraint by one part it names.
#[derive(Clone, Debug, PartialEq)]
pub struct Glyph {
    pub id: ConstraintId,
    /// The constraint's place in the set.
    pub index: usize,
    /// The kind's icon (`catalog/v1/additions.json`).
    pub icon: &'static str,
    /// The kind's name (“Paralel”).
    pub title: &'static str,
    /// Where on the part: an edge's middle, a point, a circle's top.
    pub at: Vec2,
    /// A dimension's name and value (“d1 = 8”), on its first mark only.
    pub label: Option<String>,
    /// The drawing does not meet it (a set that could not be solved, an edit relaxed elsewhere).
    pub unmet: bool,
    /// A dimension that only measures.
    pub reference: bool,
}

fn icon(rule: &ConstraintRule) -> &'static str {
    match rule {
        ConstraintRule::Coincident { .. } => "constraintCoincident",
        ConstraintRule::Horizontal { .. } => "constraintHorizontal",
        ConstraintRule::Vertical { .. } => "constraintVertical",
        ConstraintRule::Parallel { .. } => "constraintParallel",
        ConstraintRule::Perpendicular { .. } => "constraintPerpendicular",
        ConstraintRule::Tangent { .. } => "constraintTangent",
        ConstraintRule::Concentric { .. } => "constraintConcentric",
        ConstraintRule::Collinear { .. } => "constraintCollinear",
        ConstraintRule::Equal { .. } => "constraintEqual",
        ConstraintRule::Symmetric { .. } => "constraintSymmetric",
        ConstraintRule::Fix { .. } => "constraintFix",
        ConstraintRule::Distance { axis, .. } => match axis {
            kentos_commands::DistanceAxis::Horizontal => "constraintHorizontalDistance",
            kentos_commands::DistanceAxis::Vertical => "constraintVerticalDistance",
            kentos_commands::DistanceAxis::Aligned => "constraintDistance",
        },
        ConstraintRule::Angle { .. } => "constraintAngle",
        ConstraintRule::Radius { .. } => "constraintRadius",
        ConstraintRule::Diameter { .. } => "constraintDiameter",
    }
}

/// The object a part names, as the core reads it.
fn object(doc: &Document, uid: kentos_commands::EntityId) -> Option<(&Entity, Shape)> {
    let e = doc
        .slot_of(Uuid::from_bytes(uid.0))
        .and_then(|s| doc.get(s))?;
    Some((e, core_shape(e)?))
}

/// Where a part's mark goes, and its edges as runs of points.
fn part(
    doc: &Document,
    uid: kentos_commands::EntityId,
    element: &CadElement,
) -> Option<(Vec2, Vec<Vec<Vec2>>)> {
    let (e, shape) = object(doc, uid)?;
    let core = core_entity(e);
    match element {
        CadElement::Point { place } => {
            let at = point(&core, core_place(*place))?;
            Some((at, Vec::new()))
        }
        CadElement::Segment { part, ring, index } => {
            let place = Place::Mid {
                part: *part as usize,
                ring: *ring as usize,
                index: *index as usize,
            };
            let at = point(&core, place)?;
            let (a, b) = kentos_geometry_core::ops::subref::edge_line(&core, place)?;
            Some((at, vec![vec![a, b]]))
        }
        CadElement::Circle => {
            let (c, r) = match shape {
                Shape::Circle { c, r } | Shape::Arc { c, r, .. } => (c, r),
                _ => return None,
            };
            let edges = entity_edges(&shape).iter().map(chords).collect();
            Some((Vec2::new(c.x, c.y + r), edges))
        }
    }
}

/// A value as a dimension's mark writes it: three decimals at most, no trailing zeros.
fn value_words(v: f64, kind: ParameterKind) -> String {
    let t = format!("{v:.3}");
    let t = t.trim_end_matches('0').trim_end_matches('.');
    let t = if t == "-0" { "0" } else { t };
    match kind {
        ParameterKind::Angle => format!("{t}°"),
        _ => t.to_owned(),
    }
}

/// The marks of the drawing's constraints, each part's in the set's order.
pub fn glyphs(doc: &Document) -> Vec<Glyph> {
    let set = doc.constraints();
    if set.constraints.is_empty() {
        return Vec::new();
    }
    let unmet = unmet(doc, set, None, None);
    let values = parameter_values(doc, set).unwrap_or_default();
    let mut out = Vec::new();
    for (index, c) in set.constraints.iter().enumerate().take(MOST_CONSTRAINTS) {
        let label = c.rule.parameter().map(|name| {
            let kind = set
                .parameter(name)
                .map_or(ParameterKind::Length, |p| p.kind);
            match values.get(name) {
                Some(v) => format!("{name} = {}", value_words(*v, kind)),
                None => name.to_owned(),
            }
        });
        let mut placed: Vec<Vec2> = Vec::new();
        for (k, e) in c.rule.elements().into_iter().enumerate() {
            let Some((at, _)) = part(doc, e.uid, &e.element) else {
                continue;
            };
            // Two parts at one place (a coincident's two ends) show one mark.
            if placed
                .iter()
                .any(|p| (p.x - at.x).abs() < 1e-9 && (p.y - at.y).abs() < 1e-9)
            {
                continue;
            }
            placed.push(at);
            out.push(Glyph {
                id: c.id,
                index,
                icon: icon(&c.rule),
                title: rule_label(&c.rule),
                at,
                label: if k == 0 { label.clone() } else { None },
                unmet: unmet.contains(&index),
                reference: c.reference,
            });
        }
    }
    out
}

/// The parts the constraint at `index` names, for the drawing to show while the pointer is on one of its marks.
pub fn bound(doc: &Document, index: usize) -> Vec<Bond> {
    let Some(c) = doc.constraints().constraints.get(index) else {
        return Vec::new();
    };
    c.rule
        .elements()
        .into_iter()
        .filter_map(|e| {
            let (at, edges) = part(doc, e.uid, &e.element)?;
            Some(Bond { at, edges })
        })
        .collect()
}
