//! Grids, columns, stairs and ramps (docs/adr/0343 §2): their drawing from their fields, and the checks a stair's and a
//! ramp's fields are held to (docs/adr/0343 §2, `CAD-ARCH-35`). Nothing here says a stair or ramp meets a regulation.
//!
//! - A grid: its line or arc with its own line type (dash-dot), a circle bubble past each end it has one at, touching the
//!   end along its tangent, and its label centred in the bubble.
//! - A column: a rectangle or a circle round its centre, filled when it says so, its tag and place beside its corner.
//! - A stair or a ramp: its walking line's band (half its width each side, mitred at the turns), a landing at every turn
//!   (its length along the walking line, half each side of the turn), a stair's riser lines from its start a tread apart
//!   with a riser that falls inside a landing moved to the landing's end, its cut line and the arrow up its walking line;
//!   a ramp's arrow and its levels at its ends.

use crate::api::json::{FromJson, Json, ToJson, field};
use crate::block::{Piece, PieceStroke};
use crate::display::fixed;
use crate::entity::{HatchPattern, Shape};
use crate::jsmath::{atan, cos, js_hypot, sin};
use crate::text::TextAlign;
use crate::vec2::Vec2;

/// A grid bubble's diameter on paper by default, millimetres.
pub const BUBBLE_MM: f64 = 8.0;
/// A label's height on paper, millimetres: a column's tag, a ramp's levels.
pub const LABEL_MM: f64 = 2.5;
/// A stair's tread number's height on paper, millimetres.
pub const NUMBER_MM: f64 = 2.0;
/// The arrow's head, millimetres on paper.
pub const HEAD_MM: f64 = 3.0;
/// The dot at the arrow's start: its radius on paper, millimetres.
pub const DOT_MM: f64 = 1.0;
/// Blondel's step rule: twice the riser and the tread together, metres (information only).
pub const STEP_RULE: (f64, f64) = (0.60, 0.65);
/// How far a ramp's given slope may be from its rise over its sloped length, per cent points.
pub const SLOPE_TOLERANCE: f64 = 0.1;
/// Arc lengths closer than this are the same, metres.
const EPS: f64 = 1e-9;

fn word(v: &Json, name: &str) -> Option<String> {
    match v.get(name) {
        Json::Str(s) => Some(s.clone()),
        _ => None,
    }
}

fn number(v: &Json, name: &str) -> Result<Option<f64>, String> {
    Option::<f64>::from_json(v.get(name))
}

macro_rules! to_json_flat {
    ($t:ty) => {
        impl ToJson for $t {
            fn write_json(&self, out: &mut String) {
                out.push('{');
                let mut first = true;
                crate::api::json::Flat::write_flat(self, out, &mut first);
                out.push('}');
            }
        }
    };
}

/// Which ends of a grid carry a bubble.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Bubbles {
    #[default]
    Both,
    Start,
    End,
    None,
}

impl Bubbles {
    fn of(s: &str) -> Option<Self> {
        Some(match s {
            "both" => Bubbles::Both,
            "start" => Bubbles::Start,
            "end" => Bubbles::End,
            "none" => Bubbles::None,
            _ => return None,
        })
    }

    fn name(self) -> &'static str {
        match self {
            Bubbles::Both => "both",
            Bubbles::Start => "start",
            Bubbles::End => "end",
            Bubbles::None => "none",
        }
    }

    fn at_start(self) -> bool {
        matches!(self, Bubbles::Both | Bubbles::Start)
    }

    fn at_end(self) -> bool {
        matches!(self, Bubbles::Both | Bubbles::End)
    }
}

/// A grid's fields beside its path.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GridSpec {
    pub label: String,
    pub bubbles: Bubbles,
    pub bubble_size: Option<f64>,
}

impl crate::api::json::Flat for GridSpec {
    const NAMES: &'static [&'static str] = &["name", "bubbles", "bubbleSize"];

    fn read_flat(v: &Json) -> Result<Self, String> {
        let bubbles = match word(v, "bubbles").as_deref() {
            None => Bubbles::Both,
            Some(s) => Bubbles::of(s).ok_or_else(|| format!("bilinmeyen balon yeri “{s}”"))?,
        };
        Ok(GridSpec {
            label: word(v, "name").unwrap_or_default(),
            bubbles,
            bubble_size: number(v, "bubbleSize")?,
        })
    }

    fn write_flat(&self, out: &mut String, first: &mut bool) {
        field(out, first, "name", self.label.as_str());
        if self.bubbles != Bubbles::Both {
            field(out, first, "bubbles", self.bubbles.name());
        }
        if let Some(s) = &self.bubble_size {
            field(out, first, "bubbleSize", s);
        }
    }
}
to_json_flat!(GridSpec);

