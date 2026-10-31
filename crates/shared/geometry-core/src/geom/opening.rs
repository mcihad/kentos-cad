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
    /// Döner: wings about a drum's middle (docs/adr/0777 §1).
    Revolving,
    /// Cebe giren: leaves into a pocket inside the wall (docs/adr/0777 §2).
    Pocket,
    /// Katlanır: panels folding to a jamb (docs/adr/0777 §3).
    Folding,
}

/// A pocket door's play each side of its leaf in its pocket (the contract's `POCKET_PLAY`), metres.
pub const POCKET_PLAY: f64 = 0.01;
/// How much of an open pocket door's leaf stays out of its pocket when it gives none (`POCKET_LIP`), metres.
pub const POCKET_LIP: f64 = 0.05;
/// A leaf's thickness when it gives none (`DEFAULT_LEAF_THICKNESS`), metres.
pub const LEAF_THICKNESS: f64 = 0.04;
/// How far outside a revolving door's drum its walls stand, metres (docs/adr/0777 §1).
pub const DRUM_WALL: f64 = 0.03;
/// How far plan draws a folding door's panels open when it gives no angle (`FOLD_DRAWN`), degrees.
pub const FOLD_DRAWN: f64 = 30.0;

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
    /// Its construction (docs/adr/0772 §3); the default draws it as before.
    pub build: Build,
}

/// A door's or window's construction as plan draws it (docs/adr/0772 §3; the contract's `OpeningBuild`): each field's
/// absence draws nothing of it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Build {
    /// The fitting gap between the rough opening and the frame, each side.
    pub fit: f64,
    /// A leaf's thickness: a leaf is drawn as a band, else a line.
    pub leaf_thickness: Option<f64>,
    /// Two unequal leaves: the hinge-start leaf's share of the light.
    pub split: Option<f64>,
    /// The frame's depth across the wall; none: the wall's thickness.
    pub frame_depth: Option<f64>,
    /// With a depth: its distance from the face it opens to; none: centred.
    pub frame_offset: Option<f64>,
    pub threshold: Option<f64>,
    pub casing: Option<f64>,
    pub sill_board: Option<f64>,
    pub mullions: u8,
    pub use_zone: Option<f64>,
    /// A revolving door's drum's inside diameter; none: the light.
    pub drum: Option<f64>,
    /// Two sliding leaves' overlap; none: 5 % of the light.
    pub overlap: Option<f64>,
    /// A sliding door's track drawn.
    pub track: bool,
    /// A folding door's panels fold to both jambs.
    pub both_sides: bool,
    /// How much of an open pocket leaf stays out of its pocket; none: [`POCKET_LIP`].
    pub lip: Option<f64>,
}

/// How far a casing stands off its wall's face, metres (docs/adr/0772 §3).
pub const CASING_PROJECTION: f64 = 0.015;

impl Build {
    fn read(v: &Json) -> Result<Build, String> {
        if !matches!(v, Json::Obj(_)) {
            return Ok(Build::default());
        }
        let n = |k: &str| Option::<f64>::from_json(v.get(k));
        Ok(Build {
            fit: n("fit")?.unwrap_or(0.0),
            leaf_thickness: n("leafThickness")?,
            split: n("split")?,
            frame_depth: n("frameDepth")?,
            frame_offset: n("frameOffset")?,
            threshold: n("threshold")?,
            casing: n("casing")?,
            sill_board: n("sillBoard")?,
            mullions: n("mullions")?.unwrap_or(0.0).clamp(0.0, 255.0) as u8,
            use_zone: n("useZone")?,
            drum: n("drum")?,
            overlap: n("overlap")?,
            track: matches!(v.get("track"), Json::Bool(true)),
            both_sides: matches!(v.get("bothSides"), Json::Bool(true)),
            lip: n("lip")?,
        })
    }
}

