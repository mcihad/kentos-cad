//! Mekânsal otokorelasyon (Moran I) ve Sıcak nokta (Getis-Ord Gi\*)
//! (docs/adr/0238 §8–§9): the objects with a place and a value, their
//! neighbours by the chosen concept (`weights`).

use super::inference::{
    Inference, MOST_WORK, Profile, TIE, each_permutation, folded_p, moments, sim_z,
};
use super::weights::{Concept, Neighbours, neighbours};
use super::{
    Number, Pattern, Placed, StatsRun, Table, number_of, p_value, place, text, unplaced_note,
    unread_note,
};
use crate::entity::Shape;
use crate::jsmath::js_max;
use crate::ops::sampling::random::Sequence;

/// The neighbourhood a run asks for: the concept, the band (none: the
/// largest nearest-neighbour distance) and k.
#[derive(Clone, Copy, Debug)]
pub struct Neighbourhood {
    pub concept: Concept,
    pub band: Option<f64>,
    pub k: usize,
}

/// The objects with a place and a value: their places, values and places
/// in the input; the warnings about the others.
pub(super) fn valued(
    shapes: &[Shape],
    values: &[Option<String>],
    field: &str,
    run: &mut StatsRun,
) -> (Placed, Vec<f64>) {
    let mut unread = 0usize;
    let mut xs = Vec::new();
    let placed = Placed::of(shapes, |i| {
        match number_of(values.get(i).and_then(|v| v.as_deref())) {
            Some(x) => {
                xs.push(x);
                true
            }
            None => {
                unread += 1;
                false
            }
        }
    });
    run.warn_if(placed.unplaced, unplaced_note);
    run.warn_if(unread, |n| unread_note(n, "değeri", field));
    (placed, xs)
}

/// The neighbourhood's line in the table.
pub(super) fn described(nb: &Neighbours, concept: Concept) -> String {
    match (concept, nb.band) {
        (Concept::Band, Some(b)) => format!("Sabit uzaklık bandı, {} m", text(b, 3)),
        (Concept::Inverse, Some(b)) => format!("Ters uzaklık, {} m", text(b, 3)),
        _ => format!("{} en yakın komşu", nb.k),
    }
}

/// Moran's I over the objects' values (§8); `standardize`: rows divided by their sums.
pub fn morans_i(
    shapes: &[Shape],
    values: &[Option<String>],
    field: &str,
    hood: Neighbourhood,
    standardize: bool,
) -> Result<StatsRun, String> {
    morans_i_with(
        shapes,
        values,
        field,
        hood,
        standardize,
        Inference::ANALYTIC,
    )
}

