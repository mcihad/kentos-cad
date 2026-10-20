//! The grip menu (the web's `gripItems`, `ui/shell/viewportMenus.ts`,
//! docs/adr/0074, 0279): a right click on a grip of a selected object offers
//! what that grip can take, by the object:
//!
//! - a path's vertex: Köşeyi sil (its two edges merge into one straight edge);
//! - a path's edge middle: Ortasına köşe ekle (an arc edge is split in two,
//!   keeping its circle), then Düz kenar yap on an arc edge or Yaya dönüştür
//!   on a straight one: a gentle arc, its sagitta a quarter of the chord,
//!   bowing out of a ring or to the right of an open path;
//! - a path's edge middle, also (docs/adr/0288): Yay… (the edge, straight or
//!   an arc, becomes the arc through a point clicked or of a radius typed) and,
//!   on a polyline, Kenar genişliği… (its start and end widths typed): both
//!   start Kenar düzenle (`segment_edit`) on the edge;
//! - every grip: Esnet (the grip turns hot, the next click places it), then
//!   Taşı, Döndür, Ölçekle and Aynala with the grip as their base point, on
//!   the whole selection.
//!
//! A hole's vertices only move (Esnet; a hole's shape needs Patlat). A
//! multi-part area's grip acts on its own part, the others kept
//! (docs/adr/0143); the heading counts in that part.
//! The vertex and edge actions write through `cad.entities.vertex`, its
//! action naming the step (Köşe sil, Köşe ekle, Düz kenar yap, Yaya
//! dönüştür); with Topolojik düzenleme on, the shared corners and edges of
//! the objects around go in the same step (docs/adr/0160). The command's
//! refusal (a triangle's corner, a lock) is said; else “<step>: tamam.”.
//! The transforms are the host's: it starts the tool and gives it the grip.

use kentos_commands::{EntitiesVertex, StretchTopology, SubPlace, VertexAction};
use kentos_domain::Slot;
use kentos_geometry_core::entity::{Shape, area_parts};
use kentos_geometry_core::geom::bulge::{bulge_at, is_arc_bulge};
use kentos_geometry_core::ops::grips::{grip_part, hole_grip, mid_grip_segment};
use kentos_geometry_core::vec2::Vec2;
use kentos_native_application::geometry::shape;
use kentos_native_application::{ExecutionContext, entities_vertex};

use crate::log::Level;
use crate::points;
use crate::select::grip_at;
use crate::tool::Context;

/// The grip a menu is about: its object, the grip and where it stands.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GripMenu {
    pub slot: Slot,
    /// The grip's index among the object's grips (for Esnet).
    pub grip: usize,
    /// Where the grip stands: the transforms' base point.
    pub at: Vec2,
    /// The part of a multi-part area the grip is on (0: its first part, or any other object).
    pub part: usize,
    pub target: Target,
}

/// A path's vertex, the middle of one of its edges, or another grip.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    /// The vertex's index (from 0).
    Vertex(usize),
    /// The edge's index (from 0), whether it is an arc and whether its object is a polyline (it takes widths).
    Edge {
        segment: usize,
        arc: bool,
        polyline: bool,
    },
    /// A grip with no vertex action: a hole's vertex, a line's end, a circle's centre, a text's point.
    Grip,
}

/// What the grip menu does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GripAction {
    RemoveVertex,
    AddVertex,
    StraightEdge,
    ArcEdge,
    /// Yay… (docs/adr/0288): Kenar düzenle asks the point or the radius.
    ArcBy,
    /// Kenar genişliği… (docs/adr/0288): Kenar düzenle asks the widths.
    EdgeWidth,
    Stretch,
    Move,
    Rotate,
    Scale,
    Mirror,
}

/// The transforms every grip offers, after the vertex actions.
const TRANSFORMS: [GripAction; 5] = [
    GripAction::Stretch,
    GripAction::Move,
    GripAction::Rotate,
    GripAction::Scale,
    GripAction::Mirror,
];

