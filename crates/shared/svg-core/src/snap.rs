//! Snapping in the SVG editor (`apps/web/src/style/svg/snapping.ts`), like
//! the main CAD's object snaps: nodes (cusp and smooth), segment middles,
//! crossings of outlines, bounding box corners, edge middles and centres,
//! object centres, the foot of a perpendicular and tangent points (from the
//! point the tool started at), guides, and the canvas border and centre.
//! Fixed points sit in a grid of cells, outlines as chords in another;
//! crossings, perpendiculars and tangents are found only near the pointer.
//! Candidates are offered in the order the TypeScript offered them (its
//! Maps and Sets keep insertion order), so equal distances resolve alike.

use std::collections::HashMap;

use kentos_geometry_core::api::json::{FromJson, Json, read_field};
use kentos_geometry_core::geometry::Bounds;
use kentos_geometry_core::jsmath::{PI, cos, js_floor, js_hypot, js_max, js_min, or, sin};

use crate::bezier::{
    Cubic, bez, bez_deriv, flatten_cubic, flatten_sub_path_tol, nearest_on_cubic, ring_signed_area,
    segment_count, segment_cubic, segment_is_line,
};
use crate::model::{shape_box, to_path};
use crate::nodes::node_type_of;
use crate::shape::{Obj, Pt, SubPath, truthy};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Cusp,
    Smooth,
    Mid,
    Intersection,
    BboxCorner,
    BboxMid,
    BboxCentre,
    Centre,
    Perpendicular,
    Tangent,
    Guide,
    Page,
}

impl Kind {
    pub fn named(s: &str) -> Option<Kind> {
        Some(match s {
            "cusp" => Kind::Cusp,
            "smooth" => Kind::Smooth,
            "mid" => Kind::Mid,
            "intersection" => Kind::Intersection,
            "bboxCorner" => Kind::BboxCorner,
            "bboxMid" => Kind::BboxMid,
            "bboxCentre" => Kind::BboxCentre,
            "centre" => Kind::Centre,
            "perpendicular" => Kind::Perpendicular,
            "tangent" => Kind::Tangent,
            "guide" => Kind::Guide,
            "page" => Kind::Page,
            _ => return None,
        })
    }

    pub fn name(self) -> &'static str {
        match self {
            Kind::Cusp => "cusp",
            Kind::Smooth => "smooth",
            Kind::Mid => "mid",
            Kind::Intersection => "intersection",
            Kind::BboxCorner => "bboxCorner",
            Kind::BboxMid => "bboxMid",
            Kind::BboxCentre => "bboxCentre",
            Kind::Centre => "centre",
            Kind::Perpendicular => "perpendicular",
            Kind::Tangent => "tangent",
            Kind::Guide => "guide",
            Kind::Page => "page",
        }
    }

    /// The name the snap bar gives the kind (`SNAP_KINDS`).
    fn label(self) -> &'static str {
        match self {
            Kind::Cusp => "Köşe düğüm",
            Kind::Smooth => "Yumuşak düğüm",
            Kind::Mid => "Parça ortası",
            Kind::Intersection => "Kesişim",
            Kind::BboxCorner => "Kutu köşesi",
            Kind::BboxMid => "Kutu kenar ortası",
            Kind::BboxCentre => "Kutu merkezi",
            Kind::Centre => "Nesne merkezi",
            Kind::Perpendicular => "Dik",
            Kind::Tangent => "Teğet",
            Kind::Guide => "Kılavuz",
            Kind::Page => "Tuval kenarı ve ortası",
        }
    }

    /// Equal distances: the more specific snap wins.
    fn rank(self) -> u8 {
        match self {
            Kind::Cusp | Kind::Smooth => 0,
            Kind::Intersection | Kind::Guide => 1,
            Kind::Centre => 2,
            Kind::BboxCentre => 3,
            Kind::Mid | Kind::BboxCorner => 4,
            Kind::Page | Kind::BboxMid => 5,
            Kind::Perpendicular | Kind::Tangent => 6,
        }
    }
}

/// A snap: the point, its kind and name, and its distance from the pointer.
#[derive(Clone, Debug, PartialEq)]
pub struct SnapHit {
    pub p: Pt,
    pub kind: Kind,
    pub label: &'static str,
    pub d: f64,
}