impl ToJson for Build {
    fn write_json(&self, out: &mut String) {
        out.push('{');
        let mut first = true;
        if self.fit != 0.0 {
            field(out, &mut first, "fit", &self.fit);
        }
        for (name, v) in [
            ("leafThickness", self.leaf_thickness),
            ("split", self.split),
            ("frameDepth", self.frame_depth),
            ("frameOffset", self.frame_offset),
            ("threshold", self.threshold),
            ("casing", self.casing),
            ("sillBoard", self.sill_board),
        ] {
            if let Some(v) = v {
                field(out, &mut first, name, &v);
            }
        }
        if self.mullions != 0 {
            field(out, &mut first, "mullions", &f64::from(self.mullions));
        }
        if let Some(v) = self.use_zone {
            field(out, &mut first, "useZone", &v);
        }
        for (name, v) in [("drum", self.drum), ("overlap", self.overlap)] {
            if let Some(v) = v {
                field(out, &mut first, name, &v);
            }
        }
        if self.track {
            field(out, &mut first, "track", &true);
        }
        if self.both_sides {
            field(out, &mut first, "bothSides", &true);
        }
        if let Some(v) = self.lip {
            field(out, &mut first, "lip", &v);
        }
        out.push('}');
    }
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
            build: Build::default(),
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
        "build",
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
            Some("revolving") => Operation::Revolving,
            Some("pocket") => Operation::Pocket,
            Some("folding") => Operation::Folding,
            Some(s) => return Err(format!("bilinmeyen açılış “{s}”")),
        };
        let leaves = Option::<f64>::from_json(v.get("leaves"))?.unwrap_or(1.0);
        let swing = Option::<f64>::from_json(v.get("swing"))?;
        Ok(Spec {
            anchor,
            form,
            leaves: leaves.clamp(1.0, 6.0) as u8,
            operation,
            hand_end: word(v, "hand").as_deref() == Some("end"),
            side_right: word(v, "side").as_deref() == Some("right"),
            swing: swing.unwrap_or(if operation == Operation::Folding { FOLD_DRAWN } else { 90.0 }),
            show_swing: !matches!(v.get("showSwing"), Json::Bool(false)),
            frame: Option::<f64>::from_json(v.get("frame"))?.unwrap_or(0.0),
            sill: Option::<f64>::from_json(v.get("sill"))?,
            head: Option::<f64>::from_json(v.get("head"))?,
            depth: Option::<f64>::from_json(v.get("depth"))?,
            inset: Option::<f64>::from_json(v.get("inset"))?,
            build: Build::read(v.get("build"))?,
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
            Operation::Revolving => field(out, first, "operation", "revolving"),
            Operation::Pocket => field(out, first, "operation", "pocket"),
            Operation::Folding => field(out, first, "operation", "folding"),
        }
        if self.hand_end {
            field(out, first, "hand", "end");
        }
        if self.side_right {
            field(out, first, "side", "right");
        }
        let drawn = if self.operation == Operation::Folding { FOLD_DRAWN } else { 90.0 };
        if self.swing != drawn {
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
        if self.build != Build::default() {
            field(out, first, "build", &self.build);
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
    /// Its jamb at `s0` (then `s1`) opens into a pocket door's pocket of this cavity (docs/adr/0777 §2): no line across
    /// the cavity there.
    pub open: [Option<Recess>; 2],
}

impl Hole {
    /// One that goes through the wall on `[s0, s1]`.
    pub fn through(s0: f64, s1: f64) -> Hole {
        Hole {
            s0,
            s1,
            recess: None,
            open: [None, None],
        }
    }
}

/// A pocket door's pockets (docs/adr/0777 §2; the contract's `OpeningPocket`): at its end jamb (else its start's), how
/// far past the jamb, how wide across the wall (centred).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pocket {
    pub at_end: bool,
    pub length: f64,
    pub width: f64,
}

/// A pocket door's pockets for an opening `width` wide (rough); none for another.
pub fn pockets(width: f64, spec: &Spec) -> Vec<Pocket> {
    if spec.form != Form::Door || spec.operation != Operation::Pocket {
        return Vec::new();
    }
    let b = &spec.build;
    let light = width - 2.0 * b.fit - 2.0 * spec.frame;
    let t = b.leaf_thickness.unwrap_or(LEAF_THICKNESS);
    let lip = b.lip.unwrap_or(POCKET_LIP);
    let pocket = |at_end: bool, w: f64| Pocket {
        at_end,
        length: js_max(w - lip - b.fit - spec.frame + POCKET_PLAY, 0.0),
        width: t + 2.0 * POCKET_PLAY,
    };
    if spec.leaves == 2 {
        let share = b.split.unwrap_or(0.5);
        vec![pocket(false, light * share), pocket(true, light * (1.0 - share))]
    } else {
        vec![pocket(spec.hand_end, light)]
    }
}

/// What the opening at `at` of `width` takes out of its wall in plan with its pockets (docs/adr/0777 §2): its hole, then
/// each pocket's cavity, centred in the wall, open at the jamb it meets; none when plan does not cut it.
pub fn holes(at: f64, width: f64, spec: &Spec) -> Vec<Hole> {
    let Some(mut main) = hole(at, width, spec) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for p in pockets(width, spec).into_iter().filter(|p| p.length > 0.0) {
        let cavity = Recess {
            right: false,
            inset: None,
            depth: p.width,
        };
        let (s0, s1, k) = if p.at_end {
            (main.s1, main.s1 + p.length, 1)
        } else {
            (main.s0 - p.length, main.s0, 0)
        };
        main.open[k] = Some(cavity);
        let mut open = [None, None];
        open[1 - k] = Some(cavity);
        out.push(Hole {
            s0,
            s1,
            recess: Some(cavity),
            open,
        });
    }
    out.insert(0, main);
    out
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
    Some(Hole {
        s0,
        s1,
        recess,
        open: [None, None],
    })
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
        let mut out: Vec<Mark> = symbol(place, spec)
            .into_iter()
            .map(|shape| Mark { shape, dashed: false })
            .collect();
        // Its use zone, dashed (docs/adr/0772 §5).
        if let Some(ring) = use_zone(place, spec) {
            out.push(Mark { shape: closed_ring(ring), dashed: true });
        }
        return out;
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

/// Where a door's or window's parts stand in the frame its jambs make (docs/adr/0341 §2, 0772 §3): along the wall from
/// either jamb, across it from the left face.
struct Parts<'a> {
    place: &'a [Vec2],
    f: Frame,
    /// The fitting gap and the frame's width, along the wall.
    fit: f64,
    fr: f64,
    /// The light between the frame's sides.
    clear: f64,
    /// The frame's band across the wall, from the left face.
    dl: f64,
    dr: f64,
}

impl Parts<'_> {
    fn new<'a>(place: &'a [Vec2], spec: &Spec) -> Option<Parts<'a>> {
        let f = frame_of(place)?;
        let half = f.length / 2.0 * 0.999;
        let fit = spec.build.fit.clamp(0.0, half);
        let fr = spec.frame.clamp(0.0, js_max(half - fit, 0.0));
        let clear = f.length - 2.0 * fr - 2.0 * fit;
        let t = f.thickness;
        let (dl, dr) = match spec.build.frame_depth {
            Some(depth) if depth > 0.0 && depth < t => {
                let near = spec.build.frame_offset.unwrap_or((t - depth) / 2.0).clamp(0.0, t - depth);
                if spec.side_right {
                    (t - near - depth, t - near)
                } else {
                    (near, near + depth)
                }
            }
            _ => (0.0, t),
        };
        Some(Parts {
            place,
            f,
            fit,
            fr,
            clear,
            dl,
            dr,
        })
    }

    /// The point `s` along the wall from the start jamb, `d` across it from the left face.
    fn along_start(&self, s: f64, d: f64) -> Vec2 {
        add(self.across(self.place[0], self.place[1], d), mul(self.f.along, s))
    }

    /// The point `s` along the wall back from the end jamb, `d` across it from the left face.
    fn along_end(&self, s: f64, d: f64) -> Vec2 {
        sub(self.across(self.place[2], self.place[3], d), mul(self.f.along, s))
    }

    /// The point `d` from a jamb's left face point `l` toward its right one `r` (its faces and middle exactly).
    fn across(&self, l: Vec2, r: Vec2, d: f64) -> Vec2 {
        let t = self.f.thickness;
        if d == 0.0 {
            l
        } else if d == t {
            r
        } else if d == t / 2.0 {
            mid(l, r)
        } else {
            add(l, mul(sub(r, l), if t > 0.0 { d / t } else { 0.0 }))
        }
    }

    /// The band's middle across the wall.
    fn middle(&self) -> f64 {
        (self.dl + self.dr) / 2.0
    }

    /// The light's start on the band's middle line.
    fn light_start(&self) -> Vec2 {
        self.along_start(self.fit + self.fr, self.middle())
    }

    /// The band's face on the side it opens to, from the left face.
    fn face(&self, spec: &Spec) -> f64 {
        if spec.side_right { self.dr } else { self.dl }
    }

    /// The unit out of the wall on its left face's side (`left`) or its right face's.
    fn out(&self, left: bool) -> Vec2 {
        if left { self.f.left } else { mul(self.f.left, -1.0) }
    }
}

