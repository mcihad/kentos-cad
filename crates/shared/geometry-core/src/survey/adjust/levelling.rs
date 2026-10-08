//! Kot ağı dengelemesi (docs/adr/0203 §5): known heights fixed or weighted,
//! rows of height differences with their lengths, geometric levelling
//! (σ per √km) or trigonometric heights (σ from the zenith angle and the
//! distance), the approximations spread from the known heights, the
//! statistics as the horizontal network's.

use std::collections::HashMap;

use super::{
    MAX_ITERATIONS, MAX_UNKNOWNS, Normal, ObservationResult, Observed, Sigmas, name_key, statistics,
};
use crate::api::Op;
use crate::op;

/// A known height: its name, its height (m) and, when weighted, its standard deviation (m).
#[derive(Clone, Debug, PartialEq)]
pub struct KnownHeight {
    pub name: String,
    pub h: f64,
    pub sigma: Option<f64>,
}

crate::json_struct!(KnownHeight { name, h, sigma });

/// A height difference from `from` to `to` (m) over its length (m; the
/// horizontal distance of a trigonometric height); the line of the window's
/// table it is (none: its place, from 1), which a refusal names.
#[derive(Clone, Debug, PartialEq)]
pub struct LevelRow {
    pub from: String,
    pub to: String,
    pub dh: f64,
    pub length: f64,
    pub line: Option<usize>,
}

crate::json_struct!(LevelRow {
    from,
    to,
    dh,
    length,
    line
});

/// A levelling network: `geometric` or `trigonometric`, the a priori
/// standard deviations, the known heights, the height differences.
#[derive(Clone, Debug, PartialEq)]
pub struct LevelInput {
    pub kind: String,
    pub sigma: Sigmas,
    pub known: Vec<KnownHeight>,
    pub rows: Vec<LevelRow>,
}

crate::json_struct!(LevelInput {
    kind,
    sigma,
    known,
    rows
});

/// A height adjusted and its standard deviation (m).
#[derive(Clone, Debug, PartialEq)]
pub struct AdjustedHeight {
    pub name: String,
    pub h: f64,
    pub sh: f64,
}

crate::json_struct!(out AdjustedHeight { name, h, sh });

/// A levelling network adjusted: its weighted then new points, every
/// observation (the rows, then the weighted heights), the flagged
/// observation with the greatest test value, the counts, vᵀPv, m0 and the
/// model test.
#[derive(Clone, Debug, PartialEq)]
pub struct LevelResult {
    pub points: Vec<AdjustedHeight>,
    pub observations: Vec<ObservationResult>,
    pub worst: Option<usize>,
    pub n: usize,
    pub u: usize,
    pub f: usize,
    pub omega: f64,
    pub m0: Option<f64>,
    pub chi2: Option<f64>,
    pub passed: Option<bool>,
    pub iterations: usize,
}

crate::json_struct!(out LevelResult {
    points,
    observations,
    worst,
    n,
    u,
    f,
    omega,
    m0,
    chi2,
    passed,
    iterations
});

