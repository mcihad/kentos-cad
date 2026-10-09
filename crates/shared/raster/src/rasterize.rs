//! Rasterleştir (docs/adr/0234 §3): objects burnt onto a grid's cells. A
//! closed shape burns the cells whose centres it holds (docs/adr/0233 §6's
//! areas); an open one the cells any of its points lies in (a cell's
//! half-open square in cell space): its arcs followed by docs/adr/0233 §6's
//! 0.1 mm chords (the ellipse's and the spline's by the geometry core's),
//! each chord taken to cell space and its cells found row by row, the
//! chord's x on a row's edge placed between whole numbers by the geometry
//! core's exact orientation; a point the cell it lies in. An object counts
//! once in a cell. Where several burn a cell the overlap rule says which
//! value stays.

use kentos_contracts::RasterSample;
use kentos_formats::raster::samples::Samples;
use kentos_geometry_core::entity::{Shape, ellipse_geom};
use kentos_geometry_core::geom::arrangement::Ring;
use kentos_geometry_core::geom::bulge::{bulge_arc, bulge_at};
use kentos_geometry_core::geom::ellipse::is_full_ellipse;
use kentos_geometry_core::geom::intersect::Edge;
use kentos_geometry_core::ops::edges::entity_edges;
use kentos_geometry_core::predicates::orient2d;
use kentos_geometry_core::vec2::Vec2;
use serde::Deserialize;

use crate::areas::{Areas, Span, arc_inner_points};
use crate::dd::Dd;
use crate::grid::Grid;
use crate::inputs::{floor_i, place_in};
use crate::par;
use crate::points::number_of;

/// Which value stays where objects overlap.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Overlap {
    Last,
    First,
    Max,
    Min,
    Sum,
    Count,
}

/// The result's samples.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum BurnSample {
    F32,
    F64,
    I32,
    U8,
}

impl BurnSample {
    pub fn sample(self) -> RasterSample {
        match self {
            BurnSample::F32 => RasterSample::F32,
            BurnSample::F64 => RasterSample::F64,
            BurnSample::I32 => RasterSample::I32,
            BurnSample::U8 => RasterSample::U8,
        }
    }

    /// The value a cell without one holds.
    pub fn nodata(self) -> f64 {
        match self {
            BurnSample::F32 | BurnSample::F64 => f64::NAN,
            BurnSample::I32 => -2_147_483_648.0,
            BurnSample::U8 => 255.0,
        }
    }

    /// A value as the samples hold it (whole types: halves away from zero);
    /// none when it does not fit or is the one kept for no value.
    pub fn fit(self, v: f64) -> Option<f64> {
        match self {
            BurnSample::F32 => (v as f32).is_finite().then_some(v),
            BurnSample::F64 => v.is_finite().then_some(v),
            BurnSample::I32 => {
                let r = v.round();
                (-2_147_483_647.0..=2_147_483_647.0)
                    .contains(&r)
                    .then_some(r)
            }
            BurnSample::U8 => {
                let r = v.round();
                (0.0..=254.0).contains(&r).then_some(r)
            }
        }
    }
}

/// How an object burns.
enum Kind {
    /// A closed shape: the cells whose centres it holds.
    Area(Box<Shape>),
    /// An open shape's chord paths (world).
    Paths(Vec<Vec<Vec2>>),
    Points(Vec<Vec2>),
}

/// Whether a shape burns as an area, as chords or as points; none for the others.
fn kind_of(s: Shape) -> Option<Kind> {
    match s {
        Shape::Polygon { .. } | Shape::Circle { .. } | Shape::Hatch { .. } => {
            Some(Kind::Area(Box::new(s)))
        }
        Shape::Spline { closed: true, .. } => Some(Kind::Area(Box::new(s))),
        Shape::Ellipse {
            c,
            major,
            ratio,
            t0,
            t1,
        } if is_full_ellipse(&ellipse_geom(c, major, ratio, t0, t1)) => {
            Some(Kind::Area(Box::new(s)))
        }
        Shape::Line { .. }
        | Shape::Polyline { .. }
        | Shape::Arc { .. }
        | Shape::Ellipse { .. }
        | Shape::Spline { .. } => {
            let mut paths = Vec::new();
            chord_paths(&s, &mut paths);
            Some(Kind::Paths(paths))
        }
        Shape::Point { p, parts, .. } => Some(Kind::Points(
            std::iter::once(p)
                .chain(parts.into_iter().flatten().map(|q| q.p))
                .collect(),
        )),
        _ => None,
    }
}