/// A leaf's sweep: its hinge, where it shuts (a unit along the light), its length; the side it opens to.
struct Leaf {
    hinge: Vec2,
    closed: Vec2,
    len: f64,
}

/// Its leaves as they hinge (a swinging door): one from the start (or the end), or two, split as given.
fn swing_leaves(p: &Parts, spec: &Spec) -> Vec<Leaf> {
    let face = p.face(spec);
    let u = p.f.along;
    let at0 = p.along_start(p.fit + p.fr, face);
    let at1 = p.along_end(p.fit + p.fr, face);
    if spec.leaves == 2 {
        let share = spec.build.split.filter(|s| *s > 0.0 && *s < 1.0).unwrap_or(0.5);
        vec![
            Leaf { hinge: at0, closed: u, len: p.clear * share },
            Leaf { hinge: at1, closed: mul(u, -1.0), len: p.clear * (1.0 - share) },
        ]
    } else if spec.hand_end {
        vec![Leaf { hinge: at1, closed: mul(u, -1.0), len: p.clear }]
    } else {
        vec![Leaf { hinge: at0, closed: u, len: p.clear }]
    }
}

/// The turn, degrees, a leaf shut along `closed` makes toward the side it opens to.
fn turn_of(p: &Parts, spec: &Spec, closed: Vec2) -> f64 {
    let outward = if spec.side_right { mul(p.f.left, -1.0) } else { p.f.left };
    if closed.x * outward.y - closed.y * outward.x >= 0.0 {
        spec.swing
    } else {
        -spec.swing
    }
}

