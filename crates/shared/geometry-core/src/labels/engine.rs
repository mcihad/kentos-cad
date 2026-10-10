//! The label engine's placing (docs/adr/0212 §3.3–§3.8): each label unit's
//! forms and their candidates, the room labels take, the obstacles, the
//! order, the greedy pass, the one-step improvement and the result. Pure:
//! the store gathers the units, the obstacles and the drawing's texts of a
//! window on the page (geometry in px, y up) and hands them here.

use std::collections::HashMap;

use crate::jsmath::{atan2, cos, js_max, js_min, sin};
use crate::text::Font;
use crate::vec2::Vec2;

use super::geom::{
    Obb, Walk, box_inside, inside_rings, long_axis, norm, segment_distance2, upright, wrap,
};
use super::style::{AreaMode, Class, LINE_HEIGHT, LineMode, Overlap, PointMode, Position};
use super::text::{abbreviated, letter_width, line_width, lines, stacked};

/// The grid cells boxes are kept in, px.
const CELL: f64 = 32.0;
/// The most candidates the improvement tries for one label.
const IMPROVE_TRIES: usize = 32;
/// The most candidate middles along a path.
const PATH_CANDIDATES: usize = 21;
/// A label inside an area that does not fit it, when that is allowed.
const OVERRUN: f64 = 0.5;
/// An outside candidate's cost before its order.
const OUTSIDE: f64 = 1.0;
/// A point symbol a candidate covers.
const SYMBOL: f64 = 0.05;
/// An obstacle's cost per weight.
const PER_WEIGHT: f64 = 0.05;

/// Where a unit's label may go.
#[derive(Clone, Debug)]
pub enum Geo {
    /// A point and its symbol's radius.
    Point { p: Vec2, r: f64 },
    /// An object's box's top left corner (a sheet's frame).
    Corner { tl: Vec2 },
    /// A path from `s0` to `s1` along it; `interior` an outline's inside, to
    /// the left of its direction (perimeter and boundary).
    Path {
        walk: Walk,
        s0: f64,
        s1: f64,
        interior: bool,
    },
    /// An area cut to the window: its rings, its pole and its box.
    Area {
        rings: Vec<Vec<Vec2>>,
        pole: Vec2,
        bbox: [f64; 4],
    },
}

/// What a unit's label is drawn as when it is pinned.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pinned {
    /// The label's middle, px.
    pub at: Vec2,
    /// Radians.
    pub angle: f64,
}

/// One label to place: an object's class's text (a repeated line's piece).
#[derive(Clone, Debug)]
pub struct Unit<'a> {
    pub id: f64,
    /// The object's place in the document.
    pub order: u64,
    /// Its layer's place in the drawing order, the top first.
    pub rank: u32,
    /// Its class's index (in its layer's list, or its kind's default slot).
    pub class: u16,
    pub cls: &'a Class,
    pub text: &'a str,
    /// A repeated path's piece.
    pub chunk: u32,
    pub geo: Geo,
    /// Where its callout points (the point, the pole, a path's nearest point): px.
    pub target: Option<Vec2>,
    pub pin: Option<Pinned>,
    pub hidden: bool,
    /// A contour's reading direction: true the path's own, false reversed; none by the upright rule.
    pub uphill: Option<bool>,
}

/// An obstacle on the page.
#[derive(Clone, Debug)]
pub enum Shape {
    Circle(Vec2, f64),
    Segment(Vec2, Vec2),
    /// An area's rings (its inside is the obstacle; its edges come as segments).
    Area(Vec<Vec<Vec2>>, [f64; 4]),
}

/// An obstacle: its object, its weight (none: a point symbol) and its shape.
#[derive(Clone, Debug)]
pub struct Obstacle {
    pub owner: f64,
    pub weight: Option<u8>,
    pub shape: Shape,
}

/// A label as placed.
#[derive(Clone, Debug, PartialEq)]
pub enum Drawn {
    /// Straight: its block's middle, angle and its lines' middles (relative
    /// to the block's, in its frame) with the lines' texts.
    Straight {
        c: Vec2,
        angle: f64,
        lines: Vec<(Vec2, String)>,
    },
    /// Letter by letter: each letter's middle, angle and index in the text.
    Curved {
        text: String,
        letters: Vec<(Vec2, f64, u32)>,
    },
}

/// A placed (or unplaced, hidden) label.
#[derive(Clone, Debug, PartialEq)]
pub struct Label {
    pub unit: usize,
    pub drawn: Drawn,
    /// Its block's size, px, and its letters' size.
    pub w: f64,
    pub h: f64,
    pub size: f64,
    pub boxes: Vec<Obb>,
    /// From the label's edge to its object, px.
    pub callout: Option<(Vec2, Vec2)>,
    pub state: u32,
}

/// The label is where its pin puts it.
pub const PINNED: u32 = 1;
/// No place was free (drawn only on request).
pub const UNPLACED: u32 = 2;
/// Hidden by hand (drawn only on request).
pub const HIDDEN: u32 = 4;
/// Placed over another.
pub const OVERLAPPING: u32 = 8;
/// Letter by letter.
pub const CURVED: u32 = 16;
/// Outside its area.
pub const OUTSIDE_AREA: u32 = 32;
/// A contour's mask under it.
pub const MASKED: u32 = 64;

/// What a placing wants besides the placed labels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Options {
    pub unplaced: bool,
    pub hidden: bool,
}

/// A label's form: its lines and size (docs/adr/0212 §3.4).
#[derive(Clone, Debug, PartialEq)]
struct Form {
    lines: Vec<String>,
    widths: Vec<f64>,
    size: f64,
    w: f64,
    h: f64,
    outside: bool,
}

/// A candidate: where a form of a label would go and what it costs. A straight one keeps only its
/// block's middle and angle (its lines are made for the label placed) and its one box in place:
/// a unit's many candidates allocate nothing (docs/adr/0212 §6).
#[derive(Clone, Debug)]
struct Cand {
    cost: f64,
    boxes: Boxes,
    spot: Spot,
    callout: Option<(Vec2, Vec2)>,
    outside: bool,
}

/// Where a candidate is drawn: a straight block, or letter by letter.
#[derive(Clone, Debug)]
enum Spot {
    Straight {
        c: Vec2,
        angle: f64,
    },
    /// Its letters (the text is its form's one line).
    Curved {
        letters: Vec<(Vec2, f64, u32)>,
    },
}

/// A candidate's boxes: a straight block's one, in place, or a curved label's, one a letter.
#[derive(Clone, Debug)]
enum Boxes {
    One([Obb; 1]),
    Many(Vec<Obb>),
}

impl std::ops::Deref for Boxes {
    type Target = [Obb];

    fn deref(&self) -> &[Obb] {
        match self {
            Boxes::One(b) => b,
            Boxes::Many(v) => v,
        }
    }
}

/// The boxes placed labels and the drawing's texts take.
struct Room {
    x0: f64,
    y0: f64,
    cols: usize,
    rows: usize,
    cells: Vec<Vec<u32>>,
    boxes: Vec<(Obb, u32)>,
    /// Each box's bounds: the many far from a box are passed by before the exact test.
    bounds: Vec<[f64; 4]>,
    alive: Vec<bool>,
    /// Each owner's boxes, so a label leaves without a search.
    owned: HashMap<u32, Vec<u32>>,
}

