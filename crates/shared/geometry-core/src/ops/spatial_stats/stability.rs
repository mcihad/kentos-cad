//! Kümelerin kararlılığı (docs/adr/0566 §2; Hennig 2007, R's
//! `fpc::clusterboot`): the places are resampled and clustered again; a
//! cluster's stability is the mean, over the resamples it shows in, of its
//! best Jaccard similarity with a new cluster.
//!
//! - **Resample** (one sequence, `inference::below`): Bootstrap, `n` draws
//!   with replacement, each object once however often it is drawn (`fpc`'s
//!   `unique(bsamp)`, `multipleboot = FALSE`; docs/adr/0639 §1); Alt örnek,
//!   `⌊n/2⌋` without, a partial Fisher–Yates from the identity
//!   (`j = t + below(n − t)`). The positions are sorted, so the clustering
//!   visits the places in their own order.
//! - **Similarity**: a cluster C's part in the resample is its objects drawn;
//!   a new cluster D, the objects it labels (noise is no cluster).
//!   J = |C ∩ D| / (|C| + |D| − |C ∩ D|), one float64 division; γ the largest
//!   J, 0 when no D meets C, and 0 for a resample without C's objects (`fpc`'s
//!   `clujaccard(…, zerobyzero = 0)`): every resample counts.
//! - **Stability**: the γs' sum over the resamples, summed in their order;
//!   recovered when γ > 0.75, dissolved when γ ≤ 0.5.

use super::inference::{GENERATOR, Profile, RULE, below};
use super::nearest::place_line;
use super::{Number, Placed, StatsRun, Table, place, text};
use crate::entity::Shape;
use crate::ops::sampling::random::Sequence;
use crate::vec2::Vec2;

/// The most resampled places a run clusters (resamples × objects).
pub const MOST_RESAMPLED: usize = 20_000_000;
/// The resamples' range.
pub const FEWEST_RESAMPLES: usize = 10;
pub const MOST_RESAMPLES: usize = 999;

/// How the places are resampled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scheme {
    Bootstrap,
    Subset,
}

impl Scheme {
    pub fn from_key(key: &str) -> Option<Scheme> {
        Some(match key {
            "bootstrap" => Scheme::Bootstrap,
            "subset" => Scheme::Subset,
            _ => return None,
        })
    }
}

/// A stability test: the scheme, the resamples (0: none) and the seed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stability {
    pub scheme: Scheme,
    pub resamples: usize,
    pub seed: u64,
}

impl Stability {
    pub const NONE: Stability = Stability {
        scheme: Scheme::Bootstrap,
        resamples: 0,
        seed: 0,
    };

    /// The refusal of a test out of range.
    pub fn checked(self, objects: usize) -> Result<Stability, String> {
        if self.resamples == 0 {
            return Ok(self);
        }
        if !(FEWEST_RESAMPLES..=MOST_RESAMPLES).contains(&self.resamples) {
            return Err(format!(
                "Tekrar sayısı {FEWEST_RESAMPLES} ile {MOST_RESAMPLES} arasında olmalı; {} istendi.",
                self.resamples
            ));
        }
        if self.scheme == Scheme::Subset && objects < 4 {
            return Err(format!(
                "Alt örnek en az dört nesne ister; {objects} nesne var."
            ));
        }
        if self.resamples.saturating_mul(objects) > MOST_RESAMPLED {
            return Err(format!(
                "{} tekrar × {objects} nesne en çok {MOST_RESAMPLED} yeniden kümelenen yer eder; tekrar sayısını azaltın.",
                self.resamples
            ));
        }
        Ok(self)
    }

    /// The profile's words.
    pub fn words(self) -> &'static str {
        match self.scheme {
            Scheme::Bootstrap => "Bootstrap (yerine koyarak)",
            Scheme::Subset => "Alt örnek (yarısı, yerine koymadan)",
        }
    }

    /// The summary's noun.
    fn noun(self) -> &'static str {
        match self.scheme {
            Scheme::Bootstrap => "bootstrap tekrarı",
            Scheme::Subset => "alt örnek tekrarı",
        }
    }
}

/// What the resamples' clusterings said: k-means with fewer distinct places than k, unsettled rounds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Notes {
    pub fewer: usize,
    pub unsettled: usize,
}

/// A cluster's tally over the resamples.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ClusterStability {
    pub size: usize,
    pub sum: f64,
    pub counted: usize,
    pub recovered: usize,
    pub dissolved: usize,
}

impl ClusterStability {
    /// The stability: the γs' mean; none when no resample counted.
    pub fn mean(&self) -> Option<f64> {
        (self.counted > 0).then(|| self.sum / self.counted as f64)
    }
}