/// A shape's chord paths (world): its vertices with the 0.1 mm chords'
/// points of its arcs between them; the ellipse's and spline's chords the core's.
pub(crate) fn chord_paths(s: &Shape, out: &mut Vec<Vec<Vec2>>) {
    let path = |pts: &[Vec2], bulges: Option<&[f64]>, closed: bool| -> Vec<Vec2> {
        let n = pts.len();
        let mut p = Vec::with_capacity(n + 1);
        if n == 0 {
            return p;
        }
        p.push(pts[0]);
        let edges = if closed { n } else { n - 1 };
        for k in 0..edges {
            let (a, b) = (pts[k], pts[(k + 1) % n]);
            if let Some(arc) = bulge_arc(a, b, bulge_at(bulges, k)) {
                p.extend(arc_inner_points(arc.c, arc.r, arc.a0, arc.sweep));
            }
            p.push(b);
        }
        p
    };
    match s {
        Shape::Line { a, b } => out.push(vec![*a, *b]),
        Shape::Polyline {
            pts, bulges, parts, ..
        } => {
            out.push(path(pts, bulges.as_deref(), false));
            for p in parts.iter().flatten() {
                out.push(path(&p.pts, p.bulges.as_deref(), false));
            }
        }
        Shape::Polygon {
            pts,
            bulges,
            holes,
            parts,
        } => {
            let mut rings = |pts: &[Vec2], bulges: Option<&[f64]>, holes: Option<&Vec<Ring>>| {
                out.push(path(pts, bulges, true));
                for h in holes.into_iter().flatten() {
                    out.push(path(&h.pts, h.bulges.as_deref(), true));
                }
            };
            rings(pts, bulges.as_deref(), holes.as_ref());
            for p in parts.iter().flatten() {
                rings(&p.pts, p.bulges.as_deref(), p.holes.as_ref());
            }
        }
        Shape::Point { .. } => {}
        _ => {
            for e in entity_edges(s) {
                match e {
                    Edge::Seg { a, b } => out.push(vec![a, b]),
                    Edge::Arc { c, r, a0, sweep } => {
                        let at = |t: f64| Vec2::new(c.x + r * libm::cos(t), c.y + r * libm::sin(t));
                        let mut p = vec![at(a0)];
                        p.extend(arc_inner_points(c, r, a0, sweep));
                        p.push(at(a0 + sweep));
                        out.push(p);
                    }
                }
            }
        }
    }
}

/// The objects read: each one's value (from its field's text or the constant) and how it burns.
pub struct Objects {
    items: Vec<(u32, Kind)>,
    values: Vec<f64>,
    /// Objects whose value is not a number.
    pub unread: usize,
}

impl Objects {
    /// `shapes` with their values: each one's text read by `kentos.statistics/1`, or `constant`.
    pub fn new(
        shapes: Vec<Shape>,
        texts: Option<&[Option<String>]>,
        constant: f64,
    ) -> Result<Objects, String> {
        if texts.is_none() && !constant.is_finite() {
            return Err("Sabit değer bir sayı olmalı.".into());
        }
        let mut out = Objects {
            items: Vec::new(),
            values: vec![f64::NAN; shapes.len()],
            unread: 0,
        };
        for (o, s) in shapes.into_iter().enumerate() {
            let v = match texts {
                None => Some(constant),
                Some(t) => number_of(t.get(o).and_then(|x| x.as_deref())),
            };
            let Some(v) = v else {
                out.unread += 1;
                continue;
            };
            if let Some(k) = kind_of(s) {
                out.values[o] = v;
                out.items.push((o as u32, k));
            }
        }
        Ok(out)
    }

    /// Objects to burn.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// The box of every object's chord points and points: least x, least y, largest x, largest y.
    pub fn bounds(&self) -> Option<[f64; 4]> {
        let mut b: Option<[f64; 4]> = None;
        let mut add = |p: Vec2| {
            if !(p.x.is_finite() && p.y.is_finite()) {
                return;
            }
            b = Some(match b {
                None => [p.x, p.y, p.x, p.y],
                Some([a, c, d, e]) => [a.min(p.x), c.min(p.y), d.max(p.x), e.max(p.y)],
            });
        };
        let mut paths = Vec::new();
        for (_, k) in &self.items {
            match k {
                Kind::Area(s) => {
                    paths.clear();
                    chord_paths(s, &mut paths);
                    paths.iter().flatten().for_each(|&p| add(p));
                }
                Kind::Paths(ps) => ps.iter().flatten().for_each(|&p| add(p)),
                Kind::Points(ps) => ps.iter().for_each(|&p| add(p)),
            }
        }
        b
    }
}

