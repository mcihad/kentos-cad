//! Daire: the web's `CircleTool` (`apps/web/src/tools/curveTools.ts`) on its
//! `PointInputTool` base, step for step (docs/adr/0032). Five methods:
//!
//! - centre and radius (the default): the centre, then a point on the circle
//!   or a typed radius; Çap (Ç, or C) asks for the diameter instead, Yarıçap
//!   (R) for the radius again — the choice stays for the next circles;
//! - 2 nokta (2N): the two ends of a diameter;
//! - 3 nokta (3N): three points on the circle (collinear ones warn and start over);
//! - Teğet-teğet-yarıçap (TTY): two objects picked near where the circle is to
//!   touch them, then a typed radius, or Enter for the last circle's;
//! - Teğet-teğet-teğet (TTT): three objects picked the same way.
//!
//! A method is chosen before the first point and stays for the next circles.
//! Tangent picks take the nearest edge of the object under the pointer
//! (lines, polylines, closed areas, arcs, circles, construction lines) from
//! the geometry store; snaps are off while picking. Every circle is written
//! through `cad.circle.create`, its own object and undo step, and says “Daire
//! eklendi: r = …”. The circle of each method, the tangent picks' edges and
//! the previews are the shared core's (`kentos-geometry-core`); none is
//! computed here.

use kentos_contracts::CircleCreate;
use kentos_geometry_core::entity::tessellate_circle;
use kentos_geometry_core::geom::arc::{Circle as Round, circle_through};
use kentos_geometry_core::geom::intersect::Edge;
use kentos_geometry_core::geom::tangent_circle::{tangent_tangent_radius, tangent_tangent_tangent};
use kentos_geometry_core::geometry::dist;
use kentos_geometry_core::ops::edges::nearest_edge;
use kentos_geometry_core::tools::drawing::circle_on_diameter;
use kentos_geometry_core::tools::point_text::{js_trim, point_from_text};
use kentos_native_application::{ExecutionContext, circle};

use crate::Vec2;
use crate::format::Format;
use crate::log::Level;
use crate::points::{self, Taken, plain_number, wire};
use crate::prompt::{Prompt, upper_tr};
use crate::spatial::record;
use crate::tool::{Context, Flow, Memory, Pointer, Preview, Stroke, Tag, Tool};

/// The circle tool's id: its command is `tool.circle`.
pub const ID: &str = "circle";
pub const LABEL: &str = "Daire";

/// Kinds a tangent circle may touch (the web's `pickEdge` filter).
const TANGENT_KINDS: [&str; 7] = [
    "line", "polyline", "polygon", "arc", "circle", "xline", "ray",
];
/// Segments of a previewed circle (the web's `tessellateCircle(…, 96)`).
const PREVIEW_SEGMENTS: f64 = 96.0;
/// A radius at or below this writes nothing (the web's `r > 1e-9`).
const TINY: f64 = 1e-9;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Center,
    Two,
    Three,
    Ttr,
    Ttt,
}

/// An object picked for a tangent circle: its nearest edge and where it was clicked.
#[derive(Clone, Copy, Debug, PartialEq)]
struct TangentPick {
    edge: Edge,
    pick: Vec2,
}

/// The circle tool.
#[derive(Clone, Debug)]
pub struct Circle {
    d: Taken,
    mode: Mode,
    tangents: Vec<TangentPick>,
    /// Centre mode: the second input is a diameter (AutoCAD's “Çap”).
    diameter: bool,
    /// What the session remembered and the project's units, as of the last
    /// call: the prompt offers the last radius (the web's `lastRadius`).
    seen: Option<(Memory, Format)>,
}

impl Default for Circle {
    fn default() -> Self {
        Self {
            d: Taken::default(),
            mode: Mode::Center,
            tangents: Vec::new(),
            diameter: false,
            seen: None,
        }
    }
}

impl Circle {
    pub fn new() -> Self {
        Self::default()
    }

