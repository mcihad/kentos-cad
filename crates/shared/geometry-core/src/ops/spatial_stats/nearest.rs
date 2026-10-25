//! En yakın komşu (docs/adr/0238 §6; Clark and Evans 1954): the observed
//! mean distance to the nearest other place against a random pattern's
//! expected one over the same area; the ratio, z and p. With a Monte Carlo
//! test (docs/adr/0566 §1) as many places are thrown into the same box again
//! and again, the edge falling on both alike. The study area may be given as
//! area objects instead (docs/adr/0574 §1): their union, holes out, is the
//! region the density, the places and the thrown points are taken in.
//! Donnelly's (1978) correction moves the expectation and its error for the
//! region's edge (docs/adr/0639 §2).

use super::inference::{self, GENERATOR, Profile, RULE, TIE, folded_p, moments, sim_z};
use super::kdtree::{KdTree, dist2};
use super::{Number, Pattern, Placed, StatsRun, Table, p_value, place, text, unplaced_note};
use crate::entity::Shape;
use crate::geom::arrangement::Area;
use crate::ops::geoprocess::{Class, areas_measure, class_of, union_all};
use crate::ops::sampling::inside::Inside;
use crate::ops::sampling::random::Sequence;
use crate::ops::sampling::scatter::DRAWS;
use crate::vec2::Vec2;

/// The most simulated places a run throws (simulations × objects).
pub const MOST_SIMULATED: usize = 50_000_000;

/// The most candidates a study area's run draws, from its simulations, objects and the box's share the area fills.
pub const MOST_THROWN: f64 = 5e8;

/// The most vertices a study area's rings have in all.
pub const MOST_VERTICES: usize = 1_000_000;

/// The region a run takes the density and the simulated places in (docs/adr/0574 §1).
#[derive(Clone, Copy)]
pub enum Window<'a> {
    /// The places' box, or the box scaled to an area in m² (docs/adr/0566 §1).
    Box(Option<f64>),
    /// The union of the areas of these objects.
    Areas(&'a [Shape]),
}

/// The expectation's correction for the region's edge (docs/adr/0639 §2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Edge {
    /// Clark and Evans's: an unbounded plane.
    #[default]
    None,
    /// Donnelly's (1978): `E = 0.5·√(A/n) + (0.0514 + 0.041/√n)·P/n`, `Var = 0.0703·A/n² + 0.037·P·√(A/n⁵)`.
    Donnelly,
}

/// The least objects Donnelly's simulations had (n ≥ 7).
pub const DONNELLY_FEWEST: usize = 7;

/// A study area: its region to ask, its net area, what it is made of and its canonical lines for the profile.
struct Study {
    inside: Inside,
    area: f64,
    /// The length of every ring of the union, holes too (Donnelly's P).
    perimeter: f64,
    parts: usize,
    holes: usize,
    skipped: usize,
    lines: String,
}

/// The study area of the objects' areas (docs/adr/0574 §1).
fn study_of(shapes: &[Shape]) -> Result<Study, String> {
    use std::fmt::Write;
    let mut areas: Vec<Area> = Vec::new();
    let mut skipped = 0usize;
    for s in shapes {
        match class_of(s) {
            Class::Areas(v) if !v.is_empty() => areas.extend(v),
            _ => skipped += 1,
        }
    }
    if areas.is_empty() {
        return Err(
            "Çalışma alanı olarak verilen nesnelerde alan yok; bir alan çizin ya da seçin.".into(),
        );
    }
    let vertices: usize = areas
        .iter()
        .map(|a| a.outer.pts.len() + a.holes.iter().map(|h| h.pts.len()).sum::<usize>())
        .sum();
    if vertices > MOST_VERTICES {
        return Err(format!(
            "Çalışma alanının halkalarında {vertices} köşe var; en çok {MOST_VERTICES} köşe alınır."
        ));
    }
    let mut lines = String::new();
    for (k, a) in areas.iter().enumerate() {
        for (r, ring) in std::iter::once(&a.outer).chain(a.holes.iter()).enumerate() {
            for p in &ring.pts {
                let _ = writeln!(lines, "a{k}.{r}	{}	{}", text(p.x, 6), text(p.y, 6));
            }
        }
    }
    let union = union_all(&areas);
    let area = areas_measure(&union);
    let inside = Inside::new(&union).filter(|_| area > 0.0);
    let Some(inside) = inside else {
        return Err("Çalışma alanının alanı sıfır.".into());
    };
    Ok(Study {
        inside,
        area,
        perimeter: crate::geom::space::perimeter(&union),
        parts: areas.len(),
        holes: areas.iter().map(|a| a.holes.len()).sum(),
        skipped,
        lines,
    })
}

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
    nearest_in(shapes, Window::Box(area), sim)
}