/// A chord in cell space, its lower end first (`a.y ≤ b.y`), its object and its rows on the grid.
#[derive(Clone, Copy, Debug)]
struct Chord {
    a: Vec2,
    b: Vec2,
    object: u32,
    first: u32,
    last: u32,
}

/// ⌊x⌋ held within −1 … n (all that matters for cells 0 … n − 1).
#[inline]
fn floor_in(x: f64, n: i64) -> i64 {
    if !(x >= 0.0) {
        -1
    } else if x >= n as f64 {
        n
    } else {
        floor_i(x)
    }
}

/// Where a rising chord (a.y < b.y, a.x ≠ b.x) crosses y = `k`: ⌊X(k)⌋
/// held within −1 … `w`, and whether X(k) is whole. sign(X(k) − m) is
/// sign(orient2d(a, b, (m, k))), exact.
fn floor_on(a: Vec2, b: Vec2, k: i64, w: i64) -> (i64, bool) {
    let kf = k as f64;
    let side = |m: i64| orient2d(a, b, Vec2::new(m as f64, kf));
    let guess = a.x + (kf - a.y) * ((b.x - a.x) / (b.y - a.y));
    let mut m = floor_in(guess, w);
    while m > -1 && side(m) < 0.0 {
        m -= 1;
    }
    while m < w && side(m + 1) >= 0.0 {
        m += 1;
    }
    (m, side(m) == 0.0)
}

/// The columns (within 0 … w − 1) of the cells in row `j` the chord has a point in.
fn row_range(c: &Chord, j: i64, w: i64) -> Option<(i64, i64)> {
    let (a, b) = (c.a, c.b);
    let (ja, jb) = (floor_i(a.y), floor_i(b.y));
    if j < ja || j > jb {
        return None;
    }
    let (lo, hi) = if a.y == b.y {
        let (x0, x1) = if a.x <= b.x { (a.x, b.x) } else { (b.x, a.x) };
        (floor_in(x0, w), floor_in(x1, w))
    } else if a.x == b.x {
        let i = floor_in(a.x, w);
        (i, i)
    } else {
        // The row's points: y from max(j, a.y) up to min(j + 1, b.y), the
        // top left out when it is the next row's edge.
        let low = if j == ja {
            floor_in(a.x, w)
        } else {
            floor_on(a, b, j, w).0
        };
        if a.x < b.x {
            let top = if j == jb {
                floor_in(b.x, w)
            } else {
                // x < X(j + 1): ⌈X(j + 1)⌉ − 1.
                let (m, whole) = floor_on(a, b, j + 1, w);
                if whole { m - 1 } else { m }
            };
            (low, top)
        } else {
            let top = if j == jb {
                floor_in(b.x, w)
            } else {
                floor_on(a, b, j + 1, w).0
            };
            (top, low)
        }
    };
    let (i0, i1) = (lo.max(0), hi.min(w - 1));
    (i0 <= i1).then_some((i0, i1))
}

/// The chord `p`–`q` (cell space) with its rows within 0 … h − 1; none when off the grid's rows.
fn chord(p: Vec2, q: Vec2, object: u32, h: u32) -> Option<Chord> {
    if !(p.x.is_finite() && p.y.is_finite() && q.x.is_finite() && q.y.is_finite()) {
        return None;
    }
    let (a, b) = if p.y <= q.y { (p, q) } else { (q, p) };
    let h = i64::from(h);
    let (first, last) = (floor_in(a.y, h).max(0), floor_in(b.y, h).min(h - 1));
    (first <= last).then_some(Chord {
        a,
        b,
        object,
        first: first as u32,
        last: last as u32,
    })
}

/// The cells (cell space, within `w` × `h`) the closed segment `p`–`q` has
/// a point in, row by row; a cell's half-open square owns its lower sides.
pub fn segment_cells(p: Vec2, q: Vec2, (w, h): (u32, u32), out: &mut dyn FnMut(u32, u32)) {
    let Some(c) = chord(p, q, 0, h) else {
        return;
    };
    for j in c.first..=c.last {
        if let Some((i0, i1)) = row_range(&c, i64::from(j), i64::from(w)) {
            for i in i0..=i1 {
                out(i as u32, j);
            }
        }
    }
}

