//! Paralel çizgi: the web's `ParallelLineTool` (Netcad's “Paralel çizgi”,
//! `apps/web/src/tools/parallelTool.ts`) on its `PointInputTool` base, step
//! for step (docs/adr/0057):
//!
//! - the axis point by point, while the lines at the left and right
//!   distances appear beside it, mitred at the corners (road edges, kerbs,
//!   walls); Enter or a quick right click ends it, Kapat (K) closes it,
//!   Geri (G) takes its last point back;
//! - Sol (S) and Sağ (A) ask for a distance, typed or shown by two clicks
//!   (Enter keeps the old one); Eksen (E) draws the axis or leaves it out;
//!   Alan olarak (U) writes the corridor between the sides as one area (a
//!   closed axis: a ring with a hole); all four stay for as long as the app
//!   lives.
//!
//! What is drawn is written through `cad.entities.create` as one undo step,
//! “Paralel çizgi”: “Paralel çizgi: 3 nesne, genişlik 10.000 m.” The sides,
//! the corridor and its area are the shared core's (`parallel_sides`,
//! `corridor_area`, `net_area`); none is computed here.

use kentos_contracts::{CreateOperation, EntityGeometry};
use kentos_geometry_core::geom::arc::DEFAULT_STEP;
use kentos_geometry_core::geom::arrangement::{Area, Ring};
use kentos_geometry_core::geom::bulge::bulge_path_outline;
use kentos_geometry_core::geom::parallel::{clean_axis, corridor_area, parallel_sides};
use kentos_geometry_core::geom::region::net_area;
use kentos_geometry_core::geometry::{bearing_grad, dist};
use kentos_geometry_core::ops::areas::polygon_of_area;
use kentos_geometry_core::tools::point_text::{js_trim, point_from_text};
use kentos_native_application::geometry::edit_geometry;

use crate::Vec2;
use crate::format::Format;
use crate::log::Level;
use crate::points::{self, Taken, wire_all};
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{self, Context, Flow, Memory, Pointer, Preview, Stroke, Tag, Tool};

/// The parallel line tool's id: its command is `tool.parallel`.
pub const ID: &str = "parallel";
pub const LABEL: &str = "Paralel çizgi";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Side {
    Left,
    Right,
}

/// The parallel line tool.
#[derive(Clone, Debug, Default)]
pub struct Parallel {
    d: Taken,
    /// The side whose distance is being asked for, and its first point when shown with two clicks.
    ask: Option<Side>,
    measure_from: Option<Vec2>,
    /// What the session remembered and the project's units, as of the last call.
    seen: Option<(Memory, Format)>,
}

/// A ring's points, arcs tessellated (the web's `polygonRing`).
fn ring_points(r: &Ring) -> Vec<Vec2> {
    match &r.bulges {
        Some(b) => bulge_path_outline(&r.pts, Some(b), true, DEFAULT_STEP),
        None => r.pts.clone(),
    }
}

/// An area as the preview fills it: the outer ring, then its holes.
fn area_rings(a: &Area) -> Vec<Vec<Vec2>> {
    std::iter::once(&a.outer)
        .chain(&a.holes)
        .map(ring_points)
        .collect()
}

impl Parallel {
    pub fn new() -> Self {
        Self::default()
    }

    fn see(&mut self, cx: &Context<'_>) {
        self.seen = Some((*cx.memory, cx.format()));
    }

    fn memory(&self) -> Memory {
        self.seen.map_or_else(Memory::default, |(m, _)| m)
    }

    /// The point the next one is taken from: the distance's first point while
    /// one is shown, else the axis's last (the web's `last`).
    fn last(&self) -> Option<Vec2> {
        match self.ask {
            Some(_) => self.measure_from,
            None => self.d.last(),
        }
    }

    fn constrain(&mut self, p: &Pointer, cx: &Context<'_>) -> Vec2 {
        let (point, tracking) = points::constrain(self.last(), p, cx);
        self.d.tracking = tracking;
        point
    }

    /// The settings every prompt of the axis offers.
    fn settings(&self, prompt: Prompt) -> Prompt {
        let m = self.memory();
        let format = self.seen.map_or_else(Format::default, |(_, f)| f);
        prompt
            .option_with("Sol", "S", format.length(m.parallel_left))
            .option_with("Sağ", "A", format.length(m.parallel_right))
            .option_with(
                "Eksen",
                "E",
                if m.parallel_axis {
                    "çizilir"
                } else {
                    "çizilmez"
                },
            )
            .option_with(
                "Alan olarak",
                "U",
                if m.parallel_area { "evet" } else { "hayır" },
            )
    }

