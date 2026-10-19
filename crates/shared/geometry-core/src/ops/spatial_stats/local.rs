//! Yerel istatistikler (docs/adr/0520 §4, §6): Getis-Ord Gi\* (ADR 0238 §9)
//! with its p values by conditional permutation and corrected for multiple
//! tests too, and Anselin's local Moran (LISA), by conditional permutation
//! only. An object without others among its neighbours, or whose permuted
//! statistics do not spread, has no permutation p and is not tested.

use super::autocorrelation::{
    HOT_COLORS, Neighbourhood, described, hot_class, profile_of, too_much_work, valued,
};
use super::inference::{
    Correction, Inference, MOST_DRAWS, MOST_WORK, TIE, adjust, conditional_rows, folded_p, moments,
    sim_z,
};
use super::weights::neighbours;
use super::{Number, ObjectCopy, StatsRun, p_value, pair, text};
use crate::entity::Shape;
use crate::jsmath::js_max;
use crate::ops::sampling::random::Sequence;

/// The conditional draws of a run (`inference::conditional_rows`).
pub struct Draws {
    rows: Vec<u32>,
    kmax: usize,
    r: usize,
}

impl Draws {
    /// The draws for `n` objects whose others among their neighbours number `ks`.
    pub fn new(n: usize, ks: &[usize], inference: Inference) -> Result<Draws, String> {
        let r = inference.permutations;
        let kmax = ks
            .iter()
            .copied()
            .max()
            .unwrap_or(0)
            .min(n.saturating_sub(1));
        let work: usize = ks.iter().sum::<usize>().saturating_mul(r);
        if work > MOST_WORK {
            return Err(too_much_work());
        }
        if r.saturating_mul(kmax) > MOST_DRAWS {
            return Err(format!(
                "Permütasyon tablosu çok büyük: permütasyon sayısı ile en kalabalık komşuluk çarpımı {MOST_DRAWS}'u aşıyor; permütasyon sayısını ya da komşuluğu küçültün."
            ));
        }
        let mut seq = Sequence::new(inference.seed);
        Ok(Draws {
            rows: conditional_rows(n, kmax, r, &mut seq),
            kmax,
            r,
        })
    }

    /// Object `i`'s pseudo p and pseudo z: its statistic `base + factor · Σ wₜ·v[oₜ]` over its `others` (in their
    /// order, the drawn objects in the row's), against the observed; none when it has no others or the permuted
    /// statistics do not spread. `vmax`: the largest |v|; `sims` a buffer.
    #[allow(clippy::too_many_arguments)]
    pub fn test(
        &self,
        i: usize,
        others: &[(u32, f64)],
        v: &[f64],
        base: f64,
        factor: f64,
        vmax: f64,
        sims: &mut Vec<f64>,
    ) -> Option<(f64, f64)> {
        let k = others.len();
        if k == 0 || self.r == 0 {
            return None;
        }
        let mut sum = 0.0;
        let mut weights = 0.0;
        for &(j, w) in others {
            sum += w * v[j as usize];
            weights += w.abs();
        }
        let observed = base + factor * sum;
        let bound = base.abs() + factor.abs() * weights * vmax;
        sims.clear();
        for row in self.rows.chunks_exact(self.kmax.max(1)).take(self.r) {
            let mut s = 0.0;
            for (t, &(_, w)) in others.iter().enumerate() {
                let o = row[t] as usize;
                let o = if o >= i { o + 1 } else { o };
                s += w * v[o];
            }
            sims.push(base + factor * s);
        }
        let (mean, sd) = moments(sims);
        if sd <= TIE * bound {
            return None;
        }
        let larger = sims
            .iter()
            .filter(|&&s| s >= observed - TIE * bound)
            .count();
        Some((folded_p(larger, self.r), sim_z(observed, mean, sd)?))
    }
}

/// The largest |v|.
fn largest(v: &[f64]) -> f64 {
    v.iter().fold(0.0f64, |a, x| js_max(a, x.abs()))
}

