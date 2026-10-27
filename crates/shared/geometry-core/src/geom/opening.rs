//! Openings in walls (docs/adr/0341 §2): a door's, window's or bare opening's interval along its wall's base line, its
//! jambs drawn from the wall, and its symbol drawn from its jambs.
//!
//! An opening lies on one edge of its wall's path, within its ends. Its jambs are the path's point and left normal at
//! the interval's ends, out to the wall's left and right face (`geom::wall::left_face`). The wall's lines and bands are
//! cut between them by the join solver (`geom::wall_join`); the opening draws its frame, leaves and swing in the frame
//! its jambs make (on an arc, along the chord between the jambs' middles).
//!
//! A niche and a flue (docs/adr/0719) take only part of the wall's thickness: a niche from its face (`side`) `depth`
//! in, a flue `inset` from its face (absent: centred) and `depth` across ([`Hole`], [`Recess`]). A bare opening, a
//! niche and a flue are cut in plan only where they cross the plan's cut height [`PLAN_CUT`] ([`cut_in_plan`]); one that
//! is not leaves its wall whole and draws its outline dashed ([`pieces`]).

use crate::api::json::{FromJson, Json, ToJson, field};
use crate::entity::Shape;
use crate::geom::wall::{self, Spec as WallSpec, path_shape};
use crate::jsmath::{atan2, cos, js_hypot, js_max, js_min, sin};
use crate::ops::edges::edge_length;
use crate::ops::path::{Path, path_of, point_at_s, tangent_at_s};
use crate::vec2::Vec2;

/// How far past a vertex or an end an interval may reach and still be on one edge, metres.
pub const SPAN_TOLERANCE: f64 = 1e-9;

/// The plan's cut height above the floor, metres (docs/adr/0719 §2; the contract's `PLAN_CUT_HEIGHT`).
pub const PLAN_CUT: f64 = 1.1;

/// What an opening is (the contract's `OpeningForm`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Form {
    #[default]
    Door,
    Window,
    Opening,
    /// Niş: cut from one face, `depth` in.
    Niche,
    /// Baca: a cavity inside the wall.
    Flue,
}

/// How its leaves move (the contract's `OpeningOperation`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Operation {
    #[default]
    Swing,
    Slide,
    Fixed,
}

/// Which point of the opening `at` names.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Anchor {
    Start,
    #[default]
    Center,
    End,
}

/// An opening's fields beside its place along its wall and its jambs: what its symbol is drawn from. Its wall, sill,
/// head, style, number and break are the contract's and pass through untouched.
#[derive(Clone, Debug, PartialEq)]
pub struct Spec {
    pub anchor: Anchor,
    pub form: Form,
    pub leaves: u8,
    pub operation: Operation,
    /// Its hinge at its end (else at its start).
    pub hand_end: bool,
    /// It opens to its wall's right (else its left).
    pub side_right: bool,
    /// Degrees.
    pub swing: f64,
    pub show_swing: bool,
    pub frame: f64,
    /// Its sill and head above the floor, metres; none: the floor, full height.
    pub sill: Option<f64>,
    pub head: Option<f64>,
    /// A niche's depth from its face, a flue's size across its wall (docs/adr/0719).
    pub depth: Option<f64>,
    /// A flue's distance from its face; none: centred.
    pub inset: Option<f64>,
}

impl Default for Spec {
    fn default() -> Self {
        Spec {
            anchor: Anchor::Center,
            form: Form::Door,
            leaves: 1,
            operation: Operation::Swing,
            hand_end: false,
            side_right: false,
            swing: 90.0,
            show_swing: true,
            frame: 0.0,
            sill: None,
            head: None,
            depth: None,
            inset: None,
        }
    }
}

fn word(v: &Json, name: &str) -> Option<String> {
    match v.get(name) {
        Json::Str(s) => Some(s.clone()),
        _ => None,
    }
}

