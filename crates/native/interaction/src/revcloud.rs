//! Revizyon bulutu: the web's `RevCloudTool` (AutoCAD REVCLOUD,
//! `apps/web/src/tools/markupTools.ts`) on its `PointInputTool` base, step
//! for step (docs/adr/0057):
//!
//! - a rectangle's two corners (the default), or with Çokgen (Ç, or C) a
//!   polygon's corners ended by Enter or a quick right click; Dikdörtgen (D)
//!   goes back; both before the first corner;
//! - Yay boyu (U): the arcs' length in paper millimetres at the project's
//!   plot scale; the shape and the length stay for as long as the app lives.
//!
//! The cloud is a closed area of outward arcs written through
//! `cad.polygon.create`, its own object and undo step (“Ekle”): “Revizyon
//! bulutu eklendi: 24 yay.” Its corners and arcs are the shared core's
//! (`cloud_of`).

use kentos_geometry_core::geom::arc::DEFAULT_STEP;
use kentos_geometry_core::geom::bulge::bulge_path_outline;
use kentos_geometry_core::geom::shapes::cloud_of;
use kentos_geometry_core::geometry::{dist, signed_area};
use kentos_geometry_core::tools::point_text::{js_trim, parse_number, point_from_text};

use crate::Vec2;
use crate::format::{Format, js_number};
use crate::log::Level;
use crate::points::{self, Taken};
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Flow, Memory, Pointer, Preview, Stroke, Tool};

/// The revision cloud tool's id: its command is `tool.revcloud`.
pub const ID: &str = "revcloud";
pub const LABEL: &str = "Revizyon bulutu";

/// The revision cloud tool.
#[derive(Clone, Debug, Default)]
pub struct RevCloud {
    d: Taken,
    ask_arc: bool,
    /// What the session remembered and the project's plot scale, as of the last call.
    seen: Option<(Memory, f64)>,
}

impl RevCloud {
    pub fn new() -> Self {
        Self::default()
    }

    fn see(&mut self, cx: &Context<'_>) {
        self.seen = Some((*cx.memory, cx.doc.settings().plot_scale));
    }

    fn memory(&self) -> Memory {
        self.seen.map_or_else(Memory::default, |(m, _)| m)
    }

    /// The arcs' length in world metres: paper millimetres at the plot scale.
    fn arc_length(&self) -> f64 {
        let (m, scale) = self.seen.unwrap_or((Memory::default(), 1000.0));
        (m.cloud_arc_mm / 1000.0) * scale
    }

    /// The ring the cloud goes round: the rectangle on two corners, or the corners.
    fn ring_for(&self, pts: &[Vec2]) -> Vec<Vec2> {
        match (self.memory().cloud_rect, pts) {
            (true, [a, b, ..]) => vec![*a, Vec2::new(b.x, a.y), *b, Vec2::new(a.x, b.y)],
            _ => pts.to_vec(),
        }
    }

    /// Dikdörtgen (D), Çokgen (Ç) and Yay boyu (U), before the first corner.
    fn option(&mut self, key: &str, cx: &mut Context<'_>) -> bool {
        if !self.d.pts.is_empty() {
            return false;
        }
        match key {
            "D" => cx.memory.cloud_rect = true,
            "Ç" | "C" => cx.memory.cloud_rect = false,
            "U" => self.ask_arc = true,
            _ => return false,
        }
        true
    }

    fn accept(&mut self, p: Vec2, cx: &mut Context<'_>) {
        self.d.begin(p, cx);
        if self.ask_arc || self.d.last().is_some_and(|last| dist(last, p) <= 1e-9) {
            return;
        }
        self.d.pts.push(p);
        if cx.memory.cloud_rect && self.d.pts.len() == 2 {
            self.finish(cx);
        }
    }