/// A cell's objects so far: the last's and the first's index, the least
/// and largest value, the sum, the count.
#[derive(Clone, Copy)]
struct Acc {
    last: u32,
    first: u32,
    lo: f64,
    hi: f64,
    sum: Dd,
    count: u32,
}

const NONE: Acc = Acc {
    last: 0,
    first: u32::MAX,
    lo: f64::INFINITY,
    hi: f64::NEG_INFINITY,
    sum: Dd::ZERO,
    count: 0,
};

/// One row: its values (NaN: none), its empty cells, the objects it burnt.
type RowOut = (Vec<f64>, u64, Vec<u32>);

/// The objects on the grid.
pub struct Burn {
    areas: Areas,
    /// Each area's object.
    area_objects: Vec<u32>,
    /// By first row.
    chords: Vec<Chord>,
    /// (row, column, object), sorted.
    points: Vec<(u32, u32, u32)>,
    values: Vec<f64>,
    overlap: Overlap,
    sample: BurnSample,
    width: u32,
    /// The objects burnt and whether each has had a cell.
    burnt: Vec<u32>,
    touched: Vec<bool>,
}

impl Burn {
    /// The objects onto `grid`; refused when a value does not fit the samples.
    pub fn new(
        objects: Objects,
        grid: &Grid,
        overlap: Overlap,
        sample: BurnSample,
    ) -> Result<Burn, String> {
        let (w, h) = (grid.width, grid.height);
        if overlap != Overlap::Count {
            for &(o, _) in &objects.items {
                let v = objects.values[o as usize];
                if sample.fit(v).is_none() {
                    return Err(too_big(v));
                }
            }
        }
        let mut shapes = Vec::new();
        let mut area_objects = Vec::new();
        let mut chords = Vec::new();
        let mut points = Vec::new();
        let mut burnt = Vec::with_capacity(objects.items.len());
        let cell = |p: Vec2| {
            let (u, v) = place_in(&grid.affine, p.x, p.y);
            Vec2::new(u, v)
        };
        for (o, k) in objects.items {
            burnt.push(o);
            match k {
                Kind::Area(s) => {
                    shapes.push(*s);
                    area_objects.push(o);
                }
                Kind::Paths(paths) => {
                    for path in paths {
                        let pts: Vec<Vec2> = path.into_iter().map(cell).collect();
                        if pts.len() == 1 {
                            chords.extend(chord(pts[0], pts[0], o, h));
                        }
                        for pair in pts.windows(2) {
                            chords.extend(chord(pair[0], pair[1], o, h));
                        }
                    }
                }
                Kind::Points(ps) => {
                    for p in ps {
                        let q = cell(p);
                        if !(q.x.is_finite() && q.y.is_finite()) {
                            continue;
                        }
                        let (i, j) = (floor_in(q.x, i64::from(w)), floor_in(q.y, i64::from(h)));
                        if i >= 0 && j >= 0 && i < i64::from(w) && j < i64::from(h) {
                            points.push((j as u32, i as u32, o));
                        }
                    }
                }
            }
        }
        chords.sort_by_key(|c: &Chord| c.first);
        points.sort_unstable();
        points.dedup();
        let n = objects.values.len();
        Ok(Burn {
            areas: Areas::new(grid, &shapes),
            area_objects,
            chords,
            points,
            values: objects.values,
            overlap,
            sample,
            width: w,
            burnt,
            touched: vec![false; n],
        })
    }

    pub fn sample(&self) -> RasterSample {
        self.sample.sample()
    }

    pub fn nodata(&self) -> f64 {
        self.sample.nodata()
    }

    /// Objects burnt without a cell on the rows done (all of them once the rows are done).
    pub fn outside(&self) -> usize {
        self.burnt
            .iter()
            .filter(|&&o| !self.touched[o as usize])
            .count()
    }

