//! DBSCAN'ın parametre taraması (docs/adr/0574 §2): DBSCAN over a grid of radii and least points, each cell's clusters,
//! noise and (when asked) stability, and a suggested pair from the k-distance graph's knee (Ester et al. 1996; Schubert
//! et al. 2017; Satopää et al. 2011).
//!
//! - **Least points** `m`: `first, first + step, …` up to `last` (at most 20 values).
//! - **Radius**: `steps` values from `r₀` to `r₁`, `r₀ + (r₁ − r₀)·j / (steps − 1)`; left empty, from half the knee's radius
//!   to twice it.
//! - **Suggestion**: `m*` the grid value nearest to 4 (ties to the larger); the k-distance graph of `m*` (each place's distance
//!   to its (m* − 1)'th nearest other place, ascending); the knee `ε*` where `x − y` of the normalised graph is largest (the
//!   first such place); the grid radius nearest to `ε*` (ties to the smaller).
//!
//! Only float64's four operations, square roots and sorting enter; the random numbers are the stability's (docs/adr/0566).

use super::clusters::dbscan_labels;
use super::inference::{GENERATOR, Profile, RULE};
use super::kdtree::KdTree;
use super::nearest::place_line;
use super::stability::{MOST_RESAMPLED, Notes, Stability, stable_clusters};
use super::{Number, Placed, StatsRun, Table, place, text, unplaced_note};
use crate::entity::Shape;
use crate::vec2::Vec2;

/// The most cells (radii × least points) a sweep clusters.
pub const MOST_CELLS: usize = 400;
/// The most least-points values.
pub const MOST_VALUES: usize = 20;
/// The radii's steps.
pub const FEWEST_STEPS: usize = 2;
pub const MOST_STEPS: usize = 50;
/// The least points the suggestion aims for (twice the dimension, Schubert et al. 2017).
pub const AIM: usize = 4;

/// The grid: least points `first … last` by `step`; the radii `from … to` in `steps` values, or none to take them round the
/// knee.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Grid {
    pub first: usize,
    pub last: usize,
    pub step: usize,
    pub radii: Option<(f64, f64)>,
    pub steps: usize,
}

/// Each place's distance to its (m − 1)'th nearest other place, ascending (m = 1: all 0).
pub fn k_distances(pts: &[Vec2], m: usize) -> Vec<f64> {
    let mut out: Vec<f64> = if m <= 1 {
        vec![0.0; pts.len()]
    } else {
        let tree = KdTree::new(pts);
        pts.iter()
            .enumerate()
            .map(|(i, &p)| {
                tree.nearest(p, m - 1, Some(i as u32))
                    .last()
                    .map_or(f64::NAN, |&(d2, _)| d2.sqrt())
            })
            .collect()
    };
    out.sort_by(f64::total_cmp);
    out
}

/// The knee of an ascending graph (Kneedle without smoothing, a convex increasing curve): the first place where
/// `i/(n−1) − (dᵢ − d₀)/(dₙ₋₁ − d₀)` is largest; its distance. The first distance when there are fewer than three
/// places or all are equal.
pub fn knee(sorted: &[f64]) -> f64 {
    let n = sorted.len();
    let (d0, last) = (sorted[0], sorted[n - 1]);
    if n < 3 || last == d0 {
        return d0;
    }
    let span = last - d0;
    let m = (n - 1) as f64;
    let (mut best, mut most) = (0usize, 0.0f64);
    for (i, &d) in sorted.iter().enumerate() {
        let g = i as f64 / m - (d - d0) / span;
        if g > most {
            most = g;
            best = i;
        }
    }
    sorted[best]
}

/// The least-points values of a grid.
fn values_of(g: &Grid) -> Result<Vec<usize>, String> {
    if g.first < 1 || g.step < 1 || g.last < g.first {
        return Err("En az nokta: İlk en az 1, Adım en az 1 ve Son İlk'ten küçük olmamalı.".into());
    }
    let mut out = Vec::new();
    let mut m = g.first;
    while m <= g.last {
        out.push(m);
        if out.len() > MOST_VALUES {
            return Err(format!(
                "En az nokta için en çok {MOST_VALUES} değer denenir; Adım'ı büyütün ya da Son'u küçültün."
            ));
        }
        m += g.step;
    }
    Ok(out)
}

/// The least-points value the suggestion takes: the nearest to [`AIM`], ties to the larger.
fn aimed(values: &[usize]) -> usize {
    let mut best = values[0];
    for &m in &values[1..] {
        let (a, b) = (m.abs_diff(AIM), best.abs_diff(AIM));
        if a < b || (a == b && m > best) {
            best = m;
        }
    }
    best
}

