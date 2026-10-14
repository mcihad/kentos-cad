//! Elips: the web's `EllipseTool` (AutoCAD ELLIPSE,
//! `apps/web/src/tools/ellipseTool.ts`) on its `PointInputTool` base, step
//! for step (docs/adr/0057):
//!
//! - both ends of one axis, then the other half-axis: shown (its distance
//!   from the centre) or typed; Döndürme (D) asks for an angle instead, 0 to
//!   89.4° (the other axis is the first one seen tilted by it);
//! - Merkez (M) starts from the centre and one axis end, Eksenden (E) goes
//!   back; Yay (Y) makes an elliptical arc: after the ellipse, its start and
//!   end angles, shown or typed in degrees from the major axis;
//! - the choices stay for the next ellipses while the tool runs.
//!
//! Every ellipse is written through `cad.entities.create`, its own object
//! and undo step (“Ekle”), and says “Elips eklendi: 12.000 × 6.000 m (yarı
//! eksenler)”. The ellipse and its arc's parameters are
//! `cad.curves.construct`'s (docs/adr/0289), which refuses with the words
//! the tool says; the preview's points are the same core's.

use kentos_contracts::{CurveConstruction, EntityGeometry, MeasuredPoint};
use kentos_geometry_core::geom::ellipse::{
    EllipseGeom, ellipse_point, major_length, param_at_polar, tessellate_ellipse,
};
use kentos_geometry_core::geometry::dist;
use kentos_geometry_core::jsmath::{PI, atan2};
use kentos_geometry_core::tools::drawing::ellipse_param_toward;
use kentos_native_application::curves_construct::{curve, curve_at, ellipse_geom};
use kentos_geometry_core::tools::point_input::midpoint;
use kentos_geometry_core::tools::point_text::js_trim;

use crate::Vec2;
use crate::format::Format;
use crate::log::Level;
use crate::points::{self, Taken, wire};
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Flow, Pointer, Preview, Stroke, Tag, Tool};

/// The ellipse tool's id: its command is `tool.ellipse`.
pub const ID: &str = "ellipse";
pub const LABEL: &str = "Elips";

const DEG: f64 = PI / 180.0;
/// Points a turn of a previewed ellipse (the web's `tessellateEllipse` default).
const PER_TURN: f64 = 128.0;

/// The ellipse tool.
#[derive(Clone, Debug, Default)]
pub struct Ellipse {
    d: Taken,
    /// Merkez (M): the first point is the centre.
    from_center: bool,
    /// Yay (Y): an elliptical arc.
    arc: bool,
    /// Döndürme (D): the other axis is asked for as an angle.
    rotation_mode: bool,
    /// The ellipse, fixed while the arc's angles are chosen, and its construction.
    geom: Option<EllipseGeom>,
    base: Option<CurveConstruction>,
    /// The arc's start: its polar angle from the major axis.
    start_t: Option<f64>,
}

/// How the other half-axis was given.
#[derive(Clone, Copy, Debug)]
enum Half {
    /// Its length.
    Other(f64),
    /// The first axis seen tilted by this many radians.
    Rotation(f64),
}

impl Ellipse {
    pub fn new() -> Self {
        Self::default()
    }

    fn option(&mut self, key: &str) -> bool {
        let n = self.d.pts.len();
        match key {
            "M" | "E" if n == 0 && self.geom.is_none() => self.from_center = key == "M",
            "Y" if n == 0 && self.geom.is_none() => self.arc = !self.arc,
            "D" if n == 2 && self.geom.is_none() => self.rotation_mode = true,
            _ => return false,
        }
        true
    }

    /// `cad.curves.construct`'s construction of the ellipse with its other half-axis (docs/adr/0289), whole.
    fn construction(&self, half: Half) -> Option<CurveConstruction> {
        let [p0, p1] = [*self.d.pts.first()?, *self.d.pts.get(1)?];
        let (other, rotation) = match half {
            Half::Other(o) => (Some(o), None),
            Half::Rotation(r) => (None, Some(r)),
        };
        let at = |p: Vec2| MeasuredPoint::At { at: wire(p) };
        Some(if self.from_center {
            CurveConstruction::EllipseCentre {
                centre: at(p0),
                axis_end: at(p1),
                other,
                rotation,
                start_angle: None,
                end_angle: None,
            }
        } else {
            CurveConstruction::EllipseAxis {
                a: at(p0),
                b: at(p1),
                other,
                rotation,
                start_angle: None,
                end_angle: None,
            }
        })
    }

