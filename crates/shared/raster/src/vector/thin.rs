//! Thin lines to paths (docs/adr/0234 §5): Zhang–Suen thinning of a
//! foreground mask with Lü and Wang's correction, the skeleton's
//! m-adjacency, and its paths from node to node (or round a loop), short
//! spurs and pieces dropped.
//!
//! P1's neighbours P2 … P9 run clockwise from the north (cell space v − 1):
//! N, NE, E, SE, S, SW, W, NW. Thinning deletes, in each sub-iteration and
//! all at once from the image as the sub-iteration found it, the pixels
//! with 3 ≤ B ≤ 6 (Zhang and Suen's 2 ≤ B eats a line two pixels thick on a
//! diagonal from its ends; Lü and Wang, 1986), A = 1 and P2·P4·P6 = 0,
//! P4·P6·P8 = 0 (the first) or P2·P4·P8 = 0, P2·P6·P8 = 0 (the second),
//! until a round deletes nothing.
//! Only a pixel with a background neighbour can go (an inner one has B = 8,
//! one more B = 7), so the work walks the border pixels alone; the image
//! outside the mask is background.

use crate::par;

/// P2 … P9 as (du, dv).
const NB: [(i64, i64); 8] = [
    (0, -1),
    (1, -1),
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
    (-1, -1),
];

/// A foreground mask: 1 a line's pixel.
#[derive(Clone, Debug)]
pub struct Mask {
    pub w: usize,
    pub h: usize,
    pub cells: Vec<u8>,
}

impl Mask {
    pub fn new(w: usize, h: usize) -> Mask {
        Mask {
            w,
            h,
            cells: vec![0; w * h],
        }
    }

    #[inline]
    fn on(&self, i: i64, j: i64) -> bool {
        i >= 0
            && j >= 0
            && (i as usize) < self.w
            && (j as usize) < self.h
            && self.cells[j as usize * self.w + i as usize] != 0
    }

    /// P2 … P9 of pixel k as bits 0 … 7.
    #[inline]
    fn code(&self, k: usize) -> u8 {
        let (i, j) = ((k % self.w) as i64, (k / self.w) as i64);
        let mut c = 0u8;
        for (b, &(du, dv)) in NB.iter().enumerate() {
            if self.on(i + du, j + dv) {
                c |= 1 << b;
            }
        }
        c
    }
}

/// Whether a neighbourhood (P2 … P9 as bits 0 … 7) lets sub-iteration `second` delete P1.
fn deletes(code: u8, second: bool) -> bool {
    let p = |n: usize| (code >> (n - 2)) & 1 == 1;
    let b = code.count_ones();
    if !(3..=6).contains(&b) {
        return false;
    }
    let mut a = 0;
    for n in 2..=9 {
        let next = if n == 9 { 2 } else { n + 1 };
        if !p(n) && p(next) {
            a += 1;
        }
    }
    if a != 1 {
        return false;
    }
    if second {
        !((p(2) && p(4) && p(8)) || (p(2) && p(6) && p(8)))
    } else {
        !((p(2) && p(4) && p(6)) || (p(4) && p(6) && p(8)))
    }
}

/// The foreground pixels (their indices, row by row), looked for on `threads`.
fn foreground(mask: &Mask, threads: usize) -> Vec<u32> {
    const ROWS: usize = 64;
    let starts: Vec<usize> = (0..mask.h).step_by(ROWS).collect();
    par::map(threads, &starts, &|&j0| {
        let (a, b) = (j0 * mask.w, (j0 + ROWS).min(mask.h) * mask.w);
        (a..b)
            .filter(|&k| mask.cells[k] != 0)
            .map(|k| k as u32)
            .collect::<Vec<u32>>()
    })
    .into_iter()
    .flatten()
    .collect()
}