    fn see(&mut self, cx: &Context<'_>) {
        self.seen = Some((*cx.memory, cx.format()));
    }

    fn last_radius(&self) -> f64 {
        self.seen.map_or(0.0, |(m, _)| m.circle_radius)
    }

    fn accept(&mut self, p: Vec2, cx: &mut Context<'_>) {
        self.d.begin(p, cx);
        self.on_point(p, cx);
    }

    fn on_point(&mut self, p: Vec2, cx: &mut Context<'_>) {
        if self.d.last().is_some_and(|last| dist(last, p) < TINY) {
            return;
        }
        self.d.pts.push(p);
        let (a, b, c) = (
            self.d.pts[0],
            self.d.pts.get(1).copied(),
            self.d.pts.get(2).copied(),
        );
        match (self.mode, b, c) {
            (Mode::Center, Some(b), _) => {
                let r = if self.diameter {
                    dist(a, b) / 2.0
                } else {
                    dist(a, b)
                };
                self.commit(a, r, cx);
            }
            (Mode::Two, Some(b), _) => {
                let round = circle_on_diameter(a, b);
                self.commit(round.c, round.r, cx);
            }
            (Mode::Three, Some(b), Some(c)) => match circle_through(a, b, c) {
                Some(round) => self.commit(round.c, round.r, cx),
                None => {
                    cx.say(
                        Level::Warn,
                        "Üç nokta aynı doğru üzerinde; daire çizilemez.",
                    );
                    self.d.pts.clear();
                }
            },
            _ => {}
        }
    }

    /// Option letters: Ç/C and R after the centre; 2N, 3N, TTY, TTT and M
    /// (back to the centre) before the first point. False when the key is
    /// none of those now.
    fn option(&mut self, key: &str) -> bool {
        if self.mode == Mode::Center && self.d.pts.len() == 1 && matches!(key, "Ç" | "C" | "R") {
            self.diameter = key != "R";
            return true;
        }
        let mode = match key {
            "2N" => Mode::Two,
            "3N" => Mode::Three,
            "TTY" => Mode::Ttr,
            "TTT" => Mode::Ttt,
            "M" => Mode::Center,
            _ => return false,
        };
        if !self.d.pts.is_empty() {
            return false;
        }
        self.mode = mode;
        self.tangents.clear();
        true
    }

    fn tangent_modes(&self) -> bool {
        matches!(self.mode, Mode::Ttr | Mode::Ttt)
    }

    /// A tangent pick: the nearest edge of the object under the pointer,
    /// among the kinds a circle can touch (the web's `pickEdge`).
    fn pick_tangent(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let hit = cx
            .spatial
            .store()
            .hit_edge(p.raw, cx.pick_tolerance())
            .into_iter()
            .filter_map(|(id, _)| id_slot(id))
            .filter_map(|slot| cx.doc.get(slot))
            .find(|e| TANGENT_KINDS.contains(&e.kind()));
        let Some(entity) = hit else {
            cx.say(
                Level::Warn,
                "Teğet olunacak bir çizgi, çoklu çizgi, yay ya da daireye tıklayın.",
            );
            return;
        };
        let Some(edge) = nearest_edge(&record(entity).3, p.raw) else {
            return;
        };
        self.tangents.push(TangentPick { edge, pick: p.raw });
        if self.mode == Mode::Ttt && self.tangents.len() == 3 {
            let t = std::mem::take(&mut self.tangents);
            match tangent_tangent_tangent(
                &[t[0].edge, t[1].edge, t[2].edge],
                &[t[0].pick, t[1].pick, t[2].pick],
            ) {
                Some(round) => self.commit(round.c, round.r, cx),
                None => cx.say(
                    Level::Warn,
                    "Üç nesneye birden teğet bir daire bulunamadı; nesnelere teğet noktalarının yakınından tıklayın.",
                ),
            }
        }
    }