/// A column's fields beside its centre.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ColumnSpec {
    pub round: bool,
    pub width: f64,
    pub depth: Option<f64>,
    pub rotation: f64,
    pub fill: bool,
    pub tag: Option<String>,
    pub place: Option<String>,
}

impl crate::api::json::Flat for ColumnSpec {
    const NAMES: &'static [&'static str] = &["shape", "width", "depth", "rotation", "fill", "tag", "place"];

    fn read_flat(v: &Json) -> Result<Self, String> {
        let round = match word(v, "shape").as_deref() {
            None | Some("rect") => false,
            Some("round") => true,
            Some(s) => return Err(format!("bilinmeyen kolon biçimi “{s}”")),
        };
        Ok(ColumnSpec {
            round,
            width: number(v, "width")?.unwrap_or(0.0),
            depth: number(v, "depth")?,
            rotation: number(v, "rotation")?.unwrap_or(0.0),
            fill: matches!(v.get("fill"), Json::Bool(true)),
            tag: word(v, "tag"),
            place: word(v, "place"),
        })
    }

    fn write_flat(&self, out: &mut String, first: &mut bool) {
        if self.round {
            field(out, first, "shape", "round");
        }
        field(out, first, "width", &self.width);
        if let Some(d) = &self.depth {
            field(out, first, "depth", d);
        }
        if self.rotation != 0.0 {
            field(out, first, "rotation", &self.rotation);
        }
        if self.fill {
            field(out, first, "fill", &true);
        }
        if let Some(t) = &self.tag {
            field(out, first, "tag", t.as_str());
        }
        if let Some(t) = &self.place {
            field(out, first, "place", t.as_str());
        }
    }
}
to_json_flat!(ColumnSpec);

/// A stair's fields beside its walking line.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StairSpec {
    pub width: f64,
    pub risers: f64,
    pub riser_height: f64,
    pub tread: f64,
    pub landing: Option<f64>,
    pub cut: Option<f64>,
    pub numbering: bool,
}

impl crate::api::json::Flat for StairSpec {
    const NAMES: &'static [&'static str] = &["width", "risers", "riserHeight", "tread", "landing", "cut", "numbering"];

    fn read_flat(v: &Json) -> Result<Self, String> {
        Ok(StairSpec {
            width: number(v, "width")?.unwrap_or(0.0),
            risers: number(v, "risers")?.unwrap_or(0.0),
            riser_height: number(v, "riserHeight")?.unwrap_or(0.0),
            tread: number(v, "tread")?.unwrap_or(0.0),
            landing: number(v, "landing")?,
            cut: number(v, "cut")?,
            numbering: matches!(v.get("numbering"), Json::Bool(true)),
        })
    }

    fn write_flat(&self, out: &mut String, first: &mut bool) {
        field(out, first, "width", &self.width);
        field(out, first, "risers", &self.risers);
        field(out, first, "riserHeight", &self.riser_height);
        field(out, first, "tread", &self.tread);
        if let Some(l) = &self.landing {
            field(out, first, "landing", l);
        }
        if let Some(c) = &self.cut {
            field(out, first, "cut", c);
        }
        if self.numbering {
            field(out, first, "numbering", &true);
        }
    }
}
to_json_flat!(StairSpec);

impl StairSpec {
    pub fn landing(&self) -> f64 {
        self.landing.unwrap_or(self.width)
    }
}

/// A ramp's fields beside its centre line.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RampSpec {
    pub width: f64,
    pub rise: f64,
    pub slope: Option<f64>,
    pub base: Option<f64>,
    pub landing: Option<f64>,
}