/// Whether two bounds are clearly apart (by more than a rounding): boxes, circles, segments and areas
/// whose bounds are so cannot meet, and the exact tests are spared.
fn apart(a: &[f64; 4], b: &[f64; 4]) -> bool {
    const EPS: f64 = 1e-6;
    a[2] < b[0] - EPS || b[2] < a[0] - EPS || a[3] < b[1] - EPS || b[3] < a[1] - EPS
}

/// The placed labels' middles by their texts, for the duplicates (docs/adr/0212 §3.5): kept only when
/// a unit asks for them.
struct Texts<'a> {
    on: bool,
    by: HashMap<&'a str, Vec<(Vec2, usize)>>,
}

impl<'a> Texts<'a> {
    fn add(&mut self, text: &'a str, middle: impl FnOnce() -> Vec2, owner: usize) {
        if self.on {
            self.by.entry(text).or_default().push((middle(), owner));
        }
    }

    fn remove(&mut self, text: &str, owner: usize) {
        if let Some(list) = self.by.get_mut(text) {
            list.retain(|(_, o)| *o != owner);
        }
    }
}

/// Whether any of `a` overlaps any of `b` (as the room tests a box against one in it).
fn meets(a: &[Obb], b: &[Obb]) -> bool {
    a.iter().any(|x| b.iter().any(|y| x.overlaps(y)))
}

/// The owner of a box no label can move (a text of the drawing).
const FIXED: u32 = u32::MAX;

impl Room {
    fn new(width: f64, height: f64, margin: f64) -> Room {
        let cols = ((width + 2.0 * margin) / CELL).ceil().clamp(1.0, 4096.0) as usize;
        let rows = ((height + 2.0 * margin) / CELL).ceil().clamp(1.0, 4096.0) as usize;
        Room {
            x0: -margin,
            y0: -margin,
            cols,
            rows,
            cells: vec![Vec::new(); cols * rows],
            boxes: Vec::new(),
            bounds: Vec::new(),
            alive: Vec::new(),
            owned: HashMap::new(),
        }
    }

    fn span(&self, b: &Obb) -> (usize, usize, usize, usize) {
        let [x0, y0, x1, y1] = b.aabb();
        let col = |x: f64| {
            (((x - self.x0) / CELL)
                .floor()
                .clamp(0.0, (self.cols - 1) as f64)) as usize
        };
        let row = |y: f64| {
            (((y - self.y0) / CELL)
                .floor()
                .clamp(0.0, (self.rows - 1) as f64)) as usize
        };
        (col(x0), col(x1), row(y0), row(y1))
    }

    fn insert(&mut self, boxes: &[Obb], owner: u32) {
        for b in boxes {
            let k = self.boxes.len() as u32;
            self.boxes.push((*b, owner));
            self.bounds.push(b.aabb());
            self.alive.push(true);
            self.owned.entry(owner).or_default().push(k);
            let (c0, c1, r0, r1) = self.span(b);
            for r in r0..=r1 {
                for c in c0..=c1 {
                    self.cells[r * self.cols + c].push(k);
                }
            }
        }
    }

    /// An owner's boxes out of the room; their places, to bring them back.
    fn take(&mut self, owner: u32) -> Vec<u32> {
        let gone = self.owned.remove(&owner).unwrap_or_default();
        for &k in &gone {
            self.alive[k as usize] = false;
        }
        gone
    }

    /// Boxes taken out brought back, where they were.
    fn revive(&mut self, owner: u32, boxes: Vec<u32>) {
        for &k in &boxes {
            self.alive[k as usize] = true;
        }
        self.owned.insert(owner, boxes);
    }

    /// The owners of the boxes overlapping any of `boxes`, each once, in the order met.
    fn blockers(&self, boxes: &[Obb], out: &mut Vec<u32>) {
        out.clear();
        for b in boxes {
            let (c0, c1, r0, r1) = self.span(b);
            let ab = b.aabb();
            for r in r0..=r1 {
                for c in c0..=c1 {
                    for &k in &self.cells[r * self.cols + c] {
                        let (o, owner) = &self.boxes[k as usize];
                        if self.alive[k as usize]
                            && !apart(&ab, &self.bounds[k as usize])
                            && !out.contains(owner)
                            && b.overlaps(o)
                        {
                            out.push(*owner);
                        }
                    }
                }
            }
        }
    }

    fn free(&self, boxes: &[Obb]) -> bool {
        for b in boxes {
            let (c0, c1, r0, r1) = self.span(b);
            let ab = b.aabb();
            for r in r0..=r1 {
                for c in c0..=c1 {
                    for &k in &self.cells[r * self.cols + c] {
                        if self.alive[k as usize]
                            && !apart(&ab, &self.bounds[k as usize])
                            && b.overlaps(&self.boxes[k as usize].0)
                        {
                            return false;
                        }
                    }
                }
            }
        }
        true
    }
}

/// The obstacles in grid cells.
struct Obstacles {
    x0: f64,
    y0: f64,
    cols: usize,
    rows: usize,
    cells: Vec<Vec<u32>>,
    list: Vec<Obstacle>,
    /// Each obstacle's bounds: the many far from a box are passed by before the exact test.
    bounds: Vec<[f64; 4]>,
}

impl Obstacles {
    fn new(width: f64, height: f64, margin: f64, list: Vec<Obstacle>) -> Obstacles {
        let cols = ((width + 2.0 * margin) / CELL).ceil().clamp(1.0, 4096.0) as usize;
        let rows = ((height + 2.0 * margin) / CELL).ceil().clamp(1.0, 4096.0) as usize;
        let mut o = Obstacles {
            x0: -margin,
            y0: -margin,
            cols,
            rows,
            cells: vec![Vec::new(); cols * rows],
            list: Vec::new(),
            bounds: Vec::with_capacity(list.len()),
        };
        for (k, ob) in list.iter().enumerate() {
            let [x0, y0, x1, y1] = match &ob.shape {
                Shape::Circle(p, r) => [p.x - r, p.y - r, p.x + r, p.y + r],
                Shape::Segment(a, b) => [
                    js_min(a.x, b.x),
                    js_min(a.y, b.y),
                    js_max(a.x, b.x),
                    js_max(a.y, b.y),
                ],
                Shape::Area(_, bbox) => *bbox,
            };
            o.bounds.push([x0, y0, x1, y1]);
            if x1 < o.x0 || y1 < o.y0 {
                continue;
            }
            let col =
                |x: f64| (((x - o.x0) / CELL).floor().clamp(0.0, (o.cols - 1) as f64)) as usize;
            let row =
                |y: f64| (((y - o.y0) / CELL).floor().clamp(0.0, (o.rows - 1) as f64)) as usize;
            for r in row(y0)..=row(y1) {
                for c in col(x0)..=col(x1) {
                    o.cells[r * o.cols + c].push(k as u32);
                }
            }
        }
        o.list = list;
        o
    }