impl crate::api::json::Flat for Spec {
    const NAMES: &'static [&'static str] = &[
        "anchor",
        "form",
        "leaves",
        "operation",
        "hand",
        "side",
        "swing",
        "showSwing",
        "frame",
        "sill",
        "head",
        "depth",
        "inset",
    ];

    fn read_flat(v: &Json) -> Result<Self, String> {
        let anchor = match word(v, "anchor").as_deref() {
            None | Some("center") => Anchor::Center,
            Some("start") => Anchor::Start,
            Some("end") => Anchor::End,
            Some(s) => return Err(format!("bilinmeyen açıklık dayanağı “{s}”")),
        };
        let form = match word(v, "form").as_deref() {
            None | Some("door") => Form::Door,
            Some("window") => Form::Window,
            Some("opening") => Form::Opening,
            Some("niche") => Form::Niche,
            Some("flue") => Form::Flue,
            Some(s) => return Err(format!("bilinmeyen açıklık biçimi “{s}”")),
        };
        let operation = match word(v, "operation").as_deref() {
            None | Some("swing") => Operation::Swing,
            Some("slide") => Operation::Slide,
            Some("fixed") => Operation::Fixed,
            Some(s) => return Err(format!("bilinmeyen açılış “{s}”")),
        };
        let leaves = Option::<f64>::from_json(v.get("leaves"))?.unwrap_or(1.0);
        Ok(Spec {
            anchor,
            form,
            leaves: if leaves >= 2.0 { 2 } else { 1 },
            operation,
            hand_end: word(v, "hand").as_deref() == Some("end"),
            side_right: word(v, "side").as_deref() == Some("right"),
            swing: Option::<f64>::from_json(v.get("swing"))?.unwrap_or(90.0),
            show_swing: !matches!(v.get("showSwing"), Json::Bool(false)),
            frame: Option::<f64>::from_json(v.get("frame"))?.unwrap_or(0.0),
            sill: Option::<f64>::from_json(v.get("sill"))?,
            head: Option::<f64>::from_json(v.get("head"))?,
            depth: Option::<f64>::from_json(v.get("depth"))?,
            inset: Option::<f64>::from_json(v.get("inset"))?,
        })
    }

    fn write_flat(&self, out: &mut String, first: &mut bool) {
        match self.anchor {
            Anchor::Center => {}
            Anchor::Start => field(out, first, "anchor", "start"),
            Anchor::End => field(out, first, "anchor", "end"),
        }
        let form = match self.form {
            Form::Door => "door",
            Form::Window => "window",
            Form::Opening => "opening",
            Form::Niche => "niche",
            Form::Flue => "flue",
        };
        field(out, first, "form", form);
        if self.leaves != 1 {
            field(out, first, "leaves", &f64::from(self.leaves));
        }
        match self.operation {
            Operation::Swing => {}
            Operation::Slide => field(out, first, "operation", "slide"),
            Operation::Fixed => field(out, first, "operation", "fixed"),
        }
        if self.hand_end {
            field(out, first, "hand", "end");
        }
        if self.side_right {
            field(out, first, "side", "right");
        }
        if self.swing != 90.0 {
            field(out, first, "swing", &self.swing);
        }
        if !self.show_swing {
            field(out, first, "showSwing", &false);
        }
        if self.frame != 0.0 {
            field(out, first, "frame", &self.frame);
        }
        for (name, v) in [("sill", self.sill), ("head", self.head), ("depth", self.depth), ("inset", self.inset)] {
            if let Some(v) = v {
                field(out, first, name, &v);
            }
        }
    }
}

impl ToJson for Spec {
    fn write_json(&self, out: &mut String) {
        out.push('{');
        let mut first = true;
        crate::api::json::Flat::write_flat(self, out, &mut first);
        out.push('}');
    }
}

/// The opening's interval along its wall's base line.
pub fn span(at: f64, anchor: Anchor, width: f64) -> (f64, f64) {
    match anchor {
        Anchor::Start => (at, at + width),
        Anchor::Center => (at - width / 2.0, at + width / 2.0),
        Anchor::End => (at - width, at),
    }
}

/// Whether plan draws the opening cut (docs/adr/0719 §2): a door or a window always; a bare opening, a niche or a flue
/// only when it crosses [`PLAN_CUT`] (its sill none: the floor; its head none: full height).
pub fn cut_in_plan(spec: &Spec) -> bool {
    match spec.form {
        Form::Door | Form::Window => true,
        Form::Opening | Form::Niche | Form::Flue => {
            spec.sill.unwrap_or(0.0) < PLAN_CUT && spec.head.is_none_or(|h| h > PLAN_CUT)
        }
    }
}