/// What the classes come from, the info's words.
fn basis(inference: Inference) -> String {
    let mut s = if inference.permuted() {
        format!(
            "{} koşullu permütasyonun p'sinden (tohum {})",
            inference.permutations, inference.seed
        )
    } else {
        "normal dağılımın p'sinden".to_owned()
    };
    if inference.correction != Correction::None {
        s.push_str(&format!(", {} düzeltmesiyle", inference.correction.title()));
    }
    s
}

/// An optional number's text.
fn opt(v: Option<f64>, d: u32) -> String {
    v.map_or_else(String::new, |v| text(v, d))
}

/// Getis-Ord Gi\* of every object with a place and a value (ADR 0238 §9),
/// with permutation p values and a correction when `inference` asks.
pub fn hot_spots_with(
    shapes: &[Shape],
    values: &[Option<String>],
    field: &str,
    hood: Neighbourhood,
    inference: Inference,
) -> Result<StatsRun, String> {
    let inference = inference.checked()?;
    let mut run = StatsRun::default();
    let (placed, xs) = valued(shapes, values, field, &mut run);
    let n = xs.len();
    if n < 3 {
        return Err(format!(
            "Sıcak nokta en az üç nesne ister; yeri ve değeri olan {n} nesne var."
        ));
    }
    let nf = n as f64;
    let mean = xs.iter().sum::<f64>() / nf;
    let dev: Vec<f64> = xs.iter().map(|x| x - mean).collect();
    let s = (dev.iter().map(|d| d * d).sum::<f64>() / nf).sqrt();
    if s.is_nan() || s <= 0.0 {
        return Err("Değerlerin hepsi aynı; sıcak nokta aranamaz.".into());
    }
    let nb = neighbours(&placed.pts, hood.concept, hood.band, hood.k, true)?;
    let draws = if inference.permuted() {
        let ks: Vec<usize> = nb.lists.iter().map(|l| l.len().saturating_sub(1)).collect();
        Some(Draws::new(n, &ks, inference)?)
    } else {
        None
    };
    let vmax = largest(&dev);
    let mut sims = Vec::new();
    let mut others = Vec::new();
    // Per object: the analytic z and p, the permutation's p and z.
    let mut analytic = Vec::with_capacity(n);
    let mut permuted = Vec::with_capacity(n);
    let mut undefined = 0usize;
    for (k, list) in nb.lists.iter().enumerate() {
        let (mut sw, mut sw2, mut num) = (0.0, 0.0, 0.0);
        for &(j, w) in list {
            sw += w;
            sw2 += w * w;
            num += w * dev[j as usize];
        }
        let den = s * ((nf * sw2 - sw * sw) / (nf - 1.0)).sqrt();
        analytic.push(if den > 0.0 && den.is_finite() {
            let z = num / den;
            Some((z, p_value(z)))
        } else {
            undefined += 1;
            None
        });
        permuted.push(draws.as_ref().and_then(|d| {
            others.clear();
            let mut own = 0.0;
            for &(j, w) in list {
                if j as usize == k {
                    own = w;
                } else {
                    others.push((j, w));
                }
            }
            d.test(k, &others, &dev, own * dev[k], 1.0, vmax, &mut sims)
        }));
    }
    let chosen: Vec<Option<f64>> = if inference.permuted() {
        permuted.iter().map(|t| t.map(|(p, _)| p)).collect()
    } else {
        analytic.iter().map(|t| t.map(|(_, p)| p)).collect()
    };
    let finals = adjust(&chosen, inference.correction);
    let mut counts = [0usize; 7];
    let mut untested = 0usize;
    for k in 0..n {
        let sign = analytic[k].map(|(z, _)| z).or(permuted[k].map(|(_, z)| z));
        let class = match (sign, finals[k]) {
            (Some(z), Some(p)) => hot_class(z, p),
            _ => 0,
        };
        if inference.permuted() && permuted[k].is_none() {
            untested += 1;
        }
        let mut attrs = vec![
            pair("z puanı", opt(analytic[k].map(|(z, _)| z), 4)),
            pair("p değeri", opt(analytic[k].map(|(_, p)| p), 6)),
        ];
        if inference.permuted() {
            attrs.push(pair("Permütasyon z", opt(permuted[k].map(|(_, z)| z), 4)));
            attrs.push(pair("Permütasyon p", opt(permuted[k].map(|(p, _)| p), 6)));
        }
        if inference.correction != Correction::None {
            attrs.push(pair("Düzeltilmiş p", opt(finals[k], 6)));
        }
        attrs.push(pair("Güven sınıfı", class.to_string()));
        counts[(class + 3) as usize] += 1;
        run.copies.push(ObjectCopy {
            index: placed.index[k],
            attrs,
            color: HOT_COLORS[(class + 3) as usize],
        });
    }
    run.warn_if(undefined, |n| {
        format!("{n} nesnenin bütün nesneler komşusu; z'si hesaplanamadı.")
    });
    run.warn_if(untested, |n| {
        format!(
            "{n} nesnenin permütasyon p'si yok (kendisinden başka komşusu yok ya da permütasyonları yayılmıyor); sınıfı 0."
        )
    });
    let hot = counts[4] + counts[5] + counts[6];
    let cold = counts[0] + counts[1] + counts[2];
    run.infos.push(format!(
        "Sıcak: %99 {}, %95 {}, %90 {}; soğuk: %99 {}, %95 {}, %90 {}.",
        counts[6], counts[5], counts[4], counts[0], counts[1], counts[2]
    ));
    if inference.profiled() {
        run.infos
            .push(format!("Güven sınıfları {}.", basis(inference)));
    }
    run.numbers = vec![
        Number {
            name: "hot",
            value: hot as f64,
        },
        Number {
            name: "cold",
            value: cold as f64,
        },
    ];
    run.summary =
        format!("{n} nesne yazıldı: {hot} sıcak, {cold} soğuk nokta (%90 ve üstü güvenle).");
    if inference.profiled() {
        run.profile = Some(profile_of(
            "Sıcak nokta (Gi*)",
            inference,
            true,
            described(&nb, hood.concept),
            None,
            field,
            shapes,
            &placed,
            &xs,
        ));
    }
    Ok(run)
}