/// The levelling network adjusted (docs/adr/0203 §5), or why not.
pub fn adjust(input: &LevelInput) -> Result<LevelResult, String> {
    let trig = match input.kind.as_str() {
        "geometric" => false,
        "trigonometric" => true,
        other => {
            return Err(format!(
                "Ölçü türü “{other}” tanınmıyor (geometric ya da trigonometric)."
            ));
        }
    };
    let mut names: HashMap<String, String> = HashMap::new();
    let mut known: HashMap<String, (f64, Option<f64>)> = HashMap::new();
    let mut known_order: Vec<String> = Vec::new();
    for k in &input.known {
        let key = name_key(&k.name);
        if known.contains_key(&key) {
            return Err(format!("{} bilinen noktalarda iki kez var.", k.name.trim()));
        }
        if !k.h.is_finite() {
            return Err(format!("{} noktasının kotu bir sayı değil.", k.name.trim()));
        }
        if let Some(s) = k.sigma
            && !(s.is_finite() && s > 0.0)
        {
            return Err(format!(
                "{} noktasının σ'sı sıfırdan büyük olmalı.",
                k.name.trim()
            ));
        }
        known.insert(key.clone(), (k.h, k.sigma));
        names.insert(key.clone(), k.name.trim().to_owned());
        known_order.push(key);
    }
    if input.rows.is_empty() {
        return Err("Gözlem yok: gözlemler tablosuna kot farkı girin.".to_owned());
    }
    let mut appearance: Vec<String> = Vec::new();
    let mut rows: Vec<(String, String)> = Vec::new();
    for (i, row) in input.rows.iter().enumerate() {
        let (a, b) = (name_key(&row.from), name_key(&row.to));
        let line = row.line.unwrap_or(i + 1);
        if a == b {
            return Err(format!("{line}. satırda başlangıç ve bitiş aynı nokta."));
        }
        if !(row.length.is_finite() && row.length > 0.0) {
            return Err(format!("{line}. satırda uzunluk sıfırdan büyük olmalı."));
        }
        if !row.dh.is_finite() {
            return Err(format!("{line}. satırda kot farkı bir sayı değil."));
        }
        for (key, name) in [(&a, &row.from), (&b, &row.to)] {
            names
                .entry(key.clone())
                .or_insert_with(|| name.trim().to_owned());
            if !appearance.contains(key) {
                appearance.push(key.clone());
            }
        }
        rows.push((a, b));
    }
    let mut h: HashMap<String, f64> = known_order
        .iter()
        .map(|k| (k.clone(), known[k].0))
        .collect();
    loop {
        let mut changed = false;
        for (i, (a, b)) in rows.iter().enumerate() {
            let dh = input.rows[i].dh;
            match (h.get(a).copied(), h.get(b).copied()) {
                (Some(ha), None) => {
                    h.insert(b.clone(), ha + dh);
                    changed = true;
                }
                (None, Some(hb)) => {
                    h.insert(a.clone(), hb - dh);
                    changed = true;
                }
                _ => {}
            }
        }
        if !changed {
            break;
        }
    }
    let missing: Vec<&str> = appearance
        .iter()
        .filter(|k| !h.contains_key(*k))
        .map(|k| names[k].as_str())
        .collect();
    if !missing.is_empty() {
        return Err(format!(
            "Bilinen bir kota bağlı olmayan nokta: {}. Noktayı bilinen bir kota bağlayan kot farkı girin.",
            missing.join(", ")
        ));
    }
    let weighted: Vec<String> = known_order
        .iter()
        .filter(|k| known[*k].1.is_some())
        .cloned()
        .collect();
    let points: Vec<String> = weighted
        .iter()
        .cloned()
        .chain(
            appearance
                .iter()
                .filter(|k| !known.contains_key(*k))
                .cloned(),
        )
        .collect();
    let col: HashMap<String, usize> = points
        .iter()
        .enumerate()
        .map(|(i, k)| (k.clone(), i))
        .collect();
    let u = points.len();
    if u > MAX_UNKNOWNS {
        return Err(format!(
            "Ağ çok büyük: {u} bilinmeyen; en çok {MAX_UNKNOWNS} bilinmeyen dengelenir."
        ));
    }
    let s = input.sigma;
    let observations = |h: &HashMap<String, f64>| -> Vec<Observed> {
        let mut out = Vec::new();
        for (i, (a, b)) in rows.iter().enumerate() {
            let row = &input.rows[i];
            let mut coef = Vec::with_capacity(2);
            if let Some(&c) = col.get(a) {
                coef.push((c, -1.0));
            }
            if let Some(&c) = col.get(b) {
                coef.push((c, 1.0));
            }
            let sigma = if trig {
                let plain = s.distance + s.ppm * 1e-6 * row.length;
                let angle = row.length * s.zenith;
                let slope = row.dh / row.length * plain;
                (angle * angle + slope * slope).sqrt()
            } else {
                s.levelling * (row.length / 1000.0).sqrt()
            };
            out.push(Observed {
                kind: "dh",
                row: i,
                coef,
                l: row.dh - (h[b] - h[a]),
                sigma,
            });
        }
        for k in &weighted {
            let (h0, sigma) = known[k];
            out.push(Observed {
                kind: "h",
                row: known_order.iter().position(|o| o == k).unwrap_or(0),
                coef: vec![(col[k], 1.0)],
                l: h0 - h[k],
                sigma: sigma.unwrap_or(1.0),
            });
        }
        out
    };
    let normal = |obs: &[Observed]| -> (Normal, Vec<f64>) {
        let mut n = Normal::new(u);
        let mut rhs = vec![0.0; u];
        for o in obs {
            let p = 1.0 / (o.sigma * o.sigma);
            for &(i, ai) in &o.coef {
                rhs[i] += p * ai * o.l;
                for &(j, aj) in &o.coef {
                    n.add(i, j, p * ai * aj);
                }
            }
        }
        (n, rhs)
    };
    let singular = |k: usize| {
        format!(
            "Ağın dayanağı ya da gözlemleri yetersiz: {} noktasının kotu belirlenemiyor. En az bir sabit ya da ağırlıklı kot ve her noktayı belirleyen gözlemler gerekir.",
            names[&points[k]]
        )
    };
    let mut iterations = 0;
    loop {
        iterations += 1;
        if iterations > MAX_ITERATIONS {
            return Err(format!(
                "Dengeleme {MAX_ITERATIONS} yinelemede yakınsamadı: gözlemleri denetleyin."
            ));
        }
        let obs = observations(&h);
        let (n, rhs) = normal(&obs);
        let delta = n.cholesky().map_err(singular)?.solve(&rhs);
        let mut biggest: f64 = 0.0;
        for k in &points {
            let d = delta[col[k]];
            *h.entry(k.clone()).or_insert(0.0) += d;
            biggest = crate::jsmath::js_max(biggest, d.abs());
        }
        if biggest <= 1e-9 {
            break;
        }
    }
    let obs = observations(&h);
    let (n, _) = normal(&obs);
    let q = n.cholesky().map_err(singular)?.inverse();
    let stats = statistics(&obs, &q);
    let s0 = stats.sigma0();
    let adjusted = points
        .iter()
        .map(|k| AdjustedHeight {
            name: names[k].clone(),
            h: h[k],
            sh: s0 * q.get(col[k], col[k]).sqrt(),
        })
        .collect();
    Ok(LevelResult {
        points: adjusted,
        observations: stats.observations,
        worst: stats.worst,
        n: stats.n,
        u: stats.u,
        f: stats.f,
        omega: stats.omega,
        m0: stats.m0,
        chi2: stats.chi2,
        passed: stats.passed,
        iterations,
    })
}

pub(crate) const OP: Op = op!("levelAdjust", |input: LevelInput| adjust(&input));
