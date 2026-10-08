//! Yatay ağ dengelemesi (docs/adr/0203 §2–§4): known points fixed or
//! weighted, rows of directions and horizontal distances, the new points'
//! approximations from the drawing or from the observations (orientation,
//! polar, free station, forward intersection), Gauss–Newton on the normal
//! equations, the statistics. Y is east (`x`), X north (`y`); a bearing runs
//! clockwise from north; directions are readings in the project's unit.

use std::collections::HashMap;

use super::{
    MAX_ITERATIONS, MAX_UNKNOWNS, Normal, ObservationResult, Observed, Sigmas, name_key, statistics,
};
use crate::api::Op;
use crate::crs::ground::{Grid, grid_factor};
use crate::jsmath::{PI, cos, sin};
use crate::op;
use crate::survey::{Unit, bearing, distance, positive, signed};
use crate::vec2::Vec2;

/// A known point: its name, its place and, when weighted, its standard
/// deviation (m, the same in Y and X); none: fixed.
#[derive(Clone, Debug, PartialEq)]
pub struct KnownPoint {
    pub name: String,
    pub y: f64,
    pub x: f64,
    pub sigma: Option<f64>,
}

crate::json_struct!(KnownPoint { name, y, x, sigma });

/// A new point's place in the drawing: where its approximation starts.
#[derive(Clone, Debug, PartialEq)]
pub struct ApproxPoint {
    pub name: String,
    pub y: f64,
    pub x: f64,
}

crate::json_struct!(ApproxPoint { name, y, x });

/// A row of observations: the station and the target, the direction
/// reading in the project's unit and the horizontal distance (m), either
/// or both; the line of the window's table it is (none: its place, from 1),
/// which a refusal names.
#[derive(Clone, Debug, PartialEq)]
pub struct NetRow {
    pub station: String,
    pub target: String,
    pub direction: Option<f64>,
    pub distance: Option<f64>,
    pub line: Option<usize>,
}

crate::json_struct!(NetRow {
    station,
    target,
    direction,
    distance,
    line
});

/// A horizontal network: the angle unit, the a priori standard deviations,
/// the project's grid when distances are measured on the ground
/// (docs/adr/0171 §4), the known points, the drawing's places of new
/// points, the observations.
#[derive(Clone, Debug, PartialEq)]
pub struct NetworkInput {
    pub unit: String,
    pub sigma: Sigmas,
    pub grid: Option<Grid>,
    pub known: Vec<KnownPoint>,
    pub approx: Vec<ApproxPoint>,
    pub rows: Vec<NetRow>,
}

crate::json_struct!(NetworkInput {
    unit,
    sigma,
    grid,
    known,
    approx,
    rows
});

/// A point adjusted: its place, its standard deviations in Y and X and its
/// position error (m), its error ellipse's semi-axes (m) and its major
/// axis's bearing in [0, π).
#[derive(Clone, Debug, PartialEq)]
pub struct AdjustedPoint {
    pub name: String,
    pub y: f64,
    pub x: f64,
    pub sy: f64,
    pub sx: f64,
    pub sp: f64,
    pub a: f64,
    pub b: f64,
    pub theta: f64,
}

crate::json_struct!(out AdjustedPoint {
    name,
    y,
    x,
    sy,
    sx,
    sp,
    a,
    b,
    theta
});

/// A station's orientation (the bearing of its reading zero), in [0, 2π).
#[derive(Clone, Debug, PartialEq)]
pub struct Orientation {
    pub station: String,
    pub z: f64,
}

crate::json_struct!(out Orientation { station, z });