    /// What the boxes cost among the obstacles for a label of `priority` on
    /// object `own`; none when an obstacle it may not cover is under them.
    fn cost(
        &self,
        boxes: &[Obb],
        own: f64,
        priority: u8,
        seen: &mut Vec<u32>,
        hit: &mut Vec<f64>,
    ) -> Option<f64> {
        seen.clear();
        hit.clear();
        let mut cost = 0.0;
        for b in boxes {
            let ab = b.aabb();
            let [x0, y0, x1, y1] = ab;
            let col = |x: f64| {
                (((x - self.x0) / CELL)
                    .floor()
                    .clamp(0.0, (self.cols - 1) as f64)) as usize
            };
            let row = |y: f64| {
                (((y - self.y0) / CELL)
                    .floor()
                    .clamp(0.0, (self.rows - 1) as f64)) as usize
            };
            for r in row(y0)..=row(y1) {
                for c in col(x0)..=col(x1) {
                    for &k in &self.cells[r * self.cols + c] {
                        if apart(&ab, &self.bounds[k as usize]) {
                            continue;
                        }
                        let ob = &self.list[k as usize];
                        if ob.owner == own || seen.contains(&k) {
                            continue;
                        }
                        let covers = match &ob.shape {
                            Shape::Circle(p, rad) => b.hits_circle(*p, *rad),
                            Shape::Segment(p, q) => b.hits_segment(*p, *q),
                            Shape::Area(rings, _) => inside_rings(b.c, rings),
                        };
                        if !covers {
                            continue;
                        }
                        seen.push(k);
                        match ob.weight {
                            None => cost += SYMBOL,
                            Some(w) => {
                                if hit.contains(&ob.owner) {
                                    continue;
                                }
                                if priority < w {
                                    return None;
                                }
                                hit.push(ob.owner);
                                cost += PER_WEIGHT * f64::from(w);
                            }
                        }
                    }
                }
            }
        }
        Some(cost)
    }
}

/// A placing's inputs: the window's size on the page, the units, the
/// obstacles, the drawing's texts (fixed boxes) and the typeface.
pub struct Scene<'a> {
    pub width: f64,
    pub height: f64,
    /// Px per metre.
    pub scale: f64,
    pub font: Font,
    pub units: Vec<Unit<'a>>,
    pub obstacles: Vec<Obstacle>,
    pub fixed: Vec<Obb>,
    pub options: Options,
}

/// The forms a unit's label is tried in, in order (docs/adr/0212 §3.4).
fn forms(u: &Unit<'_>, font: Font, size: f64) -> Vec<Form> {
    let c = u.cls;
    let make = |lines: Vec<String>, size: f64, outside: bool| {
        let widths: Vec<f64> = lines
            .iter()
            .map(|l| line_width(l, font, c.bold, size))
            .collect();
        let w = widths.iter().copied().fold(0.0, js_max);
        let n = lines.len().max(1) as f64;
        Form {
            lines,
            widths,
            size,
            w,
            h: size + (n - 1.0) * LINE_HEIGHT * size,
            outside,
        }
    };
    let base = match &c.abbreviate {
        Some(a) if a.always => abbreviated(u.text, a),
        _ => u.text.to_owned(),
    };
    let lay = |t: &str| match &c.stack {
        Some(s) if s.always => stacked(t, s),
        _ => lines(t),
    };
    let mut texts: Vec<Vec<String>> = vec![lay(&base)];
    let push = |l: Vec<String>, texts: &mut Vec<Vec<String>>| {
        if !texts.contains(&l) {
            texts.push(l);
        }
    };
    if let Some(s) = c.stack.as_ref().filter(|s| !s.always) {
        push(stacked(&base, s), &mut texts);
    }
    if let Some(a) = c.abbreviate.as_ref().filter(|a| !a.always) {
        let short = abbreviated(&base, a);
        push(lay(&short), &mut texts);
        if let Some(s) = c.stack.as_ref().filter(|s| !s.always) {
            push(stacked(&short, s), &mut texts);
        }
    }
    let mut out: Vec<Form> = texts.iter().map(|l| make(l.clone(), size, false)).collect();
    if c.shrink < 1.0 {
        let last = texts.last().cloned().unwrap_or_default();
        let mut k = 1;
        loop {
            let f = 1.0 - 0.1 * f64::from(k);
            if f < c.shrink - 1e-9 || f <= 0.0 {
                break;
            }
            out.push(make(last.clone(), size * f, false));
            k += 1;
        }
    }
    if c.outside && matches!(u.geo, Geo::Area { .. }) {
        out.push(make(texts[0].clone(), size, true));
    }
    out
}

/// The straight label's lines' middles in its frame and their texts.
fn straight_lines(f: &Form, align: super::style::Align) -> Vec<(Vec2, String)> {
    let mut out = Vec::with_capacity(f.lines.len());
    for (i, (l, w)) in f.lines.iter().zip(&f.widths).enumerate() {
        let dy = (f.h - f.size) / 2.0 - i as f64 * LINE_HEIGHT * f.size;
        let dx = match align {
            super::style::Align::Left => -(f.w - w) / 2.0,
            super::style::Align::Right => (f.w - w) / 2.0,
            super::style::Align::Center => 0.0,
        };
        out.push((Vec2::new(dx, dy), l.clone()));
    }
    out
}

fn straight(f: &Form, u: &Unit<'_>, c: Vec2, angle: f64, cost: f64) -> Cand {
    let pad = u.cls.pad();
    Cand {
        cost,
        // A level box is its own (cos 0 is 1 and sin 0 is 0 exactly): no turn to work out.
        boxes: Boxes::One([if angle == 0.0 {
            Obb::level(c, f.w / 2.0 + pad, f.h / 2.0 + pad)
        } else {
            Obb::new(c, angle, f.w / 2.0 + pad, f.h / 2.0 + pad)
        }]),
        spot: Spot::Straight { c, angle },
        callout: None,
        outside: false,
    }
}

/// The offsets a path label's side takes: on it, its plus side, its minus side.
fn sides(position: Position) -> &'static [f64] {
    match position {
        Position::On => &[0.0],
        Position::Above => &[1.0],
        Position::Below => &[-1.0],
        Position::Sides => &[1.0, -1.0],
    }
}

/// The middles tried along a path's stretch `s0..s1` for a label `w` long:
/// its middle first, then either side by the step.
fn middles(s0: f64, s1: f64, w: f64) -> Vec<f64> {
    let len = s1 - s0;
    if len < w {
        return Vec::new();
    }
    let mid = (s0 + s1) / 2.0;
    let room = (len - w) / 2.0;
    let step = js_max(js_max(w / 4.0, (len - w) / 20.0), 2.0);
    let mut out = vec![mid];
    let mut k = 1.0;
    while out.len() < PATH_CANDIDATES && k * step <= room + 1e-9 {
        out.push(mid + k * step);
        if out.len() < PATH_CANDIDATES {
            out.push(mid - k * step);
        }
        k += 1.0;
    }
    out
}

