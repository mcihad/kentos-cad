//! Ötele: the web's `OffsetTool` (`apps/web/src/tools/edgeTools.ts`) on its
//! edge-picking base, step for step (docs/adr/0047):
//!
//! - click the object to offset (hovered by its edge as the pointer moves);
//! - then the side the copy goes to, at the remembered distance, or with
//!   Noktadan geç (N) the point it passes through (the distance is then the
//!   point's to the object, and object snaps apply);
//! - a typed number sets the distance (and turns Noktadan geç off); the
//!   distance and the options stay for as long as the app lives
//!   ([`crate::tool::Memory`]);
//! - İki yana (I) makes two copies, one on each side of the object, at the
//!   same distance; Kaynağı sil (S) deletes the object in the same step
//!   (docs/adr/0140);
//! - Enter or Esc drops the picked object; with none, Enter leaves.
//!
//! The copy is a new object from the picked one (its layer and colour),
//! written through `cad.entities.edit`: “1.000 m ötelenmiş kopya eklendi.”
//! One undo step, copies and deletion together. The copy and the distance
//! are the shared core's (`offset_entity`, `through_distance`).

use kentos_contracts::{EditOperation, EntityEdit};
use kentos_domain::Slot;
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::intersect::closest_on_edge;
use kentos_geometry_core::ops::curve_cuts::Geometry;
use kentos_geometry_core::ops::edges::entity_edges;
use kentos_geometry_core::ops::offset::{offset_entity, through_distance};
use kentos_geometry_core::tools::point_text::js_trim;
use kentos_native_application::geometry::shape;

use crate::Vec2;
use crate::edge::{self, Outline};
use crate::format::Format;
use crate::log::Level;
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Cursor, Flow, Memory, Pointer, Preview, Tag, Tone, Tool};

/// The offset tool's id: its command is `tool.offset`.
pub const ID: &str = "offset";
pub const LABEL: &str = "Ötele";

/// How far past the object the other side's point is put, metres: enough to
/// be on the far side, close enough to be judged at the same edge.
const OTHER_SIDE: f64 = 1e-3;

/// A point just across `target` from `side`: where İki yana's second copy is
/// judged from. `None` when `side` is on the object (there is no far side).
fn across(target: &Shape, side: Vec2) -> Option<Vec2> {
    let nearest = entity_edges(target)
        .iter()
        .map(|edge| closest_on_edge(edge, side))
        .min_by(|a, b| a.d.total_cmp(&b.d))?;
    if nearest.d < 1e-9 {
        return None;
    }
    let (ux, uy) = (
        (nearest.p.x - side.x) / nearest.d,
        (nearest.p.y - side.y) / nearest.d,
    );
    Some(Vec2::new(
        nearest.p.x + ux * OTHER_SIDE,
        nearest.p.y + uy * OTHER_SIDE,
    ))
}

/// The offset tool.
#[derive(Clone, Debug, Default)]
pub struct Offset {
    /// The object picked to offset.
    target: Option<Slot>,
    /// Where the copy goes: the pointer, snapped with Noktadan geç.
    side: Option<Vec2>,
    /// The preview, made when the pointer moves: `preview` sees no drawing.
    drawn: Preview,
    /// What the session remembered and the project's units, as of the last call.
    seen: Option<(Memory, Format)>,
}

impl Offset {
    pub fn new() -> Self {
        Self::default()
    }

    fn see(&mut self, cx: &Context<'_>) {
        self.seen = Some((*cx.memory, cx.format()));
    }

    fn memory(&self) -> Memory {
        self.seen.map(|(m, _)| m).unwrap_or_default()
    }

    /// The distance for a side point: the remembered one, or with Noktadan
    /// geç the point's distance to the object (the core's).
    fn distance_for(&self, target: &kentos_geometry_core::entity::Shape, p: Vec2) -> f64 {
        let m = self.memory();
        if m.offset_through {
            through_distance(target, p)
        } else {
            m.offset_distance
        }
    }

    /// The copies `d` from `target` on the side of `side`, and with İki yana
    /// on the other side too: each the copy or why it is not made.
    fn copies(&self, target: &Shape, d: f64, side: Vec2) -> Vec<Geometry> {
        let mut out = vec![offset_entity(target, d, side)];
        if self.memory().offset_both {
            match across(target, side) {
                Some(other) => out.push(offset_entity(target, d, other)),
                None => out.push(Geometry::Error(
                    "Nokta nesnenin üzerinde; iki yana ötelemek için bir yanına tıklayın.".into(),
                )),
            }
        }
        out
    }

    /// The copies at `side` dashed, the object solid, the distance beside the cursor.
    fn redraw(&mut self, cx: &Context<'_>) {
        self.drawn = Preview::default();
        let (Some(slot), Some(side)) = (self.target, self.side) else {
            return;
        };
        let Some(target) = cx.doc.get(slot).map(shape) else {
            return;
        };
        let d = self.distance_for(&target, side);
        let mut out = Outline::default();
        for copy in self.copies(&target, d, side) {
            if let Geometry::Ok(copy) = copy {
                out = out.and(Outline::of(
                    &copy.shape,
                    Some([4.0, 3.0]),
                    1.0,
                    Tone::Accent,
                ));
            }
        }
        let out = out.and(Outline::of(&target, None, 1.0, Tone::Accent));
        let mut lines = vec![format!("Mesafe {}", cx.format().length(d))];
        if self.memory().offset_both {
            lines.push("İki yana".to_owned());
        }
        if self.memory().offset_erase {
            lines.push("Kaynak silinir".to_owned());
        }
        self.drawn = Preview {
            strokes: out.strokes,
            marks: out.marks,
            tag: Some(Tag { at: side, lines }),
            ..Preview::default()
        };
    }