/// The class of a stability (Hennig's thresholds).
pub fn class_of(g: Option<f64>) -> &'static str {
    match g {
        None => "Sınanamadı",
        Some(g) if g >= 0.85 => "Çok kararlı",
        Some(g) if g >= 0.75 => "Kararlı",
        Some(g) if g >= 0.6 => "Belirsiz",
        Some(_) => "Güvenilmez",
    }
}

/// A resample's positions among `0 … n−1`, sorted, each once.
pub fn resample(n: usize, scheme: Scheme, seq: &mut Sequence) -> Vec<u32> {
    let mut idx: Vec<u32> = match scheme {
        Scheme::Bootstrap => (0..n).map(|_| below(seq, n) as u32).collect(),
        Scheme::Subset => {
            let m = n / 2;
            let mut pool: Vec<u32> = (0..n as u32).collect();
            for t in 0..m {
                let j = t + below(seq, n - t);
                pool.swap(t, j);
            }
            pool.truncate(m);
            pool
        }
    };
    idx.sort_unstable();
    idx.dedup();
    idx
}

/// A clustering of the resampled places: its labels (0: noise), whether it worked with fewer clusters, whether its
/// rounds did not settle.
pub type Clustered = (Vec<u32>, bool, bool);

/// The tally of the original clusters (by label, 1 … the largest) over the resamples.
pub fn stable_clusters(
    placed: &Placed,
    labels: &[u32],
    stability: Stability,
    mut cluster: impl FnMut(&[Vec2]) -> Result<Clustered, String>,
    notes: &mut Notes,
) -> Result<Vec<ClusterStability>, String> {
    let n = placed.pts.len();
    let k = labels.iter().copied().max().unwrap_or(0) as usize;
    let mut out = vec![ClusterStability::default(); k + 1];
    for &l in labels {
        out[l as usize].size += 1;
    }
    const UNSEEN: u32 = u32::MAX;
    let mut seq = Sequence::new(stability.seed);
    let mut seen = vec![UNSEEN; n];
    let mut sample = Vec::with_capacity(n);
    let mut pairs: Vec<(u32, u32)> = Vec::with_capacity(n);
    for _ in 0..stability.resamples {
        let idx = resample(n, stability.scheme, &mut seq);
        sample.clear();
        sample.extend(idx.iter().map(|&i| placed.pts[i as usize]));
        let (boot, fewer, unsettled) = cluster(&sample)?;
        notes.fewer += usize::from(fewer);
        notes.unsettled += usize::from(unsettled);
        seen.iter_mut().for_each(|s| *s = UNSEEN);
        for (t, &i) in idx.iter().enumerate() {
            if seen[i as usize] == UNSEEN {
                seen[i as usize] = boot[t];
            }
        }
        let kb = boot.iter().copied().max().unwrap_or(0) as usize;
        let mut part = vec![0usize; k + 1];
        let mut dsize = vec![0usize; kb + 1];
        pairs.clear();
        for (i, &d) in seen.iter().enumerate() {
            if d == UNSEEN {
                continue;
            }
            let c = labels[i];
            if c > 0 {
                part[c as usize] += 1;
            }
            if d > 0 {
                dsize[d as usize] += 1;
            }
            if c > 0 && d > 0 {
                pairs.push((c, d));
            }
        }
        pairs.sort_unstable();
        let mut best = vec![0.0f64; k + 1];
        let mut at = 0;
        while at < pairs.len() {
            let (c, d) = pairs[at];
            let mut end = at;
            while end < pairs.len() && pairs[end] == (c, d) {
                end += 1;
            }
            let both = end - at;
            let j = both as f64 / (part[c as usize] + dsize[d as usize] - both) as f64;
            if j > best[c as usize] {
                best[c as usize] = j;
            }
            at = end;
        }
        for c in 1..=k {
            // A cluster none of whose objects is drawn counts with γ = 0 (`fpc`'s `bootmean`).
            let g = if part[c] == 0 { 0.0 } else { best[c] };
            let s = &mut out[c];
            s.sum += g;
            s.counted += 1;
            if g > 0.75 {
                s.recovered += 1;
            }
            if g <= 0.5 {
                s.dissolved += 1;
            }
        }
    }
    Ok(out)
}

