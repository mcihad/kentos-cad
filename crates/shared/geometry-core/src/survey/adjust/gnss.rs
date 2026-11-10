//! GNSS baz ağı dengelemesi (plan/map-cad.md MAP-05, docs/adr/1380): stations in Earth-centred, Earth-fixed X, Y, Z (m)
//! tied by baseline vectors ΔX, ΔY, ΔZ with their full 3×3 covariance. The model is linear: a baseline is the
//! difference of its stations' coordinates. Each baseline's covariance (scaled by the variance factor) is factored
//! L·Lᵀ and its three equations whitened by L⁻¹, so the correlated components go into the shared statistics (vᵀPv, m0,
//! χ², redundancy numbers, Baarda) as three independent unit-weight observations. Known stations are held fixed or
//! weighted by their own standard deviations; with neither, a minimal constraint holds the first station that has
//! coordinates and says so. The normal equations are sparse (docs/adr/4560): the stations in reverse Cuthill–McKee order,
//! the matrix and its factor within the envelope, the inverse's entries the statistics read by the Takahashi equations;
//! a network is refused only when its envelope passes [`MAX_ENVELOPE`] entries.

use std::collections::HashMap;

use super::linalg::Inverse;
use super::sparse::{Envelope, reverse_cuthill_mckee};
use super::{Observed, Statistics, name_key, statistics};
use crate::jsmath::{atan2, cos, js_hypot, js_max, sin};

/// A station: its name, its coordinates when known (m), whether they are held, and when weighted their standard
/// deviations (m) or their full covariance xx, xy, xz, yy, yz, zz (m²; a known point's north, east, up deviations turned
/// into X, Y, Z are correlated, docs/adr/1740).
#[derive(Clone, Debug, PartialEq)]
pub struct GnssStation {
    pub name: String,
    pub xyz: Option<[f64; 3]>,
    pub fixed: bool,
    pub sigma: Option<[f64; 3]>,
    pub cov: Option<[f64; 6]>,
}

/// A baseline from `from` to `to`: ΔX, ΔY, ΔZ (m) and its covariance's upper triangle xx, xy, xz, yy, yz, zz (m²).
#[derive(Clone, Debug, PartialEq)]
pub struct Baseline {
    pub from: String,
    pub to: String,
    pub d: [f64; 3],
    pub cov: [f64; 6],
}

/// How the network's datum is held: by its fixed and weighted stations, or (`minimal`) by holding the first station
/// with coordinates when none is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Datum {
    Given,
    Minimal,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GnssInput {
    pub stations: Vec<GnssStation>,
    pub baselines: Vec<Baseline>,
    /// The variance factor the baselines' covariances are multiplied by (a processor's are often optimistic).
    pub scale: f64,
    pub datum: Datum,
}

/// A station adjusted: X, Y, Z and their standard deviations (scaled by m0), the same in the station's local east,
/// north, up (m), and its east, north, up covariance ee, en, eu, nn, nu, uu (m², by m0²; a confidence region's source,
/// docs/adr/2560).
#[derive(Clone, Debug, PartialEq)]
pub struct AdjustedStation {
    pub name: String,
    pub xyz: [f64; 3],
    pub sxyz: [f64; 3],
    pub senu: [f64; 3],
    pub cenu: [f64; 6],
    pub held: bool,
}

/// A baseline's residual vector (computed − observed, m), its largest component test value and what it says, and the
/// standard deviations of the adjusted vector's components (m, by m0; docs/adr/1740).
#[derive(Clone, Debug, PartialEq)]
pub struct BaselineResidual {
    pub v: [f64; 3],
    pub w: Option<f64>,
    pub flag: &'static str,
    pub adjusted: [f64; 3],
}

#[derive(Clone, Debug, PartialEq)]
pub struct GnssResult {
    pub stations: Vec<AdjustedStation>,
    pub baselines: Vec<BaselineResidual>,
    /// The station the minimal constraint held, when it held one.
    pub held: Option<String>,
    pub stats: Statistics,
}

/// Why a network was not adjusted: a code and the words.
#[derive(Clone, Debug, PartialEq)]
pub struct Refused {
    pub code: &'static str,
    pub message: String,
    pub path: String,
}

fn refuse(code: &'static str, message: String, path: String) -> Refused {
    Refused { code, message, path }
}