impl kentos_geometry_core::api::json::ToJson for SnapHit {
    fn write_json(&self, out: &mut String) {
        use kentos_geometry_core::api::json::write_str;
        out.push_str("{\"p\":");
        self.p.write_json(out);
        out.push_str(",\"kind\":");
        write_str(out, self.kind.name());
        out.push_str(",\"label\":");
        write_str(out, self.label);
        out.push_str(",\"d\":");
        self.d.write_json(out);
        out.push('}');
    }
}

/// A guide: an endless line through (x, y) at `angle` degrees.
pub struct Guide {
    pub x: f64,
    pub y: f64,
    pub angle: f64,
}

kentos_geometry_core::json_struct!(Guide { x, y, angle });

/// What the index is built from (the page's `SnapSource`, its node filter as a list).
pub struct SnapSource {
    pub shapes: Vec<Obj>,
    pub guides: Vec<Guide>,
    pub width: f64,
    pub height: f64,
    pub kinds: Vec<Kind>,
    /// Shapes that move with the pointer (never snap to themselves).
    pub exclude: Vec<String>,
    /// Nodes that move (node dragging): shape id, sub-path, index.
    pub skip: Vec<(String, f64, f64)>,
}

impl FromJson for SnapSource {
    fn from_json(v: &Json) -> Result<SnapSource, String> {
        let page = v.get("page");
        let kinds: Vec<String> = read_field(v, "kinds")?;
        let skip: Vec<(String, f64, f64)> = match v.get("skip") {
            Json::Arr(items) => items
                .iter()
                .map(|t| {
                    let parts: Vec<Json> = Vec::from_json(t)?;
                    let id = String::from_json(parts.first().unwrap_or(&Json::Null))?;
                    let sub = f64::from_json(parts.get(1).unwrap_or(&Json::Null))?;
                    let index = f64::from_json(parts.get(2).unwrap_or(&Json::Null))?;
                    Ok((id, sub, index))
                })
                .collect::<Result<_, String>>()?,
            _ => Vec::new(),
        };
        Ok(SnapSource {
            shapes: read_field(v, "shapes")?,
            guides: Option::<Vec<Guide>>::from_json(v.get("guides"))?.unwrap_or_default(),
            width: read_field(page, "width")?,
            height: read_field(page, "height")?,
            kinds: kinds.iter().filter_map(|k| Kind::named(k)).collect(),
            exclude: Option::<Vec<String>>::from_json(v.get("exclude"))?.unwrap_or_default(),
            skip,
        })
    }
}

struct Fixed {
    p: Pt,
    kind: Kind,
    label: &'static str,
}

struct Seg {
    c: Cubic,
    line: bool,
}

struct Chord {
    a: Pt,
    b: Pt,
    seg: usize,
}

#[derive(Clone)]
struct Line {
    p: Pt,
    d: Pt,
    kind: Kind,
    label: &'static str,
    /// A finite line (a canvas edge) runs t ∈ [0, len]; a guide is endless.
    len: Option<f64>,
}

/// A number as `${x}` names it in a key (−0 is 0, NaN is one value).
fn num_key(x: f64) -> u64 {
    if x == 0.0 {
        0
    } else if x.is_nan() {
        f64::NAN.to_bits()
    } else {
        x.to_bits()
    }
}

/// Grid of cells holding item indices by their boxes; cells in the order they were made (a Map).
struct Cells {
    size: f64,
    order: Vec<Vec<usize>>,
    at: HashMap<(u64, u64), usize>,
}

impl Cells {
    fn new(size: f64) -> Cells {
        Cells {
            size,
            order: Vec::new(),
            at: HashMap::new(),
        }
    }

    fn add(&mut self, b: &Bounds, i: usize) {
        let s = self.size;
        let mut x = js_floor(b.min_x / s);
        while x <= js_floor(b.max_x / s) {
            let mut y = js_floor(b.min_y / s);
            while y <= js_floor(b.max_y / s) {
                let key = (num_key(x), num_key(y));
                match self.at.get(&key) {
                    Some(&k) => self.order[k].push(i),
                    None => {
                        self.at.insert(key, self.order.len());
                        self.order.push(vec![i]);
                    }
                }
                y += 1.0;
            }
            x += 1.0;
        }
    }