    /// The ellipse for the second axis given as a half-length, for the preview: the command's answer.
    fn ellipse_for(&self, other_half: f64) -> Option<EllipseGeom> {
        curve_at(&self.construction(Half::Other(other_half))?)
            .ok()
            .as_ref()
            .and_then(ellipse_geom)
    }

    fn center(&self) -> Option<Vec2> {
        let [p0, p1] = [*self.d.pts.first()?, *self.d.pts.get(1)?];
        Some(if self.from_center {
            p0
        } else {
            midpoint(p0, p1)
        })
    }

    fn accept(&mut self, p: Vec2, cx: &mut Context<'_>) {
        self.d.begin(p, cx);
        self.on_point(p, cx);
    }

    fn on_point(&mut self, p: Vec2, cx: &mut Context<'_>) {
        if let Some(geom) = self.geom {
            // The polar angle from the major axis, as seen from the centre.
            let polar = atan2(p.y - geom.c.y, p.x - geom.c.x) - atan2(geom.major.y, geom.major.x);
            return self.angle(polar, cx);
        }
        if self.d.last().is_some_and(|last| dist(last, p) < 1e-9) {
            return;
        }
        if self.d.pts.len() < 2 {
            self.d.pts.push(p);
            return;
        }
        if self.rotation_mode {
            return;
        }
        let Some(c) = self.center() else { return };
        self.shape(Half::Other(dist(c, p)), cx);
    }

    /// A whole ellipse is written now; an arc waits for its angles. The command's words when it gives none.
    fn shape(&mut self, half: Half, cx: &mut Context<'_>) {
        let Some(c) = self.construction(half) else {
            return;
        };
        let g = match curve(cx.doc, &c) {
            Ok(curve) => match ellipse_geom(&curve) {
                Some(g) => g,
                None => return,
            },
            Err(why) => {
                cx.say(Level::Warn, why);
                return;
            }
        };
        if self.arc {
            self.geom = Some(g);
            self.base = Some(c);
            self.d.pts.clear();
            return;
        }
        self.commit(g, cx);
    }

    /// An elliptical arc's polar angle (radians from the major axis): its start, then its end, the arc then
    /// written through the command.
    fn angle(&mut self, polar: f64, cx: &mut Context<'_>) {
        let (Some(base), Some(start)) = (&self.base, self.start_t) else {
            self.start_t = Some(polar);
            return;
        };
        let mut c = base.clone();
        if let CurveConstruction::EllipseAxis {
            start_angle,
            end_angle,
            ..
        }
        | CurveConstruction::EllipseCentre {
            start_angle,
            end_angle,
            ..
        } = &mut c
        {
            *start_angle = Some(start);
            *end_angle = Some(polar);
        }
        match curve(cx.doc, &c) {
            Ok(curve) => {
                if let Some(g) = ellipse_geom(&curve) {
                    self.commit(g, cx);
                }
            }
            Err(why) => cx.say(Level::Warn, why),
        }
    }

    /// Writes through `cad.entities.create` (docs/adr/0057): its own object
    /// and undo step, “Ekle”; the draft starts over either way.
    fn commit(&mut self, g: EllipseGeom, cx: &mut Context<'_>) {
        let a = major_length(&g);
        let geometry = EntityGeometry::Ellipse {
            c: wire(g.c),
            major: wire(g.major),
            ratio: g.ratio,
            t0: g.t0,
            t1: g.t1,
        };
        if let Some(out) = points::write_objects(vec![geometry], None, cx) {
            if let Some(&id) = out.ids.first() {
                self.d.note(id, cx);
            }
            let format = cx.format();
            let what = if g.t0 == g.t1 { "Elips" } else { "Eliptik yay" };
            let line = format!(
                "{what} eklendi: {} × {} (yarı eksenler)",
                format.length_bare(a),
                format.length(a * g.ratio)
            );
            cx.say(Level::Success, line);
        }
        self.reset();
    }

    fn reset(&mut self) {
        self.geom = None;
        self.base = None;
        self.start_t = None;
        self.rotation_mode = false;
        self.d.reset();
    }
}

impl Tool for Ellipse {
    /// A point computed by the point calculator, as if clicked (the web's `acceptPoint`).
    fn accepts_points(&self) -> bool {
        true
    }