/// How far into its wall a niche or a flue goes (docs/adr/0719 §3).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Recess {
    /// From the wall's right face; else its left.
    pub right: bool,
    /// From that face to where it starts; none: centred in the wall (a flue).
    pub inset: Option<f64>,
    /// How deep, across the wall.
    pub depth: f64,
}

impl Recess {
    /// Its near and far distance from its face in a wall `thickness` thick, each within the wall.
    pub fn across(&self, thickness: f64) -> (f64, f64) {
        let near = js_max(
            0.0,
            js_min(self.inset.unwrap_or((thickness - self.depth) / 2.0), thickness),
        );
        (near, js_min(near + js_max(self.depth, 0.0), thickness))
    }
}

/// What an opening takes out of its wall in plan (docs/adr/0341 §2, 0719 §3): its interval along the base line and,
/// for a niche or a flue, how far into the wall; none goes through.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hole {
    pub s0: f64,
    pub s1: f64,
    pub recess: Option<Recess>,
}

impl Hole {
    /// One that goes through the wall on `[s0, s1]`.
    pub fn through(s0: f64, s1: f64) -> Hole {
        Hole { s0, s1, recess: None }
    }
}

/// What the opening at `at` of `width` takes out of its wall in plan; none when plan does not cut it (or a niche or
/// flue without its depth).
pub fn hole(at: f64, width: f64, spec: &Spec) -> Option<Hole> {
    if !cut_in_plan(spec) {
        return None;
    }
    let (s0, s1) = span(at, spec.anchor, width);
    let recess = match spec.form {
        Form::Niche => Some(Recess {
            right: spec.side_right,
            inset: Some(0.0),
            depth: spec.depth?,
        }),
        Form::Flue => Some(Recess {
            right: spec.side_right,
            inset: spec.inset,
            depth: spec.depth?,
        }),
        Form::Door | Form::Window | Form::Opening => None,
    };
    Some(Hole { s0, s1, recess })
}

/// The `at` that puts an opening of `width` centred at `s` given its anchor.
pub fn at_of_centre(s: f64, anchor: Anchor, width: f64) -> f64 {
    match anchor {
        Anchor::Start => s - width / 2.0,
        Anchor::Center => s,
        Anchor::End => s + width / 2.0,
    }
}

/// A wall's base line as a path.
pub fn wall_path(pts: &[Vec2], bulges: Option<&[f64]>, closed: bool) -> Option<Path> {
    path_of(&path_shape(pts, bulges, closed))
}

/// The edge of `path` the interval lies on, or none when it runs past an end or round a vertex.
pub fn edge_of(path: &Path, s0: f64, s1: f64) -> Option<usize> {
    let tol = SPAN_TOLERANCE * js_max(path.length, 1.0);
    if !(s0.is_finite() && s1.is_finite()) || s1 <= s0 || s0 < -tol || s1 > path.length + tol {
        return None;
    }
    (0..path.edges.len()).find(|&i| {
        let a = path.cum[i];
        let b = a + edge_length(&path.edges[i]);
        s0 >= a - tol && s1 <= b + tol
    })
}

/// The path's point `d` to its left at `s`.
pub fn beside(path: &Path, s: f64, d: f64) -> Vec2 {
    let p = point_at_s(path, s);
    let t = tangent_at_s(path, s);
    Vec2::new(p.x - t.y * d, p.y + t.x * d)
}

/// The jambs of an opening on `[s0, s1]` of a wall: the start jamb's points on its left and right face, then the
/// end jamb's; none when the interval is not on one edge within the wall's ends.
pub fn jambs(path: &Path, spec: &WallSpec, s0: f64, s1: f64) -> Option<[Vec2; 4]> {
    edge_of(path, s0, s1)?;
    let left = wall::left_face(spec);
    let right = left - spec.width;
    Some([
        beside(path, s0, left),
        beside(path, s0, right),
        beside(path, s1, left),
        beside(path, s1, right),
    ])
}