/// A horizontal network adjusted: its weighted then new points, the
/// stations' orientations, every observation (each row's direction then
/// distance, then the weighted points' Y and X), the flagged observation
/// with the greatest test value, the counts, vᵀPv, m0, the model test and
/// how many iterations it took.
#[derive(Clone, Debug, PartialEq)]
pub struct NetworkResult {
    pub points: Vec<AdjustedPoint>,
    pub orientations: Vec<Orientation>,
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

crate::json_struct!(out NetworkResult {
    points,
    orientations,
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

/// The network's points and observations as the adjustment holds them.
struct Net<'a> {
    input: &'a NetworkInput,
    unit: Unit,
    /// Display names by key.
    names: HashMap<String, String>,
    /// Points by first appearance in the rows (station, then target).
    appearance: Vec<String>,
    /// Stations with directions, by first appearance.
    stations: Vec<String>,
    /// Known points by key: their place and σ; the known table's order.
    known: HashMap<String, (Vec2, Option<f64>)>,
    known_order: Vec<String>,
    /// The rows' keys.
    rows: Vec<(String, String)>,
    pos: HashMap<String, Vec2>,
    orient: HashMap<String, f64>,
}

impl Net<'_> {
    /// Station `s`'s orientation from its directions to placed targets: the
    /// first's, plus the mean of the others' differences from it.
    fn orient_station(&mut self, s: &str) -> bool {
        let Some(&ps) = self.pos.get(s) else {
            return false;
        };
        let mut diffs = Vec::new();
        for (i, row) in self.input.rows.iter().enumerate() {
            let (rs, rt) = &self.rows[i];
            if rs != s {
                continue;
            }
            let (Some(r), Some(&pt)) = (row.direction, self.pos.get(rt)) else {
                continue;
            };
            if !(distance(ps, pt) > 0.0) {
                continue;
            }
            diffs.push(signed(bearing(ps, pt) - self.unit.rad(r)));
        }
        let Some(&z0) = diffs.first() else {
            return false;
        };
        let mean = diffs.iter().map(|d| signed(d - z0)).sum::<f64>() / diffs.len() as f64;
        self.orient.insert(s.to_owned(), z0 + mean);
        true
    }

    /// One round of ADR §3's steps; whether anything was placed or oriented.
    fn round(&mut self) -> bool {
        let mut changed = false;
        // a. Orientation of placed stations.
        for s in self.stations.clone() {
            if self.pos.contains_key(&s) && !self.orient.contains_key(&s) && self.orient_station(&s)
            {
                changed = true;
            }
        }
        // b. Polar points from oriented stations.
        for (i, row) in self.input.rows.iter().enumerate() {
            let (s, t) = &self.rows[i];
            let (Some(r), Some(d)) = (row.direction, row.distance) else {
                continue;
            };
            if self.pos.contains_key(t) {
                continue;
            }
            if let (Some(&ps), Some(&z)) = (self.pos.get(s), self.orient.get(s)) {
                let b = z + self.unit.rad(r);
                self.pos
                    .insert(t.clone(), Vec2::new(ps.x + d * sin(b), ps.y + d * cos(b)));
                changed = true;
            }
        }
        // c. Free stations on two placed targets with directions and distances.
        for s in self.stations.clone() {
            if self.pos.contains_key(&s) {
                continue;
            }
            let got: Vec<(f64, f64, Vec2)> = self
                .input
                .rows
                .iter()
                .enumerate()
                .filter(|(i, _)| self.rows[*i].0 == s)
                .filter_map(|(i, row)| {
                    let k = self.pos.get(&self.rows[i].1)?;
                    Some((self.unit.rad(row.direction?), row.distance?, *k))
                })
                .collect();
            let mut pair = None;
            'find: for i in 0..got.len() {
                for j in i + 1..got.len() {
                    if distance(got[i].2, got[j].2) > 0.0 {
                        pair = Some((got[i], got[j]));
                        break 'find;
                    }
                }
            }
            if let Some(((r1, d1, k1), (r2, d2, k2))) = pair {
                let l1 = Vec2::new(d1 * sin(r1), d1 * cos(r1));
                let l2 = Vec2::new(d2 * sin(r2), d2 * cos(r2));
                let theta = bearing(k1, k2) - bearing(l1, l2);
                let b = r1 + theta;
                self.pos
                    .insert(s.clone(), Vec2::new(k1.x - d1 * sin(b), k1.y - d1 * cos(b)));
                self.orient.insert(s.clone(), theta);
                changed = true;
            }
        }
        // d. Forward intersections of two rays from oriented, placed stations.
        let limit = sin(PI / 200.0);
        for t in self.appearance.clone() {
            if self.pos.contains_key(&t) {
                continue;
            }
            let rays: Vec<(Vec2, f64)> = self
                .input
                .rows
                .iter()
                .enumerate()
                .filter(|(i, _)| self.rows[*i].1 == t)
                .filter_map(|(i, row)| {
                    let s = &self.rows[i].0;
                    Some((
                        *self.pos.get(s)?,
                        self.orient.get(s)? + self.unit.rad(row.direction?),
                    ))
                })
                .collect();
            'rays: for i in 0..rays.len() {
                for j in i + 1..rays.len() {
                    let ((p1, t1), (p2, t2)) = (rays[i], rays[j]);
                    if (sin(t1 - t2)).abs() > limit {
                        let det = sin(t1) * -cos(t2) - cos(t1) * -sin(t2);
                        let (dy, dx) = (p2.x - p1.x, p2.y - p1.y);
                        let u = (dy * -cos(t2) - dx * -sin(t2)) / det;
                        self.pos
                            .insert(t.clone(), Vec2::new(p1.x + u * sin(t1), p1.y + u * cos(t1)));
                        changed = true;
                        break 'rays;
                    }
                }
            }
        }
        changed
    }
}