impl GripAction {
    /// The item's words.
    pub fn label(self) -> &'static str {
        match self {
            GripAction::RemoveVertex => "Köşeyi sil",
            GripAction::AddVertex => "Ortasına köşe ekle",
            GripAction::StraightEdge => "Düz kenar yap",
            GripAction::ArcEdge => "Yaya dönüştür",
            GripAction::ArcBy => "Noktadan ya da yarıçapla yay…",
            GripAction::EdgeWidth => "Kenar genişliği…",
            GripAction::Stretch => "Esnet",
            GripAction::Move => "Taşı",
            GripAction::Rotate => "Döndür",
            GripAction::Scale => "Ölçekle",
            GripAction::Mirror => "Aynala",
        }
    }

    /// The web's icon of the item.
    pub fn icon(self) -> &'static str {
        match self {
            GripAction::RemoveVertex => "erase",
            GripAction::AddVertex => "vertex",
            GripAction::StraightEdge => "line",
            GripAction::ArcEdge => "arc",
            GripAction::ArcBy => "arcCenter",
            GripAction::EdgeWidth => "lineWeight",
            GripAction::Stretch => "stretch",
            GripAction::Move => "move",
            GripAction::Rotate => "rotate",
            GripAction::Scale => "scale",
            GripAction::Mirror => "mirror",
        }
    }

    /// The tool a transform starts with the grip as its first point; none for the others.
    pub fn tool(self) -> Option<&'static str> {
        match self {
            GripAction::Move => Some("move"),
            GripAction::Rotate => Some("rotate"),
            GripAction::Scale => Some("scale"),
            GripAction::Mirror => Some("mirror"),
            _ => None,
        }
    }

    /// The command's action of a vertex or edge item; none for the transforms.
    pub fn vertex_action(self) -> Option<VertexAction> {
        match self {
            GripAction::RemoveVertex => Some(VertexAction::Remove),
            GripAction::AddVertex => Some(VertexAction::Insert),
            GripAction::StraightEdge => Some(VertexAction::Straighten),
            GripAction::ArcEdge => Some(VertexAction::Arc),
            _ => None,
        }
    }

    /// What Kenar düzenle asks for an edge item that takes a value (docs/adr/0288); none for the others.
    pub fn segment_edit(self) -> Option<crate::segment_edit::Ask> {
        match self {
            GripAction::ArcBy => Some(crate::segment_edit::Ask::Arc),
            GripAction::EdgeWidth => Some(crate::segment_edit::Ask::Width),
            _ => None,
        }
    }
}

impl GripMenu {
    /// The menu's heading: “Köşe N”, “Kenar N” or “Tutamaç”.
    pub fn header(&self) -> String {
        match self.target {
            Target::Vertex(i) => format!("Köşe {}", i + 1),
            Target::Edge { segment, .. } => format!("Kenar {}", segment + 1),
            Target::Grip => "Tutamaç".to_owned(),
        }
    }

    /// The items: the vertex or edge actions in the web's order, then the transforms.
    pub fn actions(&self) -> Vec<GripAction> {
        let own: &[GripAction] = match self.target {
            Target::Vertex(_) => &[GripAction::RemoveVertex],
            Target::Edge {
                arc: true,
                polyline: true,
                ..
            } => &[
                GripAction::AddVertex,
                GripAction::StraightEdge,
                GripAction::ArcBy,
                GripAction::EdgeWidth,
            ],
            Target::Edge {
                arc: false,
                polyline: true,
                ..
            } => &[
                GripAction::AddVertex,
                GripAction::ArcEdge,
                GripAction::ArcBy,
                GripAction::EdgeWidth,
            ],
            Target::Edge { arc: true, .. } => &[
                GripAction::AddVertex,
                GripAction::StraightEdge,
                GripAction::ArcBy,
            ],
            Target::Edge { arc: false, .. } => &[
                GripAction::AddVertex,
                GripAction::ArcEdge,
                GripAction::ArcBy,
            ],
            Target::Grip => &[],
        };
        own.iter().chain(&TRANSFORMS).copied().collect()
    }

    /// The vertex actions are separate from the transforms after this many items.
    pub fn own_count(&self) -> usize {
        self.actions().len() - TRANSFORMS.len()
    }
}