/// A door's or window's own pieces from its jambs (docs/adr/0341 §2, 0772 §3): its frame (with its fitting gap, depth and
/// place in the wall), its leaves (unequal, thick) and their swing, a window's lines and mullions, a threshold, casings
/// and a sill board; the other forms' pieces are [`pieces`]'.
pub fn symbol(place: &[Vec2], spec: &Spec) -> Vec<Shape> {
    if !matches!(spec.form, Form::Door | Form::Window) {
        return pieces(place, spec).into_iter().map(|m| m.shape).collect();
    }
    let Some(p) = Parts::new(place, spec) else {
        return Vec::new();
    };
    let b = &spec.build;
    let mut out = Vec::new();
    let (fit, fr, clear) = (p.fit, p.fr, p.clear);
    let (dl, dr) = (p.dl, p.dr);
    let u = p.f.along;
    if fr > 0.0 {
        out.push(closed_ring(vec![
            p.along_start(fit, dl),
            p.along_start(fit + fr, dl),
            p.along_start(fit + fr, dr),
            p.along_start(fit, dr),
        ]));
        out.push(closed_ring(vec![
            p.along_end(fit, dl),
            p.along_end(fit + fr, dl),
            p.along_end(fit + fr, dr),
            p.along_end(fit, dr),
        ]));
    }
    let m0 = p.light_start();
    let depth = dr - dl;
    match spec.form {
        Form::Window => {
            out.push(line(p.along_start(fit + fr, dl), p.along_end(fit + fr, dl)));
            out.push(line(p.along_start(fit + fr, dr), p.along_end(fit + fr, dr)));
            if spec.operation == Operation::Slide {
                let k = depth / 12.0;
                let lap = b.overlap.map_or(clear * 0.05, |o| o / 2.0);
                let half = clear / 2.0;
                out.push(line(add(m0, mul(p.f.left, k)), add(m0, add(mul(u, half + lap), mul(p.f.left, k)))));
                out.push(line(
                    add(m0, add(mul(u, half - lap), mul(p.f.left, -k))),
                    add(m0, add(mul(u, clear), mul(p.f.left, -k))),
                ));
                if b.track {
                    out.push(line(m0, add(m0, mul(u, clear))));
                }
            } else {
                out.push(line(m0, add(m0, mul(u, clear))));
            }
            let n = f64::from(b.mullions);
            for i in 1..=b.mullions {
                let s = fit + fr + clear * f64::from(i) / (n + 1.0);
                out.push(line(p.along_start(s, dl), p.along_start(s, dr)));
            }
            if let Some(board) = b.sill_board.filter(|v| *v > 0.0) {
                // The sill board on the face away from the side it opens to.
                let left = spec.side_right;
                let d = if left { 0.0 } else { p.f.thickness };
                let o = p.out(left);
                let (a, c) = (p.along_start(0.0, d), p.along_end(0.0, d));
                out.push(closed_ring(vec![a, c, add(c, mul(o, board)), add(a, mul(o, board))]));
            }
        }
        Form::Door => {
            match spec.operation {
                Operation::Slide => {
                    let k = depth / 8.0;
                    if spec.leaves == 1 && b.track {
                        out.extend(surface_slide(&p, spec));
                    } else if spec.leaves == 1 {
                        out.push(line(add(m0, mul(p.f.left, k)), add(m0, add(mul(u, clear), mul(p.f.left, k)))));
                    } else {
                        let lap = b.overlap.map_or(clear * 0.05, |o| o / 2.0);
                        let half = clear / 2.0;
                        out.push(line(add(m0, mul(p.f.left, k)), add(m0, add(mul(u, half + lap), mul(p.f.left, k)))));
                        out.push(line(
                            add(m0, add(mul(u, half - lap), mul(p.f.left, -k))),
                            add(m0, add(mul(u, clear), mul(p.f.left, -k))),
                        ));
                        if b.track {
                            out.push(line(m0, add(m0, mul(u, clear))));
                        }
                    }
                }
                Operation::Revolving => out.extend(revolving(&p, spec)),
                Operation::Pocket => out.extend(pocket_leaves(&p, spec)),
                Operation::Folding => {
                    for g in fold_groups(&p, spec) {
                        let mut at = g.hinge;
                        for i in 0..g.panels {
                            let rise = if i % 2 == 0 { g.sin } else { -g.sin };
                            let next = add(at, add(mul(g.dir, g.width * g.cos), mul(g.out, g.width * rise)));
                            out.push(line(at, next));
                            at = next;
                        }
                    }
                }
                Operation::Fixed => {
                    out.push(line(m0, add(m0, mul(u, clear))));
                }
                Operation::Swing => {
                    for l in swing_leaves(&p, spec) {
                        let turn = turn_of(&p, spec, l.closed);
                        let open = rotated(l.closed, turn);
                        let tip = add(l.hinge, mul(open, l.len));
                        match b.leaf_thickness.filter(|t| *t > 0.0) {
                            // A thick leaf: a band on the side it swung from.
                            Some(t) => {
                                let back = rotated(open, if turn < 0.0 { 90.0 } else { -90.0 });
                                out.push(closed_ring(vec![l.hinge, tip, add(tip, mul(back, t)), add(l.hinge, mul(back, t))]));
                            }
                            None => out.push(line(l.hinge, tip)),
                        }
                        if spec.show_swing {
                            let a_closed = atan2(l.closed.y, l.closed.x);
                            let a_open = atan2(open.y, open.x);
                            let (a0, a1) = if turn > 0.0 { (a_closed, a_open) } else { (a_open, a_closed) };
                            out.push(Shape::Arc { c: l.hinge, r: l.len, a0, a1 });
                        }
                    }
                }
            }
            if let Some(th) = b.threshold.filter(|v| *v > 0.0) {
                let c = p.middle();
                let (a, z) = (js_max(c - th / 2.0, 0.0), js_min(c + th / 2.0, p.f.thickness));
                out.push(closed_ring(vec![
                    p.along_start(fit + fr, a),
                    p.along_end(fit + fr, a),
                    p.along_end(fit + fr, z),
                    p.along_start(fit + fr, z),
                ]));
            }
        }
        Form::Opening | Form::Niche | Form::Flue => {}
    }
    if let Some(w) = b.casing.filter(|v| *v > 0.0) {
        let (s0, s1) = (fit + fr - w, fit + fr);
        for left in [true, false] {
            let d = if left { 0.0 } else { p.f.thickness };
            let o = mul(p.out(left), CASING_PROJECTION);
            for along_end in [false, true] {
                let at = |s: f64| if along_end { p.along_end(s, d) } else { p.along_start(s, d) };
                let (a, c) = (at(s0), at(s1));
                out.push(closed_ring(vec![a, c, add(c, o), add(a, o)]));
            }
        }
    }
    out
}

