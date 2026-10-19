//! Permütasyon çıkarımı, çoklu test düzeltmesi ve deney profili
//! (docs/adr/0520): the rule `kentos.spatial-inference/1`.
//!
//! The numbers are the sampling tools' (`ops::sampling::random`, ADR 0220):
//! SplitMix64 spreads the seed, xoshiro256** gives the words; an index below
//! `m` is the word's high half times `m` (`below`). A run draws from one
//! sequence:
//!
//! - **Global** (Moran I): `R` permutations of the values, each a
//!   Fisher–Yates shuffle of `0 … n−1` from the identity (from the last place
//!   down to 1, `j = below(i + 1)`).
//! - **Conditional** (Gi\*, Yerel Moran): `R` rows of `kmax` draws without
//!   replacement from `0 … n−2`, each a partial Fisher–Yates from the
//!   identity (`j = t + below(n − 1 − t)`); every object reads the first of
//!   its `kᵢ` draws of each row, a draw `o` standing for object `o + (o ≥ i)`
//!   (esda's `crand`: the same rows for every object, the object itself
//!   held).
//!
//! A permuted statistic counts as at least the observed one when it is no
//! less than it minus `TIE` times the statistic's bound (so a tie in exact
//! arithmetic is one on every platform); the pseudo p is folded (esda):
//! `larger` such, `R − larger` when that is less, `(larger + 1) / (R + 1)`.

use crate::ops::sampling::random::Sequence;

/// The rule's name, written in the profile.
pub const RULE: &str = "kentos.spatial-inference/1";
/// The generator, written in the profile.
pub const GENERATOR: &str = "SplitMix64 + xoshiro256**";
/// The most permutations a run makes.
pub const MOST_PERMUTATIONS: usize = 99_999;
/// The most draws a conditional run holds (R × kmax).
pub const MOST_DRAWS: usize = 25_000_000;
/// The most products a run computes (R × the neighbour pairs).
pub const MOST_WORK: usize = 4_000_000_000;
/// A tie's width, relative to the statistic's bound.
pub const TIE: f64 = 1e-9;

/// The multiple testing correction of local statistics' p values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Correction {
    None,
    Fdr,
    Bonferroni,
}

impl Correction {
    pub fn from_key(key: &str) -> Option<Correction> {
        Some(match key {
            "none" | "" => Correction::None,
            "fdr" => Correction::Fdr,
            "bonferroni" => Correction::Bonferroni,
            _ => return None,
        })
    }

    /// The profile's words.
    pub fn title(self) -> &'static str {
        match self {
            Correction::None => "Yok",
            Correction::Fdr => "FDR (Benjamini–Hochberg)",
            Correction::Bonferroni => "Bonferroni",
        }
    }
}

/// How a run's p values are found: `permutations` 0 is the normal
/// distribution's; the seed of the numbers; the correction of local p values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Inference {
    pub permutations: usize,
    pub seed: u64,
    pub correction: Correction,
}

impl Inference {
    /// The normal distribution, no correction: the tools' default (ADR 0238).
    pub const ANALYTIC: Inference = Inference {
        permutations: 0,
        seed: 0,
        correction: Correction::None,
    };

    pub fn permuted(&self) -> bool {
        self.permutations > 0
    }

    /// Whether the run writes a profile: permutations or a correction.
    pub fn profiled(&self) -> bool {
        self.permuted() || self.correction != Correction::None
    }

    /// The refusal of a count out of range.
    pub fn checked(self) -> Result<Inference, String> {
        if self.permutations > MOST_PERMUTATIONS {
            return Err(format!(
                "Permütasyon sayısı en çok {MOST_PERMUTATIONS} olabilir; {} istendi.",
                self.permutations
            ));
        }
        if self.permuted() && self.permutations < 9 {
            return Err(format!(
                "Permütasyon sayısı en az 9 olmalı; {} istendi.",
                self.permutations
            ));
        }
        Ok(self)
    }
}