/// A path's candidates at its `k`-th middle `t` (paralel, kıvrık, yatay, eş yükselti; an outline's),
/// each with its place in the made order (`4k` and its side's).
#[allow(clippy::too_many_arguments)]
fn path_at(
    f: &Form,
    u: &Unit<'_>,
    font: Font,
    (walk, s0, s1, interior): (&Walk, f64, f64, bool),
    back: &std::cell::OnceCell<Walk>,
    k: usize,
    t: f64,
    out: &mut Vec<(usize, Cand)>,
) {
    let c = u.cls;
    let len = s1 - s0;
    let mid = (s0 + s1) / 2.0;
    let pad = c.pad();
    let outline = interior;
    let mode = if outline {
        if c.curved {
            LineMode::Curved
        } else {
            LineMode::Parallel
        }
    } else {
        c.line
    };
    let curved = matches!(mode, LineMode::Curved | LineMode::Contour) && f.lines.len() == 1;
    let place = 0.0001 + 0.001 * (t - mid).abs() / len;
    match mode {
        LineMode::Horizontal => {
            let p = walk.point(t);
            out.push((4 * k, straight(f, u, p, 0.0, place)));
        }
        _ if !curved => {
            let (a, b) = (t - f.w / 2.0, t + f.w / 2.0);
            let (p0, p1) = (walk.point(a), walk.point(b));
            let chord = norm(p1.x - p0.x, p1.y - p0.y);
            let q = chord / f.w;
            if !(q >= 0.8) {
                return;
            }
            let mut e2: f64 = 0.0;
            for p in walk.between(a, b) {
                e2 = js_max(e2, segment_distance2(p, p0, p1));
            }
            let e = e2.sqrt();
            if e > f.size / 2.0 {
                return;
            }
            let along = atan2(p1.y - p0.y, p1.x - p0.x);
            let theta = upright(along);
            let m = Vec2::new((p0.x + p1.x) / 2.0, (p0.y + p1.y) / 2.0);
            // The plus side: the text's top, or an outline's inside.
            let n = if outline {
                Vec2::new(-sin(along), cos(along))
            } else {
                Vec2::new(-sin(theta), cos(theta))
            };
            let base = place + 0.01 * (1.0 - q) + 0.001 * e / f.size;
            for (si, &side) in sides(c.position).iter().enumerate() {
                let off = if side == 0.0 {
                    0.0
                } else {
                    side * (c.distance + f.h / 2.0 + e)
                };
                let cc = Vec2::new(m.x + n.x * off, m.y + n.y * off);
                let extra = if side < 0.0 { 0.0005 } else { 0.0 };
                out.push((4 * k + si, straight(f, u, cc, theta, base + extra)));
            }
        }
        _ => {
            let text = &f.lines[0];
            let adv: Vec<f64> = text
                .chars()
                .map(|ch| letter_width(ch, font, c.bold, f.size))
                .collect();
            let total = walk.length();
            // Each letter's middle, angle and the angle's cosine and sine.
            let letters_on = |w: &Walk, start: f64| -> Vec<(Vec2, f64, f64, f64)> {
                let mut s = start;
                adv.iter()
                    .map(|a| {
                        let at = s + a / 2.0;
                        s += a;
                        let (angle, cs, sn) = w.turn(at);
                        (w.point(at), angle, cs, sn)
                    })
                    .collect()
            };
            let fwd = letters_on(walk, t - f.w / 2.0);
            let reversed = match (mode, u.uphill) {
                (LineMode::Contour, Some(own)) => !own,
                _ => {
                    let lean: f64 = fwd.iter().zip(&adv).map(|((_, _, cs, _), w)| w * cs).sum();
                    lean < 0.0
                }
            };
            let letters = if reversed {
                letters_on(back.get_or_init(|| walk.reversed()), total - t - f.w / 2.0)
            } else {
                fwd
            };
            let mut turn = 0.0;
            for w in letters.windows(2) {
                let d = wrap(w[1].1 - w[0].1).abs();
                if d > c.max_angle {
                    return;
                }
                turn += d;
            }
            let mean = if letters.len() > 1 {
                turn / (letters.len() - 1) as f64
            } else {
                0.0
            };
            let base = place + 0.01 * mean / std::f64::consts::PI;
            let position = if mode == LineMode::Contour {
                Position::On
            } else {
                c.position
            };
            for (si, &side) in sides(position).iter().enumerate() {
                let off = side * (c.distance + f.size / 2.0);
                let mut boxes = Vec::with_capacity(letters.len());
                let mut drawn = Vec::with_capacity(letters.len());
                for (i, ((p, a, cs, sn), w)) in letters.iter().zip(&adv).enumerate() {
                    // An outline's inside is the path's left whichever way the letters read.
                    let n = if outline && reversed {
                        Vec2::new(*sn, -cs)
                    } else {
                        Vec2::new(-sn, *cs)
                    };
                    let q = Vec2::new(p.x + n.x * off, p.y + n.y * off);
                    boxes.push(Obb::along(
                        q,
                        Vec2::new(*cs, *sn),
                        w / 2.0 + pad,
                        f.size / 2.0 + pad,
                    ));
                    drawn.push((q, *a, i as u32));
                }
                let extra = if side < 0.0 { 0.0005 } else { 0.0 };
                out.push((
                    4 * k + si,
                    Cand {
                        cost: base + extra,
                        boxes: Boxes::Many(boxes),
                        spot: Spot::Curved { letters: drawn },
                        callout: None,
                        outside: false,
                    },
                ));
            }
        }
    }
}

/// The farthest grid step an area's candidates go from its pole, either way.
const AREA_STEPS: i32 = 8;

/// The grid's cells nearest the pole first: of equal distances, as the 7 × 7 grid always was, by
/// ring, then column, then row; every cell up to `AREA_STEPS` either way, once (a smaller grid's are
/// these in this order).
fn grid_order() -> &'static [(i32, i32)] {
    static ORDER: std::sync::OnceLock<Vec<(i32, i32)>> = std::sync::OnceLock::new();
    ORDER.get_or_init(|| {
        let mut all: Vec<(i32, i32)> = (-AREA_STEPS..=AREA_STEPS)
            .flat_map(|i| (-AREA_STEPS..=AREA_STEPS).map(move |j| (i, j)))
            .collect();
        all.sort_by_key(|&(i, j)| (i * i + j * j, i, j));
        all
    })
}

/// How far the grid an area's candidates for a form inside it take round its pole goes: a quarter
/// of the label's width by half its height a step, as far as the area's box goes (3 to 8 steps either
/// way; only the pole when the label cannot fit the box at all); its steps.
fn area_reach(
    f: &Form,
    u: &Unit<'_>,
    pole: Vec2,
    bbox: [f64; 4],
    free: bool,
) -> ((i32, i32), (f64, f64)) {
    let c = u.cls;
    let (sx, sy) = (js_max(f.w / 4.0, 2.0), js_max(f.h / 2.0, 2.0));
    let [x0, y0, x1, y1] = bbox;
    let (bw, bh) = (f.w + 2.0 * c.halo, f.h + 2.0 * c.halo);
    let roomy = bw <= x1 - x0 && bh <= y1 - y0;
    let reach = |near: f64, far: f64, step: f64| -> i32 {
        let n = (js_max(near, far) / step).ceil();
        if n.is_finite() {
            (n as i32).clamp(3, AREA_STEPS)
        } else {
            3
        }
    };
    let reach = if roomy || free {
        (
            reach(pole.x - x0, x1 - pole.x, sx),
            reach(pole.y - y0, y1 - pole.y, sy),
        )
    } else {
        (0, 0)
    };
    (reach, (sx, sy))
}

/// The cells of the table from `at` on within `(ni, nj)`: the next one's place and cell.
fn next_cell(at: usize, (ni, nj): (i32, i32)) -> Option<(usize, (i32, i32))> {
    grid_order()
        .iter()
        .enumerate()
        .skip(at)
        .find(|(_, (i, j))| i.abs() <= ni && j.abs() <= nj)
        .map(|(k, c)| (k, *c))
}

/// An area's long axis for its turned candidates (eğik), when it has one worth turning to.
fn turned_axis(rings: &[Vec<Vec2>]) -> Option<f64> {
    let a = upright(long_axis(rings.first()?)?);
    (a.abs() > 1e-3).then_some(a)
}

