//! A large batch's items grouped by where they lie, so a view draws only those near it (plan/render.md RM-08,
//! docs/adr/9241).
//!
//! A batch gathers a style's items across the whole layer: a city plan's parcel lines are a few batches of hundreds of
//! thousands of segments each. A view tested the batch's box alone (`StyledBatch::in_view`), so eight wheel notches
//! into the NCZ still drew 1 223 608 items: every segment of the city for a view of a neighbourhood.
//!
//! Here each of a large batch's parts (its main numbers and each detail level's, `Lod::ranges`; or all its numbers) has
//! its items sorted by the cell of a grid over the part, stably: within a cell they keep their order. A part's cells
//! keep where their items start and the box their items cover, so a view finds the item ranges it meets
//! ([`CellPart::ranges_in`]) and the renderer draws those.
//!
//! **The picture does not change.** A batch is one style: its items have one colour, and over one another the same
//! colour gives the same pixel in any order, whatever share of it each covers (two layers of alphas a and b leave
//! `c·a + c·b − c·a·b` over `d·(1−a)(1−b)` either way). A paint read from the ground (a hatch, a gradient, a pattern, a
//! picture) gives one colour a pixel too. Markers are left as they are: a batch's markers are one symbol, but a
//! picture's colours differ across it, and which of two overlapping markers is on top would show.

use std::ops::Range;
use std::sync::Arc;

use crate::batches::{BatchKind, FillPaintBatch, StyledBatch, StyledLayer};

/// A batch part with fewer items is drawn whole: its cells would save less than they cost. A large layer is built in
/// parts of its objects in the document's order (docs/adr/0121), each spread over the whole drawing: the NCZ's parts'
/// batches hold some 8 000 items, a part of them under 4 096, and a first threshold of 4 096 left 1.1 M of a
/// neighbourhood view's 1.24 M items in batches drawn whole.
pub const LEAST_ITEMS: usize = 512;
/// Items a cell holds on average.
const ITEMS_A_CELL: usize = 64;
/// Cells across a part, at most (a grid of 64 × 64 at most).
const MOST_CELLS_ACROSS: usize = 64;
/// Draws a part takes at most for a view: nearer ranges are joined past it (each draw costs the CPU a few µs).
pub const MOST_RANGES: usize = 8;

/// A batch's parts' cells.
#[derive(Clone, Debug, PartialEq)]
pub struct Cells {
    pub parts: Vec<CellPart>,
}

/// One part's items by cell.
#[derive(Clone, Debug, PartialEq)]
pub struct CellPart {
    /// The part's numbers, from the batch's first.
    pub range: Range<usize>,
    /// Numbers an item takes.
    pub floats: usize,
    /// The grid: its corner (from the layers' origin), a cell's width and height, cells across and up.
    pub corner: [f64; 2],
    pub cell: [f64; 2],
    pub nx: usize,
    pub ny: usize,
    /// Each cell's first item (from the part's first), row by row from the grid's foot, then one past the last.
    pub starts: Vec<u32>,
    /// Each cell's items' box (from the layers' origin): x0, y0, x1, y1; an empty cell's is inverted.
    pub boxes: Vec<[f32; 4]>,
    /// How far an item reaches past its cell's box, most, cells across and up: the cells a view looks at past its own.
    pub spill: [usize; 2],
}