    /// Items in the cells around p (a Set: first appearance order).
    fn near(&self, p: Pt, r: f64, seen: &mut Vec<bool>, out: &mut Vec<usize>) {
        out.clear();
        let s = self.size;
        let x0 = js_floor((p[0] - r) / s);
        let x1 = js_floor((p[0] + r) / s);
        let y0 = js_floor((p[1] - r) / s);
        let y1 = js_floor((p[1] + r) / s);
        let mut put = |i: usize, out: &mut Vec<usize>| {
            if i >= seen.len() {
                seen.resize(i + 1, false);
            }
            if !seen[i] {
                seen[i] = true;
                out.push(i);
            }
        };
        // A search wider than the grid is worth: at most a few hundred cells.
        if (x1 - x0 + 1.0) * (y1 - y0 + 1.0) > 400.0 {
            for list in &self.order {
                for &i in list {
                    put(i, out);
                }
            }
        } else {
            let mut x = x0;
            while x <= x1 {
                let mut y = y0;
                while y <= y1 {
                    if let Some(&k) = self.at.get(&(num_key(x), num_key(y))) {
                        for &i in &self.order[k] {
                            put(i, out);
                        }
                    }
                    y += 1.0;
                }
                x += 1.0;
            }
        }
        for &i in out.iter() {
            seen[i] = false;
        }
    }
}

pub struct SnapIndex {
    kinds: Vec<Kind>,
    fixed: Vec<Fixed>,
    segs: Vec<Seg>,
    chords: Vec<Chord>,
    lines: Vec<Line>,
    fixed_cells: Cells,
    chord_cells: Cells,
}

impl SnapIndex {
    fn on(&self, k: Kind) -> bool {
        self.kinds.contains(&k)
    }

    fn add_fixed(&mut self, p: Pt, kind: Kind, label: &'static str) {
        if !self.on(kind) || !p[0].is_finite() || !p[1].is_finite() {
            return;
        }
        let b = Bounds {
            min_x: p[0],
            min_y: p[1],
            max_x: p[0],
            max_y: p[1],
        };
        self.fixed_cells.add(&b, self.fixed.len());
        self.fixed.push(Fixed { p, kind, label });
    }

