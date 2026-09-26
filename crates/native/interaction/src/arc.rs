//! Yay: the web's `ArcTool` (`apps/web/src/tools/curveTools.ts`) on its
//! `PointInputTool` base, step for step (docs/adr/0032). Every AutoCAD arc
//! construction, chosen with the options as the arc is given:
//!
//! - üç nokta (the default): start, a point on the arc, end;
//! - başlangıç–merkez (M after the start): then the end, Açı (A, a typed or
//!   pointed included angle) or Kiriş (U, a chord; negative: the major arc);
//! - başlangıç–bitiş (B after the start): then the centre, Açı (A), Yön (Y,
//!   the tangent direction at the start) or Yarıçap (R; negative: major arc);
//! - merkez–başlangıç (M first): the centre, the start, then as above;
//! - Devam (D first): tangent to the newest line, arc or polyline, from its end.
//!
//! Arcs are stored counter-clockwise; a clockwise pick gives the same curve.
//! Every arc is written through `cad.arc.create`, its own object and undo
//! step, and says “Yay eklendi: r = …, açı …°”; a construction that gives no
//! arc warns and keeps the points. Every construction and preview is the
//! shared core's (`kentos-geometry-core`); none is computed here.

use kentos_contracts::{ArcCreate, Entity};
use kentos_geometry_core::entity::tessellate_circle;
use kentos_geometry_core::geom::arc::{
    ArcGeom, DEFAULT_STEP, arc_through, norm_angle, tessellate_arc,
};
use kentos_geometry_core::geom::shapes::{
    arc_start_center_angle, arc_start_center_chord, arc_start_center_end, arc_start_end_angle,
    arc_start_end_center, arc_start_end_direction, arc_start_end_radius,
};
use kentos_geometry_core::geometry::{angle_deg, dist};
use kentos_geometry_core::jsmath::PI;
use kentos_geometry_core::tools::drawing::{deg_direction, end_tangent};
use kentos_geometry_core::tools::point_text::{js_trim, point_from_text};
use kentos_native_application::{ExecutionContext, arc};

use crate::Vec2;
use crate::format::{Format, degrees, fixed};
use crate::log::Level;
use crate::points::{self, Taken, plain_number, wire};
use crate::prompt::{Prompt, upper_tr};
use crate::spatial::record;
use crate::tool::{Context, Flow, Pointer, Preview, Stroke, Tag, Tool};

/// The arc tool's id: its command is `tool.arc`.
pub const ID: &str = "arc";
pub const LABEL: &str = "Yay";
/// Two points closer than this are the same point.
const SAME: f64 = 1e-9;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Three,
    StartCenter,
    StartEnd,
    CenterStart,
    Continue,
}

/// What the third input means.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Sub {
    End,
    Angle,
    Chord,
    Center,
    Direction,
    Radius,
}

/// The arc tool.
#[derive(Clone, Debug)]
pub struct Arc {
    d: Taken,
    mode: Mode,
    sub: Sub,
    /// Devam: the direction the arc leaves its start in.
    cont_dir: Option<Vec2>,
}

impl Default for Arc {
    fn default() -> Self {
        Self {
            d: Taken::default(),
            mode: Mode::Three,
            sub: Sub::End,
            cont_dir: None,
        }
    }
}

impl Arc {
    pub fn new() -> Self {
        Self::default()
    }

    fn ready(&self) -> bool {
        match self.mode {
            Mode::Continue => self.d.pts.len() == 1,
            _ => self.d.pts.len() == 2,
        }
    }

    /// The start and the centre, whichever order they were given in.
    fn start_center(&self) -> (Vec2, Vec2) {
        let (p0, p1) = (self.d.pts[0], self.d.pts[1]);
        if self.mode == Mode::StartCenter {
            (p0, p1)
        } else {
            (p1, p0)
        }
    }