/// DBSCAN over the grid; `border_noise`: DBSCAN\*'s rule in every cell; `stability` its resampling (none: no columns).
pub fn dbscan_sweep(
    shapes: &[Shape],
    grid: Grid,
    border_noise: bool,
    stability: Stability,
) -> Result<StatsRun, String> {
    let placed = Placed::of(shapes, |_| true);
    let mut run = StatsRun::default();
    run.warn_if(placed.unplaced, unplaced_note);
    let n = placed.pts.len();
    if n < 2 {
        return Err(format!(
            "DBSCAN taraması en az iki nesne ister; {n} nesnenin yeri var."
        ));
    }
    let mins = values_of(&grid)?;
    let Some(&widest) = mins.last() else {
        return Err("En az nokta için değer yok.".into());
    };
    if widest > n {
        return Err(format!(
            "En az nokta ({widest}) nesne sayısından ({n}) büyük olamaz."
        ));
    }
    if !(FEWEST_STEPS..=MOST_STEPS).contains(&grid.steps) {
        return Err(format!(
            "Yarıçap adım sayısı {FEWEST_STEPS} ile {MOST_STEPS} arasında olmalı; {} istendi.",
            grid.steps
        ));
    }
    let cells = mins.len() * grid.steps;
    if cells > MOST_CELLS {
        return Err(format!(
            "{cells} hücre (yarıçap × en az nokta) en çok {MOST_CELLS}; adım sayısını ya da en az nokta değerlerini azaltın."
        ));
    }
    let stability = stability.checked(n)?;
    if stability.resamples.saturating_mul(n).saturating_mul(cells) > MOST_RESAMPLED {
        return Err(format!(
            "{cells} hücre × {} tekrar × {n} nesne en çok {MOST_RESAMPLED} yeniden kümelenen yer eder; tekrar sayısını azaltın.",
            stability.resamples
        ));
    }
    // The suggestion's least points and the knee of its k-distance graph (§2).
    let aim = aimed(&mins);
    let graph = k_distances(&placed.pts, aim);
    let eps = knee(&graph);
    let (r0, r1) = match grid.radii {
        Some((a, b)) => {
            if !(a > 0.0 && b > a && b.is_finite()) {
                return Err("Yarıçap: İlk sıfırdan büyük ve Son İlk'ten büyük olmalı.".into());
            }
            (a, b)
        }
        None => {
            if !(eps > 0.0) {
                return Err(format!(
                    "En az {aim} nokta için k-uzaklık sıfır (nesneler çakışık); yarıçap aralığını İlk ve Son ile yazın."
                ));
            }
            (0.5 * eps, 2.0 * eps)
        }
    };
    let last = (grid.steps - 1) as f64;
    let radii: Vec<f64> = (0..grid.steps)
        .map(|j| r0 + (r1 - r0) * (j as f64) / last)
        .collect();
    let mut suggested = 0usize;
    for (j, &r) in radii.iter().enumerate() {
        if (r - eps).abs() < (radii[suggested] - eps).abs() {
            suggested = j;
        }
    }
    let with_stability = stability.resamples > 0;
    let mut rows = Vec::with_capacity(cells);
    let mut chosen = None;
    let mut notes = Notes::default();
    for &m in &mins {
        for (j, &r) in radii.iter().enumerate() {
            let labels = dbscan_labels(&placed.pts, r, m, border_noise)
                .map_err(|why| format!("{why} (yarıçap {} m, en az {m} nokta)", text(r, 3)))?;
            let clusters = labels.iter().copied().max().unwrap_or(0) as usize;
            let noise = labels.iter().filter(|&&l| l == 0).count();
            let share = 100.0 * noise as f64 / n as f64;
            let mut row = vec![
                m.to_string(),
                text(r, 3),
                clusters.to_string(),
                noise.to_string(),
                text(share, 1),
            ];
            if with_stability {
                let found = stable_clusters(
                    &placed,
                    &labels,
                    stability,
                    |sample| dbscan_labels(sample, r, m, border_noise).map(|l| (l, false, false)),
                    &mut notes,
                )?;
                let means: Vec<f64> = found
                    .iter()
                    .skip(1)
                    .filter(|s| s.size > 0)
                    .filter_map(|s| s.mean())
                    .collect();
                let stable = means.iter().filter(|&&g| g >= 0.75).count();
                row.push(if means.is_empty() {
                    "—".to_owned()
                } else {
                    text(means.iter().sum::<f64>() / means.len() as f64, 3)
                });
                row.push(stable.to_string());
            }
            let is = m == aim && j == suggested;
            row.push(if is {
                "◀ önerilen".to_owned()
            } else {
                String::new()
            });
            if is {
                chosen = Some((clusters, share));
            }
            rows.push(row);
        }
    }
    let (clusters, share) = chosen.unwrap_or_default();
    let mut columns: Vec<String> = [
        "En az nokta",
        "Yarıçap (m)",
        "Küme sayısı",
        "Gürültü",
        "Gürültü payı (%)",
    ]
    .iter()
    .map(|c| (*c).to_owned())
    .collect();
    if with_stability {
        columns.push("Ortalama kararlılık".into());
        columns.push("Kararlı küme".into());
    }
    columns.push("Önerilen".into());
    run.table = Some(Table { columns, rows });
    // The k-distance graph the knee was read off, to be drawn: rank, distance and the knee marked (docs/adr/0639 §3).
    let at = graph.iter().position(|&d| d == eps);
    run.graph = Some(Table {
        columns: vec![
            "Sıra".into(),
            format!("{}. en yakın uzaklık (m)", aim.saturating_sub(1)),
            "Dirsek".into(),
        ],
        rows: graph
            .iter()
            .enumerate()
            .map(|(i, &d)| {
                vec![
                    (i + 1).to_string(),
                    text(d, 3),
                    if Some(i) == at {
                        "◀ dirsek".to_owned()
                    } else {
                        String::new()
                    },
                ]
            })
            .collect(),
    });
    run.numbers = vec![
        Number {
            name: "cells",
            value: cells as f64,
        },
        Number {
            name: "suggestedRadius",
            value: radii[suggested],
        },
        Number {
            name: "suggestedMinPoints",
            value: aim as f64,
        },
        Number {
            name: "kneeRadius",
            value: eps,
        },
        Number {
            name: "clusters",
            value: clusters as f64,
        },
        Number {
            name: "noiseShare",
            value: share,
        },
    ];
    run.summary = format!(
        "{cells} çift denendi; önerilen: yarıçap {} m, en az {aim} nokta ({clusters} küme, %{} gürültü); k-uzaklık dirseği {} m.",
        text(radii[suggested], 3),
        text(share, 1),
        text(eps, 3)
    );
    run.infos.push(
        "Öneri bir sezgidir: en az nokta ızgaradaki 4'e en yakın değer (Schubert ve ark. 2017), yarıçap o en az nokta için sıralı \
         k-uzaklık grafiğinin dirseğine (Ester ve ark. 1996; Satopää ve ark. 2011) en yakın ızgara değeridir."
            .to_owned(),
    );
    if with_stability {
        let parameters = format!(
            "Yarıçap {} – {} m ({} adım), en az nokta {} – {} (adım {}){}",
            text(radii[0], 3),
            text(radii[radii.len() - 1], 3),
            grid.steps,
            mins[0],
            widest,
            grid.step,
            if border_noise {
                ", sınır noktaları gürültü"
            } else {
                ""
            }
        );
        let mut profile = Profile::default();
        profile.push("rule", "Kural", RULE);
        profile.push("tool", "Araç", "DBSCAN parametre taraması");
        profile.push("inference", "Çıkarım", stability.words());
        profile.push(
            "resamples",
            "Tekrar sayısı",
            stability.resamples.to_string(),
        );
        profile.push("seed", "Tohum", stability.seed.to_string());
        profile.push("generator", "Sayı üreteci", GENERATOR);
        profile.push("parameters", "Parametreler", parameters);
        profile.push("objects", "Nesne sayısı", n.to_string());
        for &i in &placed.index {
            let at = place(&shapes[i]).unwrap_or_default();
            place_line(&mut profile.inputs, i, at.x, at.y);
        }
        run.profile = Some(profile);
    }
    Ok(run)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_knee_is_where_the_graph_bends_up() {
        // Flat then steep: the knee is the last flat place's distance.
        let d = [1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 5.0, 9.0];
        assert_eq!(knee(&d), 1.0);
        assert_eq!(knee(&[2.0, 2.0, 2.0]), 2.0);
        assert_eq!(knee(&[1.0, 4.0]), 1.0);
    }

    #[test]
    fn the_aim_is_four_with_ties_to_the_larger() {
        assert_eq!(aimed(&[3, 4, 5]), 4);
        assert_eq!(aimed(&[3, 5]), 5);
        assert_eq!(aimed(&[6, 7, 8]), 6);
        assert_eq!(aimed(&[1, 2]), 2);
    }

    #[test]
    fn the_values_run_from_first_to_last() {
        let g = Grid {
            first: 3,
            last: 8,
            step: 2,
            radii: None,
            steps: 7,
        };
        assert_eq!(values_of(&g).unwrap(), vec![3, 5, 7]);
        let wide = Grid {
            first: 1,
            last: 100,
            step: 1,
            radii: None,
            steps: 7,
        };
        assert!(values_of(&wide).is_err());
    }
}
