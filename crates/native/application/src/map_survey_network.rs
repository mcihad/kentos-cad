//! `map.survey.network` v1 (plan/map-cad.md MAP-03, MAP-05; docs/adr/1740): a map project's GNSS network adjusted from
//! its archive, reading only. The baselines that go into the adjustment and the known points on their marks make the
//! network. The shared core (`survey::adjust::gnss`) adjusts it twice: free, held by one known point alone, for the
//! blunders and the network's own precision (BÖHHBÜY Madde 15(1)(b)); then held by all its known points, fixed or
//! weighted, for the marks' coordinates (15(1)(ç)). The marks that are not known points are moved into the project by
//! the coordinate chain and compared with their accepted points. A rule class (KentOS's package: BÖHHBÜY 2018's C1,
//! C2, C3) checks the baselines' and the stations' standard deviations at its confidence. `map.survey.edit`'s
//! `network` writes what this reads. The shared cases are `fixtures/commands/v1/map.survey.network.json`.
//!
//! The measurement epoch (docs/adr/2120): given, or the mean of the baselines' observation mid-times; the known points
//! are moved to it (BÖHHBÜY Madde 15(1)(a)): by their own velocity, linearly in the frame's Earth-centred X, Y, Z
//! (docs/adr/2520; IERS Conventions 2010 §4.1), else along the campaign's velocity grid by the coordinate chain itself.
//!
//! With three known points or more the given coordinates and the free stage's are related by a 3D similarity and its
//! scale tested (Madde 15(1)(c), docs/adr/2160).

use std::collections::{HashMap, HashSet};

use kentos_commands::{
    BaselineSigmaOf, BaselineTest, CommandError, CommandResult, CommandWarning, CrsPoint, CrsSide, CrsTransform,
    ErrorEllipse, GnssRuleClass, GnssRules, HeightKind, HeightReference, KnownMoved, KnownMovedBy, MapSurvey,
    MapSurveyNetwork, MapSurveyNetworked, NetworkBaseline, NetworkEpochFrom, NetworkRules, NetworkSimilarity,
    NetworkStage, NetworkStation, SimilarityResidual, SurveyBaseline, SurveyCampaign, SurveyKnown, VelocityModel,
    gnss_rules_default,
};
use kentos_domain::Document;
use kentos_geometry_core::survey::adjust::gnss::{self, Baseline, Datum, GnssInput, GnssResult, GnssStation};
use kentos_geometry_core::survey::adjust::Normal;
use kentos_geometry_core::survey::adjust::{W_LIMIT, chi2_quantile, t_quantile};

use crate::ExecutionContext;
use crate::checks::Stop;
use crate::codes;
use crate::map_survey::{Placed, baseline_length, fail, map_project, place};

/// A mark that is not a known point, as the stage held by the known points leaves it: its latitude, longitude
/// (degrees), ellipsoidal height (m), standard deviations north, east, up (m), and its place in the project or why the
/// chain did not reach it.
pub(crate) struct NewMark {
    pub mark: String,
    pub lat: f64,
    pub lon: f64,
    pub height: f64,
    pub sigma: [f64; 3],
    pub placed: Result<Placed, CommandError>,
}

/// What the adjustment read and said, for the answer and for the write: the baselines and known points it rests on,
/// the one the free stage held, the variance factor, the class and its package, and the new marks.
pub(crate) struct Analysis {
    pub out: MapSurveyNetworked,
    pub baselines: Vec<String>,
    pub known: Vec<String>,
    pub hold: String,
    pub scale: f64,
    pub class: Option<(String, String)>,
    pub new: Vec<NewMark>,
}

const RAD: f64 = 180.0 / std::f64::consts::PI;

/// The ellipsoid (semi-major axis, inverse flattening) of a geographic frame of the registry.
fn ellipsoid_of(srid: u32) -> Option<(f64, f64)> {
    let system = kentos_project::crs::system(srid).filter(|s| s.kind == "geographic")?;
    match system.transform_system()? {
        kentos_geometry_core::crs::System::Geographic { datum } => Some(datum.ellipsoid()),
        _ => None,
    }
}

/// Why a rule package's numbers cannot be used.
fn rules_problem(r: &GnssRules) -> Option<&'static str> {
    if r.id.trim().is_empty() || r.source.trim().is_empty() || r.classes.is_empty() {
        return Some("Kural paketinin kimliği, kaynağı ve en az bir sınıfı olmalı.");
    }
    let mut ids = HashSet::new();
    for c in &r.classes {
        if c.id.trim().is_empty() || !ids.insert(c.id.as_str()) {
            return Some("Kural paketinin sınıflarının kimlikleri boş olmayan ve tek olmalı.");
        }
        let limit = [c.constant, c.ppm];
        if !limit.iter().all(|v| v.is_finite() && *v >= 0.0) || limit.iter().all(|v| *v == 0.0) {
            return Some("Sınıfın baz sınırı eksi olmayan sonlu iki sayıdır (m ve ppm), ikisi birden sıfır olamaz.");
        }
        if !(c.horizontal.is_finite() && c.horizontal > 0.0 && c.height.is_finite() && c.height > 0.0) {
            return Some("Sınıfın nokta sınırları 0'dan büyük sonlu sayılar olmalı (m).");
        }
        if c.confidence.is_some_and(|p| !(p.is_finite() && p > 0.0 && p < 1.0)) {
            return Some("Sınıfın güveni 0 ile 1 arasında olmalı.");
        }
        if c.similarity.is_some_and(|v| !(v.is_finite() && v > 0.0)) {
            return Some("Sınıfın benzerlik ölçeği sınırı 0'dan büyük sonlu bir sayı olmalı (ppm).");
        }
    }
    None
}