    /// The arc the third input, a point, would make in the current mode.
    fn arc_for(&self, p: Vec2) -> Option<ArcGeom> {
        let p0 = *self.d.pts.first()?;
        match self.mode {
            Mode::Continue => arc_start_end_direction(p0, p, self.cont_dir?),
            Mode::Three => arc_through(p0, *self.d.pts.get(1)?, p),
            Mode::StartCenter | Mode::CenterStart => {
                self.d.pts.get(1)?;
                let (s, c) = self.start_center();
                if self.sub == Sub::Chord {
                    arc_start_center_chord(s, c, dist(s, p))
                } else {
                    arc_start_center_end(s, c, p)
                }
            }
            Mode::StartEnd => {
                let p1 = *self.d.pts.get(1)?;
                match self.sub {
                    Sub::Center => arc_start_end_center(p0, p1, p),
                    Sub::Direction => {
                        arc_start_end_direction(p0, p1, Vec2::new(p.x - p0.x, p.y - p0.y))
                    }
                    Sub::Radius => arc_start_end_radius(p0, p1, dist(p1, p)),
                    // The included angle shown by a direction from the start, measured from east.
                    _ => arc_start_end_angle(p0, p1, angle_deg(p0, p)),
                }
            }
        }
    }

    /// The end and travel direction of the newest line, arc or polyline (a
    /// zero-length line is passed over): where Devam starts.
    fn last_end(cx: &Context<'_>) -> Option<(Vec2, Vec2)> {
        let all: Vec<&Entity> = cx.doc.entities().collect();
        all.into_iter()
            .rev()
            .filter(|e| matches!(e.kind(), "line" | "arc" | "polyline"))
            .find_map(|e| end_tangent(&record(e).3))
            .map(|end| (end.p, end.dir))
    }

    /// Option letters by step (the web's `option`); false when the key is none of those now.
    fn option(&mut self, key: &str, cx: &mut Context<'_>) -> bool {
        let n = self.d.pts.len();
        match (n, self.mode, key) {
            (0, Mode::Three, "M") => self.mode = Mode::CenterStart,
            (0, Mode::Three, "D") => match Self::last_end(cx) {
                None => {
                    cx.say(
                        Level::Warn,
                        "Devam edilecek bir çizgi, yay ya da çoklu çizgi yok.",
                    );
                    return true;
                }
                Some((p, dir)) => {
                    self.mode = Mode::Continue;
                    self.d.pts = vec![p];
                    self.cont_dir = Some(dir);
                }
            },
            (1, Mode::Three, "M") => self.mode = Mode::StartCenter,
            (1, Mode::Three, "B") => self.mode = Mode::StartEnd,
            (2, Mode::StartCenter | Mode::CenterStart, "A" | "U" | "N") => {
                self.sub = match key {
                    "A" => Sub::Angle,
                    "U" => Sub::Chord,
                    _ => Sub::End,
                };
            }
            (2, Mode::StartEnd, "M" | "A" | "Y" | "R") => {
                self.sub = match key {
                    "M" => Sub::Center,
                    "A" => Sub::Angle,
                    "Y" => Sub::Direction,
                    _ => Sub::Radius,
                };
            }
            _ => return false,
        }
        if self.mode == Mode::StartEnd && n < 2 && self.sub == Sub::End {
            self.sub = Sub::Center;
        }
        true
    }

    fn accept(&mut self, p: Vec2, cx: &mut Context<'_>) {
        self.d.begin(p, cx);
        if self.d.last().is_some_and(|last| dist(last, p) < SAME) {
            return;
        }
        if self.ready() {
            let arc = self.arc_for(p);
            self.commit(arc, cx);
        } else {
            self.d.pts.push(p);
        }
    }

    /// Writes the arc and starts over; with none, warns and keeps the points.
    fn commit(&mut self, g: Option<ArcGeom>, cx: &mut Context<'_>) {
        let Some(g) = g else {
            cx.say(
                Level::Warn,
                "Bu değerlerle yay oluşmuyor (noktalar aynı doğruda ya da yarıçap kiriş için küçük).",
            );
            return;
        };
        if self.write(&g, cx) {
            let format = cx.format();
            let sweep = or_full(norm_angle(g.a1 - g.a0));
            let line = format!(
                "Yay eklendi: r = {}, açı {}°",
                format.length(g.r),
                fixed(degrees(sweep), 4)
            );
            cx.say(Level::Success, line);
        }
        self.d.pts.clear();
        self.mode = Mode::Three;
        self.sub = Sub::End;
        self.cont_dir = None;
    }

