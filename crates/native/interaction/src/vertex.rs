//! Köşe ekle/sil: the web's `VertexTool` (`apps/web/src/tools/pathEditTools.ts`)
//! on its edge-picking base, step for step (docs/adr/0047):
//!
//! - a click near a vertex (7 px) of a polyline or a closed area removes it;
//!   anywhere else on an edge of a line, a polyline or a closed area adds
//!   one there (a line becomes a polyline); the pointer shows which, a red ×
//!   or a +, as it moves;
//! - a closed area's holes are not edited here: the tool says to explode it;
//! - the tool stays until Esc.
//!
//! The object keeps its place and persistent id, through `cad.entities.vertex`
//! (docs/adr/0279): the click is a place on the object, a vertex or a point
//! along its edge (“Köşe eklendi.”, “Köşe silindi.”); a closed area keeps its
//! holes. The new geometry is the command's; the segment clicked, the place
//! and the hole check are the shared core's (`nearest_segment`, `place_at`,
//! `near_hole`).

use kentos_commands::{EntitiesVertex, Entity, SubPlace, VertexAction};
use kentos_domain::Document;
use kentos_domain::dimension_ties::contract_place;
use kentos_geometry_core::geom::intersect::closest_on_edge;
use kentos_geometry_core::geometry::dist;
use kentos_geometry_core::ops::edges::entity_edges;
use kentos_geometry_core::ops::subref::place_at;
use kentos_geometry_core::ops::vertex::{near_hole, nearest_segment};
use kentos_geometry_core::store::snap::SnapKind;
use kentos_native_application::blocks::core_entity;
use kentos_native_application::geometry::shape;
use kentos_native_application::{ExecutionContext, entities_vertex};

use crate::Vec2;
use crate::edge;
use crate::format::Format;
use crate::log::Level;
use crate::points;
use crate::prompt::Prompt;
use crate::tool::{Context, Cursor, Flow, Marker, MarkerShape, Pointer, Preview, Tag, Tone, Tool};

/// The vertex tool's id: its command is `tool.vertex`.
pub const ID: &str = "vertex";
pub const LABEL: &str = "Köşe ekle/sil";

/// How near a vertex a click removes it, logical pixels (the web's `VERTEX_PX`).
const VERTEX_PX: f64 = 7.0;

/// The kinds the vertex tool takes, off locked layers.
fn editable(e: &Entity, doc: &Document) -> bool {
    matches!(
        e,
        Entity::Line(_) | Entity::Polyline(_) | Entity::Polygon(_)
    ) && edge::unlocked(e, doc)
}

/// What a click would do: remove vertex `remove`, or add one on segment `seg` at `at`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Action {
    remove: Option<usize>,
    /// The vertex to remove as a place (its part and index there).
    place: Option<SubPlace>,
    seg: usize,
    at: Vec2,
}

/// The vertex tool.
#[derive(Clone, Debug, Default)]
pub struct Vertex {
    action: Option<Action>,
}

impl Vertex {
    pub fn new() -> Self {
        Self::default()
    }

    /// The web's `plan`: a vertex within reach is removed, else one is added
    /// on the nearest segment (a line's only one).
    fn plan(e: &Entity, p: &Pointer, cx: &Context<'_>) -> Action {
        let tol = cx.view.world_length(VERTEX_PX);
        if let Entity::Polyline(path) | Entity::Polygon(path) = e {
            // Every part's outer vertices, part after part, as the core's `remove_vertex`
            // counts them (docs/adr/0143).
            let rings =
                std::iter::once(&path.pts).chain(path.parts.iter().flatten().map(|q| &q.pts));
            let mut offset = 0;
            for (part, ring) in rings.enumerate() {
                if let Some(i) = ring
                    .iter()
                    .position(|q| dist(Vec2::new(q.x, q.y), p.raw) <= tol)
                {
                    let q = ring[i];
                    return Action {
                        remove: Some(offset + i),
                        place: Some(SubPlace::Vertex {
                            part: part as u32,
                            ring: 0,
                            index: i as u32,
                        }),
                        seg: 0,
                        at: Vec2::new(q.x, q.y),
                    };
                }
                offset += ring.len();
            }
        }
        let seg = match e {
            Entity::Line(_) => 0,
            _ => nearest_segment(&shape(e), p.raw),
        };
        Action {
            remove: None,
            place: None,
            seg,
            at: p.raw,
        }
    }
}

