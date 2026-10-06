//! Çokgenle seç (`tool.selectPolygon`, docs/adr/0187 §2): a polygon is drawn
//! and the visible objects inside it, touching it or outside it are
//! selected. The web's `PolygonSelectTool` (`apps/web/src/tools/selectTools.ts`).
//!
//! - Click the polygon's corners (snaps, ortho and polar tracking apply from
//!   the last); Geri (G) drops the last one. A right click or Enter ends.
//! - İçindekiler (İ) takes what lies wholly inside, Kesişenler (K) what it
//!   touches too, Dışındakiler (D) every visible object it does not touch;
//!   the mode is remembered from run to run ([`crate::tool::Memory`]). The
//!   query is the store's `in_polygon`, the selection filter passes what it
//!   holds (docs/adr/0187 §5).
//! - A polygon of fewer than three corners, with no area, or crossing itself
//!   is said (the core's words, `ring_problem`) and the tool waits.
//! - What it finds replaces the selection, or joins it when Shift is held (at
//!   the last pointer event, or as Enter is pressed), and the tool goes back
//!   to Seç. When it finds nothing the selection is left as it was, and the
//!   tool leaves all the same.
//! - Esc drops the polygon drawn so far; with none it leaves.

use kentos_geometry_core::geometry::dist;
use kentos_geometry_core::store::polygon::{PolygonMode, ring_problem};
use kentos_geometry_core::tools::point_input::Tracking;
use kentos_geometry_core::tools::point_text::js_trim;

use crate::Vec2;
use crate::format::Format;
use crate::log::Level;
use crate::points::{self, SAME};
use crate::prompt::{Prompt, upper_tr};
use crate::selectable;
use crate::tool::{
    Area, Context, Flow, Marker, MarkerShape, Memory, Pointer, Preview, Stroke, Tone, Tool,
};

/// The tool's id: its command is `tool.selectPolygon`.
pub const ID: &str = "selectPolygon";
pub const LABEL: &str = "Çokgenle seç";

/// The polygon's dash and gap, logical pixels.
const DASH: [f32; 2] = [5.0, 4.0];

/// The modes' options in the prompt and the keys that choose them.
const MODES: [(PolygonMode, &str, &str); 3] = [
    (PolygonMode::Inside, "İçindekiler", "İ"),
    (PolygonMode::Crossing, "Kesişenler", "K"),
    (PolygonMode::Outside, "Dışındakiler", "D"),
];

/// What the tool says it found, and found none of, by its mode.
fn found(mode: PolygonMode, n: usize) -> String {
    match mode {
        PolygonMode::Inside => format!("Çokgenin içinde {n} nesne; seçildi."),
        PolygonMode::Crossing => format!("Çokgene dokunan {n} nesne; seçildi."),
        PolygonMode::Outside => format!("Çokgenin dışında {n} nesne; seçildi."),
    }
}

fn none_found(mode: PolygonMode) -> &'static str {
    match mode {
        PolygonMode::Inside => "Çokgenin içinde nesne yok.",
        PolygonMode::Crossing => "Çokgene dokunan nesne yok.",
        PolygonMode::Outside => "Çokgenin dışında nesne yok.",
    }
}

/// Çokgenle seç.
#[derive(Clone, Debug, Default)]
pub struct SelectPolygon {
    pts: Vec<Vec2>,
    /// The next corner as the pointer would give it (ortho and polar tracking applied).
    hover: Option<Vec2>,
    tracking: Option<Tracking>,
    /// Shift was held at the last pointer event: ending the polygon then adds to the selection.
    shift: bool,
    /// What the session remembered, as of the last call (the prompt sees no context).
    memory: Memory,
}

impl SelectPolygon {
    pub fn new() -> Self {
        Self::default()
    }

    fn see(&mut self, cx: &Context<'_>) {
        self.memory = *cx.memory;
    }

    fn last(&self) -> Option<Vec2> {
        self.pts.last().copied()
    }

    /// A corner of the polygon, unless it is on the last one.
    fn add(&mut self, p: Vec2) {
        if self.last().is_none_or(|q| dist(q, p) > SAME) {
            self.pts.push(p);
        }
    }