/// The index over `window`: the places' box, a box of a given area, or the study area of area objects (docs/adr/0574 §1).
pub fn nearest_in(
    shapes: &[Shape],
    window: Window<'_>,
    sim: Simulation,
) -> Result<StatsRun, String> {
    nearest_edged(shapes, window, sim, Edge::None)
}

/// The index over `window` with the expectation corrected for the region's edge by `edge` (docs/adr/0639 §2).
pub fn nearest_edged(
    shapes: &[Shape],
    window: Window<'_>,
    sim: Simulation,
    edge: Edge,
) -> Result<StatsRun, String> {
    let study = match window {
        Window::Areas(objects) => Some(study_of(objects)?),
        Window::Box(_) => None,
    };
    let area = match window {
        Window::Box(a) => a,
        Window::Areas(_) => None,
    };
    let mut near = Vec::new();
    let placed = Placed::of(shapes, |i| match &study {
        None => true,
        Some(st) => place(&shapes[i]).is_some_and(|p| st.inside.holds(p, &mut near)),
    });
    let mut run = StatsRun::default();
    run.warn_if(placed.unplaced, unplaced_note);
    let n = placed.pts.len();
    let outside = shapes.len() - placed.unplaced - n;
    if let Some(st) = &study {
        run.warn_if(outside, |k| {
            format!("{k} nesnenin yeri çalışma alanının dışında; alınmadı.")
        });
        run.warn_if(st.skipped, |k| {
            format!("{k} çalışma alanı nesnesinde alan yok; alınmadı.")
        });
    }
    if n < 2 {
        return Err(if study.is_some() {
            format!(
                "En yakın komşu çalışma alanında en az iki nesne ister; {n} nesnenin yeri alanın içinde."
            )
        } else {
            format!("En yakın komşu en az iki nesne ister; {n} nesnenin yeri var.")
        });
    }
    let a = match (&study, area) {
        (Some(st), _) => st.area,
        (None, Some(a)) => a,
        (None, None) => {
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
    // The region's perimeter: the study area's rings, else the box's (scaled to the typed area).
    let perimeter = match &study {
        Some(st) => st.perimeter,
        None => {
            let (_, w, h, _) = region(&placed.pts, area);
            2.0 * (w + h)
        }
    };
    let (expected, se) = match edge {
        Edge::None => (0.5 / (nf / a).sqrt(), 0.26136 / (nf * nf / a).sqrt()),
        Edge::Donnelly => (
            0.5 * (a / nf).sqrt() + (0.0514 + 0.041 / nf.sqrt()) * perimeter / nf,
            (0.0703 * a / (nf * nf) + 0.037 * perimeter * (a / (nf * nf * nf * nf * nf)).sqrt())
                .sqrt(),
        ),
    };
    let ratio = observed / expected;
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
    if edge == Edge::Donnelly {
        rows.push(row("Çevre (m)", text(perimeter, 2)));
        rows.push(row("Kenar düzeltmesi", "Donnelly (1978)".to_owned()));
        run.infos.push(
            "Donnelly'nin kenar düzeltmesi: beklenen uzaklık 0,5·√(A/n) + (0,0514 + 0,041/√n)·P/n, standart hata \
             √(0,0703·A/n² + 0,037·P·√(A/n⁵)); P bölgenin çevresi. Dikdörtgene yakın bölgeler için türetildi."
                .to_owned(),
        );
        if n < DONNELLY_FEWEST {
            run.warnings.push(format!(
                "Donnelly'nin düzeltmesi en az {DONNELLY_FEWEST} nesneyle türetildi; {n} nesnede beklenen uzaklık yaklaşıktır."
            ));
        }
    }
    let edged = if edge == Edge::Donnelly {
        "Donnelly kenar düzeltmesiyle "
    } else {
        ""
    };
    if let Some(st) = &study {
        rows.push(row(
            "Çalışma alanı",
            format!("{} alan, {} delik", st.parts, st.holes),
        ));
        rows.push(row("Alanın dışında kalan nesne", outside.to_string()));
    }
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
            "En yakın komşu oranı {} ({edged}z {}, p {}): {}.",
            text(ratio, 4),
            text(z, 4),
            text(p, 6),
            pattern.word()
        );
        return Ok(run);
    }
    // The Monte Carlo test (§1): the same count thrown into the box, point by point, x then y.
    // With a study area the box is the region's own (absolute) and a candidate must fall in or on it (docs/adr/0574 §1).
    let (lo, w, h, words) = match &study {
        None => region(&placed.pts, area),
        Some(st) => {
            let b = st.inside.bounds;
            (
                Vec2::new(b.min_x, b.min_y),
                b.max_x - b.min_x,
                b.max_y - b.min_y,
                "Çalışma alanı nesnesi",
            )
        }
    };
    if study.is_some() {
        let share = a / (w * h);
        if sim.count as f64 * n as f64 / share > MOST_THROWN {
            return Err(format!(
                "Çalışma alanı kutusunun yalnız %{} kadarını kaplıyor; {} benzetim × {n} nesne en çok {MOST_THROWN:.0} aday çeker. Benzetim sayısını azaltın.",
                text(share * 100.0, 3),
                sim.count
            ));
        }
    }
    let bound = (w * w + h * h).sqrt();
    let mut seq = Sequence::new(sim.seed);
    let mut thrown = vec![Vec2::new(0.0, 0.0); n];
    let mut sims = Vec::with_capacity(sim.count);
    let mut larger = 0usize;
    for _ in 0..sim.count {
        for q in thrown.iter_mut() {
            let Some(st) = &study else {
                let x = lo.x + seq.unit() * w;
                let y = lo.y + seq.unit() * h;
                *q = Vec2::new(x, y);
                continue;
            };
            let mut found = None;
            for _ in 0..DRAWS {
                let u = seq.unit();
                let v = seq.unit();
                let p = Vec2::new(lo.x + u * w, lo.y + v * h);
                if st.inside.holds(p, &mut near) {
                    found = Some(p);
                    break;
                }
            }
            let Some(p) = found else {
                return Err(format!(
                    "Çalışma alanında {DRAWS} adayda bir nokta düşmedi: alan kutusunun çok küçük bir parçası."
                ));
            };
            *q = Vec2::new(p.x - placed.origin.x, p.y - placed.origin.y);
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
        format!(
            "{words}, {} m²",
            text(if study.is_some() { a } else { w * h }, 2)
        ),
    );
    profile.push("objects", "Nesne sayısı", n.to_string());
    if edge == Edge::Donnelly {
        profile.push("edge", "Kenar düzeltmesi", "Donnelly (1978)");
    }
    for &i in &placed.index {
        let at = super::place(&shapes[i]).unwrap_or_default();
        place_line(&mut profile.inputs, i, at.x, at.y);
    }
    if let Some(st) = &study {
        profile.inputs.push_str(&st.lines);
    }
    run.profile = Some(profile);
    Ok(run)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::arrangement::Ring;

    fn square(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<Vec2> {
        vec![
            Vec2::new(x0, y0),
            Vec2::new(x1, y0),
            Vec2::new(x1, y1),
            Vec2::new(x0, y1),
        ]
    }

    fn dot(x: f64, y: f64) -> Shape {
        Shape::Point {
            p: Vec2::new(x, y),
            z: None,
            parts: None,
        }
    }

    #[test]
    fn a_study_area_takes_its_net_area_and_leaves_the_hole_out() {
        let area = Shape::Polygon {
            pts: square(0.0, 0.0, 100.0, 100.0),
            bulges: None,
            holes: Some(vec![Ring {
                pts: square(30.0, 30.0, 70.0, 70.0),
                bulges: None,
            }]),
            parts: None,
        };
        let places = [
            dot(10.0, 10.0),
            dot(90.0, 10.0),
            dot(10.0, 90.0),
            dot(50.0, 50.0),  // in the hole
            dot(150.0, 50.0), // beyond the ring
        ];
        let run = nearest_in(
            &places,
            Window::Areas(std::slice::from_ref(&area)),
            Simulation::default(),
        )
        .expect("a run");
        let rows = run.table.expect("a table").rows;
        let row = |k: &str| rows.iter().find(|r| r[0] == k).map(|r| r[1].clone());
        assert_eq!(row("Alan (m²)").as_deref(), Some("8400.00"));
        assert_eq!(row("Nesne sayısı").as_deref(), Some("3"));
        assert_eq!(row("Alanın dışında kalan nesne").as_deref(), Some("2"));
        assert_eq!(row("Çalışma alanı").as_deref(), Some("1 alan, 1 delik"));
        assert_eq!(run.warnings.len(), 1);
    }

    #[test]
    fn objects_that_are_no_areas_are_no_study_area() {
        let places = [dot(0.0, 0.0), dot(1.0, 1.0)];
        let err = nearest_in(
            &places,
            Window::Areas(&[dot(5.0, 5.0)]),
            Simulation::default(),
        )
        .expect_err("a refusal");
        assert!(err.contains("alan yok"), "{err}");
    }
}