impl crate::api::json::Flat for RampSpec {
    const NAMES: &'static [&'static str] = &["width", "rise", "slope", "base", "landing"];

    fn read_flat(v: &Json) -> Result<Self, String> {
        Ok(RampSpec {
            width: number(v, "width")?.unwrap_or(0.0),
            rise: number(v, "rise")?.unwrap_or(0.0),
            slope: number(v, "slope")?,
            base: number(v, "base")?,
            landing: number(v, "landing")?,
        })
    }

    fn write_flat(&self, out: &mut String, first: &mut bool) {
        field(out, first, "width", &self.width);
        field(out, first, "rise", &self.rise);
        if let Some(s) = &self.slope {
            field(out, first, "slope", s);
        }
        if let Some(b) = &self.base {
            field(out, first, "base", b);
        }
        if let Some(l) = &self.landing {
            field(out, first, "landing", l);
        }
    }
}
to_json_flat!(RampSpec);

impl RampSpec {
    pub fn landing(&self) -> f64 {
        self.landing.unwrap_or(self.width)
    }
}

// ── Pieces ──────────────────────────────────────────────────────────────────────────────────────────────────────

fn piece(shape: Shape) -> Piece {
    Piece {
        shape,
        color: None,
        line_weight: None,
        by_layer: Vec::new(),
        gates: Vec::new(),
        stroke: PieceStroke::default(),
        attribute: None,
    }
}

fn text(p: Vec2, s: String, height: f64, align: TextAlign) -> Shape {
    Shape::Text {
        p,
        text: s,
        height,
        rotation: 0.0,
        align: Some(align),
        width_factor: None,
        mask: None,
        box_width: None,
        line_spacing: None,
        runs: None,
        flow: None,
        face: Default::default(),
        path: None,
    }
}

fn line(a: Vec2, b: Vec2) -> Shape {
    Shape::Line { a, b }
}

fn polyline(pts: Vec<Vec2>) -> Shape {
    Shape::Polyline {
        pts,
        bulges: None,
        holes: None,
        parts: None,
        closed: false,
        widths: None,
    }
}

fn polygon(pts: Vec<Vec2>) -> Shape {
    Shape::Polygon {
        pts,
        bulges: None,
        holes: None,
        parts: None,
    }
}

fn unit(v: Vec2) -> Vec2 {
    let l = js_hypot(v.x, v.y);
    if l == 0.0 { v } else { Vec2::new(v.x / l, v.y / l) }
}

fn turned(v: Vec2, a: f64) -> Vec2 {
    let (s, c) = (sin(a), cos(a));
    Vec2::new(v.x * c - v.y * s, v.x * s + v.y * c)
}

/// A grid's ends' outward tangents: its start's back along it, its end's on along it (an arc's by its bulge).
pub fn grid_tangents(pts: &[Vec2], bulges: Option<&[f64]>) -> Option<(Vec2, Vec2)> {
    let [a, b] = pts else { return None };
    let d = unit(Vec2::new(b.x - a.x, b.y - a.y));
    let half = 2.0 * atan(bulges.and_then(|b| b.first().copied()).unwrap_or(0.0));
    let start = turned(d, -half);
    let end = turned(d, half);
    Some((Vec2::new(-start.x, -start.y), end))
}

/// A grid's pieces at 1:`scale`: its path (dash-dot), and a bubble and its label at each end it has one.
pub fn grid_pieces(pts: &[Vec2], bulges: Option<&[f64]>, spec: &GridSpec, scale: f64) -> Vec<Piece> {
    let mut out = Vec::new();
    let path = match bulges.filter(|b| b.iter().any(|x| *x != 0.0)) {
        Some(b) => Shape::Polyline {
            pts: pts.to_vec(),
            bulges: Some(b.to_vec()),
            holes: None,
            parts: None,
            closed: false,
            widths: None,
        },
        None if pts.len() == 2 => line(pts[0], pts[1]),
        None => return out,
    };
    let mut p = piece(path);
    p.stroke.line_type = Some("dashdot".into());
    out.push(p);
    let Some((back, on)) = grid_tangents(pts, bulges) else {
        return out;
    };
    let r = spec.bubble_size.unwrap_or(BUBBLE_MM) * scale / 2000.0;
    let mut bubble = |end: Vec2, t: Vec2| {
        let c = Vec2::new(end.x + t.x * r, end.y + t.y * r);
        out.push(piece(Shape::Circle { c, r }));
        out.push(piece(text(c, spec.label.clone(), r, TextAlign::MiddleCenter)));
    };
    if spec.bubbles.at_start() {
        bubble(pts[0], back);
    }
    if spec.bubbles.at_end() {
        bubble(pts[1], on);
    }
    out
}

