//! Birikimli maliyet's network (docs/adr/0236 §4): the cell centres joined
//! to 8 or 16 neighbours, a step costing the average of the cells it touches
//! times its length in metres; the least sums from the sources by Dijkstra
//! over the radix heap of docs/adr/0235 §3, then every cell's predecessor by
//! the ADR's rule, the sources carried along in settling order, and a path
//! traced back from any cell.

use crate::frame::Frame;
use crate::hydro::heap::{RadixHeap, key_of, value_of};
use crate::par;

/// The moves in their order (§2): from east clockwise (cell space, v down).
pub const MOVES: [(i64, i64); 16] = [
    (1, 0),
    (2, 1),
    (1, 1),
    (1, 2),
    (0, 1),
    (-1, 2),
    (-1, 1),
    (-2, 1),
    (-1, 0),
    (-2, -1),
    (-1, -1),
    (-1, -2),
    (0, -1),
    (1, -2),
    (1, -1),
    (2, -1),
];

const EIGHT: [usize; 8] = [0, 2, 4, 6, 8, 10, 12, 14];
const SIXTEEN: [usize; 16] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];

/// A cell without a predecessor (a source, or one not reached).
pub const NO_MOVE: u8 = u8::MAX;

/// Cells settled a step.
pub const STEP_CELLS: usize = 1 << 21;

/// The neighbourhood (§4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Neighbours {
    Eight,
    Sixteen,
}

/// A row's steps: each move's x·x + y·y and its length √(x·x + y·y).
#[derive(Clone, Copy, Debug)]
struct RowSteps {
    sq: [f64; 16],
    len: [f64; 16],
}

impl RowSteps {
    fn of([a, b, c, d]: [f64; 4]) -> RowSteps {
        let mut sq = [0.0; 16];
        let mut len = [0.0; 16];
        for (q, &(di, dj)) in MOVES.iter().enumerate() {
            let (di, dj) = (di as f64, dj as f64);
            let x = a * di + b * dj;
            let y = c * di + d * dj;
            sq[q] = x * x + y * y;
            len[q] = libm::sqrt(sq[q]);
        }
        RowSteps { sq, len }
    }
}

/// The cells a move touches besides its ends: a diagonal's two side cells, a knight's two crossed cells.
fn sides(q: usize) -> [(i64, i64); 2] {
    let (di, dj) = MOVES[q];
    match (di.abs(), dj.abs()) {
        (1, 1) => [(di, 0), (0, dj)],
        (2, 1) => [(di / 2, 0), (di / 2, dj)],
        (1, 2) => [(0, dj / 2), (di, dj / 2)],
        _ => [(0, 0), (0, 0)],
    }
}

/// A move's kind: how its average is taken (§4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Straight,
    Diagonal,
    Knight,
}

fn kind(q: usize) -> Kind {
    match MOVES[q].0.abs() + MOVES[q].1.abs() {
        1 => Kind::Straight,
        2 => Kind::Diagonal,
        _ => Kind::Knight,
    }
}

/// The cost grid in memory.
pub struct Network {
    pub width: u32,
    pub height: u32,
    /// Cost per metre, NaN where a cell cannot be entered (no cost, or no height on the surface).
    pub cost: Vec<f64>,
    /// The surface's heights, when given.
    pub z: Option<Vec<f64>>,
    rows: Vec<RowSteps>,
    geographic: bool,
    moves: &'static [usize],
    surface_length: bool,
    /// The largest |Δz| a step may climb or fall per metre (infinity: no limit).
    slope: f64,
    /// Each move's offset in the arrays, for a cell two from every edge.
    off: [isize; 16],
    /// Each move's kind, its touched cells and their offsets.
    kinds: [Kind; 16],
    sides: [[(i64, i64); 2]; 16],
    side_off: [[isize; 2]; 16],
}