/// A revolving door's drum (docs/adr/0777 §1): its middle and inside radius, centred on the light and the band's
/// middle; the drum at most the light.
fn drum_of(p: &Parts, spec: &Spec) -> (Vec2, f64) {
    let r = js_min(spec.build.drum.unwrap_or(p.clear), p.clear) / 2.0;
    (p.along_start(p.fit + p.fr + p.clear / 2.0, p.middle()), r)
}

/// A revolving door's pieces (docs/adr/0777 §1): its drum, the drum's two walls (arcs 360°/n wide about the wall's line,
/// [`DRUM_WALL`] outside it) and its wings from the middle, the first at 90° − 180°/n from along the wall.
fn revolving(p: &Parts, spec: &Spec) -> Vec<Shape> {
    let (c, r) = drum_of(p, spec);
    if !(r > 0.0) {
        return Vec::new();
    }
    let n = f64::from(spec.leaves.clamp(3, 4));
    let pi = crate::jsmath::PI;
    let a = atan2(p.f.along.y, p.f.along.x);
    let half = pi / n;
    let mut out = vec![Shape::Circle { c, r }];
    for base in [a, a + pi] {
        out.push(Shape::Arc {
            c,
            r: r + DRUM_WALL,
            a0: base - half,
            a1: base + half,
        });
    }
    for k in 0..spec.leaves.clamp(3, 4) {
        let ang = a + pi / 2.0 - half + 2.0 * pi * f64::from(k) / n;
        out.push(line(c, add(c, Vec2::new(r * cos(ang), r * sin(ang)))));
    }
    out
}

