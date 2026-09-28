//! Kesişim noktası (docs/adr/0140): a point found from what is known of it,
//! three ways. The ribbon starts a method by sending its letter right after
//! the tool, and the same letters work typed on the command line: `U`, `D`,
//! `L`.
//!
//! - **İki uzaklık** (the default): a known point A and the distance from it,
//!   a known point B and the distance from it. A distance is typed or shown
//!   by clicking on its circle. The circles meet in up to two points: the
//!   click near the wanted one chooses it, Enter takes the one to the right
//!   of A towards B; a single meeting point (touching circles) is taken at
//!   once. Enter at a distance takes the one kept from last time.
//! - **İki doğrultu** (`D`): A and its direction, B and its direction. A
//!   direction is a survey bearing (from grid north, clockwise) typed in the
//!   project's angle unit, or shown by clicking along it. The meeting must
//!   lie ahead of both points.
//! - **İki doğru** (`L`): the four points of two lines, the first pair and
//!   the second; the lines meet wherever they extend to.
//!
//! What is picked so far is drawn live, the meeting too as soon as it exists.
//! A meeting that does not exist is said and the last answer asked again.
//! Esc steps back one answer; a quick right click or Enter with something
//! begun and nothing to take starts over.
//!
//! The point goes on the active layer through `cad.point.create`, one undo
//! step (“Ekle”); the message says its Y and X. The meetings are the shared
//! core's (`construct::distance_distance`, `bearing_bearing`,
//! `intersect::line_line`).

use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::intersect::line_line;
use kentos_geometry_core::geometry::dist;
use kentos_geometry_core::jsmath::{TAU, atan2, cos, sin};
use kentos_geometry_core::tools::construct::{bearing_bearing, distance_distance};
use kentos_geometry_core::tools::point_text::{js_trim, point_from_text};

use crate::Vec2;
use crate::edge::Outline;
use crate::format::Format;
use crate::log::Level;
use crate::points::{self, Taken, plain_number};
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{
    Context, Flow, Marker, MarkerShape, Memory, Pointer, Preview, Stroke, Tag, Tone, Tool,
};

/// The tool's id: its command is `tool.intersectPoint`.
pub const ID: &str = "intersectPoint";
pub const LABEL: &str = "Kesişim noktası";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Method {
    Distances,
    Bearings,
    Lines,
}

/// What an answer is: a point, or the number that belongs to the point before it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Point,
    Value,
}

use Kind::{Point, Value};

impl Method {
    /// The answers in the order they are given.
    fn sequence(self) -> [Kind; 4] {
        match self {
            Method::Distances | Method::Bearings => [Point, Value, Point, Value],
            Method::Lines => [Point; 4],
        }
    }
}

/// The dash and gap of what is not settled yet, logical pixels.
const DASH: [f32; 2] = [5.0, 3.0];

/// Kesişim noktası.
#[derive(Clone, Debug)]
pub struct IntersectPoint {
    d: Taken,
    method: Method,
    /// The distances (metres) or the bearings (radians from north, clockwise) given.
    vals: Vec<f64>,
    /// İki uzaklık: the two meeting points waiting for the choice.
    cands: Vec<Vec2>,
    /// How far a line is drawn either way, metres, as of the last event.
    reach: f64,
    /// What the session remembered and the project's units, as of the last event.
    seen: Option<(Memory, Format)>,
}

impl IntersectPoint {
    pub fn new() -> Self {
        Self {
            d: Taken::default(),
            method: Method::Distances,
            vals: Vec::new(),
            cands: Vec::new(),
            reach: 1000.0,
            seen: None,
        }
    }

    fn see(&mut self, cx: &Context<'_>) {
        self.seen = Some((*cx.memory, cx.format()));
        self.reach = points::reach(cx.view, 2.0);
    }

    fn memory(&self) -> Memory {
        self.seen.map(|(m, _)| m).unwrap_or_default()
    }

    fn format(&self) -> Format {
        self.seen.map(|(_, f)| f).unwrap_or_default()
    }

    fn taken(&self) -> usize {
        self.d.pts.len() + self.vals.len()
    }

    fn fresh(&self) -> bool {
        self.taken() == 0
    }

    /// What is asked next; none once all is given (the choice, or nothing).
    fn next(&self) -> Option<Kind> {
        if !self.cands.is_empty() {
            return None;
        }
        self.method.sequence().get(self.taken()).copied()
    }