/// The confidence the similarity's scale is tested at when the class gives none: BÖHHBÜY Madde 15(1)(d)'s 1 − α = 0.95.
const SIMILARITY_CONFIDENCE: f64 = 0.95;

/// Arc-seconds in a radian.
const ARCSEC: f64 = 180.0 * 3600.0 / std::f64::consts::PI;

/// The 3D similarity given = T + λ·R·free of three known points or more (BÖHHBÜY Madde 15(1)(c), docs/adr/2160): least
/// squares with unit weights by Gauss–Newton from λ = 1, R = I, each step linearised about the transformed points
/// reduced to their mean (map coordinates lose no digits) and R kept a rotation (Rodrigues), until the steps vanish;
/// the rotations reported are R's antisymmetric part (position vector convention). σ0² = vᵀv/(3n − 7), the scale's σ
/// from the linearised normals on the free points reduced to their mean; t = |λ − 1|/σ against Student's t at
/// (1 + confidence)/2. `points` are (id, mark, given, free, given latitude and longitude).
fn similarity(points: &[(String, String, [f64; 3], [f64; 3], f64, f64)], confidence: f64) -> Option<NetworkSimilarity> {
    let n = points.len();
    if n < 3 {
        return None;
    }
    let mean = |v: &dyn Fn(usize) -> [f64; 3]| {
        let mut s = [0.0; 3];
        for i in 0..n {
            let x = v(i);
            for k in 0..3 {
                s[k] += x[k];
            }
        }
        s.map(|x| x / n as f64)
    };
    let rows = |x: [f64; 3]| -> [[f64; 7]; 3] {
        [
            [1.0, 0.0, 0.0, x[0], 0.0, x[2], -x[1]],
            [0.0, 1.0, 0.0, x[1], -x[2], 0.0, x[0]],
            [0.0, 0.0, 1.0, x[2], x[1], -x[0], 0.0],
        ]
    };
    let mul = |r: &[[f64; 3]; 3], x: [f64; 3]| [0, 1, 2].map(|i| r[i][0] * x[0] + r[i][1] * x[1] + r[i][2] * x[2]);
    // λ and R, then T = mean(given) − λ·R·mean(free).
    let (mut lambda, mut r) = (1.0f64, [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]);
    let gm = mean(&|i| points[i].2);
    let fm = mean(&|i| points[i].3);
    for _ in 0..20 {
        // The free points as the current transformation puts them, reduced to their mean.
        let moved: Vec<[f64; 3]> = points.iter().map(|p| mul(&r, [p.3[0] - fm[0], p.3[1] - fm[1], p.3[2] - fm[2]]).map(|v| lambda * v)).collect();
        let mut normal = Normal::new(7);
        let mut b = [0.0; 7];
        for (p, x) in points.iter().zip(&moved) {
            let a = rows(*x);
            for (k, row) in a.iter().enumerate() {
                let l = (p.2[k] - gm[k]) - x[k];
                for i in 0..7 {
                    b[i] += row[i] * l;
                    for j in 0..7 {
                        normal.add(i, j, row[i] * row[j]);
                    }
                }
            }
        }
        let step = normal.cholesky().ok()?.solve(&b);
        let (dd, w) = (step[3], [step[4], step[5], step[6]]);
        // R ← exp([w]×)·R by Rodrigues; λ ← λ·(1 + dd).
        let angle = (w[0] * w[0] + w[1] * w[1] + w[2] * w[2]).sqrt();
        let turn = if angle > 0.0 {
            let k = w.map(|v| v / angle);
            let (s, c) = angle.sin_cos();
            let kk = [[0.0, -k[2], k[1]], [k[2], 0.0, -k[0]], [-k[1], k[0], 0.0]];
            let mut m = [[0.0; 3]; 3];
            for i in 0..3 {
                for j in 0..3 {
                    let k2 = (0..3).map(|q| kk[i][q] * kk[q][j]).fold(0.0, |a, b| a + b);
                    m[i][j] = f64::from(u8::from(i == j)) + s * kk[i][j] + (1.0 - c) * k2;
                }
            }
            m
        } else {
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
        };
        let mut next = [[0.0; 3]; 3];
        for i in 0..3 {
            for j in 0..3 {
                next[i][j] = (0..3).map(|q| turn[i][q] * r[q][j]).fold(0.0, |a, b| a + b);
            }
        }
        r = next;
        lambda *= 1.0 + dd;
        if dd.abs() < 1e-15 && angle < 1e-15 {
            break;
        }
    }
    let rf = mul(&r, fm);
    let translation = [0, 1, 2].map(|k| gm[k] - lambda * rf[k]);
    // The residuals, given − transformed, east, north, up at the given places.
    let mut vv = 0.0;
    let mut residuals = Vec::with_capacity(n);
    for p in points {
        let x = mul(&r, p.3);
        let v: [f64; 3] = [0, 1, 2].map(|k| p.2[k] - (translation[k] + lambda * x[k]));
        vv += v[0] * v[0] + v[1] * v[1] + v[2] * v[2];
        let e = gnss::enu_rows(p.4 / RAD, p.5 / RAD);
        let enu = [0, 1, 2].map(|i| e[i][0] * v[0] + e[i][1] * v[1] + e[i][2] * v[2]);
        residuals.push(SimilarityResidual { id: p.0.clone(), mark: p.1.clone(), enu });
    }
    // The scale's σ from the linearised normals on the free points reduced to their mean.
    let mut normal = Normal::new(7);
    for p in points {
        let a = rows([p.3[0] - fm[0], p.3[1] - fm[1], p.3[2] - fm[2]]);
        for row in &a {
            for i in 0..7 {
                for j in 0..7 {
                    normal.add(i, j, row[i] * row[j]);
                }
            }
        }
    }
    let q = normal.cholesky().ok()?.inverse();
    let dof = 3 * n - 7;
    let s0 = (vv / dof as f64).sqrt();
    let sigma = s0 * q.get(3, 3).sqrt();
    let d = lambda - 1.0;
    let t = if sigma > 0.0 { d.abs() / sigma } else { 0.0 };
    let quantile = t_quantile(dof as f64, (1.0 + confidence) / 2.0);
    let rotation = [(r[2][1] - r[1][2]) / 2.0, (r[0][2] - r[2][0]) / 2.0, (r[1][0] - r[0][1]) / 2.0].map(|v| v * ARCSEC);
    Some(NetworkSimilarity {
        known: points.iter().map(|p| p.0.clone()).collect(),
        translation,
        rotation,
        scale: d * 1e6,
        sigma: sigma * 1e6,
        dof: u32::try_from(dof).unwrap_or(u32::MAX),
        t,
        quantile,
        agrees: t <= quantile,
        residuals,
    })
}

