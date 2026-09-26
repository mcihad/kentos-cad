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
//! The object keeps its place and persistent id, through `cad.entities.edit`
//! (“Köşe eklendi.”, “Köşe silindi.”); a closed area keeps its holes. The
//! new geometry, the segment clicked and the hole check are the shared
//! core's (`insert_vertex`, `remove_vertex`, `nearest_segment`, `near_hole`).

use kentos_contracts::{EditOperation, Entity, EntityEdit};
use kentos_domain::Document;
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geometry::dist;
use kentos_geometry_core::ops::curve_cuts::Geometry;
use kentos_geometry_core::ops::vertex::{insert_vertex, near_hole, nearest_segment, remove_vertex};
use kentos_native_application::geometry::shape;

use crate::Vec2;
use crate::edge;
use crate::format::Format;
use crate::log::Level;
use crate::prompt::Prompt;
use crate::tool::{Context, Flow, Marker, MarkerShape, Pointer, Preview, Tag, Tone, Tool};

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
        if let Entity::Polyline(path) | Entity::Polygon(path) = e
            && let Some(i) = path
                .pts
                .iter()
                .position(|q| dist(Vec2::new(q.x, q.y), p.raw) <= tol)
        {
            let q = path.pts[i];
            return Action {
                remove: Some(i),
                seg: 0,
                at: Vec2::new(q.x, q.y),
            };
        }
        let seg = match e {
            Entity::Line(_) => 0,
            _ => nearest_segment(&shape(e), p.raw),
        };
        Action {
            remove: None,
            seg,
            at: p.raw,
        }
    }
}

impl Tool for Vertex {
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
        let Some(slot) = edge::pick(p, cx, editable) else {
            cx.say(
                Level::Warn,
                "Düzenlenebilir bir çizgi, çoklu çizgi ya da kapalı alana tıklayın.",
            );
            return;
        };
        let Some(e) = cx.doc.get(slot).cloned() else {
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
        let result = match a.remove {
            Some(i) => Ok(remove_vertex(&s, i)),
            None => insert_vertex(&s, a.seg, a.at),
        };
        let g = match result {
            Ok(Geometry::Ok(g)) => g.shape,
            Ok(Geometry::Error(error)) | Err(error) => return cx.say(Level::Warn, error),
        };
        let operation = if a.remove.is_some() {
            EditOperation::VertexRemove
        } else {
            EditOperation::VertexAdd
        };
        let same_kind = std::mem::discriminant(&g) == std::mem::discriminant(&s);
        let written = if same_kind {
            // The whole geometry is written (docs/adr/0047): a closed area keeps its holes.
            let g = match (g, &s) {
                (Shape::Polygon { pts, bulges, .. }, Shape::Polygon { holes, .. }) => {
                    Shape::Polygon {
                        pts,
                        bulges,
                        holes: holes.clone(),
                    }
                }
                (g, _) => g,
            };
            let uid = edge::uid(cx.doc, slot);
            edge::geometry(&g)
                .and_then(|geometry| {
                    edge::write(operation, vec![EntityEdit::Update { uid, geometry }], cx)
                })
                .is_some()
        } else {
            edge::replace(operation, slot, &[g], cx)
        };
        if written {
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
