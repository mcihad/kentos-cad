//! Far levels of a layer's plain strokes and solid fills on the model screen (plan/render.md RM-10, docs/adr/9207).
//!
//! A city drawing seen whole puts 30 m into a pixel: most parcels and buildings are smaller than a pixel, but each was
//! drawn with every edge and fill triangle it has. The million-object project drew 3.2 M items in its far view and the
//! city plan NCZ 4.2 M (9 for each object), 87 to 105 ms of the GPU a frame (docs/perf/files-desktop-rm05-2026-11-23.md).
//!
//! Each far level is the batch on a grid of half a pixel at the level's ceiling (8, 32 and 128 m): a segment's ends go
//! to the middles of the cells they fall in, a segment whose ends meet one cell becomes a dot of the line's width, and
//! a segment met twice (two parcels' common edge, an edge and its neighbour's) is drawn once. A fill triangle keeps its
//! cells when they make an area; one that falls into a cell or two leaves its cells covered by a cell's square, so a
//! dense block keeps its colour. What is drawn moves by at most half a pixel and no object is left out: every pixel
//! the batch touched is touched again (CLAUDE.md §6.2, “geometriyi gizlice eksiltmeyin”).
//!
//! The levels go after the batch's own numbers and the batch's `Lod` says where (main 0, its own numbers level 0, the
//! far levels after; ceilings [`FAR_CEILINGS`]), so the GPU draws one level a frame as it draws a curve's levels
//! (docs/adr/1063 §3). Only the screen's model view takes them (`DecodeOptions::hold_weights`): paper and the PDF draw
//! every edge. Instanced batches, marks and patterned fills keep what they have.
//!
//! A stroke batch with levels of its own (curves' chords, docs/adr/1063 §3; long lines', 1064 §2) draws its main part
//! and its coarsest level at 4 m a pixel and farther: a small circle a few chords at every distance. Its far levels
//! are made from those two on the grid and go after its levels, with the far ceilings after its own; from the first
//! far level on, the main part is not drawn beside them (`Lod::alone`), since a far level holds it (docs/adr/9207
//! §6).

use std::collections::HashSet;
use std::hash::{BuildHasherDefault, Hasher};

use std::ops::Range;

use crate::batches::{BatchKind, FillPaintBatch, Lod, StyledBatch, StyledLayer};

/// Logical px a metre at or under which each far level is drawn: 16, 64 and 256 m a pixel.
pub const FAR_CEILINGS: [f64; 3] = [1.0 / 16.0, 1.0 / 64.0, 1.0 / 256.0];
/// The grid a level snaps to, in its ceiling's logical pixels: half, so a high-DPI screen's pixel holds too.
const GRID_PX: f64 = 0.5;
/// Fewest segments or triangles a batch must have to come with far levels: a small batch costs the GPU nothing.
const LEAST_ITEMS: usize = 1024;
/// The first far level must keep at most this share of the batch's items, else the levels would cost memory for the
/// same frame (as `line_lod`'s, docs/adr/1064 §2).
const MOST_KEPT: f64 = 0.75;
/// Numbers a stroke segment takes: its ends, the length along the path at its start and its caps.
const STROKE: usize = 6;
/// Numbers a fill triangle takes: three corners.
const TRIANGLE: usize = 6;
/// Both caps of a segment (`push_segments`' flags): a dot is a short segment drawn with its caps.
const BOTH_CAPS: f64 = 3.0;
/// A path's far simplification, in cells: an end moved to its cell's middle strays at most half the cell's diagonal
/// (0.354 of the level's ceiling pixel), so the simplification takes at most the 0.146 px left of half a pixel.
const SIMPLIFY_SHARE: f64 = 0.29;

/// A small multiply-rotate hasher for cell keys: the levels hash millions of them.
#[derive(Default)]
struct Cells(u64);