    /// Writes the copies as one edit, and with Kaynağı sil the deletion of the
    /// object too. A copy that does not come out is said; with none, nothing is written.
    fn write(&mut self, slot: Slot, target: &Shape, d: f64, side: Vec2, cx: &mut Context<'_>) {
        let mut shapes = Vec::new();
        for copy in self.copies(target, d, side) {
            match copy {
                Geometry::Ok(copy) => shapes.push(copy.shape),
                Geometry::Error(e) => cx.say(Level::Warn, e),
            }
        }
        // A new object from the picked one: its layer and colour (docs/adr/0047).
        let from = edge::uid(cx.doc, slot);
        let mut changes: Vec<EntityEdit> = shapes
            .iter()
            .filter_map(edge::geometry)
            .map(|geometry| EntityEdit::add(from.clone(), geometry, None))
            .collect();
        let copies = changes.len();
        if copies == 0 {
            return;
        }
        let erase = self.memory().offset_erase;
        if erase {
            changes.push(EntityEdit::Remove { uid: from });
        }
        if edge::write(EditOperation::Offset, changes, cx).is_none() {
            return;
        }
        cx.memory.offset_distance = d;
        let length = cx.format().length(d);
        let made = if copies == 2 {
            format!("{length} iki yana ötelenmiş 2 kopya eklendi")
        } else {
            format!("{length} ötelenmiş kopya eklendi")
        };
        let text = if erase {
            format!("{made}; kaynak silindi.")
        } else {
            format!("{made}.")
        };
        cx.say(Level::Success, text);
        if erase {
            let doc = &*cx.doc;
            cx.selection.retain(|s| doc.get(s).is_some());
            cx.selection.set_hover(None);
        }
    }

    fn drop_target(&mut self) {
        self.target = None;
        self.side = None;
        self.drawn = Preview::default();
    }
}

impl Tool for Offset {
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
        let m = self.memory();
        // The toggles say their state only when it is on: the prompt has to fit a narrow
        // window with all three of them, and the distance is by the cursor (docs/adr/0140).
        let toggle = |p: Prompt, label: &'static str, key: &'static str, yes: bool| {
            if yes {
                p.option_with(label, key, "açık")
            } else {
                p.option(label, key)
            }
        };
        let toggles = |p: Prompt| {
            let p = toggle(p, "Noktadan geç", "N", m.offset_through);
            let p = toggle(p, "İki yana", "I", m.offset_both);
            toggle(p, "Kaynağı sil", "S", m.offset_erase)
        };
        match self.target {
            None => toggles(Prompt::new(LABEL, "ötelenecek nesneye tıklayın")),
            Some(_) => toggles(Prompt::new(
                LABEL,
                if m.offset_through {
                    "kopyanın geçeceği noktaya tıklayın"
                } else {
                    "kopyanın gideceği tarafa tıklayın"
                },
            )),
        }
    }

    fn point_count(&self) -> usize {
        0
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.see(cx);
        Flow::Stay
    }

    /// Object snaps apply to the point the copy passes through.
    fn snaps(&self) -> bool {
        self.memory().offset_through && self.target.is_some()
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.see(cx);
        if self.target.is_some() {
            self.side = Some(p.world);
            return self.redraw(cx);
        }
        // Picking: the distance a copy would go, by the cursor over an object.
        let hit = edge::hover(p, cx, edge::unlocked);
        self.drawn = match hit {
            Some(_) if !self.memory().offset_through => Preview {
                tag: Some(Tag {
                    at: p.raw,
                    lines: vec![format!(
                        "Mesafe {}",
                        cx.format().length(self.memory().offset_distance)
                    )],
                }),
                ..Preview::default()
            },
            _ => Preview::default(),
        };
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.see(cx);
        let Some(slot) = self.target else {
            let Some(slot) = edge::pick(p, cx, edge::unlocked) else {
                cx.say(
                    Level::Warn,
                    "Düzenlenebilir bir çizgi, çoklu çizgi, daire ya da yaya tıklayın.",
                );
                return;
            };
            self.target = Some(slot);
            self.side = Some(p.raw);
            cx.selection.set_hover(None);
            return self.redraw(cx);
        };
        let Some(target) = cx.doc.get(slot).map(shape) else {
            return self.drop_target();
        };
        let d = self.distance_for(&target, p.world);
        self.write(slot, &target, d, p.world, cx);
        self.drop_target();
        self.see(cx);
    }

    /// N turns Noktadan geç over, I İki yana and S Kaynağı sil; a number
    /// above zero is the distance (and turns Noktadan geç off).
    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        match upper_tr(js_trim(text)).as_str() {
            "N" => cx.memory.offset_through = !cx.memory.offset_through,
            "I" => cx.memory.offset_both = !cx.memory.offset_both,
            "S" => cx.memory.offset_erase = !cx.memory.offset_erase,
            // Typed in the project's unit (docs/adr/0165 §2).
            _ => match cx.typed_length(text) {
                Some(n) if n > 0.0 => {
                    cx.memory.offset_distance = n;
                    cx.memory.offset_through = false;
                }
                _ => return false,
            },
        }
        self.see(cx);
        self.redraw(cx);
        true
    }

    /// Enter drops the picked object; with none, the tool leaves.
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        self.see(cx);
        if self.target.is_some() {
            self.drop_target();
            return Flow::Stay;
        }
        Flow::Exit
    }

    fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
        self.see(cx);
        if self.target.is_none() {
            return false;
        }
        self.drop_target();
        true
    }

    /// Ctrl+Z undoes the drawing: the web's edge tools take no step back.
    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }

    fn preview(&self, _format: &Format) -> Preview {
        self.drawn.clone()
    }
}
