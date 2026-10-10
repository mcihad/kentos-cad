//! Mekânsal otokorelasyon (Moran I) ve Sıcak nokta (Getis-Ord Gi\*)
//! (docs/adr/0238 §8–§9): the objects with a place and a value, their
//! neighbours by the chosen concept (`weights`).

use super::weights::{Concept, Neighbours, neighbours};
use super::{
    Number, ObjectCopy, Pattern, Placed, StatsRun, Table, number_of, p_value, pair, text,
    unplaced_note, unread_note,
};
use crate::entity::Shape;

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
fn valued(
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
fn described(nb: &Neighbours, concept: Concept) -> String {
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
    run.table = Some(Table {
        columns: vec!["Ölçü".into(), "Değer".into()],
        rows: vec![
            row("Nesne sayısı", n.to_string()),
            row("Moran I", text(i_value, 6)),
            row("Beklenen I", text(expected, 6)),
            row("Varyans", text(variance, 8)),
            row("z", text(zs, 4)),
            row("p", text(p, 6)),
            row("Desen", pattern.title().to_owned()),
            row("Komşuluk", described(&nb, hood.concept)),
            row("Komşusu olmayan", isolated.to_string()),
        ],
    });
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
    Ok(run)
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
    let mut counts = [0usize; 7];
    let mut undefined = 0usize;
    for (k, list) in nb.lists.iter().enumerate() {
        let (mut sw, mut sw2, mut num) = (0.0, 0.0, 0.0);
        for &(j, w) in list {
            sw += w;
            sw2 += w * w;
            num += w * dev[j as usize];
        }
        let den = s * ((nf * sw2 - sw * sw) / (nf - 1.0)).sqrt();
        let (attrs, class) = if den > 0.0 && den.is_finite() {
            let z = num / den;
            let p = p_value(z);
            let class = hot_class(z, p);
            (
                vec![
                    pair("z puanı", text(z, 4)),
                    pair("p değeri", text(p, 6)),
                    pair("Güven sınıfı", class.to_string()),
                ],
                class,
            )
        } else {
            undefined += 1;
            (
                vec![
                    pair("z puanı", ""),
                    pair("p değeri", ""),
                    pair("Güven sınıfı", "0"),
                ],
                0,
            )
        };
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
    let hot = counts[4] + counts[5] + counts[6];
    let cold = counts[0] + counts[1] + counts[2];
    run.infos.push(format!(
        "Sıcak: %99 {}, %95 {}, %90 {}; soğuk: %99 {}, %95 {}, %90 {}.",
        counts[6], counts[5], counts[4], counts[0], counts[1], counts[2]
    ));
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
    Ok(run)
}