/// The local Moran's cluster types: their codes, names and colours (§6);
/// the last two are “not significant” and “not tested”.
pub const LISA_TYPES: [(&str, &str, &str); 6] = [
    ("YY", "Yüksek-Yüksek", "#D7191C"),
    ("DD", "Düşük-Düşük", "#2C7BB6"),
    ("YD", "Yüksek-Düşük", "#FDAE61"),
    ("DY", "Düşük-Yüksek", "#ABD9E9"),
    ("", "Anlamlı değil", "#D9D9D9"),
    ("", "Sınanamadı", "#8C8C8C"),
];

/// Anselin's local Moran of every object with a place and a value (§6):
/// Iᵢ = (zᵢ / m₂) Σⱼ wᵢⱼ zⱼ, m₂ = Σ z² / n; its quadrant, its pseudo p by
/// conditional permutation (corrected when asked) and its cluster type at
/// `alpha`.
pub fn local_morans(
    shapes: &[Shape],
    values: &[Option<String>],
    field: &str,
    hood: Neighbourhood,
    standardize: bool,
    inference: Inference,
    alpha: f64,
) -> Result<StatsRun, String> {
    let inference = inference.checked()?;
    if !inference.permuted() {
        return Err("Yerel Moran permütasyonla sınanır; permütasyon sayısı en az 9 olmalı.".into());
    }
    let mut run = StatsRun::default();
    let (placed, xs) = valued(shapes, values, field, &mut run);
    let n = xs.len();
    if n < 4 {
        return Err(format!(
            "Yerel Moran en az dört nesne ister; yeri ve değeri olan {n} nesne var."
        ));
    }
    let nf = n as f64;
    let mean = xs.iter().sum::<f64>() / nf;
    let z: Vec<f64> = xs.iter().map(|x| x - mean).collect();
    let m2 = z.iter().map(|v| v * v).sum::<f64>() / nf;
    if m2.is_nan() || m2 <= 0.0 {
        return Err("Değerlerin hepsi aynı; yerel Moran hesaplanamaz.".into());
    }
    let mut nb = neighbours(&placed.pts, hood.concept, hood.band, hood.k, false)?;
    let mut isolated = 0usize;
    for list in nb.lists.iter_mut() {
        let sum: f64 = list.iter().map(|&(_, w)| w).sum();
        if list.is_empty() || sum <= 0.0 {
            isolated += 1;
            list.clear();
        } else if standardize {
            for e in list.iter_mut() {
                e.1 /= sum;
            }
        }
    }
    let ks: Vec<usize> = nb.lists.iter().map(Vec::len).collect();
    let draws = Draws::new(n, &ks, inference)?;
    let zmax = largest(&z);
    let mut sims = Vec::new();
    let mut local = Vec::with_capacity(n);
    let mut tests = Vec::with_capacity(n);
    for (i, list) in nb.lists.iter().enumerate() {
        let lag: f64 = list.iter().map(|&(j, w)| w * z[j as usize]).sum();
        let factor = z[i] / m2;
        local.push((factor * lag, lag));
        tests.push(draws.test(i, list, &z, 0.0, factor, zmax, &mut sims));
    }
    let chosen: Vec<Option<f64>> = tests.iter().map(|t| t.map(|(p, _)| p)).collect();
    let finals = adjust(&chosen, inference.correction);
    let mut counts = [0usize; 6];
    for i in 0..n {
        let (value, lag) = local[i];
        let quadrant = if nb.lists[i].is_empty() {
            None
        } else {
            Some(match (z[i] >= 0.0, lag >= 0.0) {
                (true, true) => 0,
                (false, false) => 1,
                (true, false) => 2,
                (false, true) => 3,
            })
        };
        let kind = match (quadrant, finals[i]) {
            (Some(q), Some(p)) if p < alpha => q,
            (Some(_), Some(_)) => 4,
            _ => 5,
        };
        counts[kind] += 1;
        let mut attrs = vec![
            pair(
                "Yerel Moran I",
                if quadrant.is_some() {
                    text(value, 6)
                } else {
                    String::new()
                },
            ),
            pair("Çeyrek", quadrant.map_or("", |q| LISA_TYPES[q].0)),
            pair("Permütasyon z", opt(tests[i].map(|(_, z)| z), 4)),
            pair("Permütasyon p", opt(tests[i].map(|(p, _)| p), 6)),
        ];
        if inference.correction != Correction::None {
            attrs.push(pair("Düzeltilmiş p", opt(finals[i], 6)));
        }
        attrs.push(pair("Küme türü", LISA_TYPES[kind].1));
        run.copies.push(ObjectCopy {
            index: placed.index[i],
            attrs,
            color: LISA_TYPES[kind].2,
        });
    }
    run.warn_if(isolated, |n| {
        format!("{n} nesnenin komşusu yok; sınanamadı.")
    });
    run.warn_if(counts[5] - isolated, |n| {
        format!("{n} nesnenin permütasyonları yayılmıyor; sınanamadı.")
    });
    let alpha_text = text(alpha, 2);
    run.infos.push(format!(
        "Küme türleri p < {alpha_text} ile, {}.",
        basis(inference)
    ));
    run.numbers = ["highHigh", "lowLow", "highLow", "lowHigh"]
        .iter()
        .zip(counts)
        .map(|(&name, c)| Number {
            name,
            value: c as f64,
        })
        .collect();
    run.summary = format!(
        "{n} nesne yazıldı: {} Yüksek-Yüksek, {} Düşük-Düşük, {} Yüksek-Düşük, {} Düşük-Yüksek (p < {alpha_text}).",
        counts[0], counts[1], counts[2], counts[3]
    );
    run.profile = Some(profile_of(
        "Yerel Moran",
        inference,
        true,
        described(&nb, hood.concept),
        Some(standardize),
        field,
        shapes,
        &placed,
        &xs,
    ));
    if let Some(p) = run.profile.as_mut() {
        p.push("alpha", "Anlamlılık düzeyi", alpha_text);
    }
    Ok(run)
}