/// [`morans_i`] with permutation inference too (docs/adr/0520 §3): the I of
/// `R` permutations of the values, the pseudo z and p, the profile.
pub fn morans_i_with(
    shapes: &[Shape],
    values: &[Option<String>],
    field: &str,
    hood: Neighbourhood,
    standardize: bool,
    inference: Inference,
) -> Result<StatsRun, String> {
    let inference = inference.checked()?;
    let mut run = StatsRun::default();
    let (placed, xs) = valued(shapes, values, field, &mut run);
    let n = xs.len();
    if n < 4 {
        return Err(format!(
            "Moran I en az dört nesne ister; yeri ve değeri olan {n} nesne var."
        ));
    }
    let nf = n as f64;
    let mean = xs.iter().sum::<f64>() / nf;
    let z: Vec<f64> = xs.iter().map(|x| x - mean).collect();
    let (mut m2, mut m4) = (0.0, 0.0);
    for v in &z {
        let s = v * v;
        m2 += s;
        m4 += s * s;
    }
    if m2.is_nan() || m2 <= 0.0 {
        return Err("Değerlerin hepsi aynı; Moran I hesaplanamaz.".into());
    }
    let mut nb = neighbours(&placed.pts, hood.concept, hood.band, hood.k, false)?;
    let mut isolated = 0usize;
    let mut rows = vec![0.0; n];
    for (i, list) in nb.lists.iter_mut().enumerate() {
        let sum: f64 = list.iter().map(|&(_, w)| w).sum();
        if list.is_empty() || sum <= 0.0 {
            isolated += 1;
            list.clear();
            continue;
        }
        if standardize {
            for e in list.iter_mut() {
                e.1 /= sum;
            }
            rows[i] = list.iter().map(|&(_, w)| w).sum();
        } else {
            rows[i] = sum;
        }
    }
    run.warn_if(isolated, |n| {
        format!("{n} nesnenin komşusu yok; satırı sıfır.")
    });
    let mut cols = vec![0.0; n];
    let (mut s0, mut cross, mut s1) = (0.0, 0.0, 0.0);
    for (i, list) in nb.lists.iter().enumerate() {
        let mut lag = 0.0;
        for &(j, w) in list {
            let j = j as usize;
            s0 += w;
            lag += w * z[j];
            cols[j] += w;
            // w_ji: i in j's list (sorted), else 0.
            let back = nb.lists[j]
                .binary_search_by_key(&(i as u32), |&(m, _)| m)
                .map_or(0.0, |at| nb.lists[j][at].1);
            s1 += if back > 0.0 {
                (w + back) * (w + back) / 2.0
            } else {
                w * w
            };
        }
        cross += z[i] * lag;
    }
    if s0.is_nan() || s0 <= 0.0 {
        return Err("Hiçbir nesnenin komşusu yok; bandı büyütün.".into());
    }
    let s2: f64 = rows.iter().zip(&cols).map(|(r, c)| (r + c) * (r + c)).sum();
    let i_value = nf / s0 * cross / m2;
    let expected = -1.0 / (nf - 1.0);
    let b2 = nf * m4 / (m2 * m2);
    let a = nf * ((nf * nf - 3.0 * nf + 3.0) * s1 - nf * s2 + 3.0 * s0 * s0);
    let b = b2 * ((nf * nf - nf) * s1 - 2.0 * nf * s2 + 6.0 * s0 * s0);
    let c = (nf - 1.0) * (nf - 2.0) * (nf - 3.0) * s0 * s0;
    let variance = (a - b) / c - expected * expected;
    if variance.is_nan() || variance <= 0.0 {
        return Err("Moran I'nın varyansı hesaplanamadı; komşuluğu değiştirin.".into());
    }
    let zs = (i_value - expected) / variance.sqrt();
    let p = p_value(zs);
    let pattern = Pattern::of(zs, p, false);
    let row = |k: &str, v: String| vec![k.to_owned(), v];
    let mut rows = vec![
        row("Nesne sayısı", n.to_string()),
        row("Moran I", text(i_value, 6)),
        row("Beklenen I", text(expected, 6)),
        row("Varyans", text(variance, 8)),
        row("z", text(zs, 4)),
        row("p", text(p, 6)),
        row("Desen", pattern.title().to_owned()),
        row("Komşuluk", described(&nb, hood.concept)),
        row("Komşusu olmayan", isolated.to_string()),
    ];
    run.numbers = vec![
        Number {
            name: "moransI",
            value: i_value,
        },
        Number {
            name: "z",
            value: zs,
        },
        Number {
            name: "p",
            value: p,
        },
    ];
    run.summary = format!(
        "Moran I {} (z {}, p {}): {}.",
        text(i_value, 6),
        text(zs, 4),
        text(p, 6),
        pattern.word()
    );
    if inference.permuted() {
        permute_moran(
            &mut run,
            &mut rows,
            &Moran {
                nb: &nb,
                z: &z,
                s0,
                m2,
                i_value,
            },
            inference,
        )?;
        run.profile = Some(profile_of(
            "Moran I",
            inference,
            false,
            described(&nb, hood.concept),
            Some(standardize),
            field,
            shapes,
            &placed,
            &xs,
        ));
    }
    run.table = Some(Table {
        columns: vec!["Ölçü".into(), "Değer".into()],
        rows,
    });
    Ok(run)
}

/// What the permutations of Moran's I need: the weights, the deviations, S₀,
/// Σ z² and the observed I.
struct Moran<'a> {
    nb: &'a Neighbours,
    z: &'a [f64],
    s0: f64,
    m2: f64,
    i_value: f64,
}

/// The I of `R` permutations of the values (docs/adr/0520 §3): the table's
/// rows, the numbers, the summary.
fn permute_moran(
    run: &mut StatsRun,
    rows: &mut Vec<Vec<String>>,
    m: &Moran<'_>,
    inference: Inference,
) -> Result<(), String> {
    let n = m.z.len();
    let nf = n as f64;
    let r = inference.permutations;
    let pairs: usize = m.nb.lists.iter().map(Vec::len).sum();
    if r.saturating_mul(n + pairs) > MOST_WORK {
        return Err(too_much_work());
    }
    // Every permuted I is at most this in size: what a tie is measured against.
    let total: f64 = m.nb.lists.iter().flatten().map(|&(_, w)| w.abs()).sum();
    let zmax = m.z.iter().fold(0.0f64, |a, v| js_max(a, v.abs()));
    let bound = nf / m.s0 * total * zmax * zmax / m.m2;
    let mut seq = Sequence::new(inference.seed);
    let mut sims = Vec::with_capacity(r);
    let mut shuffled = vec![0.0; n];
    each_permutation(n, r, &mut seq, |perm| {
        for (to, &from) in shuffled.iter_mut().zip(perm) {
            *to = m.z[from as usize];
        }
        let mut cross = 0.0;
        for (i, list) in m.nb.lists.iter().enumerate() {
            let mut lag = 0.0;
            for &(j, w) in list {
                lag += w * shuffled[j as usize];
            }
            cross += shuffled[i] * lag;
        }
        sims.push(nf / m.s0 * cross / m.m2);
    });
    let (mean, sd) = moments(&sims);
    let spread = sd > TIE * bound;
    let larger = sims
        .iter()
        .filter(|&&s| s >= m.i_value - TIE * bound)
        .count();
    let ps = spread.then(|| folded_p(larger, r));
    let zsim = if spread {
        sim_z(m.i_value, mean, sd)
    } else {
        None
    };
    let pattern = match (zsim, ps) {
        (Some(z), Some(p)) => Pattern::of(z, p, false),
        _ => Pattern::Random,
    };
    let opt = |v: Option<f64>, d: u32| v.map_or_else(String::new, |v| text(v, d));
    let row = |k: &str, v: String| vec![k.to_owned(), v];
    rows[6] = row("Desen", pattern.title().to_owned());
    rows.extend([
        row("Permütasyon sayısı", r.to_string()),
        row("Tohum", inference.seed.to_string()),
        row("Permütasyon ortalaması", text(mean, 6)),
        row("Permütasyon z", opt(zsim, 4)),
        row("Permütasyon p", opt(ps, 6)),
        row(
            "p'nin standart hatası",
            opt(ps.map(|p| (p * (1.0 - p) / r as f64).sqrt()), 6),
        ),
    ]);
    if !spread {
        run.warnings.push(
            "Permütasyonların I'ları yayılmıyor; permütasyon p'si yok, desen rastgele sayıldı."
                .into(),
        );
    }
    run.numbers.push(Number {
        name: "zPermutation",
        value: zsim.unwrap_or(f64::NAN),
    });
    run.numbers.push(Number {
        name: "pPermutation",
        value: ps.unwrap_or(f64::NAN),
    });
    run.summary = format!(
        "Moran I {} (permütasyon z {}, p {}; {r} permütasyon, tohum {}): {}.",
        text(m.i_value, 6),
        opt(zsim, 4),
        opt(ps, 6),
        inference.seed,
        pattern.word()
    );
    Ok(())
}