impl Tool for Vertex {
    /// An object is picked (the web's `cursor = 'pick'`).
    fn cursor(&self) -> Cursor {
        Cursor::Pick
    }

    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn prompt(&self) -> Prompt {
        Prompt::new(
            LABEL,
            "kenara tıklayın köşe eklensin, köşeye tıklayın silinsin. Çıkmak için Esc",
        )
    }

    fn point_count(&self) -> usize {
        0
    }

    fn snaps(&self) -> bool {
        false
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let hover = edge::hover(p, cx, editable);
        self.action = hover
            .and_then(|h| cx.doc.get(h.slot))
            .map(|e| Self::plan(e, p, cx));
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let Some(slot) = edge::pick_said(p, cx, editable) else {
            cx.say(
                Level::Warn,
                "Düzenlenebilir bir çizgi, çoklu çizgi ya da kapalı alana tıklayın.",
            );
            return;
        };
        let (Some(e), Some(uid)) = (cx.doc.get(slot).cloned(), cx.doc.uid(slot)) else {
            return;
        };
        let s = shape(&e);
        if near_hole(&s, p.raw) {
            cx.say(
                Level::Warn,
                "İç halkanın köşeleri tutamaçla taşınır; köşe eklemek ya da silmek için alanı Patlat ile halkalarına ayırın.",
            );
            return;
        }
        let a = Self::plan(&e, p, cx);
        // The place the click names, by the core's places (docs/adr/0279): the vertex, or the edge's point.
        let place = match a.place {
            Some(place) => place,
            None => {
                let edges = entity_edges(&s);
                let Some(on) = edges.get(a.seg).map(|g| closest_on_edge(g, a.at).p) else {
                    return;
                };
                match place_at(&core_entity(&e), SnapKind::Nearest, on) {
                    Some(place) => contract_place(place),
                    None => return,
                }
            }
        };
        let action = if a.remove.is_some() {
            VertexAction::Remove
        } else {
            VertexAction::Insert
        };
        let input = EntitiesVertex {
            uid: uid.to_string(),
            action,
            place,
            through: None,
            radius: None,
            widths: None,
            topology: None,
            expected_revision: None,
        };
        let mut ecx = ExecutionContext::with_store(cx.doc, cx.spatial.store());
        let result = entities_vertex::execute(&mut ecx, input);
        if points::written(result, cx).is_some() {
            cx.say(
                Level::Success,
                if a.remove.is_some() {
                    "Köşe silindi."
                } else {
                    "Köşe eklendi."
                },
            );
        }
        self.action = None;
    }

    fn input(&mut self, _text: &str, _cx: &mut Context<'_>) -> bool {
        false
    }

    /// Enter starts the tool again on the web (it has no confirm): it stays.
    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        self.action = None;
        Flow::Stay
    }

    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }

    /// A red × on a vertex to remove, a + where one would be added, and what a click does.
    fn preview(&self, _format: &Format) -> Preview {
        let Some(a) = self.action else {
            return Preview::default();
        };
        let (shape, tone, text) = match a.remove {
            Some(_) => (MarkerShape::Cross(5.0), Tone::Danger, "Köşeyi sil"),
            None => (MarkerShape::Plus(6.0), Tone::Accent, "Köşe ekle"),
        };
        Preview {
            markers: vec![Marker {
                at: a.at,
                shape,
                tone,
            }],
            tag: Some(Tag {
                at: a.at,
                lines: vec![text.to_owned()],
            }),
            tag_tone: tone,
            ..Preview::default()
        }
    }
}