    /// One arc through the product command `cad.arc.create` (docs/adr/0032):
    /// the active layer explicit in its input (CMD-07); the desktop has no
    /// current colour.
    fn write(&mut self, g: &ArcGeom, cx: &mut Context<'_>) -> bool {
        let input = ArcCreate {
            layer_id: cx.doc.layers().active().to_owned(),
            c: wire(g.c),
            r: g.r,
            a0: g.a0,
            a1: g.a1,
            color: None,
            attrs: None,
            expected_revision: None,
        };
        let result = arc::execute(&mut ExecutionContext::new(cx.doc), input);
        match points::written(result, cx) {
            Some(written) => {
                self.d.note(written.id, cx);
                true
            }
            None => false,
        }
    }

    fn reset(&mut self) {
        self.mode = Mode::Three;
        self.sub = Sub::End;
        self.cont_dir = None;
        self.d.reset();
    }

    fn third_around_center(&self) -> Prompt {
        match self.sub {
            Sub::Angle => Prompt::new(
                LABEL,
                "yay açısını yazın ya da gösterin (derece, saat yönünün tersine)",
            )
            .option("Bitiş noktası", "N")
            .option("Kiriş", "U"),
            Sub::Chord => Prompt::new(LABEL, "kiriş boyunu yazın ya da gösterin (eksi: büyük yay)")
                .option("Bitiş noktası", "N")
                .option("Açı", "A"),
            _ => Prompt::new(LABEL, "bitiş noktasını belirtin")
                .option("Açı", "A")
                .option("Kiriş", "U"),
        }
    }

    fn third_start_end(&self) -> Prompt {
        match self.sub {
            Sub::Angle => Prompt::new(LABEL, "yay açısını yazın ya da gösterin")
                .option("Merkez", "M")
                .option("Yön", "Y")
                .option("Yarıçap", "R"),
            Sub::Direction => {
                Prompt::new(LABEL, "başlangıçtaki teğet yönünü gösterin ya da açı yazın")
                    .option("Merkez", "M")
                    .option("Açı", "A")
                    .option("Yarıçap", "R")
            }
            Sub::Radius => Prompt::new(LABEL, "yarıçapı gösterin ya da yazın (eksi: büyük yay)")
                .option("Merkez", "M")
                .option("Açı", "A")
                .option("Yön", "Y"),
            _ => Prompt::new(LABEL, "yayın merkezini belirtin")
                .option("Açı", "A")
                .option("Yön", "Y")
                .option("Yarıçap", "R"),
        }
    }
}

/// A sweep of 0 is a full turn (the web's `normAngle(…) || 2π`).
fn or_full(sweep: f64) -> f64 {
    if sweep == 0.0 { 2.0 * PI } else { sweep }
}

impl Tool for Arc {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn prompt(&self) -> Prompt {
        let n = self.d.pts.len();
        match self.mode {
            Mode::Continue => {
                Prompt::new(LABEL, "bitiş noktasını belirtin (son nesneye teğet devam)")
            }
            Mode::StartCenter | Mode::CenterStart if n < 2 => {
                let ask = if self.mode == Mode::StartCenter || n == 0 {
                    "yayın merkezini belirtin"
                } else {
                    "başlangıç noktasını belirtin"
                };
                Prompt::new(LABEL, ask)
            }
            Mode::StartCenter | Mode::CenterStart => self.third_around_center(),
            Mode::StartEnd if n < 2 => Prompt::new(LABEL, "bitiş noktasını belirtin"),
            Mode::StartEnd => self.third_start_end(),
            Mode::Three => match n {
                0 => Prompt::new(LABEL, "başlangıç noktasını belirtin")
                    .option("Merkez", "M")
                    .option("Devam", "D"),
                1 => Prompt::new(LABEL, "yay üzerinde ikinci bir nokta belirtin")
                    .option("Merkez", "M")
                    .option("Bitiş", "B"),
                _ => Prompt::new(LABEL, "bitiş noktasını belirtin"),
            },
        }
    }