/// The grip menu for the grip at `screen` (logical pixels on the drawing
/// area): none when no selected object's grip is there.
pub fn at(screen: [f64; 2], cx: &Context<'_>) -> Option<GripMenu> {
    let (slot, grip, at) = grip_at(screen, cx)?;
    let whole = shape(cx.doc.get(slot)?);
    // The grip's own part and its index there (docs/adr/0143).
    let (part, index) = grip_part(&whole, grip)?;
    let parts = area_parts(&whole);
    let s = parts.get(part)?;
    let bulges = match s {
        Shape::Polyline { bulges, .. } | Shape::Polygon { bulges, .. } => Some(bulges.as_deref()),
        _ => None,
    };
    let target = match bulges {
        Some(_) if hole_grip(s, index).is_some() => Target::Grip,
        Some(bulges) => match mid_grip_segment(s, index) {
            None => Target::Vertex(index),
            Some(segment) => Target::Edge {
                segment,
                arc: is_arc_bulge(bulge_at(bulges, segment)),
                polyline: matches!(s, Shape::Polyline { .. }) && parts.len() == 1,
            },
        },
        None => Target::Grip,
    };
    Some(GripMenu {
        slot,
        grip,
        at,
        part,
        target,
    })
}

/// The command's input for `action` on the menu's grip; none for a transform or a grip with no vertex action.
/// Yay… and Kenar genişliği… give `cad.entities.vertex`'s `arc` and `width` on the edge, their values to come
/// (docs/adr/0288).
pub fn request(menu: GripMenu, action: GripAction, cx: &Context<'_>) -> Option<EntitiesVertex> {
    let action = match action {
        GripAction::ArcBy => VertexAction::Arc,
        GripAction::EdgeWidth => VertexAction::Width,
        other => other.vertex_action()?,
    };
    let uid = cx.doc.uid(menu.slot)?;
    let part = menu.part as u32;
    let place = match menu.target {
        Target::Vertex(i) => SubPlace::Vertex {
            part,
            ring: 0,
            index: i as u32,
        },
        Target::Edge { segment, .. } => SubPlace::Mid {
            part,
            ring: 0,
            index: segment as u32,
        },
        Target::Grip => return None,
    };
    Some(EntitiesVertex {
        uid: uid.to_string(),
        action,
        place,
        through: None,
        radius: None,
        widths: None,
        topology: cx.draft.topology.then_some(StretchTopology {
            points: cx.draft.topology_points,
        }),
        expected_revision: None,
    })
}

/// Kenar düzenle on the menu's edge for Yay… and Kenar genişliği… (docs/adr/0288); none for the other items.
pub fn segment_tool(
    menu: GripMenu,
    action: GripAction,
    cx: &Context<'_>,
) -> Option<crate::segment_edit::SegmentEdit> {
    let ask = action.segment_edit()?;
    let Target::Edge { segment, .. } = menu.target else {
        return None;
    };
    let input = request(menu, action, cx)?;
    let whole = shape(cx.doc.get(menu.slot)?);
    let parts = area_parts(&whole);
    let pts = match parts.get(menu.part)? {
        Shape::Polyline { pts, .. } | Shape::Polygon { pts, .. } => pts.clone(),
        _ => return None,
    };
    let (a, b) = (*pts.get(segment)?, *pts.get((segment + 1) % pts.len())?);
    Some(crate::segment_edit::SegmentEdit::new(input, ask, a, b))
}

/// Does a vertex or edge action on the menu's object through
/// `cad.entities.vertex`, and says how it went.
pub fn apply(menu: GripMenu, action: GripAction, cx: &mut Context<'_>) {
    let Some(input) = request(menu, action, cx) else {
        return;
    };
    let step = entities_vertex::label(input.action);
    let mut ecx = ExecutionContext::with_store(cx.doc, cx.spatial.store());
    let result = entities_vertex::execute(&mut ecx, input);
    if let Some(output) = points::written(result, cx) {
        if output.changed.is_empty() {
            return;
        }
        cx.say(Level::Success, format!("{step}: tamam."));
        if output.changed.len() > 1 {
            cx.say(
                Level::Info,
                format!(
                    "Topolojik düzenleme: {} komşu nesne de değişti.",
                    output.changed.len() - 1
                ),
            );
        }
    }
}