/// An index in `0 … m−1` (m ≥ 1): the word's high half times m (Lemire's
/// product, without the rejection: its bias is below m / 2⁶⁴).
pub fn below(seq: &mut Sequence, m: usize) -> usize {
    ((seq.next_u64() as u128 * m as u128) >> 64) as usize
}

/// The global permutations: `f` gets each, `perm[i]` the object whose value
/// object `i` takes.
pub fn each_permutation(n: usize, r: usize, seq: &mut Sequence, mut f: impl FnMut(&[u32])) {
    let mut perm: Vec<u32> = (0..n as u32).collect();
    for _ in 0..r {
        for (i, p) in perm.iter_mut().enumerate() {
            *p = i as u32;
        }
        for i in (1..n).rev() {
            let j = below(seq, i + 1);
            perm.swap(i, j);
        }
        f(&perm);
    }
}

/// The conditional draws: `r` rows of `kmax` indices in `0 … n−2`, each
/// row's distinct (row after row in one vector).
pub fn conditional_rows(n: usize, kmax: usize, r: usize, seq: &mut Sequence) -> Vec<u32> {
    let m = n.saturating_sub(1);
    let kmax = kmax.min(m);
    let mut out = Vec::with_capacity(r * kmax);
    let mut pool: Vec<u32> = (0..m as u32).collect();
    let mut swaps = Vec::with_capacity(kmax);
    for _ in 0..r {
        swaps.clear();
        for t in 0..kmax {
            let j = t + below(seq, m - t);
            pool.swap(t, j);
            swaps.push(j);
            out.push(pool[t]);
        }
        // Back to the identity: the swaps undone in reverse.
        for (t, &j) in swaps.iter().enumerate().rev() {
            pool.swap(t, j);
        }
    }
    out
}

/// The folded pseudo p of `larger` of `r` permuted statistics at least the observed one.
pub fn folded_p(larger: usize, r: usize) -> f64 {
    let l = if r - larger < larger {
        r - larger
    } else {
        larger
    };
    (l + 1) as f64 / (r + 1) as f64
}

/// The permuted statistics' mean and (population) standard deviation.
pub fn moments(sims: &[f64]) -> (f64, f64) {
    let r = sims.len() as f64;
    let mean = sims.iter().sum::<f64>() / r;
    let var = sims.iter().map(|s| (s - mean) * (s - mean)).sum::<f64>() / r;
    (mean, var.sqrt())
}

/// A pseudo-z: the observed statistic's distance from the permuted ones' mean in their standard deviations; none
/// when they do not spread.
pub fn sim_z(observed: f64, mean: f64, sd: f64) -> Option<f64> {
    (sd > 0.0 && sd.is_finite()).then(|| (observed - mean) / sd)
}

/// The p values corrected for `m` tests, `m` the defined ones: Bonferroni
/// `min(1, p·m)`; Benjamini–Hochberg's adjusted p, the k-th smallest's
/// `min(1, min over j ≥ k of p₍ⱼ₎·m / j)` (ties in their order).
pub fn adjust(ps: &[Option<f64>], c: Correction) -> Vec<Option<f64>> {
    let m = ps.iter().filter(|p| p.is_some()).count() as f64;
    match c {
        Correction::None => ps.to_vec(),
        Correction::Bonferroni => ps
            .iter()
            .map(|p| p.map(|p| crate::jsmath::js_min(1.0, p * m)))
            .collect(),
        Correction::Fdr => {
            let mut order: Vec<(f64, usize)> = ps
                .iter()
                .enumerate()
                .filter_map(|(i, p)| p.map(|p| (p, i)))
                .collect();
            order.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
            let mut out = vec![None; ps.len()];
            let mut least = 1.0f64;
            for (rank, &(p, i)) in order.iter().enumerate().rev() {
                let q = p * m / (rank + 1) as f64;
                if q < least {
                    least = q;
                }
                out[i] = Some(least);
            }
            out
        }
    }
}

