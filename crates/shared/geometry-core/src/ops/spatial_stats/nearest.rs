//! En yakın komşu (docs/adr/0238 §6; Clark and Evans 1954): the observed
//! mean distance to the nearest other place against a random pattern's
//! expected one over the same area; the ratio, z and p. With a Monte Carlo
//! test (docs/adr/0566 §1) as many places are thrown into the same box again
//! and again, the edge falling on both alike.

use super::inference::{self, GENERATOR, Profile, RULE, TIE, folded_p, moments, sim_z};
use super::kdtree::{KdTree, dist2};
use super::{Number, Pattern, Placed, StatsRun, Table, p_value, text, unplaced_note};
use crate::entity::Shape;
use crate::ops::sampling::random::Sequence;
use crate::vec2::Vec2;

/// The most simulated places a run throws (simulations × objects).
pub const MOST_SIMULATED: usize = 50_000_000;

/// A Monte Carlo test's simulations (0: none) and seed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Simulation {
    pub count: usize,
    pub seed: u64,
}

impl Simulation {
    /// The refusal of a count out of range.
    pub fn checked(self, objects: usize) -> Result<Simulation, String> {
        if self.count == 0 {
            return Ok(self);
        }
        if self.count < 9 || self.count > inference::MOST_PERMUTATIONS {
            return Err(format!(
                "Benzetim sayısı 9 ile {} arasında olmalı; {} istendi.",
                inference::MOST_PERMUTATIONS,
                self.count
            ));
        }
        if self.count.saturating_mul(objects) > MOST_SIMULATED {
            return Err(format!(
                "{} benzetim × {objects} nesne en çok {MOST_SIMULATED} rastgele yer eder; benzetim sayısını azaltın.",
                self.count
            ));
        }
        Ok(self)
    }
}

/// The places' box in the worked frame: its low corner and sides.
fn box_of(pts: &[Vec2]) -> (Vec2, f64, f64) {
    let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for p in pts {
        if p.x < x0 {
            x0 = p.x;
        }
        if p.x > x1 {
            x1 = p.x;
        }
        if p.y < y0 {
            y0 = p.y;
        }
        if p.y > y1 {
            y1 = p.y;
        }
    }
    (Vec2::new(x0, y0), x1 - x0, y1 - y0)
}

/// The simulations' box (§1): the places' own, or scaled about its centre to `area` (a square of side √A when the
/// places lie on a line); its low corner, sides and words.
fn region(pts: &[Vec2], area: Option<f64>) -> (Vec2, f64, f64, &'static str) {
    let (lo, w, h) = box_of(pts);
    let Some(a) = area else {
        return (lo, w, h, "Yerlerin kutusu");
    };
    let (cx, cy) = (lo.x + w / 2.0, lo.y + h / 2.0);
    let (w2, h2) = if w > 0.0 && h > 0.0 {
        let s = (a / (w * h)).sqrt();
        (w * s, h * s)
    } else {
        (a.sqrt(), a.sqrt())
    };
    (
        Vec2::new(cx - w2 / 2.0, cy - h2 / 2.0),
        w2,
        h2,
        "Alanla ölçeklenmiş kutu",
    )
}

/// The mean nearest-neighbour distance of the places.
fn mean_nearest(pts: &[Vec2]) -> f64 {
    nearest_distances(pts).iter().sum::<f64>() / pts.len() as f64
}

/// The canonical line of a place without a value (docs/adr/0566 §3).
pub fn place_line(out: &mut String, index: usize, x: f64, y: f64) {
    use std::fmt::Write;
    let _ = writeln!(out, "{index}\t{}\t{}", text(x, 6), text(y, 6));
}

/// The nearest-neighbour distance of every place (0 for one that shares
/// its place); none when there is a single place.
pub fn nearest_distances(pts: &[crate::vec2::Vec2]) -> Vec<f64> {
    let tree = KdTree::new(pts);
    pts.iter()
        .enumerate()
        .map(|(i, &p)| {
            tree.nearest(p, 1, Some(i as u32))
                .first()
                .map_or(f64::NAN, |&(_, j)| dist2(p, pts[j as usize]).sqrt())
        })
        .collect()
}

/// The index over the objects' places; `area` the study area in m² (none:
/// the places' box).
pub fn nearest(shapes: &[Shape], area: Option<f64>) -> Result<StatsRun, String> {
    nearest_with(shapes, area, Simulation::default())
}