    /// The polygon is done: what it finds is selected, and the tool leaves.
    fn finish(&mut self, cx: &mut Context<'_>) -> Flow {
        if self.pts.is_empty() {
            return Flow::Exit;
        }
        if let Some(problem) = ring_problem(&self.pts) {
            cx.say(Level::Warn, problem);
            return Flow::Stay;
        }
        let ring = std::mem::take(&mut self.pts);
        let mode = cx.memory.polygon_select;
        let hits = cx.spatial.in_polygon(&ring, mode);
        let hits = selectable::ids(cx, hits);
        let n = hits.len();
        if n == 0 {
            cx.say(Level::Warn, none_found(mode));
        } else {
            cx.selection.take(hits, cx.shift || self.shift);
            cx.say(Level::Info, found(mode, n));
        }
        Flow::Exit
    }
}

impl Tool for SelectPolygon {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn prompt(&self) -> Prompt {
        let mut prompt = if self.pts.is_empty() {
            Prompt::new(LABEL, "çokgenin ilk köşesine tıklayın")
        } else {
            Prompt::new(LABEL, "sonraki köşeye tıklayın")
                .option("Geri", "G")
                .option("Bitir", "Enter")
        };
        for (mode, name, key) in MODES {
            prompt = prompt.toggle(name, key, self.memory.polygon_select == mode);
        }
        prompt
    }

    fn point_count(&self) -> usize {
        self.pts.len()
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.see(cx);
        Flow::Stay
    }

    /// Perpendicular and tangent snaps are taken from the last corner.
    fn snap_from(&self) -> Option<Vec2> {
        self.last()
    }

    /// A point computed by the point calculator, as if clicked.
    fn accepts_points(&self) -> bool {
        true
    }

    fn accept_point(&mut self, p: Vec2, _cx: &mut Context<'_>) -> bool {
        self.add(p);
        true
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.see(cx);
        self.shift = p.shift;
        let (point, tracking) = points::constrain(self.last(), p, cx);
        self.hover = Some(point);
        self.tracking = tracking;
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.see(cx);
        self.shift = p.shift;
        let (point, tracking) = points::constrain(self.last(), p, cx);
        self.tracking = tracking;
        self.add(point);
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        self.see(cx);
        let key = upper_tr(js_trim(text));
        if key == "G" {
            return self.pts.pop().is_some();
        }
        if let Some((mode, _, _)) = MODES.iter().find(|(_, _, k)| *k == key) {
            cx.memory.polygon_select = *mode;
            self.see(cx);
            return true;
        }
        match cx.typed_point(text, self.last(), self.hover) {
            Some(p) => {
                self.add(p);
                true
            }
            None => false,
        }
    }

    /// Enter or a quick right click: the polygon is done.
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        self.finish(cx)
    }

    /// Esc drops the polygon drawn so far; with none it leaves.
    fn cancel(&mut self, _cx: &mut Context<'_>) -> bool {
        if self.pts.is_empty() {
            return false;
        }
        self.pts.clear();
        true
    }

    /// Ctrl+Z takes the last corner back.
    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        self.pts.pop().is_some()
    }

    /// The polygon closed back to its first corner, dashed: lightly filled in
    /// the window's colour for İçindekiler, in the snap colour for
    /// Kesişenler, unfilled for Dışındakiler (as the web's `drawSelectionPolygon`).
    fn preview(&self, _format: &Format) -> Preview {
        let mut ring = self.pts.clone();
        ring.extend(self.hover.filter(|_| !self.pts.is_empty()));
        let mode = self.memory.polygon_select;
        let tone = if mode == PolygonMode::Crossing {
            Tone::Snap
        } else {
            Tone::Accent
        };
        let mut preview = Preview {
            markers: self
                .pts
                .iter()
                .map(|&at| Marker {
                    at,
                    shape: MarkerShape::Ring(3.0),
                    tone: Tone::Accent,
                })
                .collect(),
            tracking: self.tracking.filter(|_| !self.pts.is_empty()),
            ..Preview::default()
        };
        if ring.len() >= 2 {
            if mode != PolygonMode::Outside && ring.len() >= 3 {
                preview.areas.push(Area {
                    rings: vec![ring.clone()],
                    fill: 0.1,
                    width: 1.0,
                    dash: Some(DASH),
                    fill_tone: tone,
                });
            }
            preview.strokes.push(Stroke {
                tone,
                ..Stroke::dashed(ring, true, DASH).width(1.5)
            });
        }
        preview
    }
}
