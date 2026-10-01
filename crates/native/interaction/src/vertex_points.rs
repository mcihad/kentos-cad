//! Köşelere nokta (docs/adr/0152 §5; Netcad's Otomatik Nokta Üret, QGIS'
//! Extract vertices; the web's `VertexPointsTool`,
//! `apps/web/src/tools/vertexPointsTool.ts`): a named point at every vertex
//! of the selected lines, polylines and areas. The finding is the shared
//! core's (`vertex_points`): objects in the drawing's order, an area's outer
//! ring, its holes, then its other parts; a place two vertices share (1 µm)
//! one point, a place a point already holds passed over and counted. Names
//! run from Nokta's Ad by Yazı's Artır, the code is Nokta's Kod, the
//! elevation the vertex's (docs/adr/0142).
//!
//! Selection first, as the modify tools (`modify`); the points are shown
//! with their names, Ad (A) and Kod (K) are Nokta's (`Memory::point_name`,
//! `point_code`); Enter, Uygula or a quick right click writes them in one
//! step “Köşelere nokta” through `cad.entities.create` on the active layer,
//! and Nokta's Ad moves on past the last.

use std::collections::{BTreeMap, HashSet};

use kentos_contracts::{CreateOperation, EntitiesCreate, Entity, EntityGeometry, NewObject};
use kentos_domain::Slot;
use kentos_geometry_core::geom::affine::Affine;
use kentos_geometry_core::ops::vertex_points::{VertexPoints as Found, vertex_points};
use kentos_geometry_core::tools::point_text::js_trim;
use kentos_native_application::elevation::paths as elevated_paths;
use kentos_native_application::{ExecutionContext, create};

use crate::Vec2;
use crate::format::Format;
use crate::log::Level;
use crate::modify::{MAX_GHOSTS, Modify, Stages};
use crate::point::{Field, ask_field, take_field, with_options};
use crate::points::{self, wire};
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{
    Context, Flow, Label, Marker, MarkerShape, Memory, Name, Pointer, Preview, Tag, Tone,
};

pub const ID: &str = "vertexPoints";
pub const LABEL: &str = "Köşelere nokta";
/// What is said when the selection has no vertices to take.
const NOTHING: &str = "Köşelere nokta: seçimde çizgi, çoklu çizgi ya da alan yok.";

/// The kinds whose vertices it takes (docs/adr/0152 §5).
fn is_source(e: &Entity) -> bool {
    matches!(
        e,
        Entity::Line(_) | Entity::Polyline(_) | Entity::Polygon(_)
    )
}

/// What would be written for the drawing as it was, and what it was worked out from.
struct Plan {
    generation: u64,
    first: Name,
    found: Found,
}

#[derive(Default)]
pub struct VertexPoints {
    /// The objects it reads, taken when the selection is confirmed.
    slots: HashSet<Slot>,
    /// Ad or Kod: its value is being typed in the field.
    typing: Option<Field>,
    /// The cursor as it is (the web's `hover`): where the tag goes.
    hover: Option<Vec2>,
    plan: Option<Plan>,
    /// Nokta's values when last seen: what the prompt shows.
    seen: Memory,
}

impl VertexPoints {
    pub fn tool() -> Modify<Self> {
        Modify::with(Self::default())
    }

    /// Writes the points in one step and says what came of it; a refusal is
    /// said and the tool stays.
    fn write(&mut self, cx: &mut Context<'_>) -> Flow {
        self.see(cx);
        let Some(Plan { found, .. }) = &self.plan else {
            return Flow::Exit;
        };
        let Found {
            points,
            skipped,
            next,
        } = found;
        let (count, skipped) = (points.len(), *skipped);
        if points.is_empty() {
            let line = if skipped > 0 {
                format!("{LABEL}: yazılacak nokta yok; {skipped} köşede zaten nokta var.")
            } else {
                format!("{LABEL}: yazılacak nokta yok.")
            };
            cx.say(Level::Warn, line);
            return Flow::Exit;
        }
        let code = cx.memory.point_code.as_str().to_owned();
        let color = cx.draft.color.map(str::to_owned);
        let objects = points
            .iter()
            .map(|pt| NewObject {
                geometry: EntityGeometry::Point {
                    p: wire(pt.p),
                    z: pt.z,
                },
                color: color.clone(),
                line_weight: None,
                attrs: (!code.is_empty())
                    .then(|| BTreeMap::from([("Kod".to_owned(), code.clone())])),
                label: pt.name.clone(),
            })
            .collect();
        let names = match (&points[0].name, &points[count - 1].name) {
            (Some(first), Some(last)) if first == last => format!(" ({first})"),
            (Some(first), Some(last)) => format!(" ({first} – {last})"),
            _ => String::new(),
        };
        let next = next.as_deref().and_then(Name::new);
        let input = EntitiesCreate {
            layer_id: cx.doc.layers().active().to_owned(),
            objects,
            operation: Some(CreateOperation::VertexPoints),
            expected_revision: None,
        };
        let result = create::execute(&mut ExecutionContext::new(cx.doc), input);
        if points::written(result, cx).is_none() {
            return Flow::Stay;
        }
        // Nokta goes on past the last name (docs/adr/0152 §5).
        if !cx.memory.point_name.as_str().is_empty()
            && let Some(next) = next
        {
            cx.memory.point_name = next;
        }
        let past = if skipped > 0 {
            format!("; {skipped} köşede zaten nokta vardı")
        } else {
            String::new()
        };
        cx.say(
            Level::Success,
            format!("{LABEL}: {count} nokta eklendi{names}{past}."),
        );
        Flow::Exit
    }
}

