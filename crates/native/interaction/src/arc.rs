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
//! The ribbon's methods (docs/adr/0287) give the whole construction before the
//! first point, AutoCAD's eleven: `BMS` başlangıç, merkez, bitiş; `BMA` …,
//! açı; `BMU` …, uzunluk (the chord); `BSA` başlangıç, bitiş, açı; `BSY` …,
//! yön; `BSR` …, yarıçap; `MBA` merkez, başlangıç, açı; `MBU` …, uzunluk
//! (M is merkez, başlangıç, bitiş; D Devam). A method stays for the next
//! arcs while the tool runs; an option letter changes it for the arc at hand.
//!
//! Arcs are stored counter-clockwise; a clockwise pick gives the same curve.
//! Every arc is written through `cad.arc.create`, its own object and undo
//! step, and says “Yay eklendi: r = …, açı …°”. Every construction and
//! preview is `cad.curves.construct`'s (docs/adr/0289): a construction that
//! gives no arc warns with the command's words and keeps the points. Devam
//! goes on from where the newest line, arc or polyline's way ends: an arc
//! given clockwise ends at its stored start (`Memory::arc_drawn_back`).

use kentos_commands::{ArcCreate, CurveConstruction, Entity, MeasuredPoint, ObjectEnd, SubPlace};
use kentos_geometry_core::entity::tessellate_circle;
use kentos_geometry_core::geom::arc::{
    ArcGeom, DEFAULT_STEP, norm_angle, point_on_circle, tessellate_arc,
};
use kentos_geometry_core::geometry::dist;
use kentos_geometry_core::jsmath::{PI, atan2};
use kentos_geometry_core::tools::drawing::{end_tangent, start_tangent};
use kentos_geometry_core::tools::point_text::js_trim;
use kentos_native_application::curves_construct::{arc_geom, curve, curve_at};
use kentos_native_application::{ExecutionContext, arc};

use crate::Vec2;
use crate::format::{Format, degrees, fixed};
use crate::log::Level;
use crate::points::{self, Taken, wire};
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
    /// Devam: the object it goes on from and which of its ends (docs/adr/0289).
    cont_from: Option<ObjectEnd>,
    /// The ribbon's method (docs/adr/0287): the mode and the third input it
    /// gives every arc from its first point.
    method: Option<(Mode, Sub)>,
}

impl Default for Arc {
    fn default() -> Self {
        Self {
            d: Taken::default(),
            mode: Mode::Three,
            sub: Sub::End,
            cont_dir: None,
            cont_from: None,
            method: None,
        }
    }
}

fn at(p: Vec2) -> MeasuredPoint {
    MeasuredPoint::At { at: wire(p) }
}

/// Degrees typed into radians, as the core turned them.
fn radians(degrees: f64) -> f64 {
    (degrees * PI) / 180.0
}

/// The ribbon's methods by their options (docs/adr/0287): what the arc is
/// given after its first point, or from it (merkez first).
pub const METHODS: [(&str, &str); 8] = [
    ("BMS", "Başlangıç, merkez, bitiş"),
    ("BMA", "Başlangıç, merkez, açı"),
    ("BMU", "Başlangıç, merkez, uzunluk"),
    ("BSA", "Başlangıç, bitiş, açı"),
    ("BSY", "Başlangıç, bitiş, yön"),
    ("BSR", "Başlangıç, bitiş, yarıçap"),
    ("MBA", "Merkez, başlangıç, açı"),
    ("MBU", "Merkez, başlangıç, uzunluk"),
];