/// Zhang–Suen thinning in place.
pub fn thin(mask: &mut Mask, threads: usize) {
    let table: [[bool; 256]; 2] = [
        std::array::from_fn(|c| deletes(c as u8, false)),
        std::array::from_fn(|c| deletes(c as u8, true)),
    ];
    // The border: foreground pixels with a background neighbour.
    let mut listed = vec![false; mask.cells.len()];
    let mut border: Vec<u32> = Vec::new();
    for k in foreground(mask, threads) {
        if mask.code(k as usize) != 0xff {
            listed[k as usize] = true;
            border.push(k);
        }
    }
    loop {
        let mut any = false;
        for second in [false, true] {
            let t = &table[usize::from(second)];
            let m = &*mask;
            let chunks: Vec<&[u32]> = border.chunks(16_384).collect();
            let gone: Vec<Vec<u32>> = par::map(threads, &chunks, &|chunk: &&[u32]| {
                chunk
                    .iter()
                    .copied()
                    .filter(|&k| m.cells[k as usize] != 0 && t[m.code(k as usize) as usize])
                    .collect()
            });
            let gone: Vec<u32> = gone.into_iter().flatten().collect();
            if gone.is_empty() {
                continue;
            }
            any = true;
            for &k in &gone {
                mask.cells[k as usize] = 0;
            }
            // Their neighbours now border the background.
            for &k in &gone {
                let (i, j) = ((k as usize % mask.w) as i64, (k as usize / mask.w) as i64);
                for &(du, dv) in &NB {
                    if mask.on(i + du, j + dv) {
                        let n = (j + dv) as usize * mask.w + (i + du) as usize;
                        if !listed[n] {
                            listed[n] = true;
                            border.push(n as u32);
                        }
                    }
                }
            }
            border.retain(|&k| mask.cells[k as usize] != 0);
        }
        if !any {
            break;
        }
    }
}

/// A pixel's m-neighbours (P2 … P9 as bits 0 … 7): its side neighbours, and
/// a corner neighbour when neither of the two side pixels they share is on.
#[inline]
fn m_bits(mask: &Mask, i: i64, j: i64) -> u8 {
    let c = |n: usize| mask.on(i + NB[n].0, j + NB[n].1);
    let (n, e, s, w) = (c(0), c(2), c(4), c(6));
    let mut bits = 0u8;
    for (b, on) in [
        n,
        c(1) && !n && !e,
        e,
        c(3) && !e && !s,
        s,
        c(5) && !s && !w,
        w,
        c(7) && !w && !n,
    ]
    .into_iter()
    .enumerate()
    {
        if on {
            bits |= 1 << b;
        }
    }
    bits
}

/// A path: its pixels (i, j) in order; a loop ends on its first pixel again.
pub type Path = Vec<(u32, u32)>;

/// The skeleton's paths: from each node (row by row; a node's directions
/// P2 … P9 in turn) along pixels of two neighbours to the next node, each
/// edge once; then the loops without nodes, from their first pixel row by
/// row towards its first neighbour. Lone pixels give nothing.
pub fn paths(mask: &Mask) -> Vec<Path> {
    paths_of(mask, &foreground(mask, 1))
}

/// [`paths`] over the foreground pixels `on` (row by row).
fn paths_of(mask: &Mask, on: &[u32]) -> Vec<Path> {
    let (w, h) = (mask.w, mask.h);
    let degree = |k: usize| m_bits(mask, (k % w) as i64, (k / w) as i64).count_ones();
    let mut walked = vec![0u8; w * h];
    let mut out: Vec<Path> = Vec::new();
    let step = |k: usize, b: usize| -> usize {
        let (i, j) = ((k % w) as i64 + NB[b].0, (k / w) as i64 + NB[b].1);
        j as usize * w + i as usize
    };
    let back = |b: usize| (b + 4) % 8;
    let follow = |start: usize, b0: usize, walked: &mut Vec<u8>| -> Path {
        let mut path: Path = vec![((start % w) as u32, (start / w) as u32)];
        let (mut prev, mut b) = (start, b0);
        loop {
            walked[prev] |= 1 << b;
            let cur = step(prev, b);
            walked[cur] |= 1 << back(b);
            path.push(((cur % w) as u32, (cur / w) as u32));
            if cur == start || degree(cur) != 2 {
                return path;
            }
            let bits = m_bits(mask, (cur % w) as i64, (cur / w) as i64);
            // The other neighbour: the first not walked from here.
            let Some(nb) = (0..8).find(|&d| bits & (1 << d) != 0 && walked[cur] & (1 << d) == 0)
            else {
                return path;
            };
            prev = cur;
            b = nb;
        }
    };
    for &k in on {
        let k = k as usize;
        let d = degree(k);
        if d == 2 || d == 0 {
            continue;
        }
        let bits = m_bits(mask, (k % w) as i64, (k / w) as i64);
        for b in 0..8 {
            if bits & (1 << b) != 0 && walked[k] & (1 << b) == 0 {
                out.push(follow(k, b, &mut walked));
            }
        }
    }
    for &k in on {
        let k = k as usize;
        if degree(k) != 2 || walked[k] != 0 {
            continue;
        }
        let bits = m_bits(mask, (k % w) as i64, (k / w) as i64);
        let Some(b) = (0..8).find(|&d| bits & (1 << d) != 0) else {
            continue;
        };
        out.push(follow(k, b, &mut walked));
    }
    out
}