    /// The circle of radius `r` touching the two picked objects.
    fn ttr_circle(&self, r: f64) -> Option<Round> {
        let [t1, t2] = [self.tangents.first()?, self.tangents.get(1)?];
        tangent_tangent_radius(&t1.edge, t1.pick, &t2.edge, t2.pick, r)
    }

    fn commit_tangent(&mut self, r: f64, cx: &mut Context<'_>) {
        let Some(round) = self.ttr_circle(r) else {
            cx.say(
                Level::Warn,
                "Bu yarıçapla iki nesneye birden teğet bir daire yok.",
            );
            return;
        };
        self.tangents.clear();
        self.commit(round.c, round.r, cx);
    }

    /// Writes the circle through `cad.circle.create` (docs/adr/0032) and
    /// remembers its radius; the draft's points go either way.
    fn commit(&mut self, c: Vec2, r: f64, cx: &mut Context<'_>) {
        if r > TINY && self.write(c, r, cx) {
            cx.memory.circle_radius = r;
            let line = format!("Daire eklendi: r = {}", cx.format().length(r));
            cx.say(Level::Success, line);
        }
        self.d.pts.clear();
    }

    /// One circle through the product command: the active layer explicit in
    /// its input (CMD-07); the desktop has no current colour.
    fn write(&mut self, c: Vec2, r: f64, cx: &mut Context<'_>) -> bool {
        let input = CircleCreate {
            layer_id: cx.doc.layers().active().to_owned(),
            c: wire(c),
            r,
            color: None,
            attrs: None,
            expected_revision: None,
        };
        let result = circle::execute(&mut ExecutionContext::new(cx.doc), input);
        match points::written(result, cx) {
            Some(written) => {
                self.d.note(written.id, cx);
                true
            }
            None => false,
        }
    }
}

/// A store id back to a document slot.
fn id_slot(id: f64) -> Option<kentos_domain::Slot> {
    (id >= 0.0 && id <= f64::from(u32::MAX) && id.fract() == 0.0)
        .then_some(kentos_domain::Slot(id as u32))
}

impl Tool for Circle {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn prompt(&self) -> Prompt {
        let n = self.d.pts.len();
        match self.mode {
            Mode::Two if n == 0 => Prompt::new(LABEL, "çapın ilk ucunu belirtin"),
            Mode::Two => Prompt::new(LABEL, "çapın ikinci ucunu belirtin"),
            Mode::Three => Prompt::new(
                LABEL,
                match n {
                    0 => "ilk noktayı belirtin",
                    1 => "ikinci noktayı belirtin",
                    2 => "üçüncü noktayı belirtin",
                    _ => "",
                },
            ),
            Mode::Ttr => match self.tangents.len() {
                0 => Prompt::new(LABEL, "ilk teğet çizgi, yay ya da daireyi seçin"),
                1 => Prompt::new(LABEL, "ikinci teğet nesneyi seçin"),
                _ => {
                    let hint = match self.seen {
                        Some((m, format)) if m.circle_radius > 0.0 => {
                            format!(" (Enter: {})", format.length(m.circle_radius))
                        }
                        _ => String::new(),
                    };
                    Prompt::new(LABEL, format!("yarıçapı yazın{hint}"))
                }
            },
            Mode::Ttt => Prompt::new(
                LABEL,
                match self.tangents.len() {
                    0 => "birinci teğet çizgi, yay ya da daireyi seçin",
                    1 => "ikinci teğet nesneyi seçin",
                    2 => "üçüncü teğet nesneyi seçin",
                    _ => "",
                },
            ),
            Mode::Center if n == 0 => Prompt::new(LABEL, "merkez noktasını belirtin")
                .option("2 nokta", "2N")
                .option("3 nokta", "3N")
                .option("Teğet-teğet-yarıçap", "TTY")
                .option("Teğet-teğet-teğet", "TTT"),
            Mode::Center if self.diameter => {
                Prompt::new(LABEL, "çapı gösterin ya da yazın").option("Yarıçap", "R")
            }
            Mode::Center => Prompt::new(LABEL, "yarıçapı gösterin ya da yazın").option("Çap", "Ç"),
        }
    }