    /// Which of the two points (0 or 1) a value or a point being asked belongs to.
    fn index(&self) -> usize {
        self.taken() / 2
    }

    /// The value the cursor stands for at a value step: a distance from its
    /// point, or the bearing towards the cursor from it.
    fn value_at(&self, at: Vec2) -> Option<f64> {
        let from = *self.d.pts.get(self.vals.len())?;
        Some(match self.method {
            Method::Bearings => bearing(from, at),
            _ => dist(from, at),
        })
    }

    /// The meeting points of the answers so far, the cursor completing the
    /// one being asked.
    fn meetings(&self, hover: Option<Vec2>) -> Vec<Vec2> {
        let mut vals = self.vals.clone();
        let mut pts = self.d.pts.clone();
        match (self.next(), hover) {
            (Some(Value), Some(h)) => match self.value_at(h) {
                Some(v) => vals.push(v),
                None => return Vec::new(),
            },
            (Some(Point), Some(h)) if self.method == Method::Lines => pts.push(h),
            _ => {}
        }
        match self.method {
            Method::Distances => match (pts.get(..2), vals.get(..2)) {
                (Some(p), Some(v)) => distance_distance(p[0], v[0], p[1], v[1]),
                _ => Vec::new(),
            },
            Method::Bearings => match (pts.get(..2), vals.get(..2)) {
                (Some(p), Some(v)) => bearing_bearing(p[0], v[0], p[1], v[1])
                    .into_iter()
                    .collect(),
                _ => Vec::new(),
            },
            Method::Lines => match pts.get(..4) {
                Some(p) => line_line(p[0], p[1], p[2], p[3])
                    .map(|h| h.p)
                    .into_iter()
                    .collect(),
                None => Vec::new(),
            },
        }
    }

    /// A click or a computed point: `world` is where it snapped, `raw` where
    /// the pointer is (the choice between two meetings goes by the pointer).
    fn take(&mut self, world: Vec2, raw: Vec2, cx: &mut Context<'_>) {
        self.see(cx);
        self.take_now(world, raw, cx);
        self.see(cx);
    }

    fn take_now(&mut self, world: Vec2, raw: Vec2, cx: &mut Context<'_>) {
        if !self.cands.is_empty() {
            let chosen = nearest(&self.cands, raw);
            return self.finish(chosen, cx);
        }
        match self.next() {
            Some(Point) => self.point(world, cx),
            Some(Value) => match self.value_at(world) {
                Some(v) if v > points::SAME || self.method == Method::Bearings => {
                    self.value(v, true, cx)
                }
                _ => cx.say(
                    Level::Warn,
                    "Nokta ilk noktayla çakışıyor; başka bir yer gösterin ya da değeri yazın.",
                ),
            },
            None => {}
        }
    }

    fn point(&mut self, p: Vec2, cx: &mut Context<'_>) {
        // A second point on the first makes no line and no second circle centre.
        let pair_start = if self.method == Method::Lines {
            self.d.pts.len() % 2 == 1
        } else {
            self.d.pts.len() == 1
        };
        if pair_start
            && self
                .d
                .last()
                .is_some_and(|last| dist(last, p) < points::SAME)
        {
            cx.say(
                Level::Warn,
                "Nokta öncekiyle çakışıyor; başka bir yer gösterin.",
            );
            return;
        }
        self.d.begin(p, cx);
        self.d.pts.push(p);
        if self.method == Method::Lines && self.d.pts.len() == 4 {
            self.solve_lines(cx);
        }
    }

    /// A distance or a bearing given for the point before it.
    fn value(&mut self, v: f64, echo: bool, cx: &mut Context<'_>) {
        if self.method == Method::Distances {
            cx.memory.meeting_distance = v;
        }
        self.vals.push(v);
        // A typed value is already echoed; a clicked one is read off the drawing and said.
        if echo {
            let f = self.format();
            cx.say(
                Level::Info,
                match self.method {
                    Method::Bearings => format!("  Semt {}", f.angle(v)),
                    _ => format!("  Uzaklık {}", f.length(v)),
                },
            );
        }
        if self.vals.len() == 2 {
            self.solve(cx);
        }
    }