impl CellPart {
    /// The item ranges (from the part's first item) whose cells' boxes meet `view` (from the layers' origin) grown by
    /// `reach`, in their order, those next to one another joined; at most [`MOST_RANGES`], the nearest joined over the
    /// gap between them.
    pub fn ranges_in(&self, view: [f64; 4], reach: f64) -> Vec<Range<u32>> {
        let [x0, y0, x1, y1] = [view[0] - reach, view[1] - reach, view[2] + reach, view[3] + reach];
        let col = |x: f64, count: usize, at: f64, size: f64| -> i64 {
            (((x - at) / size).floor() as i64).clamp(-1, count as i64)
        };
        let (c0, c1) = (col(x0, self.nx, self.corner[0], self.cell[0]), col(x1, self.nx, self.corner[0], self.cell[0]));
        let (r0, r1) = (col(y0, self.ny, self.corner[1], self.cell[1]), col(y1, self.ny, self.corner[1], self.cell[1]));
        // Cells whose items may reach the view though the cell lies outside it.
        let c0 = (c0 - self.spill[0] as i64).max(0) as usize;
        let r0 = (r0 - self.spill[1] as i64).max(0) as usize;
        let c1 = (c1 + self.spill[0] as i64).min(self.nx as i64 - 1);
        let r1 = (r1 + self.spill[1] as i64).min(self.ny as i64 - 1);
        let mut out: Vec<Range<u32>> = Vec::new();
        if c1 < 0 || r1 < 0 {
            return out;
        }
        let (c1, r1) = (c1 as usize, r1 as usize);
        for r in r0..=r1 {
            for c in c0..=c1 {
                let k = r * self.nx + c;
                let (a, z) = (self.starts[k], self.starts[k + 1]);
                if a == z {
                    continue;
                }
                let b = self.boxes[k];
                if f64::from(b[2]) < x0 || f64::from(b[0]) > x1 || f64::from(b[3]) < y0 || f64::from(b[1]) > y1 {
                    continue;
                }
                match out.last_mut() {
                    Some(last) if last.end == a => last.end = z,
                    _ => out.push(a..z),
                }
            }
        }
        // Too many draws: the two ranges with the least between them are joined, again and again.
        while out.len() > MOST_RANGES {
            let k = (0..out.len() - 1).min_by_key(|&k| out[k + 1].start - out[k].end).unwrap_or(0);
            out[k].end = out[k + 1].end;
            out.remove(k + 1);
        }
        out
    }
}

/// Numbers an item of a batch takes, for a batch whose items may be sorted: a stroke's segment, a fill's triangle.
fn item_floats(b: &StyledBatch) -> Option<usize> {
    if b.instances.is_some() {
        return None;
    }
    match &b.kind {
        BatchKind::Stroke { .. } => Some(6),
        BatchKind::Fill {
            paint:
                FillPaintBatch::Solid { .. }
                | FillPaintBatch::Hatch { .. }
                | FillPaintBatch::Gradient { .. }
                | FillPaintBatch::Pattern { .. }
                | FillPaintBatch::Tile { .. },
        } => Some(6),
        _ => None,
    }
}

/// An item's box (from its batch's tile): a segment's ends, a triangle's corners.
fn item_box(kind: &BatchKind, v: &[f32]) -> [f64; 4] {
    let p = |i: usize| (f64::from(v[i]), f64::from(v[i + 1]));
    let points: &[(f64, f64)] = &match kind {
        BatchKind::Stroke { .. } => [p(0), p(2), p(2)],
        _ => [p(0), p(2), p(4)],
    };
    points.iter().fold([f64::MAX, f64::MAX, f64::MIN, f64::MIN], |b, &(x, y)| {
        [b[0].min(x), b[1].min(y), b[2].max(x), b[3].max(y)]
    })
}

/// Sorts the large batches' parts by cell and keeps their cells (`StyledLayer::cells`). `KENTOS_DIAG_NO_CELLS` (a
/// measurement's, plan/render.md RM-08) leaves every batch as it was, for the frames without cells.
pub fn add(layer: &mut StyledLayer) {
    if std::env::var_os("KENTOS_DIAG_NO_CELLS").is_some() {
        return;
    }
    let mut cells: Vec<Option<Arc<Cells>>> = vec![None; layer.batches.len()];
    let has_low = layer.low.len() == layer.data.len();
    for (i, b) in layer.batches.iter().enumerate() {
        let Some(floats) = item_floats(b) else {
            continue;
        };
        let parts: Vec<Range<usize>> = match &b.lod {
            Some(lod) => {
                let mut start = 0;
                std::iter::once(lod.main)
                    .chain(lod.levels.iter().copied())
                    .map(|len| {
                        let r = start..start + len;
                        start += len;
                        r
                    })
                    .collect()
            }
            None => std::iter::once(0..b.range.len()).collect(),
        };
        let mut out = Vec::new();
        for r in parts {
            if r.len() % floats != 0 || r.len() / floats < LEAST_ITEMS {
                continue;
            }
            let at = b.range.start + r.start;
            let (data, low) = (&mut layer.data[at..at + r.len()], has_low.then(|| &mut layer.low[at..at + r.len()]));
            if let Some(part) = sort(&b.kind, data, low, floats, b.origin, r) {
                out.push(part);
            }
        }
        if !out.is_empty() {
            cells[i] = Some(Arc::new(Cells { parts: out }));
        }
    }
    if cells.iter().any(Option::is_some) {
        layer.cells = cells;
    }
}