impl Hasher for Cells {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.write_u64(u64::from(b));
        }
    }
    fn write_u64(&mut self, n: u64) {
        self.0 = (self.0.rotate_left(5) ^ n).wrapping_mul(0x517c_c1b7_2722_0a95);
    }
    fn write_i64(&mut self, n: i64) {
        self.write_u64(n as u64);
    }
}

type CellSet<T> = HashSet<T, BuildHasherDefault<Cells>>;

/// The far levels of every batch that takes them, put after its own numbers.
pub fn add(layer: &mut StyledLayer) {
    if !layer.batches.iter().any(|b| source(b).is_some_and(|[m, l]| (m.len() + l.len()) / per(b) >= LEAST_ITEMS)) {
        return;
    }
    let has_low = !layer.low.is_empty();
    let mut data = Vec::with_capacity(layer.data.len() + layer.data.len() / 2);
    let mut low = Vec::with_capacity(if has_low { data.capacity() } else { 0 });
    for b in &mut layer.batches {
        let own = b.range.clone();
        let start = data.len();
        data.extend_from_slice(&layer.data[own.clone()]);
        if has_low {
            low.extend_from_slice(&layer.low[own.clone()]);
        }
        let per = per(b);
        if let Some([main, last]) = source(b)
            && (main.len() + last.len()) / per >= LEAST_ITEMS
        {
            // The numbers drawn farthest now, the main part's then the coarsest level's, as one run.
            let at = |i: usize| {
                let k = own.start + if i < main.len() { main.start + i } else { last.start + (i - main.len()) };
                f64::from(layer.data[k]) + if has_low { f64::from(layer.low[k]) } else { 0.0 }
            };
            let n = (main.len() + last.len()) / per;
            let levels: Vec<Vec<f64>> = FAR_CEILINGS
                .iter()
                .map(|ceiling| {
                    let grid = GRID_PX / ceiling;
                    match b.kind {
                        BatchKind::Stroke { .. } => {
                            // Paths simplified first, within what the grid leaves of half a pixel (§7).
                            let lines = simplified(n, &at, grid * SIMPLIFY_SHARE);
                            strokes(lines.len() / STROKE, &|i| lines[i], grid)
                        }
                        _ => fills(n, &at, grid),
                    }
                })
                .collect();
            if (levels[0].len() as f64) <= MOST_KEPT * (main.len() + last.len()) as f64 {
                let (main, mut lens, mut ceilings, alone) = match &b.lod {
                    Some(lod) => (lod.main, lod.levels.clone(), lod.ceilings.clone(), lod.levels.len()),
                    None => (0, vec![own.len()], Vec::new(), 1),
                };
                ceilings.extend(FAR_CEILINGS);
                for level in &levels {
                    lens.push(level.len());
                    for &v in level {
                        let hi = v as f32;
                        data.push(hi);
                        if has_low {
                            low.push((v - f64::from(hi)) as f32);
                        }
                    }
                }
                b.lod = Some(Lod { main, levels: lens, ceilings, alone });
            }
        }
        b.range = start..data.len();
    }
    layer.data = data;
    layer.low = low;
}

/// Numbers an item of a batch takes.
fn per(b: &StyledBatch) -> usize {
    match &b.kind {
        BatchKind::Stroke { .. } => STROKE,
        _ => TRIANGLE,
    }
}

/// What a batch's far levels are made of, as ranges of its numbers drawn together farthest now: a plain stroke's or a
/// solid fill's all of them; a stroke batch with levels of its own (not yet far ones) its main part and its coarsest
/// level. None for a batch that takes none: instanced, marks, patterned fills.
fn source(b: &StyledBatch) -> Option<[Range<usize>; 2]> {
    if b.instances.is_some() {
        return None;
    }
    match (&b.kind, &b.lod) {
        (BatchKind::Stroke { .. }, Some(lod)) if lod.alone >= lod.levels.len() => {
            Some(lod.ranges(lod.levels.len().checked_sub(1)?))
        }
        (BatchKind::Stroke { .. } | BatchKind::Fill { paint: FillPaintBatch::Solid { .. } }, None) => {
            Some([0..b.range.len(), 0..0])
        }
        _ => None,
    }
}