/// An area's candidate at a grid cell, level or turned `angle`: none when it must fit and does not.
#[allow(clippy::too_many_arguments)]
fn area_cell(
    f: &Form,
    u: &Unit<'_>,
    rings: &[Vec<Vec2>],
    pole: Vec2,
    (sx, sy): (f64, f64),
    (i, j): (i32, i32),
    angle: f64,
    extra: f64,
) -> Option<Cand> {
    let c = u.cls;
    let (ux, uy) = (cos(angle), sin(angle));
    let (dx, dy) = (f64::from(i) * sx, f64::from(j) * sy);
    let cc = Vec2::new(pole.x + dx * ux - dy * uy, pole.y + dx * uy + dy * ux);
    let block = if angle == 0.0 {
        Obb::level(cc, f.w / 2.0 + c.halo, f.h / 2.0 + c.halo)
    } else {
        Obb::new(cc, angle, f.w / 2.0 + c.halo, f.h / 2.0 + c.halo)
    };
    let fits = box_inside(&block, rings);
    if !fits && c.inside {
        return None;
    }
    let cost = cell_least(i, j, extra) + if fits { 0.0 } else { OVERRUN };
    Some(straight(f, u, cc, angle, cost))
}

/// An area's `k`-th candidate outside it, round its box, with a callout to its pole.
fn outside_at(f: &Form, u: &Unit<'_>, bbox: [f64; 4], pole: Vec2, k: usize) -> Option<Cand> {
    let d = u.cls.distance;
    let s2 = d / std::f64::consts::SQRT_2;
    let [x0, y0, x1, y1] = bbox;
    let (xm, ym) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
    let (hw, hh) = (f.w / 2.0, f.h / 2.0);
    let spots = [
        Vec2::new(x1 + d + hw, ym),
        Vec2::new(x0 - d - hw, ym),
        Vec2::new(xm, y1 + d + hh),
        Vec2::new(xm, y0 - d - hh),
        Vec2::new(x1 + s2 + hw, y1 + s2 + hh),
        Vec2::new(x0 - s2 - hw, y1 + s2 + hh),
        Vec2::new(x1 + s2 + hw, y0 - s2 - hh),
        Vec2::new(x0 - s2 - hw, y0 - s2 - hh),
    ];
    let cc = *spots.get(k)?;
    let mut cand = straight(f, u, cc, 0.0, OUTSIDE + 0.001 * k as f64);
    cand.outside = true;
    cand.callout = Some((edge_toward(cc, hw, hh, pole), pole));
    Some(cand)
}

/// Where the line from a level box's middle toward `to` leaves the box (2 px out).
fn edge_toward(c: Vec2, hw: f64, hh: f64, to: Vec2) -> Vec2 {
    let (dx, dy) = (to.x - c.x, to.y - c.y);
    let t = js_min(
        if dx != 0.0 {
            (hw + 2.0) / dx.abs()
        } else {
            f64::INFINITY
        },
        if dy != 0.0 {
            (hh + 2.0) / dy.abs()
        } else {
            f64::INFINITY
        },
    );
    if !t.is_finite() || t >= 1.0 {
        return c;
    }
    Vec2::new(c.x + dx * t, c.y + dy * t)
}

/// A point's `k`-th candidate (çevresinde: QGIS's cartographic order; `POINT_ON`: üstünde).
fn point_at(f: &Form, u: &Unit<'_>, p: Vec2, r: f64, k: usize) -> Option<Cand> {
    if k == POINT_ON {
        return Some(straight(f, u, p, 0.0, 0.0001));
    }
    let c = u.cls;
    let (hw, hh) = (f.w / 2.0, f.h / 2.0);
    let dd = r + c.distance;
    let s2 = dd / std::f64::consts::SQRT_2;
    let spots = [
        Vec2::new(s2 + hw, s2 + hh),
        Vec2::new(-s2 - hw, s2 + hh),
        Vec2::new(s2 + hw, -s2 - hh),
        Vec2::new(-s2 - hw, -s2 - hh),
        Vec2::new(dd + hw, 0.0),
        Vec2::new(-dd - hw, 0.0),
        Vec2::new(f.w / 4.0, dd + hh),
        Vec2::new(-f.w / 4.0, -dd - hh),
    ];
    let d = *spots.get(k)?;
    Some(straight(
        f,
        u,
        Vec2::new(p.x + d.x, p.y + d.y),
        0.0,
        0.0001 + 0.001 * k as f64,
    ))
}

/// A point's candidate on it (üstünde), beside the eight round it.
const POINT_ON: usize = 8;

/// Some of a unit's candidates for a form before they are made: the least any of them costs and
/// their first's place in the made order (docs/adr/0212 §6: a unit's candidates are made only while
/// one of them may be the next cheapest; most labels take their first).
#[derive(Clone, Copy, Debug)]
struct Group {
    least: f64,
    order: usize,
    what: Make,
}

/// What a group makes.
#[derive(Clone, Copy, Debug)]
enum Make {
    Pin,
    Corner(Vec2),
    /// A point's `k`-th place (`POINT_ON`: on it).
    Point(usize),
    /// An area's `k`-th place outside it.
    Outside(usize),
    /// A path's `k`-th middle.
    Path(usize, f64),
    /// An area's grid cell, level or turned.
    Cell {
        i: i32,
        j: i32,
        angle: f64,
        extra: f64,
    },
}

/// A turned cell's cost over its level one's, and where turned cells come in the made order.
const TURNED: f64 = 0.0005;
const TURNED_ORDER: usize = 1000;

/// A grid cell's least cost (its own when the label fits there): the bound its candidate's cost starts from.
fn cell_least(i: i32, j: i32, extra: f64) -> f64 {
    0.0001 + 0.0001 * f64::from(i * i + j * j) + extra
}

/// A path's least cost below its middle's: a parallel label's straightness can be a rounding
/// under nothing.
const PATH_SLACK: f64 = 1e-12;

/// A unit's candidates for a form as groups, the cheapest bound first (of equal bounds, as made),
/// and the grid's steps of an area.
fn plan(f: &Form, u: &Unit<'_>, out: &mut Vec<Group>) -> (f64, f64) {
    out.clear();
    let mut steps = (0.0, 0.0);
    let one = |least: f64, order: usize, what: Make| Group { least, order, what };
    if u.pin.is_some() {
        out.push(one(0.0, 0, Make::Pin));
        return steps;
    }
    match &u.geo {
        Geo::Point { p, .. } => match u.cls.point {
            PointMode::Corner => out.push(one(0.0001, 0, Make::Corner(*p))),
            PointMode::Center => out.push(one(0.0001, 0, Make::Point(POINT_ON))),
            PointMode::Around => {
                out.extend((0..8).map(|k| one(0.0001 + 0.001 * k as f64, k, Make::Point(k))))
            }
        },
        Geo::Corner { tl } => out.push(one(0.0001, 0, Make::Corner(*tl))),
        Geo::Path { s0, s1, .. } => {
            let (len, mid) = (s1 - s0, (s0 + s1) / 2.0);
            for (k, t) in middles(*s0, *s1, f.w).into_iter().enumerate() {
                let place = 0.0001 + 0.001 * (t - mid).abs() / len;
                out.push(one(place - PATH_SLACK, 4 * k, Make::Path(k, t)));
            }
        }
        Geo::Area { rings, pole, bbox } => {
            if f.outside {
                out.extend((0..8).map(|k| one(OUTSIDE + 0.001 * k as f64, k, Make::Outside(k))));
            } else {
                let free = matches!(u.cls.area, AreaMode::Free | AreaMode::Parcel);
                let (reach, grid_steps) = area_reach(f, u, *pole, *bbox, free);
                steps = grid_steps;
                let turned = if free { turned_axis(rings) } else { None };
                let mut at = 0;
                while let Some((k, (i, j))) = next_cell(at, reach) {
                    at = k + 1;
                    out.push(one(
                        cell_least(i, j, 0.0),
                        k,
                        Make::Cell {
                            i,
                            j,
                            angle: 0.0,
                            extra: 0.0,
                        },
                    ));
                    if let Some(a) = turned {
                        out.push(one(
                            cell_least(i, j, TURNED),
                            TURNED_ORDER + k,
                            Make::Cell {
                                i,
                                j,
                                angle: a,
                                extra: TURNED,
                            },
                        ));
                    }
                }
            }
        }
    }
    out.sort_by(|a, b| a.least.total_cmp(&b.least).then(a.order.cmp(&b.order)));
    steps
}

