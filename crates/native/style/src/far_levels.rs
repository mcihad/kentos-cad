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
//! every edge. Batches with levels of their own (curves, long lines), instanced ones, marks and patterned fills keep
//! what they have.

use std::collections::HashSet;
use std::hash::{BuildHasherDefault, Hasher};

use crate::batches::{BatchKind, FillPaintBatch, Lod, StyledLayer};

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
    if !layer.batches.iter().any(|b| eligible(&b.kind, b) && b.range.len() / STROKE >= LEAST_ITEMS) {
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
        let per = match &b.kind {
            BatchKind::Stroke { .. } => STROKE,
            _ => TRIANGLE,
        };
        if eligible(&b.kind, b) && own.len() / per >= LEAST_ITEMS {
            let at = |i: usize| {
                f64::from(layer.data[own.start + i]) + if has_low { f64::from(layer.low[own.start + i]) } else { 0.0 }
            };
            let n = own.len() / per;
            let levels: Vec<Vec<f64>> = FAR_CEILINGS
                .iter()
                .map(|ceiling| {
                    let grid = GRID_PX / ceiling;
                    match b.kind {
                        BatchKind::Stroke { .. } => strokes(n, &at, grid),
                        _ => fills(n, &at, grid),
                    }
                })
                .collect();
            if (levels[0].len() as f64) <= MOST_KEPT * own.len() as f64 {
                let mut lens = vec![own.len()];
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
                b.lod = Some(Lod { main: 0, levels: lens, ceilings: FAR_CEILINGS });
            }
        }
        b.range = start..data.len();
    }
    layer.data = data;
    layer.low = low;
}

/// Whether a batch takes far levels: a plain stroke or a solid fill, with no levels or instances of its own.
fn eligible(kind: &BatchKind, b: &crate::batches::StyledBatch) -> bool {
    b.lod.is_none()
        && b.instances.is_none()
        && matches!(kind, BatchKind::Stroke { .. } | BatchKind::Fill { paint: FillPaintBatch::Solid { .. } })
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