fn cell(x: f64, grid: f64) -> i64 {
    (x / grid).floor() as i64
}

fn middle(c: i64, grid: f64) -> f64 {
    (c as f64 + 0.5) * grid
}

/// `n` segments read through `at` (numbers from the batch's start) on a grid: each pair of cells once, a dot for each
/// cell a segment fell into whole.
fn strokes(n: usize, at: &impl Fn(usize) -> f64, grid: f64) -> Vec<f64> {
    let mut seen: CellSet<(i64, i64, i64, i64)> = CellSet::default();
    let mut out = Vec::new();
    for s in 0..n {
        let i = s * STROKE;
        let a = (cell(at(i), grid), cell(at(i + 1), grid));
        let b = (cell(at(i + 2), grid), cell(at(i + 3), grid));
        let key = if a <= b { (a.0, a.1, b.0, b.1) } else { (b.0, b.1, a.0, a.1) };
        if !seen.insert(key) {
            continue;
        }
        let (ax, ay) = (middle(a.0, grid), middle(a.1, grid));
        // A dot: a hundredth of the cell long, drawn with its caps as wide as the line.
        let (bx, by) = if a == b { (ax + grid * 0.01, ay) } else { (middle(b.0, grid), middle(b.1, grid)) };
        out.extend([ax, ay, bx, by, 0.0, BOTH_CAPS]);
    }
    out
}

/// `n` segments read through `at` as paths, each simplified within `tolerance` (Douglas–Peucker, its ends kept): a
/// segment continues the one before it when it starts where that one ended and has no start cap. A path keeps its
/// first segment's start cap and its last one's end cap, and each segment its length along the path where it starts.
fn simplified(n: usize, at: &impl Fn(usize) -> f64, tolerance: f64) -> Vec<f64> {
    let mut out = Vec::with_capacity(n * STROKE / 2);
    let mut points: Vec<[f64; 2]> = Vec::new();
    let mut along: Vec<f64> = Vec::new();
    let mut caps = (0u32, 0u32);
    let flush = |points: &mut Vec<[f64; 2]>, along: &mut Vec<f64>, caps: (u32, u32), out: &mut Vec<f64>| {
        if points.len() >= 2 {
            let kept = peucker(points, tolerance);
            let last = kept.len().saturating_sub(2);
            for (k, w) in kept.windows(2).enumerate() {
                let (a, b) = (points[w[0]], points[w[1]]);
                let flags = if k == 0 { caps.0 } else { 0 } | if k == last { caps.1 } else { 0 };
                out.extend([a[0], a[1], b[0], b[1], along[w[0]], f64::from(flags)]);
            }
        }
        points.clear();
        along.clear();
    };
    for s in 0..n {
        let i = s * STROKE;
        let (a, b) = ([at(i), at(i + 1)], [at(i + 2), at(i + 3)]);
        let ends = at(i + 5).max(0.0) as u32;
        if points.last() != Some(&a) || ends & 1 != 0 {
            flush(&mut points, &mut along, caps, &mut out);
            points.push(a);
            along.push(at(i + 4));
            caps.0 = ends & 1;
        }
        let start = *along.last().unwrap_or(&0.0);
        points.push(b);
        along.push(start + (b[0] - a[0]).hypot(b[1] - a[1]));
        caps.1 = ends & 2;
    }
    flush(&mut points, &mut along, caps, &mut out);
    out
}

/// The points Douglas–Peucker keeps of `points` within `tolerance`, in order; the ends always.
fn peucker(points: &[[f64; 2]], tolerance: f64) -> Vec<usize> {
    let n = points.len();
    let mut keep = vec![false; n];
    keep[0] = true;
    keep[n - 1] = true;
    let limit = tolerance * tolerance;
    let mut stack = vec![(0usize, n - 1)];
    while let Some((a, b)) = stack.pop() {
        if b <= a + 1 {
            continue;
        }
        let (mut far, mut at) = (0.0, a);
        for k in a + 1..b {
            let d = distance2(points[k], points[a], points[b]);
            if d > far {
                (far, at) = (d, k);
            }
        }
        if far > limit {
            keep[at] = true;
            stack.push((a, at));
            stack.push((at, b));
        }
    }
    (0..n).filter(|&k| keep[k]).collect()
}

