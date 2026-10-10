//! Yoğunluğa göre kümeleme (DBSCAN) ve k-ortalamalar (docs/adr/0238 §10):
//! every object with a place gets a cluster (0: noise), its copy the
//! cluster's colour. Both are deterministic: places are visited in order,
//! ties go to the smaller order.

use std::collections::VecDeque;

use super::kdtree::{KdTree, dist2};
use super::weights::too_many;
use super::{MOST_PAIRS, Number, ObjectCopy, Placed, StatsRun, pair, unplaced_note};
use crate::entity::Shape;
use crate::vec2::Vec2;

/// The clusters' colours, by number (turning round), and the noise's.
pub const CLUSTER_COLORS: [&str; 10] = [
    "#1F77B4", "#FF7F0E", "#2CA02C", "#D62728", "#9467BD", "#8C564B", "#E377C2", "#17BECF",
    "#BCBD22", "#7F7F7F",
];
pub const NOISE_COLOR: &str = "#BDBDBD";

/// Most k-means rounds.
pub const MOST_ROUNDS: usize = 500;

/// The copies of the placed objects by their clusters (0: noise).
fn copies(run: &mut StatsRun, placed: &Placed, labels: &[u32]) -> Vec<usize> {
    let most = labels.iter().copied().max().unwrap_or(0) as usize;
    let mut sizes = vec![0usize; most + 1];
    for &l in labels {
        sizes[l as usize] += 1;
    }
    for (k, &l) in labels.iter().enumerate() {
        let (size, color) = if l == 0 {
            (String::new(), NOISE_COLOR)
        } else {
            (
                sizes[l as usize].to_string(),
                CLUSTER_COLORS[(l as usize - 1) % CLUSTER_COLORS.len()],
            )
        };
        run.copies.push(ObjectCopy {
            index: placed.index[k],
            attrs: vec![pair("Küme", l.to_string()), pair("Küme boyu", size)],
            color,
        });
    }
    sizes
}

/// DBSCAN: core places have at least `min_points` places (themselves too)
/// within `radius`; `border_noise`: only core places join clusters (DBSCAN\*).
pub fn dbscan(
    shapes: &[Shape],
    radius: f64,
    min_points: usize,
    border_noise: bool,
) -> Result<StatsRun, String> {
    let placed = Placed::of(shapes, |_| true);
    let mut run = StatsRun::default();
    run.warn_if(placed.unplaced, unplaced_note);
    let n = placed.pts.len();
    if n == 0 {
        return Err("Yeri bulunan nesne yok.".into());
    }
    let tree = KdTree::new(&placed.pts);
    let mut lists: Vec<Vec<u32>> = Vec::with_capacity(n);
    let mut pairs = 0usize;
    for &p in &placed.pts {
        let mut found = Vec::new();
        tree.within(p, radius, None, &mut found);
        pairs += found.len();
        if pairs > MOST_PAIRS {
            return Err(too_many("Yarıçap"));
        }
        found.sort_unstable();
        lists.push(found);
    }
    let core: Vec<bool> = lists.iter().map(|l| l.len() >= min_points).collect();
    const UNSET: u32 = u32::MAX;
    let mut labels = vec![UNSET; n];
    let mut next = 1u32;
    let mut queue = VecDeque::new();
    for i in 0..n {
        if labels[i] != UNSET || !core[i] {
            continue;
        }
        let c = next;
        next += 1;
        labels[i] = c;
        queue.push_back(i);
        while let Some(p) = queue.pop_front() {
            for &q in &lists[p] {
                let q = q as usize;
                if labels[q] != UNSET || (border_noise && !core[q]) {
                    continue;
                }
                labels[q] = c;
                if core[q] {
                    queue.push_back(q);
                }
            }
        }
    }
    for l in &mut labels {
        if *l == UNSET {
            *l = 0;
        }
    }
    let sizes = copies(&mut run, &placed, &labels);
    let clusters = sizes.len() - 1;
    let noise = sizes[0];
    run.numbers = vec![Number {
        name: "clusters",
        value: clusters as f64,
    }];
    run.summary = if clusters == 0 {
        format!("Küme bulunamadı; {n} nesnenin hepsi gürültü.")
    } else {
        format!("{n} nesne {clusters} kümeye ayrıldı; {noise} nesne gürültü.")
    };
    Ok(run)
}

