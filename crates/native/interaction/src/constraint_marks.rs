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
    if element.is_architectural() {
        // An architectural part (docs/adr/0770 §6): its adapter says where it is.
        let (at, line) = kentos_native_application::arch_bonds::part_place(doc, uid, element)?;
        return Some((at, line.map(|(a, b)| vec![vec![a, b]]).unwrap_or_default()));
    }
    let (e, shape) = object(doc, uid)?;
    let core = core_entity(e);
    match element {
        CadElement::Point { place } => {
            let at = point(&core, core_place(*place))?;
            Some((at, Vec::new()))
        }
        // A map bond's boundary edge is drawn where its straight edge is (docs/adr/2580).
        CadElement::Segment { part, ring, index } | CadElement::Boundary { part, ring, index } => {
            let place = Place::Mid {
                part: *part as usize,
                ring: *ring as usize,
                index: *index as usize,
            };
            let at = point(&core, place)?;
            let (a, b) = kentos_geometry_core::ops::subref::edge_line(&core, place)?;
            Some((at, vec![vec![a, b]]))
        }
        CadElement::ArcEdge { part, ring, index } => {
            let arc = kentos_geometry_core::ops::subref::arc_edge_of(
                &shape,
                *part as usize,
                *ring as usize,
                *index as usize,
            )?;
            let at = point(
                &core,
                Place::Mid {
                    part: *part as usize,
                    ring: *ring as usize,
                    index: *index as usize,
                },
            )?;
            let edge = kentos_geometry_core::geom::intersect::Edge::Arc {
                c: arc.c,
                r: arc.r,
                a0: arc.a0,
                sweep: arc.sweep,
            };
            Some((at, vec![chords(&edge)]))
        }
        CadElement::Wall { .. } | CadElement::Jamb { .. } => None,
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

/// A parameter value already expressed in its project unit, shared by scene marks and the parameter window.
/// Display precision never changes a stored value; small nonzero values use scientific notation.
fn value_words(v: f64) -> String {
    if !v.is_finite() {
        "—".into()
    } else if v == 0.0 {
        "0".into()
    } else if v.abs() >= 1e12 || v.abs() < 1e-6 {
        let text = format!("{v:.11e}");
        let Some((mantissa, exponent)) = text.split_once('e') else {
            return text;
        };
        format!(
            "{}e{exponent}",
            mantissa.trim_end_matches('0').trim_end_matches('.')
        )
    } else {
        let precision = (11.0 - v.abs().log10().floor()).clamp(0.0, 17.0) as usize;
        let text = format!("{v:.precision$}");
        if precision == 0 {
            text
        } else {
            text.trim_end_matches('0').trim_end_matches('.').into()
        }
    }
}

pub fn parameter_value_words(
    v: f64,
    kind: ParameterKind,
    settings: &kentos_commands::ProjectSettings,
) -> String {
    let value = value_words(v);
    match kind {
        ParameterKind::Length => format!("{value} {}", settings.unit().mark()),
        ParameterKind::Angle => format!(
            "{value} {}",
            match settings.angle_unit {
                kentos_commands::AngleUnit::Deg => "°",
                kentos_commands::AngleUnit::Grad => "gon",
            }
        ),
        ParameterKind::Number => format!("{value} · sayı"),
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
                Some(v) => format!(
                    "{name} = {}",
                    parameter_value_words(*v, kind, doc.settings())
                ),
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
