//! Böl: the web's `DivideTool` (AutoCAD DIVIDE and MEASURE,
//! `apps/web/src/tools/pathEditTools.ts`) on its edge-picking base, step
//! for step (docs/adr/0057):
//!
//! - the object is picked by its edge (a line, polyline, closed area, arc,
//!   circle, ellipse or spline, on any layer); the points to come are shown
//!   on it, measured from the end nearer the click;
//! - a typed number of parts (2 to 10 000), or with Aralık (A) a step in
//!   metres, places the points; Enter (or a quick right click) takes the
//!   last value; Parça sayısı (P) goes back to parts; the mode and both
//!   values stay for as long as the app lives;
//! - Esc drops the picked object; with none, the tool leaves.
//!
//! The points go on the active layer through `cad.entities.create` as one
//! undo step, “Böl”: “4 nokta kondu.” Where they fall is the shared core's
//! (`division_points`); an ellipse's land on the true curve.

use kentos_contracts::{CreateOperation, Entity, EntityGeometry};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::ops::path::{Division, division_points, nearest_s, path_of};
use kentos_geometry_core::tools::point_text::{js_trim, parse_number};
use kentos_native_application::geometry::shape;

use crate::Vec2;
use crate::edge::{self, Outline};
use crate::format::Format;
use crate::log::Level;
use crate::points::{self, wire};
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Flow, Marker, MarkerShape, Memory, Pointer, Preview, Tone, Tool};

/// The divide tool's id: its command is `tool.divide`.
pub const ID: &str = "divide";
pub const LABEL: &str = "Böl";

/// Most points a division places (and the most a typed part count may ask for).
const MAX_POINTS: usize = 10_000;
/// The preview marks the points only up to this many (the web's draw).
const MAX_SHOWN: usize = 2000;

/// The kinds that can be divided, on any layer: the points go on the active one.
fn divisible(e: &Entity, _: &Document) -> bool {
    matches!(
        e,
        Entity::Line(_)
            | Entity::Polyline(_)
            | Entity::Polygon(_)
            | Entity::Arc(_)
            | Entity::Circle(_)
            | Entity::Ellipse(_)
            | Entity::Spline(_)
    )
}

/// The object picked, and whether measuring starts from its far end.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Target {
    slot: Slot,
    from_end: bool,
}

/// The divide tool.
#[derive(Clone, Debug, Default)]
pub struct Divide {
    target: Option<Target>,
    /// What the session remembered and the project's units, as of the last call.
    seen: Option<(Memory, Format)>,
    /// The picked object and its points to come, as of the last call.
    drawn: Preview,
}

impl Divide {
    pub fn new() -> Self {
        Self::default()
    }

    /// Takes in what the session remembers and redraws the points to come.
    fn see(&mut self, cx: &Context<'_>) {
        self.seen = Some((*cx.memory, cx.format()));
        self.drawn = Preview::default();
        let Some(e) = self.target.and_then(|t| cx.doc.get(t.slot)) else {
            return;
        };
        let out = Outline::of(&shape(e), None, 1.5, Tone::Accent);
        let pts = self.points(cx);
        self.drawn = Preview {
            strokes: out.strokes,
            marks: out.marks,
            markers: if pts.len() > MAX_SHOWN {
                Vec::new()
            } else {
                pts.into_iter()
                    .map(|at| Marker {
                        at,
                        shape: MarkerShape::Circle(3.5),
                        tone: Tone::Accent,
                    })
                    .collect()
            },
            ..Preview::default()
        };
    }

    /// The points along the picked object, by the core: all of them in one call.
    fn points(&self, cx: &Context<'_>) -> Vec<Vec2> {
        let Some(target) = self.target else {
            return Vec::new();
        };
        let Some(e) = cx.doc.get(target.slot) else {
            return Vec::new();
        };
        let m = *cx.memory;
        let by = if m.divide_by_step {
            Division::Step(m.divide_step)
        } else {
            Division::Parts(f64::from(m.divide_parts))
        };
        division_points(&shape(e), &by, target.from_end)
    }