/// The network adjusted (docs/adr/0203 §2–§4), or why not.
pub fn adjust(input: &NetworkInput) -> Result<NetworkResult, String> {
    let unit = Unit::parse(&input.unit)?;
    let mut net = Net {
        input,
        unit,
        names: HashMap::new(),
        appearance: Vec::new(),
        stations: Vec::new(),
        known: HashMap::new(),
        known_order: Vec::new(),
        rows: Vec::new(),
        pos: HashMap::new(),
        orient: HashMap::new(),
    };
    for k in &input.known {
        let key = name_key(&k.name);
        if net.known.contains_key(&key) {
            return Err(format!("{} bilinen noktalarda iki kez var.", k.name.trim()));
        }
        if !(k.y.is_finite() && k.x.is_finite()) {
            return Err(format!(
                "{} noktasının Y ya da X'i bir sayı değil.",
                k.name.trim()
            ));
        }
        if let Some(s) = k.sigma
            && !(s.is_finite() && s > 0.0)
        {
            return Err(format!(
                "{} noktasının σ'sı sıfırdan büyük olmalı.",
                k.name.trim()
            ));
        }
        net.known
            .insert(key.clone(), (Vec2::new(k.y, k.x), k.sigma));
        net.names.insert(key.clone(), k.name.trim().to_owned());
        net.known_order.push(key.clone());
        net.pos.insert(key, Vec2::new(k.y, k.x));
    }
    if input.rows.is_empty() {
        return Err("Gözlem yok: gözlemler tablosuna doğrultu ya da kenar girin.".to_owned());
    }
    for (i, row) in input.rows.iter().enumerate() {
        let (s, t) = (name_key(&row.station), name_key(&row.target));
        let line = row.line.unwrap_or(i + 1);
        if s == t {
            return Err(format!("{line}. satırda durulan ve bakılan aynı nokta."));
        }
        if let Some(d) = row.distance
            && !(d.is_finite() && d > 0.0)
        {
            return Err(format!("{line}. satırda kenar sıfırdan büyük olmalı."));
        }
        if row.direction.is_some_and(|r| !r.is_finite()) {
            return Err(format!("{line}. satırda doğrultu bir sayı değil."));
        }
        for (key, name) in [(&s, &row.station), (&t, &row.target)] {
            net.names
                .entry(key.clone())
                .or_insert_with(|| name.trim().to_owned());
            if !net.appearance.contains(key) {
                net.appearance.push(key.clone());
            }
        }
        if row.direction.is_some() && !net.stations.contains(&s) {
            net.stations.push(s.clone());
        }
        net.rows.push((s, t));
    }
    for a in &input.approx {
        let key = name_key(&a.name);
        if !net.pos.contains_key(&key) && a.y.is_finite() && a.x.is_finite() {
            net.pos.insert(key, Vec2::new(a.y, a.x));
        }
    }
    while net.round() {}
    let missing: Vec<&str> = net
        .appearance
        .iter()
        .filter(|k| !net.pos.contains_key(*k))
        .map(|k| net.names[k].as_str())
        .collect();
    if !missing.is_empty() {
        return Err(format!(
            "Yaklaşık yeri bulunamayan nokta: {}. Noktayı çizime yaklaşık yeriyle ekleyin ya da onu belirleyen doğrultu ve kenar ölçüleri girin.",
            missing.join(", ")
        ));
    }
    for s in net.stations.clone() {
        if !net.orient.contains_key(&s) {
            net.orient_station(&s);
        }
    }
    let weighted: Vec<String> = net
        .known_order
        .iter()
        .filter(|k| net.known[*k].1.is_some())
        .cloned()
        .collect();
    let points: Vec<String> = weighted
        .iter()
        .cloned()
        .chain(
            net.appearance
                .iter()
                .filter(|k| !net.known.contains_key(*k))
                .cloned(),
        )
        .collect();
    let col: HashMap<String, usize> = points
        .iter()
        .enumerate()
        .map(|(i, k)| (k.clone(), 2 * i))
        .collect();
    let ocol: HashMap<String, usize> = net
        .stations
        .iter()
        .enumerate()
        .map(|(j, s)| (s.clone(), 2 * points.len() + j))
        .collect();
    let u = 2 * points.len() + net.stations.len();
    if u > MAX_UNKNOWNS {
        return Err(format!(
            "Ağ çok büyük: {u} bilinmeyen; en çok {MAX_UNKNOWNS} bilinmeyen dengelenir."
        ));
    }
    let unknown_name = |c: usize| {
        if c < 2 * points.len() {
            let name = &net.names[&points[c / 2]];
            if c.is_multiple_of(2) {
                format!("{name} noktasının Y'si")
            } else {
                format!("{name} noktasının X'i")
            }
        } else {
            format!(
                "{} istasyonunun yöneltmesi",
                net.names[&net.stations[c - 2 * points.len()]]
            )
        }
    };
    let observations = |net: &Net<'_>| -> Result<Vec<Observed>, String> {
        let mut out = Vec::new();
        for (i, row) in input.rows.iter().enumerate() {
            let (s, t) = &net.rows[i];
            let (ps, pt) = (net.pos[s], net.pos[t]);
            let (dy, dx) = (pt.x - ps.x, pt.y - ps.y);
            let s2 = dy * dy + dx * dx;
            let length = s2.sqrt();
            if let Some(r) = row.direction {
                let mut coef = Vec::with_capacity(5);
                if let Some(&c) = col.get(s) {
                    coef.push((c, -dx / s2));
                    coef.push((c + 1, dy / s2));
                }
                if let Some(&c) = col.get(t) {
                    coef.push((c, dx / s2));
                    coef.push((c + 1, -dy / s2));
                }
                coef.push((ocol[s], -1.0));
                let l = signed(unit.rad(r) - (bearing(ps, pt) - net.orient[s]));
                let c = input.sigma.centering / length;
                let sigma = (input.sigma.direction * input.sigma.direction + 2.0 * c * c).sqrt();
                out.push(Observed {
                    kind: "direction",
                    row: i,
                    coef,
                    l,
                    sigma,
                });
            }
            if let Some(d) = row.distance {
                let mut coef = Vec::with_capacity(4);
                if let Some(&c) = col.get(s) {
                    coef.push((c, -dy / length));
                    coef.push((c + 1, -dx / length));
                }
                if let Some(&c) = col.get(t) {
                    coef.push((c, dy / length));
                    coef.push((c + 1, dx / length));
                }
                let factor = match &input.grid {
                    Some(grid) => grid_factor(grid, ps, pt)?.combined(),
                    None => 1.0,
                };
                out.push(Observed {
                    kind: "distance",
                    row: i,
                    coef,
                    l: d * factor - length,
                    sigma: input.sigma.distance_at(d),
                });
            }
        }
        for k in &weighted {
            let (p0, sigma) = net.known[k];
            let sigma = sigma.unwrap_or(1.0);
            let row = net.known_order.iter().position(|o| o == k).unwrap_or(0);
            let p = net.pos[k];
            out.push(Observed {
                kind: "y",
                row,
                coef: vec![(col[k], 1.0)],
                l: p0.x - p.x,
                sigma,
            });
            out.push(Observed {
                kind: "x",
                row,
                coef: vec![(col[k] + 1, 1.0)],
                l: p0.y - p.y,
                sigma,
            });
        }
        Ok(out)
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
            "Ağın dayanağı ya da gözlemleri yetersiz: {} belirlenemiyor. En az iki sabit ya da ağırlıklı nokta ve her noktayı belirleyen gözlemler gerekir.",
            unknown_name(k)
        )
    };
    let mut iterations = 0;
    loop {
        iterations += 1;
        if iterations > MAX_ITERATIONS {
            return Err(format!(
                "Dengeleme {MAX_ITERATIONS} yinelemede yakınsamadı: yaklaşık koordinatları ve gözlemleri denetleyin."
            ));
        }
        let obs = observations(&net)?;
        let (n, rhs) = normal(&obs);
        let factor = n.cholesky().map_err(singular)?;
        let delta = factor.solve(&rhs);
        let mut coords: f64 = 0.0;
        for k in &points {
            let c = col[k];
            let p = net.pos[k];
            net.pos
                .insert(k.clone(), Vec2::new(p.x + delta[c], p.y + delta[c + 1]));
            coords = crate::jsmath::js_max(
                coords,
                crate::jsmath::js_max(delta[c].abs(), delta[c + 1].abs()),
            );
        }
        let mut angles: f64 = 0.0;
        for s in &net.stations {
            let c = ocol[s];
            *net.orient.entry(s.clone()).or_insert(0.0) += delta[c];
            angles = crate::jsmath::js_max(angles, delta[c].abs());
        }
        if coords <= 1e-7 && angles <= 1e-10 {
            break;
        }
    }
    let obs = observations(&net)?;
    let (n, _) = normal(&obs);
    let q = n.cholesky().map_err(singular)?.inverse();
    let stats = statistics(&obs, &q);
    let s0 = stats.sigma0();
    let adjusted = points
        .iter()
        .map(|k| {
            let c = col[k];
            let (qyy, qxx, qyx) = (q.get(c, c), q.get(c + 1, c + 1), q.get(c, c + 1));
            let half = (qyy + qxx) / 2.0;
            let root = (((qyy - qxx) / 2.0) * ((qyy - qxx) / 2.0) + qyx * qyx).sqrt();
            let mut theta = crate::jsmath::atan2(2.0 * qyx, qxx - qyy) / 2.0 % PI;
            if theta < 0.0 {
                theta += PI;
            }
            let (sy, sx) = (s0 * qyy.sqrt(), s0 * qxx.sqrt());
            let p = net.pos[k];
            AdjustedPoint {
                name: net.names[k].clone(),
                y: p.x,
                x: p.y,
                sy,
                sx,
                sp: (sy * sy + sx * sx).sqrt(),
                a: s0 * (half + root).sqrt(),
                b: s0 * crate::jsmath::js_max(half - root, 0.0).sqrt(),
                theta,
            }
        })
        .collect();
    let orientations = net
        .stations
        .iter()
        .map(|s| Orientation {
            station: net.names[s].clone(),
            z: positive(net.orient[s]),
        })
        .collect();
    Ok(NetworkResult {
        points: adjusted,
        orientations,
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

pub(crate) const OP: Op = op!("networkAdjust", |input: NetworkInput| adjust(&input));