    fn accept_point(&mut self, p: Vec2, cx: &mut Context<'_>) -> bool {
        self.accept(p, cx);
        true
    }
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn prompt(&self) -> Prompt {
        if self.geom.is_some() {
            return Prompt::new(
                LABEL,
                if self.start_t.is_none() {
                    "başlangıç açısını gösterin ya da yazın (büyük eksenden, derece)"
                } else {
                    "bitiş açısını gösterin ya da yazın"
                },
            );
        }
        match self.d.pts.len() {
            0 => {
                let what = if self.arc { "Eliptik yay: " } else { "" };
                let step = if self.from_center {
                    "elipsin merkezini belirtin"
                } else {
                    "bir eksenin ilk ucunu belirtin"
                };
                let prompt = Prompt::new(LABEL, format!("{what}{step}"));
                let prompt = if self.from_center {
                    prompt.option("Eksenden", "E")
                } else {
                    prompt.option("Merkez", "M")
                };
                prompt.option_with("Yay", "Y", if self.arc { "açık" } else { "kapalı" })
            }
            1 => Prompt::new(
                LABEL,
                if self.from_center {
                    "bir eksenin ucunu belirtin"
                } else {
                    "eksenin diğer ucunu belirtin"
                },
            ),
            _ if self.rotation_mode => {
                Prompt::new(LABEL, "döndürme açısını yazın (0 ile 89.4 derece arası)")
            }
            _ => Prompt::new(LABEL, "diğer yarı ekseni gösterin ya da uzunluğunu yazın")
                .option("Döndürme", "D"),
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
        if self.option(&upper_tr(js_trim(text))) {
            return true;
        }
        let plain = points::plain_number(text);
        if let (Some(n), Some(_)) = (plain, self.geom) {
            self.angle(n * DEG, cx);
            return true;
        }
        if let (Some(n), 2) = (plain, self.d.pts.len()) {
            // Degrees into radians as the command takes them; the command refuses what makes no ellipse.
            let half = if self.rotation_mode {
                Half::Rotation((n * PI) / 180.0)
            } else {
                Half::Other(n)
            };
            self.shape(half, cx);
            return true;
        }
        match cx.typed_point(text, self.d.last(), self.d.hover) {
            Some(p) => {
                self.accept(p, cx);
                true
            }
            None => false,
        }
    }

    /// Enter: the draft starts over; with no point (an arc's angles too), the tool leaves.
    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        if self.d.pts.is_empty() {
            return Flow::Exit;
        }
        self.reset();
        Flow::Stay
    }

    /// Ctrl+Z (ADR 0018): the ellipse just written (as an undo), else the
    /// draft starts over; while an arc's angles are chosen the drawing is
    /// undone, as on the web.
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
        if let Some(geom) = self.geom {
            let t = ellipse_param_toward(&geom, h);
            let start_t = self.start_t.map(|polar| param_at_polar(&geom, polar));
            let mut strokes = vec![
                Stroke::dashed(tessellate_ellipse(&geom, PER_TURN), true, [2.0, 4.0]),
                Stroke::dashed(vec![geom.c, h], false, [3.0, 3.0]),
            ];
            strokes.push(match start_t {
                Some(start) => Stroke::solid(
                    tessellate_ellipse(
                        &EllipseGeom {
                            t0: start,
                            t1: t,
                            ..geom
                        },
                        PER_TURN,
                    ),
                    false,
                )
                .width(1.5),
                None => Stroke::solid(vec![geom.c, ellipse_point(&geom, t)], false),
            });
            return Preview {
                strokes,
                ..Preview::default()
            };
        }
        if let (2, false, Some(c)) = (self.d.pts.len(), self.rotation_mode, self.center()) {
            let mut preview = Preview {
                strokes: vec![Stroke::dashed(vec![c, h], false, [3.0, 3.0])],
                ..Preview::default()
            };
            if let Some(e) = self.ellipse_for(dist(c, h)) {
                preview
                    .strokes
                    .push(Stroke::solid(tessellate_ellipse(&e, PER_TURN), true));
                let a = major_length(&e);
                preview.tag = Some(Tag {
                    at: h,
                    lines: vec![format!(
                        "Yarı eksenler {} × {}",
                        format.length_bare(a),
                        format.length(a * e.ratio)
                    )],
                });
            }
            return preview;
        }
        points::chain_preview(&self.d, format)
    }
}
