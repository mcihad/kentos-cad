//! Daireyle seç (`tool.selectCircle`, docs/adr/0141): a circle is drawn and
//! the visible objects inside it are selected.
//!
//! - Click the centre, then show the radius with a click or type it (a
//!   number, or a point: the radius is its distance from the centre).
//! - The circle holds the objects wholly inside it. Kesişen (K) also takes
//!   those it touches: an edge inside it, or a closed object it lies within.
//!   The switch is remembered from run to run ([`crate::tool::Memory`]).
//! - What it finds replaces the selection, or joins it when Shift is held (at
//!   the click that shows the radius, or, for a typed radius, at the last
//!   click), and the tool goes back to Seç: `Dairenin içinde 4 nesne;
//!   seçildi.` / `Daireye dokunan 5 nesne; seçildi.` When it finds nothing the
//!   selection is left as it was, and the tool leaves all the same:
//!   `Dairede nesne yok.`
//! - The circle is dashed and lightly filled, in the window's colour for what
//!   lies wholly inside and in the snap colour for Kesişen (as the selection
//!   box is); beside the cursor, its radius: `R 8.000 m`. Esc goes back to the
//!   centre; from the centre it leaves.
//!
//! The query is the store's `in_circle`: only visible objects count.

use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geometry::dist;
use kentos_geometry_core::tools::point_text::js_trim;

use crate::Vec2;
use crate::edge::Outline;
use crate::format::Format;
use crate::log::Level;
use crate::points::{self, SAME};
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{
    Area, Context, Flow, Marker, MarkerShape, Memory, Pointer, Preview, Stroke, Tag, Tone, Tool,
};

/// The tool's id: its command is `tool.selectCircle`.
pub const ID: &str = "selectCircle";
pub const LABEL: &str = "Daireyle seç";

/// The circle's dash and gap, logical pixels.
const DASH: [f32; 2] = [5.0, 4.0];

/// Daireyle seç.
#[derive(Clone, Debug, Default)]
pub struct SelectCircle {
    centre: Option<Vec2>,
    /// The point under the pointer: the centre, or a point on the circle, as shown (and snapped).
    hover: Option<Vec2>,
    /// Shift was held at the last click: a typed radius then adds to the selection.
    shift: bool,
    /// What the session remembered, as of the last call (the prompt sees no context).
    memory: Memory,
    done: bool,
}

impl SelectCircle {
    pub fn new() -> Self {
        Self::default()
    }

    fn see(&mut self, cx: &Context<'_>) {
        self.memory = *cx.memory;
    }

    /// A point given: the centre, then a point on the circle.
    fn take(&mut self, p: Vec2, shift: bool, cx: &mut Context<'_>) {
        match self.centre {
            None => self.centre = Some(p),
            Some(c) => self.finish(c, dist(c, p), shift, cx),
        }
    }

    /// The circle is drawn: what it finds is selected.
    fn finish(&mut self, centre: Vec2, radius: f64, shift: bool, cx: &mut Context<'_>) {
        // Zero, negative and not a number are no radius.
        if radius.is_nan() || radius <= SAME {
            cx.say(Level::Warn, "Yarıçap sıfırdan büyük olmalı.");
            return;
        }
        let crossing = cx.memory.circle_crossing;
        let hits = cx.spatial.in_circle(centre, radius, crossing);
        let n = hits.len();
        if n == 0 {
            cx.say(Level::Warn, "Dairede nesne yok.");
        } else {
            cx.selection.take(hits, shift);
            let text = if crossing {
                format!("Daireye dokunan {n} nesne; seçildi.")
            } else {
                format!("Dairenin içinde {n} nesne; seçildi.")
            };
            cx.say(Level::Info, text);
        }
        self.done = true;
    }
}

impl Tool for SelectCircle {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn prompt(&self) -> Prompt {
        let step = if self.centre.is_none() {
            "dairenin merkezine tıklayın"
        } else {
            "yarıçapı gösterin ya da yazın"
        };
        Prompt::new(LABEL, step).toggle("Kesişen", "K", self.memory.circle_crossing)
    }

    fn point_count(&self) -> usize {
        usize::from(self.centre.is_some())
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.see(cx);
        Flow::Stay
    }

    /// A point computed by the point calculator, as if clicked.
    fn accepts_points(&self) -> bool {
        true
    }

    fn accept_point(&mut self, p: Vec2, cx: &mut Context<'_>) -> bool {
        let shift = cx.shift || self.shift;
        self.take(p, shift, cx);
        true
    }

    fn snap_from(&self) -> Option<Vec2> {
        self.centre
    }

    /// The points are taken as shown: ortho and polar tracking bend a direction,
    /// and a radius is a distance, which they would shorten.
    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.see(cx);
        self.hover = Some(p.world);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.see(cx);
        self.shift = p.shift;
        self.take(p.world, p.shift, cx);
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        self.see(cx);
        if upper_tr(js_trim(text)) == "K" {
            cx.memory.circle_crossing = !cx.memory.circle_crossing;
            self.see(cx);
            return true;
        }
        let shift = cx.shift || self.shift;
        // A number is the radius; anything else a point.
        if let (Some(c), Some(radius)) = (self.centre, points::plain_length(text, cx)) {
            self.finish(c, radius, shift, cx);
            return true;
        }
        match cx.typed_point(text, self.centre, self.hover) {
            Some(p) => {
                self.take(p, shift, cx);
                true
            }
            None => false,
        }
    }

    /// Enter or a quick right click leaves: there is nothing to finish before the circle is.
    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        Flow::Exit
    }

    /// Esc goes back to the centre; from the centre it leaves.
    fn cancel(&mut self, _cx: &mut Context<'_>) -> bool {
        self.centre.take().is_some()
    }

    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        self.centre.take().is_some()
    }

    fn finished(&self) -> bool {
        self.done
    }

    /// The dashed circle to the cursor, lightly filled, its radius beside it.
    fn preview(&self, format: &Format) -> Preview {
        let Some(c) = self.centre else {
            return Preview::default();
        };
        let mut preview = Preview {
            markers: vec![Marker {
                at: c,
                shape: MarkerShape::Ring(3.5),
                tone: Tone::Accent,
            }],
            ..Preview::default()
        };
        let Some(h) = self.hover.filter(|h| dist(c, *h) > SAME) else {
            return preview;
        };
        let r = dist(c, h);
        // The window's colour for what lies wholly inside, the snap colour for Kesişen.
        let tone = if self.memory.circle_crossing {
            Tone::Snap
        } else {
            Tone::Accent
        };
        let outline = Outline::of(&Shape::Circle { c, r }, Some(DASH), 1.5, tone);
        preview
            .areas
            .extend(outline.strokes.first().map(|ring| Area {
                rings: vec![ring.pts.clone()],
                fill: 0.1,
                width: 1.0,
                dash: Some(DASH),
                fill_tone: tone,
            }));
        preview.strokes.extend(outline.strokes);
        preview
            .strokes
            .push(Stroke::dashed(vec![c, h], false, [2.0, 3.0]));
        preview.tag = Some(Tag {
            at: h,
            lines: vec![format!("R {}", format.length(r))],
        });
        preview
    }
}