/// A column's corners: a rectangle's four, counter-clockwise from its lower left before its turn.
pub fn column_corners(at: Vec2, spec: &ColumnSpec) -> Vec<Vec2> {
    let (w, d) = (spec.width / 2.0, spec.depth.unwrap_or(spec.width) / 2.0);
    [(-w, -d), (w, -d), (w, d), (-w, d)]
        .into_iter()
        .map(|(x, y)| {
            let p = turned(Vec2::new(x, y), spec.rotation);
            Vec2::new(at.x + p.x, at.y + p.y)
        })
        .collect()
}

/// A round column's ring for its fill: 64 points on its circle.
fn circle_ring(c: Vec2, r: f64) -> Vec<Vec2> {
    (0..64)
        .map(|k| {
            let a = f64::from(k) * crate::jsmath::TAU / 64.0;
            Vec2::new(c.x + r * cos(a), c.y + r * sin(a))
        })
        .collect()
}

/// A column's pieces at 1:`scale`: its fill, its outline, its tag and place beside its upper right.
pub fn column_pieces(at: Vec2, spec: &ColumnSpec, scale: f64) -> Vec<Piece> {
    let mut out = Vec::new();
    let r = spec.width / 2.0;
    let (ring, outline) = if spec.round {
        (circle_ring(at, r), Shape::Circle { c: at, r })
    } else {
        let c = column_corners(at, spec);
        (c.clone(), polygon(c))
    };
    if spec.fill {
        out.push(piece(Shape::Hatch {
            ring,
            holes: None,
            pattern: HatchPattern::user("solid", 0.0, 1.0),
            assoc: None,
            boundary: None,
        }));
    }
    out.push(piece(outline));
    let words: Vec<&str> = [spec.tag.as_deref(), spec.place.as_deref()]
        .into_iter()
        .flatten()
        .collect();
    if !words.is_empty() {
        let h = LABEL_MM * scale / 1000.0;
        let corner = if spec.round {
            Vec2::new(at.x + r, at.y + r)
        } else {
            let (w, d) = (spec.width / 2.0, spec.depth.unwrap_or(spec.width) / 2.0);
            Vec2::new(at.x + crate::jsmath::js_max(w, d), at.y + crate::jsmath::js_max(w, d))
        };
        let p = Vec2::new(corner.x + h * 0.4, corner.y + h * 0.4);
        out.push(piece(text(p, words.join("\n"), h, TextAlign::BottomLeft)));
    }
    out
}

/// A walking line measured: its segments' lengths and the arc length at each point.
pub struct Walk {
    pub pts: Vec<Vec2>,
    /// Arc length at each point.
    pub at: Vec<f64>,
}

impl Walk {
    pub fn of(pts: &[Vec2]) -> Walk {
        let mut at = vec![0.0];
        for w in pts.windows(2) {
            let l = js_hypot(w[1].x - w[0].x, w[1].y - w[0].y);
            at.push(at[at.len() - 1] + l);
        }
        Walk {
            pts: pts.to_vec(),
            at,
        }
    }

    pub fn length(&self) -> f64 {
        self.at.last().copied().unwrap_or(0.0)
    }

    /// The point at arc length `s` and its segment's direction (at a turn, the segment's it ends).
    pub fn point(&self, s: f64) -> (Vec2, Vec2) {
        let n = self.pts.len();
        let mut k = 0;
        while k + 2 < n && s > self.at[k + 1] {
            k += 1;
        }
        let (a, b) = (self.pts[k], self.pts[k + 1]);
        let l = self.at[k + 1] - self.at[k];
        let t = if l == 0.0 { 0.0 } else { (s - self.at[k]) / l };
        let d = unit(Vec2::new(b.x - a.x, b.y - a.y));
        (Vec2::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t), d)
    }

    /// The landings: at each turn, `length` along the walking line, half each side, within its ends.
    pub fn landings(&self, length: f64) -> Vec<(f64, f64)> {
        let s = self.length();
        (1..self.pts.len().saturating_sub(1))
            .map(|k| {
                let c = self.at[k];
                (
                    crate::jsmath::js_max(c - length / 2.0, 0.0),
                    crate::jsmath::js_min(c + length / 2.0, s),
                )
            })
            .collect()
    }
}