impl Stages for VertexPoints {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    /// Nokta's values, and the points for the drawing as it is: worked out
    /// again when the drawing or the first name changed.
    fn see(&mut self, cx: &Context<'_>) {
        self.seen = *cx.memory;
        if self.slots.is_empty() {
            return;
        }
        let (generation, first) = (cx.doc.generation(), cx.memory.point_name);
        if self
            .plan
            .as_ref()
            .is_some_and(|p| p.generation == generation && p.first == first)
        {
            return;
        }
        // In the drawing's order: the objects' vertices, and every point's
        // place (hidden layers too: a place is taken).
        let mut objects = Vec::new();
        let mut existing = Vec::new();
        for e in cx.doc.entities() {
            match e {
                Entity::Point(q) => existing.push(Vec2::new(q.p.x, q.p.y)),
                _ if self.slots.contains(&Slot(e.base().id)) => objects.push(elevated_paths(e)),
                _ => {}
            }
        }
        let name = first.as_str();
        let found = vertex_points(&objects, &existing, (!name.is_empty()).then_some(name));
        self.plan = Some(Plan {
            generation,
            first,
            found,
        });
    }

    fn begin(&mut self, cx: &mut Context<'_>) -> Flow {
        self.typing = None;
        self.plan = None;
        self.slots = cx
            .selection
            .ids()
            .iter()
            .copied()
            .filter(|&s| cx.doc.get(s).is_some_and(is_source))
            .collect();
        if self.slots.is_empty() {
            cx.say(Level::Warn, NOTHING);
            return Flow::Exit;
        }
        Flow::Stay
    }

    /// Nothing is placed by the cursor.
    fn snaps(&self) -> bool {
        false
    }

    fn anchor(&self) -> Option<Vec2> {
        None
    }

    fn prompt(&self, _n: usize) -> Prompt {
        if self.typing.is_some() {
            return Prompt::new(LABEL, "değeri yazın");
        }
        let step = self
            .plan
            .as_ref()
            .map_or_else(|| "yazılacak nokta yok".to_owned(), |p| finding(&p.found));
        with_options(Prompt::new(LABEL, step), &self.seen).option("Uygula", "Enter")
    }

    /// The cursor as it is; a click places nothing (Enter, Uygula or a
    /// quick right click writes).
    fn pointer(&mut self, p: &Pointer, down: bool, _cx: &mut Context<'_>) -> Option<Flow> {
        if down {
            return Some(Flow::Stay);
        }
        self.hover = Some(p.raw);
        None
    }

    fn point(&mut self, _p: Vec2, _cx: &mut Context<'_>) -> Flow {
        Flow::Stay
    }

    /// A and K ask Nokta's Ad and Kod in the text field.
    fn typed(&mut self, text: &str, cx: &mut Context<'_>) -> Option<Flow> {
        if self.typing.is_some() {
            return None;
        }
        let field = Field::of_key(&upper_tr(js_trim(text)))?;
        self.typing = Some(field);
        let ask = ask_field(field, self.hover, cx);
        cx.view_changes.push(ask);
        Some(Flow::Stay)
    }

    fn text_typed(&mut self, text: Option<&str>, cx: &mut Context<'_>) {
        if let Some(field) = self.typing.take() {
            take_field(field, text, cx);
        }
    }

    /// Nothing typed is a point.
    fn typed_points(&self) -> bool {
        false
    }

    /// Neither the point calculator's points nor `#ad`: nothing is placed.
    fn takes_points(&self) -> bool {
        false
    }

    /// Enter: the points written in one step, and the tool leaves (the
    /// field answers its own Enter).
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if self.typing.is_some() {
            return Flow::Stay;
        }
        self.write(cx)
    }

    fn preview(&self, _hover: Vec2) -> Option<Affine> {
        None
    }

    /// The points to come with their names (as the modify tools' ghosts, a
    /// very large selection shows its first ones), the finding beside the cursor.
    fn stage_preview(&self, hover: Option<Vec2>, _format: &Format) -> Option<Preview> {
        let plan = self.plan.as_ref()?;
        let shown = plan.found.points.iter().take(MAX_GHOSTS);
        let markers = shown
            .clone()
            .map(|pt| Marker {
                at: pt.p,
                // Round the selection's grip at the corner: 6 px clears its square.
                shape: MarkerShape::Ring(6.0),
                tone: Tone::Accent,
            })
            .collect();
        let labels = shown
            .filter_map(|pt| {
                Some(Label {
                    at: pt.p,
                    text: pt.name.clone()?,
                    offset: [7.0, -6.0],
                    tone: Tone::Accent,
                })
            })
            .collect();
        let tag = hover.map(|at| Tag {
            at,
            lines: tag_lines(&plan.found),
        });
        Some(Preview {
            markers,
            labels,
            tag,
            ..Preview::default()
        })
    }
}

/// `12 nokta; 2 köşede zaten nokta var`: what would be written, then what is passed over.
fn finding(found: &Found) -> String {
    let (n, skipped) = (found.points.len(), found.skipped);
    match (n, skipped) {
        (0, 0) => "yazılacak nokta yok".to_owned(),
        (0, s) => format!("yazılacak nokta yok; {s} köşede zaten nokta var"),
        (n, 0) => format!("{n} nokta"),
        (n, s) => format!("{n} nokta; {s} köşede zaten nokta var"),
    }
}

/// The finding beside the cursor, a part a line, and what writes it.
fn tag_lines(found: &Found) -> Vec<String> {
    let mut lines: Vec<String> = finding(found).split("; ").map(str::to_owned).collect();
    if !found.points.is_empty() {
        lines.push("Enter: uygula".to_owned());
    }
    lines
}