/// The most entries the normal matrix's envelope may hold (docs/adr/4560): the factor and the inverse's entries are as
/// many doubles each, the columns' row numbers as many 4-byte numbers (20 bytes an entry, 600 MB in all). The ADR's
/// measurement: a 140 × 140 station grid fits and is adjusted in about 6 s (release build), a 180 × 180 one does not.
pub const MAX_ENVELOPE: usize = 30_000_000;

/// The unknowns' places (docs/adr/4560): the stations not held in reverse Cuthill–McKee order over the baselines, three
/// unknowns each; each row's envelope from its station's earliest neighbour's first unknown. The station's first
/// unknown (none: held), each row's first column and the stations not held.
fn places(fixed: &[bool], ends: &[(usize, usize, [[f64; 3]; 3])]) -> (Vec<Option<usize>>, Vec<usize>, usize) {
    let free: Vec<usize> = (0..fixed.len()).filter(|&i| !fixed[i]).collect();
    let mut node = vec![usize::MAX; fixed.len()];
    for (k, &i) in free.iter().enumerate() {
        node[i] = k;
    }
    let mut neighbours: Vec<Vec<usize>> = vec![Vec::new(); free.len()];
    for (from, to, _) in ends {
        let (a, b) = (node[*from], node[*to]);
        if a != usize::MAX && b != usize::MAX {
            neighbours[a].push(b);
            neighbours[b].push(a);
        }
    }
    for list in &mut neighbours {
        list.sort_unstable();
        list.dedup();
    }
    let order = reverse_cuthill_mckee(&neighbours);
    let mut position = vec![0usize; free.len()];
    for (p, &k) in order.iter().enumerate() {
        position[k] = p;
    }
    let mut slot: Vec<Option<usize>> = vec![None; fixed.len()];
    let mut first = vec![0usize; 3 * free.len()];
    for (p, &k) in order.iter().enumerate() {
        slot[free[k]] = Some(3 * p);
        let earliest = neighbours[k].iter().map(|&w| position[w]).fold(p, usize::min);
        for c in 0..3 {
            first[3 * p + c] = 3 * earliest;
        }
    }
    (slot, first, free.len())
}

/// The station a dense factor in input order names when the network does not reach the datum (docs/adr/0203 §4, kept
/// by docs/adr/4560): the stations joined by baselines fall into parts; a part with no fixed or weighted station floats,
/// and the dense factor fails at the last station of a floating part, the first failure being the part whose last
/// station comes first.
fn floating(input: &GnssInput, ends: &[(usize, usize, [[f64; 3]; 3])], fixed: &[bool]) -> Option<usize> {
    let n = input.stations.len();
    let mut root: Vec<usize> = (0..n).collect();
    fn find(root: &mut [usize], mut v: usize) -> usize {
        while root[v] != v {
            root[v] = root[root[v]];
            v = root[v];
        }
        v
    }
    for (from, to, _) in ends {
        let (a, b) = (find(&mut root, *from), find(&mut root, *to));
        if a != b {
            root[a.max(b)] = a.min(b);
        }
    }
    let mut anchored = vec![false; n];
    let mut last = vec![0usize; n];
    for i in 0..n {
        let r = find(&mut root, i);
        let s = &input.stations[i];
        anchored[r] |= fixed[i] || s.sigma.is_some() || s.cov.is_some();
        last[r] = last[r].max(i);
    }
    (0..n).filter(|&r| find(&mut root, r) == r && !anchored[r]).map(|r| last[r]).min()
}

/// The lower Cholesky factor of a 3×3 covariance (xx, xy, xz, yy, yz, zz), none when it is not positive definite.
pub fn cholesky3(c: &[f64; 6]) -> Option<[[f64; 3]; 3]> {
    let (a, b, d, e, f, g) = (c[0], c[1], c[2], c[3], c[4], c[5]);
    if !(a > 0.0) {
        return None;
    }
    let l00 = a.sqrt();
    let l10 = b / l00;
    let l20 = d / l00;
    let t = e - l10 * l10;
    if !(t > 0.0) {
        return None;
    }
    let l11 = t.sqrt();
    let l21 = (f - l20 * l10) / l11;
    let s = g - l20 * l20 - l21 * l21;
    if !(s > 0.0) {
        return None;
    }
    Some([[l00, 0.0, 0.0], [l10, l11, 0.0], [l20, l21, s.sqrt()]])
}