fn left(d: Vec2) -> Vec2 {
    Vec2::new(-d.y, d.x)
}

/// The walking line's band, `half` each side, mitred at the turns: its left side from the start, then its right side
/// back.
pub fn band(pts: &[Vec2], half: f64) -> Vec<Vec2> {
    let n = pts.len();
    if n < 2 {
        return Vec::new();
    }
    let dirs: Vec<Vec2> = pts
        .windows(2)
        .map(|w| unit(Vec2::new(w[1].x - w[0].x, w[1].y - w[0].y)))
        .collect();
    let side = |sign: f64| -> Vec<Vec2> {
        (0..n)
            .map(|i| {
                let p = pts[i];
                let (a, b) = (
                    left(dirs[i.saturating_sub(1).min(n - 2)]),
                    left(dirs[i.min(n - 2)]),
                );
                let m = if i == 0 || i == n - 1 {
                    if i == 0 { b } else { a }
                } else {
                    let dot = a.x * b.x + a.y * b.y;
                    if 1.0 + dot < 1e-12 {
                        b
                    } else {
                        Vec2::new((a.x + b.x) / (1.0 + dot), (a.y + b.y) / (1.0 + dot))
                    }
                };
                Vec2::new(p.x + m.x * half * sign, p.y + m.y * half * sign)
            })
            .collect()
    };
    let mut out = side(1.0);
    let mut right = side(-1.0);
    right.reverse();
    out.extend(right);
    out
}

/// A line across the band at arc length `s`.
fn across(walk: &Walk, s: f64, half: f64) -> Shape {
    let (p, d) = walk.point(s);
    let n = left(d);
    line(
        Vec2::new(p.x + n.x * half, p.y + n.y * half),
        Vec2::new(p.x - n.x * half, p.y - n.y * half),
    )
}

/// The arc lengths of a stair's risers in order, as many as its walking line holds (docs/adr/0343 §2): from its start
/// a tread apart; a riser that falls inside a landing moves to the landing's end. Then the risers that do not fit.
pub fn riser_stations(walk: &Walk, spec: &StairSpec) -> (Vec<f64>, u32) {
    let landings = walk.landings(spec.landing());
    let total = walk.length();
    let mut out = Vec::new();
    let mut s = 0.0;
    let risers = if spec.risers > 0.0 { spec.risers as u32 } else { 0 };
    let mut missing = 0;
    for _ in 0..risers {
        if let Some((_, b)) = landings.iter().find(|(a, b)| s > *a + EPS && s < *b - EPS) {
            s = *b;
        }
        if s > total + EPS {
            missing += 1;
        } else {
            out.push(s);
        }
        s += spec.tread;
    }
    (out, missing)
}

fn arrow(walk: &Walk, from: f64, to: f64, scale: f64) -> Vec<Shape> {
    let mut pts = vec![walk.point(from).0];
    for (k, &s) in walk.at.iter().enumerate() {
        if s > from + EPS && s < to - EPS {
            pts.push(walk.pts[k]);
        }
    }
    let (tip, d) = walk.point(to);
    pts.push(tip);
    let head = HEAD_MM * scale / 1000.0;
    let back = Vec2::new(-d.x, -d.y);
    let wing = |a: f64| {
        let w = turned(back, a);
        line(tip, Vec2::new(tip.x + w.x * head, tip.y + w.y * head))
    };
    let r = DOT_MM * scale / 1000.0;
    vec![
        polyline(pts),
        wing(crate::jsmath::PI / 6.0),
        wing(-crate::jsmath::PI / 6.0),
        Shape::Circle {
            c: walk.point(from).0,
            r,
        },
    ]
}