/// One part's items sorted by the cell of a grid over them, stably; its cells.
fn sort(
    kind: &BatchKind,
    data: &mut [f32],
    low: Option<&mut [f32]>,
    floats: usize,
    origin: [f64; 2],
    range: Range<usize>,
) -> Option<CellPart> {
    let n = data.len() / floats;
    let boxes: Vec<[f64; 4]> = data.chunks(floats).map(|v| item_box(kind, v)).collect();
    let all = boxes.iter().fold([f64::MAX, f64::MAX, f64::MIN, f64::MIN], |b, i| {
        [b[0].min(i[0]), b[1].min(i[1]), b[2].max(i[2]), b[3].max(i[3])]
    });
    if !all.iter().all(|v| v.is_finite()) {
        return None;
    }
    let (w, h) = ((all[2] - all[0]).max(1e-6), (all[3] - all[1]).max(1e-6));
    // About ITEMS_A_CELL items a cell, the cells as square as the part allows.
    let cells = (n / ITEMS_A_CELL).max(1) as f64;
    let nx = ((cells * w / h).sqrt().round() as usize).clamp(1, MOST_CELLS_ACROSS);
    let ny = ((cells * h / w).sqrt().round() as usize).clamp(1, MOST_CELLS_ACROSS);
    if nx * ny < 2 {
        return None;
    }
    let size = [w / nx as f64, h / ny as f64];
    let cell_of = |b: &[f64; 4]| {
        let (cx, cy) = ((b[0] + b[2]) / 2.0, (b[1] + b[3]) / 2.0);
        let x = (((cx - all[0]) / size[0]) as usize).min(nx - 1);
        let y = (((cy - all[1]) / size[1]) as usize).min(ny - 1);
        y * nx + x
    };
    // A counting sort: each cell's items in their order.
    let keys: Vec<usize> = boxes.iter().map(cell_of).collect();
    let mut starts = vec![0u32; nx * ny + 1];
    for &k in &keys {
        starts[k + 1] += 1;
    }
    for k in 0..nx * ny {
        starts[k + 1] += starts[k];
    }
    let mut next = starts.clone();
    let mut order = vec![0usize; n];
    for (item, &k) in keys.iter().enumerate() {
        order[next[k] as usize] = item;
        next[k] += 1;
    }
    let permute = |values: &mut [f32]| {
        let old = values.to_vec();
        for (to, &from) in order.iter().enumerate() {
            values[to * floats..(to + 1) * floats].copy_from_slice(&old[from * floats..(from + 1) * floats]);
        }
    };
    permute(data);
    if let Some(low) = low {
        permute(low);
    }
    // Each cell's box, and how far past its own cell an item reaches.
    let mut cell_boxes = vec![[f32::MAX, f32::MAX, f32::MIN, f32::MIN]; nx * ny];
    let mut spill = [0usize; 2];
    for (item, b) in boxes.iter().enumerate() {
        let k = keys[item];
        let c = &mut cell_boxes[k];
        let o = [b[0] + origin[0], b[1] + origin[1], b[2] + origin[0], b[3] + origin[1]];
        *c = [c[0].min(o[0] as f32), c[1].min(o[1] as f32), c[2].max(o[2] as f32), c[3].max(o[3] as f32)];
        let (x, y) = (k % nx, k / nx);
        let x0 = all[0] + x as f64 * size[0];
        let y0 = all[1] + y as f64 * size[1];
        let over = |lo: f64, hi: f64, from: f64, cell: f64| {
            (((from - lo).max(hi - (from + cell)).max(0.0)) / cell).ceil() as usize
        };
        spill = [spill[0].max(over(b[0], b[2], x0, size[0])), spill[1].max(over(b[1], b[3], y0, size[1]))];
    }
    // The cells' boxes a float32 step wide of their items: none is missed for rounding.
    for c in &mut cell_boxes {
        if c[0] <= c[2] {
            let pad = |v: f32| v.abs().max(1.0) * f32::EPSILON * 4.0;
            *c = [c[0] - pad(c[0]), c[1] - pad(c[1]), c[2] + pad(c[2]), c[3] + pad(c[3])];
        }
    }
    Some(CellPart {
        range,
        floats,
        corner: [all[0] + origin[0], all[1] + origin[1]],
        cell: size,
        nx,
        ny,
        starts,
        boxes: cell_boxes,
        spill,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::batches::{Cap, Lod, Unit};

    /// A deterministic scatter: `n` short segments over a square `side` metres, from the batch's tile, then a few long
    /// ones across it.
    fn segments(n: usize, side: f64) -> Vec<f32> {
        let mut out = Vec::with_capacity((n + 4) * 6);
        let mut seed = 12_345u64;
        let mut next = || {
            seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
            (seed >> 11) as f64 / (1u64 << 53) as f64
        };
        for i in 0..n {
            let (x, y) = (next() * side, next() * side);
            let (dx, dy) = (next() * 20.0 - 10.0, next() * 20.0 - 10.0);
            out.extend([x as f32, y as f32, (x + dx) as f32, (y + dy) as f32, i as f32, 3.0]);
        }
        for k in 0..4 {
            let y = side * (k as f64 + 0.5) / 4.0;
            out.extend([0.0, y as f32, side as f32, (side - y) as f32, 0.0, 3.0]);
        }
        out
    }

    fn stroke(data: Vec<f32>, lod: Option<Lod>) -> StyledLayer {
        let batch = StyledBatch {
            range: 0..data.len(),
            kind: BatchKind::Stroke {
                color: [0.0, 0.0, 0.0, 1.0],
                width: 0.25,
                unit: Unit::World,
                dash: None,
                dash_offset: 0.0,
                cap: Cap::Butt,
                blur: 0.0,
            },
            level: 0.0,
            key: 1,
            bounds: [0.0, 0.0, 10_000.0, 10_000.0],
            origin: [100.0, 200.0],
            reach: 0.125,
            reach_unit: Unit::World,
            min_scale: None,
            max_scale: None,
            group: 0,
            effect: None,
            weight_cap: 0.0,
            line_type_loss: None,
            lod,
            instances: None,
        };
        StyledLayer { data, batches: vec![batch], ..Default::default() }
    }

    fn items(data: &[f32]) -> Vec<[u32; 6]> {
        let mut v: Vec<[u32; 6]> = data.chunks(6).map(|c| std::array::from_fn(|i| c[i].to_bits())).collect();
        v.sort_unstable();
        v
    }

    #[test]
    fn a_large_batch_keeps_every_item_each_in_its_cell() {
        let data = segments(20_000, 10_000.0);
        let mut layer = stroke(data.clone(), None);
        layer.low = data.iter().map(|v| v * 1e-7).collect();
        let low_before: Vec<(u32, u32)> = data.iter().zip(&layer.low).map(|(a, b)| (a.to_bits(), b.to_bits())).collect();
        add(&mut layer);
        assert_eq!(items(&layer.data), items(&data), "every item, once");
        // The low stream followed its numbers.
        let mut after: Vec<(u32, u32)> = layer.data.iter().zip(&layer.low).map(|(a, b)| (a.to_bits(), b.to_bits())).collect();
        let mut before = low_before;
        after.sort_unstable();
        before.sort_unstable();
        assert_eq!(after, before);
        let cells = layer.cells[0].as_ref().expect("cells");
        let part = &cells.parts[0];
        assert_eq!(part.range, 0..data.len());
        assert!(part.nx * part.ny >= 32, "{} × {}", part.nx, part.ny);
        // Each cell's items lie in its box and have their middle in the cell; within a cell the items keep their order.
        for k in 0..part.nx * part.ny {
            let (a, z) = (part.starts[k] as usize, part.starts[k + 1] as usize);
            let mut last = -1.0f32;
            for item in a..z {
                let v = &layer.data[item * 6..item * 6 + 6];
                let b = part.boxes[k];
                let o = [100.0f32, 200.0];
                assert!(v[0] + o[0] >= b[0] && v[0] + o[0] <= b[2] && v[1] + o[1] >= b[1] && v[1] + o[1] <= b[3]);
                if v[4] > 0.0 {
                    assert!(v[4] > last, "cell {k}: items in their order");
                    last = v[4];
                }
            }
        }
    }

    #[test]
    fn a_view_draws_every_item_it_meets() {
        let data = segments(20_000, 10_000.0);
        let mut layer = stroke(data, None);
        add(&mut layer);
        let part = &layer.cells[0].as_ref().expect("cells").parts[0];
        for view in [[2_000.0, 3_000.0, 2_400.0, 3_300.0], [-50.0, -50.0, 150.0, 150.0], [9_000.0, 9_500.0, 11_000.0, 12_000.0]] {
            let ranges = part.ranges_in(view, 0.125);
            assert!(ranges.len() <= MOST_RANGES);
            let drawn: usize = ranges.iter().map(|r| r.len()).sum();
            let n = layer.data.len() / 6;
            let mut met = 0;
            for item in 0..n {
                let v = &layer.data[item * 6..item * 6 + 6];
                let (x0, x1) = (v[0].min(v[2]) as f64 + 100.0, v[0].max(v[2]) as f64 + 100.0);
                let (y0, y1) = (v[1].min(v[3]) as f64 + 200.0, v[1].max(v[3]) as f64 + 200.0);
                let meets = x1 >= view[0] - 0.125 && x0 <= view[2] + 0.125 && y1 >= view[1] - 0.125 && y0 <= view[3] + 0.125;
                if meets {
                    met += 1;
                    assert!(ranges.iter().any(|r| r.contains(&(item as u32))), "{view:?}: item {item} meets it");
                }
            }
            assert!(drawn < n / 4, "{view:?}: {drawn} of {n} drawn for {met} met");
        }
        // A view over everything draws everything as one range.
        assert_eq!(part.ranges_in([-1e6, -1e6, 1e6, 1e6], 0.0), vec![0..(layer.data.len() / 6) as u32]);
    }

    #[test]
    fn a_batch_with_levels_sorts_each_part_by_itself() {
        let main = segments(5_000, 10_000.0);
        let level = segments(6_000, 10_000.0);
        let mut data = main.clone();
        data.extend(&level);
        let lod = Lod { main: main.len(), levels: vec![level.len()], ceilings: vec![], alone: 1 };
        let mut layer = stroke(data, Some(lod));
        add(&mut layer);
        assert_eq!(items(&layer.data[..main.len()]), items(&main), "the main part keeps its items");
        assert_eq!(items(&layer.data[main.len()..]), items(&level), "the level keeps its items");
        let parts = &layer.cells[0].as_ref().expect("cells").parts;
        assert_eq!(parts.iter().map(|p| p.range.clone()).collect::<Vec<_>>(), vec![0..main.len(), main.len()..main.len() + level.len()]);
    }

    #[test]
    fn small_batches_and_markers_stay_as_they_are() {
        let data = segments(400, 100.0);
        let mut layer = stroke(data.clone(), None);
        add(&mut layer);
        assert_eq!(layer.data, data);
        assert!(layer.cells.is_empty());
        let mut layer = stroke(segments(20_000, 10_000.0), None);
        layer.batches[0].kind = BatchKind::Marker {
            unit: Unit::World,
            look: crate::batches::MarkerLook::Shape {
                shape: "circle".into(),
                fill: Some([1.0; 4]),
                stroke: None,
                stroke_width: 0.0,
                params: [0.0; 4],
            },
            offset: [0.0, 0.0],
            anchor: [0.0, 0.0],
            opacity: 1.0,
            extent: [1.0, 1.0],
            vectors: false,
        };
        let before = layer.data.clone();
        add(&mut layer);
        assert_eq!(layer.data, before, "a marker batch keeps its order");
    }
}