fn add(a: Vec2, b: Vec2) -> Vec2 {
    Vec2::new(a.x + b.x, a.y + b.y)
}

fn sub(a: Vec2, b: Vec2) -> Vec2 {
    Vec2::new(a.x - b.x, a.y - b.y)
}

fn mul(a: Vec2, k: f64) -> Vec2 {
    Vec2::new(a.x * k, a.y * k)
}

fn mid(a: Vec2, b: Vec2) -> Vec2 {
    Vec2::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0)
}

fn rotated(v: Vec2, deg: f64) -> Vec2 {
    let r = deg * crate::jsmath::PI / 180.0;
    let (c, s) = (cos(r), sin(r));
    Vec2::new(v.x * c - v.y * s, v.x * s + v.y * c)
}

fn line(a: Vec2, b: Vec2) -> Shape {
    Shape::Line { a, b }
}

fn closed_ring(pts: Vec<Vec2>) -> Shape {
    Shape::Polyline {
        pts,
        bulges: None,
        holes: None,
        parts: None,
        closed: true,
        widths: None,
    }
}

/// The frame its jambs make: the start jamb's middle, the unit along it, the unit to its wall's left, its length
/// between the jambs' middles and its wall's thickness; none when the jambs are one place.
pub struct Frame {
    pub origin: Vec2,
    pub along: Vec2,
    pub left: Vec2,
    pub length: f64,
    pub thickness: f64,
}

pub fn frame_of(place: &[Vec2]) -> Option<Frame> {
    if place.len() != 4 {
        return None;
    }
    let (m0, m1) = (mid(place[0], place[1]), mid(place[2], place[3]));
    let d = sub(m1, m0);
    let length = js_hypot(d.x, d.y);
    if !(length > 0.0) {
        return None;
    }
    let along = mul(d, 1.0 / length);
    let left = Vec2::new(-along.y, along.x);
    let t = sub(place[0], place[1]);
    Some(Frame {
        origin: m0,
        along,
        left,
        length,
        thickness: js_hypot(t.x, t.y),
    })
}

/// One of an opening's pieces, dashed when plan does not cut it (docs/adr/0719 §2).
#[derive(Clone, Debug, PartialEq)]
pub struct Mark {
    pub shape: Shape,
    pub dashed: bool,
}

/// Where the jambs' lines meet: an arc wall's centre; none on a straight one.
fn jambs_centre(place: &[Vec2]) -> Option<Vec2> {
    let (p, u) = (place[0], sub(place[1], place[0]));
    let (q, v) = (place[2], sub(place[3], place[2]));
    let den = u.x * v.y - u.y * v.x;
    let scale = js_hypot(u.x, u.y) * js_hypot(v.x, v.y);
    if !(den.abs() > 1e-12 * scale) {
        return None;
    }
    let w = sub(q, p);
    let t = (w.x * v.y - w.y * v.x) / den;
    Some(add(p, mul(u, t)))
}

/// From `a` to `b` round `centre` (the shorter way), or straight.
fn along(a: Vec2, b: Vec2, centre: Option<Vec2>) -> Shape {
    let Some(c) = centre else {
        return line(a, b);
    };
    let r = js_hypot(a.x - c.x, a.y - c.y);
    let (aa, ab) = (atan2(a.y - c.y, a.x - c.x), atan2(b.y - c.y, b.x - c.x));
    let tau = 2.0 * crate::jsmath::PI;
    let ccw = (ab - aa).rem_euclid(tau);
    let (a0, a1) = if ccw <= crate::jsmath::PI { (aa, ab) } else { (ab, aa) };
    Shape::Arc { c, r, a0, a1 }
}

/// The points `near` and `far` from the face along each jamb (the start's, then the end's), the face `right` or left.
fn across_jambs(place: &[Vec2], right: bool, near: f64, far: f64, thickness: f64) -> [Vec2; 4] {
    let (f0, o0, f1, o1) = if right {
        (place[1], place[0], place[3], place[2])
    } else {
        (place[0], place[1], place[2], place[3])
    };
    let at = |f: Vec2, o: Vec2, d: f64| add(f, mul(sub(o, f), d / thickness));
    [at(f0, o0, near), at(f0, o0, far), at(f1, o1, near), at(f1, o1, far)]
}