    pub fn new(src: &SnapSource) -> Result<SnapIndex, String> {
        let (width, height) = (src.width, src.height);
        let extent = js_max(js_max(width, height), 1e-6);
        let mut ix = SnapIndex {
            kinds: src.kinds.clone(),
            fixed: Vec::new(),
            segs: Vec::new(),
            chords: Vec::new(),
            lines: Vec::new(),
            fixed_cells: Cells::new(extent / 32.0),
            chord_cells: Cells::new(extent / 32.0),
        };
        let tol = extent * 1e-4;
        for s in &src.shapes {
            let id = s.text("id").unwrap_or("");
            if s.is("hidden") || src.exclude.iter().any(|e| e == id) {
                continue;
            }
            let b = shape_box(s)?;
            let cx = (b.min_x + b.max_x) / 2.0;
            let cy = (b.min_y + b.max_y) / 2.0;
            for p in [
                [b.min_x, b.min_y],
                [b.max_x, b.min_y],
                [b.min_x, b.max_y],
                [b.max_x, b.max_y],
            ] {
                ix.add_fixed(p, Kind::BboxCorner, Kind::BboxCorner.label());
            }
            for p in [[cx, b.min_y], [cx, b.max_y], [b.min_x, cy], [b.max_x, cy]] {
                ix.add_fixed(p, Kind::BboxMid, Kind::BboxMid.label());
            }
            ix.add_fixed([cx, cy], Kind::BboxCentre, Kind::BboxCentre.label());
            let kind = s.kind();
            if kind == "text" {
                ix.add_fixed([s.num("x"), s.num("y")], Kind::Centre, "Yazı başlangıcı");
                continue;
            }
            if kind == "ellipse" {
                ix.add_fixed(
                    [s.num("cx"), s.num("cy")],
                    Kind::Centre,
                    Kind::Centre.label(),
                );
            }
            if kind == "rect" {
                ix.add_fixed(
                    [s.num("x") + s.num("w") / 2.0, s.num("y") + s.num("h") / 2.0],
                    Kind::Centre,
                    Kind::Centre.label(),
                );
            }
            let path = to_path(s);
            if path.kind() != "path" {
                continue;
            }
            let subs = path.subs()?;
            if kind == "path"
                && let Some(c) = centroid(&subs)
            {
                ix.add_fixed(c, Kind::Centre, "Ağırlık merkezi");
            }
            for (si, sp) in subs.iter().enumerate() {
                let skip = |i: usize| {
                    kind == "path"
                        && src
                            .skip
                            .iter()
                            .any(|(sid, ss, ii)| sid == id && *ss == si as f64 && *ii == i as f64)
                };
                let n = sp.nodes.len();
                for (i, node) in sp.nodes.iter().enumerate() {
                    if skip(i) {
                        continue;
                    }
                    let open = !sp.closed && (i == 0 || i == n - 1);
                    let k = if open || node_type_of(sp, i).as_deref() == Some("cusp") {
                        Kind::Cusp
                    } else {
                        Kind::Smooth
                    };
                    ix.add_fixed([node.x, node.y], k, k.label());
                }
                let count = segment_count(sp);
                for i in 0..count {
                    if skip(i) || skip((i + 1) % n) {
                        continue;
                    }
                    let c = segment_cubic(sp, i);
                    let line = segment_is_line(sp, i);
                    ix.add_fixed(bez(&c, 0.5), Kind::Mid, Kind::Mid.label());
                    let seg = ix.segs.len();
                    ix.segs.push(Seg { c, line });
                    let pts = if line {
                        vec![c[0], c[3]]
                    } else {
                        flatten_cubic(&c, tol).0
                    };
                    for k in 1..pts.len() {
                        let a = pts[k - 1];
                        let bb = pts[k];
                        let chord_box = Bounds {
                            min_x: js_min(a[0], bb[0]),
                            min_y: js_min(a[1], bb[1]),
                            max_x: js_max(a[0], bb[0]),
                            max_y: js_max(a[1], bb[1]),
                        };
                        ix.chord_cells.add(&chord_box, ix.chords.len());
                        ix.chords.push(Chord { a, b: bb, seg });
                    }
                }
            }
        }
        // The canvas: corners, edge middles and centre; its edges as lines.
        for p in [[0.0, 0.0], [width, 0.0], [0.0, height], [width, height]] {
            ix.add_fixed(p, Kind::Page, "Tuval köşesi");
        }
        for p in [
            [width / 2.0, 0.0],
            [width / 2.0, height],
            [0.0, height / 2.0],
            [width, height / 2.0],
        ] {
            ix.add_fixed(p, Kind::Page, "Tuval kenar ortası");
        }
        ix.add_fixed([width / 2.0, height / 2.0], Kind::Page, "Tuval ortası");
        if ix.on(Kind::Page) {
            let edge = |p: Pt, d: Pt, len: f64| Line {
                p,
                d,
                kind: Kind::Page,
                label: "Tuval kenarı",
                len: Some(len),
            };
            ix.lines.push(edge([0.0, 0.0], [1.0, 0.0], width));
            ix.lines.push(edge([0.0, height], [1.0, 0.0], width));
            ix.lines.push(edge([0.0, 0.0], [0.0, 1.0], height));
            ix.lines.push(edge([width, 0.0], [0.0, 1.0], height));
        }
        if ix.on(Kind::Guide) {
            let gl: Vec<Line> = src
                .guides
                .iter()
                .map(|g| {
                    let a = (g.angle * PI) / 180.0;
                    Line {
                        p: [g.x, g.y],
                        d: [cos(a), sin(a)],
                        kind: Kind::Guide,
                        label: "Kılavuz",
                        len: None,
                    }
                })
                .collect();
            ix.lines.extend(gl.iter().cloned());
            // Guides crossing each other.
            for i in 0..gl.len() {
                for j in i + 1..gl.len() {
                    if let Some(x) = line_cross(&gl[i], &gl[j]) {
                        ix.add_fixed(x, Kind::Guide, "Kılavuz kesişimi");
                    }
                }
            }
        }
        Ok(ix)
    }