    /// Rows `y0 … y0 + n − 1`: their samples and the cells without a value.
    pub fn strip(&mut self, y0: u32, n: u32, threads: usize) -> Result<(Samples, u64), String> {
        let w = self.width as usize;
        let rows: Vec<u32> = (y0..y0 + n).collect();
        let made: Vec<Result<RowOut, String>> = {
            let me = &*self;
            let near_areas = me.areas.strip(y0, y0 + n);
            let end = me.chords.partition_point(|c| c.first < y0 + n);
            let near_chords: Vec<u32> = (0..end)
                .filter(|&k| me.chords[k].last >= y0)
                .map(|k| k as u32)
                .collect();
            par::map(threads, &rows, &|&j| me.row(j, &near_areas, &near_chords))
        };
        let mut out = Samples::filled(self.sample.sample(), n as usize * w, self.sample.nodata());
        let mut empty = 0;
        for (k, r) in made.into_iter().enumerate() {
            let (vals, e, burnt) = r?;
            empty += e;
            for o in burnt {
                self.touched[o as usize] = true;
            }
            for (i, v) in vals.into_iter().enumerate() {
                if !v.is_nan() {
                    out.set(k * w + i, v);
                }
            }
        }
        Ok((out, empty))
    }

    fn row(&self, j: u32, near_areas: &[u32], near_chords: &[u32]) -> Result<RowOut, String> {
        let w = self.width as usize;
        let mut acc = vec![NONE; w];
        let values = &self.values;
        let add = |acc: &mut [Acc], i: usize, o: u32| {
            let v = values[o as usize];
            let a = &mut acc[i];
            a.last = if a.count == 0 { o } else { a.last.max(o) };
            a.first = a.first.min(o);
            a.lo = a.lo.min(v);
            a.hi = a.hi.max(v);
            a.sum = a.sum.add_f64(v);
            a.count += 1;
        };
        let mut burnt: Vec<u32> = Vec::new();
        let (mut spans, mut cuts): (Vec<Span>, Vec<(u32, i64)>) = (Vec::new(), Vec::new());
        self.areas.row(j, near_areas, &mut spans, &mut cuts);
        for s in &spans {
            let o = self.area_objects[s.object as usize];
            burnt.push(o);
            for i in s.i0..s.i1 {
                add(&mut acc, i as usize, o);
            }
        }
        // Chords and points: an object once in a cell.
        let mut pairs: Vec<(u32, u32)> = Vec::new();
        for &k in near_chords {
            let c = &self.chords[k as usize];
            if j < c.first || j > c.last {
                continue;
            }
            if let Some((i0, i1)) = row_range(c, i64::from(j), w as i64) {
                pairs.extend((i0..=i1).map(|i| (i as u32, c.object)));
            }
        }
        let from = self.points.partition_point(|p| p.0 < j);
        pairs.extend(
            self.points[from..]
                .iter()
                .take_while(|p| p.0 == j)
                .map(|p| (p.1, p.2)),
        );
        pairs.sort_unstable();
        pairs.dedup();
        for &(i, o) in &pairs {
            add(&mut acc, i as usize, o);
            burnt.push(o);
        }
        burnt.sort_unstable();
        burnt.dedup();
        let mut out = vec![f64::NAN; w];
        let mut empty = 0;
        for (i, a) in acc.iter().enumerate() {
            if a.count == 0 {
                empty += 1;
                continue;
            }
            let v = match self.overlap {
                Overlap::Last => values[a.last as usize],
                Overlap::First => values[a.first as usize],
                Overlap::Max => a.hi,
                Overlap::Min => a.lo,
                // + 0.0: a sum of −0 and 0 is 0.
                Overlap::Sum => a.sum.value() + 0.0,
                Overlap::Count => f64::from(a.count),
            };
            out[i] = self.sample.fit(v).ok_or_else(|| too_big(v))?;
        }
        Ok((out, empty, burnt))
    }
}