/// A run's reproducible profile (§5): its entries (key, label, text) and the
/// canonical text of what it analysed, whose SHA-256 the host writes.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Profile {
    pub keys: Vec<String>,
    pub labels: Vec<String>,
    pub values: Vec<String>,
    pub inputs: String,
}

impl Profile {
    pub fn push(&mut self, key: &str, label: &str, value: impl Into<String>) {
        self.keys.push(key.to_owned());
        self.labels.push(label.to_owned());
        self.values.push(value.into());
    }

    /// The entry's text by key.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.keys
            .iter()
            .position(|k| k == key)
            .map(|i| self.values[i].as_str())
    }
}

/// The canonical line of an analysed object: its place in the input, its
/// place at 6 fraction digits and its value as the shortest double text.
pub fn input_line(out: &mut String, index: usize, x: f64, y: f64, value: f64) {
    use std::fmt::Write;
    let _ = writeln!(
        out,
        "{index}\t{}\t{}\t{}",
        super::text(x, 6),
        super::text(y, 6),
        value_text(value)
    );
}

/// A double's shortest text, no exponent (Rust's `Display`), no minus on zero.
pub fn value_text(v: f64) -> String {
    if v == 0.0 {
        "0".to_owned()
    } else {
        format!("{v}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_folded_p_takes_the_smaller_side() {
        assert_eq!(folded_p(0, 999), 0.001);
        assert_eq!(folded_p(999, 999), 0.001);
        assert_eq!(folded_p(500, 999), 0.5);
        assert_eq!(folded_p(499, 999), 0.5);
    }

    #[test]
    fn benjamini_hochberg_as_the_paper_works_it() {
        // Benjamini and Hochberg (1995) §4: 15 p values; at q = 0.05 the four smallest are rejected.
        let ps = [
            0.0001, 0.0004, 0.0019, 0.0095, 0.0201, 0.0278, 0.0298, 0.0344, 0.0459, 0.3240, 0.4262,
            0.5719, 0.6528, 0.7590, 1.0,
        ];
        let q = adjust(
            &ps.iter().map(|&p| Some(p)).collect::<Vec<_>>(),
            Correction::Fdr,
        );
        let rejected = q.iter().filter(|q| q.unwrap() <= 0.05).count();
        assert_eq!(rejected, 4);
        assert!((q[0].unwrap() - 0.0015).abs() < 1e-15);
        assert_eq!(q[14], Some(1.0));
        let b = adjust(&[Some(0.01), None, Some(0.6)], Correction::Bonferroni);
        assert_eq!(b, vec![Some(0.02), None, Some(1.0)]);
    }

    #[test]
    fn the_draws_are_permutations_and_distinct() {
        let mut seq = Sequence::new(7);
        each_permutation(10, 5, &mut seq, |p| {
            let mut s = p.to_vec();
            s.sort_unstable();
            assert_eq!(s, (0..10).collect::<Vec<u32>>());
        });
        let rows = conditional_rows(10, 4, 50, &mut seq);
        assert_eq!(rows.len(), 200);
        for row in rows.chunks(4) {
            let mut s = row.to_vec();
            s.sort_unstable();
            s.dedup();
            assert_eq!(s.len(), 4);
            assert!(s.iter().all(|&o| o < 9));
        }
    }

    #[test]
    fn splitmix_and_xoshiro_give_their_published_words() {
        // SplitMix64 from 0 gives 0xE220A8397B1DCDAF first (Vigna's splitmix64.c); the sequence starts from it.
        let mut seq = Sequence::new(0);
        let first = seq.next_u64();
        // xoshiro256**'s first word from the four SplitMix64 words of seed 0 (scripts/fixtures/spatial_inference_cases.py).
        assert_eq!(first, 0x99EC_5F36_CB75_F2B4);
    }
}