fn method_of(key: &str) -> Option<(Mode, Sub)> {
    Some(match key {
        "BMS" => (Mode::StartCenter, Sub::End),
        "BMA" => (Mode::StartCenter, Sub::Angle),
        "BMU" => (Mode::StartCenter, Sub::Chord),
        "BSA" => (Mode::StartEnd, Sub::Angle),
        "BSY" => (Mode::StartEnd, Sub::Direction),
        "BSR" => (Mode::StartEnd, Sub::Radius),
        "MBA" => (Mode::CenterStart, Sub::Angle),
        "MBU" => (Mode::CenterStart, Sub::Chord),
        _ => return None,
    })
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

    /// What the third input, a point, gives in the current mode: `cad.curves.construct`'s construction
    /// (docs/adr/0289). Devam's names its object; `preview` asks for the same arc by its start and direction.
    fn construction_for(&self, p: Vec2, preview: bool) -> Option<CurveConstruction> {
        let p0 = *self.d.pts.first()?;
        Some(match self.mode {
            Mode::Continue => match (&self.cont_from, preview) {
                (Some(from), false) => CurveConstruction::ArcContinue {
                    from: from.clone(),
                    end: at(p),
                },
                _ => {
                    let dir = self.cont_dir?;
                    CurveConstruction::ArcStartEndDirection {
                        start: at(p0),
                        end: at(p),
                        direction: atan2(dir.y, dir.x),
                    }
                }
            },
            Mode::Three => CurveConstruction::ArcThreePoints {
                start: at(p0),
                mid: at(*self.d.pts.get(1)?),
                end: at(p),
            },
            Mode::StartCenter | Mode::CenterStart => {
                self.d.pts.get(1)?;
                let (s, c) = self.start_center();
                if self.sub == Sub::Chord {
                    CurveConstruction::ArcStartCentreChord {
                        start: at(s),
                        centre: at(c),
                        chord: dist(s, p),
                    }
                } else {
                    CurveConstruction::ArcStartCentreEnd {
                        start: at(s),
                        centre: at(c),
                        end: at(p),
                    }
                }
            }
            Mode::StartEnd => {
                let p1 = *self.d.pts.get(1)?;
                match self.sub {
                    Sub::Center => CurveConstruction::ArcStartCentreEnd {
                        start: at(p0),
                        centre: at(p),
                        end: at(p1),
                    },
                    Sub::Direction => CurveConstruction::ArcStartEndDirection {
                        start: at(p0),
                        end: at(p1),
                        direction: atan2(p.y - p0.y, p.x - p0.x),
                    },
                    Sub::Radius => CurveConstruction::ArcStartEndRadius {
                        start: at(p0),
                        end: at(p1),
                        r: dist(p1, p),
                    },
                    // The included angle shown by a direction from the start, measured from east.
                    _ => CurveConstruction::ArcStartEndAngle {
                        start: at(p0),
                        end: at(p1),
                        angle: atan2(p.y - p0.y, p.x - p0.x),
                    },
                }
            }
        })
    }

    /// What a typed number gives after the first two points: an angle in degrees, or a chord or a radius in
    /// the project's unit (docs/adr/0165 §2, 0266).
    fn construction_typed(&self, n: f64, length: f64) -> CurveConstruction {
        let (p0, p1) = (self.d.pts[0], self.d.pts[1]);
        match self.mode {
            Mode::StartCenter | Mode::CenterStart => {
                let (s, c) = self.start_center();
                if self.sub == Sub::Chord {
                    CurveConstruction::ArcStartCentreChord {
                        start: at(s),
                        centre: at(c),
                        chord: length,
                    }
                } else {
                    CurveConstruction::ArcStartCentreAngle {
                        start: at(s),
                        centre: at(c),
                        angle: radians(n),
                    }
                }
            }
            _ => match self.sub {
                Sub::Radius => CurveConstruction::ArcStartEndRadius {
                    start: at(p0),
                    end: at(p1),
                    r: length,
                },
                Sub::Direction => CurveConstruction::ArcStartEndDirection {
                    start: at(p0),
                    end: at(p1),
                    direction: radians(n),
                },
                _ => CurveConstruction::ArcStartEndAngle {
                    start: at(p0),
                    end: at(p1),
                    angle: radians(n),
                },
            },
        }
    }

    /// The arc a point would make, for the preview: the command's answer without the drawing.
    fn arc_for(&self, p: Vec2) -> Option<ArcGeom> {
        curve_at(&self.construction_for(p, true)?)
            .ok()
            .as_ref()
            .and_then(arc_geom)
    }

    /// Where the newest line, arc or polyline's way ends and which way it runs there (a zero-length line is
    /// passed over): where Devam starts, and the object's end it names. An arc Yay was given clockwise ends at
    /// its stored start (docs/adr/0289).
    fn last_end(cx: &Context<'_>) -> Option<(Vec2, Vec2, ObjectEnd)> {
        let back = cx.memory.arc_drawn_back;
        let all: Vec<&Entity> = cx.doc.entities().collect();
        all.into_iter()
            .rev()
            .filter(|e| matches!(e.kind(), "line" | "arc" | "polyline"))
            .find_map(|e| {
                let id = e.base().id;
                let at_start = e.kind() == "arc" && back == Some(id);
                let shape = record(e).3;
                let end = if at_start {
                    start_tangent(&shape)
                } else {
                    end_tangent(&shape)
                }?;
                let uid = cx.doc.uid(kentos_domain::Slot(id))?;
                Some((
                    end.p,
                    end.dir,
                    ObjectEnd {
                        uid: uid.to_string(),
                        place: if at_start {
                            SubPlace::Start
                        } else {
                            SubPlace::End
                        },
                        path: Vec::new(),
                    },
                ))
            })
    }

    /// Option letters by step (the web's `option`); false when the key is none of those now.
    fn option(&mut self, key: &str, cx: &mut Context<'_>) -> bool {
        let n = self.d.pts.len();
        if n == 0
            && let Some(method) = method_of(key)
        {
            self.method = Some(method);
            self.begin_method();
            return true;
        }
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
                Some((p, dir, from)) => {
                    self.mode = Mode::Continue;
                    self.d.pts = vec![p];
                    self.cont_dir = Some(dir);
                    self.cont_from = Some(from);
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
            if let Some(c) = self.construction_for(p, false) {
                self.commit(&c, cx);
            }
        } else {
            self.d.pts.push(p);
            // A method that starts at the start point takes its course from it.
            if self.d.pts.len() == 1
                && self.mode == Mode::Three
                && let Some((mode, sub)) = self.method.filter(|(m, _)| *m != Mode::CenterStart)
            {
                self.mode = mode;
                self.sub = sub;
            }
        }
    }

    /// The method's course before the first point: merkez first takes it at once.
    fn begin_method(&mut self) {
        self.mode = Mode::Three;
        self.sub = Sub::End;
        if let Some((Mode::CenterStart, sub)) = self.method {
            self.mode = Mode::CenterStart;
            self.sub = sub;
        }
    }

    /// Writes the arc `cad.curves.construct` gives for `c` and starts over; when it gives none, warns with
    /// its words and keeps the points.
    fn commit(&mut self, c: &CurveConstruction, cx: &mut Context<'_>) {
        let g = match curve(cx.doc, c) {
            Ok(curve) => match arc_geom(&curve) {
                Some(g) => g,
                None => return,
            },
            Err(why) => {
                cx.say(Level::Warn, why);
                return;
            }
        };
        // Where the user started: the arc's way ends at its stored start when it does not start there.
        let start = match self.mode {
            Mode::CenterStart => self.d.pts.get(1).copied(),
            _ => self.d.pts.first().copied(),
        };
        if let Some(slot) = self.write(&g, cx) {
            let back = start
                .is_some_and(|s| dist(s, point_on_circle(g.c, g.r, g.a0)) > 1e-6 * g.r.max(1.0));
            cx.memory.arc_drawn_back = back.then_some(slot);
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
        self.cont_dir = None;
        self.cont_from = None;
        self.begin_method();
    }

    /// One arc through the product command `cad.arc.create` (docs/adr/0032):
    /// the active layer and the current colour explicit in its input (CMD-07);
    /// its slot when written.
    fn write(&mut self, g: &ArcGeom, cx: &mut Context<'_>) -> Option<u32> {
        let input = ArcCreate {
            line_type: cx.draft.line_type,
            layer_id: cx.doc.layers().active().to_owned(),
            c: wire(g.c),
            r: g.r,
            a0: g.a0,
            a1: g.a1,
            color: cx.draft.color_text(),
            line_weight: cx.draft.line_weight,
            attrs: None,
            expected_revision: None,
            label: None,
            symbol: None,
        };
        let result = arc::execute(&mut ExecutionContext::new(cx.doc), input);
        let written = points::written(result, cx)?;
        self.d.note(written.id, cx);
        Some(written.id)
    }

    fn reset(&mut self) {
        self.cont_dir = None;
        self.cont_from = None;
        self.begin_method();
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
            && let Some((n, length)) = crate::points::plain_value(text, cx)
        {
            // A chord or a radius is a length typed in the project's unit (docs/adr/0165 §2), or with its own
            // (docs/adr/0266); an angle is not.
            let c = self.construction_typed(n, length);
            self.commit(&c, cx);
            return true;
        }
        if self.option(&upper_tr(js_trim(text)), cx) {
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