    /// Places the points on the active layer, one undo step “Böl”; the object is let go.
    fn commit(&mut self, cx: &mut Context<'_>) {
        let pts = self.points(cx);
        if pts.is_empty() {
            cx.say(Level::Warn, "Aralık nesne boyundan uzun; nokta konmadı.");
        } else if pts.len() > MAX_POINTS {
            cx.say(
                Level::Warn,
                "10 000’den fazla nokta oluşacak; daha büyük bir aralık girin.",
            );
        } else {
            let n = pts.len();
            let objects = pts
                .into_iter()
                .map(|p| EntityGeometry::Point {
                    p: wire(p),
                    z: None,
                })
                .collect();
            if points::write_objects(objects, Some(CreateOperation::Divide), cx).is_some() {
                cx.say(Level::Success, format!("{n} nokta kondu."));
            }
        }
        self.target = None;
    }
}

impl Tool for Divide {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn prompt(&self) -> Prompt {
        let (m, format) = self.seen.unwrap_or((Memory::default(), Format::default()));
        let mode = if m.divide_by_step {
            format!("aralık {}", format.length(m.divide_step))
        } else {
            format!("{} parça", m.divide_parts)
        };
        let (other, key) = if m.divide_by_step {
            ("Parça sayısı", "P")
        } else {
            ("Aralık", "A")
        };
        match self.target {
            Some(_) => {
                let what = if m.divide_by_step {
                    "aralığı"
                } else {
                    "parça sayısını"
                };
                Prompt::new(LABEL, format!("{what} yazın ya da Enter ({mode})")).option(other, key)
            }
            None => Prompt::new(LABEL, "noktaların konacağı nesneyi seçin")
                .note(mode)
                .then()
                .option(other, key),
        }
    }

    fn point_count(&self) -> usize {
        0
    }

    /// The object is picked by its edge: no snaps (the web's `EdgePickTool`).
    fn snaps(&self) -> bool {
        false
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.see(cx);
        Flow::Stay
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        edge::hover(p, cx, divisible);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let Some(slot) = edge::pick(p, cx, divisible) else {
            cx.say(
                Level::Warn,
                "Çizgi, çoklu çizgi, yay, daire ya da eğriye tıklayın.",
            );
            return;
        };
        let path = cx.doc.get(slot).and_then(|e| path_of(&shape(e)));
        let Some(path) = path.filter(|path| path.length >= 1e-9) else {
            cx.say(Level::Warn, "Bu nesne bölünemez.");
            return;
        };
        // Measuring starts from the end nearer to the click.
        let from_end = !path.closed && nearest_s(&path, p.raw) > path.length / 2.0;
        self.target = Some(Target { slot, from_end });
        cx.selection.set_hover(None);
        self.see(cx);
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let key = upper_tr(js_trim(text));
        if key == "A" || key == "P" {
            cx.memory.divide_by_step = key == "A";
            self.see(cx);
            return true;
        }
        let Some(n) = parse_number(text).filter(|n| *n > 0.0) else {
            return false;
        };
        if cx.memory.divide_by_step {
            cx.memory.divide_step = n;
        } else {
            if n.fract() != 0.0 || !(2.0..=MAX_POINTS as f64).contains(&n) {
                cx.say(
                    Level::Warn,
                    "Parça sayısı 2 ile 10 000 arasında bir tam sayı olmalı.",
                );
                return true;
            }
            cx.memory.divide_parts = n as u32;
        }
        if self.target.is_some() {
            self.commit(cx);
        }
        self.see(cx);
        true
    }

    /// Enter: the picked object is divided by the last value; with none, the tool leaves.
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if self.target.is_none() {
            return Flow::Exit;
        }
        self.commit(cx);
        self.see(cx);
        Flow::Stay
    }

    /// Esc drops the picked object; with none, the tool leaves.
    fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
        if self.target.take().is_none() {
            return false;
        }
        self.see(cx);
        true
    }

    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }

    fn preview(&self, _format: &Format) -> Preview {
        self.drawn.clone()
    }
}