    /// Both answers of İki uzaklık and İki doğrultu are in.
    fn solve(&mut self, cx: &mut Context<'_>) {
        let (a, b) = (self.d.pts[0], self.d.pts[1]);
        let (va, vb) = (self.vals[0], self.vals[1]);
        let f = self.format();
        match self.method {
            Method::Distances => {
                let meets = distance_distance(a, va, b, vb);
                match meets.as_slice() {
                    [] => {
                        cx.say(
                            Level::Warn,
                            format!(
                                "İki uzaklık kesişmiyor: A ile B arası {}, uzaklıklar {} ve {}. Kesişmeleri için toplamları en az {}, farkları en çok {} olmalı; ikinci uzaklığı değiştirin.",
                                f.length(dist(a, b)),
                                f.length(va),
                                f.length(vb),
                                f.length(dist(a, b)),
                                f.length(dist(a, b)),
                            ),
                        );
                        self.vals.pop();
                    }
                    [only] => self.finish(Some(*only), cx),
                    _ => self.cands = meets,
                }
            }
            _ => match bearing_bearing(a, va, b, vb) {
                Some(p) => self.finish(Some(p), cx),
                None => {
                    cx.say(
                        Level::Warn,
                        "Doğrultular kesişmiyor: paralel ya da kesişim noktalardan birinin arkasında kalıyor. İkinci doğrultuyu değiştirin.",
                    );
                    self.vals.pop();
                }
            },
        }
    }

    fn solve_lines(&mut self, cx: &mut Context<'_>) {
        let p = &self.d.pts;
        match line_line(p[0], p[1], p[2], p[3]) {
            Some(hit) => self.finish(Some(hit.p), cx),
            None => {
                cx.say(
                    Level::Warn,
                    "Doğrular paralel; kesişmiyorlar. Dördüncü noktayı değiştirin.",
                );
                self.d.pts.pop();
            }
        }
    }

    /// Writes the point, and asks for the next one.
    fn finish(&mut self, at: Option<Vec2>, cx: &mut Context<'_>) {
        let Some(p) = at else { return };
        if let Some(id) = points::write_point(p, cx) {
            self.d.note(id, cx);
            let f = cx.format();
            cx.say(
                Level::Success,
                format!("Kesişim noktası kondu: {}", f.point(p)),
            );
        }
        self.d.pts.clear();
        self.vals.clear();
        self.cands.clear();
    }

    /// Back one answer; false when nothing was given.
    fn back(&mut self) -> bool {
        if !self.cands.is_empty() {
            self.cands.clear();
            self.vals.pop();
            return true;
        }
        let Some(last) = self.taken().checked_sub(1) else {
            return false;
        };
        match self.method.sequence()[last] {
            Point => {
                self.d.pts.pop();
            }
            Value => {
                self.vals.pop();
            }
        }
        true
    }

    /// The other methods, while nothing is given, as options.
    fn options(&self, prompt: Prompt) -> Prompt {
        if !self.fresh() {
            return prompt;
        }
        let prompt = prompt.then();
        match self.method {
            Method::Distances => prompt.option("İki doğrultu", "D").option("İki doğru", "L"),
            Method::Bearings => prompt.option("İki uzaklık", "U").option("İki doğru", "L"),
            Method::Lines => prompt
                .option("İki uzaklık", "U")
                .option("İki doğrultu", "D"),
        }
    }

    fn switch(&mut self, method: Method) {
        self.method = method;
    }
}

impl Default for IntersectPoint {
    fn default() -> Self {
        Self::new()
    }
}

/// The survey bearing from `from` to `to`, radians from grid north, clockwise.
fn bearing(from: Vec2, to: Vec2) -> f64 {
    atan2(to.x - from.x, to.y - from.y).rem_euclid(TAU)
}

/// Of the points, the one nearest to `at`.
fn nearest(points: &[Vec2], at: Vec2) -> Option<Vec2> {
    points
        .iter()
        .copied()
        .min_by(|p, q| dist(*p, at).total_cmp(&dist(*q, at)))
}

