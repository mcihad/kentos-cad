//! Paralel kaydır (docs/adr/0191; Netcad's Paralel Kaydır and Alan
//! Düzeltme (Paralel); the web's `EdgeShiftTool`,
//! `apps/web/src/tools/edgeShiftTool.ts`): an area's or a polyline's
//! straight edge moved parallel to itself, its neighbours lengthened or
//! shortened to meet it (the core's `ops::edge_shift`). The edge is clicked;
//! it follows the cursor (the parallel line through the cursor's point,
//! snaps on), the distance and an area's new size beside it. A click there,
//! a typed distance, or Alan (A) and a target size writes it through
//! `cad.entities.edit` (step “Paralel kaydır”) and the tool waits for the
//! next edge. Esc and Ctrl+Z let the edge go; Esc with none leaves.

use kentos_contracts::{EditOperation, Entity, EntityEdit};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::ops::edge_shift::{self as rules, Picked};
use kentos_geometry_core::tools::point_text::{js_trim, parse_number};
use kentos_native_application::geometry::shape;

use crate::Vec2;
use crate::edge::{self, Outline};
use crate::format::Format;
use crate::log::Level;
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Cursor, Flow, Pointer, Preview, Stroke, Tag, Tone, Tool};

/// The tool's id: its command is `tool.edgeShift`.
pub const ID: &str = "edgeShift";
pub const LABEL: &str = "Paralel kaydır";
/// A click where no area's or polyline's edge is.
pub const NO_EDGE_HERE: &str = "Tıklanan yerde kaydırılacak kenar yok; kilitsiz bir alanın ya da çoklu çizginin kenarına tıklayın.";

/// The edge picked: its object, the object's shape then and how it moves.
#[derive(Clone, Debug)]
struct Picking {
    slot: Slot,
    shape: Shape,
    picked: Picked,
    /// The area's size then (none: a polyline).
    area: Option<f64>,
}

pub struct EdgeShift {
    edge: Option<Picking>,
    asking: bool,
    /// The cursor's distance and the shape moved there, or why not.
    at: Option<(Vec2, f64, Result<Shape, String>)>,
    format: Format,
}

impl Default for EdgeShift {
    fn default() -> Self {
        Self::new()
    }
}

/// An area or a polyline, not on a locked layer: what the tool shifts.
fn editable(e: &Entity, doc: &Document) -> bool {
    matches!(e, Entity::Polygon(_) | Entity::Polyline(_)) && edge::unlocked(e, doc)
}

/// A signed amount in the project's units, its sign always shown.
fn signed(text: String, v: f64) -> String {
    if v > 0.0 { format!("+{text}") } else { text }
}

impl EdgeShift {
    pub fn new() -> Self {
        Self {
            edge: None,
            asking: false,
            at: None,
            format: Format::default(),
        }
    }

    /// The edge under the click, or why none is taken.
    fn pick(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let Some(slot) = edge::pick(p, cx, editable) else {
            cx.say(Level::Warn, NO_EDGE_HERE);
            return;
        };
        let Some(e) = cx.doc.get(slot) else {
            return;
        };
        let geom = shape(e);
        match rules::pick(&geom, p.raw) {
            Ok(picked) => {
                self.edge = Some(Picking {
                    slot,
                    area: rules::area_of(&geom),
                    shape: geom,
                    picked,
                });
                cx.selection.set_hover(None);
            }
            Err(why) => cx.say(Level::Warn, why),
        }
    }

    /// The point's distance from the picked edge along its normal; none
    /// with no edge picked.
    fn distance_at(&self, p: Vec2) -> Option<f64> {
        let k = &self.edge.as_ref()?.picked;
        Some((p.x - k.a.x) * k.normal.x + (p.y - k.a.y) * k.normal.y)
    }

    /// Writes the edge moved `d` and says so; the tool waits for the next
    /// edge. Whether it was written.
    fn write(&mut self, d: f64, cx: &mut Context<'_>) -> bool {
        let Some(picking) = &self.edge else {
            return false;
        };
        let k = &picking.picked;
        let moved = match rules::shifted(&picking.shape, k.ring, k.edge, d) {
            Ok(moved) => moved,
            Err(why) => {
                cx.say(Level::Warn, why);
                return false;
            }
        };
        let Some(geometry) = edge::geometry(&moved) else {
            return false;
        };
        let uid = edge::uid(cx.doc, picking.slot);
        let change = EntityEdit::Update { uid, geometry };
        if edge::write(EditOperation::EdgeShift, vec![change], cx).is_none() {
            return false;
        }
        let format = cx.format();
        let said = match rules::area_of(&moved) {
            Some(area) => format!(
                "{LABEL}: kenar {} kaydırıldı, alan {}.",
                format.length(d),
                format.area(area)
            ),
            None => format!("{LABEL}: kenar {} kaydırıldı.", format.length(d)),
        };
        cx.say(Level::Success, said);
        self.edge = None;
        self.at = None;
        true
    }
}

