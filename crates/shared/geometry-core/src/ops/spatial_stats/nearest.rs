//! En yakın komşu (docs/adr/0238 §6; Clark and Evans 1954): the observed
//! mean distance to the nearest other place against a random pattern's
//! expected one over the same area; the ratio, z and p.

use super::kdtree::{KdTree, dist2};
use super::{Number, Pattern, Placed, StatsRun, Table, p_value, text, unplaced_note};
use crate::entity::Shape;

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
            let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
            for p in &placed.pts {
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
            (x1 - x0) * (y1 - y0)
        }
    };
    if a.is_nan() || a <= 0.0 {
        return Err("Yerlerin kutusunun alanı sıfır; Alan'ı yazın.".into());
    }
    let d = nearest_distances(&placed.pts);
    let nf = n as f64;
    let observed = d.iter().sum::<f64>() / nf;
    let expected = 0.5 / (nf / a).sqrt();
    let ratio = observed / expected;
    let se = 0.26136 / (nf * nf / a).sqrt();
    let z = (observed - expected) / se;
    let p = p_value(z);
    let pattern = Pattern::of(z, p, true);
    let row = |k: &str, v: String| vec![k.to_owned(), v];
    run.table = Some(Table {
        columns: vec!["Ölçü".into(), "Değer".into()],
        rows: vec![
            row("Nesne sayısı", n.to_string()),
            row("Gözlenen ortalama uzaklık (m)", text(observed, 3)),
            row("Beklenen ortalama uzaklık (m)", text(expected, 3)),
            row("En yakın komşu oranı", text(ratio, 4)),
            row("z", text(z, 4)),
            row("p", text(p, 6)),
            row("Alan (m²)", text(a, 2)),
            row("Desen", pattern.title().to_owned()),
        ],
    });
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
    run.summary = format!(
        "En yakın komşu oranı {} (z {}, p {}): {}.",
        text(ratio, 4),
        text(z, 4),
        text(p, 6),
        pattern.word()
    );
    Ok(run)
}