/// L⁻¹·v for a lower triangular L.
fn whiten(l: &[[f64; 3]; 3], v: [f64; 3]) -> [f64; 3] {
    let y0 = v[0] / l[0][0];
    let y1 = (v[1] - l[1][0] * y0) / l[1][1];
    let y2 = (v[2] - l[2][0] * y0 - l[2][1] * y1) / l[2][2];
    [y0, y1, y2]
}

/// GRS80's latitude and longitude (radians) of an ECEF point, by Bowring's iteration to convergence.
pub fn geodetic(p: [f64; 3]) -> (f64, f64) {
    let a = 6_378_137.0;
    let f = 1.0 / 298.257_222_101;
    let e2 = f * (2.0 - f);
    let lon = atan2(p[1], p[0]);
    let r = js_hypot(p[0], p[1]);
    let mut lat = atan2(p[2], r * (1.0 - e2));
    for _ in 0..10 {
        let s = sin(lat);
        let n = a / (1.0 - e2 * s * s).sqrt();
        let h = r / cos(lat) - n;
        lat = atan2(p[2], r * (1.0 - e2 * n / (n + h)));
    }
    (lat, lon)
}

/// An ellipsoid's ECEF point (m) of a latitude, longitude (radians) and ellipsoidal height (m); `a` the semi-major
/// axis (m), `inv_f` the inverse flattening.
pub fn to_ecef(a: f64, inv_f: f64, lat: f64, lon: f64, h: f64) -> [f64; 3] {
    let f = 1.0 / inv_f;
    let e2 = f * (2.0 - f);
    let (sl, cl) = (sin(lat), cos(lat));
    let n = a / (1.0 - e2 * sl * sl).sqrt();
    [(n + h) * cl * cos(lon), (n + h) * cl * sin(lon), (n * (1.0 - e2) + h) * sl]
}

/// An ECEF point's latitude, longitude (radians) and ellipsoidal height (m) on an ellipsoid: Bowring's iteration to
/// convergence, the height from the normal's foot (exact at any latitude short of the poles' axis).
pub fn to_geodetic(a: f64, inv_f: f64, p: [f64; 3]) -> (f64, f64, f64) {
    let f = 1.0 / inv_f;
    let e2 = f * (2.0 - f);
    let lon = atan2(p[1], p[0]);
    let r = js_hypot(p[0], p[1]);
    let mut lat = atan2(p[2], r * (1.0 - e2));
    for _ in 0..10 {
        let s = sin(lat);
        let n = a / (1.0 - e2 * s * s).sqrt();
        let h = r * cos(lat) + p[2] * s - a * (1.0 - e2 * s * s).sqrt();
        lat = atan2(p[2], r * (1.0 - e2 * n / (n + h)));
    }
    let s = sin(lat);
    let h = r * cos(lat) + p[2] * s - a * (1.0 - e2 * s * s).sqrt();
    (lat, lon, h)
}

/// The rows of the rotation from ECEF to east, north, up at a latitude and longitude.
pub fn enu_rows(lat: f64, lon: f64) -> [[f64; 3]; 3] {
    let (sl, cl) = (sin(lat), cos(lat));
    let (sn, cn) = (sin(lon), cos(lon));
    [[-sn, cn, 0.0], [-sl * cn, -sl * sn, cl], [cl * cn, cl * sn, sl]]
}