/// A pocket door's leaves shut (docs/adr/0777 §2): bands along the band's middle across the light, its leaf's thickness
/// (none given: [`LEAF_THICKNESS`]) wide; two split as given. Its pockets are its wall's cavities.
fn pocket_leaves(p: &Parts, spec: &Spec) -> Vec<Shape> {
    let t = spec.build.leaf_thickness.filter(|t| *t > 0.0).unwrap_or(LEAF_THICKNESS);
    let (lo, hi) = (p.middle() - t / 2.0, p.middle() + t / 2.0);
    let start = p.fit + p.fr;
    let band = |s0: f64, s1: f64| {
        closed_ring(vec![
            p.along_start(s0, lo),
            p.along_start(s1, lo),
            p.along_start(s1, hi),
            p.along_start(s0, hi),
        ])
    };
    if spec.leaves == 2 {
        let mid = start + p.clear * spec.build.split.filter(|s| *s > 0.0 && *s < 1.0).unwrap_or(0.5);
        vec![band(start, mid), band(mid, start + p.clear)]
    } else {
        vec![band(start, start + p.clear)]
    }
}

/// A one-leaf sliding door on its track (docs/adr/0777 §4): its leaf beside the wall face it opens to, over the light,
/// [`LEAF_THICKNESS`] (or its own) deep from a quarter of the wall's thickness out; its track along that face from the
/// light's far side to a light past the jamb at its `hand`, where it slides to.
fn surface_slide(p: &Parts, spec: &Spec) -> Vec<Shape> {
    let t = spec.build.leaf_thickness.filter(|t| *t > 0.0).unwrap_or(LEAF_THICKNESS);
    let left = !spec.side_right;
    let d = if left { 0.0 } else { p.f.thickness };
    let o = p.out(left);
    let k = p.f.thickness / 4.0;
    let start = p.fit + p.fr;
    let on = |s: f64, off: f64| add(p.along_start(s, d), mul(o, off));
    let leaf = closed_ring(vec![
        on(start, k),
        on(start + p.clear, k),
        on(start + p.clear, k + t),
        on(start, k + t),
    ]);
    let (a, z) = if spec.hand_end {
        (start, start + 2.0 * p.clear)
    } else {
        (start + p.clear, start - p.clear)
    };
    vec![leaf, line(on(a, k / 2.0), on(z, k / 2.0))]
}