/// The opening's pieces from its jambs (docs/adr/0341 §2, 0719 §2): a door's or window's symbol; a cut flue's cross; a
/// bare opening, a niche or a flue plan does not cut drawn dashed (an opening its jambs and a cross, a niche its
/// sides and back, a flue its cavity and a cross); a cut bare opening or niche nothing (its wall shows it).
pub fn pieces(place: &[Vec2], spec: &Spec) -> Vec<Mark> {
    if matches!(spec.form, Form::Door | Form::Window) {
        return symbol(place, spec)
            .into_iter()
            .map(|shape| Mark { shape, dashed: false })
            .collect();
    }
    let Some(f) = frame_of(place) else {
        return Vec::new();
    };
    let cut = cut_in_plan(spec);
    let mark = |shape: Shape| Mark { shape, dashed: !cut };
    let t = f.thickness;
    let centre = jambs_centre(place);
    match spec.form {
        Form::Opening if cut => Vec::new(),
        Form::Opening => vec![
            mark(line(place[0], place[1])),
            mark(line(place[2], place[3])),
            mark(line(place[0], place[3])),
            mark(line(place[1], place[2])),
        ],
        Form::Niche => {
            if cut {
                return Vec::new();
            }
            let Some(d) = spec.depth else { return Vec::new() };
            let [a0, b0, a1, b1] = across_jambs(place, spec.side_right, 0.0, js_min(d, t), t);
            vec![mark(line(a0, b0)), mark(along(b0, b1, centre)), mark(line(b1, a1))]
        }
        Form::Flue => {
            let Some(d) = spec.depth else { return Vec::new() };
            let (near, far) = Recess {
                right: spec.side_right,
                inset: spec.inset,
                depth: d,
            }
            .across(t);
            let [a0, b0, a1, b1] = across_jambs(place, spec.side_right, near, far, t);
            let mut out = Vec::new();
            if !cut {
                out.push(mark(line(a0, b0)));
                out.push(mark(along(b0, b1, centre)));
                out.push(mark(line(b1, a1)));
                out.push(mark(along(a1, a0, centre)));
            }
            out.push(mark(line(a0, b1)));
            out.push(mark(line(b0, a1)));
            out
        }
        Form::Door | Form::Window => Vec::new(),
    }
}