/// A whitened equation: its coefficients (unknown, value), right-hand side, kind and the baseline or station it is of.
type Row = (Vec<(usize, f64)>, f64, &'static str, usize);

/// The network adjusted, or why not.
pub fn adjust(input: &GnssInput) -> Result<GnssResult, Refused> {
    if !(input.scale.is_finite() && input.scale > 0.0) {
        return Err(refuse("invalid_input", "Varyans çarpanı sıfırdan büyük bir sayı olmalı.".into(), "scale".into()));
    }
    if input.baselines.is_empty() {
        return Err(refuse("no_baselines", "Baz vektörü yok; en az bir baz verin.".into(), "baselines".into()));
    }
    // The stations by name, each once.
    let mut index: HashMap<String, usize> = HashMap::new();
    for (i, s) in input.stations.iter().enumerate() {
        let key = name_key(&s.name);
        if key.is_empty() {
            return Err(refuse("invalid_input", "İstasyonun adı boş olamaz.".into(), format!("stations[{i}].name")));
        }
        if index.insert(key, i).is_some() {
            return Err(refuse("duplicate_station", format!("“{}” istasyonu iki kez verilmiş.", s.name.trim()), format!("stations[{i}].name")));
        }
        if let Some(x) = s.xyz
            && !x.iter().all(|v| v.is_finite())
        {
            return Err(refuse("invalid_input", "İstasyonun koordinatı sonlu olmalı.".into(), format!("stations[{i}].xyz")));
        }
        if (s.fixed || s.sigma.is_some()) && s.xyz.is_none() {
            return Err(refuse("invalid_input", format!("“{}” sabit ya da ağırlıklı ama koordinatı yok.", s.name.trim()), format!("stations[{i}].xyz")));
        }
        if let Some(sg) = s.sigma
            && !sg.iter().all(|v| v.is_finite() && *v > 0.0)
        {
            return Err(refuse("invalid_input", "Bilinen istasyonun standart sapmaları sıfırdan büyük olmalı.".into(), format!("stations[{i}].sigma")));
        }
        if (s.fixed && (s.sigma.is_some() || s.cov.is_some())) || (s.sigma.is_some() && s.cov.is_some()) {
            return Err(refuse("invalid_input", format!("“{}” hem sabit hem ağırlıklı olamaz.", s.name.trim()), format!("stations[{i}]")));
        }
        if s.cov.is_some() && s.xyz.is_none() {
            return Err(refuse("invalid_input", format!("“{}” ağırlıklı ama koordinatı yok.", s.name.trim()), format!("stations[{i}].xyz")));
        }
        if let Some(c) = s.cov
            && cholesky3(&c).is_none()
        {
            return Err(refuse("invalid_covariance", format!("“{}” istasyonunun kovaryansı pozitif tanımlı değil.", s.name.trim()), format!("stations[{i}].cov")));
        }
    }
    let mut ends: Vec<(usize, usize, [[f64; 3]; 3])> = Vec::new();
    for (k, b) in input.baselines.iter().enumerate() {
        let at = |f: &str| format!("baselines[{k}].{f}");
        let from = *index.get(&name_key(&b.from)).ok_or_else(|| refuse("unknown_station", format!("“{}” istasyonu verilmedi.", b.from.trim()), at("from")))?;
        let to = *index.get(&name_key(&b.to)).ok_or_else(|| refuse("unknown_station", format!("“{}” istasyonu verilmedi.", b.to.trim()), at("to")))?;
        if from == to {
            return Err(refuse("invalid_input", "Bazın iki ucu aynı istasyon.".into(), at("to")));
        }
        if !b.d.iter().chain(b.cov.iter()).all(|v| v.is_finite()) {
            return Err(refuse("invalid_input", "Bazın bileşenleri ve kovaryansı sonlu olmalı.".into(), at("d")));
        }
        let scaled = b.cov.map(|c| c * input.scale);
        let l = cholesky3(&scaled).ok_or_else(|| {
            refuse("invalid_covariance", format!("{}. bazın kovaryansı pozitif tanımlı değil.", k + 1), at("cov"))
        })?;
        ends.push((from, to, l));
    }
    // The datum: fixed and weighted stations, else the minimal constraint.
    let mut fixed: Vec<bool> = input.stations.iter().map(|s| s.fixed).collect();
    let anchored = input.stations.iter().any(|s| s.fixed || s.sigma.is_some() || s.cov.is_some());
    let mut held = None;
    if !anchored {
        match (input.datum, input.stations.iter().position(|s| s.xyz.is_some())) {
            (Datum::Minimal, Some(i)) => {
                fixed[i] = true;
                held = Some(input.stations[i].name.trim().to_owned());
            }
            _ => {
                return Err(refuse(
                    "datum_missing",
                    "Ağın datumu yok: bir istasyonu sabit ya da ağırlıklı verin, ya da datum minimal ile koordinatı olan ilk istasyonu tutun.".into(),
                    "datum".into(),
                ));
            }
        }
    }
    // The unknowns: every station not held, three each, in reverse Cuthill–McKee order (docs/adr/4560).
    let (slot, first, unknown) = places(&fixed, &ends);
    let u = 3 * unknown;
    let entries = Envelope::entries(&first);
    if entries > MAX_ENVELOPE {
        return Err(refuse(
            "network_too_large",
            format!(
                "Ağın normal denklemi seyrek çözümde {entries} girdi tutuyor ({unknown} istasyon); en çok {MAX_ENVELOPE} girdi alınır. Ağı bölgelere ayırıp ayrı dengeleyin."
            ),
            "stations".into(),
        ));
    }
    // Approximate coordinates: a station's own, else carried along the baselines from one that has them; the
    // unknowns are corrections to them, so the normal equations hold small numbers (ECEF coordinates are 10⁶ m).
    let mut approx: Vec<Option<[f64; 3]>> = input.stations.iter().map(|s| s.xyz).collect();
    loop {
        let mut moved = false;
        for (k, (from, to, _)) in ends.iter().enumerate() {
            let d = input.baselines[k].d;
            match (approx[*from], approx[*to]) {
                (Some(a), None) => {
                    approx[*to] = Some([a[0] + d[0], a[1] + d[1], a[2] + d[2]]);
                    moved = true;
                }
                (None, Some(b)) => {
                    approx[*from] = Some([b[0] - d[0], b[1] - d[1], b[2] - d[2]]);
                    moved = true;
                }
                _ => {}
            }
        }
        if !moved {
            break;
        }
    }
    let known = |i: usize| approx[i].unwrap_or([0.0; 3]);
    // Whitened rows: (coefficients, right-hand side) of each component.
    let mut rows: Vec<Row> = Vec::new();
    for (k, (from, to, l)) in ends.iter().enumerate() {
        let b = &input.baselines[k];
        // Each component: (X⁰_to + δ_to) − (X⁰_from + δ_from) = d; a held station's δ is 0.
        let mut rhs = b.d;
        for c in 0..3 {
            rhs[c] -= known(*to)[c] - known(*from)[c];
        }
        let wr = whiten(l, rhs);
        for r in 0..3 {
            // Row r of L⁻¹ applied to the design: L⁻¹ = column of whiten on unit vectors.
            let mut coef = Vec::new();
            for c in 0..3 {
                let mut e = [0.0; 3];
                e[c] = 1.0;
                let li = whiten(l, e)[r];
                if li == 0.0 {
                    continue;
                }
                if let Some(t) = slot[*to] {
                    coef.push((t + c, li));
                }
                if let Some(f) = slot[*from] {
                    coef.push((f + c, -li));
                }
            }
            rows.push((coef, wr[r], "baseline", k));
        }
    }
    for (i, s) in input.stations.iter().enumerate() {
        if let (Some(sg), Some(x), Some(t)) = (s.sigma, s.xyz, slot[i]) {
            for c in 0..3 {
                rows.push((vec![(t + c, 1.0 / sg[c])], (x[c] - known(i)[c]) / sg[c], "station", i));
            }
        }
        // A full covariance: the three pseudo-observations whitened by its Cholesky factor, as a baseline's are.
        if let (Some(c), Some(x), Some(t)) = (s.cov, s.xyz, slot[i])
            && let Some(l) = cholesky3(&c)
        {
            let wr = whiten(&l, [x[0] - known(i)[0], x[1] - known(i)[1], x[2] - known(i)[2]]);
            for r in 0..3 {
                let mut coef = Vec::new();
                for k in 0..3 {
                    let mut e = [0.0; 3];
                    e[k] = 1.0;
                    let li = whiten(&l, e)[r];
                    if li != 0.0 {
                        coef.push((t + k, li));
                    }
                }
                rows.push((coef, wr[r], "station", i));
            }
        }
    }
    let mut n = Envelope::new(first);
    let mut rhs = vec![0.0; u];
    for (coef, l, _, _) in &rows {
        for &(i, ai) in coef {
            rhs[i] += ai * l;
            for &(j, aj) in coef {
                if i >= j {
                    n.add(i, j, ai * aj);
                }
            }
        }
    }
    let factor = n.cholesky().map_err(|bad| {
        // The station the dense factor in input order named (a floating part), else the one whose pivot failed here.
        let station = floating(input, &ends, &fixed)
            .unwrap_or_else(|| slot.iter().position(|s| s.is_some_and(|t| bad >= t && bad < t + 3)).map_or(0, |i| i));
        refuse(
            "network_disconnected",
            format!("“{}” istasyonu datuma bağlı değil: ona ulaşan baz yok ya da ağ parçalı.", input.stations[station].name.trim()),
            "baselines".into(),
        )
    })?;
    let x = factor.solve(&rhs);
    let q = factor.inverse();
    // The misclosures at the solution (observed − computed), unit weight.
    let obs: Vec<Observed> = rows
        .iter()
        .map(|(coef, l, kind, row)| {
            let computed: f64 = coef.iter().map(|&(i, a)| a * x[i]).sum();
            Observed { kind, row: *row, coef: coef.clone(), l: l - computed, sigma: 1.0 }
        })
        .collect();
    let stats = statistics(&obs, &q);
    let s0 = stats.sigma0();
    let coord = |i: usize| -> [f64; 3] {
        let a = known(i);
        match slot[i] {
            Some(t) => [a[0] + x[t], a[1] + x[t + 1], a[2] + x[t + 2]],
            None => a,
        }
    };
    let stations = input
        .stations
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let p = coord(i);
            let (sxyz, senu, cenu) = match slot[i] {
                None => ([0.0; 3], [0.0; 3], [0.0; 6]),
                Some(t) => {
                    let cov = |a: usize, b: usize| q.get(t + a, t + b) * s0 * s0;
                    let (lat, lon) = geodetic(p);
                    let rr = enu_rows(lat, lon);
                    // R·C·Rᵀ: the covariance in east, north, up.
                    let m = |k: usize, l: usize| {
                        let mut v = 0.0;
                        for a in 0..3 {
                            for b in 0..3 {
                                v += rr[k][a] * cov(a, b) * rr[l][b];
                            }
                        }
                        v
                    };
                    let senu = [0, 1, 2].map(|k| js_max(m(k, k), 0.0).sqrt());
                    let cenu = [m(0, 0), m(0, 1), m(0, 2), m(1, 1), m(1, 2), m(2, 2)];
                    ([cov(0, 0).sqrt(), cov(1, 1).sqrt(), cov(2, 2).sqrt()], senu, cenu)
                }
            };
            AdjustedStation { name: s.name.trim().to_owned(), xyz: p, sxyz, senu, cenu, held: fixed[i] }
        })
        .collect();
    let mut baselines = Vec::with_capacity(input.baselines.len());
    for (k, (from, to, _)) in ends.iter().enumerate() {
        let (pf, pt) = (coord(*from), coord(*to));
        let d = input.baselines[k].d;
        let v = [pt[0] - pf[0] - d[0], pt[1] - pf[1] - d[1], pt[2] - pf[2] - d[2]];
        let parts = &stats.observations[3 * k..3 * k + 3];
        let w = parts.iter().filter_map(|o| o.w).fold(None, |m: Option<f64>, w| Some(m.map_or(w, |m| if w > m { w } else { m })));
        let flag = if parts.iter().any(|o| o.flag == "blunder") {
            "blunder"
        } else if parts.iter().all(|o| o.flag == "uncontrolled") {
            "uncontrolled"
        } else {
            "ok"
        };
        // The adjusted vector Δ = X_to − X_from: its covariance is Q_tt + Q_ff − Q_tf − Q_ft (a held end's blocks 0).
        let block = |a: Option<usize>, b: Option<usize>, r: usize, c: usize| match (a, b) {
            (Some(a), Some(b)) => q.get(a + r, b + c),
            _ => 0.0,
        };
        let (st, sf) = (slot[*to], slot[*from]);
        let mut adjusted = [0.0; 3];
        for (c, out) in adjusted.iter_mut().enumerate() {
            let var = block(st, st, c, c) + block(sf, sf, c, c) - block(st, sf, c, c) - block(sf, st, c, c);
            *out = js_max(var * s0 * s0, 0.0).sqrt();
        }
        baselines.push(BaselineResidual { v, w, flag, adjusted });
    }
    Ok(GnssResult { stations, baselines, held, stats })
}