    /// Writes the cloud through `cad.polygon.create` (docs/adr/0057); a
    /// refused one says only the command's reason. The draft starts over.
    fn finish(&mut self, cx: &mut Context<'_>) {
        self.see(cx);
        let ring = self.ring_for(&self.d.pts);
        let cloud = (ring.len() >= 3 && signed_area(&ring).abs() > 1e-9)
            .then(|| cloud_of(&ring, self.arc_length()))
            .flatten();
        match cloud {
            Some(cloud) => {
                let pts = cloud.pts.clone();
                if points::write_ring(&mut self.d, &pts, Some(cloud.bulges), cx) {
                    let line = format!("Revizyon bulutu eklendi: {} yay.", pts.len());
                    cx.say(Level::Success, line);
                }
            }
            None if !self.d.pts.is_empty() => cx.say(
                Level::Warn,
                "Bulut için alanı olan bir dikdörtgen ya da en az üç köşe gerekir.",
            ),
            None => {}
        }
        self.d.reset();
    }
}

impl Tool for RevCloud {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn prompt(&self) -> Prompt {
        if self.ask_arc {
            return Prompt::new(LABEL, "yay boyunu kâğıt milimetresi olarak yazın");
        }
        let m = self.memory();
        let n = self.d.pts.len();
        let first = |step: &'static str| {
            let prompt = Prompt::new(LABEL, step);
            let prompt = if m.cloud_rect {
                prompt.option("Çokgen", "Ç")
            } else {
                prompt.option("Dikdörtgen", "D")
            };
            prompt.option_with("Yay boyu", "U", format!("{} mm", js_number(m.cloud_arc_mm)))
        };
        match (m.cloud_rect, n) {
            (true, 0) => first("dikdörtgenin bir köşesine tıklayın"),
            (true, _) => Prompt::new(LABEL, "karşı köşeye tıklayın"),
            (false, 0) => first("bulutun ilk köşesine tıklayın"),
            (false, 1 | 2) => Prompt::new(LABEL, "sonraki köşeye tıklayın"),
            (false, _) => Prompt::new(
                LABEL,
                "sonraki köşeye tıklayın ya da bitirmek için sağ tıklayın",
            ),
        }
    }

    fn point_count(&self) -> usize {
        self.d.pts.len()
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.see(cx);
        Flow::Stay
    }

    fn snap_from(&self) -> Option<Vec2> {
        self.d.last()
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.d.hover = Some(self.d.constrain(p, cx));
        self.see(cx);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let p = self.d.constrain(p, cx);
        self.accept(p, cx);
        self.see(cx);
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let done = if self.option(&upper_tr(js_trim(text)), cx) {
            true
        } else if let (true, Some(n)) = (self.ask_arc, parse_number(text)) {
            if n > 0.0 {
                cx.memory.cloud_arc_mm = n;
            } else {
                cx.say(Level::Warn, "Yay boyu sıfırdan büyük olmalı.");
            }
            self.ask_arc = false;
            true
        } else {
            match point_from_text(text, self.d.last(), self.d.hover, |_| None) {
                Some(p) => {
                    self.accept(p, cx);
                    true
                }
                None => false,
            }
        };
        self.see(cx);
        done
    }

    /// Enter: the corners so far make the cloud; with none, the tool leaves.
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if self.d.pts.is_empty() {
            return Flow::Exit;
        }
        self.finish(cx);
        Flow::Stay
    }

    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        self.d.undo_step(false, cx)
    }

    fn preview(&self, _format: &Format) -> Preview {
        let pts: Vec<Vec2> = self.d.pts.iter().copied().chain(self.d.hover).collect();
        let tracking = self.d.hover.and(self.d.tracking);
        if pts.len() < 2 {
            return Preview {
                tracking,
                ..Preview::default()
            };
        }
        let ring = self.ring_for(&pts);
        let cloud = (ring.len() >= 3)
            .then(|| cloud_of(&ring, self.arc_length()))
            .flatten();
        let stroke = match cloud {
            Some(cloud) => Stroke::solid(
                bulge_path_outline(&cloud.pts, Some(&cloud.bulges), true, DEFAULT_STEP),
                true,
            )
            .width(1.5),
            None => Stroke::dashed(pts, false, [4.0, 3.0]),
        };
        Preview {
            strokes: vec![stroke],
            tracking,
            ..Preview::default()
        }
    }
}