    /// The best snap within `r` of p. Points (nodes, crossings, centres …)
    /// win over lines (guides, canvas edges); `from` is where the tool started,
    /// for perpendicular and tangent snaps.
    pub fn query(&self, p: Pt, r: f64, from: Option<Pt>) -> Option<SnapHit> {
        let mut best: Option<SnapHit> = None;
        let offer = |best: &mut Option<SnapHit>, q: Pt, kind: Kind, label: &'static str| {
            let d = js_hypot(q[0] - p[0], q[1] - p[1]);
            if d > r {
                return;
            }
            let better = match best {
                None => true,
                Some(b) => {
                    d < b.d - 1e-9 * r
                        || ((d - b.d).abs() <= 1e-9 * r && kind.rank() < b.kind.rank())
                }
            };
            if better {
                *best = Some(SnapHit {
                    p: q,
                    kind,
                    label,
                    d,
                });
            }
        };
        let mut seen = Vec::new();
        let mut ids = Vec::new();
        self.fixed_cells.near(p, r, &mut seen, &mut ids);
        for &i in &ids {
            let f = &self.fixed[i];
            offer(&mut best, f.p, f.kind, f.label);
        }
        let mut near_ids = Vec::new();
        self.chord_cells.near(p, r, &mut seen, &mut near_ids);
        let near: Vec<&Chord> = near_ids.iter().map(|&i| &self.chords[i]).collect();
        if self.on(Kind::Intersection) {
            for i in 0..near.len() {
                for j in i + 1..near.len() {
                    if near[i].seg == near[j].seg {
                        continue;
                    }
                    if let Some(x) = seg_cross(near[i].a, near[i].b, near[j].a, near[j].b) {
                        offer(&mut best, x, Kind::Intersection, Kind::Intersection.label());
                    }
                }
            }
            for l in &self.lines {
                if l.kind != Kind::Guide {
                    continue;
                }
                for c in &near {
                    if let Some(x) = seg_line_cross(c.a, c.b, l) {
                        offer(&mut best, x, Kind::Intersection, "Kılavuzla kesişim");
                    }
                }
            }
        }
        if let Some(from) = from
            && (self.on(Kind::Perpendicular) || self.on(Kind::Tangent))
        {
            let mut segs: Vec<usize> = Vec::new();
            for c in &near {
                if !segs.contains(&c.seg) {
                    segs.push(c.seg);
                }
            }
            for s in segs {
                let seg = &self.segs[s];
                if self.on(Kind::Perpendicular)
                    && let Some(q) = perpendicular_foot(seg, from, p)
                {
                    offer(
                        &mut best,
                        q,
                        Kind::Perpendicular,
                        Kind::Perpendicular.label(),
                    );
                }
                if self.on(Kind::Tangent)
                    && !seg.line
                    && let Some(q) = tangent_point(&seg.c, from, p)
                {
                    offer(&mut best, q, Kind::Tangent, Kind::Tangent.label());
                }
            }
        }
        if best.is_some() {
            return best;
        }
        // Lines last: the nearest point on a guide or a canvas edge.
        for l in &self.lines {
            let mut t = (p[0] - l.p[0]) * l.d[0] + (p[1] - l.p[1]) * l.d[1];
            if let Some(len) = l.len {
                t = js_max(0.0, js_min(len, t));
            }
            offer(
                &mut best,
                [l.p[0] + l.d[0] * t, l.p[1] + l.d[1] * t],
                l.kind,
                l.label,
            );
        }
        best
    }
}

/// Area centroid of the closed sub-paths (flattened); None when none has area.
fn centroid(subs: &[SubPath]) -> Option<Pt> {
    let mut a = 0.0;
    let mut x = 0.0;
    let mut y = 0.0;
    for sp in subs {
        if !sp.closed {
            continue;
        }
        let ring = flatten_sub_path_tol(sp, 0.01);
        let ra = ring_signed_area(&ring);
        if ra.abs() < 1e-12 {
            continue;
        }
        let mut cx = 0.0;
        let mut cy = 0.0;
        let n = ring.len();
        for i in 0..n {
            let p = ring[i];
            let q = ring[(i + 1) % n];
            let k = p[0] * q[1] - q[0] * p[1];
            cx += (p[0] + q[0]) * k;
            cy += (p[1] + q[1]) * k;
        }
        x += cx / 6.0;
        y += cy / 6.0;
        a += ra;
    }
    (a.abs() > 1e-12).then(|| [x / a, y / a])
}