impl Tool for IntersectPoint {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn prompt(&self) -> Prompt {
        let f = self.format();
        let unit = f.angle_unit_name();
        let letter = if self.index() == 0 { "A" } else { "B" };
        // “A'dan”, “B'den”: the suffix follows how the letter is read.
        let from = if self.index() == 0 { "A'dan" } else { "B'den" };
        let whose = if self.index() == 0 { "A'nın" } else { "B'nin" };
        let kept = f.length(self.memory().meeting_distance);
        if !self.cands.is_empty() {
            return Prompt::new(LABEL, "istediğiniz kesişime tıklayın").option("Sağdaki", "Enter");
        }
        let prompt = match (self.method, self.next()) {
            (Method::Lines, _) => {
                let (line, which) = (self.taken() / 2 + 1, ["ilk", "ikinci"][self.taken() % 2]);
                Prompt::new(
                    LABEL,
                    format!("{line}. doğrunun {which} noktasını belirtin"),
                )
            }
            (_, Some(Point)) => Prompt::new(LABEL, format!("{letter} noktasını belirtin")),
            (Method::Distances, _) => Prompt::new(
                LABEL,
                format!("{from} uzaklığı yazın ya da tıklayın (Enter: {kept})"),
            ),
            (_, _) => Prompt::new(
                LABEL,
                format!("{whose} semtini yazın ({unit}) ya da gösterin"),
            ),
        };
        self.options(prompt)
    }

    fn point_count(&self) -> usize {
        self.d.pts.len()
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.see(cx);
        Flow::Stay
    }

    /// Object snaps apply to points; a distance is read off the point, and
    /// the choice between two meetings by the pointer itself.
    fn snaps(&self) -> bool {
        self.cands.is_empty()
    }

    fn snap_from(&self) -> Option<Vec2> {
        self.d.last()
    }

    fn accepts_points(&self) -> bool {
        self.cands.is_empty()
    }

    fn accept_point(&mut self, p: Vec2, cx: &mut Context<'_>) -> bool {
        if !self.cands.is_empty() {
            return false;
        }
        self.take(p, p, cx);
        true
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.see(cx);
        // The choice between two meetings goes by the pointer itself.
        self.d.hover = Some(if !self.cands.is_empty() {
            p.raw
        } else if self.next() == Some(Point) {
            self.d.constrain(p, cx)
        } else {
            p.world
        });
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let world = match self.next() {
            Some(Point) => self.d.constrain(p, cx),
            _ => p.world,
        };
        self.take(world, p.raw, cx);
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        self.see(cx);
        let taken = self.typed(text, cx);
        self.see(cx);
        taken
    }

    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        self.see(cx);
        let flow = self.confirmed(cx);
        self.see(cx);
        flow
    }

    fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
        let back = self.back();
        self.see(cx);
        back
    }

    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        let undone = self.d.undo_last_made(cx) || self.back();
        self.see(cx);
        undone
    }

    fn preview(&self, format: &Format) -> Preview {
        self.drawn(format)
    }
}

impl IntersectPoint {
    /// Typed text: a method letter, a value or a point.
    fn typed(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        if self.fresh() {
            match upper_tr(js_trim(text)).as_str() {
                "U" => return self.switch_to(Method::Distances),
                "D" => return self.switch_to(Method::Bearings),
                "L" => return self.switch_to(Method::Lines),
                _ => {}
            }
        }
        match self.next() {
            Some(Value) => {
                let Some(n) = plain_number(text) else {
                    return false;
                };
                let f = self.format();
                if self.method == Method::Distances {
                    if n <= 0.0 {
                        cx.say(Level::Warn, "Uzaklık sıfırdan büyük olmalı.");
                        return true;
                    }
                    self.value(n, false, cx);
                } else {
                    self.value(f.angle_from_typed(n).rem_euclid(TAU), false, cx);
                }
                true
            }
            Some(Point) => {
                let Some(p) =
                    point_from_text(text, self.d.last(), self.d.hover, |d| cx.track_along(d))
                else {
                    return false;
                };
                self.take(p, p, cx);
                true
            }
            None => false,
        }
    }

    /// The choice: the meeting right of A towards B. A distance: the one kept.
    /// Otherwise, what is begun starts over, and with nothing begun the tool leaves.
    fn confirmed(&mut self, cx: &mut Context<'_>) -> Flow {
        if let [p, q] = self.cands[..] {
            let (a, b) = (self.d.pts[0], self.d.pts[1]);
            let right = |c: Vec2| (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x) < 0.0;
            self.finish(Some(if right(p) { p } else { q }), cx);
            return Flow::Stay;
        }
        if self.method == Method::Distances && self.next() == Some(Value) {
            let kept = self.memory().meeting_distance;
            self.value(kept, true, cx);
            return Flow::Stay;
        }
        if self.fresh() {
            return Flow::Exit;
        }
        self.d.pts.clear();
        self.vals.clear();
        Flow::Stay
    }

