//! Denetimsiz sınıflandırma (docs/adr/0242 §7): k-means over a sample of the
//! cells. The sample is the cells with every band whose column and row are
//! multiples of s = ⌈√(cells / 250 000)⌉; the centres start along the bands'
//! mean ± standard deviation diagonal (ISODATA's usual start), each sample
//! goes to its nearest centre (on a tie the earlier), each centre becomes its
//! members' mean (an empty one stays), until no sample moves or the rounds
//! run out. The clusters are then numbered by their centres' band sums.
//!
//! The sums run over the sample in its order (row by row), so the centres
//! are the same floats on every target and with any threads; the samples'
//! nearest centres are worked out on the threads.

// Vectors and matrices by their bands' indices read as the formulas do.
#![allow(clippy::needless_range_loop)]

use crate::par;

/// The most cells the sample takes, about.
pub const SAMPLE_CELLS: f64 = 250_000.0;

/// The sample's step: every s-th column of every s-th row.
pub fn step(cells: u64) -> u32 {
    let s = (cells as f64 / SAMPLE_CELLS).sqrt().ceil();
    if s > 1.0 { s as u32 } else { 1 }
}

/// The nearest of `centers` (k × `b`, flat) to `x`; on a tie the earlier.
#[inline]
pub fn nearest(x: &[f64], centers: &[f64], b: usize) -> usize {
    let mut best = 0;
    let mut top = f64::INFINITY;
    for (c, ctr) in centers.chunks_exact(b).enumerate() {
        let mut s = 0.0;
        for a in 0..b {
            let e = x[a] - ctr[a];
            s += e * e;
        }
        if s < top {
            best = c;
            top = s;
        }
    }
    best
}

/// The clusters found.
#[derive(Clone, Debug, PartialEq)]
pub struct Clusters {
    /// The centres in the clusters' order (k × bands, flat).
    pub centers: Vec<f64>,
    pub rounds: u32,
    pub samples: usize,
}

/// k-means of `sample` (`b` bands a cell, flat) into `k` clusters, at most `iterations` rounds.
pub fn fit(
    sample: &[f64],
    b: usize,
    k: usize,
    iterations: u32,
    threads: usize,
) -> Result<Clusters, String> {
    let n = sample.len() / b.max(1);
    if n < k {
        return Err(format!(
            "Örnek hücre sayısı ({n}) küme sayısından ({k}) az."
        ));
    }
    let nf = n as f64;
    let mut centers = vec![0.0; k * b];
    for a in 0..b {
        let mut t = 0.0;
        for x in sample.chunks_exact(b) {
            t += x[a];
        }
        let mu = t / nf;
        let mut q = 0.0;
        for x in sample.chunks_exact(b) {
            let e = x[a] - mu;
            q += e * e;
        }
        let sd = (q / nf).sqrt();
        for c in 0..k {
            centers[c * b + a] = mu + sd * (2.0 * (c as f64 + 0.5) / k as f64 - 1.0);
        }
    }
    let label = |centers: &[f64], out: &mut Vec<u32>| {
        out.resize(n, 0);
        par::rows(threads, out, 1, &|first, chunk: &mut [u32]| {
            for (q, o) in chunk.iter_mut().enumerate() {
                let x = &sample[(first + q) * b..(first + q + 1) * b];
                *o = nearest(x, centers, b) as u32;
            }
        });
    };
    let mut labels = Vec::new();
    label(&centers, &mut labels);
    let mut next = Vec::new();
    let mut rounds = 0;
    while rounds < iterations {
        let mut sums = vec![0.0; k * b];
        let mut members = vec![0u64; k];
        for (x, &l) in sample.chunks_exact(b).zip(&labels) {
            let l = l as usize;
            for a in 0..b {
                sums[l * b + a] += x[a];
            }
            members[l] += 1;
        }
        for c in 0..k {
            if members[c] > 0 {
                for a in 0..b {
                    centers[c * b + a] = sums[c * b + a] / members[c] as f64;
                }
            }
        }
        rounds += 1;
        label(&centers, &mut next);
        if next == labels {
            break;
        }
        std::mem::swap(&mut labels, &mut next);
    }
    // Numbered by their band sums, rising (on a tie the earlier).
    let totals: Vec<f64> = (0..k)
        .map(|c| {
            let mut t = 0.0;
            for a in 0..b {
                t += centers[c * b + a];
            }
            t
        })
        .collect();
    let mut order: Vec<usize> = (0..k).collect();
    order.sort_by(|&p, &q| {
        totals[p]
            .partial_cmp(&totals[q])
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(p.cmp(&q))
    });
    let ranked = order
        .iter()
        .flat_map(|&c| centers[c * b..(c + 1) * b].iter().copied())
        .collect();
    Ok(Clusters {
        centers: ranked,
        rounds,
        samples: n,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps() {
        assert_eq!(step(120), 1);
        assert_eq!(step(250_000), 1);
        assert_eq!(step(250_001), 2);
        assert_eq!(step(4096 * 4096), 9);
    }

    #[test]
    fn two_groups_are_found_and_numbered_by_their_sums() {
        // Five cells near (10, 10) and five near (0, 0), one band each way.
        let mut s = Vec::new();
        for k in 0..5 {
            s.extend([10.0 + f64::from(k) * 0.1, 10.0]);
            s.extend([f64::from(k) * 0.1, 0.0]);
        }
        for threads in [1, 3] {
            let c = fit(&s, 2, 2, 20, threads).unwrap();
            assert_eq!(c.samples, 10);
            assert!((c.centers[0] - 0.2).abs() < 1e-12 && c.centers[1] == 0.0);
            assert!((c.centers[2] - 10.2).abs() < 1e-12 && c.centers[3] == 10.0);
        }
        assert!(fit(&s[..2], 2, 2, 5, 1).unwrap_err().contains("(1)"));
    }
}