fn seg_cross(a: Pt, b: Pt, c: Pt, d: Pt) -> Option<Pt> {
    let rx = b[0] - a[0];
    let ry = b[1] - a[1];
    let sx = d[0] - c[0];
    let sy = d[1] - c[1];
    let den = rx * sy - ry * sx;
    if den.abs() < 1e-18 {
        return None;
    }
    let t = ((c[0] - a[0]) * sy - (c[1] - a[1]) * sx) / den;
    let u = ((c[0] - a[0]) * ry - (c[1] - a[1]) * rx) / den;
    (t >= 0.0 && t <= 1.0 && u >= 0.0 && u <= 1.0).then(|| [a[0] + rx * t, a[1] + ry * t])
}

fn seg_line_cross(a: Pt, b: Pt, l: &Line) -> Option<Pt> {
    let n: Pt = [-l.d[1], l.d[0]];
    let fa = (a[0] - l.p[0]) * n[0] + (a[1] - l.p[1]) * n[1];
    let fb = (b[0] - l.p[0]) * n[0] + (b[1] - l.p[1]) * n[1];
    if fa == fb || fa * fb > 0.0 {
        return None;
    }
    let t = fa / (fa - fb);
    Some([a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t])
}

fn line_cross(a: &Line, b: &Line) -> Option<Pt> {
    let den = a.d[0] * b.d[1] - a.d[1] * b.d[0];
    if den.abs() < 1e-12 {
        return None;
    }
    let t = ((b.p[0] - a.p[0]) * b.d[1] - (b.p[1] - a.p[1]) * b.d[0]) / den;
    Some([a.p[0] + a.d[0] * t, a.p[1] + a.d[1] * t])
}

/// Newton on f(t) = 0 from t0, staying in [0, 1]; None if it does not settle.
fn solve(f: &dyn Fn(f64) -> f64, t0: f64) -> Option<f64> {
    let mut t = t0;
    for _ in 0..30 {
        let v = f(t);
        let h = 1e-7;
        let d = (f(js_min(1.0, t + h)) - f(js_max(0.0, t - h)))
            / (js_min(1.0, t + h) - js_max(0.0, t - h));
        if !d.is_finite() || d.abs() < 1e-18 {
            return None;
        }
        let next = js_min(1.0, js_max(0.0, t - v / d));
        if (next - t).abs() < 1e-12 {
            return (f(next).abs() < 1e-6).then_some(next);
        }
        t = next;
    }
    (f(t).abs() < 1e-6).then_some(t)
}

/// Foot of the perpendicular from `from` onto the segment, the one near p.
fn perpendicular_foot(seg: &Seg, from: Pt, p: Pt) -> Option<Pt> {
    let c = &seg.c;
    if seg.line {
        let dx = c[3][0] - c[0][0];
        let dy = c[3][1] - c[0][1];
        let l2 = dx * dx + dy * dy;
        if l2 < 1e-18 {
            return None;
        }
        let t = ((from[0] - c[0][0]) * dx + (from[1] - c[0][1]) * dy) / l2;
        return (t >= 0.0 && t <= 1.0).then(|| [c[0][0] + dx * t, c[0][1] + dy * t]);
    }
    let t0 = nearest_on_cubic(c, p).t;
    let f = |u: f64| {
        let q = bez(c, u);
        let d = bez_deriv(c, u);
        let l = or(js_hypot(d[0], d[1]), 1.0);
        ((q[0] - from[0]) * d[0] + (q[1] - from[1]) * d[1]) / l
    };
    solve(&f, t0).map(|t| bez(c, t))
}

/// Point near p where the line from `from` touches the curve.
fn tangent_point(c: &Cubic, from: Pt, p: Pt) -> Option<Pt> {
    let t0 = nearest_on_cubic(c, p).t;
    let f = |u: f64| {
        let q = bez(c, u);
        let d = bez_deriv(c, u);
        let l = or(js_hypot(d[0], d[1]), 1.0);
        ((q[0] - from[0]) * d[1] - (q[1] - from[1]) * d[0]) / l
    };
    solve(&f, t0).map(|t| bez(c, t))
}

/// Whether a JSON value is truthy (kept for the source's optional flags).
pub fn flag(v: &Json) -> bool {
    truthy(v)
}