impl Network {
    /// A network over `frame`'s cells, every cost NaN until read; `slope` the
    /// largest grade in per cent (0: none).
    pub fn new(
        frame: &Frame,
        neighbours: Neighbours,
        surface: bool,
        surface_length: bool,
        slope: f64,
    ) -> Network {
        let (w, h) = (frame.width, frame.height);
        let n = w as usize * h as usize;
        let rows = if frame.geographic {
            (0..h).map(|j| RowSteps::of(frame.axes(j))).collect()
        } else {
            vec![RowSteps::of(frame.axes(0))]
        };
        let wi = w as isize;
        let at = |(di, dj): (i64, i64)| dj as isize * wi + di as isize;
        let off: [isize; 16] = std::array::from_fn(|q| at(MOVES[q]));
        let side: [[(i64, i64); 2]; 16] = std::array::from_fn(sides);
        Network {
            width: w,
            height: h,
            cost: vec![f64::NAN; n],
            z: surface.then(|| vec![f64::NAN; n]),
            rows,
            geographic: frame.geographic,
            moves: match neighbours {
                Neighbours::Eight => &EIGHT,
                Neighbours::Sixteen => &SIXTEEN,
            },
            surface_length: surface && surface_length,
            slope: if surface && slope > 0.0 {
                slope / 100.0
            } else {
                f64::INFINITY
            },
            off,
            kinds: std::array::from_fn(kind),
            sides: side,
            side_off: std::array::from_fn(|q| [at(side[q][0]), at(side[q][1])]),
        }
    }

    pub fn len(&self) -> usize {
        self.cost.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cost.is_empty()
    }

    /// The moves taken.
    pub fn moves(&self) -> &'static [usize] {
        self.moves
    }

    #[inline(always)]
    fn steps(&self, j: usize) -> &RowSteps {
        if self.geographic {
            &self.rows[j]
        } else {
            &self.rows[0]
        }
    }

    /// The plan length of move `q` out of a cell on row `j`.
    #[inline]
    pub fn length(&self, j: usize, q: usize) -> f64 {
        self.steps(j).len[q]
    }

    /// Whether cell (`i`, `j`) is two cells or more from every edge (every move's cells on the grid).
    #[inline(always)]
    fn inner(&self, i: usize, j: usize) -> bool {
        i >= 2 && j >= 2 && i + 2 < self.width as usize && j + 2 < self.height as usize
    }

    /// The cell move `q` from cell `k` at (`i`, `j`) reaches, inside the grid.
    #[inline(always)]
    fn target(&self, i: usize, j: usize, k: usize, q: usize, inner: bool) -> Option<usize> {
        if inner {
            return Some((k as isize + self.off[q]) as usize);
        }
        let (w, h) = (self.width as usize, self.height as usize);
        let (di, dj) = MOVES[q];
        let (a, b) = (i as i64 + di, j as i64 + dj);
        (a >= 0 && b >= 0 && a < w as i64 && b < h as i64).then(|| b as usize * w + a as usize)
    }

    /// The cost of cell (`i` + `di`, `j` + `dj`), NaN off the grid.
    #[inline(always)]
    fn cost_at(&self, i: usize, j: usize, (di, dj): (i64, i64)) -> f64 {
        let (a, b) = (i as i64 + di, j as i64 + dj);
        if a < 0 || b < 0 || a >= i64::from(self.width) || b >= i64::from(self.height) {
            return f64::NAN;
        }
        self.cost[b as usize * self.width as usize + a as usize]
    }

    /// Move `q` out of cell `k` at (`i`, `j`), when it can be taken: the
    /// cell it reaches and the step's cost (§4).
    #[inline(always)]
    pub fn step(&self, i: usize, j: usize, k: usize, q: usize) -> Option<(usize, f64)> {
        let inner = self.inner(i, j);
        let m = self.target(i, j, k, q, inner)?;
        Some((m, self.cost_to(i, j, k, q, m, inner)?))
    }

    /// The cost of move `q` out of cell `k` at (`i`, `j`) to cell `m`, when it can be taken.
    #[inline(always)]
    fn cost_to(
        &self,
        i: usize,
        j: usize,
        k: usize,
        q: usize,
        m: usize,
        inner: bool,
    ) -> Option<f64> {
        let c1 = self.cost[m];
        if c1.is_nan() {
            return None;
        }
        let c0 = self.cost[k];
        let average = match self.kinds[q] {
            Kind::Straight => (c0 + c1) / 2.0,
            kind => {
                let (cs, ct) = if inner {
                    let [s, t] = self.side_off[q];
                    (
                        self.cost[(k as isize + s) as usize],
                        self.cost[(k as isize + t) as usize],
                    )
                } else {
                    let [s, t] = self.sides[q];
                    (self.cost_at(i, j, s), self.cost_at(i, j, t))
                };
                if kind == Kind::Knight {
                    if cs.is_nan() || ct.is_nan() {
                        return None;
                    }
                    (((c0 + c1) + cs) + ct) / 4.0
                } else {
                    if cs.is_nan() && ct.is_nan() {
                        return None;
                    }
                    (c0 + c1) / 2.0
                }
            }
        };
        let steps = self.steps(j);
        let mut len = steps.len[q];
        if let Some(z) = &self.z {
            let dz = z[m] - z[k];
            if dz.abs() > self.slope * len {
                return None;
            }
            if self.surface_length {
                len = libm::sqrt(steps.sq[q] + dz * dz);
            }
        }
        Some(average * len)
    }
}