fn too_big(v: f64) -> String {
    format!(
        "Değer {v} sonucun türüne sığmıyor: Tam sayı 32 bit ±2 147 483 647'yi, Bayt 0–254'ü alır. Türü Ondalık 32 bit ya da Ondalık 64 bit yapın."
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cells(p0: (f64, f64), p1: (f64, f64)) -> Vec<(u32, u32)> {
        let mut out = Vec::new();
        segment_cells(
            Vec2::new(p0.0, p0.1),
            Vec2::new(p1.0, p1.1),
            (10, 10),
            &mut |i, j| out.push((i, j)),
        );
        out
    }

    #[test]
    fn a_chords_cells() {
        assert_eq!(
            cells((0.5, 0.5), (2.5, 1.5)),
            vec![(0, 0), (1, 0), (1, 1), (2, 1)]
        );
        // Through a corner going right and down: no side cell, the corner its lower right's.
        assert_eq!(cells((0.5, 0.5), (2.5, 2.5)), vec![(0, 0), (1, 1), (2, 2)]);
        // Going right and up: the corner's owner is a side cell.
        assert_eq!(
            cells((0.5, 2.5), (2.5, 0.5)),
            vec![(2, 0), (1, 1), (2, 1), (0, 2), (1, 2)]
        );
        // Ending on a column line: the cell right of it holds the end.
        assert_eq!(cells((2.5, 0.5), (1.0, 0.5)), vec![(1, 0), (2, 0)]);
        assert_eq!(
            cells((2.5, 0.5), (0.999, 0.5)),
            vec![(0, 0), (1, 0), (2, 0)]
        );
        // Ending on a corner going left and down: only the corner's own cell past it.
        assert_eq!(
            cells((2.5, 0.5), (1.0, 2.0)),
            vec![(2, 0), (1, 1), (2, 1), (1, 2)]
        );
        // Off the grid: clipped, and a far chord still finds its cells.
        assert_eq!(cells((-1e12, 0.5), (1e12, 0.5)).len(), 10);
        assert_eq!(cells((-1e12, -1e12 + 0.5), (1e12, 1e12 + 0.5)).len(), 19);
    }

    #[test]
    fn overlaps() {
        let grid = Grid::of([0.0, 1.0, 0.0, 3.0, 0.0, -1.0], 3, 3).unwrap();
        let sq = Shape::Polygon {
            pts: vec![
                Vec2::new(0.0, 0.0),
                Vec2::new(2.0, 0.0),
                Vec2::new(2.0, 2.0),
                Vec2::new(0.0, 2.0),
            ],
            bulges: None,
            holes: None,
            parts: None,
        };
        let line = Shape::Line {
            a: Vec2::new(0.5, 1.5),
            b: Vec2::new(2.5, 1.5),
        };
        let texts = [Some("3".to_owned()), Some("5".to_owned())];
        let run = |overlap| {
            let objects = Objects::new(vec![sq.clone(), line.clone()], Some(&texts), 0.0).unwrap();
            let mut b = Burn::new(objects, &grid, overlap, BurnSample::F32).unwrap();
            let (s, empty) = b.strip(0, 3, 2).unwrap();
            assert_eq!(empty, 4);
            assert_eq!(b.outside(), 0);
            (0..9).map(|k| s.get(k)).collect::<Vec<f64>>()
        };
        // Row 1 (y 1 … 2) is the line's; the square holds the centres of rows 1 and 2, columns 0 and 1.
        let last = run(Overlap::Last);
        assert!(last[0..3].iter().all(|v| v.is_nan()) && last[8].is_nan());
        assert_eq!(&last[3..8], &[5.0, 5.0, 5.0, 3.0, 3.0]);
        assert_eq!(&run(Overlap::First)[3..6], &[3.0, 3.0, 5.0]);
        assert_eq!(&run(Overlap::Min)[3..6], &[3.0, 3.0, 5.0]);
        assert_eq!(&run(Overlap::Max)[3..6], &[5.0, 5.0, 5.0]);
        assert_eq!(&run(Overlap::Sum)[3..6], &[8.0, 8.0, 5.0]);
        assert_eq!(&run(Overlap::Count)[3..6], &[2.0, 2.0, 1.0]);
    }

    #[test]
    fn values_that_do_not_fit() {
        assert_eq!(BurnSample::U8.fit(254.4), Some(254.0));
        assert_eq!(BurnSample::U8.fit(254.5), None);
        assert_eq!(BurnSample::I32.fit(-2.5), Some(-3.0));
        assert_eq!(BurnSample::I32.fit(-2_147_483_648.0), None);
        assert_eq!(BurnSample::F32.fit(1e39), None);
        let line = Shape::Line {
            a: Vec2::new(0.0, 0.0),
            b: Vec2::new(1.0, 1.0),
        };
        let objects = Objects::new(vec![line], None, 300.0).unwrap();
        let grid = Grid::of([0.0, 1.0, 0.0, 3.0, 0.0, -1.0], 3, 3).unwrap();
        assert!(Burn::new(objects, &grid, Overlap::Last, BurnSample::U8).is_err());
    }
}