/// The nearest centre to `p` (squared distance; ties to the smaller).
fn nearest_center(p: Vec2, centers: &[Vec2]) -> usize {
    let mut best = 0;
    let mut bd = f64::INFINITY;
    for (c, &q) in centers.iter().enumerate() {
        let d = dist2(p, q);
        if d < bd {
            bd = d;
            best = c;
        }
    }
    best
}

/// k-means: farthest-first seeds from the place nearest the mean, then
/// Lloyd's rounds until the assignments hold or `MOST_ROUNDS`.
pub fn k_means(shapes: &[Shape], k: usize) -> Result<StatsRun, String> {
    let placed = Placed::of(shapes, |_| true);
    let mut run = StatsRun::default();
    run.warn_if(placed.unplaced, unplaced_note);
    let pts = &placed.pts;
    let n = pts.len();
    if n == 0 {
        return Err("Yeri bulunan nesne yok.".into());
    }
    let mut distinct: Vec<(f64, f64)> = pts.iter().map(|p| (p.x, p.y)).collect();
    distinct.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
    distinct.dedup();
    if distinct.len() < k {
        return Err(format!(
            "Farklı yer sayısı ({}) küme sayısından ({k}) az.",
            distinct.len()
        ));
    }
    let nf = n as f64;
    let (mut mx, mut my) = (0.0, 0.0);
    for p in pts {
        mx += p.x;
        my += p.y;
    }
    let mean = Vec2::new(mx / nf, my / nf);
    let first = nearest_center(mean, pts);
    let mut centers = vec![pts[first]];
    let mut least: Vec<f64> = pts.iter().map(|&p| dist2(p, pts[first])).collect();
    while centers.len() < k {
        let mut far = 0;
        for i in 1..n {
            if least[i] > least[far] {
                far = i;
            }
        }
        let c = pts[far];
        centers.push(c);
        for (i, &p) in pts.iter().enumerate() {
            let d = dist2(p, c);
            if d < least[i] {
                least[i] = d;
            }
        }
    }
    let mut labels: Vec<usize> = pts.iter().map(|&p| nearest_center(p, &centers)).collect();
    let mut rounds = 0;
    let mut settled = false;
    while rounds < MOST_ROUNDS {
        let mut sums = vec![(0.0, 0.0, 0usize); k];
        for (p, &l) in pts.iter().zip(&labels) {
            sums[l].0 += p.x;
            sums[l].1 += p.y;
            sums[l].2 += 1;
        }
        for (c, &(sx, sy, m)) in centers.iter_mut().zip(&sums) {
            if m > 0 {
                *c = Vec2::new(sx / m as f64, sy / m as f64);
            }
        }
        rounds += 1;
        let mut changed = false;
        for (p, l) in pts.iter().zip(labels.iter_mut()) {
            let c = nearest_center(*p, &centers);
            if c != *l {
                *l = c;
                changed = true;
            }
        }
        if !changed {
            settled = true;
            break;
        }
    }
    if !settled {
        run.warnings.push(format!(
            "Atamalar {MOST_ROUNDS} yinelemede durulmadı; son atamalar yazıldı."
        ));
    }
    let numbered: Vec<u32> = labels.iter().map(|&l| l as u32 + 1).collect();
    let sizes = copies(&mut run, &placed, &numbered);
    let clusters = sizes.iter().skip(1).filter(|&&s| s > 0).count();
    run.numbers = vec![Number {
        name: "clusters",
        value: clusters as f64,
    }];
    run.summary = format!("{n} nesne {clusters} kümeye ayrıldı ({rounds} yineleme).");
    Ok(run)
}