/// A search from sources: the least sums, the cells in settling order.
pub struct Search {
    /// Each cell's sum (infinity: not reached).
    pub acc: Vec<f64>,
    heap: RadixHeap,
    /// The cells settled, in order.
    pub order: Vec<u32>,
    /// Sums above this are not followed (infinity: none).
    max: f64,
    /// Stop once the next sum is above this (the paths' destinations), when set.
    pub stop: f64,
    pub done: bool,
    /// A step's cost vanished against a sum (float64's precision).
    pub lost: bool,
}

impl Search {
    /// A search from `sources` (cells; those that cannot be entered left out).
    pub fn new(net: &Network, sources: &[u32], max: f64) -> Search {
        let mut acc = vec![f64::INFINITY; net.len()];
        let mut heap = RadixHeap::new();
        for &k in sources {
            let k = k as usize;
            if !net.cost[k].is_nan() && acc[k] != 0.0 {
                acc[k] = 0.0;
                heap.push(key_of(0.0), k as u32);
            }
        }
        Search {
            acc,
            heap,
            order: Vec::new(),
            max: if max > 0.0 { max } else { f64::INFINITY },
            stop: f64::INFINITY,
            done: false,
            lost: false,
        }
    }

    /// Settles up to `budget` cells; whether the search is over.
    pub fn advance(&mut self, net: &Network, budget: usize) -> bool {
        if self.done {
            return true;
        }
        let w = net.width as usize;
        let mut settled = 0;
        while settled < budget {
            let Some((key, k)) = self.heap.pop() else {
                self.done = true;
                return true;
            };
            let a = value_of(key);
            let k = k as usize;
            if a != self.acc[k] {
                continue;
            }
            if a > self.max || a > self.stop {
                self.done = true;
                return true;
            }
            self.order.push(k as u32);
            settled += 1;
            let (i, j) = (k % w, k / w);
            let inner = net.inner(i, j);
            for &q in net.moves {
                let Some(m) = net.target(i, j, k, q, inner) else {
                    continue;
                };
                // A cell at or below this sum (settled, or reached as cheaply) cannot gain.
                if self.acc[m] <= a {
                    continue;
                }
                if let Some(c) = net.cost_to(i, j, k, q, m, inner) {
                    let na = a + c;
                    if na < self.acc[m] {
                        if na <= a {
                            self.lost = true;
                        }
                        self.acc[m] = na;
                        self.heap.push(key_of(na), m as u32);
                    }
                }
            }
        }
        false
    }
}

/// The surface length of move `q` out of a cell on row `j` climbing `dz` (§4): √(x·x + y·y + Δz·Δz).
#[inline]
pub fn surface_length(net: &Network, j: usize, q: usize, dz: f64) -> f64 {
    libm::sqrt(net.steps(j).sq[q] + dz * dz)
}

/// Cell `c`'s predecessor (§4): among its neighbours `n` with
/// sum(n) + step(n → c) = sum(c), the least sum, then the least cell
/// number; the move from it, or `NO_MOVE` (a source, or not reached).
#[inline]
pub fn predecessor(net: &Network, acc: &[f64], c: usize) -> u8 {
    let ac = acc[c];
    if !ac.is_finite() || ac == 0.0 {
        return NO_MOVE;
    }
    let w = net.width as usize;
    let (i, j) = (c % w, c / w);
    let mut best: Option<(f64, usize, u8)> = None;
    for &q in net.moves {
        let (di, dj) = MOVES[q];
        let (a, b) = (i as i64 - di, j as i64 - dj);
        if a < 0 || b < 0 || a >= i64::from(net.width) || b >= i64::from(net.height) {
            continue;
        }
        let n = b as usize * w + a as usize;
        let an = acc[n];
        if !an.is_finite() {
            continue;
        }
        if let Some((m, cost)) = net.step(a as usize, b as usize, n, q)
            && m == c
            && an + cost == ac
        {
            let better = match best {
                None => true,
                Some((ab, nb, _)) => an < ab || (an == ab && n < nb),
            };
            if better {
                best = Some((an, n, q as u8));
            }
        }
    }
    best.map_or(NO_MOVE, |b| b.2)
}