/// The refusal of a run whose permutations are too much work.
pub(super) fn too_much_work() -> String {
    format!(
        "Permütasyonların işi çok büyük: permütasyon sayısı ile komşu çiftlerinin çarpımı {MOST_WORK}'ı aşıyor; permütasyon sayısını ya da komşuluğu küçültün."
    )
}

/// A run's profile (docs/adr/0520 §5): what makes it again, and the canonical
/// text of the objects it analysed (their place in the input, their place and
/// value).
#[allow(clippy::too_many_arguments)]
pub(super) fn profile_of(
    tool: &str,
    inference: Inference,
    conditional: bool,
    hood: String,
    standardize: Option<bool>,
    field: &str,
    shapes: &[Shape],
    placed: &Placed,
    xs: &[f64],
) -> Profile {
    let mut p = Profile::default();
    let permuted = inference.permuted();
    p.push("rule", "Kural", super::inference::RULE);
    p.push("tool", "Araç", tool);
    p.push(
        "inference",
        "Çıkarım",
        match (permuted, conditional) {
            (false, _) => "Normal dağılım",
            (true, false) => "Permütasyon",
            (true, true) => "Koşullu permütasyon",
        },
    );
    p.push(
        "permutations",
        "Permütasyon sayısı",
        inference.permutations.to_string(),
    );
    p.push(
        "seed",
        "Tohum",
        if permuted {
            inference.seed.to_string()
        } else {
            "—".into()
        },
    );
    p.push(
        "generator",
        "Sayı üreteci",
        if permuted {
            super::inference::GENERATOR
        } else {
            "—"
        },
    );
    p.push(
        "correction",
        "Çoklu test düzeltmesi",
        if tool == "Moran I" {
            "Yok (tek sınama)"
        } else {
            inference.correction.title()
        },
    );
    p.push("weights", "Komşuluk", hood);
    p.push(
        "standardize",
        "Satır standartlaştırma",
        match standardize {
            Some(true) => "Evet",
            Some(false) => "Hayır",
            None => "Uygulanmaz",
        },
    );
    p.push("objects", "Nesne sayısı", xs.len().to_string());
    p.push("field", "Değer alanı", field);
    for (k, &i) in placed.index.iter().enumerate() {
        let at = place(&shapes[i]).unwrap_or_default();
        super::inference::input_line(&mut p.inputs, i, at.x, at.y, xs[k]);
    }
    p
}

/// The confidence classes' colours (ColorBrewer's RdBu), −3 … 3.
pub const HOT_COLORS: [&str; 7] = [
    "#2166AC", "#67A9CF", "#D1E5F0", "#D9D9D9", "#FDDBC7", "#EF8A62", "#B2182B",
];

/// A Gi\* z's class: ±3 at p < 0.01, ±2 at p < 0.05, ±1 at p < 0.10, else 0.
pub fn hot_class(z: f64, p: f64) -> i32 {
    let level = if p < 0.01 {
        3
    } else if p < 0.05 {
        2
    } else if p < 0.10 {
        1
    } else {
        0
    };
    if z < 0.0 { -level } else { level }
}

/// Getis-Ord Gi\* of every object with a place and a value (§9).
pub fn hot_spots(
    shapes: &[Shape],
    values: &[Option<String>],
    field: &str,
    hood: Neighbourhood,
) -> Result<StatsRun, String> {
    super::local::hot_spots_with(shapes, values, field, hood, Inference::ANALYTIC)
}