#[cfg(test)]
mod tests {
    use super::super::linalg::Normal;
    use super::*;

    /// A `w` × `w` grid of stations 1 km apart near Ankara (ECEF), each joined to its east, north and north-east
    /// neighbours; a baseline's vector is the true difference plus a few millimetres (a sine of its number), its
    /// covariance correlated (3, 3, 6 mm with xy 0.3 of the product). The first station is fixed, every tenth one is
    /// weighted by 2 cm, the others are unknown.
    fn grid(w: usize) -> GnssInput {
        let base = [4_121_000.0, 2_652_000.0, 4_069_000.0];
        let (east, north) = ([-0.54, 0.84, 0.0], [-0.53, -0.34, 0.78]);
        let at = |r: usize, c: usize| [0, 1, 2].map(|k| base[k] + 1000.0 * (c as f64 * east[k] + r as f64 * north[k]));
        let mut stations = Vec::with_capacity(w * w);
        for r in 0..w {
            for c in 0..w {
                let i = r * w + c;
                let known = i == 0 || i % 10 == 0;
                stations.push(GnssStation {
                    name: format!("S{i}"),
                    xyz: known.then(|| at(r, c)),
                    fixed: i == 0,
                    sigma: (known && i != 0).then_some([0.02; 3]),
                    cov: None,
                });
            }
        }
        let mut baselines = Vec::new();
        for r in 0..w {
            for c in 0..w {
                for (dr, dc) in [(0, 1), (1, 0), (1, 1)] {
                    if r + dr < w && c + dc < w {
                        let (a, b) = (at(r, c), at(r + dr, c + dc));
                        let k = baselines.len() as f64;
                        let noise = [0.003 * (k * 0.7).sin(), 0.003 * (k * 1.3).cos(), 0.006 * (k * 0.9).sin()];
                        baselines.push(Baseline {
                            from: format!("S{}", r * w + c),
                            to: format!("S{}", (r + dr) * w + c + dc),
                            d: [0, 1, 2].map(|q| b[q] - a[q] + noise[q]),
                            cov: [9e-6, 0.3 * 9e-6, 0.0, 9e-6, 0.0, 36e-6],
                        });
                    }
                }
            }
        }
        GnssInput { stations, baselines, scale: 1.0, datum: Datum::Given }
    }