/// The stability's table, copies' attributes, summary, notes and profile on the run (docs/adr/0566 §2, §3).
#[allow(clippy::too_many_arguments)]
pub fn stability_result(
    run: &mut StatsRun,
    shapes: &[Shape],
    placed: &Placed,
    labels: &[u32],
    stability: Stability,
    found: &[ClusterStability],
    tool: &str,
    parameters: String,
    notes: Notes,
) {
    let means: Vec<Option<f64>> = found.iter().map(ClusterStability::mean).collect();
    for (copy, &l) in run.copies.iter_mut().zip(labels) {
        let (g, class) = if l == 0 {
            (String::new(), String::new())
        } else {
            let g = means[l as usize];
            (
                g.map_or_else(String::new, |g| text(g, 3)),
                class_of(g).to_owned(),
            )
        };
        copy.attrs.push(["Kararlılık".to_owned(), g]);
        copy.attrs.push(["Kararlılık sınıfı".to_owned(), class]);
    }
    let mut rows = Vec::new();
    let mut clusters = 0usize;
    let mut stable = 0usize;
    for (c, s) in found.iter().enumerate().skip(1) {
        if s.size == 0 {
            continue;
        }
        clusters += 1;
        let g = means[c];
        if g.is_some_and(|g| g >= 0.75) {
            stable += 1;
        }
        rows.push(vec![
            c.to_string(),
            s.size.to_string(),
            g.map_or_else(|| "—".to_owned(), |g| text(g, 3)),
            class_of(g).to_owned(),
            s.recovered.to_string(),
            s.dissolved.to_string(),
            s.counted.to_string(),
        ]);
    }
    run.table = Some(Table {
        columns: [
            "Küme",
            "Nesne sayısı",
            "Kararlılık",
            "Sınıf",
            "Yeniden bulunma",
            "Çözülme",
            "Sayılan tekrar",
        ]
        .iter()
        .map(|c| (*c).to_owned())
        .collect(),
        rows,
    });
    run.numbers.push(Number {
        name: "stable",
        value: stable as f64,
    });
    run.summary.push_str(&format!(
        " Kararlı küme: {stable} / {clusters} (Jaccard ≥ 0.75; {} {}, tohum {}).",
        stability.resamples,
        stability.noun(),
        stability.seed
    ));
    run.infos.push(
        "Kararlılık: her kümenin tekrarlarda en çok benzediği yeni kümeyle Jaccard benzerliğinin ortalaması (Hennig 2007); \
         0.85 ve üstü çok kararlı, 0.75 ve üstü kararlı, 0.6'nın altı güvenilmez."
            .to_owned(),
    );
    if notes.fewer > 0 {
        run.warnings.push(format!(
            "{} tekrarda örneğin farklı yer sayısı küme sayısından az; o tekrarlar daha az kümeyle kümelendi.",
            notes.fewer
        ));
    }
    if notes.unsettled > 0 {
        run.warnings.push(format!(
            "{} tekrarda atamalar yinelemelerin sınırında durulmadı; son atamalarla sayıldı.",
            notes.unsettled
        ));
    }
    let mut profile = Profile::default();
    profile.push("rule", "Kural", RULE);
    profile.push("tool", "Araç", tool);
    profile.push("inference", "Çıkarım", stability.words());
    profile.push(
        "resamples",
        "Tekrar sayısı",
        stability.resamples.to_string(),
    );
    profile.push("seed", "Tohum", stability.seed.to_string());
    profile.push("generator", "Sayı üreteci", GENERATOR);
    profile.push("parameters", "Parametreler", parameters);
    profile.push("objects", "Nesne sayısı", placed.pts.len().to_string());
    for &i in &placed.index {
        let at = place(&shapes[i]).unwrap_or_default();
        place_line(&mut profile.inputs, i, at.x, at.y);
    }
    run.profile = Some(profile);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_subset_is_half_the_places_once_each_and_sorted() {
        let mut seq = Sequence::new(3);
        let idx = resample(11, Scheme::Subset, &mut seq);
        assert_eq!(idx.len(), 5);
        assert!(idx.windows(2).all(|w| w[0] < w[1]));
        // A bootstrap draws 11 with replacement and keeps each object once (fpc's unique(bsamp)).
        let boot = resample(11, Scheme::Bootstrap, &mut seq);
        assert!(!boot.is_empty() && boot.len() < 11);
        assert!(boot.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn the_classes_follow_hennig() {
        assert_eq!(class_of(Some(0.9)), "Çok kararlı");
        assert_eq!(class_of(Some(0.75)), "Kararlı");
        assert_eq!(class_of(Some(0.6)), "Belirsiz");
        assert_eq!(class_of(Some(0.59)), "Güvenilmez");
        assert_eq!(class_of(None), "Sınanamadı");
    }
}