/// A folding door's panels as they fold (docs/adr/0777 §3): from the jamb at its `hand` (or both jambs, half each), on
/// the face it opens to, `swing` degrees open (none given: [`FOLD_DRAWN`]), zigzagging out of the wall and back.
struct FoldGroup {
    hinge: Vec2,
    dir: Vec2,
    out: Vec2,
    panels: u8,
    width: f64,
    cos: f64,
    sin: f64,
}

fn fold_groups(p: &Parts, spec: &Spec) -> Vec<FoldGroup> {
    let n = spec.leaves.clamp(2, 6);
    let width = p.clear / f64::from(n);
    let face = p.face(spec);
    let o = p.out(!spec.side_right);
    let r = spec.swing * crate::jsmath::PI / 180.0;
    let (c, s) = (cos(r), sin(r));
    let at0 = p.along_start(p.fit + p.fr, face);
    let at1 = p.along_end(p.fit + p.fr, face);
    let u = p.f.along;
    let group = |hinge: Vec2, dir: Vec2, panels: u8| FoldGroup {
        hinge,
        dir,
        out: o,
        panels,
        width,
        cos: c,
        sin: s,
    };
    if spec.build.both_sides {
        vec![group(at0, u, n / 2), group(at1, mul(u, -1.0), n - n / 2)]
    } else if spec.hand_end {
        vec![group(at1, mul(u, -1.0), n)]
    } else {
        vec![group(at0, u, n)]
    }
}

/// The sweep of each of a swinging door's leaves (docs/adr/0772 §5): its hinge and its arc's points, a closed ring; a
/// revolving door's drum (docs/adr/0777 §1); a folding door's panels' reach, a panel deep from each jamb they fold to
/// (§3); none for a door that slides, a fixed door, or a window.
pub fn swing_zones(place: &[Vec2], spec: &Spec) -> Vec<Vec<Vec2>> {
    if spec.form != Form::Door {
        return Vec::new();
    }
    let Some(p) = Parts::new(place, spec) else {
        return Vec::new();
    };
    match spec.operation {
        Operation::Revolving => {
            let (c, r) = drum_of(&p, spec);
            if !(r > 0.0) {
                return Vec::new();
            }
            let steps = 32;
            let ring = (0..steps)
                .map(|k| {
                    let a = 2.0 * crate::jsmath::PI * f64::from(k) / f64::from(steps);
                    add(c, Vec2::new(r * cos(a), r * sin(a)))
                })
                .collect();
            return vec![ring];
        }
        Operation::Folding => {
            return fold_groups(&p, spec)
                .into_iter()
                .filter(|g| g.width > 0.0)
                .map(|g| {
                    let along = mul(g.dir, g.width * f64::from(g.panels));
                    let deep = mul(g.out, g.width);
                    vec![g.hinge, add(g.hinge, along), add(add(g.hinge, along), deep), add(g.hinge, deep)]
                })
                .collect();
        }
        Operation::Swing => {}
        _ => return Vec::new(),
    }
    swing_leaves(&p, spec)
        .into_iter()
        .filter(|l| l.len > 0.0)
        .map(|l| {
            let turn = turn_of(&p, spec, l.closed);
            let steps = 16;
            let mut ring = vec![l.hinge];
            for k in 0..=steps {
                let dir = rotated(l.closed, turn * f64::from(k) / f64::from(steps));
                ring.push(add(l.hinge, mul(dir, l.len)));
            }
            ring
        })
        .collect()
}

/// A door's or window's use zone (docs/adr/0772 §5): the light's width, `useZone` deep, out of its wall on the side it
/// opens to; none when it has none.
pub fn use_zone(place: &[Vec2], spec: &Spec) -> Option<Vec<Vec2>> {
    let depth = spec.build.use_zone.filter(|v| *v > 0.0)?;
    if !matches!(spec.form, Form::Door | Form::Window) {
        return None;
    }
    let p = Parts::new(place, spec)?;
    let left = !spec.side_right;
    let d = if left { 0.0 } else { p.f.thickness };
    let o = mul(p.out(left), depth);
    let (a, c) = (p.along_start(p.fit + p.fr, d), p.along_end(p.fit + p.fr, d));
    Some(vec![a, c, add(c, o), add(a, o)])
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