/// A group's candidates, each with its place in the made order.
/// The path's walk backwards is made once for all of a unit's middles (`back`).
fn make(
    g: &Group,
    f: &Form,
    u: &Unit<'_>,
    font: Font,
    steps: (f64, f64),
    back: &std::cell::OnceCell<Walk>,
    out: &mut Vec<(usize, Cand)>,
) {
    match (g.what, &u.geo) {
        (Make::Pin, _) => {
            if let Some(pin) = u.pin {
                out.push((0, straight(f, u, pin.at, pin.angle, 0.0)));
            }
        }
        (Make::Corner(tl), _) => out.push((0, corner(f, u, tl))),
        (Make::Point(k), Geo::Point { p, r }) => {
            out.extend(point_at(f, u, *p, *r, k).map(|c| (k, c)))
        }
        (Make::Outside(k), Geo::Area { bbox, pole, .. }) => {
            out.extend(outside_at(f, u, *bbox, *pole, k).map(|c| (k, c)))
        }
        (
            Make::Path(k, t),
            Geo::Path {
                walk,
                s0,
                s1,
                interior,
            },
        ) => path_at(f, u, font, (walk, *s0, *s1, *interior), back, k, t, out),
        (Make::Cell { i, j, angle, extra }, Geo::Area { rings, pole, .. }) => {
            out.extend(
                area_cell(f, u, rings, *pole, steps, (i, j), angle, extra).map(|c| (g.order, c)),
            );
        }
        _ => {}
    }
}

/// A unit's candidates in a form, all of them, cheapest first (of equal costs, as made).
fn cands(f: &Form, u: &Unit<'_>, font: Font) -> Vec<Cand> {
    let mut groups = Vec::new();
    let steps = plan(f, u, &mut groups);
    let mut made = Vec::new();
    let back = std::cell::OnceCell::new();
    for g in &groups {
        make(g, f, u, font, steps, &back, &mut made);
    }
    made.sort_by(|a, b| a.1.cost.total_cmp(&b.1.cost).then(a.0.cmp(&b.0)));
    made.into_iter().map(|(_, c)| c).collect()
}

/// A candidate made and costed: its cost with its obstacles', its own and its place in the made order.
#[derive(Debug)]
struct Costed {
    total: f64,
    own: f64,
    order: usize,
    cand: Cand,
}

impl Costed {
    fn key(&self) -> (f64, f64, usize) {
        (self.total, self.own, self.order)
    }
}

/// Keys compared as the full list is sorted: by the cost with the obstacles', then the cost alone,
/// then the made order.
fn before(a: (f64, f64, usize), b: (f64, f64, usize)) -> std::cmp::Ordering {
    a.0.total_cmp(&b.0)
        .then(a.1.total_cmp(&b.1))
        .then(a.2.cmp(&b.2))
}

impl PartialEq for Costed {
    fn eq(&self, other: &Self) -> bool {
        before(self.key(), other.key()).is_eq()
    }
}

impl Eq for Costed {}

impl PartialOrd for Costed {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Costed {
    // The heap is a max-heap: the cheapest is the greatest.
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        before(other.key(), self.key())
    }
}

/// A unit's candidates in a form that no obstacle forbids, cheapest first (of equal costs, by their
/// own cost, then as made: the full list's order), made as they are asked for. A made candidate
/// comes out once nothing still unmade can be cheaper: its key below the next group's bound.
struct Lazy {
    source: Source,
    steps: (f64, f64),
    heap: std::collections::BinaryHeap<Costed>,
    made: Vec<(usize, Cand)>,
    /// A path's walk backwards, made once when a middle reads it so.
    back: std::cell::OnceCell<Walk>,
    /// A group list kept while an area's grid is read.
    spare: Vec<Group>,
}

/// Where a unit's groups come from: a few made at once, or an area's grid read from the table as
/// it is asked for (its turned cells only once one may be next: most areas take their pole).
enum Source {
    List(Vec<Group>, usize),
    Grid {
        reach: (i32, i32),
        free: bool,
        /// The table's next level and turned cells' places.
        level: usize,
        turned: usize,
        /// The long axis turned to: unknown until a turned cell may be next.
        axis: Option<Option<f64>>,
    },
}

impl Lazy {
    fn new(f: &Form, u: &Unit<'_>) -> Lazy {
        let mut lazy = Lazy {
            source: Source::List(Vec::new(), 0),
            steps: (0.0, 0.0),
            heap: std::collections::BinaryHeap::new(),
            made: Vec::new(),
            back: std::cell::OnceCell::new(),
            spare: Vec::new(),
        };
        lazy.reset(f, u);
        lazy
    }

    /// The same for another unit's form, its buffers kept (a placing asks this for every unit).
    fn reset(&mut self, f: &Form, u: &Unit<'_>) {
        self.heap.clear();
        self.made.clear();
        self.back = std::cell::OnceCell::new();
        if let Source::List(groups, _) =
            std::mem::replace(&mut self.source, Source::List(Vec::new(), 0))
        {
            self.spare = groups;
        }
        match &u.geo {
            Geo::Area { pole, bbox, .. } if u.pin.is_none() && !f.outside => {
                let free = matches!(u.cls.area, AreaMode::Free | AreaMode::Parcel);
                let (reach, steps) = area_reach(f, u, *pole, *bbox, free);
                self.steps = steps;
                self.source = Source::Grid {
                    reach,
                    free,
                    level: 0,
                    turned: 0,
                    axis: (!free).then_some(None),
                };
            }
            _ => {
                let mut groups = std::mem::take(&mut self.spare);
                self.steps = plan(f, u, &mut groups);
                self.source = Source::List(groups, 0);
            }
        }
    }

    /// The next group's bound, without taking it (an unknown turned cell's counts: a bound may be low).
    fn bound(&self) -> Option<(f64, usize)> {
        match &self.source {
            Source::List(groups, next) => groups.get(*next).map(|g| (g.least, g.order)),
            Source::Grid {
                reach,
                level,
                turned,
                axis,
                ..
            } => {
                let l = next_cell(*level, *reach).map(|(k, (i, j))| (cell_least(i, j, 0.0), k));
                let t = (*axis != Some(None))
                    .then(|| {
                        next_cell(*turned, *reach)
                            .map(|(k, (i, j))| (cell_least(i, j, TURNED), TURNED_ORDER + k))
                    })
                    .flatten();
                match (l, t) {
                    (Some(a), Some(b)) => {
                        Some(if before((a.0, a.0, a.1), (b.0, b.0, b.1)).is_le() {
                            a
                        } else {
                            b
                        })
                    }
                    (a, b) => a.or(b),
                }
            }
        }
    }

