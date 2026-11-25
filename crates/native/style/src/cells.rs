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
//! picture) gives one colour a pixel too.
//!
//! **Markers keep their order.** A batch's markers are one symbol, but a picture's colours differ across it, and which
//! of two overlapping markers is on top would show. Their numbers stay where they are; their cells keep the items they
//! hold, each cell's in their order (`CellPart::order`), and a view draws the runs of the items its cells hold, in the
//! batch's order (the NCZ: a neighbourhood's 886 markers in 170 runs, of 222 664).

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
/// An instanced batch with fewer placements is drawn whole.
const LEAST_PLACEMENTS: usize = 8;
/// Cells across a part, at most (a grid of 64 × 64 at most).
const MOST_CELLS_ACROSS: usize = 64;
/// Draws a part takes at most for a view: nearer ranges are joined past it (each draw costs the CPU a few µs).
pub const MOST_RANGES: usize = 8;
/// Draws a part that keeps its order takes at most: its cells' items lie in runs through the whole part (a city's
/// markers in the document's order), and joining runs far apart draws much of it.
pub const MOST_RUNS: usize = 32;
/// Ranges fewer items apart are one draw: the few items between cost the GPU less than another draw costs the CPU.
const JOIN_GAP: u32 = 16;

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
    /// A part that keeps its order (a batch's markers): each cell's items (from the part's first) in their order, cell
    /// k's at `order[starts[k]..starts[k + 1]]`. None: the items are sorted by cell, cell k's are
    /// `starts[k]..starts[k + 1]`.
    pub order: Option<Vec<u32>>,
}

impl CellPart {
    /// The item ranges (from the part's first item) whose cells' boxes meet `view` (from the layers' origin) grown by
    /// `reach`, in their order, those next to one another joined; at most [`MOST_RANGES`], the nearest joined over the
    /// gap between them. A part that keeps its order gives the runs of the items those cells hold.
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
        let most = match &self.order {
            Some(order) => {
                out = runs(order, &out, self.starts.last().map_or(0, |&n| n as usize));
                MOST_RUNS
            }
            None => MOST_RANGES,
        };
        fewer(out, most)
    }
}

/// `ranges` (in order, apart) joined where fewer than [`JOIN_GAP`] items lie between them, then over their smallest gaps
/// until `most` are left: in one pass, the gaps to join found by their size.
fn fewer(ranges: Vec<Range<u32>>, most: usize) -> Vec<Range<u32>> {
    let most = most.max(1);
    let gaps: Vec<u32> = ranges.windows(2).map(|w| w[1].start - w[0].end).collect();
    let near = gaps.iter().filter(|&&g| g < JOIN_GAP).count();
    // The gaps joined: those under JOIN_GAP, and the smallest of the rest while more than `most` ranges would be left.
    let over = (ranges.len() - near).saturating_sub(most);
    let mut sizes: Vec<u32> = gaps.iter().copied().filter(|&g| g >= JOIN_GAP).collect();
    let limit = match over {
        0 => None,
        _ => {
            let (_, &mut nth, _) = sizes.select_nth_unstable(over - 1);
            Some(nth)
        }
    };
    // Gaps under the limit are joined, those at it while some of the `over` are left.
    let mut at_limit = limit.map_or(0, |l| over - sizes.iter().filter(|&&g| g < l).count());
    let mut out: Vec<Range<u32>> = Vec::with_capacity(ranges.len().min(most + 1));
    for (k, r) in ranges.into_iter().enumerate() {
        let join = k > 0 && {
            let g = gaps[k - 1];
            match limit {
                _ if g < JOIN_GAP => true,
                Some(l) if g < l => true,
                Some(l) if g == l && at_limit > 0 => {
                    at_limit -= 1;
                    true
                }
                _ => false,
            }
        };
        match out.last_mut() {
            Some(last) if join => last.end = r.end,
            _ => out.push(r),
        }
    }
    out
}