/// A stair's pieces at 1:`scale`: its band, its landings' edges, its risers up to its cut, the cut line, the arrow and
/// the numbers.
pub fn stair_pieces(pts: &[Vec2], spec: &StairSpec, scale: f64) -> Vec<Piece> {
    let mut out = Vec::new();
    if pts.len() < 2 {
        return out;
    }
    let walk = Walk::of(pts);
    let half = spec.width / 2.0;
    out.push(piece(polygon(band(pts, half))));
    for (a, b) in walk.landings(spec.landing()) {
        out.push(piece(across(&walk, a, half)));
        out.push(piece(across(&walk, b, half)));
    }
    let (stations, _) = riser_stations(&walk, spec);
    let cut = spec
        .cut
        .map(|c| (if c > 1.0 { c as usize } else { 1 }).min(stations.len()));
    let shown = cut.unwrap_or(stations.len());
    for &s in &stations[..shown] {
        out.push(piece(across(&walk, s, half)));
    }
    let end = match cut {
        Some(c) if c > 0 => {
            let s = stations[c - 1];
            let t = crate::jsmath::js_min(s + spec.tread, walk.length());
            let (p, d) = walk.point(s);
            let (q, e) = walk.point(t);
            let (n, m) = (left(d), left(e));
            out.push(piece(line(
                Vec2::new(p.x + n.x * half, p.y + n.y * half),
                Vec2::new(q.x - m.x * half, q.y - m.y * half),
            )));
            s
        }
        _ => walk.length(),
    };
    for s in arrow(&walk, 0.0, end, scale) {
        out.push(piece(s));
    }
    if spec.numbering {
        let h = NUMBER_MM * scale / 1000.0;
        for (i, &s) in stations[..shown].iter().enumerate() {
            let mid = s + spec.tread / 2.0;
            if mid > walk.length() + EPS {
                continue;
            }
            let (p, d) = walk.point(mid);
            let n = left(d);
            let at = Vec2::new(p.x + n.x * half / 2.0, p.y + n.y * half / 2.0);
            out.push(piece(text(at, (i + 1).to_string(), h, TextAlign::MiddleCenter)));
        }
    }
    out
}

/// A level written as the drawing shows it: its sign and two decimals (“+0.45”, “-0.30”, “+0.00”).
pub fn level_text(v: f64) -> String {
    let t = fixed(v.abs(), 2);
    if v < 0.0 && t != "0.00" {
        format!("-{t}")
    } else {
        format!("+{t}")
    }
}

/// A ramp's sloped length: its walking line less its landings.
pub fn ramp_run(walk: &Walk, spec: &RampSpec) -> f64 {
    let land: f64 = walk.landings(spec.landing()).iter().map(|(a, b)| b - a).sum();
    walk.length() - land
}

/// A ramp's pieces at 1:`scale`: its band, its landings' edges, the arrow up it and its levels at its ends.
pub fn ramp_pieces(pts: &[Vec2], spec: &RampSpec, scale: f64) -> Vec<Piece> {
    let mut out = Vec::new();
    if pts.len() < 2 {
        return out;
    }
    let walk = Walk::of(pts);
    let half = spec.width / 2.0;
    out.push(piece(polygon(band(pts, half))));
    for (a, b) in walk.landings(spec.landing()) {
        out.push(piece(across(&walk, a, half)));
        out.push(piece(across(&walk, b, half)));
    }
    for s in arrow(&walk, 0.0, walk.length(), scale) {
        out.push(piece(s));
    }
    let h = LABEL_MM * scale / 1000.0;
    let base = spec.base.unwrap_or(0.0);
    let (p0, d0) = walk.point(0.0);
    let (p1, d1) = walk.point(walk.length());
    out.push(piece(text(
        Vec2::new(p0.x - d0.x * h * 1.5, p0.y - d0.y * h * 1.5),
        level_text(base),
        h,
        TextAlign::MiddleCenter,
    )));
    out.push(piece(text(
        Vec2::new(p1.x + d1.x * h * 1.5, p1.y + d1.y * h * 1.5),
        level_text(base + spec.rise),
        h,
        TextAlign::MiddleCenter,
    )));
    out
}

// ── Checks ──────────────────────────────────────────────────────────────────────────────────────────────────────

/// What a stair's or ramp's fields do not fit: a code (the commands' warning) and its words.
#[derive(Clone, Debug, PartialEq)]
pub struct Check {
    pub code: &'static str,
    pub words: String,
}

