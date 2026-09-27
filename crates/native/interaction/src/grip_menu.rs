//! The grip menu (the web's `gripItems`, `ui/shell/viewportMenus.ts`,
//! docs/adr/0074): a right click on a grip of a selected path, an open
//! polyline or a closed area, offers what that grip's vertex or edge can
//! take:
//!
//! - a vertex: Köşeyi sil (its two edges merge into one straight edge);
//! - an edge's middle: Ortasına köşe ekle (an arc edge is split in two,
//!   keeping its circle), then Düz kenar yap on an arc edge or Yaya dönüştür
//!   on a straight one: a gentle arc, its sagitta a quarter of the chord,
//!   bowing out of a ring or to the right of an open path.
//!
//! A hole's vertices only move by dragging (a hole's shape needs Patlat).
//! Each action writes through `cad.entities.edit`, its operation naming the
//! step (Köşe sil, Köşe ekle, Düz kenar yap, Yaya dönüştür); the object's
//! geometry stays but for what the action gives, so a hole stays. The
//! core's refusal (a triangle's corner) or the command's (a lock) is said;
//! else “<step>: tamam.”.

use kentos_contracts::{EditOperation, EntitiesEdit, EntityEdit};
use kentos_domain::Slot;
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::bulge::{bulge_at, bulge_ring_area, is_arc_bulge, segment_mid};
use kentos_geometry_core::ops::curve_cuts::Geometry;
use kentos_geometry_core::ops::grips::{hole_grip, mid_grip_segment};
use kentos_geometry_core::ops::vertex::{insert_vertex, remove_vertex};
use kentos_geometry_core::vec2::Vec2;
use kentos_native_application::geometry::{edit_geometry, shape};
use kentos_native_application::{ExecutionContext, edit};

use crate::log::Level;
use crate::points;
use crate::select::grip_at;
use crate::tool::Context;

/// The grip a menu is about: its object, and a vertex or an edge's middle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GripMenu {
    pub slot: Slot,
    pub target: Target,
}

/// A path's vertex, or the middle of one of its edges.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    /// The vertex's index (from 0).
    Vertex(usize),
    /// The edge's index (from 0) and whether it is an arc.
    Edge { segment: usize, arc: bool },
}

/// What the grip menu does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GripAction {
    RemoveVertex,
    AddVertex,
    StraightEdge,
    ArcEdge,
}

impl GripAction {
    /// The item's words (the web's).
    pub fn label(self) -> &'static str {
        match self {
            GripAction::RemoveVertex => "Köşeyi sil",
            GripAction::AddVertex => "Ortasına köşe ekle",
            GripAction::StraightEdge => "Düz kenar yap",
            GripAction::ArcEdge => "Yaya dönüştür",
        }
    }

    /// The web's icon of the item.
    pub fn icon(self) -> &'static str {
        match self {
            GripAction::RemoveVertex => "erase",
            GripAction::AddVertex => "vertex",
            GripAction::StraightEdge => "line",
            GripAction::ArcEdge => "arc",
        }
    }
}

impl GripMenu {
    /// The menu's heading: “Köşe N” or “Kenar N”.
    pub fn header(&self) -> String {
        match self.target {
            Target::Vertex(i) => format!("Köşe {}", i + 1),
            Target::Edge { segment, .. } => format!("Kenar {}", segment + 1),
        }
    }

    /// The items, in the web's order.
    pub fn actions(&self) -> Vec<GripAction> {
        match self.target {
            Target::Vertex(_) => vec![GripAction::RemoveVertex],
            Target::Edge { arc: true, .. } => {
                vec![GripAction::AddVertex, GripAction::StraightEdge]
            }
            Target::Edge { arc: false, .. } => vec![GripAction::AddVertex, GripAction::ArcEdge],
        }
    }
}

/// A path's vertices and bulges, and whether it is closed.
struct Path<'a> {
    pts: &'a [Vec2],
    bulges: Option<&'a [f64]>,
    closed: bool,
}

fn path(s: &Shape) -> Option<Path<'_>> {
    match s {
        Shape::Polyline { pts, bulges, .. } => Some(Path {
            pts,
            bulges: bulges.as_deref(),
            closed: false,
        }),
        Shape::Polygon { pts, bulges, .. } => Some(Path {
            pts,
            bulges: bulges.as_deref(),
            closed: true,
        }),
        _ => None,
    }
}