/// Every cell's predecessor, rows on the threads.
pub fn predecessors(net: &Network, acc: &[f64], threads: usize) -> Vec<u8> {
    let w = net.width as usize;
    let mut out = vec![NO_MOVE; net.len()];
    par::rows(threads, &mut out, w, &|first, chunk: &mut [u8]| {
        for (r, row) in chunk.chunks_mut(w).enumerate() {
            let j = first + r;
            for (i, o) in row.iter_mut().enumerate() {
                *o = predecessor(net, acc, j * w + i);
            }
        }
    });
    out
}

/// The cell move `q` came from to `c`.
#[inline]
pub fn from_cell(net: &Network, c: usize, q: u8) -> usize {
    let w = net.width as usize;
    let (di, dj) = MOVES[q as usize];
    let (i, j) = (c % w, c / w);
    (j as i64 - dj) as usize * w + (i as i64 - di) as usize
}

/// Each settled cell's source (the sources' own given): carried from its
/// predecessor in settling order.
pub fn sources_of(net: &Network, order: &[u32], pred: &[u8], own: &mut [f64]) {
    for &k in order {
        let k = k as usize;
        let q = pred[k];
        if q != NO_MOVE {
            own[k] = own[from_cell(net, k, q)];
        }
    }
}

/// The cells from a source to `end`, by the predecessors (`pred` gives a
/// cell's, e.g. computed on demand).
pub fn path_to(net: &Network, end: usize, pred: &dyn Fn(usize) -> u8) -> Vec<usize> {
    let mut cells = vec![end];
    let mut c = end;
    loop {
        let q = pred(c);
        if q == NO_MOVE {
            break;
        }
        c = from_cell(net, c, q);
        cells.push(c);
    }
    cells.reverse();
    cells
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(w: u32, h: u32, cell: f64) -> Frame {
        Frame::new([0.0, cell, 0.0, 0.0, 0.0, -cell], w, h, false).expect("a frame")
    }

    #[test]
    fn a_constant_cost_is_the_networks_distance() {
        let f = frame(9, 9, 10.0);
        let mut net = Network::new(&f, Neighbours::Sixteen, false, false, 0.0);
        net.cost.fill(1.0);
        let mut s = Search::new(&net, &[40], 0.0);
        assert!(s.advance(&net, usize::MAX));
        // Straight, diagonal and knight's moves from the centre.
        assert_eq!(s.acc[41], 10.0);
        assert_eq!(s.acc[50], libm::sqrt(200.0));
        assert_eq!(s.acc[51], libm::sqrt(500.0));
        let pred = predecessors(&net, &s.acc, 2);
        assert_eq!(pred[40], NO_MOVE);
        assert_eq!(pred[51], 1);
        let path = path_to(&net, 0, &|c| pred[c]);
        assert_eq!(path.first(), Some(&40));
        assert_eq!(path.last(), Some(&0));
    }

    #[test]
    fn a_corner_joined_barrier_holds() {
        // A diagonal wall of no-cost cells: the diagonal step between them is not taken.
        let f = frame(4, 4, 1.0);
        let mut net = Network::new(&f, Neighbours::Eight, false, false, 0.0);
        net.cost.fill(1.0);
        for k in [3, 6, 9, 12] {
            net.cost[k] = f64::NAN;
        }
        let mut s = Search::new(&net, &[0], 0.0);
        s.advance(&net, usize::MAX);
        assert!(s.acc[15].is_infinite());
        assert!(s.acc[5].is_finite());
    }

    #[test]
    fn the_grade_limit_turns_the_path_and_both_ways() {
        let f = frame(5, 1, 10.0);
        let mut net = Network::new(&f, Neighbours::Eight, true, false, 10.0);
        net.cost.fill(1.0);
        net.z = Some(vec![0.0, 1.0, 3.0, 2.0, 2.5]);
        let mut s = Search::new(&net, &[0], 0.0);
        s.advance(&net, usize::MAX);
        // 0 → 1 climbs 1 m in 10 m (10 %): taken; 1 → 2 climbs 2 m: not.
        assert_eq!(s.acc[1], 10.0);
        assert!(s.acc[2].is_infinite());
    }
}