impl Tool for EdgeShift {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn prompt(&self) -> Prompt {
        if self.asking {
            return Prompt::new(
                LABEL,
                format!("hedef alanı yazın ({})", self.format.area_unit_label()),
            );
        }
        match &self.edge {
            None => Prompt::new(LABEL, "kaydırılacak kenara tıklayın (alan ya da çoklu çizgi)"),
            Some(picking) => {
                let step = if picking.area.is_some() {
                    "uzaklığı yazın ya da yerine tıklayın (dışarı artı)"
                } else {
                    "uzaklığı yazın ya da yerine tıklayın (sağa artı)"
                };
                let prompt = Prompt::new(LABEL, step);
                if picking.area.is_some() {
                    prompt.option("Alan", "A")
                } else {
                    prompt
                }
            }
        }
    }

    fn point_count(&self) -> usize {
        usize::from(self.edge.is_some())
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.format = cx.format();
        Flow::Stay
    }

    /// The cursor snaps once the edge is picked: the edge goes through the point.
    fn snaps(&self) -> bool {
        self.edge.is_some()
    }

    fn cursor(&self) -> Cursor {
        if self.edge.is_some() {
            Cursor::Cross
        } else {
            Cursor::Pick
        }
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.format = cx.format();
        let Some(d) = self.distance_at(p.world) else {
            edge::hover(p, cx, editable);
            return;
        };
        let moved = self.edge.as_ref().map(|picking| {
            let k = &picking.picked;
            rules::shifted(&picking.shape, k.ring, k.edge, d)
        });
        self.at = moved.map(|moved| (p.world, d, moved));
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if self.asking {
            return;
        }
        match self.distance_at(p.world) {
            None => self.pick(p, cx),
            Some(d) => {
                self.write(d, cx);
            }
        }
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let t = js_trim(text);
        let Some((shape, ring, index, has_area)) = self.edge.as_ref().map(|picking| {
            let k = &picking.picked;
            (picking.shape.clone(), k.ring, k.edge, picking.area.is_some())
        }) else {
            return false;
        };
        if self.asking {
            let format = cx.format();
            match parse_number(t) {
                Some(n) if n > 0.0 && n.is_finite() => {
                    let target = format.area_to_square_metres(n);
                    match rules::for_area(&shape, ring, index, target) {
                        Ok(d) => {
                            self.asking = false;
                            self.write(d, cx);
                        }
                        Err(why) => cx.say(Level::Warn, why),
                    }
                }
                _ => cx.say(
                    Level::Warn,
                    format!("Hedef alan sıfırdan büyük bir sayı olmalı; “{t}” yazıldı."),
                ),
            }
            return true;
        }
        if upper_tr(t) == "A" {
            if has_area {
                self.asking = true;
            } else {
                cx.say(Level::Warn, rules::NOT_AREA);
            }
            return true;
        }
        match cx.typed_length(t) {
            Some(d) if d.is_finite() => {
                self.write(d, cx);
                true
            }
            _ => false,
        }
    }

    /// Enter: Alan's question ends; else the tool leaves (a click or a typed
    /// distance writes).
    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        if std::mem::take(&mut self.asking) {
            return Flow::Stay;
        }
        Flow::Exit
    }

    /// Esc: Alan's question, then the edge go first.
    fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
        if std::mem::take(&mut self.asking) {
            return true;
        }
        self.undo_step(cx)
    }

    /// Ctrl+Z: the edge goes; with none the drawing is undone.
    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        self.asking = false;
        self.at = None;
        self.edge.take().is_some()
    }

    fn preview(&self, format: &Format) -> Preview {
        let mut out = Preview::default();
        let Some(picking) = &self.edge else {
            return out;
        };
        let k = &picking.picked;
        // The edge as it is, then the shape moved to the cursor.
        out.strokes
            .push(Stroke::solid(vec![k.a, k.b], false).width(2.0).tone(Tone::Snap));
        let Some((at, d, moved)) = &self.at else {
            return out;
        };
        let mut lines = vec![format.length(*d)];
        match moved {
            Ok(moved) => {
                let outline = Outline::of(moved, Some([6.0, 4.0]), 1.0, Tone::Accent);
                out.strokes.extend(outline.strokes);
                if let (Some(before), Some(after)) = (picking.area, rules::area_of(moved)) {
                    lines.push(format!(
                        "{} ({})",
                        format.area(after),
                        signed(format.area_bare(after - before), after - before)
                    ));
                }
            }
            Err(why) => {
                lines.push(why.clone());
                out.tag_tone = Tone::Danger;
            }
        }
        out.tag = Some(Tag { at: *at, lines });
        out
    }
}