    fn option(&mut self, key: &str, cx: &mut Context<'_>) -> bool {
        match key {
            "S" | "A" => {
                self.ask = Some(if key == "S" { Side::Left } else { Side::Right });
                self.measure_from = None;
            }
            "E" => cx.memory.parallel_axis = !cx.memory.parallel_axis,
            "U" => cx.memory.parallel_area = !cx.memory.parallel_area,
            "G" if !self.d.pts.is_empty() && self.ask.is_none() => {
                self.d.pts.pop();
            }
            "K" if self.d.pts.len() >= 3 && self.ask.is_none() => self.commit(true, cx),
            _ => return false,
        }
        true
    }

    fn set_distance(&mut self, d: f64, cx: &mut Context<'_>) {
        let side = match self.ask {
            Some(Side::Left) => {
                cx.memory.parallel_left = d;
                "Sol"
            }
            _ => {
                cx.memory.parallel_right = d;
                "Sağ"
            }
        };
        let line = format!("  {side} mesafe: {}", cx.format().length(d));
        cx.say(Level::Info, line);
        self.ask = None;
        self.measure_from = None;
    }

    fn accept(&mut self, p: Vec2, cx: &mut Context<'_>) {
        self.d.begin(p, cx);
        if self.ask.is_some() {
            match self.measure_from {
                None => self.measure_from = Some(p),
                Some(from) => self.set_distance(dist(from, p), cx),
            }
            return;
        }
        self.d.pts.push(p);
    }

    /// The sides (or the corridor) and the axis, written through
    /// `cad.entities.create` (docs/adr/0057) as one undo step, “Paralel
    /// çizgi”: all of them, or none with the command's reason.
    fn commit(&mut self, closed: bool, cx: &mut Context<'_>) {
        let axis = clean_axis(&self.d.pts, closed);
        if axis.len() < if closed { 3 } else { 2 } {
            self.d.reset();
            return;
        }
        let m = *cx.memory;
        let format = cx.format();
        let line = |pts: &[Vec2]| {
            if closed {
                EntityGeometry::Polygon {
                    pts: wire_all(pts),
                    bulges: None,
                    holes: None,
                }
            } else {
                EntityGeometry::Polyline {
                    pts: wire_all(pts),
                    bulges: None,
                }
            }
        };
        let mut objects = Vec::new();
        let mut summary = String::new();
        if m.parallel_area {
            if let Some(area) = corridor_area(&axis, m.parallel_left, m.parallel_right, closed)
                && let Some(polygon) = edit_geometry(polygon_of_area(&area).shape)
            {
                objects.push(polygon);
                summary = format!(", alan {}", format.area(net_area(&area)));
            }
        } else {
            let sides = parallel_sides(&axis, m.parallel_left, m.parallel_right, closed);
            objects.extend([sides.left, sides.right].iter().flatten().map(|s| line(s)));
        }
        if m.parallel_axis {
            objects.push(line(&axis));
        }
        let made = objects.len();
        // The web's `!(S.left > 0)`: a distance that is not above zero, NaN included.
        let positive = |v: f64| v > 0.0;
        if made == 0 {
            if !m.parallel_axis && !positive(m.parallel_left) && !positive(m.parallel_right) {
                cx.say(
                    Level::Warn,
                    "Sol ve sağ mesafe sıfır ve eksen çizilmiyor: çizilecek bir şey yok.",
                );
            }
        } else if let Some(out) =
            points::write_objects(objects, Some(CreateOperation::Parallel), cx)
        {
            if let Some(&id) = out.ids.first() {
                self.d.note(id, cx);
            }
            let width = format.length(m.parallel_left + m.parallel_right);
            cx.say(
                Level::Success,
                format!("Paralel çizgi: {made} nesne, genişlik {width}{summary}."),
            );
        }
        self.d.reset();
    }
}