/// The index, with a Monte Carlo test when `sim` asks for one (docs/adr/0566 §1).
pub fn nearest_with(
    shapes: &[Shape],
    area: Option<f64>,
    sim: Simulation,
) -> Result<StatsRun, String> {
    let placed = Placed::of(shapes, |_| true);
    let mut run = StatsRun::default();
    run.warn_if(placed.unplaced, unplaced_note);
    let n = placed.pts.len();
    if n < 2 {
        return Err(format!(
            "En yakın komşu en az iki nesne ister; {n} nesnenin yeri var."
        ));
    }
    let a = match area {
        Some(a) => a,
        None => {
            let (_, w, h) = box_of(&placed.pts);
            w * h
        }
    };
    if a.is_nan() || a <= 0.0 {
        return Err("Yerlerin kutusunun alanı sıfır; Alan'ı yazın.".into());
    }
    let sim = sim.checked(n)?;
    let d = nearest_distances(&placed.pts);
    let nf = n as f64;
    let observed = d.iter().sum::<f64>() / nf;
    let expected = 0.5 / (nf / a).sqrt();
    let ratio = observed / expected;
    let se = 0.26136 / (nf * nf / a).sqrt();
    let z = (observed - expected) / se;
    let p = p_value(z);
    let row = |k: &str, v: String| vec![k.to_owned(), v];
    let mut rows = vec![
        row("Nesne sayısı", n.to_string()),
        row("Gözlenen ortalama uzaklık (m)", text(observed, 3)),
        row("Beklenen ortalama uzaklık (m)", text(expected, 3)),
        row("En yakın komşu oranı", text(ratio, 4)),
        row("z", text(z, 4)),
        row("p", text(p, 6)),
        row("Alan (m²)", text(a, 2)),
    ];
    run.numbers = vec![
        Number {
            name: "ratio",
            value: ratio,
        },
        Number {
            name: "z",
            value: z,
        },
        Number {
            name: "p",
            value: p,
        },
    ];
    if sim.count == 0 {
        let pattern = Pattern::of(z, p, true);
        rows.push(row("Desen", pattern.title().to_owned()));
        run.table = Some(Table {
            columns: vec!["Ölçü".into(), "Değer".into()],
            rows,
        });
        run.summary = format!(
            "En yakın komşu oranı {} (z {}, p {}): {}.",
            text(ratio, 4),
            text(z, 4),
            text(p, 6),
            pattern.word()
        );
        return Ok(run);
    }
    // The Monte Carlo test (§1): the same count thrown into the box, point by point, x then y.
    let (lo, w, h, words) = region(&placed.pts, area);
    let bound = (w * w + h * h).sqrt();
    let mut seq = Sequence::new(sim.seed);
    let mut thrown = vec![Vec2::new(0.0, 0.0); n];
    let mut sims = Vec::with_capacity(sim.count);
    let mut larger = 0usize;
    for _ in 0..sim.count {
        for q in thrown.iter_mut() {
            let x = lo.x + seq.unit() * w;
            let y = lo.y + seq.unit() * h;
            *q = Vec2::new(x, y);
        }
        let m = mean_nearest(&thrown);
        if m >= observed - TIE * bound {
            larger += 1;
        }
        sims.push(m);
    }
    let (mean, sd) = moments(&sims);
    let ps = folded_p(larger, sim.count);
    let zs = sim_z(observed, mean, sd);
    let se = (ps * (1.0 - ps) / sim.count as f64).sqrt();
    let pattern = match zs {
        Some(zs) => Pattern::of(zs, ps, true),
        None => Pattern::Random,
    };
    let zs_text = zs.map_or_else(|| "—".to_owned(), |z| text(z, 4));
    rows.extend([
        row("Benzetim sayısı", sim.count.to_string()),
        row("Tohum", sim.seed.to_string()),
        row("Benzetim alanı", words.to_owned()),
        row("Benzetimlerin ortalama uzaklığı (m)", text(mean, 3)),
        row("Benzetim oranı", text(observed / mean, 4)),
        row("Benzetim z", zs_text.clone()),
        row("Benzetim p", text(ps, 6)),
        row("p'nin standart hatası", text(se, 6)),
        row("Desen", pattern.title().to_owned()),
    ]);
    run.table = Some(Table {
        columns: vec!["Ölçü".into(), "Değer".into()],
        rows,
    });
    if let Some(zs) = zs {
        run.numbers.push(Number {
            name: "zSimulation",
            value: zs,
        });
    }
    run.numbers.push(Number {
        name: "pSimulation",
        value: ps,
    });
    run.summary = format!(
        "En yakın komşu oranı {} (benzetim z {}, p {}; {} rastgele desen, tohum {}): {}.",
        text(ratio, 4),
        zs_text,
        text(ps, 6),
        sim.count,
        sim.seed,
        pattern.word()
    );
    let mut profile = Profile::default();
    profile.push("rule", "Kural", RULE);
    profile.push("tool", "Araç", "En yakın komşu");
    profile.push("inference", "Çıkarım", "Monte Carlo (rastgele desen)");
    profile.push("simulations", "Benzetim sayısı", sim.count.to_string());
    profile.push("seed", "Tohum", sim.seed.to_string());
    profile.push("generator", "Sayı üreteci", GENERATOR);
    profile.push(
        "region",
        "Benzetim alanı",
        format!("{words}, {} m²", text(w * h, 2)),
    );
    profile.push("objects", "Nesne sayısı", n.to_string());
    for &i in &placed.index {
        let at = super::place(&shapes[i]).unwrap_or_default();
        place_line(&mut profile.inputs, i, at.x, at.y);
    }
    run.profile = Some(profile);
    Ok(run)
}
