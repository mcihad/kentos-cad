//! Dere ağı (docs/adr/0235 §10): the cells whose D8 accumulation (m²)
//! reaches the threshold; links from each source or junction along the D8
//! directions to the next junction or the path's end, numbered by their
//! first cells row by row; Strahler's and Shreve's orders; each link's line,
//! length, drop, slope, area and the link it runs into.

use super::flow::receiver;
use super::surface::{N, Surface, back};

/// A stream link.
#[derive(Clone, Debug, PartialEq)]
pub struct Link {
    /// Its cells downstream.
    pub cells: Vec<u32>,
    /// The junction it runs into (the next link's first cell), if any.
    pub end: Option<u32>,
    /// The link it runs into (1…), 0 for none.
    pub down: u32,
    pub strahler: u32,
    pub shreve: u32,
}

/// The network.
#[derive(Debug)]
pub struct Network {
    pub threshold: f64,
    pub links: Vec<Link>,
    /// Each stream cell's link (1…), 0 off the streams.
    pub link_of: Vec<u32>,
}

/// The stream network of `area` (D8 accumulation, m²) at `threshold` (0: the largest / 100).
pub fn network(s: &Surface, dirs: &[u8], area: &[f64], threshold: f64) -> Network {
    let n = s.len();
    let w = s.width as usize;
    let threshold = if threshold > 0.0 {
        threshold
    } else {
        let mut most = 0.0;
        for (k, &a) in area.iter().enumerate() {
            if s.valid(k) && a > most {
                most = a;
            }
        }
        most / 100.0
    };
    let stream = |k: usize| s.valid(k) && area[k] >= threshold;
    // The stream donors of every stream cell.
    let mut ups = vec![0u8; n];
    for k in 0..n {
        if stream(k)
            && let Some(to) = receiver(s, dirs, k)
        {
            ups[to] += 1;
        }
    }
    let starts: Vec<usize> = (0..n).filter(|&k| stream(k) && ups[k] != 1).collect();
    let mut number = vec![0u32; n];
    for (x, &k) in starts.iter().enumerate() {
        number[k] = x as u32 + 1;
    }
    let mut link_of = vec![0u32; n];
    let mut links: Vec<Link> = Vec::with_capacity(starts.len());
    for &k in &starts {
        let mut cells = vec![k as u32];
        let mut x = k;
        let mut end = None;
        while let Some(to) = receiver(s, dirs, x) {
            if ups[to] != 1 {
                end = Some(to as u32);
                break;
            }
            cells.push(to as u32);
            x = to;
        }
        for &c in &cells {
            link_of[c as usize] = number[k];
        }
        links.push(Link {
            cells,
            end,
            down: end.map_or(0, |e| number[e as usize]),
            strahler: 0,
            shreve: 0,
        });
    }
    // The orders, upstream links first.
    let mut feeders: Vec<Vec<u32>> = vec![Vec::new(); links.len() + 1];
    for (x, l) in links.iter().enumerate() {
        if l.down != 0 {
            feeders[l.down as usize].push(x as u32 + 1);
        }
    }
    let mut stack: Vec<(u32, bool)> = (1..=links.len() as u32).rev().map(|x| (x, false)).collect();
    while let Some((x, ready)) = stack.pop() {
        let at = x as usize - 1;
        if links[at].strahler != 0 {
            continue;
        }
        let up = &feeders[x as usize];
        if !ready {
            stack.push((x, true));
            for &u in up {
                if links[u as usize - 1].strahler == 0 {
                    stack.push((u, false));
                }
            }
            continue;
        }
        if up.is_empty() {
            links[at].strahler = 1;
            links[at].shreve = 1;
            continue;
        }
        let top = up
            .iter()
            .map(|&u| links[u as usize - 1].strahler)
            .max()
            .unwrap_or(1);
        let many = up
            .iter()
            .filter(|&&u| links[u as usize - 1].strahler == top)
            .count();
        links[at].strahler = if many >= 2 { top + 1 } else { top };
        links[at].shreve = up.iter().map(|&u| links[u as usize - 1].shreve).sum();
    }
    let _ = (w, N, back(0));
    Network {
        threshold,
        links,
        link_of,
    }
}

impl Link {
    /// Its path: its cells and the junction it runs into.
    pub fn path(&self) -> Vec<u32> {
        let mut p = self.cells.clone();
        if let Some(e) = self.end {
            p.push(e);
        }
        p
    }
}

/// A path's length (m): each step's along the row it starts from, in order.
pub fn path_length(s: &Surface, path: &[u32]) -> f64 {
    let w = s.width as i64;
    let mut total = 0.0;
    for pair in path.windows(2) {
        let (a, b) = (i64::from(pair[0]), i64::from(pair[1]));
        let (ia, ja, ib, jb) = (a % w, a / w, b % w, b / w);
        let d = (ib - ia, jb - ja);
        let q = N.iter().position(|&x| x == d).unwrap_or(0);
        total += s.row(ja as usize).step[q];
    }
    total
}