    /// Takes the next group (an area's long axis worked out once a turned cell is next).
    fn take(&mut self, u: &Unit<'_>) -> Option<Group> {
        match &mut self.source {
            Source::List(groups, next) => {
                let g = groups.get(*next).copied();
                *next += 1;
                g
            }
            Source::Grid {
                reach,
                free,
                level,
                turned,
                axis,
            } => loop {
                let l = next_cell(*level, *reach);
                let t = if *axis == Some(None) {
                    None
                } else {
                    next_cell(*turned, *reach)
                };
                let level_first = match (l, t) {
                    (Some((ka, (ia, ja))), Some((kb, (ib, jb)))) => before(
                        (cell_least(ia, ja, 0.0), cell_least(ia, ja, 0.0), ka),
                        (
                            cell_least(ib, jb, TURNED),
                            cell_least(ib, jb, TURNED),
                            TURNED_ORDER + kb,
                        ),
                    )
                    .is_le(),
                    (Some(_), None) => true,
                    (None, Some(_)) => false,
                    (None, None) => return None,
                };
                if level_first {
                    let (k, (i, j)) = l?;
                    *level = k + 1;
                    return Some(Group {
                        least: cell_least(i, j, 0.0),
                        order: k,
                        what: Make::Cell {
                            i,
                            j,
                            angle: 0.0,
                            extra: 0.0,
                        },
                    });
                }
                let a = match *axis {
                    Some(a) => a,
                    None => {
                        let found = if *free {
                            match &u.geo {
                                Geo::Area { rings, .. } => turned_axis(rings),
                                _ => None,
                            }
                        } else {
                            None
                        };
                        *axis = Some(found);
                        found
                    }
                };
                let Some(a) = a else {
                    continue;
                };
                let (k, (i, j)) = t?;
                *turned = k + 1;
                return Some(Group {
                    least: cell_least(i, j, TURNED),
                    order: TURNED_ORDER + k,
                    what: Make::Cell {
                        i,
                        j,
                        angle: a,
                        extra: TURNED,
                    },
                });
            },
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn next(
        &mut self,
        f: &Form,
        u: &Unit<'_>,
        font: Font,
        obstacles: &Obstacles,
        seen: &mut Vec<u32>,
        hit: &mut Vec<f64>,
    ) -> Option<Cand> {
        loop {
            if let Some(top) = self.heap.peek() {
                let ready = self
                    .bound()
                    .is_none_or(|(least, order)| before(top.key(), (least, least, order)).is_lt());
                if ready {
                    return self.heap.pop().map(|c| c.cand);
                }
            }
            let g = self.take(u)?;
            make(&g, f, u, font, self.steps, &self.back, &mut self.made);
            for (order, mut c) in self.made.drain(..) {
                let own = c.cost;
                if u.pin.is_none() {
                    match obstacles.cost(&c.boxes, u.id, u.cls.priority, seen, hit) {
                        None => continue,
                        Some(extra) => c.cost += extra,
                    }
                }
                self.heap.push(Costed {
                    total: c.cost,
                    own,
                    order,
                    cand: c,
                });
            }
        }
    }
}

/// The corner label: its first line's left middle 8 px right of and 14 px under the box's top left.
fn corner(f: &Form, u: &Unit<'_>, tl: Vec2) -> Cand {
    let c = Vec2::new(tl.x + 8.0 + f.w / 2.0, tl.y - 14.0 - (f.h - f.size) / 2.0);
    straight(f, u, c, 0.0, 0.0001)
}

/// One unit's state while placing.
#[derive(Clone, Debug, Default)]
struct Slot {
    placed: Option<(usize, usize)>,
    forced: bool,
}

/// The result of a placing: the labels, placed first in their order, then
/// (on request) the unplaced and the hidden.
pub struct Outcome {
    pub labels: Vec<Label>,
}

/// Places the scene's units (docs/adr/0212 §3.6).
pub fn place(scene: &Scene<'_>) -> Outcome {
    let units = &scene.units;
    let margin = 256.0;
    let mut room = Room::new(scene.width, scene.height, margin);
    room.insert(&scene.fixed, FIXED);
    let obstacles = Obstacles::new(scene.width, scene.height, margin, scene.obstacles.clone());
    let mut order: Vec<usize> = (0..units.len()).filter(|&i| !units[i].hidden).collect();
    order.sort_by(|&a, &b| {
        let (u, v) = (&units[a], &units[b]);
        v.pin
            .is_some()
            .cmp(&u.pin.is_some())
            .then(v.cls.priority.cmp(&u.cls.priority))
            .then(u.rank.cmp(&v.rank))
            .then(u.class.cmp(&v.class))
            .then(u.order.cmp(&v.order))
            .then(u.chunk.cmp(&v.chunk))
    });
    let mut forms_of: Vec<Option<Vec<Form>>> = vec![None; units.len()];
    let mut slots: Vec<Slot> = vec![Slot::default(); units.len()];
    let mut placed_cands: Vec<Option<Cand>> = vec![None; units.len()];
    let mut texts = Texts {
        on: units.iter().any(|u| u.cls.duplicates.is_some()),
        by: HashMap::new(),
    };
    let (mut seen, mut hit, mut blockers) = (Vec::new(), Vec::new(), Vec::new());

    // The candidates of a unit's form that no obstacle forbids, with their obstacles' costs, cheapest first:
    // all of them (the improvement's).
    let valid = |u: &Unit<'_>, f: &Form, seen: &mut Vec<u32>, hit: &mut Vec<f64>| -> Vec<Cand> {
        let mut lazy = Lazy::new(f, u);
        let mut list = Vec::new();
        while let Some(c) = lazy.next(f, u, scene.font, &obstacles, seen, hit) {
            list.push(c);
        }
        list
    };
    let middle = |c: &Cand| -> Vec2 {
        match &c.spot {
            Spot::Straight { c, .. } => *c,
            Spot::Curved { letters, .. } => {
                let n = letters.len().max(1) as f64;
                let (sx, sy) = letters
                    .iter()
                    .fold((0.0, 0.0), |(x, y), (p, _, _)| (x + p.x, y + p.y));
                Vec2::new(sx / n, sy / n)
            }
        }
    };
    let duplicate = |texts: &Texts<'_>, u: &Unit<'_>, c: &Cand, me: usize| -> bool {
        let Some(d) = u.cls.duplicates else {
            return false;
        };
        let m = middle(c);
        texts.by.get(u.text).is_some_and(|list| {
            list.iter()
                .any(|(p, owner)| *owner != me && norm(p.x - m.x, p.y - m.y) < d)
        })
    };

    // The candidates a label's form has, kept for the improvement (a form's are worked out once).
    let mut kept: HashMap<(usize, usize), Vec<Cand>> = HashMap::new();
    let mut lazy: Option<Lazy> = None;
    for &i in &order {
        let u = &units[i];
        let size = u.cls.size_at(scene.scale);
        let fs: &Vec<Form> = forms_of[i].get_or_insert_with(|| forms(u, scene.font, size));
        let mut done = false;
        for (fi, f) in fs.iter().enumerate() {
            // The form's candidates as they are asked for: most labels take their first.
            let lazy = match &mut lazy {
                Some(l) => {
                    l.reset(f, u);
                    l
                }
                None => lazy.insert(Lazy::new(f, u)),
            };
            // A pinned label and one that may always cover another: at the first form's best place.
            if u.pin.is_some() || u.cls.overlap == Overlap::Always {
                if let Some(c) = lazy.next(f, u, scene.font, &obstacles, &mut seen, &mut hit) {
                    room.insert(&c.boxes, i as u32);
                    texts.add(u.text, || middle(&c), i);
                    placed_cands[i] = Some(c);
                    slots[i].placed = Some((fi, 0));
                    slots[i].forced = u.pin.is_none();
                    break;
                }
                continue;
            }
            // An unplaced label's first form's candidates, all of them by the end: the improvement's.
            let mut tried = Vec::new();
            while let Some(c) = lazy.next(f, u, scene.font, &obstacles, &mut seen, &mut hit) {
                if room.free(&c.boxes) && !duplicate(&texts, u, &c, i) {
                    room.insert(&c.boxes, i as u32);
                    texts.add(u.text, || middle(&c), i);
                    placed_cands[i] = Some(c);
                    slots[i].placed = Some((fi, tried.len()));
                    done = true;
                    break;
                }
                tried.push(c);
            }
            if done {
                break;
            }
            if fi == 0 {
                kept.insert((i, 0), tried);
            }
        }
    }

    // The improvement: an unplaced label takes a place one movable label leaves for another of its own.
    for &i in &order {
        if slots[i].placed.is_some() || units[i].pin.is_some() {
            continue;
        }
        let u = &units[i];
        let Some(first) = forms_of[i].as_ref().and_then(|fs| fs.first()) else {
            continue;
        };
        let mine = kept
            .remove(&(i, 0))
            .unwrap_or_else(|| valid(u, first, &mut seen, &mut hit));
        'cands: for c in mine.into_iter().take(IMPROVE_TRIES) {
            room.blockers(&c.boxes, &mut blockers);
            if blockers.len() != 1 || blockers[0] == FIXED {
                continue;
            }
            let v = blockers[0] as usize;
            let other = &units[v];
            if other.pin.is_some() || slots[v].forced || other.cls.priority > u.cls.priority {
                continue;
            }
            let Some((vf, vc)) = slots[v].placed else {
                continue;
            };
            let (Some(old), Some(vform)) = (
                placed_cands[v].clone(),
                forms_of[v].as_ref().and_then(|fs| fs.get(vf)),
            ) else {
                continue;
            };
            let gone = room.take(v as u32);
            texts.remove(other.text, v);
            if duplicate(&texts, u, &c, i) {
                room.revive(v as u32, gone);
                texts.add(other.text, || middle(&old), v);
                continue;
            }
            // The unplaced label where the other was: the other's other places are tried against its
            // boxes as if they were in the room (they go in only when one is found).
            texts.add(u.text, || middle(&c), i);
            let moved = kept
                .entry((v, vf))
                .or_insert_with(|| valid(other, vform, &mut seen, &mut hit));
            for (k, cv) in moved.iter().enumerate() {
                if k == vc
                    || !room.free(&cv.boxes)
                    || meets(&cv.boxes, &c.boxes)
                    || duplicate(&texts, other, cv, v)
                {
                    continue;
                }
                room.insert(&c.boxes, i as u32);
                room.insert(&cv.boxes, v as u32);
                texts.add(other.text, || middle(cv), v);
                placed_cands[v] = Some(cv.clone());
                slots[v].placed = Some((vf, k));
                placed_cands[i] = Some(c.clone());
                slots[i].placed = Some((0, 0));
                break 'cands;
            }
            // Nothing: as it was.
            texts.remove(u.text, i);
            room.revive(v as u32, gone);
            texts.add(other.text, || middle(&old), v);
        }
    }