/// The grip menu for the grip at `screen` (logical pixels on the drawing
/// area): none when no selected path's grip is there, or it is a hole's vertex.
pub fn at(screen: [f64; 2], cx: &Context<'_>) -> Option<GripMenu> {
    let (slot, index, _) = grip_at(screen, cx)?;
    let s = shape(cx.doc.get(slot)?);
    let bulges = path(&s)?.bulges;
    if hole_grip(&s, index).is_some() {
        return None;
    }
    let target = match mid_grip_segment(&s, index) {
        None => Target::Vertex(index),
        Some(segment) => Target::Edge {
            segment,
            arc: is_arc_bulge(bulge_at(bulges, segment)),
        },
    };
    Some(GripMenu { slot, target })
}

/// Does `action` on the menu's object, through `cad.entities.edit`, and
/// says how it went.
pub fn apply(menu: GripMenu, action: GripAction, cx: &mut Context<'_>) {
    let (Some(e), Some(uid)) = (cx.doc.get(menu.slot).cloned(), cx.doc.uid(menu.slot)) else {
        return;
    };
    let s = shape(&e);
    let Some(Path {
        pts,
        bulges,
        closed,
    }) = path(&s)
    else {
        return;
    };
    let n = pts.len();
    let (operation, edited): (EditOperation, Result<Shape, String>) = match (action, menu.target) {
        (GripAction::RemoveVertex, Target::Vertex(i)) => {
            (EditOperation::VertexRemove, shape_of(remove_vertex(&s, i)))
        }
        (GripAction::AddVertex, Target::Edge { segment, .. }) => {
            let (Some(&a), Some(&b)) = (pts.get(segment), pts.get((segment + 1) % n.max(1))) else {
                return;
            };
            let mid = segment_mid(a, b, bulge_at(bulges, segment));
            (
                EditOperation::VertexAdd,
                insert_vertex(&s, segment, mid).and_then(shape_of),
            )
        }
        (GripAction::StraightEdge, Target::Edge { segment, .. }) => (
            EditOperation::StraightEdge,
            Ok(with_bulge(pts, bulges, closed, segment, 0.0)),
        ),
        (GripAction::ArcEdge, Target::Edge { segment, .. }) => {
            // Outside of a counter-clockwise ring is right of travel, where a positive bulge bows.
            let outward = if closed && bulge_ring_area(pts, bulges) <= 0.0 {
                -0.5
            } else {
                0.5
            };
            (
                EditOperation::ArcEdge,
                Ok(with_bulge(pts, bulges, closed, segment, outward)),
            )
        }
        _ => return,
    };
    let edited = match edited {
        Ok(edited) => edited,
        Err(refusal) => {
            cx.say(Level::Warn, refusal);
            return;
        }
    };
    // The object's geometry stays but for what the action gives: a hole stays.
    let Some(geometry) = edit_geometry(keep_holes(&s, edited)) else {
        return;
    };
    let input = EntitiesEdit {
        operation,
        changes: vec![EntityEdit::Update {
            uid: uid.to_string(),
            geometry,
        }],
        expected_revision: None,
    };
    let result = edit::execute(&mut ExecutionContext::new(cx.doc), input);
    if points::written(result, cx).is_some() {
        cx.say(
            Level::Success,
            format!("{}: tamam.", edit::label(operation)),
        );
    }
}

/// The core's answer as a shape, or its refusal.
fn shape_of(g: Geometry) -> Result<Shape, String> {
    match g {
        Geometry::Ok(e) => Ok(e.shape),
        Geometry::Error(refusal) => Err(refusal),
    }
}

/// The path with edge `segment` bent to `bulge`; its bulges kept only while
/// one is an arc (the web's `withBulge`).
fn with_bulge(
    pts: &[Vec2],
    bulges: Option<&[f64]>,
    closed: bool,
    segment: usize,
    bulge: f64,
) -> Shape {
    let all: Vec<f64> = (0..pts.len())
        .map(|i| {
            if i == segment {
                bulge
            } else {
                bulge_at(bulges, i)
            }
        })
        .collect();
    let bulges = all.iter().any(|&b| is_arc_bulge(b)).then_some(all);
    if closed {
        Shape::Polygon {
            pts: pts.to_vec(),
            bulges,
            holes: None,
        }
    } else {
        Shape::Polyline {
            pts: pts.to_vec(),
            bulges,
            holes: None,
        }
    }
}

/// An area's holes kept on its edited ring (the web's `{...geometryOf(e), ...edit}`).
fn keep_holes(before: &Shape, edited: Shape) -> Shape {
    match (before, edited) {
        (
            Shape::Polygon {
                holes: Some(holes), ..
            },
            Shape::Polygon { pts, bulges, .. },
        ) => Shape::Polygon {
            pts,
            bulges,
            holes: Some(holes.clone()),
        },
        (_, edited) => edited,
    }
}