/// The items of the cells `cells` names (ranges of `order`, the part's items by cell), as runs of the part's own order:
/// the whole part when they are most of it. `n` is the part's item count.
fn runs(order: &[u32], cells: &[Range<u32>], n: usize) -> Vec<Range<u32>> {
    let held: usize = cells.iter().map(|r| r.len()).sum();
    if held == 0 {
        return Vec::new();
    }
    if held * 2 > n {
        return std::iter::once(0..n as u32).collect();
    }
    let mut met = vec![0u64; n.div_ceil(64)];
    for r in cells {
        for &item in &order[r.start as usize..r.end as usize] {
            met[item as usize / 64] |= 1 << (item % 64);
        }
    }
    let mut out: Vec<Range<u32>> = Vec::new();
    for (w, &word) in met.iter().enumerate() {
        let mut bits = word;
        while bits != 0 {
            let at = (w * 64) as u32 + bits.trailing_zeros();
            match out.last_mut() {
                Some(last) if last.end == at => last.end = at + 1,
                _ => out.push(at..at + 1),
            }
            bits &= bits - 1;
        }
    }
    out
}

/// Numbers an item of a batch takes, for a batch whose items cells may hold, and whether it keeps its order: a
/// stroke's segment, a fill's triangle (sorted by cell), a marker's place (in its order).
fn item_layout(b: &StyledBatch) -> Option<(usize, bool)> {
    if b.instances.is_some() {
        return None;
    }
    match &b.kind {
        BatchKind::Stroke { .. } => Some((6, false)),
        BatchKind::Fill {
            paint:
                FillPaintBatch::Solid { .. }
                | FillPaintBatch::Hatch { .. }
                | FillPaintBatch::Gradient { .. }
                | FillPaintBatch::Pattern { .. }
                | FillPaintBatch::Tile { .. },
        } => Some((6, false)),
        BatchKind::Marker { .. } => Some((5, true)),
        _ => None,
    }
}