    // Those that may cover another, at their best place.
    for &i in &order {
        let u = &units[i];
        if slots[i].placed.is_some() || u.cls.overlap != Overlap::IfNeeded {
            continue;
        }
        if let Some(c) = forms_of[i]
            .as_ref()
            .and_then(|fs| fs.first())
            .and_then(|f| valid(u, f, &mut seen, &mut hit).into_iter().next())
        {
            room.insert(&c.boxes, i as u32);
            placed_cands[i] = Some(c);
            slots[i].placed = Some((0, 0));
            slots[i].forced = true;
        }
    }

    let mut labels = Vec::new();
    let mask = |u: &Unit<'_>| if u.cls.mask { MASKED } else { 0 };
    for &i in &order {
        let u = &units[i];
        let Some(c) = placed_cands[i].take() else {
            continue;
        };
        let (fi, _) = slots[i].placed.unwrap_or((0, 0));
        let Some(f) = forms_of[i].as_ref().and_then(|fs| fs.get(fi)) else {
            continue;
        };
        let mut state = mask(u);
        if u.pin.is_some() {
            state |= PINNED;
        }
        if slots[i].forced && u.cls.overlap == Overlap::IfNeeded {
            state |= OVERLAPPING;
        }
        labels.push(label(i, u, f, c, state));
    }
    if scene.options.unplaced {
        for &i in &order {
            let u = &units[i];
            if slots[i].placed.is_some() {
                continue;
            }
            if let Some(f) = forms_of[i].as_ref().and_then(|fs| fs.first())
                && let Some(c) = cands(f, u, scene.font).into_iter().next()
            {
                labels.push(label(i, u, f, c, UNPLACED | mask(u)));
            }
        }
    }
    if scene.options.hidden {
        for (i, u) in units.iter().enumerate() {
            if !u.hidden {
                continue;
            }
            let f = forms(u, scene.font, u.cls.size_at(scene.scale));
            if let Some(f) = f.first()
                && let Some(c) = cands(f, u, scene.font).into_iter().next()
            {
                labels.push(label(i, u, f, c, HIDDEN | mask(u)));
            }
        }
    }
    Outcome { labels }
}

fn label(i: usize, u: &Unit<'_>, f: &Form, c: Cand, mut state: u32) -> Label {
    if matches!(c.spot, Spot::Curved { .. }) {
        state |= CURVED;
    }
    if c.outside {
        state |= OUTSIDE_AREA;
    }
    let callout = c.callout.or_else(|| {
        let min = u.cls.callout?;
        let target = u.target?;
        let Spot::Straight { c: m, angle } = &c.spot else {
            return None;
        };
        // From the box's edge (in its frame) toward the object.
        let (ux, uy) = (cos(*angle), sin(*angle));
        let (dx, dy) = (target.x - m.x, target.y - m.y);
        let local = Vec2::new(dx * ux + dy * uy, -dx * uy + dy * ux);
        let e = edge_toward(Vec2::new(0.0, 0.0), f.w / 2.0, f.h / 2.0, local);
        let from = Vec2::new(m.x + e.x * ux - e.y * uy, m.y + e.x * uy + e.y * ux);
        (norm(target.x - from.x, target.y - from.y) >= min && !c.boxes[0].contains(target))
            .then_some((from, target))
    });
    let drawn = match c.spot {
        Spot::Straight { c, angle } => Drawn::Straight {
            c,
            angle,
            lines: straight_lines(f, u.cls.align),
        },
        Spot::Curved { letters } => Drawn::Curved {
            text: f.lines.first().cloned().unwrap_or_default(),
            letters,
        },
    };
    Label {
        unit: i,
        drawn,
        w: f.w,
        h: f.h,
        size: f.size,
        boxes: c.boxes.to_vec(),
        callout,
        state,
    }
}