/// Squared distance from `p` to the segment `a`–`b` (to `a` when it is a point): a closed path's chord is a point.
fn distance2(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let (px, py) = (p[0] - a[0], p[1] - a[1]);
    let len2 = dx * dx + dy * dy;
    let t = if len2 > 0.0 { ((px * dx + py * dy) / len2).clamp(0.0, 1.0) } else { 0.0 };
    let (ex, ey) = (px - t * dx, py - t * dy);
    ex * ex + ey * ey
}

/// `n` triangles read through `at` on a grid: each triangle that keeps an area, once, by its cells; the cells of one
/// that does not as a cell's square, once each.
fn fills(n: usize, at: &impl Fn(usize) -> f64, grid: f64) -> Vec<f64> {
    let mut seen: CellSet<[(i64, i64); 3]> = CellSet::default();
    let mut covered: CellSet<(i64, i64)> = CellSet::default();
    let mut squares: Vec<(i64, i64)> = Vec::new();
    let mut out = Vec::new();
    for t in 0..n {
        let i = t * TRIANGLE;
        let mut c = [0usize, 2, 4].map(|k| (cell(at(i + k), grid), cell(at(i + k + 1), grid)));
        let area = (c[1].0 - c[0].0) * (c[2].1 - c[0].1) - (c[2].0 - c[0].0) * (c[1].1 - c[0].1);
        if area != 0 {
            let corners = c;
            c.sort_unstable();
            if seen.insert(c) {
                for (x, y) in corners {
                    out.extend([middle(x, grid), middle(y, grid)]);
                }
            }
            continue;
        }
        for k in c {
            if covered.insert(k) {
                squares.push(k);
            }
        }
    }
    for (x, y) in squares {
        let (x0, y0, x1, y1) = (x as f64 * grid, y as f64 * grid, (x + 1) as f64 * grid, (y + 1) as f64 * grid);
        out.extend([x0, y0, x1, y0, x1, y1, x0, y0, x1, y1, x0, y1]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(v: &[f64]) -> impl Fn(usize) -> f64 + '_ {
        move |i| v[i]
    }

    #[test]
    fn a_shared_edge_is_drawn_once_and_a_small_ring_becomes_a_dot() {
        // Two squares of 3 m side by side (a common edge), and a ring of 1 m far away: on a 10 m grid.
        let mut v = Vec::new();
        let mut square = |x: f64| {
            for (a, b) in [((x, 0.0), (x + 3.0, 0.0)), ((x + 3.0, 0.0), (x + 3.0, 3.0)), ((x + 3.0, 3.0), (x, 3.0)), ((x, 3.0), (x, 0.0))] {
                v.extend([a.0, a.1, b.0, b.1, 0.0, 0.0]);
            }
        };
        square(1.0);
        square(4.0);
        let ring = [(100.2, 100.2), (101.0, 100.2), (101.0, 101.0), (100.2, 101.0), (100.2, 100.2)];
        for w in ring.windows(2) {
            v.extend([w[0].0, w[0].1, w[1].0, w[1].1, 0.0, 0.0]);
        }
        let n = v.len() / STROKE;
        let out = strokes(n, &read(&v), 10.0);
        let segs = out.len() / STROKE;
        // The squares lie in cell (0, 0): every edge is a dot there, drawn once; the ring is one dot in cell (10, 10).
        assert_eq!(segs, 2, "{out:?}");
        assert_eq!((out[0], out[1]), (5.0, 5.0));
        assert_eq!((out[6], out[7]), (105.0, 105.0));
        assert_eq!(out[5], BOTH_CAPS);
    }

    #[test]
    fn a_long_line_keeps_its_cells_and_a_tiny_fill_keeps_its_pixel() {
        // A line across three cells, drawn as one segment from cell to cell.
        let line = [0.5, 0.5, 25.0, 0.5, 0.0, 0.0];
        let out = strokes(1, &read(&line), 10.0);
        assert_eq!(&out[..4], &[5.0, 5.0, 25.0, 5.0]);
        // A big triangle keeps an area; a tiny one in another cell leaves that cell covered by its square.
        let tris = [0.0, 0.0, 30.0, 0.0, 0.0, 30.0, 51.0, 51.0, 52.0, 51.0, 51.0, 52.0];
        let out = fills(2, &read(&tris), 10.0);
        assert_eq!(out.len(), 6 + 12, "a triangle and a square of two triangles");
        assert_eq!(&out[6..8], &[50.0, 50.0], "the square is the cell the tiny one lies in");
    }

    #[test]
    fn a_curve_batch_takes_far_levels_after_its_own_and_drops_its_main_part_there() {
        use crate::batches::{Cap, StyledBatch, Unit};
        // A main part of 10 short lines, then four curve levels; the coarsest has 1 200 chords, 300 small rings.
        let mut data = Vec::new();
        let seg = |d: &mut Vec<f32>, x: f32, y: f32| d.extend([x, y, x + 0.5, y + 0.3, 0.0, 0.0]);
        for k in 0..10 {
            seg(&mut data, 1000.0 + k as f32 * 50.0, 0.0);
        }
        let main = data.len();
        let mut levels = Vec::new();
        for (l, count) in [(0, 2400), (1, 1800), (2, 1500), (3, 1200)] {
            let before = data.len();
            // Rings of four chords, 30 m apart: a ring's chords lie in one cell of the far grid.
            for k in 0..count {
                let r = k / 4;
                seg(&mut data, (r % 20) as f32 * 30.0 + (k % 4) as f32 * 0.2 + l as f32 * 0.01, (r / 20) as f32 * 30.0);
            }
            levels.push(data.len() - before);
        }
        let mut layer = StyledLayer {
            batches: vec![StyledBatch {
                range: 0..data.len(),
                kind: BatchKind::Stroke {
                    color: [1.0; 4],
                    width: 1.0,
                    unit: Unit::Px,
                    dash: None,
                    dash_offset: 0.0,
                    cap: Cap::Butt,
                    blur: 0.0,
                },
                level: 0.0,
                key: 1,
                bounds: [0.0, 0.0, 1500.0, 1000.0],
                origin: [0.0, 0.0],
                reach: 1.0,
                reach_unit: Unit::Px,
                min_scale: None,
                max_scale: None,
                group: 0,
                effect: None,
                weight_cap: 0.0,
                line_type_loss: None,
                lod: Some(Lod { main, alone: 4, levels: levels.clone(), ceilings: crate::batches::CURVE_LEVELS.to_vec() }),
                instances: None,
            }],
            data,
            low: Vec::new(),
            pictures: Vec::new(),
            dropped: 0,
        };
        add(&mut layer);
        let b = &layer.batches[0];
        let lod = b.lod.as_ref().expect("levels");
        assert_eq!(lod.levels.len(), 7, "its four levels and three far ones");
        assert_eq!(&lod.levels[..4], &levels[..]);
        assert_eq!(lod.ceilings.len(), 6);
        assert!(lod.ceilings.ends_with(&FAR_CEILINGS));
        assert_eq!(lod.alone, 4);
        // Near, the main part draws with a level; at the far ones it does not.
        assert_eq!(lod.ranges(3)[0], 0..main);
        assert_eq!(lod.ranges(4)[0], 0..0);
        // The first far level (8 m) at 1/16 px a metre (16 m a pixel): fewer strokes than main and the coarsest level,
        // and every cell either touched is touched again.
        assert_eq!(lod.level(1.0 / 20.0, 1.0), 4);
        let grid = GRID_PX / FAR_CEILINGS[0];
        let numbers: Vec<f64> = layer.data[b.range.clone()].iter().map(|&v| f64::from(v)).collect();
        let far = &numbers[lod.ranges(4)[1].clone()];
        assert!(far.len() < main + levels[3], "{} ≥ {}", far.len(), main + levels[3]);
        let drawn: HashSet<(i64, i64)> =
            far.chunks(STROKE).flat_map(|s| [(cell(s[0], grid), cell(s[1], grid)), (cell(s[2], grid), cell(s[3], grid))]).collect();
        let coarsest = lod.ranges(3)[1].clone();
        for s in numbers[..main].chunks(STROKE).chain(numbers[coarsest].chunks(STROKE)) {
            assert!(drawn.contains(&(cell(s[0], grid), cell(s[1], grid))), "a touched cell is left out");
        }
    }

    #[test]
    fn paths_are_simplified_before_the_grid_and_keep_their_caps() {
        // An open path of 100 points along a gentle wave (amplitude 0.5 m), then a separate segment with its own caps.
        let mut v = Vec::new();
        let mut along = 0.0;
        for k in 0..99 {
            let (x0, x1) = (k as f64 * 3.0, (k + 1) as f64 * 3.0);
            let (y0, y1) = ((x0 / 9.0).sin() * 0.5, (x1 / 9.0).sin() * 0.5);
            let caps = if k == 0 { 1.0 } else if k == 98 { 2.0 } else { 0.0 };
            v.extend([x0, y0, x1, y1, along, caps]);
            along += (x1 - x0).hypot(y1 - y0);
        }
        v.extend([500.0, 500.0, 501.0, 500.0, 0.0, 3.0]);
        let out = simplified(v.len() / STROKE, &read(&v), 2.0);
        let segs: Vec<&[f64]> = out.chunks(STROKE).collect();
        // The wave within 2 m is one segment from end to end; the separate one stays.
        assert_eq!(segs.len(), 2, "{out:?}");
        assert_eq!(&segs[0][..4], &[0.0, 0.0, 297.0, (297.0f64 / 9.0).sin() * 0.5]);
        assert_eq!(segs[0][5], 3.0, "the path's start and end caps");
        assert_eq!(&segs[1][..4], &[500.0, 500.0, 501.0, 500.0]);
        assert_eq!(segs[1][5], 3.0);
        // A turn is kept: an L of two long legs keeps its corner.
        let l = [0.0, 0.0, 100.0, 0.0, 0.0, 1.0, 100.0, 0.0, 100.0, 100.0, 100.0, 2.0];
        let out = simplified(2, &read(&l), 2.0);
        assert_eq!(out.len(), 2 * STROKE, "{out:?}");
        assert_eq!(out[STROKE + 4], 100.0, "the second leg starts 100 m along");
    }

    #[test]
    fn every_cell_a_batch_touched_is_touched_again() {
        // Many small segments scattered: the far level's dots and segments cover every cell an original end fell in.
        let mut v = Vec::new();
        let mut seed = 7u64;
        let mut next = || {
            seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            (seed >> 11) as f64 / (1u64 << 53) as f64 * 1000.0
        };
        for _ in 0..5000 {
            let (x, y) = (next(), next());
            v.extend([x, y, x + 0.3, y + 0.2, 0.0, 0.0]);
        }
        let grid = 8.0;
        let out = strokes(5000, &read(&v), grid);
        let drawn: HashSet<(i64, i64)> =
            out.chunks(STROKE).flat_map(|s| [(cell(s[0], grid), cell(s[1], grid)), (cell(s[2], grid), cell(s[3], grid))]).collect();
        for s in v.chunks(STROKE) {
            assert!(drawn.contains(&(cell(s[0], grid), cell(s[1], grid))), "a touched cell is left out");
        }
        assert!(out.len() < v.len(), "fewer than before");
    }
}
