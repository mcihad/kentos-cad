//! Ötele: the web's `OffsetTool` (`apps/web/src/tools/edgeTools.ts`) on its
//! edge-picking base, step for step (docs/adr/0047):
//!
//! - click the object to offset (hovered by its edge as the pointer moves);
//! - then the side the copy goes to, at the remembered distance, or with
//!   Noktadan geç (N) the point it passes through (the distance is then the
//!   point's to the object, and object snaps apply);
//! - a typed number sets the distance (and turns Noktadan geç off); the
//!   distance and the option stay for as long as the app lives
//!   ([`crate::tool::Memory`]);
//! - Enter or Esc drops the picked object; with none, Enter leaves.
//!
//! The copy is a new object from the picked one (its layer and colour),
//! written through `cad.entities.edit`: “1.000 m ötelenmiş kopya eklendi.”
//! The copy and the distance are the shared core's (`offset_entity`,
//! `through_distance`).

use kentos_contracts::{EditOperation, EntityEdit};
use kentos_domain::Slot;
use kentos_geometry_core::ops::curve_cuts::Geometry;
use kentos_geometry_core::ops::offset::{offset_entity, through_distance};
use kentos_geometry_core::tools::point_text::{js_trim, parse_number};
use kentos_native_application::geometry::shape;

use crate::Vec2;
use crate::edge::{self, Outline};
use crate::format::Format;
use crate::log::Level;
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Flow, Memory, Pointer, Preview, Tag, Tone, Tool};

/// The offset tool's id: its command is `tool.offset`.
pub const ID: &str = "offset";
pub const LABEL: &str = "Ötele";

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

    /// The copy at `side` dashed, the object solid, the distance beside the cursor.
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
        if let Geometry::Ok(copy) = offset_entity(&target, d, side) {
            out = Outline::of(&copy.shape, Some([4.0, 3.0]), 1.0, Tone::Accent);
        }
        let out = out.and(Outline::of(&target, None, 1.0, Tone::Accent));
        self.drawn = Preview {
            strokes: out.strokes,
            marks: out.marks,
            tag: Some(Tag {
                at: side,
                lines: vec![format!("Mesafe {}", cx.format().length(d))],
            }),
            ..Preview::default()
        };
    }

    fn drop_target(&mut self) {
        self.target = None;
        self.side = None;
        self.drawn = Preview::default();
    }
}

impl Tool for Offset {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn prompt(&self) -> Prompt {
        let m = self.memory();
        let through = if m.offset_through {
            "açık"
        } else {
            "kapalı"
        };
        match self.target {
            None => {
                let p = Prompt::new(LABEL, "ötelenecek nesneye tıklayın");
                let p = if m.offset_through {
                    p
                } else {
                    let format = self.seen.map(|(_, f)| f).unwrap_or_default();
                    p.note(format!("mesafe {}", format.length(m.offset_distance)))
                        .then()
                        .note("mesafe için sayı yazın")
                        .then()
                };
                p.option_with("Noktadan geç", "N", through)
            }
            Some(_) => Prompt::new(
                LABEL,
                if m.offset_through {
                    "kopyanın geçeceği noktaya tıklayın"
                } else {
                    "kopyanın gideceği tarafa tıklayın"
                },
            )
            .option_with("Noktadan geç", "N", through),
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
        edge::hover(p, cx, edge::unlocked);
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
        match offset_entity(&target, d, p.world) {
            Geometry::Error(e) => cx.say(Level::Warn, e),
            Geometry::Ok(copy) => {
                cx.memory.offset_distance = d;
                // A new object from the picked one: its layer and colour (docs/adr/0047).
                let from = edge::uid(cx.doc, slot);
                let written = edge::geometry(&copy.shape).and_then(|geometry| {
                    edge::write(
                        EditOperation::Offset,
                        vec![EntityEdit::Add {
                            from,
                            geometry,
                            keep_data: None,
                        }],
                        cx,
                    )
                });
                if written.is_some() {
                    let text = format!("{} ötelenmiş kopya eklendi.", cx.format().length(d));
                    cx.say(Level::Success, text);
                }
            }
        }
        self.drop_target();
        self.see(cx);
    }

    /// N turns Noktadan geç over; a number above zero is the distance (and turns it off).
    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        if upper_tr(js_trim(text)) == "N" {
            cx.memory.offset_through = !cx.memory.offset_through;
        } else {
            match parse_number(text) {
                Some(n) if n > 0.0 => {
                    cx.memory.offset_distance = n;
                    cx.memory.offset_through = false;
                }
                _ => return false,
            }
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