    /// The same network by dense normal equations in input order (ADR 1380's solve, the unknowns corrections to
    /// coordinates carried along the baselines): the coordinates and the X, Y, Z standard deviations, by name.
    fn dense(input: &GnssInput) -> HashMap<String, ([f64; 3], [f64; 3])> {
        let n = input.stations.len();
        let index: HashMap<&str, usize> = input.stations.iter().enumerate().map(|(i, s)| (s.name.as_str(), i)).collect();
        let mut slot = vec![None; n];
        let mut u = 0;
        for (i, s) in input.stations.iter().enumerate() {
            if !s.fixed {
                slot[i] = Some(u);
                u += 3;
            }
        }
        let mut approx: Vec<Option<[f64; 3]>> = input.stations.iter().map(|s| s.xyz).collect();
        let mut grew = true;
        while grew {
            grew = false;
            for b in &input.baselines {
                let (f, t) = (index[b.from.as_str()], index[b.to.as_str()]);
                match (approx[f], approx[t]) {
                    (Some(a), None) => {
                        approx[t] = Some([0, 1, 2].map(|c| a[c] + b.d[c]));
                        grew = true;
                    }
                    (None, Some(z)) => {
                        approx[f] = Some([0, 1, 2].map(|c| z[c] - b.d[c]));
                        grew = true;
                    }
                    _ => {}
                }
            }
        }
        let held = |i: usize| approx[i].unwrap_or([0.0; 3]);
        let mut normal = Normal::new(u);
        let mut rhs = vec![0.0; u];
        let mut rows: Vec<(Vec<(usize, f64)>, f64)> = Vec::new();
        for b in &input.baselines {
            let (f, t) = (index[b.from.as_str()], index[b.to.as_str()]);
            let l = cholesky3(&b.cov).expect("positive definite");
            let mut d = b.d;
            for c in 0..3 {
                d[c] -= held(t)[c] - held(f)[c];
            }
            let wd = whiten(&l, d);
            for r in 0..3 {
                let mut coef = Vec::new();
                for c in 0..3 {
                    let mut e = [0.0; 3];
                    e[c] = 1.0;
                    let li = whiten(&l, e)[r];
                    if let Some(s) = slot[t] {
                        coef.push((s + c, li));
                    }
                    if let Some(s) = slot[f] {
                        coef.push((s + c, -li));
                    }
                }
                rows.push((coef, wd[r]));
            }
        }
        for (i, s) in input.stations.iter().enumerate() {
            if let (Some(sg), Some(x), Some(t)) = (s.sigma, s.xyz, slot[i]) {
                for c in 0..3 {
                    rows.push((vec![(t + c, 1.0 / sg[c])], (x[c] - held(i)[c]) / sg[c]));
                }
            }
        }
        for (coef, l) in &rows {
            for &(i, a) in coef {
                rhs[i] += a * l;
                for &(j, b) in coef {
                    normal.add(i, j, a * b);
                }
            }
        }
        let factor = normal.cholesky().expect("tied to the datum");
        let x = factor.solve(&rhs);
        let q = factor.inverse();
        let mut omega = 0.0;
        for (coef, l) in &rows {
            let v: f64 = coef.iter().map(|&(i, a)| a * x[i]).sum::<f64>() - l;
            omega += v * v;
        }
        let s0 = (omega / (rows.len() - u) as f64).sqrt();
        input
            .stations
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let v = match slot[i] {
                    Some(t) => ([0, 1, 2].map(|c| held(i)[c] + x[t + c]), [0, 1, 2].map(|c| (q.get(t + c, t + c)).sqrt() * s0)),
                    None => (held(i), [0.0; 3]),
                };
                (s.name.clone(), v)
            })
            .collect()
    }

    #[test]
    fn a_grid_network_is_the_dense_solution() {
        let input = grid(14);
        let got = adjust(&input).expect("adjusted");
        let want = dense(&input);
        for s in &got.stations {
            let (xyz, sigma) = want[&s.name];
            for c in 0..3 {
                assert!((s.xyz[c] - xyz[c]).abs() < 1e-9, "{} {c}: {} {}", s.name, s.xyz[c], xyz[c]);
                assert!((s.sxyz[c] - sigma[c]).abs() < 1e-12, "{} σ{c}", s.name);
            }
        }
        assert_eq!(got.stats.u, 3 * (14 * 14 - 1));
    }

    #[test]
    fn a_floating_part_is_named_as_the_dense_factor_named_it() {
        // Two parts float: S3–S5 and S1–S6; the dense factor in input order fails at S5 first (its part ends at S5).
        let mut input = grid(2);
        input.stations.extend(["S4", "S5", "S6"].map(|n| GnssStation { name: n.into(), xyz: None, fixed: false, sigma: None, cov: None }));
        let cov = [9e-6, 0.0, 0.0, 9e-6, 0.0, 36e-6];
        input.baselines.push(Baseline { from: "S4".into(), to: "S5".into(), d: [10.0, 0.0, 0.0], cov });
        input.baselines.push(Baseline { from: "S6".into(), to: "S4".into(), d: [10.0, 0.0, 0.0], cov });
        let refused = adjust(&input).expect_err("floating");
        assert_eq!(refused.code, "network_disconnected");
        assert!(refused.message.starts_with("“S6”"), "{}", refused.message);
        let mut apart = grid(2);
        apart.stations.extend(["S4", "S5", "S6", "S7"].map(|n| GnssStation { name: n.into(), xyz: None, fixed: false, sigma: None, cov: None }));
        apart.baselines.push(Baseline { from: "S7".into(), to: "S4".into(), d: [10.0, 0.0, 0.0], cov });
        apart.baselines.push(Baseline { from: "S5".into(), to: "S6".into(), d: [10.0, 0.0, 0.0], cov });
        let refused = adjust(&apart).expect_err("floating");
        assert!(refused.message.starts_with("“S6”"), "the part S5–S6 ends first: {}", refused.message);
    }

    /// The measurement of docs/adr/4560 (grids of 50 × 50 to 180 × 180 stations: the envelope's entries, the time),
    /// by hand:
    ///
    /// ```text
    /// cargo test --release -p kentos-geometry-core --lib survey::adjust::gnss::tests::large_grids -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore = "a measurement, run by hand"]
    fn large_grids() {
        for w in [50usize, 100, 140, 180] {
            let input = grid(w);
            let fixed: Vec<bool> = input.stations.iter().map(|s| s.fixed).collect();
            let index: HashMap<&str, usize> = input.stations.iter().enumerate().map(|(i, s)| (s.name.as_str(), i)).collect();
            let ends: Vec<(usize, usize, [[f64; 3]; 3])> =
                input.baselines.iter().map(|b| (index[b.from.as_str()], index[b.to.as_str()], [[0.0; 3]; 3])).collect();
            let (_, first, _) = places(&fixed, &ends);
            println!("{w}×{w}: envelope {} entries", Envelope::entries(&first));
            let t = std::time::Instant::now();
            let got = adjust(&input);
            let took = t.elapsed();
            match got {
                Ok(r) => println!("{w}×{w}: {} stations, {} unknowns, m0 {:?}, {:.3} s", w * w, r.stats.u, r.stats.m0, took.as_secs_f64()),
                Err(e) => println!("{w}×{w}: {} ({:.3} s)", e.message, took.as_secs_f64()),
            }
        }
    }
}