    fn point_count(&self) -> usize {
        self.d.pts.len()
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.see(cx);
        Flow::Stay
    }

    /// No snaps while objects are picked for a tangent circle (the web's `snaps`).
    fn snaps(&self) -> bool {
        !self.tangent_modes()
    }

    fn snap_from(&self) -> Option<Vec2> {
        self.d.last()
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.d.hover = Some(self.d.constrain(p, cx));
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if !self.tangent_modes() {
            let p = self.d.constrain(p, cx);
            self.accept(p, cx);
        } else if self.tangents.len() < if self.mode == Mode::Ttt { 3 } else { 2 } {
            self.pick_tangent(p, cx);
        }
        self.see(cx);
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let radius = plain_number(text).filter(|r| *r > 0.0);
        let done = if self.mode == Mode::Ttr && self.tangents.len() == 2 {
            match radius {
                Some(r) => {
                    self.commit_tangent(r, cx);
                    true
                }
                None => false,
            }
        } else if let (Mode::Center, Some(centre), Some(r)) = (self.mode, self.d.last(), radius) {
            self.commit(centre, if self.diameter { r / 2.0 } else { r }, cx);
            true
        } else if self.option(&upper_tr(js_trim(text))) {
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

    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        let last_radius = cx.memory.circle_radius;
        let flow = if self.mode == Mode::Ttr && self.tangents.len() == 2 && last_radius > 0.0 {
            self.commit_tangent(last_radius, cx);
            Flow::Stay
        } else if self.tangent_modes() && !self.tangents.is_empty() {
            self.tangents.clear();
            Flow::Stay
        } else if self.d.pts.is_empty() {
            Flow::Exit
        } else {
            self.d.reset();
            Flow::Stay
        };
        self.see(cx);
        flow
    }

    /// Ctrl+Z (ADR 0018): the circle just written (as an undo), else the
    /// draft starts over; tangent picks stay, as on the web.
    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        let done = self.d.undo_step(false, cx);
        self.see(cx);
        done
    }

    fn preview(&self, format: &Format) -> Preview {
        if self.tangent_modes() {
            let mut preview = Preview {
                squares: self.tangents.iter().map(|t| t.pick).collect(),
                ..Preview::default()
            };
            let last = self.last_radius();
            if self.tangents.len() == 2
                && last > 0.0
                && let Some(round) = self.ttr_circle(last)
            {
                preview.strokes.push(Stroke::dashed(
                    tessellate_circle(round.c, round.r, PREVIEW_SEGMENTS),
                    true,
                    [4.0, 3.0],
                ));
            }
            return preview;
        }
        let (Some(h), Some(&a)) = (self.d.hover, self.d.pts.first()) else {
            return points::chain_preview(&self.d, format);
        };
        let b = self.d.pts.get(1).copied();
        let round = match (self.mode, b) {
            (Mode::Center, _) => Some(Round {
                c: a,
                r: if self.diameter {
                    dist(a, h) / 2.0
                } else {
                    dist(a, h)
                },
            }),
            (Mode::Two, _) => Some(circle_on_diameter(a, h)),
            (_, Some(b)) => circle_through(a, b, h),
            _ => None,
        };
        let Some(round) = round else {
            return points::chain_preview(&self.d, format);
        };
        let from = if self.mode == Mode::Center {
            a
        } else {
            round.c
        };
        Preview {
            strokes: vec![
                Stroke::solid(tessellate_circle(round.c, round.r, PREVIEW_SEGMENTS), true),
                Stroke::dashed(vec![from, h], false, [3.0, 3.0]),
            ],
            tag: Some(Tag {
                at: h,
                lines: vec![format!("r {}", format.length(round.r))],
            }),
            ..Preview::default()
        }
    }
}