/// An item's box (from its batch's tile): a segment's ends, a triangle's corners, a marker's place (the batch's reach
/// is its symbol's extent).
fn item_box(kind: &BatchKind, v: &[f32]) -> [f64; 4] {
    let p = |i: usize| (f64::from(v[i]), f64::from(v[i + 1]));
    let points: &[(f64, f64)] = &match kind {
        BatchKind::Stroke { .. } => [p(0), p(2), p(2)],
        BatchKind::Marker { .. } => [p(0), p(0), p(0)],
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
        if let Some(part) = placements(b, &layer.data) {
            cells[i] = Some(Arc::new(Cells { parts: vec![part] }));
            continue;
        }
        let Some((floats, keep)) = item_layout(b) else {
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
            if let Some(part) = sort(&b.kind, data, low, floats, keep, b.origin, r) {
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

/// The cells of an instanced batch's placements (a block's inserts, a symbol's vectors in its markers' places), in their
/// order: two overlapping inserts show which is on top. A placement's box is its definition's box carried by its
/// placing; its part's range is the placements' numbers (from the batch's first). A placement draws its whole
/// definition, so its cells hold [`ITEMS_A_CELL`] of the definition's items (a segment, a triangle), one placement at
/// least, and a batch whose placements draw fewer than [`LEAST_ITEMS`] items in all is drawn whole: the NCZ's symbols
/// drawn by their vectors are 7 batches of a few hundred placements, 64 890 triangles near the ground. None too for
/// fewer than [`LEAST_PLACEMENTS`], or a definition the renderer does not place (`Placed`: strokes, solid fills).
fn placements(b: &StyledBatch, data: &[f32]) -> Option<CellPart> {
    let inst = b.instances.as_ref()?;
    let count = inst.count as usize;
    let per = crate::batches::INSTANCE_FLOATS;
    // A segment's six numbers, a triangle's three corners.
    let items = (inst.definition() / 6).max(1);
    if count < LEAST_PLACEMENTS || count * items < LEAST_ITEMS {
        return None;
    }
    let numbers = data.get(b.range.clone())?;
    let (definition, placed) = (numbers.get(..inst.definition())?, numbers.get(inst.mesh..inst.mesh + count * per)?);
    let points: Vec<(f32, f32)> = match &b.kind {
        BatchKind::Stroke { .. } => definition.as_chunks::<6>().0.iter().flat_map(|v| [(v[0], v[1]), (v[2], v[3])]).collect(),
        BatchKind::Fill { paint: FillPaintBatch::Solid { .. } } => {
            definition.as_chunks::<2>().0.iter().map(|v| (v[0], v[1])).collect()
        }
        _ => return None,
    };
    let d = points.iter().fold([f64::MAX, f64::MAX, f64::MIN, f64::MIN], |b, &(x, y)| {
        let (x, y) = (f64::from(x), f64::from(y));
        [b[0].min(x), b[1].min(y), b[2].max(x), b[3].max(y)]
    });
    if !d.iter().all(|v| v.is_finite()) {
        return None;
    }
    // Each placement's box: its definition's corners by its linear part (x axis, y axis) and translation.
    let boxes: Vec<[f64; 4]> = placed
        .chunks_exact(per)
        .map(|x| {
            let x: [f64; 6] = std::array::from_fn(|i| f64::from(x[i]));
            [(d[0], d[1]), (d[2], d[1]), (d[0], d[3]), (d[2], d[3])].iter().fold(
                [f64::MAX, f64::MAX, f64::MIN, f64::MIN],
                |b, &(px, py)| {
                    let (qx, qy) = (x[0] * px + x[2] * py + x[4], x[1] * px + x[3] * py + x[5]);
                    [b[0].min(qx), b[1].min(qy), b[2].max(qx), b[3].max(qy)]
                },
            )
        })
        .collect();
    let per_cell = (ITEMS_A_CELL / items).max(1);
    let (mut part, order) = grid(&boxes, b.origin, inst.mesh..inst.mesh + count * per, per, per_cell)?;
    part.order = Some(order.iter().map(|&i| i as u32).collect());
    Some(part)
}

/// One part's items sorted by the cell of a grid over them, stably, or kept in their order (`keep`); its cells.
fn sort(
    kind: &BatchKind,
    data: &mut [f32],
    low: Option<&mut [f32]>,
    floats: usize,
    keep: bool,
    origin: [f64; 2],
    range: Range<usize>,
) -> Option<CellPart> {
    let boxes: Vec<[f64; 4]> = data.chunks(floats).map(|v| item_box(kind, v)).collect();
    let (mut part, order) = grid(&boxes, origin, range, floats, ITEMS_A_CELL)?;
    if keep {
        part.order = Some(order.iter().map(|&i| i as u32).collect());
    } else {
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
    }
    Some(part)
}

/// The cells of a grid over items of these boxes (from the batch's tile): about `per_cell` items a cell, the cells as
/// square as the part allows; each cell's items in their order (`order`, the items by cell).
fn grid(
    boxes: &[[f64; 4]],
    origin: [f64; 2],
    range: Range<usize>,
    floats: usize,
    per_cell: usize,
) -> Option<(CellPart, Vec<usize>)> {
    let n = boxes.len();
    let all = boxes.iter().fold([f64::MAX, f64::MAX, f64::MIN, f64::MIN], |b, i| {
        [b[0].min(i[0]), b[1].min(i[1]), b[2].max(i[2]), b[3].max(i[3])]
    });
    if !all.iter().all(|v| v.is_finite()) {
        return None;
    }
    let (w, h) = ((all[2] - all[0]).max(1e-6), (all[3] - all[1]).max(1e-6));
    let cells = (n / per_cell.max(1)).max(1) as f64;
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
    let part = CellPart {
        range,
        floats,
        corner: [all[0] + origin[0], all[1] + origin[1]],
        cell: size,
        nx,
        ny,
        starts,
        boxes: cell_boxes,
        spill,
        order: None,
    };
    Some((part, order))
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
    fn small_batches_stay_as_they_are() {
        let data = segments(400, 100.0);
        let mut layer = stroke(data.clone(), None);
        add(&mut layer);
        assert_eq!(layer.data, data);
        assert!(layer.cells.is_empty());
    }

    /// `n` markers' places over a square `side` metres (x, y, turn, width, height), deterministic: in a drawing's order
    /// (`blocks`: block by block of 500 m, as a city is drawn a neighbourhood after another), or scattered.
    fn markers(n: usize, side: f64, blocks: bool) -> StyledLayer {
        let mut seed = 99u64;
        let mut next = || {
            seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
            (seed >> 11) as f64 / (1u64 << 53) as f64
        };
        let mut places: Vec<(f64, f64)> = (0..n).map(|_| (next() * side, next() * side)).collect();
        if blocks {
            places.sort_by_key(|&(x, y)| ((y / 500.0) as u32, (x / 500.0) as u32));
        }
        let mut data = Vec::with_capacity(n * 5);
        for (i, (x, y)) in places.into_iter().enumerate() {
            data.extend([x as f32, y as f32, i as f32, 1.0, 1.0]);
        }
        let mut layer = stroke(data, None);
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
        layer
    }

    #[test]
    fn a_marker_batch_keeps_its_order_and_a_view_draws_the_runs_it_meets() {
        for blocks in [true, false] {
            a_marker_view(blocks);
        }
    }

    /// In a drawing's order a near view draws a few runs; scattered, its runs lie all through the batch and are joined
    /// past [`MOST_RUNS`]: every met item is still drawn, never more than the whole.
    fn a_marker_view(blocks: bool) {
        let mut layer = markers(20_000, 10_000.0, blocks);
        let before = layer.data.clone();
        add(&mut layer);
        assert_eq!(layer.data, before, "a marker batch keeps its numbers where they were");
        let part = &layer.cells[0].as_ref().expect("cells").parts[0];
        let order = part.order.as_ref().expect("the cells hold the items in their order");
        let n = before.len() / 5;
        let mut seen: Vec<u32> = order.clone();
        seen.sort_unstable();
        assert_eq!(seen, (0..n as u32).collect::<Vec<_>>(), "every item in one cell");
        for k in 0..part.nx * part.ny {
            let held = &order[part.starts[k] as usize..part.starts[k + 1] as usize];
            assert!(held.windows(2).all(|w| w[0] < w[1]), "cell {k}: its items in their order");
        }
        let reach = 2.0;
        for view in [[2_000.0, 3_000.0, 2_400.0, 3_300.0], [-50.0, -50.0, 150.0, 150.0], [9_000.0, 9_500.0, 11_000.0, 12_000.0]] {
            let ranges = part.ranges_in(view, reach);
            assert!(ranges.len() <= MOST_RUNS);
            assert!(ranges.windows(2).all(|w| w[0].end < w[1].start), "{view:?}: runs apart, in order");
            let drawn: usize = ranges.iter().map(|r| r.len()).sum();
            let mut met = 0;
            for item in 0..n {
                let v = &before[item * 5..item * 5 + 5];
                let (x, y) = (f64::from(v[0]) + 100.0, f64::from(v[1]) + 200.0);
                if x >= view[0] - reach && x <= view[2] + reach && y >= view[1] - reach && y <= view[3] + reach {
                    met += 1;
                    assert!(ranges.iter().any(|r| r.contains(&(item as u32))), "{view:?}: item {item} meets it");
                }
            }
            eprintln!("markers {}: {view:?} met {met}, drawn {drawn} in {} runs", if blocks { "in blocks" } else { "scattered" }, ranges.len());
            if blocks {
                assert!(drawn < n / 20, "{view:?}: {drawn} of {n} drawn for {met} met");
            }
            assert!(drawn <= n);
        }
        // A view over most of it draws it whole, in one range.
        assert_eq!(part.ranges_in([-1e6, -1e6, 1e6, 1e6], 0.0), vec![0..n as u32]);
    }

    /// A block's 5 000 inserts of a 4 m square (rotated, in blocks of 500 m as a drawing lays them) over 10 km.
    fn inserts() -> StyledLayer {
        let square = [[0.0, 0.0, 4.0, 0.0], [4.0, 0.0, 4.0, 4.0], [4.0, 4.0, 0.0, 4.0], [0.0, 4.0, 0.0, 0.0]];
        let mut data: Vec<f32> = square.iter().flat_map(|s| [s[0], s[1], s[2], s[3], 0.0, 3.0]).collect();
        let mesh = data.len();
        let mut seed = 5u64;
        let mut next = || {
            seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
            (seed >> 11) as f64 / (1u64 << 53) as f64
        };
        let mut places: Vec<(f64, f64, f64)> = (0..5_000).map(|_| (next() * 10_000.0, next() * 10_000.0, next() * 6.0)).collect();
        places.sort_by_key(|&(x, y, _)| ((y / 500.0) as u32, (x / 500.0) as u32));
        for (x, y, turn) in places {
            let (s, c) = turn.sin_cos();
            data.extend([c, s, -s, c, x, y, 1.0, 0.0].map(|v| v as f32));
        }
        let mut layer = stroke(data, None);
        layer.batches[0].instances =
            Some(crate::batches::Instances { mesh, count: 5_000, scale: 1.0, proxy: None, mark: None });
        layer
    }

    #[test]
    fn an_instanced_batch_keeps_its_placements_order_and_a_view_draws_those_it_meets() {
        let mut layer = inserts();
        let before = layer.data.clone();
        add(&mut layer);
        assert_eq!(layer.data, before, "the placements where they were");
        let part = &layer.cells[0].as_ref().expect("cells").parts[0];
        let mesh = 24;
        assert_eq!((part.range.clone(), part.floats), (mesh..mesh + 5_000 * 8, 8));
        assert!(part.order.is_some(), "in their order");
        let reach = 1.0;
        for view in [[2_000.0, 3_000.0, 2_400.0, 3_300.0], [9_000.0, 9_500.0, 11_000.0, 12_000.0]] {
            let ranges = part.ranges_in(view, reach);
            let drawn: usize = ranges.iter().map(|r| r.len()).sum();
            let mut met = 0;
            for (n, x) in before[mesh..].chunks(8).enumerate() {
                // The placed square's corners, from the layers' origin.
                let corners = [(0.0f32, 0.0f32), (4.0, 0.0), (0.0, 4.0), (4.0, 4.0)]
                    .map(|(px, py)| (f64::from(x[0] * px + x[2] * py + x[4]) + 100.0, f64::from(x[1] * px + x[3] * py + x[5]) + 200.0));
                let (x0, x1) = corners.iter().fold((f64::MAX, f64::MIN), |b, c| (b.0.min(c.0), b.1.max(c.0)));
                let (y0, y1) = corners.iter().fold((f64::MAX, f64::MIN), |b, c| (b.0.min(c.1), b.1.max(c.1)));
                if x1 >= view[0] - reach && x0 <= view[2] + reach && y1 >= view[1] - reach && y0 <= view[3] + reach {
                    met += 1;
                    assert!(ranges.iter().any(|r| r.contains(&(n as u32))), "{view:?}: insert {n} meets it");
                }
            }
            assert!(met > 0 && drawn < 5_000 / 10, "{view:?}: {drawn} of 5 000 drawn for {met} met");
        }
    }

    /// A symbol of 200 segments (a ring) placed 120 times: few placements, each drawing much. Its cells hold a placement
    /// or so each, and a near view draws the few it meets; 4 placements are drawn whole.
    #[test]
    fn a_heavy_definitions_few_placements_have_cells() {
        let ring: Vec<f32> = (0..200)
            .flat_map(|k| {
                let (a, b) = (k as f64 / 200.0 * std::f64::consts::TAU, (k + 1) as f64 / 200.0 * std::f64::consts::TAU);
                [3.0 * a.cos(), 3.0 * a.sin(), 3.0 * b.cos(), 3.0 * b.sin(), 0.0, 3.0].map(|v| v as f32)
            })
            .collect();
        let place = |count: usize| {
            let mut data = ring.clone();
            for k in 0..count {
                let (x, y) = ((k % 12) as f32 * 800.0, (k / 12) as f32 * 800.0);
                data.extend([1.0, 0.0, 0.0, 1.0, x, y, 1.0, 0.0]);
            }
            let mut layer = stroke(data, None);
            layer.batches[0].instances =
                Some(crate::batches::Instances { mesh: ring.len(), count: count as u32, scale: 1.0, proxy: None, mark: None });
            add(&mut layer);
            layer
        };
        let layer = place(120);
        let part = &layer.cells[0].as_ref().expect("cells for 120 placements of 200 segments").parts[0];
        assert!(part.nx * part.ny >= 64, "{} × {} cells", part.nx, part.ny);
        // Round the fourth placement of the second row (2 400, 800 from the batch's tile; its origin is 100, 200).
        let ranges = part.ranges_in([2_490.0, 990.0, 2_510.0, 1_010.0], 1.0);
        assert_eq!(ranges, vec![15..16], "the one placement the view meets");
        assert!(place(4).cells.is_empty(), "four placements are drawn whole");
    }

}