/// The confidence a region is drawn at without a class's (BÖHHBÜY Madde 15(1)(d)).
const CONFIDENCE: f64 = 0.95;

/// A mark's confidence region at `p` from its east, north, up covariance (ee, en, eu, nn, nu, uu; docs/adr/2560): the
/// horizontal ellipse's semi-axes, √χ²₂(p) times the roots of the 2×2's eigenvalues, the major axis's direction from
/// north clockwise, ½·atan2(2σne, σnn − σee) in [0°, 180°) (none for a circle: the axes within a millionth of the major
/// one, where the direction is rounding), and the height's half-interval √χ²₁(p)·σu.
fn error_ellipse(c: [f64; 6], p: f64) -> ErrorEllipse {
    let (ee, en, nn, uu) = (c[0], c[1], c[3], c[5]);
    let mid = (nn + ee) / 2.0;
    let half = ((nn - ee) / 2.0).hypot(en);
    let (k2, k1) = (chi2_quantile(2.0, p).sqrt(), chi2_quantile(1.0, p).sqrt());
    let mut azimuth = 0.5 * (2.0 * en).atan2(nn - ee) * RAD;
    if azimuth < 0.0 {
        azimuth += 180.0;
    }
    if azimuth >= 180.0 {
        azimuth -= 180.0;
    }
    let (major, minor) = (k2 * (mid + half).max(0.0).sqrt(), k2 * (mid - half).max(0.0).sqrt());
    ErrorEllipse {
        confidence: p,
        major,
        minor,
        azimuth: (major - minor > 1e-6 * major).then_some(azimuth),
        up: k1 * uu.max(0.0).sqrt(),
    }
}

/// The test value above which a component is a blunder: two-sided at the class's confidence (√χ²₁), else Baarda's.
fn w_limit(class: Option<&GnssRuleClass>) -> f64 {
    match class.and_then(|c| c.confidence) {
        Some(p) => chi2_quantile(1.0, p).sqrt(),
        None => W_LIMIT,
    }
}

/// A refusal of what the archive holds (no input field is wrong): without a path in the query, the operation's in an
/// edit (`prefix` “operation.”).
fn held_wrong(code: &str, message: String, prefix: &str) -> Stop {
    if prefix.is_empty() {
        Stop::failed(crate::checks::error(code, message, None))
    } else {
        fail(code, message, "operation")
    }
}

/// A place's X, Y, Z (m) on a frame's ellipsoid from its latitude, longitude (degrees) and height (m).
fn ecef((lat, lon, h): (f64, f64, f64), (a, inv_f): (f64, f64)) -> [f64; 3] {
    gnss::to_ecef(a, inv_f, lat / RAD, lon / RAD, h)
}

/// A weighted known point's north, east, up deviations as an X, Y, Z covariance: Rᵀ·diag(σe², σn², σu²)·R, R the
/// rows east, north, up at its place.
fn weight((lat, lon, _): (f64, f64, f64), sigma: [f64; 3]) -> [f64; 6] {
    let r = gnss::enu_rows(lat / RAD, lon / RAD);
    let var = [sigma[1] * sigma[1], sigma[0] * sigma[0], sigma[2] * sigma[2]];
    let c = |i: usize, j: usize| (0..3).map(|k| r[k][i] * var[k] * r[k][j]).sum::<f64>();
    [c(0, 0), c(0, 1), c(0, 2), c(1, 1), c(1, 2), c(2, 2)]
}