/// A stair's checks (docs/adr/0343 §2): too short a walking line, too long one, the step rule (information only).
pub fn stair_checks(pts: &[Vec2], spec: &StairSpec) -> Vec<Check> {
    let mut out = Vec::new();
    if pts.len() < 2 {
        return out;
    }
    let walk = Walk::of(pts);
    let (stations, missing) = riser_stations(&walk, spec);
    if missing > 0 {
        out.push(Check {
            code: "stair_short",
            words: format!(
                "Yürüme çizgisi ({} m) {} rıhta yetmiyor: {missing} rıht sığmıyor. Yolu uzatın, basamağı kısaltın ya da rıht sayısını azaltın.",
                fixed(walk.length(), 2),
                spec.risers
            ),
        });
    } else if let Some(&last) = stations.last() {
        let extra = walk.length() - last;
        if extra > 1e-6 {
            out.push(Check {
                code: "stair_long",
                words: format!(
                    "Son rıhttan sonra yürüme çizgisinde {} m kalıyor; merdiven yoldan kısa.",
                    fixed(extra, 2)
                ),
            });
        }
    }
    let rule = 2.0 * spec.riser_height + spec.tread;
    if rule < STEP_RULE.0 - 1e-9 || rule > STEP_RULE.1 + 1e-9 {
        out.push(Check {
            code: "stair_step_rule",
            words: format!(
                "2 × rıht + basamak = {} m; Blondel'in oranı 0.60–0.65 m. Bu bir bilgidir, yönetmelik denetimi değildir.",
                fixed(rule, 3)
            ),
        });
    }
    out
}

/// A ramp's checks (docs/adr/0343 §2): its given slope against its rise over its sloped length.
pub fn ramp_checks(pts: &[Vec2], spec: &RampSpec) -> Vec<Check> {
    let mut out = Vec::new();
    if pts.len() < 2 {
        return out;
    }
    let run = ramp_run(&Walk::of(pts), spec);
    if run <= EPS {
        out.push(Check {
            code: "ramp_no_run",
            words: "Sahanlıklardan sonra rampanın eğimli kolu kalmıyor; yolu uzatın ya da sahanlığı kısaltın.".into(),
        });
        return out;
    }
    let actual = spec.rise / run * 100.0;
    if let Some(slope) = spec.slope
        && (slope - actual).abs() > SLOPE_TOLERANCE
    {
        out.push(Check {
            code: "ramp_slope_mismatch",
            words: format!(
                "Rampanın eğimi %{} verildi; yükseklik ve eğimli uzunlukla %{}.",
                fixed(slope, 2),
                fixed(actual, 2)
            ),
        });
    }
    out
}

/// The shape a grid, column, stair or ramp stands for where one shape is asked (vertices, edges, picking): a grid's
/// path, a column's outline, a stair's or ramp's band; none for another shape.
pub fn outline(shape: &Shape) -> Option<Shape> {
    Some(match shape {
        Shape::Grid { pts, bulges, .. } => match bulges.as_ref().filter(|b| b.iter().any(|x| *x != 0.0)) {
            Some(b) => Shape::Polyline {
                pts: pts.clone(),
                bulges: Some(b.clone()),
                holes: None,
                parts: None,
                closed: false,
                widths: None,
            },
            None => line(*pts.first()?, *pts.get(1)?),
        },
        Shape::Column { at, column } => {
            if column.round {
                Shape::Circle {
                    c: *at,
                    r: column.width / 2.0,
                }
            } else {
                polygon(column_corners(*at, column))
            }
        }
        Shape::Stair { pts, stair } => polygon(band(pts, stair.width / 2.0)),
        Shape::Ramp { pts, ramp } => polygon(band(pts, ramp.width / 2.0)),
        // A tag's middle: its frame is sized on paper (docs/adr/0344 §1).
        Shape::Tag { at, .. } => Shape::Point {
            p: *at,
            z: None,
            parts: None,
        },
        // A view mark's points: its bubbles are sized on paper (docs/adr/0345 §1); a detail's circle is in the world.
        Shape::ViewMark { points, mark } => match (mark.form, mark.radius, points.first()) {
            (crate::geom::viewmark::Form::Detail, Some(r), Some(&c)) => Shape::Circle { c, r },
            (_, _, Some(&p)) if points.len() == 1 => Shape::Point { p, z: None, parts: None },
            _ => Shape::Polyline {
                pts: points.clone(),
                bulges: None,
                holes: None,
                parts: None,
                closed: false,
                widths: None,
            },
        },
        _ => return None,
    })
}