impl Tool for Parallel {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn prompt(&self) -> Prompt {
        if let Some(side) = self.ask {
            let side = if side == Side::Left { "sol" } else { "sağ" };
            return Prompt::new(
                LABEL,
                match self.measure_from {
                    Some(_) => format!("{side} mesafenin ikinci noktasını gösterin"),
                    None => format!("{side} mesafeyi yazın ya da iki noktayla gösterin"),
                },
            );
        }
        let n = self.d.pts.len();
        if n == 0 {
            return self.settings(Prompt::new(LABEL, "eksenin ilk noktasını gösterin"));
        }
        let more = if n >= 2 {
            " ya da bitirmek için sağ tıklayın"
        } else {
            ""
        };
        let prompt = Prompt::new(LABEL, format!("eksenin sonraki noktasını gösterin{more}"))
            .option("Geri", "G");
        let prompt = if n >= 3 {
            prompt.option("Kapat", "K")
        } else {
            prompt
        };
        self.settings(prompt)
    }

    fn point_count(&self) -> usize {
        self.d.pts.len()
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.see(cx);
        Flow::Stay
    }

    fn snap_from(&self) -> Option<Vec2> {
        self.last()
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.d.hover = Some(self.constrain(p, cx));
        self.see(cx);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let p = self.constrain(p, cx);
        self.accept(p, cx);
        self.see(cx);
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let done = if self.option(&upper_tr(js_trim(text)), cx) {
            true
        } else if let (Some(_), Some(n)) = (self.ask, points::plain_number(text)) {
            if n < 0.0 {
                cx.say(
                    Level::Warn,
                    "Mesafe sıfır ya da pozitif olmalı; karşı taraf için diğer seçeneği kullanın.",
                );
            } else {
                self.set_distance(n, cx);
            }
            true
        } else {
            match point_from_text(text, self.last(), self.d.hover, |_| None) {
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

    /// Enter: leaving a distance's prompt keeps the old value; an axis of two
    /// points or more is written; with none, the tool leaves.
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        let flow = if self.ask.is_some() {
            self.ask = None;
            self.measure_from = None;
            Flow::Stay
        } else if self.d.pts.is_empty() {
            Flow::Exit
        } else {
            if clean_axis(&self.d.pts, false).len() >= 2 {
                self.commit(false, cx);
            } else {
                self.d.reset();
            }
            Flow::Stay
        };
        self.see(cx);
        flow
    }

    /// Ctrl+Z (ADR 0018): Geri (G) first, the axis's last point; then what
    /// was just written, as an undo; else the draft starts over.
    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        if !self.d.pts.is_empty() && self.option("G", cx) {
            return true;
        }
        self.d.undo_step(false, cx)
    }

    fn preview(&self, format: &Format) -> Preview {
        let tracking = self.d.hover.and(self.d.tracking);
        if self.ask.is_some() {
            let mut preview = Preview {
                tracking,
                ..Preview::default()
            };
            if let (Some(from), Some(h)) = (self.measure_from, self.d.hover) {
                preview.strokes = vec![Stroke::dashed(vec![from, h], false, [4.0, 3.0])];
                preview.tag = Some(Tag {
                    at: h,
                    lines: vec![format.length(dist(from, h))],
                });
            }
            return preview;
        }
        let chain: Vec<Vec2> = self.d.pts.iter().copied().chain(self.d.hover).collect();
        if chain.len() < 2 {
            return Preview {
                tracking,
                ..Preview::default()
            };
        }
        let m = self.memory();
        let mut preview = Preview {
            tracking,
            ..Preview::default()
        };
        if m.parallel_area {
            if let Some(area) = corridor_area(&chain, m.parallel_left, m.parallel_right, false) {
                preview.areas.push(tool::Area {
                    rings: area_rings(&area),
                    fill: 0.16,
                    width: 1.5,
                });
            }
        } else {
            let sides = parallel_sides(&chain, m.parallel_left, m.parallel_right, false);
            for side in [sides.left, sides.right].into_iter().flatten() {
                preview.strokes.push(Stroke::solid(side, false).width(1.5));
            }
        }
        // The axis: solid when it will be drawn, dashed when it only guides.
        let mut axis = Stroke::solid(chain, false);
        if !m.parallel_axis {
            axis.dash = Some([6.0, 4.0]);
        }
        preview.strokes.push(axis);
        if let (Some(last), Some(h)) = (self.d.last(), self.d.hover) {
            preview.tag = Some(Tag {
                at: h,
                lines: vec![
                    format.length(dist(last, h)),
                    format!("Semt {}", format.bearing(bearing_grad(last, h))),
                ],
            });
        }
        preview
    }
}