    fn point_count(&self) -> usize {
        self.d.pts.len()
    }

    fn snap_from(&self) -> Option<Vec2> {
        self.d.last()
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.d.hover = Some(self.d.constrain(p, cx));
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let p = self.d.constrain(p, cx);
        self.accept(p, cx);
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        if self.ready()
            && self.mode != Mode::Continue
            && self.mode != Mode::Three
            && let Some(n) = plain_number(text)
        {
            let (p0, p1) = (self.d.pts[0], self.d.pts[1]);
            let g = match self.mode {
                Mode::StartCenter | Mode::CenterStart => {
                    let (s, c) = self.start_center();
                    if self.sub == Sub::Chord {
                        arc_start_center_chord(s, c, n)
                    } else {
                        arc_start_center_angle(s, c, n)
                    }
                }
                _ => match self.sub {
                    Sub::Radius => arc_start_end_radius(p0, p1, n),
                    Sub::Direction => arc_start_end_direction(p0, p1, deg_direction(n)),
                    _ => arc_start_end_angle(p0, p1, n),
                },
            };
            self.commit(g, cx);
            return true;
        }
        if self.option(&upper_tr(js_trim(text)), cx) {
            return true;
        }
        match point_from_text(text, self.d.last(), self.d.hover, |_| None) {
            Some(p) => {
                self.accept(p, cx);
                true
            }
            None => false,
        }
    }

    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        if self.d.pts.is_empty() {
            return Flow::Exit;
        }
        self.reset();
        Flow::Stay
    }

    /// Ctrl+Z (ADR 0018): the arc just written (as an undo), else the arc
    /// being given starts over.
    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        if self.d.undo_last_made(cx) {
            return true;
        }
        if self.d.pts.is_empty() {
            return false;
        }
        self.reset();
        true
    }

    fn preview(&self, format: &Format) -> Preview {
        let Some(h) = self.d.hover else {
            return Preview::default();
        };
        if self.ready() {
            let mut preview = Preview::default();
            match self.mode {
                Mode::StartCenter | Mode::CenterStart => {
                    let (_, c) = self.start_center();
                    preview
                        .strokes
                        .push(Stroke::dashed(vec![c, h], false, [3.0, 3.0]));
                }
                Mode::StartEnd if self.sub != Sub::Center => {
                    let from = if self.sub == Sub::Radius {
                        self.d.pts[1]
                    } else {
                        self.d.pts[0]
                    };
                    preview
                        .strokes
                        .push(Stroke::dashed(vec![from, h], false, [3.0, 3.0]));
                }
                _ => {}
            }
            if let Some(arc) = self.arc_for(h) {
                preview
                    .strokes
                    .push(Stroke::solid(tessellate_arc(&arc, DEFAULT_STEP), false).width(1.5));
                let sweep = or_full(norm_angle(arc.a1 - arc.a0));
                preview.tag = Some(Tag {
                    at: h,
                    lines: vec![
                        format!("r {}", format.length(arc.r)),
                        format!("Açı {}°", fixed(degrees(sweep), 2)),
                    ],
                });
            }
            return preview;
        }
        let mut preview = points::chain_preview(&self.d, format);
        if let Some(&p0) = self.d.pts.first()
            && matches!(self.mode, Mode::StartCenter | Mode::CenterStart)
        {
            // Choosing the centre (or the start around a centre): the circle the arc will lie on.
            let (c, on) = if self.mode == Mode::StartCenter {
                (h, p0)
            } else {
                (p0, h)
            };
            preview.strokes.push(Stroke::dashed(
                tessellate_circle(c, dist(c, on), 96.0),
                true,
                [2.0, 4.0],
            ));
        }
        preview
    }
}