/// Its pieces at 1:`scale`; none for another shape.
pub fn pieces(shape: &Shape, scale: f64) -> Vec<Piece> {
    match shape {
        Shape::Grid { pts, bulges, grid } => grid_pieces(pts, bulges.as_deref(), grid, scale),
        Shape::Column { at, column } => column_pieces(*at, column, scale),
        Shape::Stair { pts, stair } => stair_pieces(pts, stair, scale),
        Shape::Ramp { pts, ramp } => ramp_pieces(pts, ramp, scale),
        Shape::Tag { at, tag } => crate::geom::tag::pieces(*at, tag, scale),
        Shape::ViewMark { points, mark } => crate::geom::viewmark::pieces(points, mark, scale),
        _ => Vec::new(),
    }
}

/// A grid's, stair's or ramp's path; none for another shape.
pub fn path_pts(shape: &Shape) -> Option<&[Vec2]> {
    match shape {
        Shape::Grid { pts, .. } | Shape::Stair { pts, .. } | Shape::Ramp { pts, .. } => Some(pts),
        _ => None,
    }
}

/// The same grid, stair or ramp on another path.
pub fn with_pts(shape: &Shape, pts: Vec<Vec2>) -> Option<Shape> {
    Some(match shape {
        Shape::Grid { bulges, grid, .. } => Shape::Grid {
            pts,
            bulges: bulges.clone(),
            grid: grid.clone(),
        },
        Shape::Stair { stair, .. } => Shape::Stair {
            pts,
            stair: stair.clone(),
        },
        Shape::Ramp { ramp, .. } => Shape::Ramp {
            pts,
            ramp: ramp.clone(),
        },
        _ => return None,
    })
}

/// A grid's path as one edge: its segment, or its arc by its first bulge.
pub fn grid_edge(pts: &[Vec2], bulges: Option<&[f64]>) -> Option<crate::geom::intersect::Edge> {
    if pts.len() != 2 {
        return None;
    }
    crate::geom::bulge::bulge_path_edges(pts, bulges, false).into_iter().next()
}

/// Where two grids cross with their lines unbounded and their arcs as whole circles, the crossing nearest `near`
/// (docs/adr/0343 §3); none: parallel lines or circles apart.
pub fn grid_cross(
    a: &crate::geom::intersect::Edge,
    b: &crate::geom::intersect::Edge,
    near: Vec2,
) -> Option<Vec2> {
    use crate::geom::intersect::{Edge, circle_circle, line_circle_params, line_line};
    let on = |p: Vec2, q: Vec2, t: f64| Vec2::new(p.x + (q.x - p.x) * t, p.y + (q.y - p.y) * t);
    let found: Vec<Vec2> = match (*a, *b) {
        (Edge::Seg { a: p, b: q }, Edge::Seg { a: r, b: s }) => {
            line_line(p, q, r, s).map(|h| h.p).into_iter().collect()
        }
        (Edge::Seg { a: p, b: q }, Edge::Arc { c, r, .. })
        | (Edge::Arc { c, r, .. }, Edge::Seg { a: p, b: q }) => line_circle_params(p, q, c, r)
            .into_iter()
            .map(|t| on(p, q, t))
            .collect(),
        (Edge::Arc { c: c1, r: r1, .. }, Edge::Arc { c: c2, r: r2, .. }) => {
            circle_circle(c1, r1, c2, r2)
        }
    };
    let d = |p: &Vec2| js_hypot(p.x - near.x, p.y - near.y);
    found
        .into_iter()
        .filter(|p| p.x.is_finite() && p.y.is_finite())
        .min_by(|p, q| d(p).total_cmp(&d(q)))
}

/// Where two grids cross within both of them, in order along the first (Aks kesişimleri).
pub fn grid_crossings(a: &crate::geom::intersect::Edge, b: &crate::geom::intersect::Edge) -> Vec<Vec2> {
    let mut hits = crate::geom::intersect::intersect_edges(a, b);
    hits.sort_by(|h, k| h.t.total_cmp(&k.t));
    hits.into_iter().map(|h| h.p).collect()
}