    /// What is drawn over the drawing.
    fn drawn(&self, _format: &Format) -> Preview {
        let f = self.format();
        let hover = self.d.hover;
        let mut out = Preview {
            tracking: self.d.tracking,
            ..Preview::default()
        };
        // Where each answer stands: solid once given, dashed under the cursor.
        for (i, &p) in self.d.pts.iter().enumerate() {
            out.markers.push(Marker {
                at: p,
                shape: MarkerShape::Ring(5.0),
                tone: Tone::Snap,
            });
            let given = self.method != Method::Lines && self.vals.len() > i;
            let live =
                self.method != Method::Lines && self.vals.len() == i && self.next() == Some(Value);
            let value = if given {
                Some((self.vals[i], None))
            } else if live {
                hover
                    .and_then(|h| self.value_at(h))
                    .map(|v| (v, Some(DASH)))
            } else {
                None
            };
            let Some((v, dash)) = value else { continue };
            match self.method {
                Method::Distances => {
                    let circle = Outline::of(&Shape::Circle { c: p, r: v }, dash, 1.0, Tone::Snap);
                    out.strokes.extend(circle.strokes);
                    if let (Some(h), true) = (hover, live) {
                        out.strokes
                            .push(Stroke::solid(vec![p, h], false).tone(Tone::Snap));
                        out.tag = Some(Tag {
                            at: h,
                            lines: vec![format!("Uzaklık {}", f.length(v))],
                        });
                    }
                }
                _ => {
                    let far = Vec2::new(p.x + sin(v) * self.reach, p.y + cos(v) * self.reach);
                    let mut stroke = Stroke::solid(vec![p, far], false).tone(Tone::Snap);
                    stroke.dash = dash;
                    out.strokes.push(stroke);
                    if let (Some(h), true) = (hover, live) {
                        out.tag = Some(Tag {
                            at: h,
                            lines: vec![format!("Semt {}", f.angle(v))],
                        });
                    }
                }
            }
        }
        if self.method == Method::Lines {
            let mut pts = self.d.pts.clone();
            if self.next() == Some(Point)
                && let Some(h) = hover
            {
                pts.push(h);
            }
            for (i, pair) in pts.chunks(2).enumerate().filter(|(_, c)| c.len() == 2) {
                let (p, q) = (pair[0], pair[1]);
                let l = dist(p, q).max(1e-9);
                let (ux, uy) = ((q.x - p.x) / l * self.reach, (q.y - p.y) / l * self.reach);
                let far = vec![Vec2::new(p.x - ux, p.y - uy), Vec2::new(p.x + ux, p.y + uy)];
                let given = i * 2 + 2 <= self.d.pts.len();
                out.strokes.push(if given {
                    Stroke::solid(far, false).tone(Tone::Snap)
                } else {
                    Stroke::dashed(far, false, DASH).tone(Tone::Snap)
                });
            }
        }
        // What the answers make: the meetings, the nearer one to the cursor marked.
        let meets = if self.cands.is_empty() {
            self.meetings(hover)
        } else {
            self.cands.clone()
        };
        let near = hover.and_then(|h| nearest(&meets, h));
        for &m in &meets {
            let chosen = near == Some(m);
            out.markers.push(Marker {
                at: m,
                shape: if chosen {
                    MarkerShape::Plus(7.0)
                } else {
                    MarkerShape::Ring(6.0)
                },
                tone: Tone::Accent,
            });
        }
        if let (Some(m), Some(h)) = (near, hover) {
            let mut lines = out.tag.take().map(|t| t.lines).unwrap_or_default();
            lines.push(format!("Y {}", f.coord(m.x)));
            lines.push(format!("X {}", f.coord(m.y)));
            out.tag = Some(Tag { at: h, lines });
        }
        out
    }
}

impl IntersectPoint {
    /// A method chosen by its letter, while nothing is given.
    fn switch_to(&mut self, method: Method) -> bool {
        self.switch(method);
        true
    }
}