/// The paths after dropping spurs (a free end to a junction, fewer than
/// `least` steps: their pixels but the junction's go and the skeleton is
/// built again) and then every path of fewer than `least` steps.
pub fn clean_paths(mask: &mut Mask, least: u32, threads: usize) -> Vec<Path> {
    let mut on = foreground(mask, threads);
    let found = paths_of(mask, &on);
    if least == 0 {
        return found;
    }
    let w = mask.w;
    let degree = |m: &Mask, (i, j): (u32, u32)| m_bits(m, i64::from(i), i64::from(j)).count_ones();
    let mut spurs: Vec<(u32, u32)> = Vec::new();
    for p in &found {
        let (Some(&a), Some(&b)) = (p.first(), p.last()) else {
            continue;
        };
        if a == b || (p.len() as u32 - 1) >= least {
            continue;
        }
        let (da, db) = (degree(mask, a), degree(mask, b));
        if da == 1 && db >= 3 {
            spurs.extend(&p[..p.len() - 1]);
        } else if db == 1 && da >= 3 {
            spurs.extend(&p[1..]);
        }
    }
    for (i, j) in spurs {
        mask.cells[j as usize * w + i as usize] = 0;
    }
    on.retain(|&k| mask.cells[k as usize] != 0);
    paths_of(mask, &on)
        .into_iter()
        .filter(|p| p.len() as u32 > least)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mask_of(rows: &[&str]) -> Mask {
        let mut m = Mask::new(rows[0].len(), rows.len());
        for (j, r) in rows.iter().enumerate() {
            for (i, c) in r.chars().enumerate() {
                if c == '#' {
                    m.cells[j * m.w + i] = 1;
                }
            }
        }
        m
    }

    fn text(m: &Mask) -> Vec<String> {
        (0..m.h)
            .map(|j| {
                (0..m.w)
                    .map(|i| if m.cells[j * m.w + i] != 0 { '#' } else { '.' })
                    .collect()
            })
            .collect()
    }

    #[test]
    fn a_thick_bar_thins_to_a_line() {
        let mut m = mask_of(&[
            "..........",
            ".########.",
            ".########.",
            ".########.",
            "..........",
        ]);
        thin(&mut m, 2);
        // The middle row's columns 1 … 6 stay (the left end's corner pixels have B = 3, its middle B = 5).
        assert_eq!(
            text(&m),
            vec![
                "..........",
                "..........",
                ".######...",
                "..........",
                ".........."
            ]
        );
        let p = paths(&m);
        assert_eq!(p, vec![(1..=6).map(|i| (i, 2)).collect::<Vec<_>>()]);
    }

    #[test]
    fn a_line_two_pixels_thick_on_a_diagonal_stays() {
        let mut m = Mask::new(12, 10);
        for k in 0..8 {
            m.cells[(k + 1) * 12 + k + 1] = 1;
            m.cells[(k + 1) * 12 + k + 2] = 1;
        }
        let before = m.cells.clone();
        thin(&mut m, 1);
        assert_eq!(m.cells, before);
        // One path along its steps, by sides.
        let p = paths(&m);
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].len(), 16);
    }

    #[test]
    fn a_cross_has_four_paths_and_spurs_go() {
        let mut m = mask_of(&[
            ".....#.....",
            ".....#.....",
            "###########",
            ".....#.....",
            ".....#.....",
        ]);
        let p = paths(&m);
        // Nodes row by row: the north tip, the west tip, the centre (east, then south), the rest walked.
        assert_eq!(
            p,
            vec![
                vec![(5, 0), (5, 1), (5, 2)],
                (0..6).map(|i| (i, 2)).collect::<Vec<_>>(),
                (5..11).map(|i| (i, 2)).collect::<Vec<_>>(),
                vec![(5, 2), (5, 3), (5, 4)],
            ]
        );
        // Spurs of under three steps go (north, south), the long arms join into one line.
        let q = clean_paths(&mut m, 3, 2);
        assert_eq!(q, vec![(0..11).map(|i| (i, 2)).collect::<Vec<_>>()]);
    }

    #[test]
    fn a_loop_and_its_corners() {
        // A diamond of corner steps: m-adjacency keeps one loop, no junctions.
        let m = mask_of(&["..#..", ".#.#.", "#...#", ".#.#.", "..#.."]);
        let p = paths(&m);
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].first(), p[0].last());
        assert_eq!(p[0].len(), 9);
        // An L of side steps: its corner pixel joins both arms (no diagonal shortcut).
        let m = mask_of(&["#..", "#..", "###"]);
        assert_eq!(
            paths(&m),
            vec![vec![(0, 0), (0, 1), (0, 2), (1, 2), (2, 2)]]
        );
    }
}