/// A GPS time `YYYY-MM-DDTHH:MM:SS` (seconds may have a fraction; a trailing `Z` allowed) as a decimal year: the year
/// and the share of its seconds gone (docs/adr/2120 §1); none when it does not read.
pub(crate) fn decimal_year(t: &str) -> Option<f64> {
    let t = t.strip_suffix('Z').unwrap_or(t);
    let (date, time) = t.split_once('T')?;
    let mut d = date.split('-');
    let (y, mo, da) = (d.next()?.parse::<i64>().ok()?, d.next()?.parse::<i64>().ok()?, d.next()?.parse::<i64>().ok()?);
    let mut c = time.split(':');
    let (h, mi, s) = (c.next()?.parse::<i64>().ok()?, c.next()?.parse::<i64>().ok()?, c.next()?.parse::<f64>().ok()?);
    if d.next().is_some() || c.next().is_some() || !(1..=12).contains(&mo) || !(0..24).contains(&h) || !(0..60).contains(&mi) {
        return None;
    }
    let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
    let days = [31, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    if !(1..=days[(mo - 1) as usize]).contains(&da) || !(0.0..60.0).contains(&s) {
        return None;
    }
    let before: i64 = days[..(mo - 1) as usize].iter().sum::<i64>() + da - 1;
    let seconds = (before * 86_400 + h * 3_600 + mi * 60) as f64 + s;
    let year = (if leap { 366.0 } else { 365.0 }) * 86_400.0;
    Some(y as f64 + seconds / year)
}

/// The measurement epoch and where it comes from (docs/adr/2120 §1): `given`, else the mean of the baselines'
/// observation mid-times when every one has its start and end; none otherwise.
fn measurement_epoch(given: Option<f64>, used: &[&SurveyBaseline]) -> Option<(f64, NetworkEpochFrom)> {
    if let Some(e) = given {
        return Some((e, NetworkEpochFrom::Given));
    }
    let mut sum = 0.0;
    for b in used {
        let start = decimal_year(b.start.as_deref()?)?;
        let end = decimal_year(b.end.as_deref()?)?;
        sum += (start + end) / 2.0;
    }
    Some((sum / used.len() as f64, NetworkEpochFrom::Baselines))
}

/// A known point's latitude, longitude and ellipsoidal height moved from its epoch to the measurement epoch along the
/// campaign's velocity grid, by the coordinate chain itself (cad.crs.transform, docs/adr/0631 §2); the chain's own
/// refusal (a grid not on this device …) kept, its path the operation's.
fn moved_to(
    doc: &Document,
    grid: &VelocityModel,
    (lat, lon, h): (f64, f64, f64),
    srid: u32,
    from: f64,
    to: f64,
) -> Result<(f64, f64, f64), Stop> {
    let ellipsoidal = || Some(HeightReference { kind: HeightKind::Ellipsoidal, vertical_srid: None, geoid: None });
    let input = CrsTransform {
        points: vec![CrsPoint { x: lon, y: lat, z: Some(h) }],
        from: Some(CrsSide { srid: Some(srid), heights: ellipsoidal(), epoch: Some(from) }),
        to: Some(CrsSide { srid: Some(srid), heights: ellipsoidal(), epoch: Some(to) }),
        uncertainty: None,
        velocity: Some(grid.clone()),
    };
    match crate::crs_transform::answer(doc, &input) {
        CommandResult::Completed { output, .. } => match output.points.first() {
            Some(Some(p)) => Ok((p.y, p.x, p.z.unwrap_or(h))),
            _ => {
                let why = output.unreached.first().map_or("", |u| crate::crs_transform::reason_text(&u.reason));
                Err(fail(codes::EPOCH_UNSUPPORTED, format!("Bilinen nokta ölçme epokuna taşınamadı: {why}"), "operation"))
            }
        },
        CommandResult::Failed { mut error } => {
            error.path = Some("operation".into());
            Err(Stop::failed(error))
        }
        other => Err(fail(codes::INVALID_SURVEY_VALUE, format!("{other:?}"), "operation")),
    }
}

/// A known point's latitude, longitude (degrees) and ellipsoidal height carried `years` by its own velocity
/// (docs/adr/2520): linearly in the frame's Earth-centred X, Y, Z, X(t) = X(t₀) + V·(t − t₀) (IERS Conventions 2010
/// §4.1), on the frame's ellipsoid both ways.
fn by_velocity(given: (f64, f64, f64), shape: (f64, f64), v: [f64; 3], years: f64) -> (f64, f64, f64) {
    let p = ecef(given, shape);
    let q = [p[0] + v[0] * years, p[1] + v[1] * years, p[2] + v[2] * years];
    let (lat, lon, h) = gnss::to_geodetic(shape.0, shape.1, q);
    (lat * RAD, lon * RAD, h)
}

/// A residual vector (ECEF, m) in the local east, north, up of a place.
fn enu(at: [f64; 3], v: [f64; 3]) -> [f64; 3] {
    let (lat, lon) = gnss::geodetic(at);
    let r = gnss::enu_rows(lat, lon);
    [0, 1, 2].map(|k| r[k][0] * v[0] + r[k][1] * v[1] + r[k][2] * v[2])
}

/// A stage's summary and each baseline's test at `limit`; `froms` is each baseline's start among the stations,
/// `known_ids` names the stations that are known points.
fn stage(
    r: &GnssResult,
    ids: &[String],
    froms: &[usize],
    known_ids: &[Option<String>],
    limit: f64,
    held: Vec<String>,
) -> (NetworkStage, Vec<BaselineTest>) {
    let obs = &r.stats.observations;
    let mut tests = Vec::with_capacity(ids.len());
    let mut worst: Option<(f64, String)> = None;
    let mut blunders = 0u32;
    let mut note = |w: f64, id: &str, worst: &mut Option<(f64, String)>| {
        blunders += 1;
        if worst.as_ref().is_none_or(|(m, _)| w > *m) {
            *worst = Some((w, id.to_owned()));
        }
    };
    for (k, id) in ids.iter().enumerate() {
        let parts = &obs[3 * k..3 * k + 3];
        let w = parts.iter().filter_map(|o| o.w).fold(None, |m: Option<f64>, w| Some(m.map_or(w, |m| if w > m { w } else { m })));
        let flag = if parts.iter().all(|o| o.flag == "uncontrolled") {
            "uncontrolled"
        } else if w.is_some_and(|w| w > limit) {
            "blunder"
        } else {
            "ok"
        };
        if flag == "blunder" {
            note(w.unwrap_or(0.0), id, &mut worst);
        }
        let at = r.stations[froms[k]].xyz;
        tests.push(BaselineTest { v: enu(at, r.baselines[k].v), w, flag: flag.into() });
    }
    // A weighted known point's pseudo-observations: a known point the network does not agree with.
    let mut i = 3 * ids.len();
    while i < obs.len() {
        let row = obs[i].row;
        let mut w: Option<f64> = None;
        while i < obs.len() && obs[i].row == row {
            if let Some(x) = obs[i].w {
                w = Some(w.map_or(x, |m| if x > m { x } else { m }));
            }
            i += 1;
        }
        if let Some(w) = w.filter(|w| *w > limit) {
            let id = known_ids.get(row).cloned().flatten().unwrap_or_default();
            note(w, &id, &mut worst);
        }
    }
    let s = &r.stats;
    (
        NetworkStage {
            held,
            m0: s.m0,
            dof: u32::try_from(s.f).unwrap_or(u32::MAX),
            omega: s.omega,
            chi2: s.chi2,
            passed: s.passed,
            limit,
            blunders,
            worst: worst.map(|(_, id)| id),
        },
        tests,
    )
}

/// The network of `input` adjusted, or why it cannot be. `prefix` is the input's path (“operation.” in an edit).
#[allow(clippy::too_many_lines)]
pub(crate) fn analyse(doc: &Document, input: &MapSurveyNetwork, prefix: &str) -> Result<Analysis, Stop> {
    map_project(doc)?;
    let empty = MapSurvey::default();
    let survey = doc.map_survey().unwrap_or(&empty);
    let path = |f: &str| format!("{prefix}{f}");
    if let Some(c) = &input.campaign
        && survey.campaign(c).is_none()
    {
        return Err(fail(codes::UNKNOWN_SURVEY_RECORD, format!("GNSS arşivinde “{c}” kimlikli arazi ölçümü yok."), &path("campaign")));
    }
    let scale = input.scale.unwrap_or(1.0);
    if !(scale.is_finite() && scale > 0.0) {
        return Err(fail(codes::INVALID_SURVEY_VALUE, "Varyans çarpanı 0'dan büyük sonlu bir sayı olmalı.".into(), &path("scale")));
    }
    let package = match &input.rules {
        Some(r) => r.clone(),
        None => gnss_rules_default().ok_or_else(|| {
            fail(codes::INVALID_SURVEY_VALUE, "KentOS'un GNSS ağı kural paketi okunamadı.".into(), &path("rules"))
        })?,
    };
    if let Some(why) = rules_problem(&package) {
        return Err(fail(codes::INVALID_SURVEY_VALUE, why.into(), &path("rules")));
    }
    let class = match &input.class {
        Some(id) => Some(package.classes.iter().find(|c| c.id == *id).ok_or_else(|| {
            let names: Vec<&str> = package.classes.iter().map(|c| c.id.as_str()).collect();
            fail(
                codes::UNKNOWN_RULE_CLASS,
                format!("“{}” kural paketinde “{id}” sınıfı yok; sınıflar: {}.", package.id, names.join(", ")),
                &path("class"),
            )
        })?),
        None => None,
    };
    if let Some(e) = input.epoch.filter(|e| kentos_commands::epoch_problem(*e).is_some()) {
        return Err(fail(
            codes::INVALID_EPOCH,
            format!("Ölçme epoku 1900 ile 2100 arasında ondalık bir yıl olmalı (2026.84 gibi); yazılan {e}."),
            &path("epoch"),
        ));
    }
    // The baselines that go in.
    let used: Vec<&SurveyBaseline> = survey
        .baselines
        .iter()
        .filter(|b| b.adjust && b.excluded.is_none())
        .filter(|b| input.campaign.as_ref().is_none_or(|c| b.campaign == *c))
        .collect();
    if used.is_empty() {
        return Err(held_wrong(
            codes::NO_BASELINES,
            "Dengelemeye girecek baz yok: bazları içe aktarın, dışlananları geri alın ya da dengeleme seçimini açın.".into(),
            prefix,
        ));
    }
    let campaign = survey.campaign(&used[0].campaign).cloned();
    for b in &used[1..] {
        let c = survey.campaign(&b.campaign);
        if c.map(|c| (c.epoch, &c.velocity)) != campaign.as_ref().map(|c| (c.epoch, &c.velocity)) {
            return Err(held_wrong(
                codes::SOLUTIONS_DIFFER,
                format!(
                    "“{}” ile “{}” bazlarının arazi ölçümlerinin epok kararı ya da hız ızgarası farklı; ağ tek epok kararıyla projeye taşınır.",
                    used[0].id, b.id
                ),
                prefix,
            ));
        }
    }
    // The marks in the order the baselines name them.
    let mut marks: Vec<String> = Vec::new();
    for b in &used {
        for m in [&b.from, &b.to] {
            if !marks.iter().any(|x| x == m) {
                marks.push(m.clone());
            }
        }
    }
    let known: Vec<&SurveyKnown> =
        survey.known.iter().filter(|k| k.excluded.is_none() && marks.contains(&k.mark)).collect();
    if known.is_empty() {
        return Err(held_wrong(
            codes::DATUM_MISSING,
            "Ağın bilinen noktası yok: bazların uçlarından en az birine bilinen nokta (TUSAGA-Aktif, C1 …) ekleyin.".into(),
            prefix,
        ));
    }
    let (srid, epoch) = (known[0].srid, known[0].epoch);
    let measured = measurement_epoch(input.epoch, &used);
    match measured {
        // Without a measurement epoch the known points' one epoch is the network's (docs/adr/1740 §3).
        None => {
            if let Some(k) = known.iter().find(|k| k.srid != srid || k.epoch != epoch) {
                return Err(held_wrong(
                    codes::KNOWN_DIFFER,
                    format!(
                        "“{}” ile “{}” bilinen noktaları aynı çerçeve ve epokta değil; hepsini ölçme epokunda aynı çerçevede girin ya da bazların gözlem zamanlarını verin.",
                        known[0].id, k.id
                    ),
                    prefix,
                ));
            }
        }
        // With one each known point says its epoch, in one frame; another epoch is moved to it (docs/adr/2120 §2).
        Some((t, _)) => {
            if let Some(k) = known.iter().find(|k| k.srid != srid) {
                return Err(held_wrong(
                    codes::KNOWN_DIFFER,
                    format!("“{}” ile “{}” bilinen noktaları aynı çerçevede değil; hepsini aynı çerçevede girin.", known[0].id, k.id),
                    prefix,
                ));
            }
            if let Some(k) = known.iter().find(|k| k.epoch.is_none()) {
                return Err(held_wrong(
                    codes::KNOWN_DIFFER,
                    format!(
                        "“{}” bilinen noktasının epoku yok; ölçme epoku {} olan ağda her bilinen noktanın epoku verilmeli.",
                        k.id,
                        epoch_text(t)
                    ),
                    prefix,
                ));
            }
        }
    }
    let Some(shape) = ellipsoid_of(srid) else {
        return Err(held_wrong(
            codes::UNKNOWN_FRAME,
            format!("EPSG:{srid} kayıttaki coğrafi bir sistem değil; bilinen noktaların çerçevesini (4326 WGS 84, 5252 TUREF) verin."),
            prefix,
        ));
    };
    let hold = match &input.hold {
        Some(h) => known.iter().find(|k| k.id == *h).copied().ok_or_else(|| {
            fail(codes::UNKNOWN_SURVEY_RECORD, format!("“{h}” ağın bilinen noktalarından biri değil."), &path("hold"))
        })?,
        None => known[0],
    };
    // Each known point at the measurement epoch (BÖHHBÜY Madde 15(1)(a)): by its own velocity when it has one
    // (docs/adr/2520), else along the campaign's velocity grid.
    let mut at: HashMap<String, (f64, f64, f64)> = HashMap::new();
    let mut moved = Vec::new();
    for k in &known {
        let given = (k.lat, k.lon, k.height);
        let place_now = match (measured, k.epoch) {
            (Some((t, _)), Some(e)) if e != t => {
                let (now, by) = match k.velocity {
                    Some(v) => (by_velocity(given, shape, v, t - e), KnownMovedBy::Velocity),
                    None => {
                        let Some(grid) = velocity_of(campaign.as_ref()) else {
                            return Err(held_wrong(
                                codes::EPOCH_UNSUPPORTED,
                                format!(
                                    "“{}” bilinen noktası {} epokunda, ağın ölçme epoku {}: Madde 15(1)(a) uyarınca ölçme epokuna taşınmalı. Bilinen noktaya kendi hızını (yer merkezli VX, VY, VZ) verin, arazi ölçümüne hız ızgarasını verin (Proje ayarları › Izgaralar) ya da bilinen noktayı ölçme epokunda girin.",
                                    k.id,
                                    epoch_text(e),
                                    epoch_text(t)
                                ),
                                prefix,
                            ));
                        };
                        (moved_to(doc, &grid, given, srid, e, t)?, KnownMovedBy::Grid)
                    }
                };
                let (a, b) = (ecef(given, shape), ecef(now, shape));
                let r = gnss::enu_rows(given.0 / RAD, given.1 / RAD);
                let d = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
                let shift = [0, 1, 2].map(|i| r[i][0] * d[0] + r[i][1] * d[1] + r[i][2] * d[2]);
                moved.push(KnownMoved { id: k.id.clone(), mark: k.mark.clone(), by, from: e, years: t - e, shift });
                now
            }
            _ => given,
        };
        at.insert(k.id.clone(), place_now);
    }
    let epoch = measured.map(|m| m.0).or(epoch);
    let epoch_from = measured.map_or(NetworkEpochFrom::Known, |m| m.1);
    let place_of = |k: &SurveyKnown| at.get(&k.id).copied().unwrap_or((k.lat, k.lon, k.height));
    let known_of = |m: &str| known.iter().find(|k| k.mark == m).copied();
    let baselines: Vec<Baseline> =
        used.iter().map(|b| Baseline { from: b.from.clone(), to: b.to.clone(), d: b.d, cov: b.covariance }).collect();
    let free_stations: Vec<GnssStation> = marks
        .iter()
        .map(|m| match known_of(m) {
            Some(k) => GnssStation { name: m.clone(), xyz: Some(ecef(place_of(k), shape)), fixed: k.id == hold.id, sigma: None, cov: None },
            None => GnssStation { name: m.clone(), xyz: None, fixed: false, sigma: None, cov: None },
        })
        .collect();
    let held_stations: Vec<GnssStation> = marks
        .iter()
        .map(|m| match known_of(m) {
            Some(k) => GnssStation {
                name: m.clone(),
                xyz: Some(ecef(place_of(k), shape)),
                fixed: k.sigma.is_none(),
                sigma: None,
                cov: k.sigma.map(|s| weight(place_of(k), s)),
            },
            None => GnssStation { name: m.clone(), xyz: None, fixed: false, sigma: None, cov: None },
        })
        .collect();
    let refused = |r: gnss::Refused| held_wrong(r.code, r.message, prefix);
    let free = gnss::adjust(&GnssInput { stations: free_stations, baselines: baselines.clone(), scale, datum: Datum::Given })
        .map_err(refused)?;
    let held = gnss::adjust(&GnssInput { stations: held_stations, baselines, scale, datum: Datum::Given }).map_err(refused)?;
    let limit = w_limit(class);
    let ids: Vec<String> = used.iter().map(|b| b.id.clone()).collect();
    let known_ids: Vec<Option<String>> = marks.iter().map(|m| known_of(m).map(|k| k.id.clone())).collect();
    let froms: Vec<usize> = used.iter().map(|b| marks.iter().position(|m| *m == b.from).unwrap_or(0)).collect();
    let (free_stage, free_tests) = stage(&free, &ids, &froms, &known_ids, limit, vec![hold.mark.clone()]);
    // Madde 15(1)(c): the known points' given coordinates (at the measurement epoch) against the free stage's.
    let common: Vec<(String, String, [f64; 3], [f64; 3], f64, f64)> = known
        .iter()
        .filter_map(|k| {
            let i = marks.iter().position(|m| *m == k.mark)?;
            let at = place_of(k);
            Some((k.id.clone(), k.mark.clone(), ecef(at, shape), free.stations[i].xyz, at.0, at.1))
        })
        .collect();
    let similar = similarity(&common, class.and_then(|c| c.confidence).unwrap_or(SIMILARITY_CONFIDENCE));
    let fixed: Vec<String> = known.iter().filter(|k| k.sigma.is_none()).map(|k| k.mark.clone()).collect();
    let (held_stage, held_tests) = stage(&held, &ids, &froms, &known_ids, limit, fixed);
    let mut failed_baselines = 0u32;
    let rows: Vec<NetworkBaseline> = used
        .iter()
        .enumerate()
        .map(|(k, b)| {
            let length = baseline_length(b);
            let c = b.covariance;
            let sigma = [(c[0] * scale).sqrt(), (c[3] * scale).sqrt(), (c[5] * scale).sqrt()];
            let adjusted = free.baselines[k].adjusted;
            let limit = class.map(|c| c.constant + c.ppm * 1e-6 * length);
            let rules = class.zip(limit).map(|(c, l)| {
                let of = if c.baseline_of == BaselineSigmaOf::Adjusted { adjusted } else { sigma };
                of.iter().all(|v| *v <= l)
            });
            if rules == Some(false) {
                failed_baselines += 1;
            }
            NetworkBaseline {
                id: b.id.clone(),
                from: b.from.clone(),
                to: b.to.clone(),
                length,
                sigma,
                adjusted,
                free: free_tests[k].clone(),
                constrained: held_tests[k].clone(),
                limit,
                rules,
            }
        })
        .collect();
    let mut failed_stations = 0u32;
    let mut stations = Vec::with_capacity(marks.len());
    let mut new = Vec::new();
    for (i, m) in marks.iter().enumerate() {
        let s = &held.stations[i];
        let (lat, lon, h) = gnss::to_geodetic(shape.0, shape.1, s.xyz);
        let (lat, lon) = (lat * RAD, lon * RAD);
        let sigma = [s.senu[1], s.senu[0], s.senu[2]];
        let k = known_of(m);
        let point = survey.points.iter().find(|p| p.mark == *m);
        let mut row = NetworkStation {
            mark: m.clone(),
            known: k.map(|k| k.id.clone()),
            held: s.held,
            lat,
            lon,
            height: h,
            sigma,
            point: point.map(|p| p.id.clone()),
            x: None,
            y: None,
            z: None,
            dx: None,
            dy: None,
            dz: None,
            rules: None,
            // docs/adr/2560: the confidence region of a mark the stage moved.
            ellipse: (!s.held).then(|| error_ellipse(s.cenu, class.and_then(|c| c.confidence).unwrap_or(CONFIDENCE))),
        };
        if k.is_none() {
            let placed = place(doc, campaign.as_ref(), (lat, lon, h), srid, epoch, "operation").map_err(Stop::into_error);
            if let Ok(p) = &placed {
                row.x = Some(p.x);
                row.y = Some(p.y);
                row.z = p.z;
                if let Some(pt) = point {
                    row.dx = Some(p.x - pt.result.x);
                    row.dy = Some(p.y - pt.result.y);
                    row.dz = p.z.zip(pt.result.z).map(|(a, b)| a - b);
                }
            }
            if let Some(c) = class {
                let ok = sigma[0] <= c.horizontal && sigma[1] <= c.horizontal && sigma[2] <= c.height;
                row.rules = Some(ok);
                if !ok {
                    failed_stations += 1;
                }
            }
            new.push(NewMark { mark: m.clone(), lat, lon, height: h, sigma, placed });
        }
        stations.push(row);
    }
    let rules = class.map(|c| {
        // The similarity's scale against the class's limit (Madde 15(1)(c): C1's 3 ppm), when both are there.
        let similar_holds = c.similarity.zip(similar.as_ref()).map(|(limit, s)| s.scale.abs() <= limit);
        NetworkRules {
            package: package.id.clone(),
            release: package.release,
            class: c.id.clone(),
            articles: c.articles.clone(),
            confidence: c.confidence,
            baseline_of: c.baseline_of,
            passed: failed_baselines == 0 && failed_stations == 0 && similar_holds != Some(false),
            baselines: failed_baselines,
            stations: failed_stations,
            similarity: similar_holds,
        }
    });
    // The same network already accepted: the same baselines, known points, held point, factor and class, and every
    // new mark's point carrying that adjustment now.
    let class_of = class.map(|c| (c.id.clone(), format!("{}@{}", package.id, package.release)));
    let known_ids_used: Vec<String> = known.iter().map(|k| k.id.clone()).collect();
    let current = survey
        .adjustments
        .iter()
        .rev()
        .find(|a| {
            a.baselines == ids
                && a.known == known_ids_used
                && a.hold == hold.id
                && a.scale == scale
                && a.epoch == epoch
                && a.class.clone().zip(a.rules.clone()) == class_of
                && !new.is_empty()
                && new.iter().all(|n| {
                    survey.points.iter().any(|p| p.mark == n.mark && p.result.adjustment.as_deref() == Some(a.id.as_str()))
                })
        })
        .map(|a| a.id.clone());
    let mut why = Vec::new();
    if let Some(a) = &current {
        why.push(format!("Bu ağ “{a}” dengelemesiyle kabul edildi; noktaları güncel."));
    }
    for (name, st) in [("Serbest dengelemede", &free_stage), ("Bilinen noktalara dayalı dengelemede", &held_stage)] {
        if st.blunders > 0 {
            why.push(format!(
                "{name} {} uyuşumsuz ölçü var (test değeri {:.2}'yi aşıyor); en kötüsü “{}”.",
                st.blunders,
                st.limit,
                st.worst.clone().unwrap_or_default()
            ));
        }
        if st.passed == Some(false) {
            why.push(format!(
                "{name} model testi geçmedi: vᵀPv = {:.2}, χ² sınırı {:.2} (m0 = {:.2}).",
                st.omega,
                st.chi2.unwrap_or(0.0),
                st.m0.unwrap_or(0.0)
            ));
        }
    }
    for n in &new {
        if let Err(e) = &n.placed {
            why.push(format!("“{}” projeye taşınamadı: {}", n.mark, e.message));
        }
    }
    if new.is_empty() {
        why.push("Ağda bilinen nokta olmayan işaret yok; dengelenecek nokta yok.".into());
    }
    let out = MapSurveyNetworked {
        srid,
        epoch,
        epoch_from,
        moved,
        known: known.iter().map(|k| k.id.clone()).collect(),
        baselines: rows,
        stations,
        free: free_stage,
        constrained: held_stage,
        similarity: similar,
        rules,
        current,
        acceptable: why.is_empty(),
        why,
        revision: doc.revision().to_string(),
    };
    Ok(Analysis {
        out,
        baselines: ids,
        known: known.iter().map(|k| k.id.clone()).collect(),
        hold: hold.id.clone(),
        scale,
        class: class.map(|c| (c.id.clone(), format!("{}@{}", package.id, package.release))),
        new,
    })
}

/// The campaign's velocity grid as the chain takes it.
fn velocity_of(campaign: Option<&SurveyCampaign>) -> Option<VelocityModel> {
    campaign.and_then(|c| c.velocity.as_ref()).map(|g| VelocityModel { name: g.name.clone(), sha256: g.sha256.clone() })
}

/// An epoch as a sentence writes it: two decimals, a decimal comma (2026,84).
fn epoch_text(e: f64) -> String {
    format!("{e:.2}").replace('.', ",")
}

/// The answer's warnings: a class that does not hold, a network without degrees of freedom; `prefix` is the input's
/// path (“operation.” in an edit).
pub(crate) fn warnings_of(out: &MapSurveyNetworked, prefix: &str) -> Vec<CommandWarning> {
    let mut w = Vec::new();
    if let Some(r) = out.rules.as_ref().filter(|r| !r.passed) {
        w.push(CommandWarning {
            code: codes::NETWORK_RULES_FAILED.into(),
            message: {
                // Madde 15(1)(c)'s scale, when it is what fails (docs/adr/2160).
                let scale = match (r.similarity, out.similarity.as_ref()) {
                    (Some(false), Some(s)) => format!("; benzerlik dönüşümünün ölçeği sınırı aşıyor (|1 − λ| = {:.2} ppm)", s.scale.abs()),
                    _ => String::new(),
                };
                format!(
                    "{} sınıfı ({} {}) tutmuyor: {} baz ve {} nokta sınırı aşıyor{scale}.",
                    r.class, r.package, r.articles, r.baselines, r.stations
                )
            },
            path: Some(format!("{prefix}class")),
        });
    }
    if out.free.dof == 0 {
        w.push(CommandWarning {
            code: codes::NETWORK_UNCHECKED.into(),
            message: "Ağda fazla ölçü yok; uyuşumsuz baz ve model sınanamaz.".into(),
            path: None,
        });
    }
    w
}

/// Checks `input` against the document.
pub fn validate(cx: &ExecutionContext<'_>, input: &MapSurveyNetwork) -> CommandResult<()> {
    match analyse(cx.doc, input, "") {
        Ok(a) => CommandResult::Completed { output: (), warnings: warnings_of(&a.out, "") },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn plan(cx: &ExecutionContext<'_>, input: &MapSurveyNetwork) -> CommandResult<MapSurveyNetworked> {
    match analyse(cx.doc, input, "") {
        Ok(a) => {
            let warnings = warnings_of(&a.out, "");
            CommandResult::Completed { output: a.out, warnings }
        }
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: MapSurveyNetwork) -> CommandResult<MapSurveyNetworked> {
    plan(cx, &input)
}

/// The network for a window, or why it does not adjust.
pub fn preview(doc: &Document, input: &MapSurveyNetwork) -> Result<MapSurveyNetworked, String> {
    analyse(doc, input, "").map(|a| a.out).map_err(|stop| stop.into_error().message)
}