/// A door's or window's own pieces from its jambs (docs/adr/0341 §2): its frame, its leaves and their swing, a window's
/// lines; the other forms' pieces are [`pieces`]'.
pub fn symbol(place: &[Vec2], spec: &Spec) -> Vec<Shape> {
    if !matches!(spec.form, Form::Door | Form::Window) {
        return pieces(place, spec).into_iter().map(|m| m.shape).collect();
    }
    let Some(f) = frame_of(place) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let fr = spec.frame.clamp(0.0, f.length / 2.0 * 0.999);
    let (l0, r0, l1, r1) = (place[0], place[1], place[2], place[3]);
    let u = f.along;
    if fr > 0.0 {
        out.push(closed_ring(vec![l0, add(l0, mul(u, fr)), add(r0, mul(u, fr)), r0]));
        out.push(closed_ring(vec![l1, sub(l1, mul(u, fr)), sub(r1, mul(u, fr)), r1]));
    }
    let clear = f.length - 2.0 * fr;
    let m0 = add(f.origin, mul(u, fr));
    match spec.form {
        Form::Opening | Form::Niche | Form::Flue => {}
        Form::Window => {
            out.push(line(add(l0, mul(u, fr)), sub(l1, mul(u, fr))));
            out.push(line(add(r0, mul(u, fr)), sub(r1, mul(u, fr))));
            if spec.operation == Operation::Slide {
                let k = f.thickness / 12.0;
                let lap = clear * 0.05;
                let half = clear / 2.0;
                out.push(line(add(m0, mul(f.left, k)), add(m0, add(mul(u, half + lap), mul(f.left, k)))));
                out.push(line(
                    add(m0, add(mul(u, half - lap), mul(f.left, -k))),
                    add(m0, add(mul(u, clear), mul(f.left, -k))),
                ));
            } else {
                out.push(line(m0, add(m0, mul(u, clear))));
            }
        }
        Form::Door => match spec.operation {
            Operation::Slide => {
                let k = f.thickness / 8.0;
                if spec.leaves == 1 {
                    out.push(line(add(m0, mul(f.left, k)), add(m0, add(mul(u, clear), mul(f.left, k)))));
                } else {
                    let lap = clear * 0.05;
                    let half = clear / 2.0;
                    out.push(line(add(m0, mul(f.left, k)), add(m0, add(mul(u, half + lap), mul(f.left, k)))));
                    out.push(line(
                        add(m0, add(mul(u, half - lap), mul(f.left, -k))),
                        add(m0, add(mul(u, clear), mul(f.left, -k))),
                    ));
                }
            }
            Operation::Fixed => {
                out.push(line(m0, add(m0, mul(u, clear))));
            }
            Operation::Swing => {
                let (face0, face1) = if spec.side_right { (r0, r1) } else { (l0, l1) };
                let outward = if spec.side_right { mul(f.left, -1.0) } else { f.left };
                let leaf = |hinge: Vec2, closed: Vec2, len: f64, out: &mut Vec<Shape>| {
                    // The leaf turns from where it shuts toward the side it opens to.
                    let turn = if closed.x * outward.y - closed.y * outward.x >= 0.0 {
                        spec.swing
                    } else {
                        -spec.swing
                    };
                    let open = rotated(closed, turn);
                    out.push(line(hinge, add(hinge, mul(open, len))));
                    if spec.show_swing {
                        let a_closed = atan2(closed.y, closed.x);
                        let a_open = atan2(open.y, open.x);
                        let (a0, a1) = if turn > 0.0 {
                            (a_closed, a_open)
                        } else {
                            (a_open, a_closed)
                        };
                        out.push(Shape::Arc {
                            c: hinge,
                            r: len,
                            a0,
                            a1,
                        });
                    }
                };
                if spec.leaves == 2 {
                    let len = clear / 2.0;
                    leaf(add(face0, mul(u, fr)), u, len, &mut out);
                    leaf(sub(face1, mul(u, fr)), mul(u, -1.0), len, &mut out);
                } else if spec.hand_end {
                    leaf(sub(face1, mul(u, fr)), mul(u, -1.0), clear, &mut out);
                } else {
                    leaf(add(face0, mul(u, fr)), u, clear, &mut out);
                }
            }
        },
    }
    out
}

/// The point the opening's jambs are centred on (their four points' mean).
pub fn centre(place: &[Vec2]) -> Option<Vec2> {
    (place.len() == 4).then(|| {
        Vec2::new(
            (place[0].x + place[1].x + place[2].x + place[3].x) / 4.0,
            (place[0].y + place[1].y + place[2].y + place[3].y) / 4.0,
        )
    })
}

/// An opening read back from jambs a step moved by itself (docs/adr/0341 §3: Taşı, Döndür, Aynala, a grip): its `at`
/// on the wall's path (the jambs' centre's nearest place), and whether its hand and its side turned (the jambs run
/// against the path, the left points fell on its right). None for jambs that are not four.
pub struct ReadBack {
    pub at: f64,
    pub hand_turned: bool,
    pub side_turned: bool,
}

/// [`ReadBack`] of `place` on a wall's `path` for an opening of `width` anchored at `anchor`.
pub fn read_back(path: &Path, place: &[Vec2], anchor: Anchor, width: f64) -> Option<ReadBack> {
    let m = centre(place)?;
    let s = crate::ops::path::nearest_s(path, m);
    let p = point_at_s(path, s);
    let t = tangent_at_s(path, s);
    let n = Vec2::new(-t.y, t.x);
    let run = sub(mid(place[2], place[3]), mid(place[0], place[1]));
    let dot = |a: Vec2, b: Vec2| a.x * b.x + a.y * b.y;
    let left = dot(sub(mid(place[0], place[2]), p), n);
    let right = dot(sub(mid(place[1], place[3]), p), n);
    Some(ReadBack {
        at: at_of_centre(s, anchor, width),
        hand_turned: dot(run, t) < 0.0,
        side_turned: left < right,
    })
}
